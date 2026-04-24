use std::sync::{Arc, RwLock};

use crate::commands::{ScoreDebugInfo, SearchResult, TagDto};
use crate::db::{repo, DbPool};
use crate::kb::example_index::ExampleImageIndex;
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

fn passes_result_filter(raw_cosine: f32, s_ocr: f32, s_kw: f32, s_role: f32) -> bool {
    if s_role >= 0.9 {
        return true;
    }
    let has_text_signal = s_ocr > 0.0 || s_kw > 0.0;
    let semantic_pass =
        raw_cosine >= crate::search::vector_store::VectorStore::semantic_threshold();
    let text_pass = has_text_signal && (s_ocr >= 0.2 || s_kw >= 0.5);
    semantic_pass || text_pass
}

fn popularity_gate(relevance_score: f32) -> f32 {
    if relevance_score >= 0.55 {
        1.0
    } else {
        0.0
    }
}

fn collect_match_candidates(
    raw_query: &str,
    normalized_query: &str,
    related_terms: &[String],
) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut push_unique = |term: &str| {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            return;
        }
        if candidates.iter().any(|existing| existing == trimmed) {
            return;
        }
        candidates.push(trimmed.to_string());
    };

    push_unique(raw_query);
    push_unique(normalized_query);
    for term in related_terms {
        push_unique(term);
    }
    candidates
}

fn extract_text_matches(content: &str, candidates: &[String], limit: usize) -> Vec<String> {
    if content.is_empty() {
        return vec![];
    }
    let content_lower = content.to_lowercase();
    let mut matches = Vec::new();
    for candidate in candidates {
        let candidate_lower = candidate.to_lowercase();
        if candidate_lower.is_empty() {
            continue;
        }
        if content_lower.contains(&candidate_lower)
            && !matches.iter().any(|existing| existing == candidate)
        {
            matches.push(candidate.clone());
        }
        if matches.len() >= limit {
            break;
        }
    }
    matches
}

fn extract_tag_matches(
    tags: &[repo::TagRecord],
    candidates: &[String],
    limit: usize,
) -> Vec<String> {
    let mut matches = Vec::new();
    for tag in tags {
        let tag_lower = tag.tag_text.to_lowercase();
        if candidates.iter().any(|candidate| {
            let candidate_lower = candidate.to_lowercase();
            !candidate_lower.is_empty() && tag_lower.contains(&candidate_lower)
        }) && !matches.iter().any(|existing| existing == &tag.tag_text)
        {
            matches.push(tag.tag_text.clone());
        }
        if matches.len() >= limit {
            break;
        }
    }
    matches
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Clone, Copy)]
enum MainRoute {
    Ocr,
    Semantic,
    PrivateRole,
    Tag,
}

impl MainRoute {
    fn as_str(self) -> &'static str {
        match self {
            MainRoute::Ocr => "ocr",
            MainRoute::Semantic => "semantic",
            MainRoute::PrivateRole => "privateRole",
            MainRoute::Tag => "tag",
        }
    }
}

pub struct SearchEngine {
    pool: DbPool,
    vector_store: Arc<RwLock<VectorStore>>,
    kb: Arc<RwLock<Box<dyn KnowledgeBaseProvider>>>,
    example_image_index: Arc<RwLock<ExampleImageIndex>>,
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
            kb: Arc::new(RwLock::new(kb)),
            example_image_index: Arc::new(RwLock::new(ExampleImageIndex::empty())),
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

    pub fn replace_knowledge_base(
        &self,
        kb: Box<dyn KnowledgeBaseProvider>,
        example_image_index: ExampleImageIndex,
    ) {
        *self.kb.write().unwrap() = kb;
        *self.example_image_index.write().unwrap() = example_image_index;
    }

