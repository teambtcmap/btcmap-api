use crate::db;
use crate::db::main::user::schema::User;
use crate::service::nip98;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub nostr_event: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct Res {
    pub npub: String,
}

pub async fn run(params: Params, user: &User, pool: &Pool) -> Result<Res> {
    let verified = nip98::verify(&params.nostr_event, &params.url, "POST")
        .map_err(|e| format!("NIP-98 verification failed: {e}"))?;

    // Check if user already has a linked Nostr identity
    if user.npub.is_some() {
        return Err(
            "User already has a linked Nostr identity. Use update_nostr_identity to change it."
                .into(),
        );
    }

    // Check if this pubkey is already linked to another account
    let existing = db::main::user::queries::select_by_npub(&verified.pubkey, pool).await?;
    if existing.is_some() {
        return Err("This Nostr pubkey is already linked to another account".into());
    }

    db::main::user::queries::set_npub(user.id, Some(verified.pubkey.clone()), pool).await?;

    Ok(Res {
        npub: verified.pubkey,
    })
}
