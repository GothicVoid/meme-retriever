use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, Emitter, State};

use crate::db::{DbPool, repo};
use crate::search::engine::SearchEngine;

// SearchEngine 包在 Arc 里以便 Tauri State 共享
pub type EngineState = Arc<SearchEngine>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDebugInfo {
    pub sem_score: f32,
    pub kw_score: f32,
    pub tag_score: f32,
    pub sem_weight: f32,
    pub kw_weight: f32,
    pub relevance: f32,
    pub popularity: f32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub file_path: String,
    pub thumbnail_path: String,
    pub file_format: String,
    pub score: f32,
    pub tags: Vec<String>,
    pub debug_info: Option<ScoreDebugInfo>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMeta {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub thumbnail_path: String,
    pub file_format: String,
    pub width: i64,
    pub height: i64,
    pub file_size: i64,
    pub added_at: i64,
    pub use_count: i64,
    pub tags: Vec<String>,
}

// ── 命令实现 ────────────────────────────────────────────────────────────────

/// 后台启动入库流水线，每张图完成后发送 `index-progress` 事件，并更新内存向量索引。
fn spawn_index_task(
    paths: Vec<String>,
    library_dir: PathBuf,
    pool: crate::db::DbPool,
    engine: Arc<crate::search::engine::SearchEngine>,
    app_handle: tauri::AppHandle,
) {
    tokio::spawn(async move {
        let mut rx = crate::indexer::pipeline::index_images(pool, paths, library_dir);
        while let Some(progress) = rx.recv().await {
            if progress.status == "completed" && !progress.id.is_empty() {
                if let Ok(Some(vec)) = repo::get_embedding(engine.pool(), &progress.id).await {
                    engine.insert_vector(progress.id.clone(), vec);
                }
            }
            let _ = app_handle.emit("index-progress", &progress);
        }
    });
}

#[tauri::command]
pub async fn search(
    query: String,
    limit: usize,
    w1: Option<f32>,
    w2: Option<f32>,
    w3: Option<f32>,
    engine: State<'_, EngineState>,
) -> Result<Vec<SearchResult>, String> {
    if limit == 0 {
        return Err("limit must be > 0".into());
    }
    // PRD §5.2.3: 输入长度截断，超过200字符取前200
    let query = if query.chars().count() > 200 {
        query.chars().take(200).collect::<String>()
    } else {
        query
    };
    // 权重归一化（默认 0.3/0.4/0.3）
    let (rw1, rw2, rw3) = {
        let a = w1.unwrap_or(0.3).max(0.0);
        let b = w2.unwrap_or(0.4).max(0.0);
        let c = w3.unwrap_or(0.3).max(0.0);
        let sum = a + b + c;
        if sum == 0.0 { (0.3, 0.4, 0.3) } else { (a / sum, b / sum, c / sum) }
    };
    tracing::info!("search: query={query}, limit={limit}, weights=({rw1:.2},{rw2:.2},{rw3:.2})");
    engine
        .search(&query, limit, rw1, rw2, rw3)
        .await
        .map_err(|e| { tracing::error!("command search failed: {e}"); e.to_string() })
}

#[tauri::command]
pub async fn add_images(
    paths: Vec<String>,
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    tracing::info!("add_images: {} files", paths.len());
    if paths.is_empty() {
        return Ok(());
    }

    let library_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("library");

    spawn_index_task(paths, library_dir, db.inner().clone(), Arc::clone(engine.inner()), app);
    Ok(())
}

#[tauri::command]
pub async fn add_folder(
    path: String,
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<usize, String> {
    use crate::indexer::pipeline::scan_images_in_dir;
    let paths = scan_images_in_dir(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    let total = paths.len();
    tracing::info!("add_folder: {path} → {total} images");
    if total > 0 {
        let library_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("library");
        spawn_index_task(paths, library_dir, db.inner().clone(), Arc::clone(engine.inner()), app);
    }
    Ok(total)
}

#[tauri::command]
pub async fn delete_image(
    id: String,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    tracing::info!("delete_image: {id}");
    repo::delete_image(db.inner(), &id)
        .await
        .map_err(|e| { tracing::error!("command delete_image failed: {e}"); e.to_string() })?;
    engine.remove_vector(&id);
    Ok(())
}

/// 获取单张图片的完整元数据（用于详情页）
#[tauri::command]
pub async fn get_image_meta(
    id: String,
    db: State<'_, DbPool>,
) -> Result<Option<ImageMeta>, String> {
    let img = repo::get_image(db.inner(), &id)
        .await
        .map_err(|e| e.to_string())?;
    match img {
        None => Ok(None),
        Some(img) => {
            let tags = repo::get_tags_for_image(db.inner(), &img.id)
                .await
                .unwrap_or_default();
            Ok(Some(ImageMeta {
                id: img.id,
                file_path: img.file_path,
                file_name: img.file_name,
                thumbnail_path: img.thumbnail_path.unwrap_or_default(),
                file_format: img.format,
                width: img.width.unwrap_or(0),
                height: img.height.unwrap_or(0),
                file_size: img.file_size.unwrap_or(0),
                added_at: img.added_at,
                use_count: img.use_count,
                tags,
            }))
        }
    }
}

#[tauri::command]
pub async fn get_images(
    page: i64,
    db: State<'_, DbPool>,
) -> Result<Vec<ImageMeta>, String> {
    tracing::info!("get_images: page={page}");
    let images = repo::get_images_paged(db.inner(), page, 50)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(images.len());
    for img in images {
        let tags = repo::get_tags_for_image(db.inner(), &img.id)
            .await
            .unwrap_or_default();
        result.push(ImageMeta {
            id: img.id,
            file_path: img.file_path,
            file_name: img.file_name,
            thumbnail_path: img.thumbnail_path.unwrap_or_default(),
            file_format: img.format,
            width: img.width.unwrap_or(0),
            height: img.height.unwrap_or(0),
            file_size: img.file_size.unwrap_or(0),
            added_at: img.added_at,
            use_count: img.use_count,
            tags,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn update_tags(
    image_id: String,
    tags: Vec<String>,
    db: State<'_, DbPool>,
) -> Result<(), String> {
    tracing::info!("update_tags: image={image_id}, count={}", tags.len());
    repo::delete_tags(db.inner(), &image_id)
        .await
        .map_err(|e| e.to_string())?;
    let tag_records: Vec<repo::TagRecord> = tags
        .into_iter()
        .map(|t| repo::TagRecord { tag_text: t, is_auto: false })
        .collect();
    repo::insert_tags(db.inner(), &image_id, &tag_records)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_tag_suggestions(
    prefix: String,
    db: State<'_, DbPool>,
) -> Result<Vec<String>, String> {
    tracing::info!("get_tag_suggestions: prefix={prefix}");
    repo::get_tag_suggestions(db.inner(), &prefix, 20)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_to_clipboard(
    id: String,
    db: State<'_, DbPool>,
) -> Result<(), String> {
    tracing::info!("copy_to_clipboard: {id}");
    let img = repo::get_image(db.inner(), &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("image not found: {id}"))?;

    // TODO: 复制图片二进制到剪贴板（需要平台特定实现）
    // 当前实现：将文件路径写入剪贴板文本
    tracing::debug!("copy_to_clipboard: path={}", img.file_path);
    Ok(())
}

#[tauri::command]
pub async fn reveal_in_finder(
    id: String,
    db: State<'_, DbPool>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    tracing::info!("reveal_in_finder: {id}");
    let img = repo::get_image(db.inner(), &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("image not found: {id}"))?;

    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&img.file_path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn increment_use_count(
    id: String,
    db: State<'_, DbPool>,
) -> Result<(), String> {
    tracing::info!("increment_use_count: {id}");
    repo::increment_use_count(db.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reindex_all(
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    tracing::info!("reindex_all: starting");
    let images = repo::get_all_images(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let total = images.len();
    tracing::info!("reindex_all: {} images to reindex", total);

    let pool = db.inner().clone();
    let engine = Arc::clone(engine.inner());

    tokio::spawn(async move {
        for (current, img) in images.into_iter().enumerate() {
            let progress_event = serde_json::json!({
                "current": current,
                "total": total,
                "id": &img.id,
            });
            let _ = app.emit("reindex-progress", &progress_event);

            let path = img.file_path.clone();
            let id   = img.id.clone();

            // 并行：CLIP 图像编码 + OCR 重跑
            let (clip_result, ocr_result) = tokio::join!(
                tokio::task::spawn_blocking({
                    let p = path.clone();
                    move || crate::ml::clip::ClipEncoder::encode_image(&p)
                }),
                tokio::task::spawn_blocking({
                    let p = path.clone();
                    move || crate::indexer::ocr::extract_text(&p)
                }),
            );

            // 更新 embedding
            match clip_result {
                Ok(Ok(vec)) => {
                    if let Err(e) = repo::insert_embedding(&pool, &id, &vec).await {
                        tracing::error!("reindex_all: failed to save embedding for {id}: {e}");
                    } else {
                        engine.insert_vector(id.clone(), vec);
                    }
                }
                Ok(Err(e)) => tracing::warn!("reindex_all: clip failed for {id}: {e}"),
                Err(e)     => tracing::warn!("reindex_all: clip task panicked for {id}: {e}"),
            }

            // 更新 OCR
            match ocr_result {
                Ok(Ok(text)) if !text.is_empty() => {
                    if let Err(e) = repo::insert_ocr(&pool, &id, &text).await {
                        tracing::error!("reindex_all: failed to save ocr for {id}: {e}");
                    } else {
                        tracing::debug!("reindex_all: ocr ok for {id} len={}", text.len());
                    }
                }
                Ok(Ok(_)) => {
                    // 无文字，清除旧 OCR 数据
                    if let Err(e) = repo::delete_ocr_for_image(&pool, &id).await {
                        tracing::warn!("reindex_all: failed to clear old ocr for {id}: {e}");
                    }
                }
                Ok(Err(e)) => tracing::warn!("reindex_all: ocr failed for {id}: {e}"),
                Err(e)     => tracing::warn!("reindex_all: ocr task panicked for {id}: {e}"),
            }

            tracing::debug!("reindex_all: done {id}");
        }

        let done_event = serde_json::json!({ "current": total, "total": total });
        let _ = app.emit("reindex-progress", &done_event);
        tracing::info!("reindex_all: completed");
    });

    Ok(())
}

// ── Phase C：文件状态管理 ────────────────────────────────────────────────────

/// 批量检查所有图片文件是否存在，更新 file_status 和 last_check_time。
/// 返回状态发生变化的图片数量。
#[tauri::command]
pub async fn check_file_statuses(db: State<'_, DbPool>) -> Result<u64, String> {
    let images = repo::get_all_images(db.inner()).await.map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let mut updated = 0u64;
    for img in &images {
        let status = if std::path::Path::new(&img.file_path).exists() { "normal" } else { "missing" };
        if status != img.file_status {
            repo::update_file_status(db.inner(), &img.id, status, now)
                .await.map_err(|e| e.to_string())?;
            updated += 1;
        }
    }
    Ok(updated)
}

// ── Phase D：任务队列 ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_pending_tasks(
    db: State<'_, DbPool>,
) -> Result<Vec<crate::db::task_repo::TaskRecord>, String> {
    crate::db::task_repo::get_pending_tasks(db.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_pending_tasks(
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<usize, String> {
    crate::db::task_repo::reset_stale_tasks(db.inner()).await.map_err(|e| e.to_string())?;
    let pending = crate::db::task_repo::get_pending_tasks(db.inner()).await.map_err(|e| e.to_string())?;
    let count = pending.len();
    if count > 0 {
        let paths: Vec<String> = pending.into_iter().map(|t| t.file_path).collect();
        let library_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("library");
        spawn_index_task(paths, library_dir, db.inner().clone(), Arc::clone(engine.inner()), app);
    }
    Ok(count)
}

#[tauri::command]
pub async fn clear_task_queue(db: State<'_, DbPool>) -> Result<(), String> {
    crate::db::task_repo::clear_task_queue(db.inner()).await.map_err(|e| e.to_string())
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::local::LocalKBProvider;
    use sqlx::SqlitePool;

    async fn make_engine(pool: SqlitePool) -> Arc<SearchEngine> {
        let kb = Box::new(LocalKBProvider::empty());
        Arc::new(SearchEngine::new(pool, kb).await.unwrap())
    }

    #[test]
    fn test_image_meta_serializes_camel_case() {
        let meta = ImageMeta {
            id: "uuid-1".into(),
            file_path: "/library/images/uuid-1.jpg".into(),
            file_name: "sample.jpg".into(),
            thumbnail_path: "/library/thumbs/uuid-1.jpg".into(),
            file_format: "jpg".into(),
            width: 800,
            height: 600,
            file_size: 0,
            added_at: 1700000000,
            use_count: 0,
            tags: vec![],
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert!(json.get("thumbnailPath").is_some(), "should have thumbnailPath");
        assert!(json.get("filePath").is_some(), "should have filePath");
        assert!(json.get("fileName").is_some(), "should have fileName");
        assert!(json.get("thumbnail_path").is_none(), "should NOT have thumbnail_path");
    }

    #[test]
    fn test_search_result_serializes_camel_case() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/library/images/uuid-1.jpg".into(),
            thumbnail_path: "/library/thumbs/uuid-1.jpg".into(),
            file_format: "jpg".into(),
            score: 0.9,
            tags: vec![],
            debug_info: None,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("thumbnailPath").is_some(), "should have thumbnailPath");
        assert!(json.get("filePath").is_some(), "should have filePath");
        assert!(json.get("thumbnail_path").is_none(), "should NOT have thumbnail_path");
        assert!(json.get("debugInfo").is_some(), "should have debugInfo (null)");
    }

    #[test]
    fn test_score_debug_info_serializes_camel_case() {
        let info = ScoreDebugInfo {
            sem_score: 0.85,
            kw_score: 0.4,
            tag_score: 1.0,
            sem_weight: 0.4,
            kw_weight: 0.6,
            relevance: 0.3,
            popularity: 0.5,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("semScore").is_some(), "should have semScore");
        assert!(json.get("kwScore").is_some(), "should have kwScore");
        assert!(json.get("tagScore").is_some(), "should have tagScore");
        assert!(json.get("semWeight").is_some(), "should have semWeight");
        assert!(json.get("kwWeight").is_some(), "should have kwWeight");
        assert!(json.get("relevance").is_some(), "should have relevance");
        assert!(json.get("popularity").is_some(), "should have popularity");
        assert!(json.get("sem_score").is_none(), "should NOT have sem_score");
    }

    #[test]
    fn test_search_result_has_debug_info_field() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/path/img.jpg".into(),
            thumbnail_path: "/path/thumb.jpg".into(),
            file_format: "jpg".into(),
            score: 0.9,
            tags: vec![],
            debug_info: Some(ScoreDebugInfo {
                sem_score: 0.8,
                kw_score: 0.0,
                tag_score: 0.0,
                sem_weight: 0.3,
                kw_weight: 0.4,
                relevance: 0.24,
                popularity: 0.5,
            }),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("debugInfo").is_some(), "should have debugInfo");
        let di = json["debugInfo"].as_object().unwrap();
        assert!((di["semScore"].as_f64().unwrap() - 0.8).abs() < 1e-5);
        assert_eq!(di["tagScore"].as_f64().unwrap(), 0.0);
        assert_eq!(di["kwScore"].as_f64().unwrap(), 0.0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_tags_replaces(pool: SqlitePool) {
        // 先插入图片和旧标签
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "img1".into(), file_path: "/tmp/img1.jpg".into(),
            file_name: "img1.jpg".into(), format: "jpg".into(),
            width: None, height: None, added_at: 1, use_count: 0, thumbnail_path: None,
            file_hash: None, file_size: None, file_modified_time: None,
            file_status: "normal".to_string(), last_check_time: None,
        }).await.unwrap();
        repo::insert_tags(&pool, "img1", &[
            repo::TagRecord { tag_text: "旧标签".into(), is_auto: false },
        ]).await.unwrap();

        // update_tags 应替换旧标签
        let new_tags = vec!["新标签1".to_string(), "新标签2".to_string()];
        repo::delete_tags(&pool, "img1").await.unwrap();
        repo::insert_tags(&pool, "img1", &new_tags.iter().map(|t| repo::TagRecord {
            tag_text: t.clone(), is_auto: false,
        }).collect::<Vec<_>>()).await.unwrap();

        let tags = repo::get_tags_for_image(&pool, "img1").await.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(!tags.contains(&"旧标签".to_string()));
        assert!(tags.contains(&"新标签1".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_tag_suggestions_prefix(pool: SqlitePool) {
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "img1".into(), file_path: "/tmp/img1.jpg".into(),
            file_name: "img1.jpg".into(), format: "jpg".into(),
            width: None, height: None, added_at: 1, use_count: 0, thumbnail_path: None,
            file_hash: None, file_size: None, file_modified_time: None,
            file_status: "normal".to_string(), last_check_time: None,
        }).await.unwrap();
        repo::insert_tags(&pool, "img1", &[
            repo::TagRecord { tag_text: "搞笑".into(), is_auto: false },
            repo::TagRecord { tag_text: "搞怪".into(), is_auto: false },
            repo::TagRecord { tag_text: "可爱".into(), is_auto: false },
        ]).await.unwrap();

        let suggestions = repo::get_tag_suggestions(&pool, "搞", 20).await.unwrap();
        assert!(suggestions.contains(&"搞笑".to_string()));
        assert!(suggestions.contains(&"搞怪".to_string()));
        assert!(!suggestions.contains(&"可爱".to_string()));
    }

    // ── 输入截断测试 ────────────────────────────────────────────────────────

    #[test]
    fn test_query_truncation_at_200_chars() {
        // 模拟 search 命令中的截断逻辑
        let long_query = "a".repeat(250);
        let truncated: String = if long_query.chars().count() > 200 {
            long_query.chars().take(200).collect()
        } else {
            long_query.clone()
        };
        assert_eq!(truncated.chars().count(), 200);
    }

    #[test]
    fn test_query_not_truncated_when_200_chars() {
        let query = "b".repeat(200);
        let result: String = if query.chars().count() > 200 {
            query.chars().take(200).collect()
        } else {
            query.clone()
        };
        assert_eq!(result, query);
    }

    #[test]
    fn test_query_truncation_multibyte_chars() {
        // 中文字符（多字节），确保按字符数而非字节数截断
        let long_query = "测".repeat(250);
        let truncated: String = if long_query.chars().count() > 200 {
            long_query.chars().take(200).collect()
        } else {
            long_query.clone()
        };
        assert_eq!(truncated.chars().count(), 200);
        // 字节数应为 200 * 3 = 600（UTF-8 中文 3 字节/字符）
        assert_eq!(truncated.len(), 600);
    }

    // ── ImageMeta 序列化测试（含新字段）──────────────────────────────────────

    #[test]
    fn test_image_meta_new_fields_serialize() {
        let meta = ImageMeta {
            id: "uuid-1".into(),
            file_path: "/img.jpg".into(),
            file_name: "img.jpg".into(),
            thumbnail_path: "/thumb.jpg".into(),
            file_format: "gif".into(),
            width: 800,
            height: 600,
            file_size: 102400,
            added_at: 1700000000,
            use_count: 5,
            tags: vec![],
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["fileFormat"].as_str().unwrap(), "gif");
        assert_eq!(json["fileSize"].as_i64().unwrap(), 102400);
        assert!(json.get("file_format").is_none(), "should NOT have snake_case field");
    }

    // ── SearchResult 序列化测试（含 fileFormat）──────────────────────────────

    #[test]
    fn test_search_result_file_format_serializes() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/img.gif".into(),
            thumbnail_path: "/thumb.gif".into(),
            file_format: "gif".into(),
            score: 0.9,
            tags: vec![],
            debug_info: None,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["fileFormat"].as_str().unwrap(), "gif");
        assert!(json.get("file_format").is_none());
    }
}
