ALTER TABLE area ADD COLUMN alias TEXT;

UPDATE area SET alias = json_extract(tags, '$.url_alias');

DROP TRIGGER area_updated_at;

CREATE TRIGGER area_updated_at UPDATE OF tags, alias, created_at, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;