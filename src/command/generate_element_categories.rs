use crate::model::element;
use crate::model::Element;
use crate::model::OverpassElement;
use crate::Connection;
use crate::Result;
use rusqlite::named_params;
use serde_json::Value;
use tracing::info;

pub async fn run(conn: &Connection) -> Result<()> {
    info!("Generating element categories");

    let elements: Vec<Element> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == "")
        .collect();

    info!(elements = elements.len(), "Loaded elements from database");

    let mut known = 0;
    let mut unknown = 0;

    for element in elements {
        let old_category = element.get_btcmap_tag_value_str("category");
        let new_category = element.generate_category();

        if new_category != old_category {
            info!(element.id, old_category, new_category, "Updating category",);

            conn.execute(
                element::INSERT_TAG,
                named_params! {
                    ":element_id": element.id,
                    ":tag_name": "$.category",
                    ":tag_value": new_category,
                },
            )?;
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        if new_category == "other" {
            unknown += 1;
        } else {
            known += 1;
        }

        let category_plural = element
            .tags
            .get("category:plural")
            .unwrap_or(&Value::Null)
            .as_str()
            .unwrap_or("");

        if category_plural.len() > 0 {
            info!(element.id, category_plural, "Removing category:plural",);

            conn.execute(
                "update element set tags = json_remove(tags, '$.category:plural') where id = :element_id;",
                named_params! { ":element_id": element.id },
            )?;
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }
    }

    let coverage = known as f64 / (known as f64 + unknown as f64) * 100.0;

    info!(
        known,
        unknown,
        coverage = format!("{:.2}", coverage),
        "Finished generating categories",
    );

    Ok(())
}

impl Element {
    pub fn generate_category(&self) -> String {
        self.osm_json.generate_category()
    }
}

impl OverpassElement {
    pub fn generate_category(&self) -> String {
        let amenity = self.get_tag_value("amenity");
        let tourism = self.get_tag_value("tourism");

        let mut category = "other";

        if amenity == "atm" {
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
    use std::collections::HashMap;

    use rusqlite::Connection;

    use crate::command::db;
    use crate::model::OverpassElement;

    use crate::model::element::Element;
    use crate::Result;

    #[actix_web::test]
    async fn run() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "atm".into());
        let element = OverpassElement {
            r#type: "node".into(),
            id: 1,
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        Element::insert(&element, &conn)?;
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "cafe".into());
        let element = OverpassElement {
            r#type: "node".into(),
            id: 2,
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        Element::insert(&element, &conn)?;

        super::run(&conn).await?;

        let elements = Element::select_all(None, &conn)?;

        assert_eq!("atm", elements[0].get_btcmap_tag_value_str("category"));
        assert_eq!("cafe", elements[1].get_btcmap_tag_value_str("category"));

        Ok(())
    }

    #[test]
    fn generate_category() {
        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "atm".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("atm", &element.generate_category());

        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "cafe".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("cafe", &element.generate_category());

        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "restaurant".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("restaurant", &element.generate_category());

        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "bar".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("bar", &element.generate_category());

        let mut tags = HashMap::new();
        tags.insert("amenity".into(), "pub".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("pub", &element.generate_category());

        let mut tags = HashMap::new();
        tags.insert("tourism".into(), "hotel".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("hotel", &element.generate_category());

        let mut tags = HashMap::new();
        tags.insert("foo".into(), "bar".into());
        let element = OverpassElement {
            tags: Some(tags),
            ..OverpassElement::mock()
        };
        assert_eq!("other", &element.generate_category());
    }
}
