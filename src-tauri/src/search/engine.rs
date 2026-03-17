use crate::db::DbPool;
use crate::commands::SearchResult;

pub struct SearchEngine {
    pub pool: DbPool,
}

impl SearchEngine {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        // TODO: KB expand → parallel CLIP + FTS → weighted merge
        let _ = (query, limit);
        Ok(vec![])
    }
}
