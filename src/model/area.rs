use rusqlite::Result;
use rusqlite::Row;
use serde_json::Map;
use serde_json::Value;

#[derive(Clone)]
pub struct Area {
    pub id: String,
    pub tags: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Area {
    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        let north = match self.tags.get("box:north") {
            Some(north) => {
                if north.is_f64() {
                    north.as_f64().unwrap()
                } else {
                    let res = north.as_str().map(|it| it.parse::<f64>());

                    if res.is_none() {
                        return false;
                    }

                    let res = res.unwrap();

                    if res.is_err() {
                        return false;
                    }

                    res.unwrap()
                }
            }
            None => return false,
        };

        let east = match self.tags.get("box:east") {
            Some(east) => {
                if east.is_f64() {
                    east.as_f64().unwrap()
                } else {
                    let res = east.as_str().map(|it| it.parse::<f64>());

                    if res.is_none() {
                        return false;
                    }

                    let res = res.unwrap();

                    if res.is_err() {
                        return false;
                    }

                    res.unwrap()
                }
            }
            None => return false,
        };

        let south = match self.tags.get("box:south") {
            Some(south) => {
                if south.is_f64() {
                    south.as_f64().unwrap()
                } else {
                    let res = south.as_str().map(|it| it.parse::<f64>());

                    if res.is_none() {
                        return false;
                    }

                    let res = res.unwrap();

                    if res.is_err() {
                        return false;
                    }

                    res.unwrap()
                }
            }
            None => return false,
        };

        let west = match self.tags.get("box:west") {
            Some(west) => {
                if west.is_f64() {
                    west.as_f64().unwrap()
                } else {
                    let res = west.as_str().map(|it| it.parse::<f64>());

                    if res.is_none() {
                        return false;
                    }

                    let res = res.unwrap();

                    if res.is_err() {
                        return false;
                    }

                    res.unwrap()
                }
            }
            None => return false,
        };

        lat < north && lat > south && lon > west && lon < east
    }
}

pub static INSERT: &str = r#"
    INSERT INTO area (
        id
    )
    VALUES (
        :id
    )
"#;

pub static SELECT_ALL: &str = r#"
    SELECT
        id,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM area
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_ALL_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

pub static SELECT_BY_ID: &str = r#"
    SELECT
        id,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM area
    WHERE id = :id
"#;

pub static SELECT_BY_ID_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

pub static SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM area
    WHERE updated_at > :updated_since
    ORDER BY updated_at
    LIMIT :limit
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

pub static UPDATE_TAGS: &str = r#"
    UPDATE area
    SET tags = :tags
    WHERE id = :area_id
"#;

pub static INSERT_TAG: &str = r#"
    UPDATE area
    SET tags = json_set(tags, :tag_name, :tag_value)
    WHERE id = :area_id
"#;

pub static DELETE_TAG: &str = r#"
    UPDATE area
    SET tags = json_remove(tags, :tag_name)
    where id = :area_id
"#;

pub static MARK_AS_DELETED: &str = r#"
    UPDATE area
    SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ')
    WHERE id = :id
"#;

pub static TOUCH: &str = r#"
    UPDATE area
    SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ')
    WHERE id = :id
"#;

const fn full_mapper() -> fn(&Row) -> Result<Area> {
    |row: &Row| -> Result<Area> {
        let tags: String = row.get(1)?;
        let mut tags: Map<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        let geo_json = tags.get("geo_json");

        if geo_json.is_some() && geo_json.unwrap().is_string() {
            let unescaped: String = geo_json.unwrap().as_str().unwrap().replace("\\\"", "\"");
            let unescaped: Value = serde_json::from_str(&unescaped).unwrap_or_default();
            tags.insert("geo_json".into(), unescaped);
        }

        Ok(Area {
            id: row.get(0)?,
            tags: tags,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Number, Value, Map};

    use crate::Result;

    use super::Area;

    #[test]
    fn contains() -> Result<()> {
        let mut tags: Map<_, Value> = Map::new();
        tags.insert("box:north".into(), 49.60003042758964.into());
        tags.insert(
            "box:east".into(),
            Number::from_f64(-121.77932739257814).unwrap().into(),
        );
        tags.insert("box:south".into(), 48.81861991362668.into());
        tags.insert(
            "box:west".into(),
            Number::from_f64(-124.41604614257814).unwrap().into(),
        );

        let area = Area {
            id: "".into(),
            tags: tags,
            created_at: "".into(),
            updated_at: "".into(),
            deleted_at: "".into(),
        };

        assert_eq!(area.contains(49.2623463, -123.0886088), true);
        assert_eq!(area.contains(47.6084752, -122.3270694), false);

        let mut tags: Map<_, Value> = Map::new();
        tags.insert("box:north".into(), "18.86515".into());
        tags.insert("box:east".into(), "99.07234".into());
        tags.insert("box:south".into(), "18.70702".into());
        tags.insert("box:west".into(), "98.92883".into());

        let area = Area {
            id: "".into(),
            tags: tags,
            created_at: "".into(),
            updated_at: "".into(),
            deleted_at: "".into(),
        };

        assert_eq!(area.contains(18.78407, 98.99283), true);

        Ok(())
    }
}
