//! Minimal SPKI-pinned Electrum client used when `electrum_server.spki_pin` is set.
//!
//! The `electrum-client` crate does not expose a way to inject a custom certificate
//! verifier, so we open the TLS connection ourselves with `rustls`, validate the
//! server's SPKI against the pin, and then run a tiny JSON-RPC client over the
//! resulting stream. Only the methods we use for wallet balance lookups are
//! implemented.
//!
//! Requests and responses are exchanged one at a time: a single batched call writes
//! the request and reads back the response before the next call is issued. This
//! lets us use a single owned `StreamOwned` as both the writer and the reader
//! without any locking.

use crate::Result;
use electrum_client::bitcoin::hashes::{sha256, Hash as HashTrait};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, Error, StreamOwned};
use rustls_pki_types::UnixTime;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const RESPONSE_BUFFER_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize, Clone)]
pub struct GetBalanceRes {
    pub confirmed: u64,
    pub unconfirmed: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetHistoryRes {
    pub height: i32,
    pub tx_hash: String,
}

pub struct PinnedClient {
    stream: StreamOwned<ClientConnection, TcpStream>,
    next_id: u64,
    leftover: Vec<u8>,
}

impl PinnedClient {
    pub fn connect(addr: &str, spki_pin: &str) -> Result<Self> {
        let (host, port) = parse_ssl_addr(addr)?;
        let pin_sha256 = parse_pin(spki_pin)?;
        let tcp = TcpStream::connect((host.as_str(), port))?;
        let timeout = Some(Duration::from_secs(5));
        tcp.set_read_timeout(timeout)?;
        tcp.set_write_timeout(timeout)?;
        let provider = provider()?;
        let verifier = Arc::new(PinnedServerCertVerifier {
            pin_sha256,
            provider: provider.clone(),
        });
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| crate::Error::Other(format!("rustls protocol versions: {}", e)))?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();
        let server_name = make_server_name(&host)?;
        let conn = ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| crate::Error::Other(format!("TLS handshake setup failed: {}", e)))?;
        let stream = StreamOwned::new(conn, tcp);
        Ok(Self {
            stream,
            next_id: 0,
            leftover: Vec::new(),
        })
    }

    pub fn batch_script_get_balance(
        &mut self,
        scripts: &[&electrum_client::bitcoin::Script],
    ) -> Result<Vec<GetBalanceRes>> {
        let requests: Vec<Value> = scripts
            .iter()
            .map(|s| {
                let hash = sha256::Hash::hash(s.as_ref());
                json!({
                    "method": "blockchain.scripthash.get_balance",
                    "params": [hex_encode(hash.as_ref())],
                    "id": self.next_id(),
                })
            })
            .collect();
        let responses = self.call_batch(&requests)?;
        decode_results(&responses)
    }

    pub fn batch_script_get_history(
        &mut self,
        scripts: &[&electrum_client::bitcoin::Script],
    ) -> Result<Vec<Vec<GetHistoryRes>>> {
        let requests: Vec<Value> = scripts
            .iter()
            .map(|s| {
                let hash = sha256::Hash::hash(s.as_ref());
                json!({
                    "method": "blockchain.scripthash.get_history",
                    "params": [hex_encode(hash.as_ref())],
                    "id": self.next_id(),
                })
            })
            .collect();
        let responses = self.call_batch(&requests)?;
        decode_results(&responses)
    }

    pub fn batch_transaction_get(&mut self, txids: &[Txid]) -> Result<Vec<String>> {
        let requests: Vec<Value> = txids
            .iter()
            .map(|t| {
                json!({
                    "method": "blockchain.transaction.get",
                    "params": [t.to_string()],
                    "id": self.next_id(),
                })
            })
            .collect();
        let responses = self.call_batch(&requests)?;
        decode_results(&responses)
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn call_batch(&mut self, requests: &[Value]) -> Result<Vec<Value>> {
        let body = serde_json::to_string(requests)
            .map_err(|e| crate::Error::Other(format!("serialize request: {}", e)))?;
        self.stream
            .write_all(body.as_bytes())
            .map_err(|e| crate::Error::Other(format!("write request: {}", e)))?;
        self.stream
            .write_all(b"\n")
            .map_err(|e| crate::Error::Other(format!("write newline: {}", e)))?;
        self.stream
            .flush()
            .map_err(|e| crate::Error::Other(format!("flush request: {}", e)))?;
        let buf = self.read_one_response()?;
        let responses: Vec<Value> = serde_json::from_str(buf.trim())
            .map_err(|e| crate::Error::Other(format!("decode response: {}", e)))?;
        Ok(responses)
    }

    /// Read bytes until we have a full JSON document terminated by `\n`. A single
    /// batched electrum response is one line of JSON, so reading until newline is
    /// sufficient. Any bytes after the newline are buffered in `self.leftover` for
    /// the next call (in practice batch responses are always exactly one line).
    fn read_one_response(&mut self) -> Result<String> {
        let mut buf = std::mem::take(&mut self.leftover);
        loop {
            if let Some(idx) = buf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buf.drain(..=idx).collect();
                self.leftover = buf;
                let line = strip_newline(&line);
                return String::from_utf8(line.to_vec())
                    .map_err(|e| crate::Error::Other(format!("response is not utf-8: {}", e)));
            }
            let mut chunk = [0u8; 4096];
            let n = self
                .stream
                .read(&mut chunk)
                .map_err(|e| crate::Error::Other(format!("read response: {}", e)))?;
            if n == 0 {
                return Err(crate::Error::Other(
                    "electrum server closed the connection before sending a response".into(),
                ));
            }
            if buf.len() + n > RESPONSE_BUFFER_BYTES as usize {
                return Err(crate::Error::Other(format!(
                    "electrum response exceeded {} bytes",
                    RESPONSE_BUFFER_BYTES
                )));
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    }
}

pub struct Txid([u8; 32]);

impl Txid {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        hex_encode(&self.0)
    }
}

