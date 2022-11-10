use crate::model::user;
use crate::model::User;
use crate::Connection;
use rusqlite::named_params;
use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

pub async fn sync(db: Connection) {
    log::info!("Syncing users");

    let users: Vec<User> = db
        .prepare(user::SELECT_ALL)
        .unwrap()
        .query_map([], user::SELECT_ALL_MAPPER)
        .unwrap()
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    log::info!("Found {} cached users", users.len());

    for db_user in &users {
        let url = format!(
            "https://api.openstreetmap.org/api/0.6/user/{}.json",
            db_user.id,
        );
        log::info!("Querying {url}");
        let res = reqwest::get(&url).await;
        if let Err(_) = res {
            log::error!("Failed to fetch user {}", db_user.id);
            continue;
        }
        let body = res.unwrap().text().await;
        if let Err(_) = body {
            log::error!("Failed to fetch user {}", db_user.id);
            continue;
        }
        let body: serde_json::Result<Value> = serde_json::from_str(&body.unwrap());
        if let Err(_) = body {
            log::error!("Failed to fetch user {}", db_user.id);
            continue;
        }
        let body = body.unwrap();
        let fresh_user: Option<&Value> = body.get("user");
        if fresh_user.is_none() {
            log::error!("Failed to fetch user {}", db_user.id);
            continue;
        }
        let db_user_str = serde_json::to_string(&db_user.osm_json).unwrap();
        let fresh_user_str = serde_json::to_string(&fresh_user.unwrap()).unwrap();
        if fresh_user_str != db_user_str {
            log::info!("Change detected");

            db.execute(
                user::UPDATE_OSM_JSON,
                named_params! {
                    ":id": db_user.id,
                    ":osm_json": fresh_user_str,
                },
            )
            .unwrap();
        }

        sleep(Duration::from_millis(5000)).await;
    }
}
