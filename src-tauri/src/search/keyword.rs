use crate::db::DbPool;
use sqlx::Row;

pub async fn fts_search(pool: &DbPool, query: &str, limit: i64) -> anyhow::Result<Vec<(String, f32)>> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    tracing::debug!("fts_search: query={query}");

    // 1. FTS5 trigram 子串匹配（查询 ≥3 字符时可用 trigram index，有 BM25 排名）
    let fts_rows = if query.chars().count() >= 3 {
        sqlx::query(
            "SELECT image_id, rank FROM ocr_fts WHERE content MATCH ?1 ORDER BY rank LIMIT ?2"
        )
        .bind(query)
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    // FTS 排名归一化
    let ranks: Vec<f64> = fts_rows.iter().map(|r| {
        let rank: f64 = r.get("rank");
        rank.abs()
    }).collect();
    let max_abs = ranks.iter().cloned().fold(0f64, f64::max);

    let mut id_score: std::collections::HashMap<String, f32> = fts_rows
        .iter()
        .zip(ranks.iter())
        .map(|(r, &abs_rank)| {
            let image_id: String = r.get("image_id");
            let score = if max_abs > 0.0 { (abs_rank / max_abs) as f32 } else { 1.0 };
            (image_id, score)
        })
        .collect();

    // 2. LIKE 兜底：在 ocr_texts 上做子串扫描，捕获 FTS 未命中的情况
    //    （查询 <3 字符时 trigram 无法建立索引，必须走此路；同时也处理边界情况）
    let like_pattern = format!("%{query}%");
    let like_rows = sqlx::query(
        "SELECT image_id FROM ocr_texts WHERE content LIKE ?1 LIMIT ?2"
    )
    .bind(&like_pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    for row in &like_rows {
        let image_id: String = row.get("image_id");
        // 已有 FTS 分数的不覆盖；LIKE 新增命中给固定分 1.0（表明确实匹配）
        id_score.entry(image_id).or_insert(1.0);
    }

    if id_score.is_empty() {
        tracing::debug!("[FTS] 0 hits for {query:?}");
        return Ok(vec![]);
    }

    // 排序（FTS5 命中按归一化分，LIKE 补充命中固定 1.0 排前面）
    let mut results: Vec<(String, f32)> = id_score.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit as usize);

    tracing::debug!(
        "[FTS] {} hits for {:?} (fts={} like={})",
        results.len(), query, fts_rows.len(), like_rows.len()
    );
    Ok(results)
}

pub async fn tag_search(pool: &DbPool, query: &str, limit: i64) -> anyhow::Result<Vec<(String, f32)>> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    let pattern = format!("%{query}%");
    let rows = sqlx::query(
        "SELECT DISTINCT image_id, tag_text FROM tags WHERE tag_text LIKE ?1 LIMIT ?2"
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| {
        let image_id: String = r.get("image_id");
        let tag_text: String = r.get("tag_text");
        // 完全匹配=1.0，部分匹配=0.8（PRD §4.2.3）
        let score = if tag_text.to_lowercase() == query.to_lowercase() { 1.0_f32 } else { 0.8_f32 };
        (image_id, score)
    }).collect())
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

    /// 同时插入 ocr_texts 和 ocr_fts，模拟真实入库流程
    async fn insert_ocr(pool: &SqlitePool, image_id: &str, content: &str) {
        sqlx::query("INSERT INTO ocr_texts(image_id,content) VALUES(?1,?2)")
            .bind(image_id).bind(content).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO ocr_fts(image_id,content) VALUES(?1,?2)")
            .bind(image_id).bind(content).execute(pool).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_fts_basic_match(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        insert_image(&pool, "id2").await;
        insert_ocr(&pool, "id1", "hello world").await;
        insert_ocr(&pool, "id2", "goodbye world").await;

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
        insert_ocr(&pool, "id1", "蚌埠住了哈哈哈").await;

        let results = fts_search(&pool, "蚌埠住了", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "id1");
    }

    /// 验证中文子串匹配：搜索"操作"能命中"还有这种操作"（原 bug 复现）
    #[sqlx::test(migrations = "./migrations")]
    async fn test_fts_chinese_mid_string(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        insert_ocr(&pool, "id1", "还有这种操作").await;

        // 搜索句尾 2 字符子串（< 3 字符，走 LIKE 兜底）
        let results = fts_search(&pool, "操作", 10).await.unwrap();
        assert!(!results.is_empty(), "2-char mid-string '操作' should match '还有这种操作'");
        assert_eq!(results[0].0, "id1");

        // 搜索中间 4 字符子串（≥ 3 字符，trigram 直接命中）
        let results3 = fts_search(&pool, "这种操作", 10).await.unwrap();
        assert!(!results3.is_empty(), "4-char mid-string '这种操作' should match");
        assert_eq!(results3[0].0, "id1");

        // 不相关的词不应匹配
        let none = fts_search(&pool, "搞笑", 10).await.unwrap();
        assert!(none.is_empty(), "unrelated query should not match");
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

    #[sqlx::test(migrations = "./migrations")]
    async fn test_tag_search_exact_returns_1_0(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        sqlx::query("INSERT INTO tags(image_id,tag_text,is_auto,created_at) VALUES('id1','搞笑',0,1)")
            .execute(&pool).await.unwrap();
        let results = tag_search(&pool, "搞笑", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 1.0).abs() < 1e-6,
            "exact match should be 1.0, got {}", results[0].1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_tag_search_partial_returns_0_8(pool: SqlitePool) {
        insert_image(&pool, "id1").await;
        sqlx::query("INSERT INTO tags(image_id,tag_text,is_auto,created_at) VALUES('id1','搞笑表情',0,1)")
            .execute(&pool).await.unwrap();
        let results = tag_search(&pool, "搞笑", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 0.8).abs() < 1e-6,
            "partial match should be 0.8, got {}", results[0].1);
    }
}
