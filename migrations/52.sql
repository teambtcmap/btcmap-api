DROP TRIGGER area_updated_at;

DROP INDEX idx_area_updated_at;

ALTER TABLE area RENAME TO area_old;

CREATE TABLE area(
    id INTEGER PRIMARY KEY NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO area(
    id,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    rowid,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM area_old;

DROP TABLE area_old;

CREATE TRIGGER area_updated_at UPDATE OF tags, created_at, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX area_updated_at ON area(updated_at);