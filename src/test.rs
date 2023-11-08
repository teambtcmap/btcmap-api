use deadpool_sqlite::{Config, Pool, Runtime};
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::{area::AreaRepo, command::db, element::ElementRepo, service::AuthService};

pub fn mock_conn() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    db::migrate(&mut conn).unwrap();
    conn
}

static MEM_DB_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub fn mock_state() -> State {
    let uri = format!(
        "file::testdb_{}:?mode=memory&cache=shared",
        MEM_DB_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let mut conn = Connection::open(&uri).unwrap();
    db::migrate(&mut conn).unwrap();
    let pool = Arc::new(Config::new(uri).create_pool(Runtime::Tokio1).unwrap());
    State {
        pool: pool.clone(),
        conn: conn,
        auth: AuthService::new(&pool),
        area_repo: Arc::new(AreaRepo::new(&pool)),
        element_repo: ElementRepo::new(&pool),
    }
}

pub struct State {
    pub pool: Arc<Pool>,
    pub conn: Connection,
    pub auth: AuthService,
    pub area_repo: Arc<AreaRepo>,
    pub element_repo: ElementRepo,
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

pub fn mock_osm_tags(kv_pairs: &[&str]) -> HashMap<String, String> {
    let mut res = HashMap::new();
    for chunk in kv_pairs.chunks(2) {
        res.insert(chunk[0].into(), chunk[1].into());
    }
    res
}
