use crate::db::DbPool;
use sqlx::Row;

pub async fn fts_search(pool: &DbPool, query: &str, limit: i64) -> anyhow::Result<Vec<(String, f32)>> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    tracing::debug!("fts_search: query={query}");

    // FTS5 默认 tokenizer 将中文视为整体 token，加 * 支持前缀匹配
    let fts_query = format!("{query}*");
    let rows = sqlx::query(
        "SELECT image_id, rank FROM ocr_fts WHERE content MATCH ?1 ORDER BY rank LIMIT ?2"
    )
    .bind(&fts_query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        tracing::debug!("[FTS] 0 hits for {query:?}");
        return Ok(vec![]);
    }

    // FTS5 rank 为负数（越小越相关），取绝对值后归一化为 0~1
    let ranks: Vec<f64> = rows.iter().map(|r| {
        let rank: f64 = r.get("rank");
        rank.abs()
    }).collect();
    let max_abs = ranks.iter().cloned().fold(0f64, f64::max);

    let results: Vec<(String, f32)> = rows
        .iter()
        .zip(ranks.iter())
        .map(|(r, &abs_rank)| {
            let image_id: String = r.get("image_id");
            let score = if max_abs > 0.0 { (abs_rank / max_abs) as f32 } else { 0.0 };
            (image_id, score)
        })
        .collect();

    {
        let detail: Vec<String> = rows.iter().zip(ranks.iter()).zip(results.iter())
            .map(|((_r, &raw), (id, score))| {
                format!("  {}  raw={:.3}  score={:.3}", id, -raw, score)
            })
            .collect();
        tracing::debug!("[FTS] {} hits for {:?} (max_abs={:.3}):\n{}", results.len(), query, max_abs, detail.join("\n"));
    }
    Ok(results)
}

pub async fn tag_search(pool: &DbPool, query: &str, limit: i64) -> anyhow::Result<Vec<(String, f32)>> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    let pattern = format!("%{query}%");
    let rows = sqlx::query(
        "SELECT DISTINCT image_id FROM tags WHERE tag_text LIKE ?1 LIMIT ?2"
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.get("image_id"), 1.0f32)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn insert_image(pool: &SqlitePool, id: &str) {
        sqlx::query(
            "INSERT INTO images(id,file_path,file_name,format,added_at) VALUES(?1,?2,?3,'jpg',1)"
        )
        .bind(id).bind(format!("/p/{id}")).bind(format!("{id}.jpg"))
        .execute(pool).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_fts_basic_match(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        insert_image(&pool, "id2").await;
        sqlx::query("INSERT INTO ocr_fts(image_id,content) VALUES('id1','hello world')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO ocr_fts(image_id,content) VALUES('id2','goodbye world')")
            .execute(&pool).await.unwrap();

        let results = fts_search(&pool, "hello", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "id1");

        let results2 = fts_search(&pool, "goodbye", 10).await.unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].0, "id2");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_fts_chinese(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        sqlx::query("INSERT INTO ocr_fts(image_id,content) VALUES('id1','蚌埠住了哈哈哈')")
            .execute(&pool).await.unwrap();

        let results = fts_search(&pool, "蚌埠住了", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "id1");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_fts_empty_query(pool: SqlitePool) {
        let results = fts_search(&pool, "", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_tag_search_exact(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        sqlx::query("INSERT INTO tags(image_id,tag_text,is_auto,created_at) VALUES('id1','搞笑',0,1)")
            .execute(&pool).await.unwrap();

        let results = tag_search(&pool, "搞笑", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "id1");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_tag_search_empty_query(pool: SqlitePool) {
        let results = tag_search(&pool, "", 10).await.unwrap();
        assert!(results.is_empty());
    }
}
