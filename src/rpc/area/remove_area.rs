use crate::db::main::{area::schema::Area, user::schema::User};
use crate::{service, Result};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: JsonObject,
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
    service::area::check_geofence(user, &params.id, pool).await?;
    service::area::soft_delete_async(params.id, pool)
        .await
        .map(Into::into)
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::{
        db::{
            self,
            main::test::pool,
            main::user::schema::{Role, User},
        },
        Result,
    };
    use serde_json::{json, Map};

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

    async fn seed_area(pool: &deadpool_sqlite::Pool, alias: &str) -> Result<i64> {
        let mut tags = Map::new();
        tags.insert("geo_json".into(), json!({"type":"Polygon","coordinates":[[[0.0,0.0],[0.0,1.0],[1.0,1.0],[1.0,0.0],[0.0,0.0]]]}));
        tags.insert("url_alias".into(), json!(alias));
        tags.insert("name".into(), json!(alias));
        Ok(db::main::area::queries::insert(tags, pool).await?.id)
    }

    #[test]
    fn area_manager_with_empty_geofence_can_delete_anywhere() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let area_id = seed_area(&pool, "anywhere").await?;
            let user = am_user(vec![]);
            let res = run(
                super::Params {
                    id: area_id.to_string(),
                },
                &user,
                &pool,
            )
            .await?;
            assert_eq!(res.id, area_id);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn area_manager_with_geofence_can_delete_fenced_area() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let area_id = seed_area(&pool, "fenced").await?;
            let user = am_user(vec![area_id]);
            let res = run(
                super::Params {
                    id: area_id.to_string(),
                },
                &user,
                &pool,
            )
            .await?;
            assert_eq!(res.id, area_id);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn area_manager_with_geofence_cannot_delete_unfenced_area() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let fenced = seed_area(&pool, "fenced").await?;
            let other = seed_area(&pool, "other").await?;
            let user = am_user(vec![fenced]);
            let err = match run(
                super::Params {
                    id: other.to_string(),
                },
                &user,
                &pool,
            )
            .await
            {
                Ok(_) => panic!("expected geofence violation"),
                Err(e) => e,
            };
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }
}
