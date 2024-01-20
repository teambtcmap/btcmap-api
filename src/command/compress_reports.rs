use crate::{report::Report, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use tracing::info;

pub fn run(conn: &Connection) -> Result<()> {
    let reports: Vec<Report> = Report::select_all(None, conn)?
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();
    info!(count = reports.len(), "loaded reports");
    let mut deleted_reports = 0;
    let mut map: HashMap<i64, Vec<Report>> = HashMap::new();

    for report in reports {
        if !map.contains_key(&report.area_id) {
            map.insert(report.area_id.clone(), vec![]);
        }

        let prev_entries = map.get_mut(&report.area_id).unwrap();

        if prev_entries.last().is_none() || prev_entries.last().unwrap().tags != report.tags {
            prev_entries.push(report);
        } else {
            Report::delete_permanently(report.id, conn)?;
            deleted_reports = deleted_reports + 1;
        }
    }

    info!(deleted_reports, "finished report compression");
    Ok(())
}
