CREATE UNIQUE INDEX user_npub ON "user"(npub) WHERE npub IS NOT NULL;
