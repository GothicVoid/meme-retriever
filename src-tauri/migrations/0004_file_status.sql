ALTER TABLE images ADD COLUMN file_status     TEXT NOT NULL DEFAULT 'normal';
ALTER TABLE images ADD COLUMN last_check_time INTEGER;

CREATE INDEX IF NOT EXISTS idx_images_file_status ON images(file_status);
