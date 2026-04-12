DROP TRIGGER user_updated_at;
ALTER TABLE user ADD COLUMN npub TEXT;
CREATE TRIGGER user_updated_at UPDATE OF name, password, roles, saved_places, saved_areas, npub, created_at, deleted_at ON user
BEGIN
    UPDATE user SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;