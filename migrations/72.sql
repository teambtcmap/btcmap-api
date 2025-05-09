DROP TRIGGER admin_updated_at;

DROP INDEX admin_name;

ALTER TABLE admin RENAME TO admin_old;

CREATE TABLE admin(
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    roles TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

INSERT INTO admin(
    id,
    name,
    password,
    roles,
    created_at,
    updated_at,
    deleted_at
)
SELECT
    id,
    name,
    password,
    allowed_actions,
    created_at,
    updated_at,
    deleted_at
FROM admin_old;

ALTER TABLE boost RENAME COLUMN admin_id TO admin_id_old;
ALTER TABLE boost ADD COLUMN admin_id INTEGER REFERENCES admin(id);
UPDATE boost SET admin_id = admin_id_old;
PRAGMA foreign_keys = OFF;
ALTER TABLE boost DROP COLUMN admin_id_old;
PRAGMA foreign_keys = ON;

DROP TABLE admin_old;

CREATE TRIGGER admin_updated_at UPDATE OF name, password, roles, created_at, deleted_at ON admin
BEGIN
    UPDATE admin SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX admin_name ON admin(name);
CREATE INDEX admin_updated_at ON admin(updated_at);