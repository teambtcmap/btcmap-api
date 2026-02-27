CREATE TABLE IF NOT EXISTS rpc_call (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER REFERENCES user(id),
    ip TEXT NOT NULL,
    method TEXT NOT NULL,
    params_json TEXT,
    created_at TEXT NOT NULL,
    processed_at TEXT NOT NULL,
    duration_ns INTEGER NOT NULL
) STRICT;