DROP TRIGGER report_updated_at;

UPDATE report SET area_url_alias = 'earth' WHERE area_url_alias = '';

ALTER TABLE report ADD COLUMN area_id INTEGER;

CREATE INDEX idx_area_tags_url_alias ON area(json_extract(tags, '$.url_alias'));

UPDATE report SET area_id = (SELECT rowid FROM area WHERE json_extract(tags, '$.url_alias') = area_url_alias);

ALTER TABLE report DROP COLUMN area_url_alias;

CREATE TRIGGER report_updated_at UPDATE OF area_id, date, tags, created_at, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE rowid = old.rowid;
END;