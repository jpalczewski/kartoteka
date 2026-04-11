CREATE VIRTUAL TABLE lists_fts USING fts5(
    name,
    description
);

INSERT INTO lists_fts(rowid, name, description)
SELECT rowid, name, COALESCE(description, '')
FROM lists;

CREATE TRIGGER lists_fts_ai AFTER INSERT ON lists BEGIN
    INSERT INTO lists_fts(rowid, name, description)
    VALUES (new.rowid, new.name, COALESCE(new.description, ''));
END;

CREATE TRIGGER lists_fts_ad AFTER DELETE ON lists BEGIN
    INSERT INTO lists_fts(lists_fts, rowid, name, description)
    VALUES ('delete', old.rowid, old.name, COALESCE(old.description, ''));
END;

CREATE TRIGGER lists_fts_au AFTER UPDATE OF name, description ON lists BEGIN
    INSERT INTO lists_fts(lists_fts, rowid, name, description)
    VALUES ('delete', old.rowid, old.name, COALESCE(old.description, ''));
    INSERT INTO lists_fts(rowid, name, description)
    VALUES (new.rowid, new.name, COALESCE(new.description, ''));
END;

CREATE VIRTUAL TABLE containers_fts USING fts5(
    name,
    description
);

INSERT INTO containers_fts(rowid, name, description)
SELECT rowid, name, COALESCE(description, '')
FROM containers;

CREATE TRIGGER containers_fts_ai AFTER INSERT ON containers BEGIN
    INSERT INTO containers_fts(rowid, name, description)
    VALUES (new.rowid, new.name, COALESCE(new.description, ''));
END;

CREATE TRIGGER containers_fts_ad AFTER DELETE ON containers BEGIN
    INSERT INTO containers_fts(containers_fts, rowid, name, description)
    VALUES ('delete', old.rowid, old.name, COALESCE(old.description, ''));
END;

CREATE TRIGGER containers_fts_au AFTER UPDATE OF name, description ON containers BEGIN
    INSERT INTO containers_fts(containers_fts, rowid, name, description)
    VALUES ('delete', old.rowid, old.name, COALESCE(old.description, ''));
    INSERT INTO containers_fts(rowid, name, description)
    VALUES (new.rowid, new.name, COALESCE(new.description, ''));
END;
