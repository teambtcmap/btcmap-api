CREATE TABLE invoice(
    id INTEGER PRIMARY KEY NOT NULL,
    amount_sats: INTEGER NOT NULL,
    payment_hash TEXT NOT NULL,
    payment_request TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;

CREATE TRIGGER invoice_updated_at UPDATE OF amount_sats, payment_hash, payment_request, status, created_at, deleted_at ON invoice
BEGIN
    UPDATE invoice SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;

CREATE INDEX invoice_status ON invoice(status);
