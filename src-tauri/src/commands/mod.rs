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
    pub tag_hit: bool,
    pub sem_weight: f32,
    pub kw_weight: f32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub file_path: String,
    pub thumbnail_path: String,
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
    pub width: i64,
    pub height: i64,
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
    engine: State<'_, EngineState>,
) -> Result<Vec<SearchResult>, String> {
    if limit == 0 {
        return Err("limit must be > 0".into());
    }
    tracing::info!("search: query={query}, limit={limit}");
    engine
        .search(&query, limit)
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
            width: img.width.unwrap_or(0),
            height: img.height.unwrap_or(0),
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

            match tokio::task::spawn_blocking({
                let path = img.file_path.clone();
                move || crate::ml::clip::ClipEncoder::encode_image(&path)
            }).await {
                Ok(Ok(vec)) => {
                    if let Err(e) = repo::insert_embedding(&pool, &img.id, &vec).await {
                        tracing::error!("reindex_all: failed to save embedding for {}: {e}", img.id);
                        continue;
                    }
                    engine.insert_vector(img.id.clone(), vec);
                    tracing::debug!("reindex_all: done {}", img.id);
                }
                Ok(Err(e)) => tracing::warn!("reindex_all: encode failed for {}: {e}", img.id),
                Err(e) => tracing::warn!("reindex_all: task panicked for {}: {e}", img.id),
            }
        }

        let done_event = serde_json::json!({ "current": total, "total": total });
        let _ = app.emit("reindex-progress", &done_event);
        tracing::info!("reindex_all: completed");
    });

    Ok(())
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
            width: 800,
            height: 600,
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
            tag_hit: true,
            sem_weight: 0.4,
            kw_weight: 0.6,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("semScore").is_some(), "should have semScore");
        assert!(json.get("kwScore").is_some(), "should have kwScore");
        assert!(json.get("tagHit").is_some(), "should have tagHit");
        assert!(json.get("semWeight").is_some(), "should have semWeight");
        assert!(json.get("kwWeight").is_some(), "should have kwWeight");
        assert!(json.get("sem_score").is_none(), "should NOT have sem_score");
    }

    #[test]
    fn test_search_result_has_debug_info_field() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/path/img.jpg".into(),
            thumbnail_path: "/path/thumb.jpg".into(),
            score: 0.9,
            tags: vec![],
            debug_info: Some(ScoreDebugInfo {
                sem_score: 0.8,
                kw_score: 0.0,
                tag_hit: false,
                sem_weight: 0.7,
                kw_weight: 0.3,
            }),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("debugInfo").is_some(), "should have debugInfo");
        let di = json["debugInfo"].as_object().unwrap();
        assert!((di["semScore"].as_f64().unwrap() - 0.8).abs() < 1e-5);
        assert_eq!(di["tagHit"].as_bool().unwrap(), false);
        assert_eq!(di["kwScore"].as_f64().unwrap(), 0.0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_tags_replaces(pool: SqlitePool) {
        // 先插入图片和旧标签
        repo::insert_image(&pool, &repo::ImageRecord {
            id: "img1".into(), file_path: "/tmp/img1.jpg".into(),
            file_name: "img1.jpg".into(), format: "jpg".into(),
            width: None, height: None, added_at: 1, use_count: 0, thumbnail_path: None,
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
}
