use crate::db::main::electrum_server::schema::ElectrumServer;
use crate::{db, Result};
use deadpool_sqlite::Pool;
use electrum_client::bitcoin::base58;
use electrum_client::bitcoin::bip32::{ChildNumber, Xpub};
use electrum_client::bitcoin::hashes::Hash;
use electrum_client::bitcoin::secp256k1::Secp256k1;
use electrum_client::bitcoin::taproot::TapTweakHash;
use electrum_client::bitcoin::XOnlyPublicKey;
use electrum_client::bitcoin::{ScriptBuf, Transaction, Txid};
use electrum_client::{Client, ElectrumApi};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::task;

const GAP_LIMIT: u32 = 100;

const RECENT_TX_LIMIT: usize = 10;

const ELECTRUM_ENDPOINT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

const XPUB_VERSION: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
const TPUB_VERSION: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];
const YPUB_VERSION: [u8; 4] = [0x04, 0x9D, 0x7C, 0xB2];
const UPUB_VERSION: [u8; 4] = [0x04, 0x4A, 0x52, 0x62];
const ZPUB_VERSION: [u8; 4] = [0x04, 0xB2, 0x47, 0x46];
const VPUB_VERSION: [u8; 4] = [0x04, 0x5F, 0x1C, 0xF6];

