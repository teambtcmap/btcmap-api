use super::schema::{self};
use crate::Result;
use rusqlite::{named_params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;
use std::time::Instant;
use time::{Duration, OffsetDateTime};
use tracing::info;

pub struct InsertArgs {
    pub ip: String,
    pub user_agent: Option<String>,
    pub user_id: Option<i64>,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub body: Option<String>,
    pub response_code: i64,
    pub processing_time_ns: i64,
}

pub fn insert(request: InsertArgs, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} (
                {Ip},
                {UserAgent},
                {UserId},
                {Method},
                {Path},
                {Query},
                {Body},
                {ResponseCode},
                {ProcessingTimeNs}
            ) VALUES (
                :{Ip},
                :{UserAgent},
                :{UserId},
                :{Method},
                :{Path},
                :{Query},
                :{Body},
                :{ResponseCode},
                :{ProcessingTimeNs}
             );
          "#,
    );
    conn.execute(
        &sql,
        named_params! {
            ":ip": request.ip,
            ":user_agent": request.user_agent,
            ":user_id": request.user_id,
            ":method": request.method,
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
pub fn select_count_since(since: OffsetDateTime, conn: &Connection) -> Result<i64> {
    let since = since
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(crate::Error::from)?;
    let sql = format!(
        r#"
            SELECT COUNT(*)
            FROM {TABLE}
            WHERE {Date} > ?1
        "#,
    );
    conn.query_row(&sql, [&since], |row| row.get(0))
        .map_err(Into::into)
}

#[allow(dead_code)]
pub fn select_latest(minutes: i64, conn: &Connection) -> Result<Vec<schema::Request>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {Date} > strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-{minutes} minutes')
            ORDER BY {Date} DESC
        "#,
        projection = schema::Request::projection(),
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
            SELECT COUNT(*), COUNT(DISTINCT {Ip})
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
        "#,
    );
    let (total_requests, unique_ips): (i64, i64) =
        conn.query_row(&sql_total, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let sql_web = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {Ip})
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {UserAgent} = 'btcmap.org'
        "#,
    );
    let (web_requests, web_unique_ips): (i64, i64) =
        conn.query_row(&sql_web, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let sql_android = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {Ip})
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {UserAgent} LIKE 'BTC Map Android%'
        "#,
    );
    let (android_requests, android_unique_ips): (i64, i64) =
        conn.query_row(&sql_android, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let sql_ios = format!(
        r#"
            SELECT COUNT(*), COUNT(DISTINCT {Ip})
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {UserAgent} LIKE '%CFNetwork%'
        "#,
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

pub struct TopRpcMethod {
    pub method: String,
    pub count: i64,
}

pub fn select_top_rpc_methods(
    since: OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<TopRpcMethod>> {
    let since = since
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(crate::Error::from)?;
    let sql = format!(
        r#"
            SELECT rpc_method, COUNT(*) AS count
            FROM {TABLE}
            WHERE {Date} > ?1
              AND path = '/rpc'
              AND rpc_method IS NOT NULL
            GROUP BY rpc_method
            ORDER BY count DESC
            LIMIT 10
        "#,
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([&since], |row| {
        Ok(TopRpcMethod {
            method: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub struct TopRestApiCall {
    pub method: String,
    pub path: String,
    pub count: i64,
}

pub fn select_top_rest_api_calls(
    since: OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<TopRestApiCall>> {
    let since = since
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(crate::Error::from)?;
    let sql = format!(
        r#"
            SELECT {Method}, {Path}, COUNT(*) AS count
            FROM {TABLE}
            WHERE {Date} > ?1
              AND ({Path} LIKE '/v%' OR {Path} LIKE '/feeds%')
            GROUP BY {Method}, {Path}
            ORDER BY count DESC, {Path} ASC, {Method} ASC
            LIMIT 10
        "#,
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([&since], |row| {
        Ok(TopRestApiCall {
            method: row.get(0)?,
            path: row.get(1)?,
            count: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub struct PlatformUniqueIps24h {
    pub web: i64,
    pub android: i64,
    pub ios: i64,
    pub other_humans: i64,
    pub bots: i64,
}

pub fn select_platform_unique_ips_24h(conn: &Connection) -> Result<PlatformUniqueIps24h> {
    // Assign each distinct IP a single platform based on its User-Agent(s) seen in the window:
    //   1. bot            if any request from the IP matched a known bot signature
    //   2. android        if any request matched an official Android client UA (current or old fork)
    //   3. ios            if any request matched an official iOS client UA
    //   4. web            if any request matched the official web client UA
    //   5. other_humans   otherwise (regular browsers, curl/scripts with no bot signature,
    //                      or clients whose UA is missing/empty)
    // The bucketing is mutually exclusive per IP and prefers the most specific platform match.
    let sql = format!(
        r#"
            WITH ip_ua AS (
                SELECT DISTINCT {Ip} AS ip,
                    group_concat(DISTINCT {UserAgent}) AS uas
                FROM {TABLE}
                WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                GROUP BY {Ip}
            )
            SELECT
                COALESCE(SUM(CASE WHEN classify = 'web' THEN 1 ELSE 0 END), 0) AS web,
                COALESCE(SUM(CASE WHEN classify = 'android' THEN 1 ELSE 0 END), 0) AS android,
                COALESCE(SUM(CASE WHEN classify = 'ios' THEN 1 ELSE 0 END), 0) AS ios,
                COALESCE(SUM(CASE WHEN classify = 'other_humans' THEN 1 ELSE 0 END), 0) AS other_humans,
                COALESCE(SUM(CASE WHEN classify = 'bots' THEN 1 ELSE 0 END), 0) AS bots
            FROM (
                SELECT
                    CASE
                        WHEN uas LIKE '%bot%' COLLATE NOCASE
                          OR uas LIKE '%spider%' COLLATE NOCASE
                          OR uas LIKE '%crawler%' COLLATE NOCASE
                          OR uas LIKE 'Zapier%' COLLATE NOCASE
                          OR uas LIKE 'Twitterbot%' COLLATE NOCASE
                          OR uas LIKE 'facebookexternalhit%' COLLATE NOCASE
                          OR uas LIKE 'meta-externalagent%' COLLATE NOCASE
                          OR uas LIKE 'Applebot%' COLLATE NOCASE
                          OR uas LIKE 'AhrefsBot%' COLLATE NOCASE
                          OR uas LIKE 'SemrushBot%' COLLATE NOCASE
                          OR uas LIKE 'DuckDuckBot%' COLLATE NOCASE
                          OR uas LIKE 'Bytespider%' COLLATE NOCASE
                          OR uas LIKE 'btcmap-e2e-tests%' COLLATE NOCASE
                        THEN 'bots'
                        WHEN uas LIKE 'BTC Map Android%' OR uas LIKE 'okhttp/5.0.0-alpha.14' THEN 'android'
                        WHEN uas LIKE '%CFNetwork%' THEN 'ios'
                        WHEN uas LIKE 'btcmap.org' THEN 'web'
                        ELSE 'other_humans'
                    END AS classify
                FROM ip_ua
            )
        "#,
    );
    let (web, android, ios, other_humans, bots): (i64, i64, i64, i64, i64) =
        conn.query_row(&sql, [], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
    Ok(PlatformUniqueIps24h {
        web,
        android,
        ios,
        other_humans,
        bots,
    })
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
            SELECT {UserAgent}, COUNT(*) as count
            FROM {TABLE}
            WHERE {Date} >= ?1
            AND {UserAgent} IS NOT NULL
            GROUP BY {UserAgent}
            ORDER BY count DESC
            LIMIT 10
        "#,
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
            SELECT {UserAgent}, COUNT(DISTINCT {Ip}) as unique_ips
            FROM {TABLE}
            WHERE {Date} >= ?1
            AND {UserAgent} IN ({})
            GROUP BY {UserAgent}
        "#,
        placeholders.join(", "),
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
            SELECT {Ip}, COUNT(*) as count
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {UserAgent} = 'btcmap.org'
            GROUP BY {Ip}
            ORDER BY count DESC
            LIMIT 10
        "#,
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
                SELECT COUNT(*), COUNT(DISTINCT {Ip})
                FROM {TABLE}
                WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                AND {UserAgent} = 'btcmap.org'
            "#,
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let sql_android = format!(
        r#"
            SELECT {Ip}, COUNT(*) as count
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {UserAgent} LIKE 'BTC Map Android%'
            GROUP BY {Ip}
            ORDER BY count DESC
            LIMIT 10
        "#,
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
                SELECT COUNT(*), COUNT(DISTINCT {Ip})
                FROM {TABLE}
                WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                AND {UserAgent} LIKE 'BTC Map Android%'
            "#,
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let sql_ios = format!(
        r#"
            SELECT {Ip}, COUNT(*) as count
            FROM {TABLE}
            WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
            AND {UserAgent} LIKE '%CFNetwork%'
            GROUP BY {Ip}
            ORDER BY count DESC
            LIMIT 10
        "#,
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
                SELECT COUNT(*), COUNT(DISTINCT {Ip})
                FROM {TABLE}
                WHERE {Date} >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-24 hours')
                AND {UserAgent} LIKE '%CFNetwork%'
            "#,
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
                method: "GET".to_string(),
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
                method: "GET".to_string(),
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
                method: "GET".to_string(),
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
                method: "POST".to_string(),
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

    #[test]
    fn select_count_since() -> crate::Result<()> {
        let conn = conn();

        conn.execute(
            "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.1', '/api/v1/old', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 hours'))",
            [],
        )?;
        conn.execute(
            "INSERT INTO request (ip, path, response_code, processing_time_ns, date) VALUES ('10.0.0.2', '/api/v1/recent', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )?;

        let now = time::OffsetDateTime::now_utc();
        let count = super::select_count_since(now - time::Duration::hours(1), &conn)?;
        assert_eq!(1, count);
        let count = super::select_count_since(now - time::Duration::hours(3), &conn)?;
        assert_eq!(2, count);
        let count = super::select_count_since(now + time::Duration::hours(1), &conn)?;
        assert_eq!(0, count);

        Ok(())
    }

    #[test]
    fn select_platform_unique_ips_24h_empty() -> crate::Result<()> {
        let conn = conn();
        let report = super::select_platform_unique_ips_24h(&conn)?;
        assert_eq!(0, report.web);
        assert_eq!(0, report.android);
        assert_eq!(0, report.ios);
        assert_eq!(0, report.other_humans);
        assert_eq!(0, report.bots);
        Ok(())
    }

    #[test]
    fn select_platform_unique_ips_24h() -> crate::Result<()> {
        let conn = conn();

        let insert = |ip: &str, ua: Option<&str>| {
            conn.execute(
                "INSERT INTO request (ip, user_agent, path, response_code, processing_time_ns, date) VALUES (?1, ?2, '/v4/places', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                rusqlite::params![ip, ua],
            )
        };

        // web
        insert("10.0.0.1", Some("btcmap.org"))?;
        insert("10.0.0.2", Some("btcmap.org"))?;
        insert("10.0.0.3", Some("btcmap.org"))?;
        // android (current)
        insert("10.0.0.4", Some("BTC Map Android 56"))?;
        insert("10.0.0.5", Some("BTC Map Android 57"))?;
        insert("10.0.0.5", Some("BTC Map Android 57"))?;
        // android (old fork)
        insert("10.0.0.20", Some("okhttp/5.0.0-alpha.14"))?;
        insert("10.0.0.21", Some("okhttp/5.0.0-alpha.14"))?;
        // ios
        insert(
            "10.0.0.6",
            Some("BTCMap/19 CFNetwork/1494.0.7 Darwin/23.4.0"),
        )?;
        // other_humans: cli, mobile Safari, NULL UA, browsers
        insert("10.0.0.7", Some("curl/8.5.0"))?;
        insert(
            "10.0.0.8",
            Some("Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X)"),
        )?;
        insert("10.0.0.9", None)?;
        // bots
        insert("10.0.0.10", Some("Amazonbot/0.1"))?;
        insert("10.0.0.11", Some("Mozilla/5.0 (compatible; Googlebot/2.1)"))?;
        insert("10.0.0.12", Some("Baiduspider-render/2.0"))?;
        insert("10.0.0.13", Some("btcmap-e2e-tests/1.0"))?;
        // mixed IP: bot UA once and human UA another time -> bots wins (priority)
        insert(
            "10.0.0.14",
            Some("Mozilla/5.0 (Windows NT 10.0; Chrome/149)"),
        )?;
        insert("10.0.0.14", Some("Applebot/0.1"))?;
        // dedup web IP
        insert("10.0.0.3", Some("btcmap.org"))?;

        // out-of-window: should not count
        conn.execute(
            "INSERT INTO request (ip, user_agent, path, response_code, processing_time_ns, date) VALUES ('10.0.0.99', 'btcmap.org', '/v4/places', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 days'))",
            [],
        )?;
        conn.execute(
            "INSERT INTO request (ip, user_agent, path, response_code, processing_time_ns, date) VALUES ('10.0.0.98', 'okhttp/5.0.0-alpha.14', '/v4/places', 200, 1000000, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-2 days'))",
            [],
        )?;

        let report = super::select_platform_unique_ips_24h(&conn)?;
        assert_eq!(
            3, report.web,
            "web counts distinct btcmap.org IPs in the last 24h"
        );
        assert_eq!(
            4, report.android,
            "android counts distinct IPs across current and old-fork clients (2 + 2)"
        );
        assert_eq!(1, report.ios, "ios counts distinct CFNetwork IPs");
        assert_eq!(
            3, report.other_humans,
            "other_humans: curl, mobile Safari, NULL UA"
        );
        assert_eq!(
            5, report.bots,
            "bots: dedicated bot IPs (4) plus the mixed IP (1) that also did a human request"
        );
        Ok(())
    }
}
