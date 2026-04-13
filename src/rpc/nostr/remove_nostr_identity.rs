use crate::db;
use crate::db::main::user::schema::User;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub message: String,
}

pub async fn run(user: &User, pool: &Pool) -> Result<Res> {
    if user.npub.is_none() {
        return Err("No Nostr identity linked to this account".into());
    }

    db::main::user::queries::set_npub(user.id, None, pool).await?;

    Ok(Res {
        message: "Nostr identity removed".into(),
    })
}