const TAPROOT_XPUB_PREFIX: &str = "taproot:";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TxSummary {
    pub id: String,
    pub received: i64,
    pub sent: i64,
    pub delta: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub id: i64,
    pub name: String,
    pub xpub: String,
    pub balance_sats: i64,
    pub tx: Vec<TxSummary>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Res {
    pub wallets: Vec<WalletSnapshot>,
}

impl Res {
    fn empty() -> Self {
        Self {
            wallets: Vec::new(),
        }
    }
}

#[allow(dead_code)]
pub async fn run(pool: &Pool) -> Result<Res> {
    let wallets = db::main::wallet::queries::select_all(pool).await?;
    let wallets: Vec<(i64, String, String)> = wallets
        .into_iter()
        .filter(|w| w.deleted_at.is_none())
        .map(|w| (w.id, w.name, w.xpub))
        .collect();
    let servers = db::main::electrum_server::queries::select_all(pool).await?;
    let servers = active_servers_as_tuples(servers);
    let res = task::spawn_blocking(move || aggregate(&wallets, &servers))
        .await
        .map_err(|e| crate::Error::Other(format!("blocking join failed: {}", e)))??;
    Ok(res)
}

/// Drops soft-deleted rows and projects the rest to the `(name, url, spki_pin)`
/// tuples the scan loop iterates over. Order is preserved: `select_all` already
/// returns rows ordered by descending priority, so the resulting `Vec` is in
/// the order the scan should try them.
pub(crate) fn active_servers_as_tuples(
    servers: Vec<ElectrumServer>,
) -> Vec<(String, String, String)> {
    servers
        .into_iter()
        .filter(|s| s.deleted_at.is_none())
        .map(|s| (s.name, s.url, s.spki_pin))
        .collect()
}

pub(crate) fn aggregate(
    wallets: &[(i64, String, String)],
    electrum_servers: &[(String, String, String)],
) -> Result<Res> {
    if wallets.is_empty() {
        return Ok(Res::empty());
    }
    let endpoints = parse_electrum_endpoints(electrum_servers);
    if endpoints.is_empty() {
        return Err(crate::Error::Other(
            "no electrum servers configured but at least one wallet is set".into(),
        ));
    }
    let mut last_err: Option<crate::Error> = None;
    for (name, url, spki_pin) in &endpoints {
        tracing::debug!(
            server = name.as_str(),
            endpoint = url.as_str(),
            "wallet scan: connecting to endpoint"
        );
        let mut client = match connect_client(url, spki_pin) {
            Ok(c) => c,
            Err(err) => {
                last_err = Some(err);
                continue;
            }
        };
        tracing::debug!(
            server = name.as_str(),
            endpoint = url.as_str(),
            "wallet scan: endpoint connected"
        );
        match scan_wallets(client.as_mut(), wallets) {
            Ok(res) => return Ok(res),
            Err(err) => {
                last_err = Some(err);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| crate::Error::Other("no electrum endpoints succeeded".into())))
}

fn connect_client(url: &str, spki_pin: &str) -> Result<Box<dyn WalletBackend>> {
    if spki_pin.is_empty() {
        let config = electrum_client::Config::builder()
            .timeout(Some(ELECTRUM_ENDPOINT_TIMEOUT))
            .build();
        tracing::debug!(
            endpoint = url,
            timeout_secs = ELECTRUM_ENDPOINT_TIMEOUT.as_secs(),
            "wallet scan: building client config"
        );
        let client = Client::from_config(url, config).map_err(|e| {
            crate::Error::Other(format!("electrum client connect failed for {}: {}", url, e))
        })?;
        Ok(Box::new(client))
    } else {
        let client = crate::service::electrum_pinned::PinnedClient::connect(url, spki_pin)
            .map_err(|e| {
                crate::Error::Other(format!("pinned electrum connect failed for {}: {}", url, e))
            })?;
        Ok(Box::new(client))
    }
}

fn scan_wallets(client: &mut dyn WalletBackend, wallets: &[(i64, String, String)]) -> Result<Res> {
    let mut snapshots: Vec<WalletSnapshot> = Vec::with_capacity(wallets.len());
    let mut last_err: Option<crate::Error> = None;
    for (id, name, xpub) in wallets {
        match scan_single_wallet(client, xpub, name) {
            Ok((balance, tx)) => snapshots.push(WalletSnapshot {
                id: *id,
                name: name.clone(),
                xpub: xpub.clone(),
                balance_sats: balance,
                tx,
            }),
            Err(err) => {
                tracing::warn!(wallet = name.as_str(), %err, "wallet scan: failed");
                last_err = Some(err);
            }
        }
    }
    if snapshots.is_empty() {
        return Err(last_err.unwrap_or_else(|| crate::Error::Other("electrum scan failed".into())));
    }
    Ok(Res { wallets: snapshots })
}

fn parse_electrum_endpoints(servers: &[(String, String, String)]) -> Vec<(String, String, String)> {
    servers
        .iter()
        .map(|(name, url, spki_pin)| {
            (
                name.clone(),
                url.trim().to_string(),
                spki_pin.trim().to_string(),
            )
        })
        .filter(|(_, url, _)| !url.is_empty())
        .collect()
}

fn scan_single_wallet(
    client: &mut dyn WalletBackend,
    xpub: &str,
    wallet: &str,
) -> Result<(i64, Vec<TxSummary>)> {
    let started_at = std::time::Instant::now();
    tracing::debug!(wallet, "wallet scan: xpub started");
    let (balance, recent) = xpub_scan(client, xpub, wallet, 1)?;
    tracing::debug!(
        wallet,
        elapsed = ?started_at.elapsed(),
        "wallet scan: xpub completed"
    );
    Ok((balance, recent))
}

fn derive_scripts(
    xpub: &Xpub,
    kind: ScriptKind,
) -> Result<Vec<electrum_client::bitcoin::ScriptBuf>> {
    let secp = Secp256k1::new();
    let cap = (GAP_LIMIT as usize) * 2;
    let mut scripts = Vec::with_capacity(cap);
    let verify = Secp256k1::verification_only();
    for chain in 0..2u32 {
        for index in 0..GAP_LIMIT {
            let path = [
                ChildNumber::from_normal_idx(chain)
                    .map_err(|e| crate::Error::Other(format!("xpub derivation failed: {}", e)))?,
                ChildNumber::from_normal_idx(index)
                    .map_err(|e| crate::Error::Other(format!("xpub derivation failed: {}", e)))?,
            ];
            let child = xpub
                .derive_pub(&secp, &path)
                .map_err(|e| crate::Error::Other(format!("xpub derivation failed: {}", e)))?;
            let compressed = child.to_pub();
            match kind {
                ScriptKind::Legacy => {
                    scripts.push(ScriptBuf::new_p2pkh(&compressed.pubkey_hash()));
                }
                ScriptKind::Nested => {
                    scripts.push(ScriptBuf::new_p2sh(
                        &ScriptBuf::p2wpkh_script_code(compressed.wpubkey_hash()).script_hash(),
                    ));
                }
                ScriptKind::Native => {
                    scripts.push(ScriptBuf::new_p2wpkh(&compressed.wpubkey_hash()));
                }
                ScriptKind::Taproot => {
                    let xonly = XOnlyPublicKey::from(compressed.0);
                    let tweak = TapTweakHash::from_key_and_tweak(xonly, None).to_scalar();
                    let (tweaked, _parity) = xonly
                        .add_tweak(&verify, &tweak)
                        .map_err(|e| crate::Error::Other(format!("taproot tweak failed: {}", e)))?;
                    scripts.push(
                        electrum_client::bitcoin::script::Builder::new()
                            .push_opcode(electrum_client::bitcoin::opcodes::all::OP_PUSHNUM_1)
                            .push_slice(tweaked.serialize())
                            .into_script(),
                    );
                }
            }
        }
    }
    Ok(scripts)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScriptKind {
    Legacy,
    Nested,
    Native,
    Taproot,
}

fn detect_script_kind_from_base58(xpub: &str) -> Result<ScriptKind> {
    let data = base58::decode_check(xpub)
        .map_err(|e| crate::Error::Other(format!("invalid base58 xpub: {}", e)))?;
    if data.len() < 4 {
        return Err(crate::Error::Other(format!(
            "xpub payload too short: {} bytes",
            data.len()
        )));
    }
    let mut version = [0u8; 4];
    version.copy_from_slice(&data[..4]);
    match version {
        XPUB_VERSION | TPUB_VERSION => Ok(ScriptKind::Legacy),
        YPUB_VERSION | UPUB_VERSION => Ok(ScriptKind::Nested),
        ZPUB_VERSION | VPUB_VERSION => Ok(ScriptKind::Native),
        _ => Err(crate::Error::Other(format!(
            "unsupported extended public key version: {:02x?}",
            version
        ))),
    }
}

fn script_kind_and_raw_xpub(xpub: &str) -> Result<(ScriptKind, &str)> {
    if let Some(stripped) = xpub.strip_prefix(TAPROOT_XPUB_PREFIX) {
        Ok((ScriptKind::Taproot, stripped))
    } else {
        Ok((detect_script_kind_from_base58(xpub)?, xpub))
    }
}

fn xpub_scan(
    client: &mut dyn WalletBackend,
    xpub: &str,
    wallet: &str,
    xpub_index: usize,
) -> Result<(i64, Vec<TxSummary>)> {
    let (kind, raw_xpub) = script_kind_and_raw_xpub(xpub)?;
    let xpub = parse_xpub(raw_xpub)?;
    let scripts = derive_scripts(&xpub, kind)?;
    let refs: Vec<&electrum_client::bitcoin::Script> = scripts.iter().map(|s| s.as_ref()).collect();
    tracing::debug!(
        wallet,
        xpub_index,
        script_count = refs.len(),
        "wallet scan: requesting balances"
    );
    let balances = client.balance(&refs)?;
    tracing::debug!(wallet, xpub_index, "wallet scan: balances received");
    let mut total: i64 = 0;
    for (confirmed, unconfirmed) in balances {
        let sat = (confirmed as i64)
            .checked_add(unconfirmed)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
        total = total
            .checked_add(sat)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
    }

    let recent = recent_txs_for_scripts(client, &refs, wallet, xpub_index)?;
    Ok((total, recent))
}

fn recent_txs_for_scripts(
    client: &mut dyn WalletBackend,
    scripts: &[&electrum_client::bitcoin::Script],
    wallet: &str,
    xpub_index: usize,
) -> Result<Vec<TxSummary>> {
    tracing::debug!(wallet, xpub_index, "wallet scan: requesting histories");
    let histories = client.history(scripts)?;
    tracing::debug!(wallet, xpub_index, "wallet scan: histories received");
    let mut candidates: Vec<(i32, [u8; 32])> = Vec::new();
    let mut seen: HashSet<[u8; 32]> = HashSet::new();
    for h in histories {
        for (height, tx_hash) in h {
            if seen.insert(tx_hash) {
                candidates.push((height, tx_hash));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let selected: Vec<Txid> = candidates
        .into_iter()
        .take(RECENT_TX_LIMIT)
        .map(|(_, raw)| Txid::from_byte_array(raw))
        .collect();
    if selected.is_empty() {
        return Ok(Vec::new());
    }

    tracing::debug!(
        wallet,
        xpub_index,
        transaction_count = selected.len(),
        "wallet scan: requesting recent transactions"
    );
    let txs: Vec<Transaction> = client.transaction(&selected)?;
    tracing::debug!(
        wallet,
        xpub_index,
        "wallet scan: recent transactions received"
    );

    let script_set: HashSet<&electrum_client::bitcoin::Script> = scripts.iter().copied().collect();

    let mut prev_needed: HashSet<Txid> = HashSet::new();
    for tx in &txs {
        for input in &tx.input {
            prev_needed.insert(input.previous_output.txid);
        }
    }
    let prev_needed: Vec<Txid> = prev_needed.into_iter().collect();
    let prev_txs: Vec<Transaction> = if prev_needed.is_empty() {
        Vec::new()
    } else {
        tracing::debug!(
            wallet,
            xpub_index,
            transaction_count = prev_needed.len(),
            "wallet scan: requesting previous transactions"
        );
        let transactions = client.transaction(&prev_needed)?;
        tracing::debug!(
            wallet,
            xpub_index,
            "wallet scan: previous transactions received"
        );
        transactions
    };
    let mut prev_value: HashMap<(Txid, u32), i64> = HashMap::new();
    let mut prev_script: HashMap<(Txid, u32), &electrum_client::bitcoin::Script> = HashMap::new();
    for tx in &prev_txs {
        let txid = tx.compute_txid();
        for (vout, out) in tx.output.iter().enumerate() {
            let vout = vout as u32;
            prev_value.insert((txid, vout), out.value.to_sat() as i64);
            prev_script.insert((txid, vout), out.script_pubkey.as_script());
        }
    }

    let mut summaries = Vec::with_capacity(txs.len());
    for tx in &txs {
        let received = sum_outputs_to_xpub(tx, &script_set);
        let sent = sum_inputs_from_xpub(tx, &script_set, &prev_value, &prev_script);
        let delta = match received.checked_sub(sent) {
            Some(v) => v,
            None => return Err(crate::Error::Other("delta overflow".into())),
        };
        summaries.push(TxSummary {
            id: tx.compute_txid().to_string(),
            received,
            sent,
            delta,
        });
    }
    Ok(summaries)
}

fn sum_outputs_to_xpub(
    tx: &Transaction,
    xpub_scripts: &HashSet<&electrum_client::bitcoin::Script>,
) -> i64 {
    let mut total: i64 = 0;
    for output in &tx.output {
        if xpub_scripts.contains(output.script_pubkey.as_script()) {
            total = match total.checked_add(output.value.to_sat() as i64) {
                Some(v) => v,
                None => return i64::MAX,
            };
        }
    }
    total
}

fn sum_inputs_from_xpub(
    tx: &Transaction,
    xpub_scripts: &HashSet<&electrum_client::bitcoin::Script>,
    prev_value: &HashMap<(Txid, u32), i64>,
    prev_script: &HashMap<(Txid, u32), &electrum_client::bitcoin::Script>,
) -> i64 {
    let mut total: i64 = 0;
    for input in &tx.input {
        let key = (input.previous_output.txid, input.previous_output.vout);
        let Some(script) = prev_script.get(&key) else {
            continue;
        };
        if !xpub_scripts.contains(*script) {
            continue;
        }
        if let Some(value) = prev_value.get(&key) {
            total = match total.checked_add(*value) {
                Some(v) => v,
                None => return i64::MAX,
            };
        }
    }
    total
}

fn parse_xpub(s: &str) -> Result<Xpub> {
    let mut data = base58::decode_check(s)
        .map_err(|e| crate::Error::Other(format!("invalid base58 xpub: {}", e)))?;
    if data.len() != 78 {
        return Err(crate::Error::Other(format!(
            "invalid xpub length: {} bytes (expected 78)",
            data.len()
        )));
    }

    let mut version = [0u8; 4];
    version.copy_from_slice(&data[..4]);
    let is_mainnet = matches!(version, XPUB_VERSION | YPUB_VERSION | ZPUB_VERSION);
    let is_testnet = matches!(version, TPUB_VERSION | UPUB_VERSION | VPUB_VERSION);
    if !is_mainnet && !is_testnet {
        return Err(crate::Error::Other(format!(
            "unsupported extended public key version: {:02x?}",
            version
        )));
    }

    data[..4].copy_from_slice(if is_mainnet {
        &XPUB_VERSION
    } else {
        &TPUB_VERSION
    });
    Xpub::decode(&data).map_err(|e| crate::Error::Other(format!("invalid xpub: {}", e)))
}

/// Backend abstraction shared by the public electrum-client and our SPKI-pinned
/// client. All wallet scanning happens through this interface so the same
/// script-derivation, balance-summing and history-walking logic works whether
/// the server is reached through a normal CA-validated TLS connection or a
/// pinned self-signed one.
type History = Vec<(i32, [u8; 32])>;

trait WalletBackend {
    /// Returns `(confirmed, unconfirmed)` in satoshis for each script, in the
    /// same order as the input slice.
    fn balance(&mut self, scripts: &[&electrum_client::bitcoin::Script])
        -> Result<Vec<(u64, i64)>>;
    /// Returns the history `(height, tx_hash)` for each script, in input order.
    #[allow(clippy::type_complexity)]
    fn history(&mut self, scripts: &[&electrum_client::bitcoin::Script]) -> Result<Vec<History>>;
    /// Fetches the full transactions for the given txids, in input order.
    fn transaction(&mut self, txids: &[Txid]) -> Result<Vec<Transaction>>;
}

impl WalletBackend for Client {
    fn balance(
        &mut self,
        scripts: &[&electrum_client::bitcoin::Script],
    ) -> Result<Vec<(u64, i64)>> {
        let res = Client::batch_script_get_balance(self, scripts)?;
        Ok(res
            .into_iter()
            .map(|b| (b.confirmed, b.unconfirmed))
            .collect())
    }

    fn history(
        &mut self,
        scripts: &[&electrum_client::bitcoin::Script],
    ) -> Result<Vec<Vec<(i32, [u8; 32])>>> {
        let res = Client::batch_script_get_history(self, scripts)?;
        let mut out = Vec::with_capacity(res.len());
        for entries in res {
            let mut parsed = Vec::with_capacity(entries.len());
            for e in entries {
                let bytes: [u8; 32] = *e.tx_hash.as_ref();
                parsed.push((e.height, bytes));
            }
            out.push(parsed);
        }
        Ok(out)
    }

    fn transaction(&mut self, txids: &[Txid]) -> Result<Vec<Transaction>> {
        Client::batch_transaction_get(self, txids).map_err(Into::into)
    }
}

impl WalletBackend for crate::service::electrum_pinned::PinnedClient {
    fn balance(
        &mut self,
        scripts: &[&electrum_client::bitcoin::Script],
    ) -> Result<Vec<(u64, i64)>> {
        let res =
            crate::service::electrum_pinned::PinnedClient::batch_script_get_balance(self, scripts)?;
        Ok(res
            .into_iter()
            .map(|b| (b.confirmed, b.unconfirmed))
            .collect())
    }

    fn history(
        &mut self,
        scripts: &[&electrum_client::bitcoin::Script],
    ) -> Result<Vec<Vec<(i32, [u8; 32])>>> {
        let res =
            crate::service::electrum_pinned::PinnedClient::batch_script_get_history(self, scripts)?;
        res.into_iter()
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|e| {
                        let bytes = hex_decode(&e.tx_hash)
                            .map_err(|err| crate::Error::Other(format!("history hex: {}", err)))?;
                        if bytes.len() != 32 {
                            return Err(crate::Error::Other(format!(
                                "history tx_hash is {} bytes, expected 32",
                                bytes.len()
                            )));
                        }
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&bytes);
                        Ok((e.height, hash))
                    })
                    .collect()
            })
            .collect()
    }

    fn transaction(&mut self, txids: &[Txid]) -> Result<Vec<Transaction>> {
        let pinned_ids: Vec<crate::service::electrum_pinned::Txid> = txids
            .iter()
            .map(|id| {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(id.as_ref());
                crate::service::electrum_pinned::Txid::from_bytes(bytes)
            })
            .collect();
        let hexes = crate::service::electrum_pinned::PinnedClient::batch_transaction_get(
            self,
            &pinned_ids,
        )?;
        let mut txs = Vec::with_capacity(hexes.len());
        for hex in hexes {
            let bytes = hex_decode(&hex)
                .map_err(|err| crate::Error::Other(format!("transaction hex: {}", err)))?;
            let tx = electrum_client::bitcoin::consensus::deserialize(&bytes)
                .map_err(|e| crate::Error::Other(format!("transaction parse: {}", e)))?;
            txs.push(tx);
        }
        Ok(txs)
    }
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

#[cfg(test)]
mod test {
    use crate::db::main::electrum_server::schema::ElectrumServer;
    use crate::db::main::test::pool;
    use crate::Result;
    use electrum_client::bitcoin::base58;
    use electrum_client::bitcoin::bip32::{ChildNumber, Xpriv, Xpub};
    use electrum_client::bitcoin::secp256k1::Secp256k1;
    use electrum_client::bitcoin::Network;

    fn fresh_xpub(seed: &[u8], path: &[ChildNumber]) -> String {
        let secp = Secp256k1::new();
        let mut key = Xpriv::new_master(Network::Bitcoin, seed).unwrap();
        for cn in path {
            key = key.derive_priv(&secp, cn).unwrap();
        }
        Xpub::from_priv(&secp, &key).to_string()
    }

    fn with_version(xpub: &str, version: [u8; 4]) -> String {
        let mut data = base58::decode_check(xpub).unwrap();
        data[..4].copy_from_slice(&version);
        base58::encode_check(&data)
    }

    #[test]
    fn parse_electrum_endpoints_single_plain() {
        let servers = vec![(
            "foo".to_string(),
            "ssl://electrum.foo.bar:50002".to_string(),
            "".to_string(),
        )];
        let endpoints = super::parse_electrum_endpoints(&servers);
        assert_eq!(
            endpoints,
            vec![(
                "foo".to_string(),
                "ssl://electrum.foo.bar:50002".to_string(),
                "".to_string(),
            )]
        );
    }

    #[test]
    fn parse_electrum_endpoints_does_not_strip_insecure_prefix() {
        // The `insecure-` prefix is no longer recognised: it is left in the URL
        // and would just cause the electrum client to fail to connect.
        let servers = vec![(
            "a".to_string(),
            "insecure-ssl://electrs.com.au:50002".to_string(),
            "".to_string(),
        )];
        let endpoints = super::parse_electrum_endpoints(&servers);
        assert_eq!(
            endpoints,
            vec![(
                "a".to_string(),
                "insecure-ssl://electrs.com.au:50002".to_string(),
                "".to_string(),
            )]
        );
    }

    #[test]
    fn parse_electrum_endpoints_trims_whitespace_and_skips_empty() {
        let servers = vec![
            (
                "a".to_string(),
                "  tcp://a:50001 ".to_string(),
                "".to_string(),
            ),
            ("b".to_string(), " ".to_string(), "".to_string()),
            ("c".to_string(), "tcp://b:50001".to_string(), "".to_string()),
            ("d".to_string(), "tcp://c:50001".to_string(), "".to_string()),
        ];
        let endpoints = super::parse_electrum_endpoints(&servers);
        assert_eq!(
            endpoints,
            vec![
                ("a".to_string(), "tcp://a:50001".to_string(), "".to_string(),),
                ("c".to_string(), "tcp://b:50001".to_string(), "".to_string(),),
                ("d".to_string(), "tcp://c:50001".to_string(), "".to_string(),),
            ]
        );
    }

    #[test]
    fn parse_electrum_endpoints_preserves_pin() {
        let pin = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let servers = vec![(
            "pinned".to_string(),
            "ssl://foo:50002".to_string(),
            pin.to_string(),
        )];
        let endpoints = super::parse_electrum_endpoints(&servers);
        assert_eq!(endpoints[0].2, pin);
    }

    #[test]
    fn parse_electrum_endpoints_empty_yields_empty_vec() {
        let endpoints = super::parse_electrum_endpoints(&[]);
        assert!(endpoints.is_empty());
    }

    #[actix_web::test]
    async fn run_with_no_wallets_returns_empty() -> Result<()> {
        let pool = pool();
        let res = super::run(&pool).await?;
        assert!(res.wallets.is_empty());
        Ok(())
    }

    #[actix_web::test]
    async fn run_with_wallet_but_no_servers_returns_error() -> Result<()> {
        let pool = pool();
        // Insert a wallet but no electrum servers at all
        crate::db::main::wallet::queries::insert(
            "spending".into(),
            "xpub0000000000000000000000000000000000000000000000000000000000000000".into(),
            &pool,
        )
        .await?;
        let res = super::run(&pool).await;
        assert!(res.is_err());
        let err = match res {
            Ok(_) => panic!("expected error"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("no electrum servers configured"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn parse_electrum_endpoints_preserves_input_order() -> Result<()> {
        // Soft-deleted servers are filtered out by the caller (`run` /
        // `wallet_cache::load_active_servers`) before reaching this helper,
        // so the only ordering guarantee `parse_electrum_endpoints` makes is
        // that it preserves whatever order it got.
        let servers = vec![
            (
                "low".to_string(),
                "ssl://low:50002".to_string(),
                "".to_string(),
            ),
            (
                "high".to_string(),
                "ssl://high:50002".to_string(),
                "".to_string(),
            ),
        ];
        let endpoints = super::parse_electrum_endpoints(&servers);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].0, "low");
        assert_eq!(endpoints[1].0, "high");
        Ok(())
    }

    #[test]
    fn active_servers_as_tuples_skips_deleted_and_preserves_order() -> Result<()> {
        let now = time::OffsetDateTime::now_utc();
        // Input order mirrors what `select_all` returns: descending priority.
        // `active_servers_as_tuples` is a pure projection — it must drop the
        // soft-deleted row and keep the rest in the same positions.
        let servers = vec![
            ElectrumServer {
                id: 2,
                name: "deleted_high".to_string(),
                url: "ssl://deleted-high:50002".to_string(),
                priority: 100,
                spki_pin: "".to_string(),
                created_at: now,
                updated_at: now,
                deleted_at: Some(now),
            },
            ElectrumServer {
                id: 3,
                name: "active_high".to_string(),
                url: "ssl://active-high:50002".to_string(),
                priority: 50,
                spki_pin: "".to_string(),
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
            ElectrumServer {
                id: 1,
                name: "active_low".to_string(),
                url: "ssl://active-low:50002".to_string(),
                priority: 1,
                spki_pin: "".to_string(),
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
        ];
        let active = super::active_servers_as_tuples(servers);
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].0, "active_high");
        assert_eq!(active[0].1, "ssl://active-high:50002");
        assert_eq!(active[1].0, "active_low");
        assert_eq!(active[1].1, "ssl://active-low:50002");
        Ok(())
    }

    #[test]
    fn parse_legacy_xpub_mainnet() -> Result<()> {
        let xpub = fresh_xpub(
            &[7u8; 32],
            &[
                ChildNumber::Hardened { index: 44 },
                ChildNumber::Hardened { index: 0 },
                ChildNumber::Hardened { index: 0 },
            ],
        );
        super::parse_xpub(&xpub)?;
        Ok(())
    }

    #[test]
    fn parse_native_segwit_zpub() -> Result<()> {
        let xpub = fresh_xpub(
            &[8u8; 32],
            &[
                ChildNumber::Hardened { index: 84 },
                ChildNumber::Hardened { index: 0 },
                ChildNumber::Hardened { index: 0 },
            ],
        );
        let zpub = with_version(&xpub, super::ZPUB_VERSION);
        assert_eq!(
            super::detect_script_kind_from_base58(&zpub)?,
            super::ScriptKind::Native
        );
        super::parse_xpub(&zpub)?;
        Ok(())
    }

    #[test]
    fn parse_nested_segwit_ypub() -> Result<()> {
        let xpub = fresh_xpub(
            &[9u8; 32],
            &[
                ChildNumber::Hardened { index: 49 },
                ChildNumber::Hardened { index: 0 },
                ChildNumber::Hardened { index: 0 },
            ],
        );
        let ypub = with_version(&xpub, super::YPUB_VERSION);
        assert_eq!(
            super::detect_script_kind_from_base58(&ypub)?,
            super::ScriptKind::Nested
        );
        super::parse_xpub(&ypub)?;
        Ok(())
    }

    #[test]
    fn detect_script_kind_from_all_standard_prefixes() -> Result<()> {
        let xpub = fresh_xpub(&[10u8; 32], &[]);
        let expected = [
            (super::XPUB_VERSION, super::ScriptKind::Legacy),
            (super::TPUB_VERSION, super::ScriptKind::Legacy),
            (super::YPUB_VERSION, super::ScriptKind::Nested),
            (super::UPUB_VERSION, super::ScriptKind::Nested),
            (super::ZPUB_VERSION, super::ScriptKind::Native),
            (super::VPUB_VERSION, super::ScriptKind::Native),
        ];
        for (version, kind) in expected {
            let encoded = with_version(&xpub, version);
            assert_eq!(super::detect_script_kind_from_base58(&encoded)?, kind);
            super::parse_xpub(&encoded)?;
        }
        Ok(())
    }

    #[test]
    fn taproot_prefix_selects_taproot_scripts() -> Result<()> {
        let xpub = fresh_xpub(&[12u8; 32], &[]);
        let prefixed = format!("{}{}", super::TAPROOT_XPUB_PREFIX, xpub);
        let (kind, raw) = super::script_kind_and_raw_xpub(&prefixed)?;
        assert_eq!(kind, super::ScriptKind::Taproot);
        assert_eq!(raw, xpub);
        Ok(())
    }

    #[test]
    fn reject_unknown_version() -> Result<()> {
        let xpub = fresh_xpub(&[11u8; 32], &[]);
        let unknown = with_version(&xpub, [0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(super::detect_script_kind_from_base58(&unknown).is_err());
        assert!(super::parse_xpub(&unknown).is_err());
        Ok(())
    }

    #[test]
    fn net_value_sums_outputs_to_xpub_scripts() {
        use electrum_client::bitcoin::hashes::Hash;
        use electrum_client::bitcoin::Transaction;
        let script_a = electrum_client::bitcoin::ScriptBuf::new_p2pkh(
            &electrum_client::bitcoin::PubkeyHash::from_byte_array([0x11; 20]),
        );
        let script_b = electrum_client::bitcoin::ScriptBuf::new_p2wpkh(
            &electrum_client::bitcoin::WPubkeyHash::from_byte_array([0x22; 20]),
        );
        let other = electrum_client::bitcoin::ScriptBuf::new_p2pkh(
            &electrum_client::bitcoin::PubkeyHash::from_byte_array([0x33; 20]),
        );

        let tx = electrum_client::bitcoin::Transaction {
            version: electrum_client::bitcoin::transaction::Version::TWO,
            lock_time: electrum_client::bitcoin::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![
                electrum_client::bitcoin::TxOut {
                    value: electrum_client::bitcoin::Amount::from_sat(50_000),
                    script_pubkey: script_a.clone(),
                },
                electrum_client::bitcoin::TxOut {
                    value: electrum_client::bitcoin::Amount::from_sat(75_000),
                    script_pubkey: other,
                },
                electrum_client::bitcoin::TxOut {
                    value: electrum_client::bitcoin::Amount::from_sat(12_345),
                    script_pubkey: script_b.clone(),
                },
            ],
        };
        let mut set: std::collections::HashSet<&electrum_client::bitcoin::Script> =
            std::collections::HashSet::new();
        set.insert(script_a.as_script());
        set.insert(script_b.as_script());
        let net = super::sum_outputs_to_xpub(&tx, &set);
        assert_eq!(net, 50_000 + 12_345);
        let _: Transaction = tx;
    }

    #[test]
    #[ignore = "hits a public Electrum server; run with `cargo test -- --ignored`"]
    fn aggregate_fetches_known_xpub_balance_and_history() -> Result<()> {
        // Hardcoded well-known mainnet xpub used as a fixture. Bitcoin history
        // is permanent, so even if the wallet is later drained the tx history
        // stays queryable forever.
        let xpub = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz".to_string();

        let wallets = vec![(1_i64, "fixture".to_string(), xpub)];
        let servers = vec![(
            "blockstream".to_string(),
            "ssl://electrum.blockstream.info:50002".to_string(),
            "".to_string(),
        )];

        let res = super::aggregate(&wallets, &servers)?;

        assert_eq!(res.wallets.len(), 1, "expected one wallet snapshot");
        let snap = &res.wallets[0];
        eprintln!(
            "balance: {} sats ({:.8} BTC)",
            snap.balance_sats,
            snap.balance_sats as f64 / 100_000_000.0
        );
        eprintln!("recent transactions ({}):", snap.tx.len());
        for tx in &snap.tx {
            eprintln!(
                "  {} received={} sent={} delta={}",
                tx.id, tx.received, tx.sent, tx.delta
            );
        }
        assert!(
            snap.balance_sats > 0,
            "expected positive balance, got {} sats",
            snap.balance_sats
        );
        assert!(
            !snap.tx.is_empty(),
            "expected at least one recent transaction"
        );
        Ok(())
    }
}
