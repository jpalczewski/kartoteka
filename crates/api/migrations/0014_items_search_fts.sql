CREATE VIRTUAL TABLE items_fts USING fts5(
    title,
    description,
    content='items'
);

INSERT INTO items_fts(rowid, title, description)
SELECT rowid, title, COALESCE(description, '')
FROM items;

CREATE TRIGGER items_fts_ai AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, title, description)
    VALUES (new.rowid, new.title, COALESCE(new.description, ''));
END;

CREATE TRIGGER items_fts_ad AFTER DELETE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, COALESCE(old.description, ''));
END;

CREATE TRIGGER items_fts_au AFTER UPDATE OF title, description ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, COALESCE(old.description, ''));
    INSERT INTO items_fts(rowid, title, description)
    VALUES (new.rowid, new.title, COALESCE(new.description, ''));
END;
