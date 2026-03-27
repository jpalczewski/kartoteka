-- M4: Migrate feature slice due_date -> deadlines
-- Cannot UPDATE in-place because old CHECK constraint blocks 'deadlines' value.
-- Instead: create new table, INSERT with transformation, drop old, rename.

CREATE TABLE list_features_new (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL CHECK(feature_name IN ('quantity', 'deadlines')),
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);

-- Copy quantity features as-is
INSERT INTO list_features_new (list_id, feature_name, config)
SELECT list_id, feature_name, config FROM list_features WHERE feature_name = 'quantity';

-- Migrate due_date -> deadlines with new config
INSERT INTO list_features_new (list_id, feature_name, config)
SELECT list_id, 'deadlines', '{"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}'
FROM list_features WHERE feature_name = 'due_date';

DROP TABLE list_features;
ALTER TABLE list_features_new RENAME TO list_features;
CREATE INDEX idx_list_features_list ON list_features(list_id);
