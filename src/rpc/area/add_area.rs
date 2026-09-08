use crate::{
    db::{main::area::schema::Area, main::user::schema::User},
    service, Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub tags: Map<String, Value>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Area> for Res {
    fn from(val: Area) -> Self {
        Res {
            id: val.id,
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, user: &User, pool: &Pool) -> Result<Res> {
    if !user.geofence.is_empty() {
        return Err(format!(
            "Cannot add new areas when your geofence is set (allowed areas: {:?}); ask an admin to create the area",
            user.geofence
        )
        .into());
    }
    service::area::insert(params.tags, pool)
        .await
        .map(Into::into)
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::{
        db::main::{
            test::pool,
            user::schema::{Role, User},
        },
        Result,
    };
    use serde_json::{json, Map};

    const SQUARE: &str = r#"{
        "type":"Feature",
        "properties":{},
        "geometry":{
            "type":"Polygon",
            "coordinates":[[
                [98.30, 7.95],
                [98.30, 7.85],
                [98.40, 7.85],
                [98.40, 7.95],
                [98.30, 7.95]
            ]]
        }
    }"#;

    fn am_user(geofence: Vec<i64>) -> User {
        User {
            id: 1,
            name: "am".into(),
            password: String::new(),
            roles: vec![Role::AreaManager],
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            geofence,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    fn params() -> super::Params {
        let mut tags = Map::new();
        tags.insert("geo_json".into(), serde_json::from_str(SQUARE).unwrap());
        tags.insert("url_alias".into(), json!("test-area"));
        tags.insert("name".into(), json!("test-area"));
        super::Params { tags }
    }

    #[test]
    fn area_manager_with_empty_geofence_can_add_anywhere() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let user = am_user(vec![]);
            let res = run(params(), &user, &pool).await?;
            assert!(res.id > 0);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn area_manager_with_geofence_is_rejected() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let user = am_user(vec![1, 2, 3]);
            let err = match run(params(), &user, &pool).await {
                Ok(_) => panic!("expected geofence violation"),
                Err(e) => e,
            };
            assert!(err.to_string().contains("geofence"));
            Ok::<(), crate::Error>(())
        })
    }
}
