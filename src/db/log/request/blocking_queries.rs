use super::schema::{self, Columns};
use crate::Result;
use rusqlite::{named_params, Connection};

pub struct InsertArgs {
    pub ip: String,
    pub user_agent: Option<String>,
    pub user_id: Option<i64>,
    pub path: String,
    pub query: Option<String>,
    pub body: Option<String>,
    pub response_code: i64,
    pub processing_time_ns: i64,
}

pub fn insert(request: InsertArgs, conn: &Connection) -> Result<()> {
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
            ":ip": request.ip,
            ":user_agent": request.user_agent,
            ":user_id": request.user_id,
            ":path": request.path,
            ":query": request.query,
            ":body": request.body,
            ":response_code": request.response_code,
            ":processing_time_ns": request.processing_time_ns,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::log::request::blocking_queries::InsertArgs;
    use crate::db::log::request::schema::Request;
    use crate::db::log::test::conn;

    #[test]
    fn insert() -> crate::Result<()> {
        let conn = conn();

        super::insert(
            InsertArgs {
                ip: "192.168.1.1".to_string(),
                user_agent: Some("Mozilla/5.0".to_string()),
                user_id: Some(123),
                path: "/api/v1/places".to_string(),
                query: Some("lat=40.7128&lon=-74.0060".to_string()),
                body: Some(r#"{"key": "value"}"#.to_string()),
                response_code: 200,
                processing_time_ns: 15000000,
            },
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
            InsertArgs {
                ip: "10.0.0.1".to_string(),
                user_agent: None,
                user_id: None,
                path: "/api/v1/status".to_string(),
                query: None,
                body: None,
                response_code: 404,
                processing_time_ns: 5000000,
            },
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
            InsertArgs {
                ip: "192.168.1.1".to_string(),
                user_agent: Some("Mozilla/5.0".to_string()),
                user_id: Some(1),
                path: "/api/v1/places".to_string(),
                query: None,
                body: None,
                response_code: 200,
                processing_time_ns: 1000000,
            },
            &conn,
        )?;

        super::insert(
            InsertArgs {
                ip: "192.168.1.2".to_string(),
                user_agent: Some("curl/7.68.0".to_string()),
                user_id: None,
                path: "/api/v1/users".to_string(),
                query: None,
                body: None,
                response_code: 401,
                processing_time_ns: 2000000,
            },
            &conn,
        )?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM request", [], |row| row.get(0))?;
        assert_eq!(count, 2);

        Ok(())
    }
}
