DROP TRIGGER user_updated_at;

ALTER TABLE user RENAME TO user_old;

CREATE TABLE user (
    osm_json TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO user(
    rowid,
    osm_json,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    id,
    osm_json,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM user_old;

DROP TABLE user_old;

UPDATE user SET deleted_at = NULL where deleted_at = '';

CREATE TRIGGER user_updated_at UPDATE OF osm_json, tags, created_at, deleted_at ON user
BEGIN
    UPDATE user SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE rowid = old.rowid;
END;