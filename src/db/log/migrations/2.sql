CREATE TABLE sync (
    id INTEGER PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    finished_at TEXT,
    duration_s REAL,
    overpass_response_time_s REAL,
    elements_affected INTEGER NOT NULL DEFAULT 0,
    elements_created INTEGER NOT NULL DEFAULT 0,
    elements_updated INTEGER NOT NULL DEFAULT 0,
    elements_deleted INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE INDEX sync_started_at ON sync(started_at);
