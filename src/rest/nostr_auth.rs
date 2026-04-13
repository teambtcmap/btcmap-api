// See the note in `src/service/nip98.rs` — this extractor is infrastructure
// ahead of the endpoints that will consume it in a follow-up PR.
#![allow(dead_code)]

use actix_web::web::Data;
use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use std::future::Future;
use std::pin::Pin;

use crate::service::nip98;

/// Trusted external base URL of the API (e.g. `https://api.btcmap.org`, or
/// `http://localhost:8000` in dev). Must be injected via `app_data` by
/// `main.rs`. The extractor uses this, not the `Host`/`X-Forwarded-*`
/// headers, to reconstruct the URL the signed NIP-98 event is expected to
/// bind to. Trusting the request headers here would allow an attacker who
/// had tricked a user into signing a bogus-host URL to replay the event
/// against the real server by spoofing the `Host` header.
#[derive(Clone)]
pub struct ApiBaseUrl(pub String);

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
        let base_url = req.app_data::<Data<ApiBaseUrl>>().cloned();
        Box::pin(async move {
            // Without a trusted base URL we can't safely verify the `u` tag,
            // so fail closed (matches the `Auth` extractor's pattern of
            // returning None-auth when state is missing).
            let Some(base_url) = base_url else {
                return Ok(NostrAuth { npub: None });
            };

            let Some(payload) = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .and_then(nip98::extract_nostr_auth)
            else {
                return Ok(NostrAuth { npub: None });
            };

            // Base URL comes from config, never from request headers — path
            // and query are the only request-derived pieces. An attacker who
            // spoofs `Host` or `X-Forwarded-*` cannot influence what the
            // signature is checked against.
            let path_and_query = req
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or(req.uri().path());
            let full_url = format!("{}{}", base_url.0.trim_end_matches('/'), path_and_query);
            let method = req.method().as_str();

            match nip98::verify(payload, &full_url, method) {
                Ok(event) => Ok(NostrAuth {
                    npub: Some(event.npub),
                }),
                Err(e) => {
                    tracing::debug!(
                        error = %e,
                        url = %full_url,
                        method = %method,
                        "NIP-98 verification failed"
                    );
                    Ok(NostrAuth { npub: None })
                }
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

    const TEST_BASE: &str = "https://api.example/test";

    async fn handler(auth: NostrAuth) -> HttpResponse {
        match auth.npub {
            Some(npub) => HttpResponse::Ok().body(npub),
            None => HttpResponse::Unauthorized().finish(),
        }
    }

    fn app_with_base() -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        App::new()
            .app_data(Data::new(ApiBaseUrl(TEST_BASE.to_string())))
            .route("/auth", web::post().to(handler))
            .route("/auth", web::get().to(handler))
            .route("/", web::post().to(handler))
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
        let app = test::init_service(app_with_base()).await;
        let req = TestRequest::post().uri("/").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }

    #[test]
    async fn bearer_header_yields_none() {
        let app = test::init_service(app_with_base()).await;
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
        let app = test::init_service(app_with_base()).await;
        let signed = signed_nip98(&keys, &format!("{TEST_BASE}/auth"), "POST");
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
        let app = test::init_service(app_with_base()).await;
        let signed = signed_nip98(&keys, &format!("{TEST_BASE}/auth"), "POST");
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
        let app = test::init_service(app_with_base()).await;
        // Event signs a different path than the request is for.
        let signed = signed_nip98(&keys, &format!("{TEST_BASE}/different"), "POST");
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
        let app = test::init_service(app_with_base()).await;
        // Event signs POST, request is GET — should fail.
        let signed = signed_nip98(&keys, &format!("{TEST_BASE}/auth"), "POST");
        let req = TestRequest::get()
            .uri("/auth")
            .insert_header((header::AUTHORIZATION, format!("Nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }

    #[test]
    async fn spoofed_host_header_is_ignored() {
        // Attacker signs an event for "http://evil.example/auth" and replays
        // it to the real server, sending a spoofed Host header. Because the
        // extractor pins the base URL from app_data (not from connection
        // info), the `u` tag in the event will not match, and auth must fail.
        let keys = Keys::generate();
        let app = test::init_service(app_with_base()).await;
        let signed = signed_nip98(&keys, "http://evil.example/auth", "POST");
        let req = TestRequest::post()
            .uri("/auth")
            .insert_header((header::HOST, "evil.example"))
            .insert_header((header::AUTHORIZATION, format!("Nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }

    #[test]
    async fn no_base_url_configured_yields_none() {
        let keys = Keys::generate();
        // Intentionally omit Data::new(ApiBaseUrl(...)). Auth must fail
        // closed: if main.rs forgets to inject the config, we do not fall
        // back to reconstructing from headers.
        let app = test::init_service(App::new().route("/auth", web::post().to(handler))).await;
        let signed = signed_nip98(&keys, "https://api.example/test/auth", "POST");
        let req = TestRequest::post()
            .uri("/auth")
            .insert_header((header::AUTHORIZATION, format!("Nostr {signed}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 401);
    }
}
