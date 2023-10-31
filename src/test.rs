use std::collections::HashMap;

use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::command::db;

pub fn mock_conn() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    db::migrate(&mut conn).unwrap();
    conn
}

pub fn mock_tags() -> HashMap<String, Value> {
    let mut tags = HashMap::new();
    tags.insert("null".into(), Value::Null);
    tags.insert("bool".into(), Value::Bool(true));
    tags.insert("number".into(), Value::Number(1.into()));
    tags.insert("string".into(), Value::String("test".into()));
    tags.insert("array".into(), Value::Array(vec![]));
    tags.insert("object".into(), Value::Object(Map::new()));
    tags
}