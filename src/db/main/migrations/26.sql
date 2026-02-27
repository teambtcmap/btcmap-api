DROP TRIGGER element_updated_at;

CREATE TRIGGER element_updated_at UPDATE OF osm_json, tags, created_at, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

DROP TRIGGER user_updated_at;

CREATE TRIGGER user_updated_at UPDATE OF osm_json, tags, created_at, deleted_at ON user
BEGIN
    UPDATE user SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

DROP TRIGGER event_updated_at;

CREATE TRIGGER event_updated_at UPDATE OF user_id, element_id, type, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

DROP TRIGGER area_updated_at;

CREATE TRIGGER area_updated_at UPDATE OF tags, created_at, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

DROP TRIGGER report_updated_at;

CREATE TRIGGER report_updated_at UPDATE OF area_id, date, tags, created_at, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

DROP TRIGGER token_updated_at;

CREATE TRIGGER token_updated_at UPDATE OF user_id, secret, created_at, deleted_at ON token
BEGIN
    UPDATE token SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;