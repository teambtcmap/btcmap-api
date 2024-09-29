ALTER TABLE token ADD COLUMN allowed_methods TEXT NOT NULL DEFAULT '[]';

DROP TRIGGER token_updated_at;

CREATE TRIGGER token_updated_at UPDATE OF user_id, secret, allowed_methods, created_at, deleted_at ON token
BEGIN
    UPDATE token SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;