fn strip_newline(line: &[u8]) -> &[u8] {
    if line.last() == Some(&b'\n') {
        &line[..line.len() - 1]
    } else {
        line
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn decode_results<T: serde::de::DeserializeOwned>(responses: &[Value]) -> Result<Vec<T>> {
    responses
        .iter()
        .map(|r| {
            let result = r
                .get("result")
                .ok_or_else(|| crate::Error::Other("missing result field".into()))?;
            serde_json::from_value(result.clone())
                .map_err(|e| crate::Error::Other(format!("decode result: {}", e)))
        })
        .collect()
}

fn parse_ssl_addr(addr: &str) -> Result<(String, u16)> {
    let stripped = addr.strip_prefix("ssl://").ok_or_else(|| {
        crate::Error::Other(format!(
            "pinned client only supports ssl:// URLs, got {}",
            addr
        ))
    })?;
    let (host, port) = stripped
        .rsplit_once(':')
        .ok_or_else(|| crate::Error::Other(format!("invalid electrum address: {}", addr)))?;
    let port: u16 = port
        .parse()
        .map_err(|e| crate::Error::Other(format!("invalid port: {}", e)))?;
    Ok((host.to_string(), port))
}

/// Returns the host with optional surrounding brackets stripped (`[::1]` → `::1`).
/// Combined with `make_server_name` this lets callers pass IPv6 literals like
/// `ssl://[::1]:50002` without the brackets leaking into the DNS name and
/// tripping rustls's hostname parser.
fn strip_ipv6_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host)
}

fn make_server_name(host: &str) -> Result<ServerName<'static>> {
    let inner = strip_ipv6_brackets(host);
    if let Ok(ip) = inner.parse::<std::net::IpAddr>() {
        Ok(ServerName::from(ip))
    } else {
        ServerName::try_from(inner.to_string())
            .map_err(|_| crate::Error::Other(format!("invalid DNS name: {}", host)))
    }
}

fn parse_pin(pin: &str) -> Result<Vec<u8>> {
    let hex = pin
        .strip_prefix("sha256:")
        .ok_or_else(|| crate::Error::Other("spki_pin must start with 'sha256:'".into()))?;
    let trimmed: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if trimmed.len() != 64 {
        return Err(crate::Error::Other(format!(
            "spki_pin must be 64 hex chars (sha256), got {} chars",
            trimmed.len()
        )));
    }
    let bytes = hex_decode(&trimmed)
        .map_err(|e| crate::Error::Other(format!("spki_pin hex decode: {}", e)))?;
    Ok(bytes)
}

fn hex_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err("odd length".into());
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks(2) {
        let hi = nibble(chunk[0])?;
        let lo = nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn nibble(b: u8) -> std::result::Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex char: {}", b as char)),
    }
}

fn provider() -> Result<Arc<CryptoProvider>> {
    static PROVIDER: OnceLock<Arc<CryptoProvider>> = OnceLock::new();
    Ok(PROVIDER
        .get_or_init(|| Arc::new(rustls::crypto::ring::default_provider()))
        .clone())
}

