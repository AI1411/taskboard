CREATE TABLE IF NOT EXISTS checks (
    id BLOB NOT NULL PRIMARY KEY,
    display_id TEXT NOT NULL UNIQUE,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    text TEXT NOT NULL,
    done INTEGER NOT NULL,
    sort_order INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_checks_task_sort ON checks (task_id, sort_order, id);

INSERT OR IGNORE INTO counters (name, value) VALUES ('check', 0);
