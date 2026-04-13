// See the note in `src/service/nip98.rs` — this extractor is infrastructure
// ahead of the endpoints that will consume it in a follow-up PR.
#![allow(dead_code)]

use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use std::future::Future;
use std::pin::Pin;

use crate::service::nip98;

/// NIP-98 extractor. Mirrors the `Auth` bearer extractor: never fails the
/// request, always yields a struct. When `npub` is `None` the handler
/// decides whether to reject (401) or treat auth as optional.
///
/// A `Some(npub)` value guarantees the event was:
/// - base64-decoded + JSON-parsed successfully
/// - kind 27235 with an in-window `created_at`
/// - signed such that the `u` tag matches the actual request URL
/// - signed such that the `method` tag matches the actual request method
/// - verified under a valid Schnorr signature
///
/// The `npub` is bech32-encoded (`npub1...`), matching the encoding used by
/// the `user.npub` DB column.
pub struct NostrAuth {
    pub npub: Option<String>,
}

impl FromRequest for NostrAuth {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let Some(payload) = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .and_then(nip98::extract_nostr_auth)
            else {
                return Ok(NostrAuth { npub: None });
            };

            // The `u` tag must match the URL the client signed against. We
            // reconstruct it from the actual request so a signature for one
            // endpoint can't be replayed on another.
            let full_url = {
                let ci = req.connection_info();
                let path_and_query = req
                    .uri()
                    .path_and_query()
                    .map(|p| p.as_str())
                    .unwrap_or(req.uri().path());
                format!("{}://{}{}", ci.scheme(), ci.host(), path_and_query)
            };
            let method = req.method().as_str();

            match nip98::verify(payload, &full_url, method) {
                Ok(event) => Ok(NostrAuth {
                    npub: Some(event.npub),
                }),
                Err(_) => Ok(NostrAuth { npub: None }),
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test::TestRequest;
    use actix_web::{test, web, App, HttpResponse};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use nostr::event::EventBuilder;
    use nostr::key::Keys;
    use nostr::nips::nip19::ToBech32;
    use nostr::{JsonUtil, Kind, Tag, Timestamp};

    async fn handler(auth: NostrAuth) -> HttpResponse {
        match auth.npub {
            Some(npub) => HttpResponse::Ok().body(npub),
            None => HttpResponse::Unauthorized().finish(),
        }
    }

    fn signed_nip98(keys: &Keys, url: &str, method: &str) -> String {
        let event = EventBuilder::new(Kind::from_u16(27235), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", method]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign_with_keys(keys)
            .unwrap();
        BASE64.encode(event.as_json().as_bytes())
    }

    #[test]
    async fn missing_header_yields_none() {
        let app = test::init_service(App::new().route("/", web::post().to(handler))).await;
        let req = TestRequest::post().uri("/").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }

    #[test]
    async fn bearer_header_yields_none() {
        let app = test::init_service(App::new().route("/", web::post().to(handler))).await;
        let req = TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer something"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }

    #[test]
    async fn valid_header_yields_bech32_npub() {
        let keys = Keys::generate();
        let app = test::init_service(App::new().route("/auth", web::post().to(handler))).await;
        let signed = signed_nip98(&keys, "http://localhost:8080/auth", "POST");
        let req = TestRequest::post()
            .uri("/auth")
            .insert_header((header::AUTHORIZATION, format!("Nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        let body = test::read_body(res).await;
        let npub = std::str::from_utf8(&body).unwrap();
        assert_eq!(npub, keys.public_key().to_bech32().unwrap());
    }

    #[test]
    async fn lowercase_scheme_accepted() {
        let keys = Keys::generate();
        let app = test::init_service(App::new().route("/auth", web::post().to(handler))).await;
        let signed = signed_nip98(&keys, "http://localhost:8080/auth", "POST");
        let req = TestRequest::post()
            .uri("/auth")
            .insert_header((header::AUTHORIZATION, format!("nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
    }

    #[test]
    async fn url_mismatch_yields_none() {
        let keys = Keys::generate();
        let app = test::init_service(App::new().route("/auth", web::post().to(handler))).await;
        // Event signs a different URL than the request is for.
        let signed = signed_nip98(&keys, "http://localhost/different", "POST");
        let req = TestRequest::post()
            .uri("/auth")
            .insert_header((header::AUTHORIZATION, format!("Nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }

    #[test]
    async fn method_derived_from_request() {
        let keys = Keys::generate();
        let app = test::init_service(App::new().route("/auth", web::get().to(handler))).await;
        // Event signs POST, request is GET — should fail.
        let signed = signed_nip98(&keys, "http://localhost:8080/auth", "POST");
        let req = TestRequest::get()
            .uri("/auth")
            .insert_header((header::AUTHORIZATION, format!("Nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }
}
