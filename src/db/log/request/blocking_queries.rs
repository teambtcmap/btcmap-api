use super::schema::{self, Columns};
use crate::Result;
use rusqlite::{named_params, Connection};
use std::time::Instant;
use time::{Duration, OffsetDateTime};
use tracing::info;

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

#[allow(dead_code)]
pub fn select_latest(minutes: i64, conn: &Connection) -> Result<Vec<schema::Request>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {date} > strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-{minutes} minutes')
            ORDER BY {date} DESC
        "#,
        projection = schema::Request::projection(),
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        minutes = minutes,
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], schema::Request::mapper())?;
    let mut requests = Vec::new();
    for row in rows {
        requests.push(row?);
    }
    Ok(requests)
}

pub struct DailyInfraReport {
    pub total_requests: i64,
    pub unique_ips: i64,
    pub web_requests: i64,
    pub web_unique_ips: i64,
    pub android_requests: i64,
    pub android_unique_ips: i64,
    pub ios_requests: i64,
    pub ios_unique_ips: i64,
}

pub fn select_daily_infra_report(conn: &Connection) -> Result<DailyInfraReport> {
    let sql_total = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {ip})
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
    );
    let (total_requests, unique_ips): (i64, i64) =
        conn.query_row(&sql_total, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let sql_web = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {ip})
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {user_agent} = 'btcmap.org'
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );
    let (web_requests, web_unique_ips): (i64, i64) =
        conn.query_row(&sql_web, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let sql_android = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {ip})
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {user_agent} LIKE 'BTC Map Android%'
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );
    let (android_requests, android_unique_ips): (i64, i64) =
        conn.query_row(&sql_android, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let sql_ios = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {ip})
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {user_agent} LIKE '%CFNetwork%'
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );
    let (ios_requests, ios_unique_ips): (i64, i64) =
        conn.query_row(&sql_ios, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    Ok(DailyInfraReport {
        total_requests,
        unique_ips,
        web_requests,
        web_unique_ips,
        android_requests,
        android_unique_ips,
        ios_requests,
        ios_unique_ips,
    })
}

pub struct TopUserAgent {
    pub user_agent: String,
    pub count: i64,
    pub unique_ips: i64,
}

pub struct TopClientsReport {
    pub web: PlatformClientStats,
    pub android: PlatformClientStats,
    pub ios: PlatformClientStats,
}

pub struct PlatformClientStats {
    pub total_requests: i64,
    pub unique_ips: i64,
    pub top_ips: Vec<TopIp>,
}

pub struct TopIp {
    pub ip: String,
    pub count: i64,
}

pub fn select_top_user_agents(conn: &Connection) -> Result<Vec<TopUserAgent>> {
    let overall_start = Instant::now();

    let since_date = (OffsetDateTime::now_utc() - Duration::hours(24))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
        .to_string();

    let sql_top = format!(
        r#"
            SELECT {user_agent}, COUNT(*) as count
            FROM {table}
            WHERE {date} >= ?1
            AND {user_agent} IS NOT NULL
            GROUP BY {user_agent}
            ORDER BY count DESC
            LIMIT 10
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );

    let top_agents: Vec<(String, i64)> = {
        let start = Instant::now();
        let mut stmt = conn.prepare(&sql_top)?;
        let rows = stmt.query_map([&since_date], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let result = rows.collect::<Result<Vec<_>, _>>()?;
        info!(
            elapsed_ms = start.elapsed().as_millis() as u64,
            count = result.len(),
            "top_user_agents: first query (get top 10 user agents)"
        );
        result
    };

    if top_agents.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<String> = top_agents.iter().map(|_| "?".to_string()).collect();
    let sql_unique = format!(
        r#"
            SELECT {user_agent}, COUNT(DISTINCT {ip}) as unique_ips
            FROM {table}
            WHERE {date} >= ?1
            AND {user_agent} IN ({})
            GROUP BY {user_agent}
        "#,
        placeholders.join(", "),
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        user_agent = Columns::UserAgent.as_str(),
        ip = Columns::Ip.as_str(),
    );

    let unique_ips: std::collections::HashMap<String, i64> = {
        let start = Instant::now();
        let mut stmt = conn.prepare(&sql_unique)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&since_date as &dyn rusqlite::ToSql];
        params.extend(top_agents.iter().map(|(ua, _)| ua as &dyn rusqlite::ToSql));
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let result = rows
            .collect::<Result<Vec<(String, i64)>, _>>()?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        info!(
            elapsed_ms = start.elapsed().as_millis() as u64,
            count = result.len(),
            "top_user_agents: second query (get unique IPs for {} agents)",
            top_agents.len()
        );
        result
    };

    let result = top_agents
        .into_iter()
        .map(|(user_agent, count)| TopUserAgent {
            user_agent: user_agent.clone(),
            count,
            unique_ips: *unique_ips.get(&user_agent).unwrap_or(&0),
        })
        .collect();

    info!(
        elapsed_ms = overall_start.elapsed().as_millis() as u64,
        "top_user_agents: total"
    );

    Ok(result)
}

