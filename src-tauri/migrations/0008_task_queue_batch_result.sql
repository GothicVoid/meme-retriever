ALTER TABLE task_queue ADD COLUMN batch_id TEXT;

ALTER TABLE task_queue ADD COLUMN result_kind TEXT;

CREATE INDEX IF NOT EXISTS idx_task_queue_batch_id ON task_queue(batch_id);
