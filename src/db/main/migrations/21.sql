DROP TRIGGER user_updated_at;

UPDATE user SET deleted_at = '' WHERE deleted_at IS NULL;

CREATE TABLE user_new (
    id INTEGER PRIMARY KEY,
    osm_json TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

INSERT INTO user_new(
    id,
    osm_json,
    created_at,
    deleted_at
)
SELECT
    id,
    data,
    created_at,
    deleted_at
FROM user;

DROP TABLE user;

ALTER TABLE user_new RENAME TO user;

CREATE TRIGGER user_updated_at UPDATE OF osm_json, tags, deleted_at ON user
BEGIN
    UPDATE user SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;