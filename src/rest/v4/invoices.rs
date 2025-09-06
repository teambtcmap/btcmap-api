use crate::db;
use crate::db::invoice::schema::InvoiceStatus;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use deadpool_sqlite::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct GetByIdRes {
    uuid: String,
    status: String,
}

#[get("{uuid}")]
pub async fn get_by_uuid(uuid: Path<String>, pool: Data<Pool>) -> Res<GetByIdRes> {
    let mut invoice = db::invoice::queries::select_by_uuid(uuid.as_str(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    if invoice.status == InvoiceStatus::Unpaid
        && crate::service::invoice::sync_unpaid_invoice(&invoice, &pool)
            .await
            .map_err(|_| RestApiError::database())?
    {
        invoice = db::invoice::queries::select_by_uuid(uuid.as_str(), &pool)
            .await
            .map_err(|e| match e {
                Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
                _ => RestApiError::database(),
            })?;
    }

    Ok(Json(GetByIdRes {
        uuid: invoice.uuid,
        status: invoice.status.into(),
    }))
}
