CREATE TABLE og (
    element_id INTEGER PRIMARY KEY NOT NULL,
    version INTEGER NOT NULL,
    image_data BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ'))
) STRICT;

CREATE INDEX og_created_at ON og(created_at);