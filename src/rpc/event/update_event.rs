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
    let lat = params.lat.unwrap_or(event.lat);
    let lon = params.lon.unwrap_or(event.lon);
    super::geofence::check(user, lat, lon, pool).await?;
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
        db::{
            self,
            main::{
                event::queries as event_queries,
                test::pool,
                user::schema::{Role, User},
            },
        },
        Result,
    };
    use serde_json::{json, Map};
    use time::macros::datetime;

    // A small Phuket-shaped polygon used as the fenced (parent) area.
    const PHUKET: &str = r#"{
        "type":"Feature",
        "properties":{},
        "geometry":{
            "type":"Polygon",
            "coordinates":[[
                [98.2181205776469, 8.20412838698085],
                [98.2181205776469, 7.74024270965898],
                [98.4806081271279, 7.74024270965898],
                [98.4806085771279, 8.20412838698085],
                [98.2181205776469, 8.20412838698085]
            ]]
        }
    }"#;

    async fn insert_area(
        name: &str,
        geo_json: serde_json::Value,
        pool: &deadpool_sqlite::Pool,
    ) -> Result<i64> {
        let mut tags = Map::new();
        tags.insert("name".into(), json!(name));
        tags.insert("geo_json".into(), geo_json);
        tags.insert("url_alias".into(), json!(name));
        Ok(db::main::area::queries::insert(tags, pool).await?.id)
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

    #[test]
    fn rejects_update_when_point_is_outside_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            // Event lives in London, but the user is fenced to Phuket.
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let event = event_queries::insert(
                Some(phuket),
                51.5,
                -0.1,
                "meetup".into(),
                "https://example.com".into(),
                None,
                None,
                None,
                &pool,
            )
            .await?;
            let user = em_user(vec![phuket]);
            let err = match super::run(
                super::Params {
                    id: event.id,
                    area_id: None,
                    lat: None,
                    lon: None,
                    name: Some("renamed".into()),
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
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn allows_editing_event_in_subarea_when_geofence_is_parent() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            // The user's geofence is the parent (Phuket). The event already
            // has a child area_id assigned, but the point is inside the
            // parent polygon, so the update must succeed.
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let child = insert_area(
                "phuket-tourism",
                serde_json::from_str(PHUKET).unwrap(),
                &pool,
            )
            .await?;
            let event = event_queries::insert(
                Some(child),
                7.98,
                98.33,
                "meetup".into(),
                "https://example.com".into(),
                None,
                None,
                None,
                &pool,
            )
            .await?;
            let user = em_user(vec![phuket]);
            let res = super::run(
                super::Params {
                    id: event.id,
                    area_id: None,
                    lat: None,
                    lon: None,
                    name: Some("renamed".into()),
                    website: None,
                    starts_at: None,
                    ends_at: None,
                    cron_schedule: None,
                },
                &user,
                &pool,
            )
            .await?;
            assert_eq!(res.name, "renamed");
            assert_eq!(res.area_id, Some(child));
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn rejects_update_when_new_lat_lon_move_event_outside_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let event = event_queries::insert(
                Some(phuket),
                7.98,
                98.33,
                "meetup".into(),
                "https://example.com".into(),
                None,
                None,
                None,
                &pool,
            )
            .await?;
            let user = em_user(vec![phuket]);
            // Try to drag the event to London via the lat/lon params.
            let err = match super::run(
                super::Params {
                    id: event.id,
                    area_id: None,
                    lat: Some(51.5),
                    lon: Some(-0.1),
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
                event_queries::select_by_id(event.id, &pool).await?.lat,
                7.98
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
