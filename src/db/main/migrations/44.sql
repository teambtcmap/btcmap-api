DROP TRIGGER report_updated_at;

ALTER TABLE report RENAME TO report_old;

CREATE TABLE report (
    area_url_alias TEXT NOT NULL,
    date TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO report(
    rowid,
    area_url_alias,
    date,
    tags,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    id,
    area_url_alias,
    date,
    tags,
    created_at,
    updated_at,
    deleted_at
FROM report_old;

DROP TABLE report_old;

UPDATE report SET deleted_at = NULL where deleted_at = '';

CREATE TRIGGER report_updated_at UPDATE OF area_url_alias, date, tags, created_at, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE rowid = old.rowid;
END;