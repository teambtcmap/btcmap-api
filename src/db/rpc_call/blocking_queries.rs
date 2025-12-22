use crate::db::rpc_call::schema::{self, Columns, RpcCall};
use crate::Result;
use geojson::JsonObject;
use rusqlite::{named_params, Connection};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn insert(
    user_id: i64,
    ip: String,
    method: String,
    params: Option<JsonObject>,
    created_at: OffsetDateTime,
    processed_at: OffsetDateTime,
    conn: &Connection,
) -> Result<RpcCall> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {user_id},
                {ip},
                {method},
                {params_json},
                {created_at},
                {processed_at},
                {duration_ns}
            ) VALUES (
                :user_id,
                :ip,
                :method,
                :params_json,
                :created_at,
                :processed_at,
                :duration_ns
            )
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        user_id = Columns::UserId.as_str(),
        ip = Columns::Ip.as_str(),
        method = Columns::Method.as_str(),
        params_json = Columns::ParamsJson.as_str(),
        created_at = Columns::CreatedAt.as_str(),
        processed_at = Columns::ProcessedAt.as_str(),
        duration_ns = Columns::DurationNs.as_str(),
        projection = RpcCall::projection(),
    );
    let params = named_params! {
        ":user_id" : user_id,
        ":ip" : ip,
        ":method" : method,
        ":params_json" : params.map(|it| serde_json::to_string(&it).unwrap()),
        ":created_at" : created_at.format(&Rfc3339)?,
        ":processed_at" : processed_at.format(&Rfc3339)?,
        ":duration_ns" : (processed_at - created_at).whole_nanoseconds() as i64,
    };
    conn.query_row(&sql, params, RpcCall::mapper())
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use crate::db::test::conn;
    use serde_json::json;
    use std::time::Duration;
    use time::OffsetDateTime;

    #[test]
    fn insert() {
        let conn = conn();

        let user_id = 123;
        let ip = "192.168.1.100".to_string();
        let method = "get_element".to_string();
        let params = Some(
            json!({"id": 456, "name": "test"})
                .as_object()
                .unwrap()
                .clone(),
        );

        let created_at = OffsetDateTime::now_utc();
        let processed_at = created_at + Duration::from_millis(150);

        let result = super::insert(
            user_id,
            ip.clone(),
            method.clone(),
            params.clone(),
            created_at,
            processed_at,
            &conn,
        );

        assert!(result.is_ok(), "insert should succeed");

        let rpc_call = result.unwrap();

        assert_eq!(rpc_call.user_id, Some(user_id));
        assert_eq!(rpc_call.ip, ip);
        assert_eq!(rpc_call.method, method);
        assert!(rpc_call.params_json.is_some());
        assert_eq!(rpc_call.params_json, params);

        assert_eq!(rpc_call.created_at, created_at);
        assert_eq!(rpc_call.processed_at, processed_at);

        let expected_ns = Duration::from_millis(150).as_nanos() as i64;
        assert_eq!(rpc_call.duration_ns, expected_ns);
    }
}
