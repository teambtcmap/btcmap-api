CREATE TABLE ban(
    id INTEGER PRIMARY KEY NOT NULL,
    ip TEXT NOT NULL,
    reason TEXT NOT NULL,    
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL
) STRICT;

CREATE INDEX ban_ip ON ban(ip);

CREATE INDEX ban_start_at_end_at on ban (start_at, end_at);