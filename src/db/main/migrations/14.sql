CREATE TABLE report( 
    area_id TEXT NOT NULL,
    date TEXT NOT NULL,
    total_elements INTEGER NOT NULL DEFAULT 0,
    total_elements_onchain INTEGER NOT NULL DEFAULT 0,
    total_elements_lightning INTEGER NOT NULL DEFAULT 0,
    total_elements_lightning_contactless INTEGER NOT NULL DEFAULT 0,
    up_to_date_elements INTEGER NOT NULL DEFAULT 0,
    outdated_elements INTEGER NOT NULL DEFAULT 0,
    legacy_elements INTEGER NOT NULL DEFAULT 0,
    elements_created INTEGER NOT NULL DEFAULT 0,
    elements_updated INTEGER NOT NULL DEFAULT 0,
    elements_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    updated_at TEXT NOT NULL DEFAULT ( strftime('%Y-%m-%dT%H:%M:%SZ') ),
    deleted_at TEXT NOT NULL DEFAULT ''
);

INSERT INTO report (area_id, date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted) SELECT '', date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted FROM daily_report;