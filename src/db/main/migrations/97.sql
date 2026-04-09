DROP TRIGGER admin_updated_at;
DROP TRIGGER user_updated_at;
ALTER TABLE user ADD COLUMN saved_places TEXT NOT NULL DEFAULT '';
ALTER TABLE user ADD COLUMN saved_areas TEXT NOT NULL DEFAULT '';
CREATE TRIGGER user_updated_at UPDATE OF name, password, roles, saved_places, saved_areas, created_at, deleted_at ON user
BEGIN
    UPDATE user SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER osm_user_updated_at UPDATE OF osm_data, tags, created_at, deleted_at ON osm_user
BEGIN
    UPDATE osm_user SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;