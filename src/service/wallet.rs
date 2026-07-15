use crate::{db, Result};
use deadpool_sqlite::Pool;
use electrum_client::bitcoin::base58;
use electrum_client::bitcoin::bip32::{ChildNumber, Xpub};
use electrum_client::bitcoin::hashes::Hash;
use electrum_client::bitcoin::secp256k1::Secp256k1;
use electrum_client::bitcoin::taproot::TapTweakHash;
use electrum_client::bitcoin::XOnlyPublicKey;
use electrum_client::bitcoin::{Network, ScriptBuf, Transaction, Txid};
use electrum_client::{Client, ElectrumApi};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::task;

const GAP_LIMIT: u32 = 100;

const RECENT_TX_LIMIT: usize = 10;

const XPUB_MAINNET: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
const XPUB_TESTNET: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];

#[derive(Clone, Debug, Serialize)]
pub struct TxSummary {
    pub id: String,
    pub received: i64,
    pub sent: i64,
    pub delta: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub spending: i64,
    pub donations: i64,
    pub treasury: i64,
    pub spending_tx: Vec<TxSummary>,
    pub donations_tx: Vec<TxSummary>,
    pub treasury_tx: Vec<TxSummary>,
}

impl Res {
    fn empty() -> Self {
        Self {
            spending: 0,
            donations: 0,
            treasury: 0,
            spending_tx: Vec::new(),
            donations_tx: Vec::new(),
            treasury_tx: Vec::new(),
        }
    }
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let conf = db::main::conf::queries::select(pool).await?;
    let res = task::spawn_blocking(move || {
        aggregate(
            &conf.xpub_spending,
            &conf.xpub_donations,
            &conf.xpub_treasury,
            &conf.electrum_url,
        )
    })
    .await
    .map_err(|e| crate::Error::Other(format!("blocking join failed: {}", e)))??;
    Ok(res)
}

fn aggregate(spending: &str, donations: &str, treasury: &str, electrum_url: &str) -> Result<Res> {
    let has_any_xpub =
        !spending.trim().is_empty() || !donations.trim().is_empty() || !treasury.trim().is_empty();
    if !has_any_xpub {
        return Ok(Res::empty());
    }
    let endpoints = parse_electrum_endpoints(electrum_url)?;
    if endpoints.is_empty() {
        return Err(crate::Error::Other(
            "electrum_url is empty but at least one xpub is configured".into(),
        ));
    }
    let mut last_err: Option<crate::Error> = None;
    for (url, insecure_tls) in &endpoints {
        if *insecure_tls {
            tracing::warn!(
                "Electrum endpoint {} uses the insecure- prefix: TLS certificate validation \
                 is disabled for this endpoint. This is unsafe on untrusted networks.",
                url
            );
        }
        let config = electrum_client::Config::builder()
            .validate_domain(!*insecure_tls)
            .build();
        let mut client = match Client::from_config(url, config) {
            Ok(client) => client,
            Err(e) => {
                last_err = Some(crate::Error::Other(format!(
                    "electrum client connect failed for {}: {}",
                    url, e
                )));
                continue;
            }
        };
        match (
            scan_xpubs(&mut client, spending),
            scan_xpubs(&mut client, donations),
            scan_xpubs(&mut client, treasury),
        ) {
            (
                Ok((spending_bal, spending_tx)),
                Ok((donations_bal, donations_tx)),
                Ok((treasury_bal, treasury_tx)),
            ) => {
                return Ok(Res {
                    spending: spending_bal,
                    donations: donations_bal,
                    treasury: treasury_bal,
                    spending_tx,
                    donations_tx,
                    treasury_tx,
                });
            }
            (a, b, c) => {
                let err = a.err().or(b.err()).or(c.err()).unwrap_or_else(|| {
                    crate::Error::Other(format!("electrum scan failed for {}", url))
                });
                last_err = Some(err);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| crate::Error::Other("no electrum endpoints succeeded".into())))
}

fn parse_electrum_endpoints(raw: &str) -> Result<Vec<(String, bool)>> {
    let mut out = Vec::new();
    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (url, insecure_tls) = if let Some(stripped) = entry.strip_prefix("insecure-") {
            (stripped.to_string(), true)
        } else {
            (entry.to_string(), false)
        };
        out.push((url, insecure_tls));
    }
    Ok(out)
}

