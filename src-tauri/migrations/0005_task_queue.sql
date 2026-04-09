CREATE TABLE IF NOT EXISTS task_queue (
    id            TEXT PRIMARY KEY,
    file_path     TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_queue_status ON task_queue(status);
