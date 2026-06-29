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

CREATE INDEX area_area_id ON area(area_id);