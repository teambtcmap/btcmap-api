use crate::service;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TxSummary {
    pub id: String,
    pub received: i64,
    pub sent: i64,
    pub delta: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Res {
    pub spending: i64,
    pub donations: i64,
    pub treasury: i64,
    pub spending_tx: Vec<TxSummary>,
    pub donations_tx: Vec<TxSummary>,
    pub treasury_tx: Vec<TxSummary>,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let wallet = service::wallet::run(pool).await?;
    Ok(Res {
        spending: wallet.spending,
        donations: wallet.donations,
        treasury: wallet.treasury,
        spending_tx: wallet
            .spending_tx
            .into_iter()
            .map(|t| TxSummary {
                id: t.id,
                received: t.received,
                sent: t.sent,
                delta: t.delta,
            })
            .collect(),
        donations_tx: wallet
            .donations_tx
            .into_iter()
            .map(|t| TxSummary {
                id: t.id,
                received: t.received,
                sent: t.sent,
                delta: t.delta,
            })
            .collect(),
        treasury_tx: wallet
            .treasury_tx
            .into_iter()
            .map(|t| TxSummary {
                id: t.id,
                received: t.received,
                sent: t.sent,
                delta: t.delta,
            })
            .collect(),
    })
}

#[cfg(test)]
mod test {
    use crate::{
        db::{
            self,
            image::test::pool as image_pool,
            log::test::pool as log_pool,
            main::{access_token::queries::insert as insert_token, test::pool, user::schema::Role},
        },
        rpc::{
            get_wallets::Res,
            handler::{handle, handle_rpc_error},
        },
    };
    use actix_web::{
        http::header,
        middleware::ErrorHandlers,
        test,
        web::{scope, Data},
        App,
    };
    use deadpool_sqlite::Pool;
    use serde_json::{json, Value};

    async fn issue_token(name: &str, secret: &str, roles: Vec<Role>, pool: &Pool) {
        let user = db::main::user::queries::insert(name, "", pool)
            .await
            .unwrap();
        insert_token(user.id, "".into(), secret.into(), roles, pool)
            .await
            .unwrap();
    }

    #[test]
    async fn rejects_unauthenticated_call() {
        let pool = pool();
        let log_pool = log_pool();
        let image_pool = image_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(log_pool))
                .app_data(Data::new(image_pool))
                .wrap(ErrorHandlers::new().default_handler(handle_rpc_error))
                .service(scope("/").service(handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "get_wallets",
                "id": 1
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        let err = res.get("error").expect("expected error response");
        assert_eq!(err["message"], "Auth header is missing");
    }

    #[test]
    async fn rejects_non_admin_user() {
        let pool = pool();
        issue_token("alice", "alice-secret", vec![Role::User], &pool).await;
        let log_pool = log_pool();
        let image_pool = image_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(log_pool))
                .app_data(Data::new(image_pool))
                .wrap(ErrorHandlers::new().default_handler(handle_rpc_error))
                .service(scope("/").service(handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer alice-secret"))
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "get_wallets",
                "id": 1
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        let err = res.get("error").expect("expected error response");
        assert_eq!(
            err["message"],
            "You don't have permissions to call this method"
        );
    }

    #[test]
    async fn admin_gets_zero_balances_with_no_xpubs() {
        let pool = pool();
        issue_token("bob", "bob-secret", vec![Role::Admin], &pool).await;
        let log_pool = log_pool();
        let image_pool = image_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(log_pool))
                .app_data(Data::new(image_pool))
                .wrap(ErrorHandlers::new().default_handler(handle_rpc_error))
                .service(scope("/").service(handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer bob-secret"))
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "get_wallets",
                "id": 1
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert!(res.get("error").is_none(), "admin should be allowed");
        let body: Res = serde_json::from_value(res["result"].clone()).unwrap();
        assert_eq!(body.spending, 0);
        assert_eq!(body.donations, 0);
        assert_eq!(body.treasury, 0);
    }

    #[test]
    async fn root_can_call() {
        let pool = pool();
        issue_token("root", "root-secret", vec![Role::Root], &pool).await;
        let log_pool = log_pool();
        let image_pool = image_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(log_pool))
                .app_data(Data::new(image_pool))
                .wrap(ErrorHandlers::new().default_handler(handle_rpc_error))
                .service(scope("/").service(handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer root-secret"))
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "get_wallets",
                "id": 1
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        if let Some(err) = res.get("error") {
            panic!("root should be allowed, got error: {}", err);
        }
        assert_eq!(res["result"]["spending"], 0);
        assert_eq!(res["result"]["donations"], 0);
        assert_eq!(res["result"]["treasury"], 0);
    }
}
