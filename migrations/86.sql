ALTER TABLE event ADD COLUMN starts_at_new TEXT;
UPDATE event SET starts_at_new = starts_at;
ALTER TABLE event DROP COLUMN starts_at;
ALTER TABLE event RENAME COLUMN starts_at_new to starts_at;