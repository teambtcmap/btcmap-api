CREATE TABLE place_report(
    id INTEGER PRIMARY KEY NOT NULL,
    place_id INTEGER NOT NULL REFERENCES element(id),
    origin_id INTEGER NOT NULL REFERENCES place_import_origin(id),
    type TEXT NOT NULL,
    extra_fields TEXT NOT NULL DEFAULT (json_object()),
    ticket_url TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    closed_at TEXT,
    deleted_at TEXT
) STRICT;

CREATE TRIGGER place_report_updated_at UPDATE OF place_id, origin_id, type, extra_fields, ticket_url, created_at, closed_at, deleted_at ON place_report
BEGIN
    UPDATE place_report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX place_report_place_id ON place_report(place_id);