CREATE TABLE daily_report (
    date TEXT NOT NULL PRIMARY KEY,
    total_elements INTEGER NOT NULL DEFAULT 0,
    up_to_date_elements INTEGER NOT NULL DEFAULT 0,
    outdated_elements INTEGER NOT NULL DEFAULT 0,
    legacy_elements INTEGER NOT NULL DEFAULT 0,
    elements_created INTEGER NOT NULL DEFAULT 0,
    elements_updated INTEGER NOT NULL DEFAULT 0,
    elements_deleted INTEGER NOT NULL DEFAULT 0
);
