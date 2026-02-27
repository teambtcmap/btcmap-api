DROP INDEX element_comment_updated_at;

DROP TRIGGER element_comment_updated_at;

ALTER TABLE element_comment RENAME COLUMN review TO comment;

CREATE TRIGGER element_comment_updated_at UPDATE OF element_id, comment, created_at, deleted_at ON element_comment
BEGIN
    UPDATE element_comment SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX element_comment_updated_at ON element_comment(updated_at);