#[derive(Debug)]
struct PinnedServerCertVerifier {
    pin_sha256: Vec<u8>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let spki = extract_spki_der(end_entity.as_ref())
            .map_err(|e| Error::General(format!("SPKI extraction failed: {}", e)))?;
        let hash = Sha256::digest(&spki);
        if !constant_time_eq(hash.as_slice(), &self.pin_sha256) {
            return Err(Error::General(format!(
                "SPKI pin mismatch: expected sha256:{} got sha256:{}",
                hex_encode(&self.pin_sha256),
                hex_encode(hash.as_slice()),
            )));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn extract_spki_der(cert_der: &[u8]) -> std::result::Result<Vec<u8>, String> {
    use x509_parser::prelude::FromDer;
    let (_, cert) = x509_parser::certificate::X509Certificate::from_der(cert_der)
        .map_err(|e| format!("parse cert: {}", e))?;
    Ok(cert.public_key().raw.to_vec())
}

/// Constant-time byte slice equality. Both inputs are always 32 bytes (a SHA-256
/// digest), so the length check is data-independent and the loop runs in time
/// proportional to the digest length regardless of where the first mismatch is.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_pin_accepts_sha256_hex() {
        let pin = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let bytes = parse_pin(pin).unwrap();
        assert_eq!(bytes.len(), 32);
        assert!(bytes.iter().all(|b| *b == 0));
    }

    #[test]
    fn parse_pin_accepts_uppercase() {
        let pin = "sha256:DEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF";
        assert_eq!(parse_pin(pin).unwrap().len(), 32);
    }

    #[test]
    fn parse_pin_rejects_missing_prefix() {
        assert!(
            parse_pin("0000000000000000000000000000000000000000000000000000000000000000").is_err()
        );
    }

    #[test]
    fn parse_pin_rejects_wrong_length() {
        assert!(parse_pin("sha256:abcd").is_err());
    }

    #[test]
    fn parse_pin_rejects_non_hex() {
        assert!(
            parse_pin("sha256:zzzz00000000000000000000000000000000000000000000000000000000")
                .is_err()
        );
    }

    #[test]
    fn parse_ssl_addr_extracts_host_and_port() {
        let (host, port) = parse_ssl_addr("ssl://electrum.example:50002").unwrap();
        assert_eq!(host, "electrum.example");
        assert_eq!(port, 50002);
    }

    #[test]
    fn parse_ssl_addr_preserves_ipv6_brackets_for_caller_stripping() {
        let (host, port) = parse_ssl_addr("ssl://[::1]:50002").unwrap();
        assert_eq!(host, "[::1]");
        assert_eq!(port, 50002);
        let (host, port) = parse_ssl_addr("ssl://[fe80::1]:50002").unwrap();
        assert_eq!(host, "[fe80::1]");
        assert_eq!(port, 50002);
    }

    #[test]
    fn make_server_name_strips_ipv6_brackets() {
        match make_server_name("[::1]").unwrap() {
            ServerName::IpAddress(ip) => assert_eq!(std::net::IpAddr::from(ip).to_string(), "::1"),
            _ => panic!("expected IpAddress"),
        }
        match make_server_name("::1").unwrap() {
            ServerName::IpAddress(ip) => assert_eq!(std::net::IpAddr::from(ip).to_string(), "::1"),
            _ => panic!("expected IpAddress"),
        }
        match make_server_name("electrum.example").unwrap() {
            ServerName::DnsName(dns) => assert_eq!(dns.as_ref(), "electrum.example"),
            _ => panic!("expected DnsName"),
        }
    }

    #[test]
    fn make_server_name_rejects_invalid_dns() {
        assert!(make_server_name("not a valid dns name!").is_err());
    }

    #[test]
    fn constant_time_eq_returns_false_on_difference() {
        let a = vec![0u8; 32];
        let mut b = vec![0u8; 32];
        b[31] = 1;
        assert!(!constant_time_eq(&a, &b));
    }

    #[test]
    fn constant_time_eq_returns_true_on_equal() {
        let a = vec![42u8; 32];
        let b = vec![42u8; 32];
        assert!(constant_time_eq(&a, &b));
    }

    #[test]
    fn constant_time_eq_returns_false_on_length_mismatch() {
        let a = vec![0u8; 32];
        let b = vec![0u8; 16];
        assert!(!constant_time_eq(&a, &b));
    }

    #[test]
    fn parse_ssl_addr_rejects_non_ssl_scheme() {
        assert!(parse_ssl_addr("tcp://electrum.example:50001").is_err());
    }

    #[test]
    fn hex_encode_round_trips() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        assert_eq!(hex_encode(&bytes), "deadbeef");
    }

    #[test]
    fn hex_decode_rejects_garbage() {
        assert!(hex_decode("xyz").is_err());
        assert!(hex_decode("abc").is_err());
    }
}
