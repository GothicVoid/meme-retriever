ALTER TABLE images ADD COLUMN file_hash          TEXT;
ALTER TABLE images ADD COLUMN file_size          INTEGER;
ALTER TABLE images ADD COLUMN file_modified_time INTEGER;

-- NULL 不参与唯一性判断，允许旧记录 hash 为 NULL
CREATE UNIQUE INDEX IF NOT EXISTS idx_images_file_hash
    ON images(file_hash) WHERE file_hash IS NOT NULL;
