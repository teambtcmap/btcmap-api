CREATE TABLE place (
    id INTEGER PRIMARY KEY NOT NULL,
    place_id INTEGER NOT NULL,
    type TEXT NOT NULL,
    image_data BLOB NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ'))
) STRICT;

CREATE INDEX place_place_id ON place(place_id);
