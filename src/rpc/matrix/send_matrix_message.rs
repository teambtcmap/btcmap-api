use crate::{service, service::matrix};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    room_id: String,
    message: String,
}

pub async fn run(params: Params, pool: &Pool) {
    let matrix_client = matrix::try_client(pool);
    service::matrix::send_message(&matrix_client, &params.room_id, &params.message);
}
