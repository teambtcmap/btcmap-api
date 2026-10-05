-- Track the review state of an event. Events default to `live` so every
-- pre-existing and RPC-created row is published; user submissions can opt into
-- `pending` explicitly. The trigger is rebuilt so a status transition bumps
-- updated_at and is observable through delta sync.
ALTER TABLE event ADD COLUMN status TEXT NOT NULL DEFAULT 'live';

DROP TRIGGER event_updated_at;
CREATE TRIGGER event_updated_at UPDATE OF lat, lon, name, website, starts_at, ends_at, area_id, created_at, deleted_at, status ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
