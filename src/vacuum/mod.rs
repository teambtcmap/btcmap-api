use crate::{area::Area, Result};
use rusqlite::Connection;
use std::{thread::sleep, time::Duration};
use tracing::warn;

struct VacuumResult {
    nulls_removed: i32,
    useless_strings_removed: i32,
}

pub fn vacuum_areas(conn: &Connection) -> Result<()> {
    let mut nulls_removed = 0;
    let mut useless_strings_removed = 0;
    let areas: Vec<Area> = Area::select_all(&conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();
    for area in areas {
        let res = vacuum_area(&area, conn)?;
        nulls_removed += res.nulls_removed;
        useless_strings_removed += res.useless_strings_removed;
        sleep(Duration::from_millis(1));
    }
    warn!(nulls_removed, useless_strings_removed);
    Ok(())
}

fn vacuum_area(area: &Area, conn: &Connection) -> Result<VacuumResult> {
    let mut nulls_removed = 0;
    let mut useless_strings_removed = 0;
    let tags = &area.tags;

    for key in tags.keys() {
        let value = tags.get(key).unwrap();

        if value.is_null() {
            warn!(area_id = area.id, key, "Area tag is null");
            Area::remove_tag(area.id, key, conn)?;
            nulls_removed += 1;
        }

        if value.is_string() {
            let value = value.as_str().unwrap_or("");

            if value == "" {
                warn!(area_id = area.id, key, value, "Area tag is useless string");
                Area::remove_tag(area.id, key, conn)?;
                useless_strings_removed += 1;
            }
        }
    }

    return Ok(VacuumResult {
        nulls_removed,
        useless_strings_removed,
    });
}
