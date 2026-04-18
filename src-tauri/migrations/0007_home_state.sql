ALTER TABLE images ADD COLUMN last_used_at INTEGER;

CREATE TABLE IF NOT EXISTS search_history (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    query      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_search_history_query
    ON search_history(query);

CREATE INDEX IF NOT EXISTS idx_images_last_used
    ON images(last_used_at DESC);

CREATE INDEX IF NOT EXISTS idx_search_history_updated_at
    ON search_history(updated_at DESC);
