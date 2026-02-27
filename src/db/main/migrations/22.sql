CREATE TABLE event_new (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    element_id TEXT NOT NULL,
    type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

INSERT INTO event_new(
    id,
    user_id,
    element_id,
    type,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    ROWID,
    user_id,
    element_id,
    type,
    created_at,
    updated_at,
    deleted_at
FROM event;

DROP TABLE event;

ALTER TABLE event_new RENAME TO event;

CREATE TRIGGER event_updated_at UPDATE OF user_id, element_id, type, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;