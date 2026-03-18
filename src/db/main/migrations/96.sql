CREATE INDEX area_type ON area(json_extract(tags, '$.type'));
