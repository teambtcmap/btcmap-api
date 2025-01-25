use crate::Result;
use serde::Serialize;

pub const NAME: &str = "paywall_get_add_element_comment_quote";

#[derive(Serialize)]
pub struct Res {
    pub quote_sats: i64,
}

pub async fn run() -> Result<Res> {
    Ok(Res { quote_sats: 500 })
}
