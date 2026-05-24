-- Restore the `user.npub` UNIQUE constraint that was originally part of
-- the Nostr-auth work but was reverted from migration 101 to avoid a
-- merge conflict with the multi-vendor import PR.
--
-- The endpoint handler in `src/rest/v4/nostr.rs` is already written to
-- treat a `user.npub` UNIQUE violation as the lost-race recovery path
-- for the auto-create flow. Without this index that branch is dormant
-- and two concurrent first-time sign-ins for the same pubkey can produce
-- two rows.

-- Pre-flight: between the endpoint going live and this migration landing
-- a small number of duplicate-npub rows could have been created. For
-- each duplicate group keep the oldest row's npub link and NULL out the
-- npub on the rest. The losing rows keep all their other state (saved
-- items, existing tokens) — they just lose their Nostr identity link, so
-- the user's next Nostr sign-in lands deterministically on the winner.
UPDATE "user"
SET npub = NULL
WHERE npub IS NOT NULL
  AND id NOT IN (
      SELECT MIN(id)
      FROM "user"
      WHERE npub IS NOT NULL
      GROUP BY npub
  );

CREATE UNIQUE INDEX user_npub_unique ON "user"(npub) WHERE npub IS NOT NULL;
