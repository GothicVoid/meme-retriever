use std::sync::{Arc, RwLock};

use crate::commands::{SearchResult, ScoreDebugInfo};
use crate::db::{DbPool, repo};
use crate::kb::provider::KnowledgeBaseProvider;
use crate::ml::clip::ClipEncoder;
use crate::search::{keyword, vector_store::VectorStore};

fn char_coverage(query: &str, ocr_text: &str) -> f32 {
    if query.is_empty() { return 0.0; }
    let q_chars: std::collections::HashSet<char> = query.chars().collect();
    let o_chars: std::collections::HashSet<char> = ocr_text.chars().collect();
    q_chars.intersection(&o_chars).count() as f32 / q_chars.len() as f32
}

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

    /// 当前向量索引中的向量数量（用于测试和监控）
    pub fn vector_store_len(&self) -> usize {
        self.vector_store.read().unwrap().len()
    }

    /// 暴露 pool 供外部使用（如 commands 层入库后更新向量索引）
    pub fn pool(&self) -> &DbPool {
        &self.pool
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
            // PRD §5.2.3: 展示使用频次最高的 N 张
            let images = repo::get_top_used_images(&self.pool, limit as i64).await?;
            let mut results = Vec::with_capacity(images.len());
            for img in images {
                let tags = repo::get_tags_for_image(&self.pool, &img.id).await?;
                results.push(SearchResult {
                    id: img.id,
                    file_path: img.file_path,
                    thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                    score: 1.0,
                    tags,
                    debug_info: None,
                });
            }
            return Ok(results);
        }

        let start = std::time::Instant::now();

        // 1. KB 查询扩展
        let expanded = self.kb.expand_query(query);
        if expanded != query {
            tracing::debug!("[KB] {:?} → {:?} (expanded)", query, expanded);
        } else {
            tracing::debug!("[KB] {:?} → no match, using as-is", query);
        }

        // 2. 并行：CLIP 文本编码 + FTS 搜索
        let pool = self.pool.clone();
        let expanded_clone = expanded.clone();
        let limit_i64 = (limit * 2) as i64;

        let (text_vec_result, fts_result) = tokio::join!(
            tokio::task::spawn_blocking(move || ClipEncoder::encode_text(&expanded_clone)),
            keyword::fts_search(&pool, query, limit_i64),
        );
        let text_vec = text_vec_result??;

        // CLIP 向量摘要
        {
            let norm: f32 = text_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            let preview: Vec<String> = text_vec.iter().take(4).map(|x| format!("{x:.3}")).collect();
            tracing::debug!("[CLIP] vec[:4]=[{}] norm={:.4}", preview.join(", "), norm);
        }

        // 3. 标签搜索（三级评分：精确=1.0 / 部分=0.8 / 关联=0.5）
        let tag_score_map: std::collections::HashMap<String, f32> = {
            let mut m = std::collections::HashMap::new();
            for (id, score) in keyword::tag_search(&self.pool, query, limit_i64).await? {
                m.insert(id, score);
            }
            if expanded != query {
                for (id, _) in keyword::tag_search(&self.pool, &expanded, limit_i64).await? {
                    m.entry(id).or_insert(0.5);
                }
            }
            if m.is_empty() {
                tracing::debug!("[TAG] 0 hits");
            } else {
                let list: Vec<&str> = m.keys().map(|s| s.as_str()).collect();
                tracing::debug!("[TAG] {} hits: [{}]", m.len(), list.join(", "));
            }
            m
        };

        // 4. 语义检索
        let semantic_hits = self.vector_store.read().unwrap().query(&text_vec, limit * 2);
        {
            let detail: Vec<String> = semantic_hits.iter()
                .map(|(id, cos)| format!("  {}  cos={:.4}", id, cos))
                .collect();
            tracing::debug!("[VEC] {} semantic hits:\n{}", semantic_hits.len(), detail.join("\n"));
        }

        // 5. 按 docs/scoring.md 新公式合并得分
        //    Final_Score = 0.75·Relevance + 0.25·Popularity
        //    Relevance   = max(0.3·S_kw, 0.4·S_ocr, 0.3·S_clip)
        //    Popularity  = log(1+use_count)/log(1+max_use_count)，冷启动 → 0.1
        //    低相关过滤：relevance < 0.2 → 不计入结果
        let fts_map: std::collections::HashMap<String, f32> = fts_result
            .unwrap_or_default()
            .into_iter()
            .collect();

        // 预查 max_use_count 及候选集的 use_count
        let max_use_count = repo::get_max_use_count(&self.pool).await?.max(1);
        let all_candidate_ids: Vec<&str> = {
            let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for (id, _) in &semantic_hits { ids.insert(id.as_str()); }
            for id in fts_map.keys() { ids.insert(id.as_str()); }
            ids.into_iter().collect()
        };
        let use_count_map = repo::get_use_counts(&self.pool, &all_candidate_ids).await?;
        let ocr_text_map = repo::get_ocr_texts(&self.pool, &all_candidate_ids).await?;

        let mut score_map: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut debug_map: std::collections::HashMap<String, ScoreDebugInfo> = std::collections::HashMap::new();

        let merge_one = |id: &str, raw_cosine: f32, s_ocr: f32, s_kw: f32,
                         use_count_map: &std::collections::HashMap<String, i64>, max_uc: i64|
                         -> (f32, ScoreDebugInfo) {
            let s_clip: f32 = (raw_cosine + 1.0) / 2.0;   // cosine → [0,1]

            let relevance = (0.3_f32 * s_kw)
                .max(0.4_f32 * s_ocr)
                .max(0.3_f32 * s_clip);

            let use_count = use_count_map.get(id).copied().unwrap_or(0);
            let popularity: f32 = if use_count == 0 {
                0.1  // PRD §4.2.3: 冷启动给予较低初始值
            } else {
                ((1.0 + use_count as f32).ln()) / ((1.0 + max_uc as f32).ln())
            };

            let final_score = if relevance < 0.2 {
                0.0
            } else {
                (0.75 * relevance + 0.25 * popularity).clamp(0.0, 1.0)
            };

            let dbg = ScoreDebugInfo {
                sem_score: s_clip,
                kw_score: s_ocr,
                tag_score: s_kw,
                sem_weight: 0.3,
                kw_weight: 0.4,
                relevance,
                popularity,
            };
            (final_score, dbg)
        };

        for (id, raw_cosine) in &semantic_hits {
            let s_ocr = char_coverage(query, ocr_text_map.get(id.as_str()).map(|s| s.as_str()).unwrap_or(""));
            let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
            let (score, dbg) = merge_one(id, *raw_cosine, s_ocr, s_kw,
                                         &use_count_map, max_use_count);
            tracing::debug!(
                "[MERGE] {}  clip={:.4}(w=0.3)  ocr={:.4}(w=0.4)  kw={:.4}(w=0.3)  rel={:.4}  pop={:.4}  final={:.4}",
                id, dbg.sem_score, dbg.kw_score, dbg.tag_score,
                dbg.relevance, dbg.popularity, score
            );
            if score > 0.0 {
                score_map.insert(id.clone(), score);
                debug_map.insert(id.clone(), dbg);
            }
        }
        // FTS 命中但语义未命中的也加入
        for id in fts_map.keys() {
            if !score_map.contains_key(id) {
                let s_ocr = char_coverage(query, ocr_text_map.get(id.as_str()).map(|s| s.as_str()).unwrap_or(""));
                let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
                let (score, dbg) = merge_one(id, -1.0, s_ocr, s_kw,
                                             &use_count_map, max_use_count);
                tracing::debug!(
                    "[MERGE] {}  clip=none(w=0.3)  ocr={:.4}(w=0.4)  kw={:.4}(w=0.3)  rel={:.4}  pop={:.4}  final={:.4}",
                    id, dbg.kw_score, dbg.tag_score,
                    dbg.relevance, dbg.popularity, score
                );
                if score > 0.0 {
                    score_map.insert(id.clone(), score);
                    debug_map.insert(id.clone(), dbg);
                }
            }
        }

        // 6. 降序排列，取 limit 条，score clamp 到 [0, 1]
        let mut ranked: Vec<(String, f32)> = score_map
            .into_iter()
            .map(|(id, s)| (id, s.clamp(0.0, 1.0)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);

        tracing::info!(
            "search: query={query} semantic_hits={} fts_hits={} total={}ms",
            semantic_hits.len(), fts_map.len(), start.elapsed().as_millis()
        );

        // 7. 从 DB 查询元数据组装结果
        let mut results = Vec::with_capacity(ranked.len());
        for (rank, (id, score)) in ranked.into_iter().enumerate() {
            if let Some(img) = repo::get_image(&self.pool, &id).await? {
                let tags = repo::get_tags_for_image(&self.pool, &id).await?;
                tracing::debug!("[RESULT] #{} {}  score={:.4}  {}", rank + 1, id, score, img.file_path);
                results.push(SearchResult {
                    id: id.clone(),
                    file_path: img.file_path,
                    thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                    score,
                    tags,
                    debug_info: debug_map.remove(&id),
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

    #[test]
    fn test_char_coverage_full() {
        assert!((char_coverage("你好", "你好世界") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_char_coverage_partial() {
        assert!((char_coverage("你好", "你坏") - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_char_coverage_none() {
        assert_eq!(char_coverage("你好", "世界"), 0.0);
    }

    #[test]
    fn test_char_coverage_empty_query() {
        assert_eq!(char_coverage("", "你好"), 0.0);
    }

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
    async fn test_search_empty_query_returns_top_used(pool: SqlitePool) {
        // 插入 3 张图，use_count 不同，验证按 use_count 降序返回
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "low".into(), file_path: "/tmp/low.jpg".into(), file_name: "low.jpg".into(),
            format: "jpg".into(), width: Some(100), height: Some(100),
            added_at: 1000, use_count: 1, thumbnail_path: Some("/tmp/low_t.jpg".into()),
        }).await.unwrap();
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "high".into(), file_path: "/tmp/high.jpg".into(), file_name: "high.jpg".into(),
            format: "jpg".into(), width: Some(100), height: Some(100),
            added_at: 2000, use_count: 10, thumbnail_path: Some("/tmp/high_t.jpg".into()),
        }).await.unwrap();
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "mid".into(), file_path: "/tmp/mid.jpg".into(), file_name: "mid.jpg".into(),
            format: "jpg".into(), width: Some(100), height: Some(100),
            added_at: 3000, use_count: 5, thumbnail_path: Some("/tmp/mid_t.jpg".into()),
        }).await.unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("", 10).await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "high");  // use_count=10
        assert_eq!(results[1].id, "mid");   // use_count=5
        assert_eq!(results[2].id, "low");   // use_count=1
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query_no_usage_respects_limit(pool: SqlitePool) {
        for i in 0..5i64 {
            repo::insert_image(&pool, &repo::ImageRecord {
                id: format!("img{i}"), file_path: format!("/tmp/img{i}.jpg"),
                file_name: format!("img{i}.jpg"), format: "jpg".into(),
                width: Some(100), height: Some(100),
                added_at: i * 1000, use_count: 0,
                thumbnail_path: Some(format!("/tmp/img{i}_t.jpg")),
            }).await.unwrap();
        }
        let engine = make_engine(pool).await;
        let results = engine.search("", 3).await.unwrap();
        assert_eq!(results.len(), 3, "should respect limit");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query_with_usage_returns_top_used(pool: SqlitePool) {
        // 有使用记录时，空查询应返回使用频次最高的图片（PRD §5.2.3）
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "used".into(), file_path: "/tmp/used.jpg".into(), file_name: "used.jpg".into(),
            format: "jpg".into(), width: Some(100), height: Some(100),
            added_at: 1000, use_count: 3, thumbnail_path: None,
        }).await.unwrap();
        let engine = make_engine(pool).await;
        let results = engine.search("", 10).await.unwrap();
        assert_eq!(results.len(), 1, "should return top used image");
        assert_eq!(results[0].id, "used");
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

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_result_contains_debug_info(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "test content", unit_vec(1)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10).await.unwrap();

        assert!(!results.is_empty(), "should have results");
        for r in &results {
            let di = r.debug_info.as_ref().expect("debug_info should be Some for search results");
            // sem_score = (cosine+1)/2 ∈ [0,1]
            assert!(di.sem_score >= 0.0 && di.sem_score <= 1.0, "sem_score out of range: {}", di.sem_score);
            // kw_score = FTS score ∈ [0,1]
            assert!(di.kw_score >= 0.0 && di.kw_score <= 1.0, "kw_score out of range: {}", di.kw_score);
            // 新公式固定权重
            assert_eq!(di.sem_weight, 0.3, "sem_weight should be 0.3 (w3/clip)");
            assert_eq!(di.kw_weight, 0.4, "kw_weight should be 0.4 (w2/ocr)");
            // relevance ∈ [0,1]
            assert!(di.relevance >= 0.0 && di.relevance <= 1.0, "relevance out of range: {}", di.relevance);
            // popularity ∈ [0,1]（冷启动为 0.1）
            assert!(di.popularity >= 0.0 && di.popularity <= 1.0, "popularity out of range: {}", di.popularity);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_cold_start_popularity_is_0_1(pool: SqlitePool) {
        insert_test_image(&pool, "cold", "test", unit_vec(1)).await;
        let engine = make_engine(pool).await;
        let results = engine.search("test", 10).await.unwrap();
        let r = results.iter().find(|r| r.id == "cold").unwrap();
        let di = r.debug_info.as_ref().unwrap();
        assert!((di.popularity - 0.1).abs() < 1e-6,
            "cold start popularity should be 0.1, got {}", di.popularity);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_empty_query_debug_info_is_none(pool: SqlitePool) {
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "img1".into(), file_path: "/tmp/img1.jpg".into(), file_name: "img1.jpg".into(),
            format: "jpg".into(), width: Some(100), height: Some(100),
            added_at: 1000, use_count: 0, thumbnail_path: Some("/tmp/img1_t.jpg".into()),
        }).await.unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("", 10).await.unwrap();

        assert!(!results.is_empty(), "should return top used images for empty query");
        for r in &results {
            assert!(r.debug_info.is_none(), "empty query results should have debug_info=None");
        }
    }
}
