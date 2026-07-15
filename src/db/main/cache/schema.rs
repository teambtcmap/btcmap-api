pub const TABLE_NAME: &str = "cache";

pub enum Columns {
    Key,
    Value,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Key => "key",
            Columns::Value => "value",
        }
    }
}
