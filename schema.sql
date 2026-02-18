PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE IF NOT EXISTS "osm_user"(
    id INTEGER PRIMARY KEY,
    osm_data TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE area(
    id INTEGER PRIMARY KEY NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
, alias TEXT, bbox_west REAL NOT NULL DEFAULT -180, bbox_south REAL NOT NULL DEFAULT -90, bbox_east REAL NOT NULL DEFAULT 180, bbox_north REAL NOT NULL DEFAULT 90) STRICT;
CREATE TABLE report(
    id INTEGER PRIMARY KEY NOT NULL,
    area_id INTEGER NOT NULL REFERENCES area(id),
    date TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE element(
    id INTEGER PRIMARY KEY NOT NULL,
    overpass_data TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
, lat REAL, lon REAL) STRICT;
CREATE TABLE IF NOT EXISTS "element_event"(
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES "osm_user"(id),
    element_id INTEGER NOT NULL REFERENCES element(id),
    type TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT (json_object()),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE IF NOT EXISTS "element_comment"(
    id INTEGER PRIMARY KEY NOT NULL,
    element_id INTEGER NOT NULL REFERENCES element(id),
    comment TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE area_element(
    id INTEGER PRIMARY KEY NOT NULL,
    area_id INTEGER NOT NULL REFERENCES area(id),
    element_id INTEGER NOT NULL REFERENCES element(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE ban(
    id INTEGER PRIMARY KEY NOT NULL,
    ip TEXT NOT NULL,
    reason TEXT NOT NULL,    
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL
) STRICT;
CREATE TABLE invoice(
    id INTEGER PRIMARY KEY NOT NULL,
    description TEXT NOT NULL, 
    amount_sats INTEGER NOT NULL,
    payment_hash TEXT NOT NULL,
    payment_request TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
, uuid TEXT, source TEXT NOT NULL DEFAULT 'lnbits') STRICT;
CREATE TABLE conf(
    id INTEGER PRIMARY KEY NOT NULL,
    paywall_add_element_comment_price_sat INTEGER NOT NULL,
    paywall_boost_element_30d_price_sat INTEGER NOT NULL,
    paywall_boost_element_90d_price_sat INTEGER NOT NULL,
    paywall_boost_element_365d_price_sat INTEGER NOT NULL
, lnbits_invoice_key TEXT NOT NULL DEFAULT '', gitea_api_key TEXT NOT NULL DEFAULT '', matrix_bot_password TEXT NOT NULL DEFAULT '', lnd_invoices_macaroon TEXT NOT NULL DEFAULT '') STRICT;
INSERT INTO conf VALUES(1,500,5000,10000,30000,'','','','');
CREATE TABLE element_issue(
    id INTEGER PRIMARY KEY NOT NULL,
    element_id INTEGER NOT NULL REFERENCES element(id),
    code TEXT NOT NULL,
    severity INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE IF NOT EXISTS "user"(
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    roles TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE access_token(
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL REFERENCES "user"(id),
    name TEXT,
    secret TEXT NOT NULL,
    roles TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
) STRICT;
CREATE TABLE event(
    id INTEGER PRIMARY KEY NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    name TEXT NOT NULL,
    website TEXT NOT NULL,
    ends_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    deleted_at TEXT
, starts_at TEXT) STRICT;
CREATE TABLE place_submission(
    id INTEGER PRIMARY KEY NOT NULL,
    origin TEXT NOT NULL,
    external_id TEXT NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    extra_fields TEXT NOT NULL DEFAULT (json_object()),
    ticket_url TEXT,
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ')),
    closed_at TEXT,
    deleted_at TEXT
) STRICT;
CREATE TABLE rpc_call (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER REFERENCES user(id),
    ip TEXT NOT NULL,
    method TEXT NOT NULL,
    params_json TEXT,
    created_at TEXT NOT NULL,
    processed_at TEXT NOT NULL,
    duration_ns INTEGER NOT NULL
) STRICT;
CREATE TRIGGER user_updated_at UPDATE OF osm_data, tags, created_at, deleted_at ON "osm_user"
BEGIN
    UPDATE "osm_user" SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER report_updated_at UPDATE OF area_id, date, tags, created_at, deleted_at ON report
BEGIN
    UPDATE report SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER element_comment_updated_at UPDATE OF element_id, comment, created_at, deleted_at ON element_comment
BEGIN
    UPDATE element_comment SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER area_element_updated_at UPDATE OF area_id, element_id, created_at, deleted_at ON area_element
BEGIN
    UPDATE area_element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER invoice_updated_at UPDATE OF description, amount_sats, payment_hash, payment_request, status, created_at, deleted_at ON invoice
BEGIN
    UPDATE invoice SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER element_issue_updated_at UPDATE OF element_id, code, severity, created_at, deleted_at ON element_issue
BEGIN
    UPDATE element_issue SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER admin_updated_at UPDATE OF name, password, roles, created_at, deleted_at ON "user"
BEGIN
    UPDATE "user" SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER acess_token_updated_at UPDATE OF user_id, name, secret, roles, created_at, deleted_at ON access_token
BEGIN
    UPDATE access_token SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER element_event_updated_at UPDATE OF user_id, element_id, type, tags, created_at, deleted_at ON element_event
BEGIN
    UPDATE element_event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER event_updated_at UPDATE OF lat, lon, name, website, starts_at, ends_at, created_at, deleted_at ON event
BEGIN
    UPDATE event SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER area_updated_at UPDATE OF alias, bbox_west, bbox_south, bbox_east, bbox_north, tags, created_at, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER place_submission_updated_at UPDATE OF origin, external_id, lat, lon, category, name, extra_fields, ticket_url, revoked, created_at, closed_at, deleted_at ON place_submission
BEGIN
    UPDATE place_submission SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE TRIGGER element_updated_at UPDATE OF overpass_data, tags, lat, lon, created_at, deleted_at ON element
BEGIN
    UPDATE element SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = old.id;
END;
CREATE INDEX idx_user_updated_at ON "osm_user"(updated_at);
CREATE INDEX area_updated_at ON area(updated_at);
CREATE INDEX report_updated_at ON report(updated_at);
CREATE INDEX element_updated_at ON element(updated_at);
CREATE UNIQUE INDEX element_overpass_data_type_and_id ON element(json_extract(overpass_data, '$.type'), json_extract(overpass_data, '$.id'));
CREATE INDEX element_comment_updated_at ON element_comment(updated_at);
CREATE INDEX area_element_area_id ON area_element(area_id);
CREATE INDEX area_element_element_id ON area_element(element_id);
CREATE INDEX ban_ip ON ban(ip);
CREATE INDEX ban_start_at_end_at on ban (start_at, end_at);
CREATE INDEX invoice_status ON invoice(status);
CREATE INDEX element_issue_element_id ON element_issue(element_id);
CREATE UNIQUE INDEX element_issue_element_id_code ON element_issue(element_id, code);
CREATE INDEX element_issue_updated_at ON element_issue(updated_at);
CREATE INDEX element_issue_deleted_at ON element_issue(deleted_at);
CREATE INDEX admin_name ON "user"(name);
CREATE INDEX admin_updated_at ON "user"(updated_at);
CREATE INDEX access_token_secret ON access_token(secret);
CREATE UNIQUE INDEX area_element_area_id_element_id ON area_element(area_id, element_id);
CREATE INDEX element_event_updated_at ON element_event(updated_at);
CREATE INDEX idx_area_bbox_west ON area(bbox_west);
CREATE INDEX idx_area_bbox_south ON area(bbox_south);
CREATE INDEX idx_area_bbox_east ON area(bbox_east);
CREATE INDEX idx_area_bbox_north ON area(bbox_north);
CREATE UNIQUE INDEX place_submission_origin_external_id ON place_submission(origin, external_id);
CREATE INDEX element_lat_lon ON element(lat, lon);
CREATE INDEX place_submission_lat_lon ON place_submission(lat, lon);
CREATE INDEX element_deleted_at ON element(deleted_at);
COMMIT;
