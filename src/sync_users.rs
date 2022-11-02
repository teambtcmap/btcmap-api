use crate::db;
use crate::model::User;
use crate::Connection;
use rusqlite::params;
use serde_json::Value;

pub async fn sync(db_conn: Connection) {
    log::info!("Syncing users");

    let users: Vec<User> = db_conn
        .prepare(db::USER_SELECT_ALL)
        .unwrap()
        .query_map([], db::mapper_user_full())
        .unwrap()
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    log::info!("Found {} cached users", users.len());

    for db_user in users {
        let url = format!(
            "https://api.openstreetmap.org/api/0.6/user/{}.json",
            db_user.id
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

            db_conn
                .execute(
                    "UPDATE user SET osm_json = ? WHERE id = ?",
                    params![fresh_user_str, db_user.id],
                )
                .unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
    }
}
