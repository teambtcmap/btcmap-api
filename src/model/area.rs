use rusqlite::Result;
use rusqlite::Row;
use serde_json::Value;

pub struct Area {
    pub id: String,
    pub tags: Value,
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
"#;

pub static SELECT_UPDATED_SINCE_MAPPER: fn(&Row) -> Result<Area> = full_mapper();

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

const fn full_mapper() -> fn(&Row) -> Result<Area> {
    |row: &Row| -> Result<Area> {
        let tags: String = row.get(1)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

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
    use crate::Result;

    use super::Area;

    #[test]
    fn contains() -> Result<()> {
        let tags = serde_json::json!({
            "box:north": 49.60003042758964,
            "box:east": -121.77932739257814,
            "box:south": 48.81861991362668,
            "box:west": -124.41604614257814,
        });

        let area = Area {
            id: "".into(),
            tags: tags,
            created_at: "".into(),
            updated_at: "".into(),
            deleted_at: "".into(),
        };

        assert_eq!(area.contains(49.2623463, -123.0886088), true);
        assert_eq!(area.contains(47.6084752, -122.3270694), false);

        let tags = serde_json::json!({
            "box:north": "18.86515",
            "box:east": "99.07234",
            "box:south": "18.70702",
            "box:west": "98.92883",
        });

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
