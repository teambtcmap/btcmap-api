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
