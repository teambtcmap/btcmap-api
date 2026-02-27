CREATE TABLE place_submission(
    id INTEGER PRIMARY KEY NOT NULL,
    origin TEXT NOT NULL,
    external_id TEXT NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    extra_fields TEXT NOT NULL DEFAULT (json_object()),
    ticket_url TEXT,
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    closed_at TEXT,
    deleted_at TEXT
) STRICT;

CREATE TRIGGER place_submission_updated_at UPDATE OF origin, external_id, lat, lon, category, name, extra_fields, ticket_url, revoked, created_at, closed_at, deleted_at ON place_submission
BEGIN
    UPDATE place_submission SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE UNIQUE INDEX place_submission_origin_external_id ON place_submission(origin, external_id);