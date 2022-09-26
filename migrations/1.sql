CREATE TABLE element (
    id TEXT NOT NULL PRIMARY KEY,
    data TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT
);

CREATE TRIGGER element_updated_at UPDATE OF data, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;
