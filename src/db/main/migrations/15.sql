CREATE TABLE event (
    date TEXT NOT NULL,
    element_id TEXT NOT NULL,
    type TEXT NOT NULL,
    user TEXT, 
    element_name TEXT NOT NULL DEFAULT '', 
    element_lat REAL NOT NULL DEFAULT -1000, 
    element_lon REAL NOT NULL DEFAULT -1000, 
    user_id INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
);

INSERT INTO event(
    date, 
    element_id, 
    type, 
    user, 
    element_name, 
    element_lat, 
    element_lon, 
    user_id, 
    created_at, 
    updated_at
)
select 
    date, 
    element_id, 
    type, 
    user, 
    element_name, 
    element_lat, 
    element_lon, 
    user_id, 
    date, 
    date 
from element_event;