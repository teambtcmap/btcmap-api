use crate::db;
use crate::db::main::access_token::schema::AccessToken;
use crate::db::main::user::schema::User;
use crate::db::main::MainPool;
use actix_web::{dev::Payload, http::header, web::Data, FromRequest, HttpRequest};
use std::future::Future;
use std::pin::Pin;

pub struct Auth {
    pub user: Option<User>,
    pub token: Option<AccessToken>,
}

impl Auth {
    pub fn user(&self) -> Option<&User> {
        self.user.as_ref()
    }

    pub fn token(&self) -> Option<&AccessToken> {
        self.token.as_ref()
    }
}

impl From<Auth> for Option<User> {
    fn from(ext: Auth) -> Self {
        ext.user
    }
}

impl FromRequest for Auth {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let pool = req.app_data::<Data<MainPool>>().cloned();

        Box::pin(async move {
            let Some(pool) = pool else {
                return Ok(Auth {
                    user: None,
                    token: None,
                });
            };

            let bearer_token = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.strip_prefix("Bearer "))
                .map(String::from);

            let Some(token) = bearer_token else {
                return Ok(Auth {
                    user: None,
                    token: None,
                });
            };

            let Ok(access_token) =
                db::main::access_token::queries::select_by_secret(token, &pool).await
            else {
                return Ok(Auth {
                    user: None,
                    token: None,
                });
            };

            let Ok(user) = db::main::user::queries::select_by_id(access_token.user_id, &pool).await
            else {
                return Ok(Auth {
                    user: None,
                    token: None,
                });
            };

            Ok(Auth {
                user: Some(user),
                token: Some(access_token),
            })
        })
    }
}
