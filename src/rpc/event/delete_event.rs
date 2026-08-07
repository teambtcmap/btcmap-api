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
        db::main::{
            event::queries as event_queries,
            test::pool,
            user::schema::{Role, User},
        },
        Result,
    };

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

    async fn seed_event(pool: &deadpool_sqlite::Pool, lat: f64, lon: f64) -> Result<i64> {
        let event = event_queries::insert(
            None,
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

    #[test]
    fn event_manager_cannot_delete_event_outside_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            // Fence is "only Phuket" but the event lives in London
            let user = em_user(vec![1]);
            let event_id = seed_event(&pool, 51.5, -0.1).await?;
            let err = match run(super::Params { id: event_id }, &user, &pool).await {
                Ok(_) => panic!("expected geofence violation"),
                Err(e) => e,
            };
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }
}
