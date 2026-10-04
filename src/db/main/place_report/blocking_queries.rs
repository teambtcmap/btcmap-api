use super::schema::{self, Columns, PlaceReport};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub struct InsertArgs {
    pub place_id: i64,
    pub origin_id: i64,
    pub r#type: String,
    pub extra_fields: Map<String, Value>,
    pub ticket_url: Option<String>,
    pub submitted_by: Option<i64>,
}

pub fn insert(args: &InsertArgs, conn: &Connection) -> Result<PlaceReport> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({place_id}, {origin_id}, {type}, {extra_fields}, {ticket_url}, {submitted_by})
            VALUES (:place_id, :origin_id, :type, json(:extra_fields), :ticket_url, :submitted_by)
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        place_id = Columns::PlaceId.as_ref(),
        origin_id = Columns::OriginId.as_ref(),
        type = Columns::Type.as_ref(),
        extra_fields = Columns::ExtraFields.as_ref(),
        ticket_url = Columns::TicketUrl.as_ref(),
        submitted_by = Columns::SubmittedBy.as_ref(),
        projection = PlaceReport::projection(),
    );
    conn.query_row(
        &sql,
        named_params! {
            ":place_id": args.place_id,
            ":origin_id": args.origin_id,
            ":type": &args.r#type,
            ":extra_fields": serde_json::to_string(&args.extra_fields)?,
            ":ticket_url": &args.ticket_url,
            ":submitted_by": args.submitted_by,
        },
        PlaceReport::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<PlaceReport> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = :id
        "#,
        projection = PlaceReport::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
    );
    conn.query_row(&sql, named_params! { ":id": id }, PlaceReport::mapper())
        .map_err(Into::into)
}

pub fn select_open_and_not_deleted(conn: &Connection) -> Result<Vec<PlaceReport>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {closed_at} IS NULL AND {deleted_at} IS NULL
            ORDER BY {updated_at} DESC, {id} DESC
        "#,
        projection = PlaceReport::projection(),
        table = schema::TABLE_NAME,
        closed_at = Columns::ClosedAt.as_ref(),
        deleted_at = Columns::DeletedAt.as_ref(),
        updated_at = Columns::UpdatedAt.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(params![], PlaceReport::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn set_ticket_url(id: i64, ticket_url: String, conn: &Connection) -> Result<PlaceReport> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {ticket_url} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        ticket_url = Columns::TicketUrl.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![id, ticket_url])?;
    select_by_id(id, conn)
}

pub fn set_closed_at(
    id: i64,
    closed_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<PlaceReport> {
    match closed_at {
        Some(closed_at) => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {closed_at} = ?2
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                closed_at = Columns::ClosedAt.as_ref(),
                id = Columns::Id.as_ref(),
            );
            conn.execute(&sql, params![id, closed_at.format(&Rfc3339)?,])?;
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {closed_at} = NULL
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                closed_at = Columns::ClosedAt.as_ref(),
                id = Columns::Id.as_ref(),
            );
            conn.execute(&sql, params![id])?;
        }
    };
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use super::InsertArgs;
    use crate::db::main::test::conn;
    use crate::Result;
    use rusqlite::{params, Connection};
    use serde_json::{Map, Value};

    #[test]
    fn insert_and_select_by_id() -> Result<()> {
        let conn = conn();
        let mut extra_fields = Map::new();
        extra_fields.insert("comment".into(), Value::String("had lunch".into()));

        let args = InsertArgs {
            place_id: 42,
            origin_id: 1,
            r#type: "verification".to_string(),
            extra_fields: extra_fields.clone(),
            ticket_url: Some("https://example.com/ticket/1".to_string()),
            submitted_by: None,
        };
        let report = super::insert(&args, &conn)?;
        assert_eq!(args.place_id, report.place_id);
        assert_eq!(args.origin_id, report.origin_id);
        assert_eq!(args.r#type, report.r#type);
        assert_eq!(extra_fields, report.extra_fields);
        assert_eq!(args.ticket_url, report.ticket_url);
        assert_eq!(args.submitted_by, report.submitted_by);
        assert!(report.closed_at.is_none());
        assert!(report.deleted_at.is_none());

        let fetched = super::select_by_id(report.id, &conn)?;
        assert_eq!(report, fetched);
        Ok(())
    }

    #[test]
    fn insert_allows_duplicate_natural_key() -> Result<()> {
        let conn = conn();
        let args = InsertArgs {
            place_id: 42,
            origin_id: 1,
            r#type: "verification".to_string(),
            extra_fields: Map::new(),
            ticket_url: None,
            submitted_by: None,
        };

        let first = super::insert(&args, &conn)?;
        let second = super::insert(&args, &conn)?;

        assert_ne!(first.id, second.id);
        assert_eq!(first.place_id, second.place_id);
        assert_eq!(first.origin_id, second.origin_id);
        assert_eq!(first.r#type, second.r#type);
        Ok(())
    }

    fn insert_report(conn: &Connection, place_id: i64, ticket_url: Option<&str>) -> Result<i64> {
        let args = InsertArgs {
            place_id,
            origin_id: 1,
            r#type: "verification".to_string(),
            extra_fields: Map::new(),
            ticket_url: ticket_url.map(|s| s.to_string()),
            submitted_by: None,
        };
        Ok(super::insert(&args, conn)?.id)
    }

    #[test]
    fn select_open_and_not_deleted_returns_only_open_rows() -> Result<()> {
        let conn = conn();
        let open = insert_report(&conn, 1, None)?;
        let with_ticket = insert_report(&conn, 2, Some("https://example.com/ticket/2"))?;
        let closed = insert_report(&conn, 3, Some("https://example.com/ticket/3"))?;
        let deleted = insert_report(&conn, 4, None)?;

        super::set_closed_at(
            closed,
            Some(time::macros::datetime!(2024-06-01 00:00 UTC)),
            &conn,
        )?;
        conn.execute(
            "UPDATE place_report SET deleted_at = '2024-06-01T00:00:00Z' WHERE id = ?1",
            params![deleted],
        )?;

        let reports = super::select_open_and_not_deleted(&conn)?;
        let ids: Vec<i64> = reports.iter().map(|r| r.id).collect();
        assert_eq!(vec![with_ticket, open], ids);
        Ok(())
    }

    #[test]
    fn set_ticket_url_persists_value() -> Result<()> {
        let conn = conn();
        let id = insert_report(&conn, 1, None)?;

        let updated = super::set_ticket_url(id, "https://example.com/ticket/new".into(), &conn)?;
        assert_eq!(
            Some("https://example.com/ticket/new".to_string()),
            updated.ticket_url
        );

        let fetched = super::select_by_id(id, &conn)?;
        assert_eq!(
            Some("https://example.com/ticket/new".to_string()),
            fetched.ticket_url
        );
        Ok(())
    }

    #[test]
    fn set_closed_at_writes_and_clears_timestamp() -> Result<()> {
        let conn = conn();
        let id = insert_report(&conn, 1, None)?;

        let updated = super::set_closed_at(
            id,
            Some(time::macros::datetime!(2024-06-01 00:00 UTC)),
            &conn,
        )?;
        assert!(updated.closed_at.is_some());

        let cleared = super::set_closed_at(id, None, &conn)?;
        assert!(cleared.closed_at.is_none());
        Ok(())
    }
}
