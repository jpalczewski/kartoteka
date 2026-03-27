-- M4: Richer time semantics - new date columns on items
ALTER TABLE items ADD COLUMN start_date TEXT;
ALTER TABLE items ADD COLUMN start_time TEXT;
ALTER TABLE items ADD COLUMN hard_deadline TEXT;

-- Rename existing due_date/due_time to deadline/deadline_time
ALTER TABLE items RENAME COLUMN due_date TO deadline;
ALTER TABLE items RENAME COLUMN due_time TO deadline_time;
