CREATE TABLE IF NOT EXISTS images (
    id             TEXT PRIMARY KEY,
    file_path      TEXT NOT NULL,
    file_name      TEXT NOT NULL,
    format         TEXT NOT NULL DEFAULT '',
    width          INTEGER,
    height         INTEGER,
    added_at       INTEGER NOT NULL,
    use_count      INTEGER NOT NULL DEFAULT 0,
    thumbnail_path TEXT
);

CREATE TABLE IF NOT EXISTS tags (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    image_id   TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    tag_text   TEXT NOT NULL,
    is_auto    INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS embeddings (
    image_id TEXT PRIMARY KEY REFERENCES images(id) ON DELETE CASCADE,
    vector   BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS ocr_texts (
    image_id TEXT PRIMARY KEY REFERENCES images(id) ON DELETE CASCADE,
    content  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tags_image_id ON tags(image_id);
CREATE INDEX IF NOT EXISTS idx_tags_text     ON tags(tag_text);
CREATE INDEX IF NOT EXISTS idx_images_added  ON images(added_at DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS ocr_fts
    USING fts5(image_id UNINDEXED, content);
