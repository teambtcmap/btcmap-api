PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE og (
    element_id INTEGER PRIMARY KEY NOT NULL,
    version INTEGER NOT NULL,
    image_data BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ'))
, width INTEGER NOT NULL DEFAULT 0, height INTEGER NOT NULL DEFAULT 0, size_bytes INTEGER NOT NULL DEFAULT 0) STRICT;
CREATE TABLE area (
    id INTEGER PRIMARY KEY NOT NULL,
    area_id INTEGER NOT NULL,
    type TEXT NOT NULL,
    image_data BLOB NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ'))
) STRICT;
CREATE INDEX og_created_at ON og(created_at);
CREATE INDEX area_area_id ON area(area_id);
COMMIT;
