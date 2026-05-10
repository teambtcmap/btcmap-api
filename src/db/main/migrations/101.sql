CREATE UNIQUE INDEX user_npub_unique ON "user"(npub) WHERE npub IS NOT NULL;
