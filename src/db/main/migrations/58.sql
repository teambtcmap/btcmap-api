DROP INDEX element_review_updated_at;

DROP TRIGGER element_review_updated_at;

ALTER TABLE element_review RENAME TO element_comment;

CREATE TRIGGER element_comment_updated_at UPDATE OF element_id, review, created_at, deleted_at ON element_comment
BEGIN
    UPDATE element_comment SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX element_comment_updated_at ON element_comment(updated_at);