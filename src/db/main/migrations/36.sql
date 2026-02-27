DROP TRIGGER report_updated_at;

ALTER TABLE report RENAME TO report_old;

CREATE TABLE report (
    id INTEGER PRIMARY KEY NOT NULL,
    area_id TEXT NOT NULL,
    date TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT ( json_object() ),
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    updated_at TEXT NOT NULL UNIQUE DEFAULT ( strftime('%Y-%m-%dT%H:%M:%fZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

INSERT INTO report(
    id,
    area_id,
    date,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    id,
    area_id,
    date,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM report_old;

DROP TABLE report_old;

CREATE TRIGGER report_updated_at UPDATE OF area_id, date, tags, created_at, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;