ALTER TABLE tags ADD COLUMN category TEXT NOT NULL DEFAULT 'custom';
ALTER TABLE tags ADD COLUMN source_strategy TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE tags ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0;

CREATE INDEX IF NOT EXISTS idx_tags_category ON tags(category);
CREATE INDEX IF NOT EXISTS idx_tags_source_strategy ON tags(source_strategy);