    pub fn set_example_image_index(&self, example_image_index: ExampleImageIndex) {
        *self.example_image_index.write().unwrap() = example_image_index;
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        _w_kw: f32,
        _w_ocr: f32,
        _w_clip: f32,
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
                    repo::update_file_status(&self.pool, &img.id, actual_status, now_secs())
                        .await?;
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
                    tags: tags.into_iter().map(TagDto::from).collect(),
                    matched_ocr_terms: vec![],
                    matched_tags: vec![],
                    matched_role_name: None,
                    debug_info: None,
                });
            }
            return Ok(results);
        }

        let start = std::time::Instant::now();

        let (normalized_query, related_terms) = {
            let kb = self.kb.read().unwrap();
            let normalized_query = kb.normalize_query(query);
            let related_terms = kb.related_terms(&normalized_query.tag_query);
            (normalized_query, related_terms)
        };
        let private_role_match = {
            let kb = self.kb.read().unwrap();
            kb.detect_private_role(query)
        };
        let evidence_candidates =
            collect_match_candidates(query, &normalized_query.tag_query, &related_terms);
        if normalized_query.tag_query != query {
            tracing::info!(
                "[KB] Tag query normalized: {:?} → {:?}",
                query,
                normalized_query.tag_query
            );
        }
        if normalized_query.expanded_query != query {
            tracing::info!(
                "[KB] Query expanded: {:?} → {:?}",
                query,
                normalized_query.expanded_query
            );
        }

        // 2. 并行：CLIP 文本编码 + FTS 搜索
        let pool = self.pool.clone();
        let expanded_clone = normalized_query.expanded_query.clone();
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
            for (id, score) in keyword::tag_search(
                    &self.pool,
                    query,
                    &normalized_query.tag_query,
                    &related_terms,
                    limit_i64,
                )
                .await?
            {
                m.insert(id, score);
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

        let role_score_map: std::collections::HashMap<String, f32> = if let Some(role_match) =
            &private_role_match
        {
            let store = self.vector_store.read().unwrap();
            let role_hits = self
                .example_image_index
                .read()
                .unwrap()
                .query_role_candidates(&role_match.name, &store, limit * 2);
            if !role_hits.is_empty() {
                tracing::info!(
                    "[ROLE] {} matched private role {}",
                    role_hits.len(),
                    role_match.name
                );
            }
            role_hits.into_iter().collect()
        } else {
            std::collections::HashMap::new()
        };

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
            for id in tag_score_map.keys() {
                ids.insert(id.as_str());
            }
            for id in role_score_map.keys() {
                ids.insert(id.as_str());
            }
            ids.into_iter().collect()
        };
        let use_count_map = repo::get_use_counts(&self.pool, &all_candidate_ids).await?;
        let ocr_text_map = repo::get_ocr_texts(&self.pool, &all_candidate_ids).await?;

        let main_route = if !fts_map.is_empty() {
            MainRoute::Ocr
        } else if !role_score_map.is_empty() {
            MainRoute::PrivateRole
        } else {
            MainRoute::Semantic
        };

        let mut score_map: std::collections::HashMap<String, f32> =
            std::collections::HashMap::new();
        let mut debug_map: std::collections::HashMap<String, ScoreDebugInfo> =
            std::collections::HashMap::new();

        let merge_one = |id: &str,
                         raw_cosine: f32,
                         s_fts: f32,
                         s_ocr: f32,
                         s_kw: f32,
                         s_role: f32,
                         use_count_map: &std::collections::HashMap<String, i64>,
                         max_uc: i64|
         -> (f32, ScoreDebugInfo) {
            let s_clip: f32 = (raw_cosine + 1.0) / 2.0; // cosine → [0,1]
            let text_score = s_ocr.max(s_fts);

            let use_count = use_count_map.get(id).copied().unwrap_or(0);
            let popularity: f32 = if use_count == 0 {
                0.1 // PRD §4.2.3: 冷启动给予较低初始值
            } else {
                ((1.0 + use_count as f32).ln()) / ((1.0 + max_uc as f32).ln())
            };
            let effective_route = if s_kw >= 0.8 && s_kw >= text_score && s_kw >= s_role {
                MainRoute::Tag
            } else {
                main_route
            };

            let (main_score, aux_score) = match effective_route {
                MainRoute::Ocr => (0.7 * text_score, 0.15 * s_clip + 0.05 * s_kw),
                MainRoute::Semantic => (0.7 * s_clip, 0.15 * text_score + 0.05 * s_kw),
                MainRoute::PrivateRole => (0.7 * s_role, 0.15 * s_clip + 0.1 * text_score + 0.05 * s_kw),
                MainRoute::Tag => (0.75 * s_kw, 0.1 * s_clip + 0.1 * text_score),
            };
            let relevance_score = main_score + aux_score;
            let popularity_boost = 0.1 * popularity * popularity_gate(relevance_score);

            // 纯语义命中只要通过向量召回阈值就应保留；文本分支仍保留最低质量门槛。
            let final_score = if passes_result_filter(raw_cosine, s_ocr, s_kw, s_role) {
                (relevance_score + popularity_boost).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let dbg = ScoreDebugInfo {
                main_route: effective_route.as_str().to_string(),
                main_score,
                aux_score,
                sem_score: s_clip,
                kw_score: text_score,
                tag_score: s_kw,
                popularity_boost,
            };
            (final_score, dbg)
        };

        for (id, raw_cosine) in &semantic_hits {
            let s_fts = fts_map.get(id.as_str()).copied().unwrap_or(0.0);
            let s_ocr = char_coverage(
                query,
                ocr_text_map
                    .get(id.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            );
            let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
            let s_role = role_score_map.get(id).copied().unwrap_or(0.0);
            let (score, dbg) = merge_one(
                id,
                *raw_cosine,
                s_fts,
                s_ocr,
                s_kw,
                s_role,
                &use_count_map,
                max_use_count,
            );
            tracing::info!(
                "[MERGE] {} route={} main={:.4} aux={:.4} pop={:.4} final={:.4}",
                id,
                dbg.main_route,
                dbg.main_score,
                dbg.aux_score,
                dbg.popularity_boost,
                score
            );
            if score > 0.0 {
                score_map.insert(id.clone(), score);
                debug_map.insert(id.clone(), dbg);
            }
        }
        // FTS 命中但语义未命中的也加入
        for id in fts_map.keys() {
            if !score_map.contains_key(id) {
                let s_fts = fts_map.get(id.as_str()).copied().unwrap_or(0.0);
                let s_ocr = char_coverage(
                    query,
                    ocr_text_map
                        .get(id.as_str())
                        .map(|s| s.as_str())
                    .unwrap_or(""),
                );
                let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
                let s_role = role_score_map.get(id).copied().unwrap_or(0.0);
                let (score, dbg) = merge_one(
                    id,
                    -1.0,
                    s_fts,
                    s_ocr,
                    s_kw,
                    s_role,
                    &use_count_map,
                    max_use_count,
                );
                tracing::info!(
                    "[MERGE] {} (no semantic) route={} main={:.4} aux={:.4} pop={:.4} final={:.4}",
                    id,
                    dbg.main_route,
                    dbg.main_score,
                    dbg.aux_score,
                    dbg.popularity_boost,
                    score
                );
                if score > 0.0 {
                    score_map.insert(id.clone(), score);
                    debug_map.insert(id.clone(), dbg);
                }
            }
        }
        // 纯标签命中但既无语义命中、也无 OCR/FTS 命中的也应进入结果
        for id in tag_score_map.keys() {
            if !score_map.contains_key(id) {
                let s_fts = fts_map.get(id.as_str()).copied().unwrap_or(0.0);
                let s_ocr = char_coverage(
                    query,
                    ocr_text_map
                        .get(id.as_str())
                        .map(|s| s.as_str())
                    .unwrap_or(""),
                );
                let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
                let s_role = role_score_map.get(id).copied().unwrap_or(0.0);
                let (score, dbg) = merge_one(
                    id,
                    -1.0,
                    s_fts,
                    s_ocr,
                    s_kw,
                    s_role,
                    &use_count_map,
                    max_use_count,
                );
                tracing::info!(
                    "[MERGE] {} (tag only) route={} main={:.4} aux={:.4} pop={:.4} final={:.4}",
                    id,
                    dbg.main_route,
                    dbg.main_score,
                    dbg.aux_score,
                    dbg.popularity_boost,
                    score
                );
                if score > 0.0 {
                    score_map.insert(id.clone(), score);
                    debug_map.insert(id.clone(), dbg);
                }
            }
        }
        for id in role_score_map.keys() {
            if !score_map.contains_key(id) {
                let s_fts = fts_map.get(id.as_str()).copied().unwrap_or(0.0);
                let s_ocr = char_coverage(
                    query,
                    ocr_text_map
                        .get(id.as_str())
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                );
                let s_kw = tag_score_map.get(id).copied().unwrap_or(0.0);
                let s_role = role_score_map.get(id).copied().unwrap_or(0.0);
                let (score, dbg) = merge_one(
                    id,
                    -1.0,
                    s_fts,
                    s_ocr,
                    s_kw,
                    s_role,
                    &use_count_map,
                    max_use_count,
                );
                tracing::info!(
                    "[MERGE] {} (private role) route={} main={:.4} aux={:.4} pop={:.4} final={:.4}",
                    id,
                    dbg.main_route,
                    dbg.main_score,
                    dbg.aux_score,
                    dbg.popularity_boost,
                    score
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
                    repo::update_file_status(&self.pool, &img.id, actual_status, now_secs())
                        .await?;
                }
                if actual_status == "missing" {
                    continue;
                }
                let tags = repo::get_tags_for_image(&self.pool, &id).await?;
                tracing::info!("[RESULT] #{} {} score={:.4}", rank + 1, id, score);
                results.push(SearchResult {
                    id: id.clone(),
                    file_path: img.file_path,
                    thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                    file_format: img.format,
                    file_status: actual_status.to_string(),
                    score,
                    tags: tags.iter().cloned().map(TagDto::from).collect(),
                    matched_ocr_terms: extract_text_matches(
                        ocr_text_map.get(&id).map(|text| text.as_str()).unwrap_or(""),
                        &evidence_candidates,
                        2,
                    ),
                    matched_tags: extract_tag_matches(&tags, &evidence_candidates, 2),
                    matched_role_name: private_role_match.as_ref().and_then(|role_match| {
                        if role_score_map.contains_key(&id) {
                            Some(role_match.name.clone())
                        } else {
                            None
                        }
                    }),
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
    use crate::kb::maintenance::{KnowledgeBaseEntry, KnowledgeBaseFile};
    use crate::kb::example_index::ExampleImageIndex;
    use sqlx::SqlitePool;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

    async fn make_engine_with_kb(pool: SqlitePool, kb_file: KnowledgeBaseFile) -> SearchEngine {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knowledge_base.json");
        std::fs::write(&path, kb_file.to_pretty_json().unwrap()).unwrap();
        let provider = LocalKBProvider::load(&path).unwrap();
        let example_image_index = ExampleImageIndex::from_knowledge_base(&kb_file, &path);
        let engine = SearchEngine::new(pool, Box::new(provider)).await.unwrap();
        engine.set_example_image_index(example_image_index);
        engine
    }

    fn fixture_path(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    async fn insert_test_image(pool: &SqlitePool, id: &str, ocr: &str, embedding: Vec<f32>) {
        insert_test_image_with_use_count(pool, id, ocr, embedding, 0).await;
    }

    async fn insert_test_image_with_use_count(
        pool: &SqlitePool,
        id: &str,
        ocr: &str,
        embedding: Vec<f32>,
        use_count: i64,
    ) {
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
                use_count,
                thumbnail_path: Some(format!("/tmp/{id}_thumb.jpg")),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
                last_used_at: None,
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
                last_used_at: None,
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
                last_used_at: None,
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
                last_used_at: None,
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
                    last_used_at: None,
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
                last_used_at: None,
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
    async fn test_search_returns_tag_only_match(pool: SqlitePool) {
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "tag-only".into(),
                file_path: fixture_path("sample.jpg"),
                file_name: "tag-only.jpg".into(),
                format: "jpg".into(),
                width: Some(100),
                height: Some(100),
                added_at: 1000,
                use_count: 0,
                thumbnail_path: Some("/tmp/tag-only_t.jpg".into()),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "normal".to_string(),
                last_check_time: None,
                last_used_at: None,
            },
        )
        .await
        .unwrap();
        repo::insert_embedding(&pool, "tag-only", &vec![0.0; 512])
            .await
            .unwrap();
        repo::insert_tags(
            &pool,
            "tag-only",
            &[repo::TagRecord {
                tag_text: "测试人物".into(),
                category: repo::TagCategory::Person,
                is_auto: false,
                source_strategy: repo::TagSourceStrategy::Manual,
                confidence: 1.0,
            }],
        )
        .await
        .unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("测试人物", 10, 0.3, 0.4, 0.3).await.unwrap();
        let result = results
            .iter()
            .find(|result| result.id == "tag-only")
            .expect("manual tag-only match should be searchable");
        assert!(
            result.score >= 0.75,
            "exact manual tag match should be a visible high-relevance result, got {}",
            result.score
        );
        assert_eq!(
            result.debug_info.as_ref().map(|debug| debug.main_route.as_str()),
            Some("tag")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_manual_partial_tag_match_is_medium_relevance(pool: SqlitePool) {
        insert_test_image(&pool, "tag-partial", "", vec![0.0; 512]).await;
        repo::insert_tags(
            &pool,
            "tag-partial",
            &[repo::TagRecord {
                tag_text: "让我看看".into(),
                category: repo::TagCategory::Custom,
                is_auto: false,
                source_strategy: repo::TagSourceStrategy::Manual,
                confidence: 1.0,
            }],
        )
        .await
        .unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("让我看", 10, 0.3, 0.4, 0.3).await.unwrap();
        let result = results
            .iter()
            .find(|result| result.id == "tag-partial")
            .expect("manual partial tag match should be searchable");
        assert!(
            result.score >= 0.55,
            "partial manual tag match should be visible before expanding low results, got {}",
            result.score
        );
        assert_eq!(
            result.debug_info.as_ref().map(|debug| debug.main_route.as_str()),
            Some("tag")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_search_prefers_ocr_main_route_over_auto_hint(pool: SqlitePool) {
        insert_test_image(&pool, "ocr-hit", "老板来了", unit_vec(1)).await;
        insert_test_image(&pool, "auto-hint", "", unit_vec(2)).await;
        repo::insert_tags(
            &pool,
            "auto-hint",
            &[repo::TagRecord {
                tag_text: "老板来了".into(),
                category: repo::TagCategory::Custom,
                is_auto: true,
                source_strategy: repo::TagSourceStrategy::Ocr,
                confidence: 1.0,
            }],
        )
        .await
        .unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("老板来了", 10, 0.3, 0.4, 0.3).await.unwrap();

        assert!(results.len() >= 2);
        assert_eq!(results[0].id, "ocr-hit");
        assert_eq!(results[0].debug_info.as_ref().unwrap().main_route, "ocr");
        assert_eq!(results[1].id, "auto-hint");
        assert!(
            results[0].score > results[1].score,
            "ocr main route should outrank auto hint"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_private_role_main_route_prioritizes_role_candidates(pool: SqlitePool) {
        insert_test_image(&pool, "abu-role", "", unit_vec(1)).await;
        insert_test_image(&pool, "other", "", unit_vec(2)).await;

        let sample_path = fixture_path("sample.jpg");
        sqlx::query("UPDATE images SET file_path = ?1 WHERE id = 'abu-role'")
            .bind(&sample_path)
            .execute(&pool)
            .await
            .unwrap();

        let sample_embedding = crate::ml::clip::ClipEncoder::encode_image(&sample_path).unwrap();
        sqlx::query("DELETE FROM embeddings WHERE image_id = 'abu-role'")
            .execute(&pool)
            .await
            .unwrap();
        repo::insert_embedding(&pool, "abu-role", &sample_embedding)
            .await
            .unwrap();

        let kb_file = KnowledgeBaseFile {
            version: 1,
            entries: vec![KnowledgeBaseEntry {
                name: "阿布".into(),
                category: "person".into(),
                aliases: vec!["布布".into()],
                notes: "私有角色".into(),
                match_mode: "contains".into(),
                priority: 10,
                example_images: vec![sample_path.clone()],
            }],
        };

        let engine = make_engine_with_kb(pool, kb_file).await;
        let results = engine.search("阿布在这", 10, 0.3, 0.4, 0.3).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "abu-role");
        assert_eq!(results[0].debug_info.as_ref().unwrap().main_route, "privateRole");
        assert_eq!(results[0].matched_role_name.as_deref(), Some("阿布"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_private_role_without_examples_falls_back_to_semantic(pool: SqlitePool) {
        insert_test_image(&pool, "img1", "", unit_vec(1)).await;

        let kb_file = KnowledgeBaseFile {
            version: 1,
            entries: vec![KnowledgeBaseEntry {
                name: "老板".into(),
                category: "person".into(),
                aliases: vec!["王总".into()],
                notes: "私有角色".into(),
                match_mode: "contains".into(),
                priority: 10,
                example_images: vec![],
            }],
        };

        let engine = make_engine_with_kb(pool, kb_file).await;
        let results = engine.search("老板来了", 10, 0.3, 0.4, 0.3).await.unwrap();

        if let Some(first) = results.first() {
            assert_ne!(first.debug_info.as_ref().unwrap().main_route, "privateRole");
        }
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
                last_used_at: None,
            },
        )
        .await
        .unwrap();
        repo::insert_embedding(&pool, "img1", &unit_vec(1))
            .await
            .unwrap();
        repo::insert_ocr(&pool, "img1", "hello world")
            .await
            .unwrap();

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
        repo::insert_tags(
            &pool,
            "img1",
            &[repo::TagRecord {
                tag_text: "test-tag".into(),
                category: repo::TagCategory::Custom,
                is_auto: false,
                source_strategy: repo::TagSourceStrategy::Manual,
                confidence: 1.0,
            }],
        )
        .await
        .unwrap();

        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();

        assert!(!results.is_empty(), "should have results");
        for r in &results {
            let di = r
                .debug_info
                .as_ref()
                .expect("debug_info should be Some for search results");
            assert!(
                di.sem_score >= 0.0 && di.sem_score <= 1.0,
                "sem_score out of range: {}",
                di.sem_score
            );
            assert!(
                di.kw_score >= 0.0 && di.kw_score <= 1.0,
                "kw_score out of range: {}",
                di.kw_score
            );
            assert!(
                matches!(di.main_route.as_str(), "ocr" | "semantic" | "privateRole"),
                "unexpected main_route: {}",
                di.main_route
            );
            assert!(
                di.main_score >= 0.0 && di.main_score <= 1.0,
                "main_score out of range: {}",
                di.main_score
            );
            assert!(
                di.aux_score >= 0.0 && di.aux_score <= 1.0,
                "aux_score out of range: {}",
                di.aux_score
            );
            assert!(
                di.popularity_boost >= 0.0 && di.popularity_boost <= 0.1,
                "popularity_boost out of range: {}",
                di.popularity_boost
            );
        }
        assert!(
            results[0].matched_ocr_terms.iter().any(|term| term == "test"),
            "should include matched OCR term"
        );
        assert!(
            results[0].matched_tags.iter().any(|term| term == "test-tag"),
            "should include matched tag"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_cold_start_popularity_boost_is_small(pool: SqlitePool) {
        insert_test_image(&pool, "cold", "test", unit_vec(1)).await;
        let engine = make_engine(pool).await;
        let results = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
        let r = results.iter().find(|r| r.id == "cold").unwrap();
        let di = r.debug_info.as_ref().unwrap();
        assert!(
            (di.popularity_boost - 0.01).abs() < 1e-6,
            "cold start popularity_boost should be 0.01, got {}",
            di.popularity_boost
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
                last_used_at: None,
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

    #[sqlx::test(migrations = "./migrations")]
    async fn test_ocr_main_route_prioritizes_text_hit(pool: SqlitePool) {
        insert_test_image(&pool, "ocr-strong", "你礼貌吗", unit_vec(1)).await;
        insert_test_image(&pool, "semantic-only", "", unit_vec(2)).await;
        let engine = make_engine(pool).await;

        let results = engine.search("你礼貌吗", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "ocr-strong");
        assert_eq!(
            results[0].debug_info.as_ref().map(|di| di.main_route.as_str()),
            Some("ocr")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_non_empty_query_popularity_is_light_bonus(pool: SqlitePool) {
        insert_test_image_with_use_count(&pool, "exact-new", "你礼貌吗", unit_vec(1), 0).await;
        insert_test_image_with_use_count(&pool, "weak-hot", "你", unit_vec(2), 500).await;
        let engine = make_engine(pool).await;

        let results = engine.search("你礼貌吗", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "exact-new");
        let exact = results.iter().find(|result| result.id == "exact-new").unwrap();
        let weak = results.iter().find(|result| result.id == "weak-hot");
        if let Some(weak) = weak {
            assert!(
                exact.score > weak.score,
                "exact text hit should rank above weak but popular result"
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_semantic_main_route_remains_available_without_ocr(pool: SqlitePool) {
        let matching_embedding = ClipEncoder::encode_text("抓狂").unwrap();
        insert_test_image(&pool, "semantic-hit", "", matching_embedding).await;
        let engine = make_engine(pool).await;

        let results = engine.search("抓狂", 10, 0.3, 0.4, 0.3).await.unwrap();
        assert!(!results.is_empty());
        assert!(
            results.iter().any(|result| {
                result.id == "semantic-hit"
                    && result
                        .debug_info
                        .as_ref()
                        .is_some_and(|di| di.main_route == "semantic")
            }),
            "results should use semantic route when no OCR main signal exists"
        );
    }

    #[test]
    fn test_pure_semantic_hit_passes_filter() {
        assert!(
            passes_result_filter(
                crate::search::vector_store::VectorStore::semantic_threshold(),
                0.0,
                0.0,
                0.0
            ),
            "pure semantic hit should pass even without OCR/tag signals"
        );
        assert!(
            !passes_result_filter(0.0, 0.1, 0.0, 0.0),
            "weak OCR-only hit should still be filtered"
        );
        assert!(
            passes_result_filter(0.0, 0.0, 0.5, 0.0),
            "strong tag hit should pass filter"
        );
    }

    #[test]
    fn test_popularity_gate_blocks_weak_relevance() {
        assert_eq!(popularity_gate(0.54), 0.0);
    }

    #[test]
    fn test_popularity_gate_keeps_medium_relevance_and_above() {
        assert_eq!(popularity_gate(0.55), 1.0);
        assert_eq!(popularity_gate(0.8), 1.0);
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
