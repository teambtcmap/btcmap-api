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

    // User must already have a linked Nostr identity to update it
    if user.npub.is_none() {
        return Err(
            "No Nostr identity linked to this account. Use link_nostr_identity first.".into(),
        );
    }

    // Check if the new pubkey is already linked to another account
    let existing = db::main::user::queries::select_by_npub(&verified.pubkey, pool).await?;
    if let Some(existing_user) = existing {
        if existing_user.id != user.id {
            return Err("This Nostr pubkey is already linked to another account".into());
        }
    }

    db::main::user::queries::set_npub(user.id, Some(verified.pubkey.clone()), pool).await?;

    Ok(Res {
        npub: verified.pubkey,
    })
}
