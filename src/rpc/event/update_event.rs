use crate::{
    db::{self, main::event::schema::Event, main::user::schema::User},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod optional_rfc3339 {
    use serde::{de::Error, Deserialize, Deserializer};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Option<OffsetDateTime>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(Some(match opt {
            None => None,
            Some(s) => Some(OffsetDateTime::parse(&s, &Rfc3339).map_err(D::Error::custom)?),
        }))
    }
}

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
    #[serde(default)]
    pub area_id: Option<Option<i64>>,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    website: Option<String>,
    #[serde(default, deserialize_with = "optional_rfc3339::deserialize")]
    starts_at: Option<Option<OffsetDateTime>>,
    #[serde(default, deserialize_with = "optional_rfc3339::deserialize")]
    ends_at: Option<Option<OffsetDateTime>>,
    #[serde(default)]
    cron_schedule: Option<Option<String>>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    ends_at: Option<OffsetDateTime>,
    cron_schedule: Option<String>,
    pub area_id: Option<i64>,
}

impl From<Event> for Res {
    fn from(event: Event) -> Self {
        Res {
            id: event.id,
            lat: event.lat,
            lon: event.lon,
            name: event.name,
            website: event.website,
            starts_at: event.starts_at,
            ends_at: event.ends_at,
            cron_schedule: event.cron_schedule,
            area_id: event.area_id,
        }
    }
}

pub async fn run(params: Params, user: &User, pool: &Pool) -> Result<Res> {
    let event = db::main::event::queries::select_by_id(params.id, pool).await?;
    let area_id = params.area_id.unwrap_or(event.area_id);
    let lat = params.lat.unwrap_or(event.lat);
    let lon = params.lon.unwrap_or(event.lon);
    super::geofence::check(user, area_id, lat, lon, pool).await?;
    db::main::event::queries::update(
        params.id,
        params.area_id,
        params.lat,
        params.lon,
        params.name,
        params.website,
        params.starts_at,
        params.ends_at,
        params.cron_schedule,
        pool,
    )
    .await
    .map(Into::into)
}

#[cfg(test)]
mod test {
    use crate::{
        db::main::{
            event::queries as event_queries,
            test::pool,
            user::schema::{Role, User},
        },
        Result,
    };
    use serde_json::json;
    use time::macros::datetime;

    #[test]
    fn rejects_moving_event_outside_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let event = event_queries::insert(
                Some(1),
                0.0,
                0.0,
                "meetup".into(),
                "https://example.com".into(),
                None,
                None,
                None,
                &pool,
            )
            .await?;
            let user = User {
                id: 1,
                name: "root".into(),
                password: String::new(),
                roles: vec![Role::Root],
                saved_places: vec![],
                saved_areas: vec![],
                npub: None,
                geofence: vec![1],
                created_at: String::new(),
                updated_at: String::new(),
                deleted_at: None,
            };
            let err = match super::run(
                super::Params {
                    id: event.id,
                    area_id: Some(Some(999)),
                    lat: None,
                    lon: None,
                    name: None,
                    website: None,
                    starts_at: None,
                    ends_at: None,
                    cron_schedule: None,
                },
                &user,
                &pool,
            )
            .await
            {
                Ok(_) => panic!("expected geofence violation"),
                Err(err) => err,
            };
            assert!(err.to_string().contains("outside your geofence"));
            assert_eq!(
                event_queries::select_by_id(event.id, &pool).await?.area_id,
                Some(1)
            );
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn parses_rfc3339_string() {
        let v = json!({
            "id": 1,
            "starts_at": "2026-08-20T19:00:00Z",
            "ends_at": "2026-08-20T22:00:00Z",
        });
        let p: super::Params = serde_json::from_value(v).unwrap();
        assert_eq!(p.starts_at, Some(Some(datetime!(2026-08-20 19:00:00 UTC))));
        assert_eq!(p.ends_at, Some(Some(datetime!(2026-08-20 22:00:00 UTC))));
    }

    #[test]
    fn parses_null_as_clear() {
        let v = json!({
            "id": 1,
            "starts_at": null,
            "ends_at": null,
        });
        let p: super::Params = serde_json::from_value(v).unwrap();
        assert_eq!(p.starts_at, Some(None));
        assert_eq!(p.ends_at, Some(None));
    }

    #[test]
    fn omits_field() {
        let v = json!({ "id": 1, "name": "renamed" });
        let p: super::Params = serde_json::from_value(v).unwrap();
        assert_eq!(p.starts_at, None);
        assert_eq!(p.ends_at, None);
        assert_eq!(p.name.as_deref(), Some("renamed"));
    }

    #[test]
    fn rejects_invalid_timestamp() {
        let v = json!({ "id": 1, "starts_at": "yesterday evening" });
        assert!(serde_json::from_value::<super::Params>(v).is_err());
    }
}
