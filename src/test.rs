use crate::db_utils;
use deadpool_sqlite::{Config, Hook, Pool, Runtime};
use serde_json::{json, Map, Value};

pub fn mock_pool() -> Pool {
    Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let mut conn = conn.lock().unwrap();
            db_utils::migrate(&mut conn).unwrap();
            Ok(())
        })))
        .build()
        .unwrap()
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
