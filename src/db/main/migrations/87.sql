DROP INDEX element_overpass_data_coinos;
DROP INDEX element_overpass_data_square;

CREATE INDEX element_deleted_at ON element(deleted_at);