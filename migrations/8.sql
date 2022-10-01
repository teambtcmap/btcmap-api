ALTER TABLE daily_report ADD COLUMN total_elements_onchain INTEGER NOT NULL DEFAULT 0;
ALTER TABLE daily_report ADD COLUMN total_elements_lightning INTEGER NOT NULL DEFAULT 0;
ALTER TABLE daily_report ADD COLUMN total_elements_lightning_contactless INTEGER NOT NULL DEFAULT 0;