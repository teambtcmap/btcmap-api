use crate::{
    db::{self, main::user::schema::User},
    service::timezone::{self, EventTime},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    area_id: Option<i64>,
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    starts_at: EventTime,
    #[serde(default)]
    ends_at: Option<EventTime>,
    /// Either "auto" (infer from lat/lon) or an IANA zone name. Only needed for
    /// floating timestamps that have no UTC offset.
    #[serde(default)]
    timezone: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    ends_at: Option<OffsetDateTime>,
    /// The timezone that was applied to floating timestamps, echoed back so the
    /// caller can verify what the server inferred.
    timezone: Option<String>,
}

pub async fn run(params: Params, user: &User, pool: &Pool) -> Result<Res> {
    super::geofence::check(user, params.lat, params.lon, pool).await?;
    let (starts_at, ends_at, timezone) = timezone::resolve_create_times(
        params.starts_at,
        params.ends_at,
        params.timezone.as_deref(),
        params.lat,
        params.lon,
    )?;
    let event = db::main::event::queries::insert(
        params.area_id,
        params.lat,
        params.lon,
        params.name,
        params.website,
        starts_at,
        ends_at,
        pool,
    )
    .await?;
    Ok(Res {
        id: event.id,
        starts_at: event.starts_at,
        ends_at: event.ends_at,
        timezone,
    })
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::{
        db,
        db::main::{
            area::schema::Area,
            test::pool,
            user::schema::{Role, User},
        },
        Result,
    };
    use serde_json::{json, Map};
    use time::macros::datetime;

    const PHUKET: &str = r#"{
        "type":"Feature",
        "properties":{},
        "geometry":{
            "type":"Polygon",
            "coordinates":[[
                [98.2181205776469, 8.20412838698085],
                [98.2181205776469, 7.74024270965898],
                [98.4806081271079, 7.74024270965898],
                [98.4806081271079, 8.20412838698085],
                [98.2181205776469, 8.20412838698085]
            ]]
        }
    }"#;

    async fn insert_area(
        name: &str,
        geo_json: serde_json::Value,
        pool: &deadpool_sqlite::Pool,
    ) -> Result<Area> {
        let mut tags = Map::new();
        tags.insert("name".into(), json!(name));
        tags.insert("geo_json".into(), geo_json);
        tags.insert("url_alias".into(), json!(name));
        db::main::area::queries::insert(tags, pool).await
    }

    fn em_user(geofence: Vec<i64>) -> User {
        User {
            id: 1,
            name: "em".into(),
            password: String::new(),
            roles: vec![Role::EventManager],
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            geofence,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    fn admin_user() -> User {
        User {
            id: 2,
            name: "admin".into(),
            password: String::new(),
            roles: vec![Role::Admin],
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            geofence: vec![],
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    fn params(area_id: Option<i64>, lat: f64, lon: f64) -> super::Params {
        super::Params {
            area_id,
            lat,
            lon,
            name: "Bitcoin meetup".into(),
            website: "https://example.com".into(),
            starts_at: crate::service::timezone::EventTime::Absolute(datetime!(
                2999-01-01 0:00 UTC
            )),
            ends_at: None,
            timezone: None,
        }
    }

    #[test]
    fn requires_starts_at() {
        let v = json!({
            "lat": 1.0,
            "lon": 2.0,
            "name": "Bitcoin meetup",
            "website": "https://example.com",
        });
        assert!(serde_json::from_value::<super::Params>(v).is_err());
    }

    #[test]
    fn event_manager_inside_geofence_can_create() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let user = em_user(vec![phuket.id]);
            let res = run(params(Some(phuket.id), 7.98, 98.33), &user, &pool).await?;
            assert!(res.id > 0);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_outside_geofence_blocked() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let user = em_user(vec![phuket.id]);
            // London lat/lon, outside Phuket polygon
            let err = match run(params(None, 51.5, -0.1), &user, &pool).await {
                Ok(_) => panic!("expected geofence violation"),
                Err(e) => e,
            };
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn admin_user_bypasses_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let user = admin_user();
            let res = run(params(None, 51.5, -0.1), &user, &pool).await?;
            assert!(res.id > 0);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_empty_geofence_can_create_anywhere() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let user = em_user(vec![]);
            let res = run(params(None, 51.5, -0.1), &user, &pool).await?;
            assert!(res.id > 0);
            Ok::<(), crate::Error>(())
        })
    }
}
