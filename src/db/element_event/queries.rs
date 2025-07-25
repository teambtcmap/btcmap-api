use super::schema::{self, Columns, ElementEvent};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use serde_json::Value;
use std::collections::HashMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(
    user_id: i64,
    element_id: i64,
    r#type: &str,
    conn: &Connection,
) -> Result<ElementEvent> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {user_id},
                {element_id},
                {type}
            ) VALUES (
                :user_id,
                :element_id,
                :type
            )
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        user_id = Columns::UserId.as_str(),
        element_id = Columns::ElementId.as_str(),
        r#type = Columns::Type.as_str(),
        projection = ElementEvent::projection(),
    );
    let params = named_params! {
        ":user_id": user_id,
        ":element_id": element_id,
        ":type": r#type,
    };
    conn.query_row(&sql, params, ElementEvent::mapper())
        .map_err(Into::into)
}

pub fn select_all(
    sort_order: Option<String>,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<ElementEvent>> {
    let sort_order = sort_order.unwrap_or("ASC".into());
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {updated_at} {sort_order}, {id} {sort_order}
            LIMIT ?1
        "#,
        projection = ElementEvent::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    Ok(conn
        .prepare(&sql)?
        .query_map(params![limit.unwrap_or(i64::MAX)], ElementEvent::mapper())?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_by_type(
    r#type: &str,
    sort_order: Option<String>,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<ElementEvent>> {
    let sort_order = sort_order.unwrap_or("ASC".into());
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {type} = ?1
            ORDER BY {updated_at} {sort_order}, {id} {sort_order}
            LIMIT ?2
        "#,
        projection = ElementEvent::projection(),
        table = schema::TABLE_NAME,
        r#type = Columns::Type.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    Ok(conn
        .prepare(&sql)?
        .query_map(
            params![r#type, limit.unwrap_or(i64::MAX)],
            ElementEvent::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<ElementEvent>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {updated_at} > ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = ElementEvent::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    Ok(conn
        .prepare(&sql)?
        .query_map(
            params![updated_since.format(&Rfc3339)?, limit.unwrap_or(i64::MAX),],
            ElementEvent::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_created_between(
    period_start: &OffsetDateTime,
    period_end: &OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<ElementEvent>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {created_at} > ?1 AND {created_at} < ?2
            ORDER BY {updated_at}, {id}
        "#,
        projection = ElementEvent::projection(),
        table = schema::TABLE_NAME,
        created_at = Columns::CreatedAt.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    let res = conn
        .prepare(&sql)?
        .query_map(
            params![period_start.format(&Rfc3339)?, period_end.format(&Rfc3339)?,],
            ElementEvent::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(res)
}

pub fn select_by_user(id: i64, limit: i64, conn: &Connection) -> Result<Vec<ElementEvent>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {user_id} = ?1
            ORDER BY {created_at} DESC
            LIMIT ?2
        "#,
        projection = ElementEvent::projection(),
        table = schema::TABLE_NAME,
        user_id = Columns::UserId.as_str(),
        created_at = Columns::CreatedAt.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![id, limit], ElementEvent::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<ElementEvent> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = ElementEvent::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], ElementEvent::mapper())
        .map_err(Into::into)
}

pub fn patch_tags(
    id: i64,
    tags: &HashMap<String, Value>,
    conn: &Connection,
) -> Result<ElementEvent> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {tags} = json_patch({tags}, ?2)
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, &serde_json::to_string(tags)?])?;
    select_by_id(id, conn)
}

#[cfg(test)]
pub fn set_updated_at(
    id: i64,
    updated_at: OffsetDateTime,
    conn: &Connection,
) -> Result<ElementEvent> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {updated_at} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, updated_at.format(&Rfc3339)?,])?;
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::{
        db::{self, test::conn},
        service::{osm::EditingApiUser, overpass::OverpassElement},
        Result,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use time::{macros::datetime, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let event = super::insert(user.id, element.id, "create", &conn)?;
        assert_eq!(event, super::select_by_id(event.id, &conn)?);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(
            vec![
                super::insert(user.id, element.id, "", &conn)?,
                super::insert(user.id, element.id, "", &conn)?,
                super::insert(user.id, element.id, "", &conn)?,
            ],
            super::select_all(None, None, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = conn();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let event_1 = super::insert(user.id, element.id, "", &conn)?;
        let _event_1 = super::set_updated_at(event_1.id, datetime!(2020-01-01 00:00 UTC), &conn)?;
        let event_2 = super::insert(1, element.id, "", &conn)?;
        let event_2 = super::set_updated_at(event_2.id, datetime!(2020-01-02 00:00 UTC), &conn)?;
        let event_3 = super::insert(1, element.id, "", &conn)?;
        let event_3 = super::set_updated_at(event_3.id, datetime!(2020-01-03 00:00 UTC), &conn)?;
        assert_eq!(
            vec![event_2, event_3,],
            super::select_updated_since(datetime!(2020-01-01 00:00 UTC), None, &conn,)?
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let event = super::insert(user.id, element.id, "", &conn)?;
        assert_eq!(event, super::select_by_id(1, &conn)?);
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value_1 = json!("tag_1_value_1");
        let tag_1_value_2 = json!("tag_1_value_2");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let event = super::insert(user.id, element.id, "", &conn)?;
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value_1.clone());
        let event = super::patch_tags(event.id, &tags, &conn)?;
        assert_eq!(tag_1_value_1, event.tags[tag_1_name]);
        tags.insert(tag_1_name.into(), tag_1_value_2.clone());
        let event = super::patch_tags(event.id, &tags, &conn)?;
        assert_eq!(tag_1_value_2, event.tags[tag_1_name]);
        tags.clear();
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let event = super::patch_tags(event.id, &tags, &conn)?;
        assert!(event.tags.contains_key(tag_1_name));
        assert_eq!(tag_2_value, event.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = conn();
        let updated_at = OffsetDateTime::now_utc();
        let user = db::osm_user::queries::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let event = super::insert(user.id, element.id, "", &conn)?;
        let event = super::set_updated_at(event.id, updated_at, &conn)?;
        assert_eq!(updated_at, super::select_by_id(event.id, &conn)?.updated_at);
        Ok(())
    }
}
