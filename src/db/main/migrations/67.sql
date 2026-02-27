DROP TRIGGER admin_updated_at;

ALTER TABLE admin RENAME COLUMN allowed_methods TO allowed_actions;

CREATE TRIGGER admin_updated_at UPDATE OF name, password, allowed_actions, created_at, deleted_at ON admin
BEGIN
    UPDATE admin SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;