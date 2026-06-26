CREATE TABLE place_import_origin(
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    gitea_sync_enabled INTEGER NOT NULL DEFAULT 0,
    gitea_label_id INTEGER
) STRICT;

INSERT INTO place_import_origin (name, gitea_sync_enabled, gitea_label_id) VALUES
    ('square', 1, 1307),
    ('coinos', 0, NULL),
    ('btcpayserver', 1, 1538),
    ('square-test', 1, 1551),
    ('bitcoin-jungle', 1, 1552);