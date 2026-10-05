-- Link an event to the signed-in user who submitted it so they can track its
-- review status via GET /v4/users/me/events. NULL for events created before
-- this migration and for events created through RPC tooling.
ALTER TABLE event ADD COLUMN submitted_by INTEGER REFERENCES "user"(id);
