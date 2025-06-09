use crate::{
    conf::Conf,
    db::{self, admin::queries::Admin},
    discord,
    element::Element,
    osm::overpass::OverpassElement,
    Result,
};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    from_element_id: i64,
    to_element_id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub changes: i64,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let res = pool
        .get()
        .await?
        .interact(move |conn| {
            generate_element_categories(params.from_element_id, params.to_element_id, conn)
        })
        .await??;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated element categories (id range {}..{})",
            admin.name, params.from_element_id, params.to_element_id,
        ),
    )
    .await;
    Ok(res)
}

fn generate_element_categories(
    from_element_id: i64,
    to_element_id: i64,
    conn: &Connection,
) -> Result<Res> {
    let mut changes = 0;
    for element_id in from_element_id..=to_element_id {
        let Ok(element) = db::element::queries::select_by_id(element_id, conn) else {
            continue;
        };
        let old_category = element.tag("category").as_str().unwrap_or_default();
        let new_category = element.overpass_data.generate_category();
        if old_category != new_category {
            Element::set_tag(element.id, "category", &new_category.clone().into(), conn)?;
            changes += 1;
        }
    }
    Ok(Res { changes })
}

impl OverpassElement {
    pub fn generate_category(&self) -> String {
        let amenity = self.tag("amenity");
        let tourism = self.tag("tourism");

        let mut category = "other";

        if amenity == "atm" || amenity == "bureau_de_change"
        // practical assumption of exchange offices are regarded as much useful as atms
        {
            category = "atm";
        }

        if amenity == "cafe" {
            category = "cafe";
        }

        if amenity == "restaurant" {
            category = "restaurant";
        }

        if amenity == "bar" {
            category = "bar";
        }

        if amenity == "pub" {
            category = "pub";
        }

        if tourism == "hotel" {
            category = "hotel";
        }

        category.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::Result;
    use crate::{db, db_utils};
    use rusqlite::Connection;
    use serde_json::Map;
    use time::OffsetDateTime;

    #[actix_web::test]
    async fn run() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db_utils::migrate(&mut conn)?;

        let mut tags = Map::new();
        tags.insert("amenity".into(), "atm".into());
        db::element::queries::insert(
            &OverpassElement {
                tags: Some(tags),
                ..OverpassElement::mock(1)
            },
            &conn,
        )?;

        let mut tags = Map::new();
        tags.insert("amenity".into(), "cafe".into());
        db::element::queries::insert(
            &OverpassElement {
                tags: Some(tags),
                ..OverpassElement::mock(2)
            },
            &conn,
        )?;

        super::generate_element_categories(1, 100, &conn)?;

        let elements =
            Element::select_updated_since(&OffsetDateTime::UNIX_EPOCH, None, true, &conn)?;

        assert_eq!("atm", elements[0].tag("category").as_str().unwrap());
        assert_eq!("cafe", elements[1].tag("category").as_str().unwrap());

        Ok(())
    }

    #[test]
    fn generate_category() {
        let mut tags = Map::new();
        tags.insert("amenity".into(), "atm".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("atm", &element.generate_category());

        let mut tags = Map::new();
        tags.insert("amenity".into(), "cafe".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("cafe", &element.generate_category());

        let mut tags = Map::new();
        tags.insert("amenity".into(), "restaurant".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("restaurant", &element.generate_category());

        let mut tags = Map::new();
        tags.insert("amenity".into(), "bar".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("bar", &element.generate_category());

        let mut tags = Map::new();
        tags.insert("amenity".into(), "pub".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("pub", &element.generate_category());

        let mut tags = Map::new();
        tags.insert("tourism".into(), "hotel".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("hotel", &element.generate_category());

        let mut tags = Map::new();
        tags.insert("foo".into(), "bar".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock(1)
        };
        assert_eq!("other", &element.generate_category());
    }
}
