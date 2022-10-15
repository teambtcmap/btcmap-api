CREATE TABLE area_new (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    min_lon REAL NOT NULL,
    min_lat REAL NOT NULL,
    max_lon REAL NOT NULL,
    max_lat REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
);

INSERT INTO area_new(
    id,
    name,
    type,
    min_lon,
    min_lat,
    max_lon,
    max_lat
)
select 
    id,
    name,
    type,
    min_lon,
    min_lat,
    max_lon,
    max_lat
from area;

DROP TABLE area;

ALTER TABLE area_new RENAME TO area;