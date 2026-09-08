use crate::{
    db::{self},
    service::area::{Bbox, BboxGenerator},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use tracing::warn;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Res {
    pub areas_affected: i64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let areas = db::main::area::queries::select(None, true, None, pool).await?;
    let mut areas_affected = 0;
    for area in areas {
        let bbox = area.geo_json()?.bbox().unwrap();
        let saved_bbox = Bbox {
            west: area.bbox_west,
            south: area.bbox_south,
            east: area.bbox_east,
            north: area.bbox_north,
        };
        if bbox != saved_bbox {
            warn!(
                bbox = format!("{:?}", bbox),
                saved_bbox = format!("{:?}", saved_bbox),
                area.alias,
                "database has incorrect bbox"
            );
            db::main::area::queries::set_bbox(
                area.id, bbox.west, bbox.south, bbox.east, bbox.north, pool,
            )
            .await?;
            areas_affected += 1;
        }
    }
    Ok(Res { areas_affected })
}
