ALTER TABLE event ADD COLUMN cron_schedule TEXT;
DROP TRIGGER event_updated_at;
CREATE TRIGGER event_updated_at UPDATE OF lat, lon, name, website, starts_at, ends_at, cron_schedule, area_id, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;