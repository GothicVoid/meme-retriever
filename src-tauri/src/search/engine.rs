use std::sync::{Arc, RwLock};

use crate::commands::SearchResult;
use crate::db::{DbPool, repo};
use crate::kb::provider::KnowledgeBaseProvider;
use crate::ml::clip::ClipEncoder;
use crate::search::{keyword, vector_store::VectorStore};

pub struct SearchEngine {
    pool: DbPool,
    vector_store: Arc<RwLock<VectorStore>>,
    kb: Box<dyn KnowledgeBaseProvider>,
}

impl SearchEngine {
    /// 创建并预加载向量索引。
    pub async fn new(
        pool: DbPool,
        kb: Box<dyn KnowledgeBaseProvider>,
    ) -> anyhow::Result<Self> {
        let mut store = VectorStore::new();
        let embeddings = repo::get_all_embeddings(&pool).await?;
        let count = embeddings.len();
        for (id, vec) in embeddings {
            store.insert(id, vec);
        }
        tracing::info!("search engine: loaded {count} vectors");
        Ok(Self {
            pool,
            vector_store: Arc::new(RwLock::new(store)),
            kb,
        })
    }

    /// 向量索引中插入新向量（入库后调用）。
    pub fn insert_vector(&self, id: String, vector: Vec<f32>) {
        self.vector_store.write().unwrap().insert(id, vector);
    }

    /// 从向量索引中移除（删除图片后调用）。
    pub fn remove_vector(&self, id: &str) {
        self.vector_store.write().unwrap().remove(id);
    }

    pub async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<SearchResult>> {
        if query.is_empty() {
            return Ok(vec![]);
        }

        let start = std::time::Instant::now();

        // 1. KB 查询扩展
        let expanded = self.kb.expand_query(query);
        tracing::debug!("search: query={query} expanded={expanded}");

        // 2. 并行：CLIP 文本编码 + FTS 搜索
        let pool = self.pool.clone();
        let expanded_clone = expanded.clone();
        let limit_i64 = (limit * 2) as i64;

        let (text_vec_result, fts_result) = tokio::join!(
            tokio::task::spawn_blocking(move || ClipEncoder::encode_text(&expanded_clone)),
            keyword::fts_search(&pool, query, limit_i64),
        );
        let text_vec = text_vec_result??;

        // 3. 标签搜索
        let tag_hits: std::collections::HashSet<String> = {
            let tag_results = keyword::tag_search(&self.pool, query, limit_i64).await?;
            tag_results.into_iter().map(|(id, _)| id).collect()
        };

        // 4. 语义检索
        let semantic_hits = self.vector_store.read().unwrap().query(&text_vec, limit * 2);

        // 5. 加权合并：score = 0.7×semantic + 0.3×keyword（标签命中时 keyword 权重升至 0.6）
        let fts_map: std::collections::HashMap<String, f32> = fts_result
            .unwrap_or_default()
            .into_iter()
            .collect();

        let mut score_map: std::collections::HashMap<String, f32> = std::collections::HashMap::new();

        for (id, sem_score) in &semantic_hits {
            let kw_score = fts_map.get(id).copied().unwrap_or(0.0);
            let kw_weight = if tag_hits.contains(id) { 0.6 } else { 0.3 };
            let sem_weight = 1.0 - kw_weight;
            let score = sem_weight * sem_score + kw_weight * kw_score;
            score_map.insert(id.clone(), score);
        }
        // FTS 命中但语义未命中的也加入
        for (id, kw_score) in &fts_map {
            if !score_map.contains_key(id) {
                let kw_weight = if tag_hits.contains(id) { 0.6 } else { 0.3 };
                score_map.insert(id.clone(), kw_weight * kw_score);
            }
        }

        // 6. 降序排列，取 limit 条
        let mut ranked: Vec<(String, f32)> = score_map.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);

        tracing::info!(
            "search: query={query} semantic_hits={} fts_hits={} total={}ms",
            semantic_hits.len(), fts_map.len(), start.elapsed().as_millis()
        );

        // 7. 从 DB 查询元数据组装结果
        let mut results = Vec::with_capacity(ranked.len());
        for (id, score) in ranked {
            if let Some(img) = repo::get_image(&self.pool, &id).await? {
                let tags = repo::get_tags_for_image(&self.pool, &id).await?;
                tracing::debug!("result: id={id} score={score:.3}");
                results.push(SearchResult {
                    id,
                    file_path: img.file_path,
                    thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                    score,
                    tags,
                });
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::local::LocalKBProvider;
    use sqlx::SqlitePool;

    async fn make_engine(pool: SqlitePool) -> SearchEngine {
        let kb = LocalKBProvider::load(std::path::Path::new("../app_data/knowledge_base.json"))
            .unwrap_or_else(|_| LocalKBProvider::load(std::path::Path::new("/nonexistent")).unwrap());
        SearchEngine::new(pool, Box::new(kb)).await.unwrap()
    }

    async fn insert_test_image(pool: &SqlitePool, id: &str, ocr: &str, embedding: Vec<f32>) {
        repo::insert_image(pool, &repo::ImageRecord {
            id: id.to_string(),
            file_path: format!("/tmp/{id}.jpg"),
            file_name: format!("{id}.jpg"),
            format: "jpg".to_string(),
            width: Some(100), height: Some(100),
            added_at: 1000, use_count: 0,
            thumbnail_path: Some(format!("/tmp/{id}_thumb.jpg")),
        }).await.unwrap();
        repo::insert_embedding(pool, id, &embedding).await.unwrap();
        if !ocr.is_empty() {
            repo::insert_ocr(pool, id, ocr).await.unwrap();
        }
    }

    fn unit_vec(seed: usize) -> Vec<f32> {
        let mut v: Vec<f32> = (0..512).map(|i| ((i + seed) as f32 * 0.017_453_3).sin()).collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
        v
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_db(pool: SqlitePool) {
        let engine = make_engine(pool).await;
        let results = engine.search("test", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query(pool: SqlitePool) {
        let engine = make_engine(pool).await;
        let results = engine.search("", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_returns_results(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "hello world", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "goodbye", unit_vec(2)).await;
        insert_test_image(&pool, "img3", "test image", unit_vec(3)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("hello", 10).await.unwrap();
        assert!(!results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_score_range(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "test", unit_vec(2)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10).await.unwrap();
        for r in &results {
            assert!(r.score >= 0.0 && r.score <= 1.0, "score out of range: {}", r.score);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_score_order(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "test", unit_vec(2)).await;
        insert_test_image(&pool, "img3", "test", unit_vec(3)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10).await.unwrap();
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score, "results not sorted: {} < {}", w[0].score, w[1].score);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_limit(pool: SqlitePool) {
        for i in 0..10 {
            insert_test_image(&pool, &format!("img{i}"), "test", unit_vec(i)).await;
        }
        let engine = make_engine(pool).await;
        let results = engine.search("test", 3).await.unwrap();
        assert!(results.len() <= 3);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_and_remove_vector(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "", unit_vec(1)).await;
        let engine = make_engine(pool).await;

        // 初始有 1 个向量
        assert_eq!(engine.vector_store.read().unwrap().len(), 1);

        // 插入新向量
        engine.insert_vector("img2".to_string(), unit_vec(2));
        assert_eq!(engine.vector_store.read().unwrap().len(), 2);

        // 移除向量
        engine.remove_vector("img1");
        assert_eq!(engine.vector_store.read().unwrap().len(), 1);
    }
}
