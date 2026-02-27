DROP TRIGGER event_updated_at;

DROP INDEX idx_event_updated_at;

DELETE FROM event WHERE user_id = 0;
DELETE FROM event WHERE user_id = 402907;
DELETE FROM event WHERE user_id = 630853;
DELETE FROM event WHERE user_id = 4345757;
DELETE FROM event WHERE user_id = 4547543;
DELETE FROM event WHERE user_id = 8866893;
DELETE FROM event WHERE user_id = 11810550;
DELETE FROM event WHERE user_id = 15587650;
DELETE FROM event WHERE user_id = 18332061;
DELETE FROM event WHERE user_id = 20113059;

ALTER TABLE event RENAME TO event_old;

CREATE TABLE event(
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES user(id),
    element_id INTEGER NOT NULL REFERENCES element(id),
    type TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO event(
    id,
    user_id,
    element_id,
    type,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    rowid,
    user_id,
    element_id,
    type,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM event_old;

DROP TABLE event_old;

CREATE TRIGGER event_updated_at UPDATE OF user_id, element_id, type, tags, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX event_updated_at ON event(updated_at);