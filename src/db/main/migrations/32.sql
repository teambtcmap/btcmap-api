DROP TRIGGER element_updated_at;

ALTER TABLE element RENAME TO element_old;

CREATE TABLE element (
    id TEXT PRIMARY KEY NOT NULL,
    osm_json TEXT NOT NULL DEFAULT ( json_object() ),
    tags TEXT NOT NULL DEFAULT ( json_object() ),
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    updated_at TEXT UNIQUE NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) WITHOUT ROWID, STRICT;

INSERT INTO element(
    id,
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
FROM element_old;

DROP TABLE element_old;

CREATE TRIGGER element_updated_at UPDATE OF osm_json, tags, created_at, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;