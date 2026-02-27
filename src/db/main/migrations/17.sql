ALTER TABLE area ADD COLUMN tags TEXT NOT NULL DEFAULT '{}';

CREATE TRIGGER area_updated_at UPDATE OF tags, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;