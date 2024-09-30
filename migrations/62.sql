DROP TRIGGER token_updated_at;

ALTER TABLE token RENAME TO admin;

ALTER TABLE admin RENAME COLUMN owner TO name;

ALTER TABLE admin RENAME COLUMN secret TO password;

CREATE TRIGGER admin_updated_at UPDATE OF name, password, allowed_methods, created_at, deleted_at ON admin
BEGIN
    UPDATE admin SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;