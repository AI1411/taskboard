CREATE INDEX IF NOT EXISTS idx_runs_task_id ON runs (task_id);
CREATE INDEX IF NOT EXISTS idx_links_task_id ON links (task_id);
CREATE INDEX IF NOT EXISTS idx_links_kind_value ON links (kind, value);
