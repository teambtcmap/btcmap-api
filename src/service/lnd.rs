use crate::db;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize)]
struct BlockchainBalanceResponse {
    confirmed_balance: String,
    unconfirmed_balance: String,
    total_balance: String,
}

#[derive(Deserialize)]
struct Amount {
    sat: String,
}

#[derive(Deserialize)]
struct ChannelBalanceResponse {
    local_balance: Amount,
    remote_balance: Amount,
    pending_open_local_balance: Option<Amount>,
    pending_open_remote_balance: Option<Amount>,
}

#[derive(Serialize)]
pub struct NodeStats {
    pub onchain_total_sat: i64,
    pub onchain_confirmed_sat: i64,
    pub onchain_unconfirmed_sat: i64,
    pub outbound_liquidity_sat: i64,
    pub inbound_liquidity_sat: i64,
    pub pending_outbound_liquidity_sat: i64,
    pub pending_inbound_liquidity_sat: i64,
    pub total_balance_sat: i64,
}

pub async fn get_node_stats(pool: &Pool) -> Result<NodeStats> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.lnd_readonly_macaroon.is_empty() {
        Err("lnd readonly macaroon is not set")?
    }
    let client = reqwest::Client::new();
    let blockchain: BlockchainBalanceResponse = get(
        &client,
        "/v1/balance/blockchain",
        &conf.lnd_readonly_macaroon,
    )
    .await?;
    let channels: ChannelBalanceResponse =
        get(&client, "/v1/balance/channels", &conf.lnd_readonly_macaroon).await?;
    let parse = |s: &str| s.parse::<i64>().unwrap_or(0);
    let onchain_total_sat = parse(&blockchain.total_balance);
    let outbound = parse(&channels.local_balance.sat);
    let inbound = parse(&channels.remote_balance.sat);
    let pending_outbound = channels
        .pending_open_local_balance
        .as_ref()
        .map(|a| parse(&a.sat))
        .unwrap_or(0);
    let pending_inbound = channels
        .pending_open_remote_balance
        .as_ref()
        .map(|a| parse(&a.sat))
        .unwrap_or(0);
    Ok(NodeStats {
        onchain_total_sat,
        onchain_confirmed_sat: parse(&blockchain.confirmed_balance),
        onchain_unconfirmed_sat: parse(&blockchain.unconfirmed_balance),
        outbound_liquidity_sat: outbound,
        inbound_liquidity_sat: inbound,
        pending_outbound_liquidity_sat: pending_outbound,
        pending_inbound_liquidity_sat: pending_inbound,
        total_balance_sat: onchain_total_sat + outbound + pending_outbound,
    })
}

async fn get<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    path: &str,
    macaroon: &str,
) -> Result<T> {
    let url = format!("https://lnd.btcmap.org{path}");
    let response = client
        .get(&url)
        .header("Grpc-Metadata-macaroon", macaroon)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("lnd {path} returned {status}: {body}").into());
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse lnd {path} response ({e}): {body}").into())
}
