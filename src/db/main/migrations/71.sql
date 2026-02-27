CREATE TABLE element_issue(
    id INTEGER PRIMARY KEY NOT NULL,
    element_id INTEGER NOT NULL REFERENCES element(id),
    code TEXT NOT NULL,
    severity INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE TRIGGER element_issue_updated_at UPDATE OF element_id, code, severity, created_at, deleted_at ON element_issue
BEGIN
    UPDATE element_issue SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX element_issue_element_id ON element_issue(element_id);
CREATE UNIQUE INDEX element_issue_element_id_code ON element_issue(element_id, code);
CREATE INDEX element_issue_updated_at ON element_issue(updated_at);
CREATE INDEX element_issue_deleted_at ON element_issue(deleted_at);
