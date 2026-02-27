DROP TRIGGER element_updated_at;

ALTER TABLE element RENAME TO element_old;

CREATE TABLE element (
    overpass_data TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO element(
    overpass_data,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    overpass_json,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM element_old;

DROP TABLE element_old;

CREATE TRIGGER element_updated_at UPDATE OF overpass_data, tags, created_at, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE rowid = old.rowid;
END;

CREATE INDEX idx_element_updated_at ON element(updated_at);
CREATE UNIQUE INDEX idx_element_overpass_data_type_and_id ON element(json_extract(overpass_data, '$.type'), json_extract(overpass_data, '$.id'));