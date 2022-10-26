use actix_web::body::EitherBody;
use actix_web::error::JsonPayloadError;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use serde::Serialize;

#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T: Serialize> Responder for Json<T> {
    type Body = EitherBody<String>;

    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        match serde_json::to_string_pretty(&self.0) {
            Ok(body) => match HttpResponse::Ok()
                .content_type("application/json")
                .message_body(body + "\n")
            {
                Ok(res) => res.map_into_left_body(),
                Err(err) => HttpResponse::from_error(err).map_into_right_body(),
            },

            Err(err) => {
                HttpResponse::from_error(JsonPayloadError::Serialize(err)).map_into_right_body()
            }
        }
    }
}
