CREATE TABLE wallet(
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    xpub TEXT NOT NULL,
    cached_balance_sats INTEGER NOT NULL DEFAULT 0,
    cached_tx TEXT NOT NULL DEFAULT '[]',
    cached_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TRIGGER wallet_updated_at UPDATE OF name, xpub, cached_balance_sats, cached_tx, cached_at, created_at, deleted_at ON wallet
BEGIN
    UPDATE wallet SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
ALTER TABLE conf DROP COLUMN xpub_spending;
ALTER TABLE conf DROP COLUMN xpub_donations;
ALTER TABLE conf DROP COLUMN xpub_treasury;
DROP TABLE cache;
