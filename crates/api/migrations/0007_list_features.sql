-- Feature slice system: separate table for list features with JSON config
CREATE TABLE list_features (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL CHECK(feature_name IN ('quantity', 'due_date')),
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);
CREATE INDEX idx_list_features_list ON list_features(list_id);

-- Migrate existing flags to list_features
INSERT INTO list_features (list_id, feature_name, config)
SELECT id, 'quantity', '{"unit_default": "szt"}' FROM lists WHERE has_quantity = 1;

INSERT INTO list_features (list_id, feature_name, config)
SELECT id, 'due_date', '{}' FROM lists WHERE has_due_date = 1;

-- Remove old columns
ALTER TABLE lists DROP COLUMN has_quantity;
ALTER TABLE lists DROP COLUMN has_due_date;