fn scan_xpubs(client: &mut Client, xpubs: &str) -> Result<(i64, Vec<TxSummary>)> {
    let mut total: i64 = 0;
    let mut all_recent: Vec<TxSummary> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for xpub in xpubs.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (balance, recent) = xpub_scan(client, xpub)?;
        total = total
            .checked_add(balance)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
        for tx in recent {
            if seen.insert(tx.id.clone()) {
                all_recent.push(tx);
            }
        }
    }
    all_recent.truncate(RECENT_TX_LIMIT);
    Ok((total, all_recent))
}

fn derive_scripts(xpub: &Xpub) -> Result<[Vec<electrum_client::bitcoin::ScriptBuf>; 4]> {
    let secp = Secp256k1::new();
    let mut legacy: Vec<electrum_client::bitcoin::ScriptBuf> =
        Vec::with_capacity((GAP_LIMIT as usize) * 2);
    let mut nested: Vec<electrum_client::bitcoin::ScriptBuf> =
        Vec::with_capacity((GAP_LIMIT as usize) * 2);
    let mut native: Vec<electrum_client::bitcoin::ScriptBuf> =
        Vec::with_capacity((GAP_LIMIT as usize) * 2);
    let mut taproot: Vec<electrum_client::bitcoin::ScriptBuf> =
        Vec::with_capacity((GAP_LIMIT as usize) * 2);
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
            legacy.push(ScriptBuf::new_p2pkh(&compressed.pubkey_hash()));
            nested.push(ScriptBuf::new_p2sh(
                &ScriptBuf::p2wpkh_script_code(compressed.wpubkey_hash()).script_hash(),
            ));
            native.push(ScriptBuf::new_p2wpkh(&compressed.wpubkey_hash()));
            let xonly = XOnlyPublicKey::from(compressed.0);
            let tweak = TapTweakHash::from_key_and_tweak(xonly, None).to_scalar();
            let (tweaked, _parity) = xonly
                .add_tweak(&verify, &tweak)
                .map_err(|e| crate::Error::Other(format!("taproot tweak failed: {}", e)))?;
            taproot.push(
                electrum_client::bitcoin::script::Builder::new()
                    .push_opcode(electrum_client::bitcoin::opcodes::all::OP_PUSHNUM_1)
                    .push_slice(tweaked.serialize())
                    .into_script(),
            );
        }
    }
    Ok([legacy, nested, native, taproot])
}

fn xpub_scan(client: &mut Client, xpub: &str) -> Result<(i64, Vec<TxSummary>)> {
    let xpub = parse_xpub(xpub)?;
    let script_sets = derive_scripts(&xpub)?;
    let mut total: i64 = 0;
    let mut all_scripts: Vec<electrum_client::bitcoin::ScriptBuf> = Vec::new();
    for set in &script_sets {
        all_scripts.extend(set.iter().cloned());
    }
    let refs: Vec<&electrum_client::bitcoin::Script> =
        all_scripts.iter().map(|s| s.as_ref()).collect();
    let balances = client.batch_script_get_balance(&refs)?;
    for balance in balances {
        let sat = (balance.confirmed as i64)
            .checked_add(balance.unconfirmed)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
        total = total
            .checked_add(sat)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
    }

    let recent = recent_txs_for_scripts(client, &refs)?;
    Ok((total, recent))
}

