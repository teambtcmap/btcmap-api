DROP TRIGGER event_updated_at;

ALTER TABLE event RENAME TO event_old;

CREATE TABLE event (
    user_id INTEGER NOT NULL,
    element_id INTEGER NOT NULL,
    type TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO event(
    rowid,
    user_id,
    element_id,
    type,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    rowid,
    user_id,
    (select rowid from element el where json_extract(el.overpass_data, '$.type') || ':' || json_extract(el.overpass_data, '$.id') = element_id),
    type,
    created_at,
    updated_at,
    deleted_at
FROM event_old;

DROP TABLE event_old;

UPDATE event SET deleted_at = NULL where deleted_at = '';

CREATE TRIGGER event_updated_at UPDATE OF user_id, element_id, type, tags, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE rowid = old.rowid;
END;

CREATE INDEX idx_event_updated_at ON event(updated_at);
CREATE INDEX idx_report_updated_at ON report(updated_at);
CREATE INDEX idx_user_updated_at ON user(updated_at);