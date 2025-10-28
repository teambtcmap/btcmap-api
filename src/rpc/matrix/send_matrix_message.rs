use crate::service;
use matrix_sdk::Client;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    room_id: String,
    message: String,
}

pub async fn run(params: Params, matrix_client: &Option<Client>) {
    service::matrix::send_message(matrix_client, &params.room_id, &params.message);
}