fn recent_txs_for_scripts(
    client: &mut Client,
    scripts: &[&electrum_client::bitcoin::Script],
) -> Result<Vec<TxSummary>> {
    let histories = client.batch_script_get_history(scripts)?;
    let mut candidates: Vec<(i32, [u8; 32])> = Vec::new();
    let mut seen: HashSet<[u8; 32]> = HashSet::new();
    for h in histories {
        for entry in h {
            let raw: [u8; 32] = *entry.tx_hash.as_ref();
            if seen.insert(raw) {
                candidates.push((entry.height, raw));
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

    let txs: Vec<Transaction> = client.batch_transaction_get(&selected)?;

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
        client.batch_transaction_get(&prev_needed)?
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

    let is_mainnet = matches!(
        version,
        [0x04, 0x88, _, _] | [0x04, 0x9D, _, _] | [0x04, 0xB2, _, _]
    );
    let network = if is_mainnet {
        Network::Bitcoin
    } else {
        Network::Testnet
    };
    let _ = network; // network is conveyed by the version bytes; we just need to normalize them

    data[..4].copy_from_slice(if is_mainnet {
        &XPUB_MAINNET
    } else {
        &XPUB_TESTNET
    });
    Xpub::decode(&data).map_err(|e| crate::Error::Other(format!("invalid xpub: {}", e)))
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::Result;
    use electrum_client::bitcoin::base58;
    use electrum_client::bitcoin::bip32::{ChildNumber, Xpriv, Xpub};
    use electrum_client::bitcoin::secp256k1::Secp256k1;
    use electrum_client::bitcoin::Network;

    const ZPUB_MAINNET_VERSION: [u8; 4] = [0x04, 0xB2, 0x47, 0x0F];
    const YPUB_MAINNET_VERSION: [u8; 4] = [0x04, 0x9D, 0x7C, 0xB2];
    // Derives a mainnet xpub at `path` from `seed` and returns its base58 form.
    // Used to build ad-hoc xpubs at test time so no real wallet material ends
    // up in the source tree.
    fn fresh_xpub(seed: &[u8], path: &[ChildNumber]) -> String {
        let secp = Secp256k1::new();
        let mut key = Xpriv::new_master(Network::Bitcoin, seed).unwrap();
        for cn in path {
            key = key.derive_priv(&secp, cn).unwrap();
        }
        Xpub::from_priv(&secp, &key).to_string()
    }

    // Re-encodes `xpub` with a different version-byte prefix.
    fn with_version(xpub: &str, version: [u8; 4]) -> String {
        let mut data = base58::decode_check(xpub).unwrap();
        data[..4].copy_from_slice(&version);
        base58::encode_check(&data)
    }

    #[test]
    fn split_xpubs_skips_empty_entries() {
        let xpubs = "  , , ";
        let entries: Vec<&str> = xpubs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_electrum_endpoints_single_plain() {
        let endpoints = super::parse_electrum_endpoints("ssl://electrum.foo.bar:50002").unwrap();
        assert_eq!(
            endpoints,
            vec![("ssl://electrum.foo.bar:50002".to_string(), false)]
        );
    }

    #[test]
    fn parse_electrum_endpoints_mixed_with_insecure_prefix() {
        let endpoints = super::parse_electrum_endpoints(
            "insecure-ssl://electrs.com.au:50002,ssl://electrum.foo.bar:50002",
        )
        .unwrap();
        assert_eq!(
            endpoints,
            vec![
                ("ssl://electrs.com.au:50002".to_string(), true),
                ("ssl://electrum.foo.bar:50002".to_string(), false),
            ]
        );
    }

    #[test]
    fn parse_electrum_endpoints_trims_whitespace_and_skips_empty() {
        let endpoints = super::parse_electrum_endpoints(
            "  tcp://a:50001 , , insecure-tcp://b:50001  ,,tcp://c:50001",
        )
        .unwrap();
        assert_eq!(
            endpoints,
            vec![
                ("tcp://a:50001".to_string(), false),
                ("tcp://b:50001".to_string(), true),
                ("tcp://c:50001".to_string(), false),
            ]
        );
    }

    #[test]
    fn parse_electrum_endpoints_empty_string_yields_empty_vec() {
        let endpoints = super::parse_electrum_endpoints("").unwrap();
        assert!(endpoints.is_empty());
    }

    #[test]
    fn split_xpubs_collects_non_empty_entries() {
        let xpubs = "xpubAAA, zpubBBB ,,";
        let entries: Vec<&str> = xpubs
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(entries, vec!["xpubAAA", "zpubBBB"]);
    }

    #[actix_web::test]
    async fn run_with_empty_conf_returns_zeros() -> Result<()> {
        let pool = pool();
        let res = super::run(&pool).await?;
        assert_eq!(res.spending, 0);
        assert_eq!(res.donations, 0);
        assert_eq!(res.treasury, 0);
        assert!(res.spending_tx.is_empty());
        assert!(res.donations_tx.is_empty());
        assert!(res.treasury_tx.is_empty());
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
        let zpub = with_version(&xpub, ZPUB_MAINNET_VERSION);
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
        let ypub = with_version(&xpub, YPUB_MAINNET_VERSION);
        super::parse_xpub(&ypub)?;
        Ok(())
    }

    #[test]
    fn parse_unknown_version_normalizes() -> Result<()> {
        let xpub = fresh_xpub(
            &[10u8; 32],
            &[
                ChildNumber::Hardened { index: 44 },
                ChildNumber::Hardened { index: 0 },
                ChildNumber::Hardened { index: 0 },
            ],
        );
        let bogus = with_version(&xpub, [0xFF, 0xFF, 0xFF, 0xFF]);
        super::parse_xpub(&bogus)?;
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
}
