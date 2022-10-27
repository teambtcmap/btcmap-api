DROP TABLE IF EXISTS daily_report;

DROP TABLE IF EXISTS element_event;

ALTER TABLE event DROP COLUMN element_lat;
ALTER TABLE event DROP COLUMN element_lon;
ALTER TABLE event DROP COLUMN element_name;
ALTER TABLE event DROP COLUMN user;