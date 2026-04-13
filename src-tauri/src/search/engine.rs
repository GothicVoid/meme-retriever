use std::sync::{Arc, RwLock};

use crate::commands::{ScoreDebugInfo, SearchResult};
use crate::db::{repo, DbPool};
use crate::kb::provider::KnowledgeBaseProvider;
use crate::ml::clip::ClipEncoder;
use crate::search::{keyword, vector_store::VectorStore};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn char_coverage(query: &str, ocr_text: &str) -> f32 {
    if query.is_empty() {
        return 0.0;
    }
    let q_chars: std::collections::HashSet<char> = query.chars().collect();
    let o_chars: std::collections::HashSet<char> = ocr_text.chars().collect();
    q_chars.intersection(&o_chars).count() as f32 / q_chars.len() as f32
}

fn passes_result_filter(raw_cosine: f32, s_ocr: f32, s_kw: f32) -> bool {
    let has_text_signal = s_ocr > 0.0 || s_kw > 0.0;
    let semantic_pass = raw_cosine >= crate::search::vector_store::VectorStore::semantic_threshold();
    let text_pass = has_text_signal && (s_ocr >= 0.2 || s_kw >= 0.5);
    semantic_pass || text_pass
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub struct SearchEngine {
    pool: DbPool,
    vector_store: Arc<RwLock<VectorStore>>,
    kb: Box<dyn KnowledgeBaseProvider>,
}

impl SearchEngine {
    /// 创建并预加载向量索引。
    pub async fn new(pool: DbPool, kb: Box<dyn KnowledgeBaseProvider>) -> anyhow::Result<Self> {
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

    /// 清空内存中的全部向量索引。
    pub fn clear_all_vectors(&self) {
        self.vector_store.write().unwrap().clear();
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        w_kw: f32,
        w_ocr: f32,
        w_clip: f32,
    ) -> anyhow::Result<Vec<SearchResult>> {
        if query.is_empty() {
            // PRD §5.2.3: 展示使用频次最高的 N 张
            let images = repo::get_top_used_images(&self.pool, limit as i64).await?;
            let mut results = Vec::with_capacity(images.len());
            for img in images {
                let actual_status = if Path::new(&img.file_path).exists() {
                    "normal"
                } else {
                    "missing"
                };
                if actual_status != img.file_status {
                    repo::update_file_status(&self.pool, &img.id, actual_status, now_secs()).await?;
                }
                if actual_status == "missing" {
                    continue;
                }
                let tags = repo::get_tags_for_image(&self.pool, &img.id).await?;
                results.push(SearchResult {
                    id: img.id,
                    file_path: img.file_path,
                    thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                    file_format: img.format,
                    file_status: actual_status.to_string(),
                    score: 1.0,
                    tags,
                    debug_info: None,
                });
            }
            return Ok(results);
        }

        let start = std::time::Instant::now();

        // KB 查询扩展
        let expanded = self.kb.expand_query(query);
        if expanded != query {
            tracing::info!("[KB] Query expanded: {:?} → {:?}", query, expanded);
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
            tracing::info!("[CLIP] Text vector norm={:.4}", norm);
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
            if !m.is_empty() {
                tracing::info!("[TAG] Found {} tag matches", m.len());
            }
            m
        };

        // 4. 语义检索
        let semantic_hits = self
            .vector_store
            .read()
            .unwrap()
            .query(&text_vec, limit * 2);
        tracing::info!("[VEC] Found {} semantic matches", semantic_hits.len());

        // 5. 按 PRD §5.2.3 公式合并得分
        //    Final_Score = 0.75·Relevance + 0.25·Popularity
        //    Relevance   = w_kw·S_kw + w_ocr·S_ocr + w_clip·S_clip  （加权求和）
        //    Popularity  = log(1+use_count)/log(1+max_use_count)，冷启动 → 0.1
        //    低相关过滤：relevance < 0.2 → 不计入结果
        let fts_map: std::collections::HashMap<String, f32> =
            fts_result.unwrap_or_default().into_iter().collect();

        // 预查 max_use_count 及候选集的 use_count
        let max_use_count = repo::get_max_use_count(&self.pool).await?.max(1);
        let all_candidate_ids: Vec<&str> = {
            let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for (id, _) in &semantic_hits {
                ids.insert(id.as_str());
            }
            for id in fts_map.keys() {
                ids.insert(id.as_str());
            }
            ids.into_iter().collect()
        };
        let use_count_map = repo::get_use_counts(&self.pool, &all_candidate_ids).await?;
        let ocr_text_map = repo::get_ocr_texts(&self.pool, &all_candidate_ids).await?;

        let mut score_map: std::collections::HashMap<String, f32> =
            std::collections::HashMap::new();
        let mut debug_map: std::collections::HashMap<String, ScoreDebugInfo> =
            std::collections::HashMap::new();

        let merge_one = |id: &str,
                         raw_cosine: f32,
                         s_ocr: f32,
                         s_kw: f32,
                         use_count_map: &std::collections::HashMap<String, i64>,
                         max_uc: i64|
         -> (f32, ScoreDebugInfo) {
            let s_clip: f32 = (raw_cosine + 1.0) / 2.0; // cosine → [0,1]

            let relevance = (w_kw * s_kw + w_ocr * s_ocr + w_clip * s_clip).clamp(0.0, 1.0);

            let use_count = use_count_map.get(id).copied().unwrap_or(0);
            let popularity: f32 = if use_count == 0 {
                0.1 // PRD §4.2.3: 冷启动给予较低初始值
            } else {
                ((1.0 + use_count as f32).ln()) / ((1.0 + max_uc as f32).ln())
            };

            // 纯语义命中只要通过向量召回阈值就应保留；文本分支仍保留最低质量门槛。
            let final_score = if passes_result_filter(raw_cosine, s_ocr, s_kw) {
                (0.75 * relevance + 0.25 * popularity).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let dbg = ScoreDebugInfo {
                sem_score: s_clip,
                kw_score: s_ocr,
                tag_score: s_kw,
                sem_weight: w_clip,
                kw_weight: w_ocr,
                relevance,
                popularity,
            };
            (final_score, dbg)
        };

        for (id, raw_cosine) in &semantic_hits {
            let s_ocr = char_coverage(
                query,
                ocr_text_map
                    .get(id.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            );
            let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
            let (score, dbg) =
                merge_one(id, *raw_cosine, s_ocr, s_kw, &use_count_map, max_use_count);
            tracing::info!(
                "[MERGE] {} relevance={:.4} popularity={:.4} final={:.4}",
                id, dbg.relevance, dbg.popularity, score
            );
            if score > 0.0 {
                score_map.insert(id.clone(), score);
                debug_map.insert(id.clone(), dbg);
            }
        }
        // FTS 命中但语义未命中的也加入
        for id in fts_map.keys() {
            if !score_map.contains_key(id) {
                let s_ocr = char_coverage(
                    query,
                    ocr_text_map
                        .get(id.as_str())
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                );
                let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
                let (score, dbg) = merge_one(id, -1.0, s_ocr, s_kw, &use_count_map, max_use_count);
                tracing::info!(
                    "[MERGE] {} (no semantic) relevance={:.4} popularity={:.4} final={:.4}",
                    id, dbg.relevance, dbg.popularity, score
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
            semantic_hits.len(),
            fts_map.len(),
            start.elapsed().as_millis()
        );

        // 7. 从 DB 查询元数据组装结果
        let mut results = Vec::with_capacity(ranked.len());
        for (rank, (id, score)) in ranked.into_iter().enumerate() {
            if let Some(img) = repo::get_image(&self.pool, &id).await? {
                let actual_status = if Path::new(&img.file_path).exists() {
                    "normal"
                } else {
                    "missing"
                };
                if actual_status != img.file_status {
                    repo::update_file_status(&self.pool, &img.id, actual_status, now_secs()).await?;
                }
                if actual_status == "missing" {
                    continue;
                }
                let tags = repo::get_tags_for_image(&self.pool, &id).await?;
                tracing::info!(
                    "[RESULT] #{} {} score={:.4}",
                    rank + 1,
                    id,
                    score
                );
                results.push(SearchResult {
                    id: id.clone(),
                    file_path: img.file_path,
                    thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                    file_format: img.format,
                    file_status: actual_status.to_string(),
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
    use std::path::PathBuf;

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
            .unwrap_or_else(|_| {
                LocalKBProvider::load(std::path::Path::new("/nonexistent")).unwrap()
            });
        SearchEngine::new(pool, Box::new(kb)).await.unwrap()
    }

    fn fixture_path(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    async fn insert_test_image(pool: &SqlitePool, id: &str, ocr: &str, embedding: Vec<f32>) {
        repo::insert_image(
            pool,
            &repo::ImageRecord {
                id: id.to_string(),
                file_path: fixture_path("sample.jpg"),
                file_name: format!("{id}.jpg"),
                format: "jpg".to_string(),
                width: Some(100),
                height: Some(100),
                added_at: 1000,
                use_count: 0,
                thumbnail_path: Some(format!("/tmp/{id}_thumb.jpg")),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();
        repo::insert_embedding(pool, id, &embedding).await.unwrap();
        if !ocr.is_empty() {
            repo::insert_ocr(pool, id, ocr).await.unwrap();
        }
    }

    fn unit_vec(seed: usize) -> Vec<f32> {
        let mut v: Vec<f32> = (0..512)
            .map(|i| ((i + seed) as f32 * 0.017_453_3).sin())
            .collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
        v
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_db(pool: SqlitePool) {
        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query_returns_top_used(pool: SqlitePool) {
        // 插入 3 张图，use_count 不同，验证按 use_count 降序返回
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "low".into(),
                file_path: fixture_path("sample.jpg"),
                file_name: "low.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 1000,
                use_count: 1,
                thumbnail_path: Some("/tmp/low_t.jpg".into()),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "high".into(),
                file_path: fixture_path("sample.jpg"),
                file_name: "high.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 2000,
                use_count: 10,
                thumbnail_path: Some("/tmp/high_t.jpg".into()),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "mid".into(),
                file_path: fixture_path("sample.jpg"),
                file_name: "mid.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 3000,
                use_count: 5,
                thumbnail_path: Some("/tmp/mid_t.jpg".into()),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("", 10, 0.3, 0.4, 0.3).await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "high"); // use_count=10
        assert_eq!(results[1].id, "mid"); // use_count=5
        assert_eq!(results[2].id, "low"); // use_count=1
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query_no_usage_respects_limit(pool: SqlitePool) {
        for i in 0..5i64 {
            repo::insert_image(
                &pool,
                &repo::ImageRecord {
                    id: format!("img{i}"),
                    file_path: fixture_path("sample.jpg"),
                    file_name: format!("img{i}.jpg"),
                    format: "jpg".into(),
                    width: Some(100),
                    height: Some(100),
                    added_at: i * 1000,
                    use_count: 0,
                    thumbnail_path: Some(format!("/tmp/img{i}_t.jpg")),
                    file_hash: None,
                    file_size: None,
                    file_modified_time: None,
                    file_status: "normal".to_string(),
                    last_check_time: None,
                },
            )
            .await
            .unwrap();
        }
        let engine = make_engine(pool).await;
        let results = engine.search("", 3, 0.3, 0.4, 0.3).await.unwrap();
        assert_eq!(results.len(), 3, "should respect limit");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query_with_usage_returns_top_used(pool: SqlitePool) {
        // 有使用记录时，空查询应返回使用频次最高的图片（PRD §5.2.3）
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "used".into(),
                file_path: fixture_path("sample.jpg"),
                file_name: "used.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 1000,
                use_count: 3,
                thumbnail_path: None,
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();
        let engine = make_engine(pool).await;
        let results = engine.search("", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert_eq!(results.len(), 1, "should return top used image");
        assert_eq!(results[0].id, "used");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_empty_query(pool: SqlitePool) {
        let engine = make_engine(pool).await;
        let results = engine.search("", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_returns_results(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "hello world", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "goodbye", unit_vec(2)).await;
        insert_test_image(&pool, "img3", "test image", unit_vec(3)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("hello", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(!results.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_filters_missing_files(pool: SqlitePool) {
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img1".into(),
                file_path: "/definitely/not/found-search.jpg".into(),
                file_name: "img1.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 1000,
                use_count: 0,
                thumbnail_path: Some("/tmp/img1_t.jpg".into()),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();
        repo::insert_embedding(&pool, "img1", &unit_vec(1))
            .await
            .unwrap();
        repo::insert_ocr(&pool, "img1", "hello world").await.unwrap();

        let engine = make_engine(pool.clone()).await;
        let results = engine.search("hello", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(results.is_empty());

        let image = repo::get_image(&pool, "img1").await.unwrap().unwrap();
        assert_eq!(image.file_status, "missing");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_score_range(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "test", unit_vec(2)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
        for r in &results {
            assert!(
                r.score >= 0.0 && r.score <= 1.0,
                "score out of range: {}",
                r.score
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_score_order(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "test", unit_vec(2)).await;
        insert_test_image(&pool, "img3", "test", unit_vec(3)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
        for w in results.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "results not sorted: {} < {}",
                w[0].score,
                w[1].score
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_limit(pool: SqlitePool) {
        for i in 0..10 {
            insert_test_image(&pool, &format!("img{i}"), "test", unit_vec(i)).await;
        }
        let engine = make_engine(pool).await;
        let results = engine.search("test", 3, 0.3, 0.4, 0.3).await.unwrap();
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
    async fn test_clear_all_vectors(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "", unit_vec(1)).await;
        insert_test_image(&pool, "img2", "", unit_vec(2)).await;

        let engine = make_engine(pool).await;
        assert_eq!(engine.vector_store_len(), 2);

        engine.clear_all_vectors();
        assert_eq!(engine.vector_store_len(), 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_result_contains_debug_info(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "test content", unit_vec(1)).await;

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();

        assert!(!results.is_empty(), "should have results");
        for r in &results {
            let di = r
                .debug_info
                .as_ref()
                .expect("debug_info should be Some for search results");
            // sem_score = (cosine+1)/2 ∈ [0,1]
            assert!(
                di.sem_score >= 0.0 && di.sem_score <= 1.0,
                "sem_score out of range: {}",
                di.sem_score
            );
            // kw_score = FTS score ∈ [0,1]
            assert!(
                di.kw_score >= 0.0 && di.kw_score <= 1.0,
                "kw_score out of range: {}",
                di.kw_score
            );
            // 权重反映传入值
            assert!(
                (di.sem_weight - 0.3).abs() < 1e-5,
                "sem_weight should be 0.3 (w_clip)"
            );
            assert!(
                (di.kw_weight - 0.4).abs() < 1e-5,
                "kw_weight should be 0.4 (w_ocr)"
            );
            // relevance ∈ [0,1]
            assert!(
                di.relevance >= 0.0 && di.relevance <= 1.0,
                "relevance out of range: {}",
                di.relevance
            );
            // popularity ∈ [0,1]（冷启动为 0.1）
            assert!(
                di.popularity >= 0.0 && di.popularity <= 1.0,
                "popularity out of range: {}",
                di.popularity
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_cold_start_popularity_is_0_1(pool: SqlitePool) {
        insert_test_image(&pool, "cold", "test", unit_vec(1)).await;
        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
        let r = results.iter().find(|r| r.id == "cold").unwrap();
        let di = r.debug_info.as_ref().unwrap();
        assert!(
            (di.popularity - 0.1).abs() < 1e-6,
            "cold start popularity should be 0.1, got {}",
            di.popularity
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_empty_query_debug_info_is_none(pool: SqlitePool) {
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img1".into(),
                file_path: fixture_path("sample.jpg"),
                file_name: "img1.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 1000,
                use_count: 0,
                thumbnail_path: Some("/tmp/img1_t.jpg".into()),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("", 10, 0.3, 0.4, 0.3).await.unwrap();

        assert!(
            !results.is_empty(),
            "should return top used images for empty query"
        );
        for r in &results {
            assert!(
                r.debug_info.is_none(),
                "empty query results should have debug_info=None"
            );
        }
    }

    // ── 动态权重测试 ────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn test_dynamic_weights_reflected_in_debug_info(pool: SqlitePool) {
        // 验证传入的权重被正确记录到 debug_info
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        let engine = make_engine(pool).await;

        let results = engine.search("test", 10, 0.5, 0.3, 0.2).await.unwrap();
        assert!(!results.is_empty());
        for r in &results {
            if let Some(di) = &r.debug_info {
                assert!(
                    (di.sem_weight - 0.2).abs() < 1e-5,
                    "sem_weight (w_clip) should be 0.2, got {}",
                    di.sem_weight
                );
                assert!(
                    (di.kw_weight - 0.3).abs() < 1e-5,
                    "kw_weight (w_ocr) should be 0.3, got {}",
                    di.kw_weight
                );
            }
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_dynamic_weights_score_range(pool: SqlitePool) {
        // 不同权重下 score 仍在 [0,1]
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        let engine = make_engine(pool).await;

        for (w1, w2, w3) in [
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.5, 0.3, 0.2),
        ] {
            let results = engine.search("test", 10, w1, w2, w3).await.unwrap();
            for r in &results {
                assert!(
                    r.score >= 0.0 && r.score <= 1.0,
                    "score out of range with weights ({w1},{w2},{w3}): {}",
                    r.score
                );
            }
        }
    }

    #[test]
    fn test_pure_semantic_hit_passes_filter() {
        assert!(
            passes_result_filter(
                crate::search::vector_store::VectorStore::semantic_threshold(),
                0.0,
                0.0
            ),
            "pure semantic hit should pass even without OCR/tag signals"
        );
        assert!(
            !passes_result_filter(0.0, 0.1, 0.0),
            "weak OCR-only hit should still be filtered"
        );
        assert!(
            passes_result_filter(0.0, 0.0, 0.5),
            "strong tag hit should pass filter"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_input_truncation_200_chars(pool: SqlitePool) {
        // 超过 200 字符的查询在 engine 层不截断（截断在 command 层），
        // 但 engine 应能正常处理长查询
        insert_test_image(&pool, "img1", "test", unit_vec(1)).await;
        let engine = make_engine(pool).await;
        let long_query = "测".repeat(250);
        // 不应 panic，正常返回结果（可能为空）
        let result = engine.search(&long_query, 10, 0.3, 0.4, 0.3).await;
        assert!(
            result.is_ok(),
            "long query should not error: {:?}",
            result.err()
        );
    }
}
