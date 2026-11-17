-- Link an event to the OpenStreetMap place it takes place at. Stored in the
-- canonical `<type>/<id>` form, e.g. "node/14385962692", so it maps directly to
-- https://www.openstreetmap.org/node/14385962692. Events can also carry a free
-- form description. The trigger is rebuilt so linking or unlinking a place and
-- editing the description bump `updated_at` for delta sync clients.
ALTER TABLE event ADD COLUMN place_osm_id TEXT;
ALTER TABLE event ADD COLUMN description TEXT;

DROP TRIGGER event_updated_at;
CREATE TRIGGER event_updated_at UPDATE OF lat, lon, name, website, starts_at, ends_at, area_id, created_at, deleted_at, status, place_osm_id, description ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
