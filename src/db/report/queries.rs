use super::schema::{self, Columns, Report};
use crate::Result;
use geojson::JsonObject;
use rusqlite::{named_params, params, Connection};
use time::{format_description::well_known::Rfc3339, Date, OffsetDateTime};

pub fn insert(area_id: i64, date: Date, tags: &JsonObject, conn: &Connection) -> Result<Report> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {area_id},
                {date},
                {tags}
            ) VALUES (
                :area_id,
                :date,
                :tags
            )
        "#,
        table = schema::TABLE_NAME,
        area_id = Columns::AreaId.as_str(),
        date = Columns::Date.as_str(),
        tags = Columns::Tags.as_str(),
    );
    conn.execute(
        &sql,
        named_params! {
            ":area_id" : area_id,
            ":date" : date.to_string(),
            ":tags" : serde_json::to_string(&tags)?,
        },
    )?;
    select_by_id(conn.last_insert_rowid(), conn)
}

pub fn select_all(
    sort_order: Option<String>,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<Report>> {
    let sort_order = sort_order.unwrap_or("ASC".into());
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {updated_at} {sort_order}, {id} {sort_order}
            LIMIT ?1
        "#,
        projection = Report::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    Ok(conn
        .prepare(&sql)?
        .query_map(params![limit.unwrap_or(i64::MAX)], Report::mapper())?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<Report>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {updated_at} > ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = Report::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            params![updated_since.format(&Rfc3339)?, limit.unwrap_or(i64::MAX),],
            Report::mapper(),
        )?
        .collect::<Result<Vec<Report>, _>>()
        .map_err(Into::into)
}

pub fn select_by_date(date: Date, limit: Option<i64>, conn: &Connection) -> Result<Vec<Report>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {date} = ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = Report::projection(),
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            params![date.to_string(), limit.unwrap_or(i64::MAX),],
            Report::mapper(),
        )?
        .collect::<Result<Vec<Report>, _>>()
        .map_err(Into::into)
}

pub fn select_by_area_id(
    area_id: i64,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<Report>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {area_id} = ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = Report::projection(),
        table = schema::TABLE_NAME,
        area_id = Columns::AreaId.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            params![area_id, limit.unwrap_or(i64::MAX)],
            Report::mapper(),
        )?
        .collect::<Result<Vec<Report>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Report> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Report::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Report::mapper())
        .map_err(Into::into)
}

pub fn select_latest_by_area_id(area_id: i64, conn: &Connection) -> Result<Report> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {area_id} = ?1
            ORDER BY {created_at} DESC, {id} DESC
            LIMIT 1
        "#,
        projection = Report::projection(),
        table = schema::TABLE_NAME,
        area_id = Columns::AreaId.as_str(),
        created_at = Columns::CreatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![area_id], Report::mapper())
        .map_err(Into::into)
}

pub fn patch_tags(id: i64, tags: &JsonObject, conn: &Connection) -> Result<Report> {
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
pub fn set_updated_at(id: i64, updated_at: OffsetDateTime, conn: &Connection) -> Result<Report> {
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
pub fn set_deleted_at(id: i64, deleted_at: OffsetDateTime, conn: &Connection) -> Result<Report> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {deleted_at} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        deleted_at = Columns::DeletedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, deleted_at.format(&Rfc3339)?])?;
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::db;
    use crate::db::area::schema::Area;
    use crate::test::mock_conn;
    use crate::Result;
    use serde_json::Map;
    use serde_json::Value;
    use std::ops::Add;
    use time::macros::date;
    use time::{macros::datetime, Duration, OffsetDateTime};

    #[test]
    fn insert_1() -> Result<()> {
        let conn = mock_conn();
        let area = db::area::queries::insert(Area::mock_tags(), &conn)?;
        let date = date!(2023 - 01 - 01);
        let mut tags = Map::new();
        tags.insert("key".to_string(), Value::String("value".to_string()));
        let report = super::insert(area.id, date.clone(), &tags, &conn)?;
        assert_eq!(report.area_id, area.id);
        assert_eq!(report.date, date);
        assert_eq!(report.tags, tags);
        Ok(())
    }

    #[test]
    fn insert_2() -> Result<()> {
        let conn = mock_conn();
        db::area::queries::insert(Area::mock_tags(), &conn)?;
        super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        let reports = super::select_updated_since(datetime!(2000-01-01 00:00 UTC), None, &conn)?;
        assert_eq!(1, reports.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        db::area::queries::insert(Area::mock_tags(), &conn)?;
        let report_1 = super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        super::set_updated_at(report_1.id, datetime!(2020-01-01 00:00:00 UTC), &conn)?;
        let report_2 = super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        super::set_updated_at(report_2.id, datetime!(2020-01-02 00:00:00 UTC), &conn)?;
        let report_3 = super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        super::set_updated_at(report_3.id, datetime!(2020-01-03 00:00:00 UTC), &conn)?;
        assert_eq!(
            2,
            super::select_updated_since(datetime!(2020-01-01 00:00 UTC), None, &conn)?.len(),
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        db::area::queries::insert(Area::mock_tags(), &conn)?;
        super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        assert!(super::select_by_id(1, &conn).is_ok());
        Ok(())
    }

    #[test]
    fn select_latest_by_area_id() -> Result<()> {
        let conn = mock_conn();
        let area = db::area::queries::insert(Area::mock_tags(), &conn)?;
        super::insert(
            area.id,
            OffsetDateTime::now_utc().date().previous_day().unwrap(),
            &Map::new(),
            &conn,
        )?;
        let latest_report = super::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            &Map::new(),
            &conn,
        )?;
        assert_eq!(
            latest_report,
            super::select_latest_by_area_id(area.id, &conn)?,
        );
        Ok(())
    }

    #[test]
    fn merge_tags() -> Result<()> {
        let conn = mock_conn();
        db::area::queries::insert(Area::mock_tags(), &conn)?;
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        let report = super::select_by_id(1, &conn)?;
        assert!(report.tags.is_empty());
        super::patch_tags(1, &tags, &conn)?;
        let report = super::select_by_id(1, &conn)?;
        assert_eq!(1, report.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        super::patch_tags(1, &tags, &conn)?;
        let report = super::select_by_id(1, &conn)?;
        assert_eq!(2, report.tags.len());
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        db::area::queries::insert(Area::mock_tags(), &conn)?;
        let report = super::insert(1, OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        let new_deleted_at = OffsetDateTime::now_utc().add(Duration::days(1));
        super::set_deleted_at(report.id, new_deleted_at, &conn)?;
        assert_eq!(
            new_deleted_at,
            super::select_by_id(report.id, &conn)?.deleted_at.unwrap(),
        );
        Ok(())
    }
}
