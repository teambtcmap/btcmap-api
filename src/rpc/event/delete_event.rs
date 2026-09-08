use crate::{
    db::{self, main::event::schema::Event, main::user::schema::User},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
}

impl From<Event> for Res {
    fn from(event: Event) -> Self {
        Res { id: event.id }
    }
}

pub async fn run(params: Params, user: &User, pool: &Pool) -> Result<Res> {
    super::geofence::check_existing(user, params.id, pool).await?;
    db::main::event::queries::set_deleted_at(params.id, Some(OffsetDateTime::now_utc()), pool)
        .await
        .map(Into::into)
}

#[cfg(test)]
mod test {
    use super::run;
    use crate::{
        db::{self, main::test::pool},
        Result,
    };
    use serde_json::{json, Map};

    fn em_user(geofence: Vec<i64>) -> crate::db::main::user::schema::User {
        crate::db::main::user::schema::User {
            id: 1,
            name: "em".into(),
            password: String::new(),
            roles: vec![crate::db::main::user::schema::Role::EventManager],
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            geofence,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    async fn seed_event(
        pool: &deadpool_sqlite::Pool,
        area_id: Option<i64>,
        lat: f64,
        lon: f64,
    ) -> Result<i64> {
        let event = db::main::event::queries::insert(
            area_id,
            lat,
            lon,
            "meetup".to_string(),
            "https://example.com".to_string(),
            None,
            None,
            None,
            pool,
        )
        .await?;
        Ok(event.id)
    }

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

    #[test]
    fn event_manager_cannot_delete_event_outside_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            // Event lives in London, but the user is fenced to Phuket
            let user = em_user(vec![phuket]);
            let event_id = seed_event(&pool, Some(phuket), 51.5, -0.1).await?;
            let err = match run(super::Params { id: event_id }, &user, &pool).await {
                Ok(_) => panic!("expected geofence violation"),
                Err(e) => e,
            };
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_can_delete_event_in_subarea_when_geofence_is_parent() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            // Fence is the parent (Phuket), but the event has a child area_id
            // and a point inside the parent polygon. Deletion must succeed.
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let child = insert_area(
                "phuket-tourism",
                serde_json::from_str(PHUKET).unwrap(),
                &pool,
            )
            .await?;
            let event_id = seed_event(&pool, Some(child), 7.98, 98.33).await?;
            let user = em_user(vec![phuket]);
            let res = run(super::Params { id: event_id }, &user, &pool).await?;
            assert_eq!(res.id, event_id);
            Ok::<(), crate::Error>(())
        })
    }
}
