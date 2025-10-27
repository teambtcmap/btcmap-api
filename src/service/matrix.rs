use crate::db::conf::schema::Conf;
use matrix_sdk::{
    config::SyncSettings, ruma::events::room::message::RoomMessageEventContent, Client,
};
use tracing::{info, warn};

pub static ROOM_PLACE_COMMENTS: &str = "!yWWvFhceozjhXmtksv:matrix.org";
pub static ROOM_PLACE_IMPORT: &str = "!EpPJoiZzeXiZkclPEg:matrix.org";

pub async fn init_client(conf: &Conf) -> Option<Client> {
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
    let sync_res = client.sync_once(SyncSettings::default()).await;
    match sync_res {
        Ok(_) => info!("sync complete"),
        Err(e) => {
            warn!("matrix sync failure: {}", e);
            return None;
        }
    };
    Some(client)
}

pub async fn send_message(client: &Option<Client>, room_id: &str, message: &str) {
    let Some(client) = client else { return };
    let rooms = client.joined_rooms();
    let room = rooms.into_iter().find(|r| r.room_id() == room_id);
    let Some(room) = room else {
        warn!(room_id, "matrix room not found");
        return;
    };
    let content = RoomMessageEventContent::text_plain(message);
    room.send(content).await.unwrap();
}
