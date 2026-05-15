ALTER TABLE access_token ADD COLUMN import_origins TEXT NOT NULL DEFAULT '[]';
UPDATE access_token
SET import_origins = json_array('*')
WHERE roles LIKE '%places_source%'
   OR user_id IN (
       SELECT id
       FROM user
       WHERE roles LIKE '%places_source%'
   );
DROP TRIGGER acess_token_updated_at;
CREATE TRIGGER acess_token_updated_at UPDATE OF user_id, name, secret, roles, import_origins, created_at, deleted_at ON access_token
BEGIN
    UPDATE access_token SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
