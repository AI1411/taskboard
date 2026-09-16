CREATE TABLE IF NOT EXISTS comments (
    id BLOB NOT NULL PRIMARY KEY,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    actor_kind TEXT NOT NULL,
    actor_label TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_comments_task_created ON comments (task_id, created_at, id);
