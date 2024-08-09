use crate::{
    area::AreaRepo, auth::AuthService, command::db, element::ElementRepo, event::model::EventRepo,
    report::model::ReportRepo, user::UserRepo,
};
use deadpool_sqlite::{Config, Pool, Runtime};
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

pub fn mock_conn() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    db::migrate(&mut conn).unwrap();
    conn
}

static MEM_DB_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub async fn mock_state() -> State {
    let mut db = mock_db();
    let pool = Arc::new(db.1);
    db::migrate(&mut db.0).unwrap();
    State {
        conn: db.0,
        pool: pool.clone(),
        auth: AuthService::new(&pool),
        area_repo: AreaRepo::new(&pool),
        element_repo: ElementRepo::new(&pool),
        event_repo: EventRepo::new(&pool),
        report_repo: ReportRepo::new(&pool),
        user_repo: UserRepo::new(&pool),
    }
}

pub fn mock_db() -> (Connection, Pool) {
    let uri = format!(
        "file::testdb_{}:?mode=memory&cache=shared",
        MEM_DB_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let conn = Connection::open(uri.clone()).unwrap();
    (
        conn,
        Config::new(uri)
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(8)
            .build()
            .unwrap(),
    )
}

pub struct State {
    pub conn: Connection,
    pub pool: Arc<Pool>,
    pub auth: AuthService,
    pub area_repo: AreaRepo,
    pub element_repo: ElementRepo,
    pub event_repo: EventRepo,
    pub report_repo: ReportRepo,
    pub user_repo: UserRepo,
}

pub fn mock_tags() -> Map<String, Value> {
    let mut tags = Map::new();
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

pub fn phuket_geo_json() -> Value {
    json!(
        {
            "type": "FeatureCollection",
            "features": [
              {
                "type": "Feature",
                "properties": {},
                "geometry": {
                  "coordinates": [
                    [
                      [
                        98.2181205776469,
                        8.20412838698085
                      ],
                      [
                        98.2181205776469,
                        7.74024270965898
                      ],
                      [
                        98.4806081271079,
                        7.74024270965898
                      ],
                      [
                        98.4806081271079,
                        8.20412838698085
                      ],
                      [
                        98.2181205776469,
                        8.20412838698085
                      ]
                    ]
                  ],
                  "type": "Polygon"
                }
              }
            ]
          }
    )
}
