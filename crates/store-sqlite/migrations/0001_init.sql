CREATE TABLE counters (
    name TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT INTO counters (name, value) VALUES ('task', 0), ('run', 0), ('activity', 0);

CREATE TABLE projects (
    id BLOB NOT NULL PRIMARY KEY,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    repo_path TEXT,
    archived INTEGER NOT NULL DEFAULT 0,
    note_markdown TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX idx_projects_live_slug
    ON projects (slug)
    WHERE deleted_at IS NULL;

CREATE TABLE tasks (
    id BLOB NOT NULL PRIMARY KEY,
    display_id TEXT NOT NULL UNIQUE,
    project_id BLOB NOT NULL REFERENCES projects (id),
    title TEXT NOT NULL,
    column TEXT NOT NULL,
    urgent INTEGER NOT NULL DEFAULT 0,
    note_markdown TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX idx_tasks_live_position
    ON tasks (project_id, column, position)
    WHERE deleted_at IS NULL;

CREATE TABLE links (
    id BLOB NOT NULL PRIMARY KEY,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    kind TEXT NOT NULL,
    value TEXT NOT NULL,
    sort_order INTEGER NOT NULL
);

CREATE TABLE runs (
    id BLOB NOT NULL PRIMARY KEY,
    display_id TEXT NOT NULL UNIQUE,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    agent TEXT NOT NULL,
    session_id TEXT,
    status TEXT NOT NULL,
    message TEXT,
    waiting_reason TEXT,
    summary TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE activities (
    id BLOB NOT NULL PRIMARY KEY,
    sequence INTEGER NOT NULL UNIQUE,
    actor_kind TEXT NOT NULL,
    actor_label TEXT NOT NULL,
    operation TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    previous_revision INTEGER,
    before_json TEXT,
    after_json TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_activities_entity ON activities (entity_id, sequence);
