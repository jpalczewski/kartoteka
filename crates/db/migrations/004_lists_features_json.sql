ALTER TABLE lists ADD COLUMN features TEXT NOT NULL DEFAULT '{}';

UPDATE lists SET features = (
    SELECT COALESCE(
        json_group_object(lf.feature_name, json(lf.config)),
        '{}'
    )
    FROM list_features lf
    WHERE lf.list_id = lists.id
);

DROP TABLE list_features;
