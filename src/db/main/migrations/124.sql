-- Notes gained an `icon` discriminator so clients can render different kinds of
-- pins. Existing notes keep the generic "notes" icon. The trigger is recreated
-- so that changing the icon also bumps `updated_at`.
ALTER TABLE note ADD COLUMN icon TEXT NOT NULL DEFAULT 'notes';

DROP TRIGGER note_updated_at;
CREATE TRIGGER note_updated_at
UPDATE OF user_id, lat, lon, text, public, icon, created_at, deleted_at ON note
BEGIN
    UPDATE note SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
