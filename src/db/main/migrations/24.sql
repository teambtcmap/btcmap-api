DROP TRIGGER report_updated_at;

CREATE TABLE report_new (
    id INTEGER PRIMARY KEY,
    area_id TEXT NOT NULL,
    date TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
) STRICT;

INSERT INTO report_new(
    id,
    area_id,
    date,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    ROWID,
    area_id,
    date,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM report;

DROP TABLE report;

ALTER TABLE report_new RENAME TO report;

CREATE TRIGGER report_updated_at UPDATE OF area_id, date, tags, created_at, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;