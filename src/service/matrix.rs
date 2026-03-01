use crate::{
    db::{self},
    error::Error,
};
use deadpool_sqlite::Pool;
use matrix_sdk::{
    config::SyncSettings, ruma::events::room::message::RoomMessageEventContent, Client,
};
use tokio::sync::OnceCell;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

pub static ROOM_PLACE_COMMENTS: &str = "!yWWvFhceozjhXmtksv:matrix.org";
pub static ROOM_PLACE_IMPORT: &str = "!EpPJoiZzeXiZkclPEg:matrix.org";
pub static ROOM_INFRASTRUCTURE: &str = "!EszQsHUXXrNXOsNCQM:matrix.org";
pub static ROOM_OSM_CHANGES: &str = "!swamvAOpEsGUAzjkeX:matrix.org";
pub static ROOM_PLACE_BOOSTS: &str = "!udVXJdCMPiTMcczvgY:matrix.org";

static MATRIX_CLIENT: OnceCell<Option<Client>> = OnceCell::const_new();

pub fn init(pool: &Pool) {
    let pool = pool.clone();
    tokio::spawn(async move {
        MATRIX_CLIENT
            .get_or_init(|| async {
                info!("initializing matrix...");
                match _init(&pool).await {
                    Some(client) => {
                        info!("matrix client initialized successfully");
                        Some(client)
                    }
                    None => {
                        warn!("failed to initialize matrix client");
                        None
                    }
                }
            })
            .await
            .clone()
    });
}

async fn _init(pool: &Pool) -> Option<Client> {
    let Ok(conf) = db::main::conf::queries::select(pool).await else {
        error!("failed to load configuration");
        return None;
    };
    if conf.matrix_bot_password.is_empty() {
        warn!("matrix bot password is not set, disabling matrix integration");
        return None;
    }
    info!("creating matrix client");
    let client = Client::builder()
        .homeserver_url("https://matrix.org")
        .build()
        .await;
    let client = match client {
        Ok(client) => client,
        Err(e) => {
            warn!("failed to create matrix client: {}", e);
            return None;
        }
    };
    info!("matrix client created");
    info!("logging in");
    let auth_res = client
        .matrix_auth()
        .login_username("btcmapbot", "P5TMrT9cetM3YM")
        .send()
        .await;
    match auth_res {
        Ok(_) => info!("logged in"),
        Err(e) => {
            warn!("matrix auth failure: {}", e);
            return None;
        }
    };
    let mut retry_delay = Duration::from_secs(30);
    for attempt in 1..=10 {
        let sync_res = client.sync_once(SyncSettings::default()).await;
        match sync_res {
            Ok(_) => {
                info!("sync complete");
                return Some(client);
            }
            Err(e) => {
                if attempt < 10 {
                    warn!(
                        "matrix sync failure (attempt {}/10): {}, retrying in {:?}",
                        attempt, e, retry_delay
                    );
                    sleep(retry_delay).await;
                    retry_delay *= 2;
                } else {
                    warn!("matrix sync failure (attempt {}/10): {}", attempt, e);
                    return None;
                }
            }
        };
    }
    None
}

pub fn try_client(_pool: &Pool) -> Option<Client> {
    MATRIX_CLIENT.get().and_then(|c| c.clone())
}

pub fn send_message(client: &Option<Client>, room_id: &str, message: &str) {
    let room_id = match room_id {
        "place-comments" => ROOM_PLACE_COMMENTS,
        "place-import" => ROOM_PLACE_IMPORT,
        "infra" | "infrastructure" => ROOM_INFRASTRUCTURE,
        _ => room_id,
    };

    let Some(client) = client else {
        warn!("matrix client not configured");
        return;
    };

    let client = client.clone();
    let room_id = room_id.to_string();
    let message = message.to_string();

    actix_web::rt::spawn(async move {
        if let Err(e) = _send_message(&client, &room_id, &message).await {
            warn!(room_id, error = %e);
        }
    });
}

async fn _send_message(client: &Client, room_id: &str, message: &str) -> Result<(), Error> {
    let rooms = client.joined_rooms();
    let room = rooms
        .into_iter()
        .find(|r| r.room_id() == room_id)
        .ok_or_else(|| Error::Matrix("room not found".to_string()))?;
    let content = RoomMessageEventContent::text_plain(message);
    room.send(content)
        .await
        .map_err(|_| Error::Matrix("failed to send message".to_string()))?;
    Ok(())
}
