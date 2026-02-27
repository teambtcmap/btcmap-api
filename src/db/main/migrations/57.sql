CREATE TABLE element_review(
    id INTEGER PRIMARY KEY NOT NULL,
    element_id INTEGER NOT NULL REFERENCES element(id),
    review TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE TRIGGER element_review_updated_at UPDATE OF element_id, review, created_at, deleted_at ON element_review
BEGIN
    UPDATE element_review SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX element_review_updated_at ON element_review(updated_at);