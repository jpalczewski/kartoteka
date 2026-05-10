ALTER TABLE lists      ADD COLUMN location_id TEXT REFERENCES locations(id) ON DELETE SET NULL;
ALTER TABLE containers ADD COLUMN location_id TEXT REFERENCES locations(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_lists_location      ON lists(location_id);
CREATE INDEX IF NOT EXISTS idx_containers_location ON containers(location_id);
