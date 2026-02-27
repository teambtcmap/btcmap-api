DROP TRIGGER report_updated_at;

ALTER TABLE report RENAME TO report_old;

CREATE TABLE report(
    id INTEGER PRIMARY KEY NOT NULL,
    area_id INTEGER NOT NULL REFERENCES area(id),
    date TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
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
    rowid,
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

CREATE INDEX report_updated_at ON report(updated_at);