ALTER TABLE element ADD COLUMN lat REAL;
ALTER TABLE element ADD COLUMN lon REAL;

CREATE INDEX element_lat_lon ON element(lat, lon);

CREATE INDEX place_submission_lat_lon ON place_submission(lat, lon);

DROP TRIGGER element_updated_at;

CREATE TRIGGER element_updated_at UPDATE OF overpass_data, tags, lat, lon, created_at, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;