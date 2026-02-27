UPDATE element SET deleted_at = '' WHERE deleted_at IS NULL;

DROP TRIGGER element_updated_at;

CREATE TABLE element_new (
    id TEXT PRIMARY KEY,
    osm_json TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) WITHOUT ROWID, STRICT;

INSERT INTO element_new(
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
FROM element;

DROP TABLE element;

ALTER TABLE element_new RENAME TO element;

CREATE TRIGGER element_updated_at UPDATE OF osm_json, tags, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;