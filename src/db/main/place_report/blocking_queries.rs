use super::schema::{self, Columns, PlaceReport};
use crate::Result;
use rusqlite::{named_params, Connection};
use serde_json::{Map, Value};

pub struct InsertArgs {
    pub place_id: i64,
    pub origin_id: i64,
    pub r#type: String,
    pub extra_fields: Map<String, Value>,
    pub ticket_url: Option<String>,
}

pub fn insert(args: &InsertArgs, conn: &Connection) -> Result<PlaceReport> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({place_id}, {origin_id}, {type}, {extra_fields}, {ticket_url})
            VALUES (:place_id, :origin_id, :type, json(:extra_fields), :ticket_url)
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        place_id = Columns::PlaceId.as_ref(),
        origin_id = Columns::OriginId.as_ref(),
        type = Columns::Type.as_ref(),
        extra_fields = Columns::ExtraFields.as_ref(),
        ticket_url = Columns::TicketUrl.as_ref(),
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
        },
        PlaceReport::mapper(),
    )
    .map_err(Into::into)
}

#[cfg(test)]
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

#[cfg(test)]
mod test {
    use super::InsertArgs;
    use crate::db::main::test::conn;
    use crate::Result;
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
        };
        let report = super::insert(&args, &conn)?;
        assert_eq!(args.place_id, report.place_id);
        assert_eq!(args.origin_id, report.origin_id);
        assert_eq!(args.r#type, report.r#type);
        assert_eq!(extra_fields, report.extra_fields);
        assert_eq!(args.ticket_url, report.ticket_url);
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
        };

        let first = super::insert(&args, &conn)?;
        let second = super::insert(&args, &conn)?;

        assert_ne!(first.id, second.id);
        assert_eq!(first.place_id, second.place_id);
        assert_eq!(first.origin_id, second.origin_id);
        assert_eq!(first.r#type, second.r#type);
        Ok(())
    }
}
