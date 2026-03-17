use crate::db::DbPool;

pub async fn fts_search(pool: &DbPool, query: &str, limit: i64) -> anyhow::Result<Vec<(String, f32)>> {
    // TODO: SELECT image_id, rank FROM ocr_fts WHERE content MATCH ?
    let _ = (pool, query, limit);
    Ok(vec![])
}
