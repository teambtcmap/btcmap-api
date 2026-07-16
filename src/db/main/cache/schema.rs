pub const TABLE_NAME: &str = "cache";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Key,
    Value,
}
