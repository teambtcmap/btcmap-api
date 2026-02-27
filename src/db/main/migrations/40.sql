DROP TRIGGER area_updated_at;

ALTER TABLE area RENAME TO area_old;

CREATE TABLE area (
    tags TEXT,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    deleted_at TEXT
) STRICT;

INSERT INTO area(
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    json_set(tags, '$.url_alias', id),
    created_at,
    updated_at,
    deleted_at
FROM area_old;

DROP TABLE area_old;

CREATE TRIGGER area_updated_at UPDATE OF tags, created_at, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE rowid = old.rowid;
END;

UPDATE area SET deleted_at = NULL where deleted_at = '';

ALTER TABLE report RENAME COLUMN area_id TO area_url_alias;