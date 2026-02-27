DROP INDEX event_updated_at;
CREATE INDEX element_event_updated_at ON element_event(updated_at);

DROP TRIGGER event_updated_at;
CREATE TRIGGER element_event_updated_at UPDATE OF user_id, element_id, type, tags, created_at, deleted_at ON element_event
BEGIN
    UPDATE element_event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE TABLE event(
    id INTEGER PRIMARY KEY NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    name TEXT NOT NULL,
    website TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE TRIGGER event_updated_at UPDATE OF lat, lon, name, website, starts_at, ends_at, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;