pub fn select_top_clients(conn: &Connection) -> Result<TopClientsReport> {
    let sql_web = format!(
        r#"
            SELECT {ip}, COUNT(*) as count
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {user_agent} = 'btcmap.org'
            GROUP BY {ip}
            ORDER BY count DESC
            LIMIT 10
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );
    let web_top_ips: Vec<TopIp> = {
        let mut stmt = conn.prepare(&sql_web)?;
        let rows = stmt.query_map([], |row| {
            Ok(TopIp {
                ip: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let (web_requests, web_unique_ips): (i64, i64) = conn.query_row(
        &format!(
            r#"
                SELECT COUNT(*), COUNT(DISTINCT {ip})
                FROM {table}
                WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                AND {user_agent} = 'btcmap.org'
            "#,
            table = schema::TABLE_NAME,
            date = Columns::Date.as_str(),
            ip = Columns::Ip.as_str(),
            user_agent = Columns::UserAgent.as_str(),
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let sql_android = format!(
        r#"
            SELECT {ip}, COUNT(*) as count
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {user_agent} LIKE 'BTC Map Android%'
            GROUP BY {ip}
            ORDER BY count DESC
            LIMIT 10
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );
    let android_top_ips: Vec<TopIp> = {
        let mut stmt = conn.prepare(&sql_android)?;
        let rows = stmt.query_map([], |row| {
            Ok(TopIp {
                ip: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let (android_requests, android_unique_ips): (i64, i64) = conn.query_row(
        &format!(
            r#"
                SELECT COUNT(*), COUNT(DISTINCT {ip})
                FROM {table}
                WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                AND {user_agent} LIKE 'BTC Map Android%'
            "#,
            table = schema::TABLE_NAME,
            date = Columns::Date.as_str(),
            ip = Columns::Ip.as_str(),
            user_agent = Columns::UserAgent.as_str(),
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let sql_ios = format!(
        r#"
            SELECT {ip}, COUNT(*) as count
            FROM {table}
            WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {user_agent} LIKE '%CFNetwork%'
            GROUP BY {ip}
            ORDER BY count DESC
            LIMIT 10
        "#,
        table = schema::TABLE_NAME,
        date = Columns::Date.as_str(),
        ip = Columns::Ip.as_str(),
        user_agent = Columns::UserAgent.as_str(),
    );
    let ios_top_ips: Vec<TopIp> = {
        let mut stmt = conn.prepare(&sql_ios)?;
        let rows = stmt.query_map([], |row| {
            Ok(TopIp {
                ip: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let (ios_requests, ios_unique_ips): (i64, i64) = conn.query_row(
        &format!(
            r#"
                SELECT COUNT(*), COUNT(DISTINCT {ip})
                FROM {table}
                WHERE {date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                AND {user_agent} LIKE '%CFNetwork%'
            "#,
            table = schema::TABLE_NAME,
            date = Columns::Date.as_str(),
            ip = Columns::Ip.as_str(),
            user_agent = Columns::UserAgent.as_str(),
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(TopClientsReport {
        web: PlatformClientStats {
            total_requests: web_requests,
            unique_ips: web_unique_ips,
            top_ips: web_top_ips,
        },
        android: PlatformClientStats {
            total_requests: android_requests,
            unique_ips: android_unique_ips,
            top_ips: android_top_ips,
        },
        ios: PlatformClientStats {
            total_requests: ios_requests,
            unique_ips: ios_unique_ips,
            top_ips: ios_top_ips,
        },
    })
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

    #[test]
    fn select_latest() -> crate::Result<()> {
        let conn = conn();

        conn.execute(
            "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.1', '/api/v1/old', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 minutes'))",
            [],
        )?;

        conn.execute(
            "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.2', '/api/v1/recent', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )?;

        let requests = super::select_latest(1, &conn)?;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].ip, "10.0.0.2");
        assert_eq!(requests[0].path, "/api/v1/recent");

        let requests = super::select_latest(5, &conn)?;
        assert_eq!(requests.len(), 2);

        Ok(())
    }
}
