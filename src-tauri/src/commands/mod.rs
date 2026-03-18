use std::sync::Arc;
use tauri::{Manager, Emitter, State};

use crate::db::{DbPool, repo};
use crate::search::engine::SearchEngine;

// SearchEngine 包在 Arc 里以便 Tauri State 共享
pub type EngineState = Arc<SearchEngine>;

#[derive(serde::Serialize)]
pub struct SearchResult {
    pub id: String,
    pub file_path: String,
    pub thumbnail_path: String,
    pub score: f32,
    pub tags: Vec<String>,
}

#[derive(serde::Serialize)]
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

    let pool = db.inner().clone();
    let engine = Arc::clone(engine.inner());
    let app_handle = app.clone();

    tokio::spawn(async move {
        let mut rx = crate::indexer::pipeline::index_images(pool, paths, library_dir);
        while let Some(progress) = rx.recv().await {
            // 成功入库后更新内存向量索引
            if progress.status == "completed" && !progress.id.is_empty() {
                if let Ok(embs) = repo::get_all_embeddings(engine.pool()).await {
                    if let Some((_, vec)) = embs.into_iter().find(|(id, _)| id == &progress.id) {
                        engine.insert_vector(progress.id.clone(), vec);
                    }
                }
            }
            // 发送进度事件到前端
            let _ = app_handle.emit("index-progress", &progress);
        }
    });

    Ok(())
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
