ALTER TABLE report ADD COLUMN tags TEXT NOT NULL DEFAULT '{}';

CREATE TRIGGER report_updated_at UPDATE OF tags, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE ROWID = old.ROWID;
END;

UPDATE report SET tags = json_set(tags, '$.total_elements', total_elements);
UPDATE report SET tags = json_set(tags, '$.total_elements_onchain', total_elements_onchain);
UPDATE report SET tags = json_set(tags, '$.total_elements_lightning', total_elements_lightning);
UPDATE report SET tags = json_set(tags, '$.total_elements_lightning_contactless', total_elements_lightning_contactless);
UPDATE report SET tags = json_set(tags, '$.up_to_date_elements', up_to_date_elements);
UPDATE report SET tags = json_set(tags, '$.outdated_elements', outdated_elements);
UPDATE report SET tags = json_set(tags, '$.legacy_elements', legacy_elements);

ALTER TABLE report DROP COLUMN total_elements;
ALTER TABLE report DROP COLUMN total_elements_onchain;
ALTER TABLE report DROP COLUMN total_elements_lightning;
ALTER TABLE report DROP COLUMN total_elements_lightning_contactless;
ALTER TABLE report DROP COLUMN up_to_date_elements;
ALTER TABLE report DROP COLUMN outdated_elements;
ALTER TABLE report DROP COLUMN legacy_elements;

ALTER TABLE report DROP COLUMN elements_created;
ALTER TABLE report DROP COLUMN elements_updated;
ALTER TABLE report DROP COLUMN elements_deleted;