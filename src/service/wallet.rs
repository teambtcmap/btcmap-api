use crate::{db, Result};
use deadpool_sqlite::Pool;
use electrum_client::bitcoin::base58;
use electrum_client::bitcoin::bip32::{ChildNumber, Xpub};
use electrum_client::bitcoin::secp256k1::Secp256k1;
use electrum_client::bitcoin::{CompressedPublicKey, Network};
use electrum_client::{Client, ElectrumApi};
use serde::Serialize;
use tokio::task;

const GAP_LIMIT: u32 = 20;

const DEFAULT_ELECTRUM_URL: &str = "ssl://electrum.blockstream.info:50002";

const XPUB_MAINNET: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
const XPUB_TESTNET: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];

#[derive(Clone, Copy, Debug)]
enum XpubKind {
    LegacyP2pkh,
    NestedSegwitP2shwpkh,
    NativeSegwitP2wpkh,
}

#[derive(Serialize)]
pub struct Res {
    pub spending: i64,
    pub donations: i64,
    pub treasury: i64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let conf = db::main::conf::queries::select(pool).await?;
    let url = std::env::var("ELECTRUM_URL").unwrap_or_else(|_| DEFAULT_ELECTRUM_URL.to_string());
    let res = task::spawn_blocking(move || {
        aggregate(
            &url,
            &conf.xpub_spending,
            &conf.xpub_donations,
            &conf.xpub_treasury,
        )
    })
    .await
    .map_err(|e| crate::Error::Other(format!("blocking join failed: {}", e)))??;
    Ok(res)
}

fn aggregate(electrum_url: &str, spending: &str, donations: &str, treasury: &str) -> Result<Res> {
    let has_any_xpub =
        !spending.trim().is_empty() || !donations.trim().is_empty() || !treasury.trim().is_empty();
    if !has_any_xpub {
        return Ok(Res {
            spending: 0,
            donations: 0,
            treasury: 0,
        });
    }
    let mut client = Client::new(electrum_url)?;
    Ok(Res {
        spending: sum_xpubs(&mut client, spending)?,
        donations: sum_xpubs(&mut client, donations)?,
        treasury: sum_xpubs(&mut client, treasury)?,
    })
}

fn sum_xpubs(client: &mut Client, xpubs: &str) -> Result<i64> {
    let mut total: i64 = 0;
    for xpub in xpubs.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        total = total
            .checked_add(xpub_balance(client, xpub)?)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
    }
    Ok(total)
}

fn xpub_balance(client: &mut Client, xpub: &str) -> Result<i64> {
    let (kind, xpub) = parse_xpub(xpub)?;
    let secp = Secp256k1::new();
    let mut scripts: Vec<electrum_client::bitcoin::ScriptBuf> =
        Vec::with_capacity((GAP_LIMIT as usize) * 2);

    for chain in 0..2 {
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
            scripts.push(script_for_kind(kind, &compressed));
        }
    }

    let refs: Vec<&electrum_client::bitcoin::Script> = scripts
        .iter()
        .map(|s| s.as_ref())
        .collect();
    let balances = client.batch_script_get_balance(&refs)?;
    let mut total: i64 = 0;
    for balance in balances {
        let sat = (balance.confirmed as i64)
            .checked_add(balance.unconfirmed)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
        total = total
            .checked_add(sat)
            .ok_or_else(|| crate::Error::Other("balance overflow".into()))?;
    }
    Ok(total)
}

fn script_for_kind(
    kind: XpubKind,
    pubkey: &CompressedPublicKey,
) -> electrum_client::bitcoin::ScriptBuf {
    use electrum_client::bitcoin::ScriptBuf;
    match kind {
        XpubKind::LegacyP2pkh => ScriptBuf::new_p2pkh(&pubkey.pubkey_hash()),
        XpubKind::NestedSegwitP2shwpkh => {
            ScriptBuf::new_p2sh(&ScriptBuf::p2wpkh_script_code(pubkey.wpubkey_hash()).script_hash())
        }
        XpubKind::NativeSegwitP2wpkh => ScriptBuf::new_p2wpkh(&pubkey.wpubkey_hash()),
    }
}

fn parse_xpub(s: &str) -> Result<(XpubKind, Xpub)> {
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

    let kind = match version {
        [0x04, 0x88, _, _] => XpubKind::LegacyP2pkh,
        [0x04, 0x9D, _, _] => XpubKind::NestedSegwitP2shwpkh,
        [0x04, 0xB2, _, _] => XpubKind::NativeSegwitP2wpkh,
        [0x04, 0x35, _, _] => XpubKind::LegacyP2pkh,
        [0x04, 0x4A, _, _] => XpubKind::NestedSegwitP2shwpkh,
        [0x04, 0x5F, _, _] => {
            return Err(crate::Error::Other(
                "taproot (vpub) xpubs are not yet supported".into(),
            ));
        }
        _ => {
            return Err(crate::Error::Other(format!(
                "unknown xpub version magic bytes: {:?}",
                version
            )));
        }
    };

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
    let xpub =
        Xpub::decode(&data).map_err(|e| crate::Error::Other(format!("invalid xpub: {}", e)))?;
    Ok((kind, xpub))
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
        let (kind, _xpub) = super::parse_xpub(&xpub)?;
        assert!(matches!(kind, super::XpubKind::LegacyP2pkh));
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
        let (kind, _xpub) = super::parse_xpub(&zpub)?;
        assert!(matches!(kind, super::XpubKind::NativeSegwitP2wpkh));
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
        let (kind, _xpub) = super::parse_xpub(&ypub)?;
        assert!(matches!(kind, super::XpubKind::NestedSegwitP2shwpkh));
        Ok(())
    }

    #[test]
    fn parse_unknown_version_rejected() {
        let xpub = fresh_xpub(
            &[10u8; 32],
            &[
                ChildNumber::Hardened { index: 44 },
                ChildNumber::Hardened { index: 0 },
                ChildNumber::Hardened { index: 0 },
            ],
        );
        let bogus = with_version(&xpub, [0xFF, 0xFF, 0xFF, 0xFF]);
        let err = super::parse_xpub(&bogus).unwrap_err();
        assert!(
            format!("{}", err).contains("unknown xpub version magic bytes"),
            "got: {}",
            err
        );
    }
}
