DROP TRIGGER event_updated_at;

ALTER TABLE event RENAME TO event_old;

CREATE TABLE event (
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    element_id TEXT NOT NULL,
    type TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

INSERT INTO event(
    id,
    user_id,
    element_id,
    type,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    id,
    user_id,
    element_id,
    type,
    created_at,
    updated_at,
    deleted_at
FROM event_old;

DROP TABLE event_old;

CREATE TRIGGER event_updated_at UPDATE OF user_id, element_id, type, tags, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;