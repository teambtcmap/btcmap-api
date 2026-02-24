use super::schema::{self, Columns};
use crate::Result;
use rusqlite::{named_params, Connection};

pub fn insert(
    ip: &str,
    user_agent: Option<&str>,
    user_id: Option<i64>,
    path: &str,
    query: Option<&str>,
    body: Option<&str>,
    response_code: i64,
    processing_time_ns: i64,
    conn: &Connection,
) -> Result<()> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {col_ip},
                {col_user_agent},
                {col_user_id},
                {col_path}, 
                {col_query},
                {col_body},
                {col_response_code},
                {col_processing_time_ns}
            ) VALUES (
                :{col_ip},
                :{col_user_agent},
                :{col_user_id},
                :{col_path},
                :{col_query},
                :{col_body},  
                :{col_response_code},
                :{col_processing_time_ns}
             );
         "#,
        table = schema::TABLE_NAME,
        col_ip = Columns::Ip.as_str(),
        col_user_agent = Columns::UserAgent.as_str(),
        col_user_id = Columns::UserId.as_str(),
        col_path = Columns::Path.as_str(),
        col_query = Columns::Query.as_str(),
        col_body = Columns::Body.as_str(),
        col_response_code = Columns::ResponseCode.as_str(),
        col_processing_time_ns = Columns::ProcessingTimeNs.as_str(),
    );
    conn.execute(
        &sql,
        named_params! {
            ":ip": ip,
            ":user_agent": user_agent,
            ":user_id": user_id,
            ":path": path,
            ":query": query,
            ":body": body,
            ":response_code": response_code,
            ":processing_time_ns": processing_time_ns,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::log::request::schema::Request;
    use crate::db::log::test::conn;

    #[test]
    fn insert() -> crate::Result<()> {
        let conn = conn();

        super::insert(
            "192.168.1.1",
            Some("Mozilla/5.0"),
            Some(123),
            "/api/v1/places",
            Some("lat=40.7128&lon=-74.0060"),
            Some(r#"{"key": "value"}"#),
            200,
            15000000,
            &conn,
        )?;

        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM {}",
            Request::projection(),
            crate::db::log::request::schema::TABLE_NAME
        ))?;
        let request = stmt.query_row([], Request::mapper())?;

        assert_eq!(request.ip, "192.168.1.1");
        assert_eq!(request.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(request.user_id, Some(123));
        assert_eq!(request.path, "/api/v1/places");
        assert_eq!(request.query, Some("lat=40.7128&lon=-74.0060".to_string()));
        assert_eq!(request.body, Some(r#"{"key": "value"}"#.to_string()));
        assert_eq!(request.response_code, 200);
        assert_eq!(request.processing_time_ns, 15000000);

        Ok(())
    }

    #[test]
    fn insert_minimal() -> crate::Result<()> {
        let conn = conn();

        super::insert(
            "10.0.0.1",
            None,
            None,
            "/api/v1/status",
            None,
            None,
            404,
            5000000,
            &conn,
        )?;

        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM {}",
            Request::projection(),
            crate::db::log::request::schema::TABLE_NAME
        ))?;
        let request = stmt.query_row([], Request::mapper())?;

        assert_eq!(request.ip, "10.0.0.1");
        assert_eq!(request.user_agent, None);
        assert_eq!(request.user_id, None);
        assert_eq!(request.path, "/api/v1/status");
        assert_eq!(request.query, None);
        assert_eq!(request.body, None);
        assert_eq!(request.response_code, 404);
        assert_eq!(request.processing_time_ns, 5000000);

        Ok(())
    }

    #[test]
    fn insert_multiple() -> crate::Result<()> {
        let conn = conn();

        super::insert(
            "192.168.1.1",
            Some("Mozilla/5.0"),
            Some(1),
            "/api/v1/places",
            None,
            None,
            200,
            1000000,
            &conn,
        )?;

        super::insert(
            "192.168.1.2",
            Some("curl/7.68.0"),
            None,
            "/api/v1/users",
            None,
            None,
            401,
            2000000,
            &conn,
        )?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM request", [], |row| row.get(0))?;
        assert_eq!(count, 2);

        Ok(())
    }
}
