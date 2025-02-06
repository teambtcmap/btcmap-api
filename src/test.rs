use crate::db;
use deadpool_sqlite::{Config, Pool, Runtime};
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicUsize, Ordering};

static MEM_DB_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub struct Database {
    pub conn: Connection,
    pub pool: Pool,
}

pub async fn mock_db() -> Database {
    let mut db = _mock_db();
    db::migrate(&mut db.0).unwrap();
    Database {
        conn: db.0,
        pool: db.1,
    }
}

pub fn mock_conn() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    db::migrate(&mut conn).unwrap();
    conn
}

fn _mock_db() -> (Connection, Pool) {
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

pub fn mock_osm_tags(kv_pairs: &[&str]) -> Map<String, Value> {
    let mut res = Map::new();
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

pub fn earth_geo_json() -> Value {
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
                        -180,
                        -90
                      ],
                      [
                        -180,
                        90
                      ],
                      [
                        180,
                        90
                      ],
                      [
                        180,
                        -90
                      ],
                      [
                        -180,
                        -90
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
