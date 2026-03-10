use super::schema::{self, Columns};
use crate::Result;
use rusqlite::{named_params, Connection};

pub fn insert(conn: &Connection) -> Result<i64> {
    let id: i64 = conn.query_row(
        &format!(
            "INSERT INTO {table} (started_at) VALUES (strftime('%Y-%m-%dT%H:%M:%fZ')) RETURNING {id}",
            table = schema::TABLE_NAME,
            id = Columns::Id.as_str(),
        ),
        [],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub struct UpdateArgs {
    pub id: i64,
    pub finished_at: String,
    pub duration_s: f64,
    pub overpass_response_time_s: f64,
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
}

pub fn update(args: UpdateArgs, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {col_finished_at} = :finished_at,
                {col_duration_s} = :duration_s,
                {col_overpass_response_time_s} = :overpass_response_time_s,
                {col_elements_affected} = :elements_affected,
                {col_elements_created} = :elements_created,
                {col_elements_updated} = :elements_updated,
                {col_elements_deleted} = :elements_deleted
            WHERE {col_id} = :id
          "#,
        table = schema::TABLE_NAME,
        col_id = Columns::Id.as_str(),
        col_finished_at = Columns::FinishedAt.as_str(),
        col_duration_s = Columns::DurationS.as_str(),
        col_overpass_response_time_s = Columns::OverpassResponseTimeS.as_str(),
        col_elements_affected = Columns::ElementsAffected.as_str(),
        col_elements_created = Columns::ElementsCreated.as_str(),
        col_elements_updated = Columns::ElementsUpdated.as_str(),
        col_elements_deleted = Columns::ElementsDeleted.as_str(),
    );
    let elements_affected = args.elements_created + args.elements_updated + args.elements_deleted;
    conn.execute(
        &sql,
        named_params! {
            ":id": args.id,
            ":finished_at": args.finished_at,
            ":duration_s": args.duration_s,
            ":overpass_response_time_s": args.overpass_response_time_s,
            ":elements_affected": elements_affected,
            ":elements_created": args.elements_created,
            ":elements_updated": args.elements_updated,
            ":elements_deleted": args.elements_deleted,
        },
    )?;
    Ok(())
}

pub struct UpdateFailedArgs {
    pub id: i64,
    pub failed_at: String,
    pub fail_reason: String,
}

pub fn update_failed(args: UpdateFailedArgs, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {col_failed_at} = :failed_at,
                {col_fail_reason} = :fail_reason
            WHERE {col_id} = :id
          "#,
        table = schema::TABLE_NAME,
        col_id = Columns::Id.as_str(),
        col_failed_at = Columns::FailedAt.as_str(),
        col_fail_reason = Columns::FailReason.as_str(),
    );
    conn.execute(
        &sql,
        named_params! {
            ":id": args.id,
            ":failed_at": args.failed_at,
            ":fail_reason": args.fail_reason,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::super::super::test::conn;
    use super::super::schema::Sync;
    use time::OffsetDateTime;

    #[test]
    fn insert_started_at() -> crate::Result<()> {
        let conn = conn();

        let id = super::insert(&conn)?;

        assert!(id > 0);

        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM {}",
            Sync::projection(),
            super::schema::TABLE_NAME
        ))?;
        let sync = stmt.query_row([], Sync::mapper())?;

        assert_eq!(sync.id, id);
        assert!(!sync.started_at.is_empty());
        assert_eq!(sync.elements_affected, 0);
        assert_eq!(sync.elements_created, 0);
        assert_eq!(sync.elements_updated, 0);
        assert_eq!(sync.elements_deleted, 0);

        Ok(())
    }

    #[test]
    fn update_completed() -> crate::Result<()> {
        let conn = conn();

        let id = super::insert(&conn)?;
        let finished_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let duration_s = 10.5;
        let overpass_response_time_s = 2.3;
        let elements_created = 5;
        let elements_updated = 3;
        let elements_deleted = 2;

        let update_args = super::UpdateArgs {
            id,
            finished_at: finished_at.clone(),
            duration_s,
            overpass_response_time_s,
            elements_created,
            elements_updated,
            elements_deleted,
        };

        super::update(update_args, &conn)?;

        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM {}",
            Sync::projection(),
            super::schema::TABLE_NAME
        ))?;
        let sync = stmt.query_row([], Sync::mapper())?;

        assert_eq!(sync.id, id);
        assert_eq!(sync.finished_at, Some(finished_at));
        assert_eq!(sync.duration_s, Some(duration_s));
        assert_eq!(
            sync.overpass_response_time_s,
            Some(overpass_response_time_s)
        );
        assert_eq!(sync.elements_affected, 10);
        assert_eq!(sync.elements_created, 5);
        assert_eq!(sync.elements_updated, 3);
        assert_eq!(sync.elements_deleted, 2);

        Ok(())
    }
}
