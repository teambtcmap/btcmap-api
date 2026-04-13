use crate::db;
use crate::service::nip98;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Serialize)]
pub struct Res {
    pub token: String,
    pub time_ms: i128,
}

pub async fn run(nostr_auth_payload: &str, expected_url: &str, pool: &Pool) -> Result<Res> {
    let start_time = OffsetDateTime::now_utc();

    let verified = nip98::verify(nostr_auth_payload, expected_url, "POST")
        .map_err(|e| format!("NIP-98 verification failed: {e}"))?;

    let user = db::main::user::queries::select_by_npub(&verified.pubkey, pool)
        .await?
        .ok_or("No BTC Map account linked to this Nostr pubkey")?;

    let token = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(user.id, String::new(), token.clone(), vec![], pool)
        .await?;

    let time_passed_ms = (OffsetDateTime::now_utc() - start_time).whole_milliseconds();
    Ok(Res {
        token,
        time_ms: time_passed_ms,
    })
}
