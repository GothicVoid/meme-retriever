use sqlx::Row;
use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

use crate::db::{repo, DbPool};
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
    pub file_status: String,
    pub score: f32,
    pub tags: Vec<TagDto>,
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
    pub file_status: String,
    pub width: i64,
    pub height: i64,
    pub file_size: i64,
    pub added_at: i64,
    pub use_count: i64,
    pub tags: Vec<TagDto>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub text: String,
    pub category: repo::TagCategory,
    pub is_auto: bool,
    pub source_strategy: repo::TagSourceStrategy,
    pub confidence: f32,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearGalleryProgress {
    pub current: u64,
    pub total: u64,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn sync_file_status(
    db: &DbPool,
    id: &str,
    file_path: &str,
    current_status: &str,
) -> Result<String, String> {
    let actual_status = if Path::new(file_path).exists() {
        "normal"
    } else {
        "missing"
    };
    if actual_status != current_status {
        repo::update_file_status(db, id, actual_status, now_secs())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(actual_status.to_string())
}

fn to_image_meta(
    img: repo::ImageRecord,
    tags: Vec<repo::TagRecord>,
    file_status: String,
) -> ImageMeta {
    ImageMeta {
        id: img.id,
        file_path: img.file_path,
        file_name: img.file_name,
        thumbnail_path: img.thumbnail_path.unwrap_or_default(),
        file_format: img.format,
        file_status,
        width: img.width.unwrap_or(0),
        height: img.height.unwrap_or(0),
        file_size: img.file_size.unwrap_or(0),
        added_at: img.added_at,
        use_count: img.use_count,
        tags: tags.into_iter().map(TagDto::from).collect(),
    }
}

impl From<repo::TagRecord> for TagDto {
    fn from(value: repo::TagRecord) -> Self {
        Self {
            text: value.tag_text,
            category: value.category,
            is_auto: value.is_auto,
            source_strategy: value.source_strategy,
            confidence: value.confidence,
        }
    }
}

impl From<TagDto> for repo::TagRecord {
    fn from(value: TagDto) -> Self {
        Self {
            tag_text: value.text,
            category: value.category,
            is_auto: value.is_auto,
            source_strategy: value.source_strategy,
            confidence: value.confidence,
        }
    }
}

fn sanitize_tags(tags: Vec<TagDto>) -> Result<Vec<repo::TagRecord>, String> {
    let mut deduped = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for mut tag in tags {
        tag.text = tag.text.trim().to_string();
        if tag.text.is_empty() {
            return Err("标签不能为空".into());
        }
        if tag.text.chars().count() > 50 {
            return Err("标签最长50字符".into());
        }
        let key = tag.text.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        if !tag.is_auto && matches!(tag.source_strategy, repo::TagSourceStrategy::Manual) {
            tag.confidence = 1.0;
        }
        deduped.push(tag.into());
    }
    Ok(deduped)
}

async fn get_image_meta_impl(id: String, db: &DbPool) -> Result<Option<ImageMeta>, String> {
    let img = repo::get_image(db, &id).await.map_err(|e| e.to_string())?;
    match img {
        None => Ok(None),
        Some(img) => {
            let file_status =
                sync_file_status(db, &img.id, &img.file_path, &img.file_status).await?;
            let tags = repo::get_tags_for_image(db, &img.id)
                .await
                .unwrap_or_default();
            Ok(Some(to_image_meta(img, tags, file_status)))
        }
    }
}

async fn relocate_image_impl(
    id: &str,
    new_path: &str,
    db: &DbPool,
    engine: &SearchEngine,
    library_dir: &Path,
) -> Result<ImageMeta, String> {
    let mut img = repo::get_image(db, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("image not found: {id}"))?;

    let src = Path::new(new_path);
    if !src.exists() {
        return Err("选择的文件不存在".into());
    }

    let file_hash = crate::indexer::hash::compute_sha256(src).map_err(|e| e.to_string())?;
    if let Some(existing) = repo::get_image_by_hash(db, &file_hash)
        .await
        .map_err(|e| e.to_string())?
    {
        if existing.id != id {
            return Err("已存在相同内容图片".into());
        }
    }

    let file_name = src
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| new_path.to_string());
    let format = src
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let metadata = std::fs::metadata(src).map_err(|e| e.to_string())?;
    let file_size = metadata.len() as i64;
    let file_modified_time = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);
    let thumb = img
        .thumbnail_path
        .clone()
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| {
            library_dir
                .join("thumbs")
                .join(format!("{id}.jpg"))
                .to_string_lossy()
                .to_string()
        });

    crate::indexer::thumbnail::generate(src, Path::new(&thumb), 150).map_err(|e| e.to_string())?;

    let src_string = src.to_string_lossy().to_string();
    let (ocr_result, clip_result) = tokio::join!(
        tokio::task::spawn_blocking({
            let value = src_string.clone();
            move || crate::indexer::ocr::extract_text(&value)
        }),
        tokio::task::spawn_blocking({
            let value = src_string.clone();
            move || crate::ml::clip::ClipEncoder::encode_image(&value)
        }),
    );
    let ocr_text = ocr_result
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let embedding = clip_result
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let (width, height) = match image::image_dimensions(src) {
        Ok((w, h)) => (Some(w as i64), Some(h as i64)),
        Err(_) => (None, None),
    };

    img.file_path = src_string;
    img.file_name = file_name;
    img.format = format;
    img.width = width;
    img.height = height;
    img.thumbnail_path = Some(thumb);
    img.file_hash = Some(file_hash);
    img.file_size = Some(file_size);
    img.file_modified_time = file_modified_time;
    img.file_status = "normal".to_string();
    img.last_check_time = Some(now_secs());

    repo::update_image_file_info(db, &img)
        .await
        .map_err(|e| e.to_string())?;
    repo::insert_embedding(db, id, &embedding)
        .await
        .map_err(|e| e.to_string())?;
    if ocr_text.is_empty() {
        repo::delete_ocr_for_image(db, id)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        repo::insert_ocr(db, id, &ocr_text)
            .await
            .map_err(|e| e.to_string())?;
    }
    let manual_tags = repo::get_tags_for_image(db, id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|tag| !tag.is_auto)
        .collect::<Vec<_>>();
    let auto_tags = engine.build_auto_tags(&ocr_text, &img.file_name);
    let mut next_tags = manual_tags;
    next_tags.extend(auto_tags);
    repo::delete_tags(db, id).await.map_err(|e| e.to_string())?;
    repo::insert_tags(db, id, &next_tags)
        .await
        .map_err(|e| e.to_string())?;
    engine.remove_vector(id);
    engine.insert_vector(id.to_string(), embedding);

    let tags = repo::get_tags_for_image(db, id).await.unwrap_or_default();
    Ok(to_image_meta(img, tags, "normal".to_string()))
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
        let mut rx =
            crate::indexer::pipeline::index_images(pool, paths, library_dir, Arc::clone(&engine));
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
        if sum == 0.0 {
            (0.3, 0.4, 0.3)
        } else {
            (a / sum, b / sum, c / sum)
        }
    };
    tracing::info!("search: query={query}, limit={limit}, weights=({rw1:.2},{rw2:.2},{rw3:.2})");
    engine
        .search(&query, limit, rw1, rw2, rw3)
        .await
        .map_err(|e| {
            tracing::error!("command search failed: {e}");
            e.to_string()
        })
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

    spawn_index_task(
        paths,
        library_dir,
        db.inner().clone(),
        Arc::clone(engine.inner()),
        app,
    );
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
    let paths = scan_images_in_dir(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
    let total = paths.len();
    tracing::info!("add_folder: {path} → {total} images");
    if total > 0 {
        let library_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("library");
        spawn_index_task(
            paths,
            library_dir,
            db.inner().clone(),
            Arc::clone(engine.inner()),
            app,
        );
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
    repo::delete_image(db.inner(), &id).await.map_err(|e| {
        tracing::error!("command delete_image failed: {e}");
        e.to_string()
    })?;
    engine.remove_vector(&id);
    Ok(())
}

#[tauri::command]
pub async fn clear_gallery(
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    const BATCH_SIZE: i64 = 1000;

    let total = repo::get_all_images(db.inner())
        .await
        .map_err(|e| e.to_string())?
        .len() as u64;

    if total == 0 {
        return Ok(());
    }

    let pool = db.inner().clone();
    let engine = Arc::clone(engine.inner());

    tokio::spawn(async move {
        let emit_progress = |current: u64| {
            let progress = ClearGalleryProgress { current, total };
            let _ = app.emit("clear-gallery-progress", &progress);
        };

        emit_progress(0);

        let mut deleted = 0u64;
        loop {
            let rows = match sqlx::query("SELECT id FROM images ORDER BY added_at ASC LIMIT ?1")
                .bind(BATCH_SIZE)
                .fetch_all(&pool)
                .await
            {
                Ok(rows) => rows,
                Err(err) => {
                    tracing::error!("clear_gallery: failed to load batch: {err}");
                    break;
                }
            };

            if rows.is_empty() {
                break;
            }

            let ids: Vec<String> = rows.into_iter().map(|row| row.get("id")).collect();
            let mut tx = match pool.begin().await {
                Ok(tx) => tx,
                Err(err) => {
                    tracing::error!("clear_gallery: failed to begin transaction: {err}");
                    break;
                }
            };

            let mut batch_failed = false;
            for id in &ids {
                if let Err(err) = sqlx::query("DELETE FROM ocr_fts WHERE image_id=?1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                {
                    tracing::error!("clear_gallery: failed to delete ocr_fts for {id}: {err}");
                    batch_failed = true;
                    break;
                }
                if let Err(err) = sqlx::query("DELETE FROM images WHERE id=?1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                {
                    tracing::error!("clear_gallery: failed to delete image {id}: {err}");
                    batch_failed = true;
                    break;
                }
            }

            if batch_failed {
                break;
            }

            if let Err(err) = tx.commit().await {
                tracing::error!("clear_gallery: failed to commit transaction: {err}");
                break;
            }

            deleted += ids.len() as u64;
            emit_progress(deleted.min(total));
        }

        if deleted == total {
            engine.clear_all_vectors();
            emit_progress(total);
            tracing::info!("clear_gallery: completed, deleted {deleted} images");
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn clear_missing_images(
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<u64, String> {
    clear_missing_images_impl(db.inner(), engine.inner()).await
}

async fn clear_missing_images_impl(db: &DbPool, engine: &EngineState) -> Result<u64, String> {
    let images = repo::get_all_images(db).await.map_err(|e| e.to_string())?;

    let mut removed = 0u64;
    for img in images {
        let is_missing = img.file_status == "missing" || !Path::new(&img.file_path).exists();
        if !is_missing {
            continue;
        }

        repo::delete_image(db, &img.id)
            .await
            .map_err(|e| e.to_string())?;
        engine.remove_vector(&img.id);
        removed += 1;
    }

    Ok(removed)
}

/// 获取单张图片的完整元数据（用于详情页）
#[tauri::command]
pub async fn get_image_meta(
    id: String,
    db: State<'_, DbPool>,
) -> Result<Option<ImageMeta>, String> {
    get_image_meta_impl(id, db.inner()).await
}

#[tauri::command]
pub async fn get_images(page: i64, db: State<'_, DbPool>) -> Result<Vec<ImageMeta>, String> {
    tracing::info!("get_images: page={page}");
    let images = repo::get_images_paged(db.inner(), page, 15)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::with_capacity(images.len());
    for img in images {
        let file_status =
            sync_file_status(db.inner(), &img.id, &img.file_path, &img.file_status).await?;
        let tags = repo::get_tags_for_image(db.inner(), &img.id)
            .await
            .unwrap_or_default();
        result.push(to_image_meta(img, tags, file_status));
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_image_count(db: State<'_, DbPool>) -> Result<i64, String> {
    tracing::info!("get_image_count");
    repo::get_image_count(db.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tags(
    image_id: String,
    tags: Vec<TagDto>,
    db: State<'_, DbPool>,
) -> Result<(), String> {
    tracing::info!("update_tags: image={image_id}, count={}", tags.len());
    let tags = sanitize_tags(tags)?;
    repo::delete_tags(db.inner(), &image_id)
        .await
        .map_err(|e| e.to_string())?;
    repo::insert_tags(db.inner(), &image_id, &tags)
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
pub async fn copy_to_clipboard(id: String, db: State<'_, DbPool>) -> Result<(), String> {
    copy_to_clipboard_impl(&id, db.inner(), |image_data| {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_image(image_data).map_err(|e| e.to_string())
    })
    .await
}

async fn copy_to_clipboard_impl<F>(id: &str, db: &DbPool, set_image: F) -> Result<(), String>
where
    F: FnOnce(arboard::ImageData<'static>) -> Result<(), String>,
{
    tracing::info!("copy_to_clipboard: {id}");
    let img = repo::get_image(db, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("image not found: {id}"))?;
    let file_status = sync_file_status(db, &img.id, &img.file_path, &img.file_status).await?;
    if file_status == "missing" {
        return Err("原文件已丢失，无法复制".into());
    }

    tracing::debug!("copy_to_clipboard: path={}", img.file_path);
    let image_data = load_image_for_clipboard(&img.file_path)?;
    set_image(image_data)?;
    repo::increment_use_count(db, id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn load_image_for_clipboard(path: &str) -> Result<arboard::ImageData<'static>, String> {
    let rgba = image::open(path)
        .map_err(|e| format!("failed to open image: {e}"))?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    let bytes = rgba.into_raw();

    Ok(arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(bytes),
    })
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
    let file_status =
        sync_file_status(db.inner(), &img.id, &img.file_path, &img.file_status).await?;
    if file_status == "missing" {
        return Err("原文件已丢失，无法定位".into());
    }

    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&img.file_path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn relocate_image(
    id: String,
    new_path: String,
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<ImageMeta, String> {
    let library_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("library");
    relocate_image_impl(&id, &new_path, db.inner(), engine.inner(), &library_dir).await
}

#[tauri::command]
pub async fn increment_use_count(id: String, db: State<'_, DbPool>) -> Result<(), String> {
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
            let id = img.id.clone();

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
                Err(e) => tracing::warn!("reindex_all: clip task panicked for {id}: {e}"),
            }

            // 更新 OCR
            match ocr_result {
                Ok(Ok(text)) if !text.is_empty() => {
                    if let Err(e) = repo::insert_ocr(&pool, &id, &text).await {
                        tracing::error!("reindex_all: failed to save ocr for {id}: {e}");
                    } else {
                        tracing::debug!("reindex_all: ocr ok for {id} len={}", text.len());
                        let manual_tags = repo::get_tags_for_image(&pool, &id)
                            .await
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|tag| !tag.is_auto)
                            .collect::<Vec<_>>();
                        let mut next_tags = manual_tags;
                        next_tags.extend(engine.build_auto_tags(&text, &img.file_name));
                        if let Err(e) = repo::delete_tags(&pool, &id).await {
                            tracing::warn!("reindex_all: failed to clear tags for {id}: {e}");
                        } else if let Err(e) = repo::insert_tags(&pool, &id, &next_tags).await {
                            tracing::warn!("reindex_all: failed to write tags for {id}: {e}");
                        }
                    }
                }
                Ok(Ok(_)) => {
                    // 无文字，清除旧 OCR 数据
                    if let Err(e) = repo::delete_ocr_for_image(&pool, &id).await {
                        tracing::warn!("reindex_all: failed to clear old ocr for {id}: {e}");
                    }
                    let manual_tags = repo::get_tags_for_image(&pool, &id)
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|tag| !tag.is_auto)
                        .collect::<Vec<_>>();
                    let mut next_tags = manual_tags;
                    next_tags.extend(engine.build_auto_tags("", &img.file_name));
                    if let Err(e) = repo::delete_tags(&pool, &id).await {
                        tracing::warn!("reindex_all: failed to clear tags for {id}: {e}");
                    } else if let Err(e) = repo::insert_tags(&pool, &id, &next_tags).await {
                        tracing::warn!("reindex_all: failed to restore manual tags for {id}: {e}");
                    }
                }
                Ok(Err(e)) => tracing::warn!("reindex_all: ocr failed for {id}: {e}"),
                Err(e) => tracing::warn!("reindex_all: ocr task panicked for {id}: {e}"),
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
    let images = repo::get_all_images(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let mut updated = 0u64;
    for img in &images {
        let status = if std::path::Path::new(&img.file_path).exists() {
            "normal"
        } else {
            "missing"
        };
        if status != img.file_status {
            repo::update_file_status(db.inner(), &img.id, status, now)
                .await
                .map_err(|e| e.to_string())?;
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
    crate::db::task_repo::get_pending_tasks(db.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_pending_tasks(
    app: tauri::AppHandle,
    db: State<'_, DbPool>,
    engine: State<'_, EngineState>,
) -> Result<usize, String> {
    crate::db::task_repo::reset_stale_tasks(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let pending = crate::db::task_repo::get_pending_tasks(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let count = pending.len();
    if count > 0 {
        let paths: Vec<String> = pending.into_iter().map(|t| t.file_path).collect();
        let library_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("library");
        spawn_index_task(
            paths,
            library_dir,
            db.inner().clone(),
            Arc::clone(engine.inner()),
            app,
        );
    }
    Ok(count)
}

#[tauri::command]
pub async fn clear_task_queue(db: State<'_, DbPool>) -> Result<(), String> {
    crate::db::task_repo::clear_task_queue(db.inner())
        .await
        .map_err(|e| e.to_string())
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::local::LocalKBProvider;
    use image::{ImageBuffer, Rgba};
    use sqlx::SqlitePool;

    fn manual_tag(text: &str) -> repo::TagRecord {
        repo::TagRecord {
            tag_text: text.to_string(),
            category: repo::TagCategory::Custom,
            is_auto: false,
            source_strategy: repo::TagSourceStrategy::Manual,
            confidence: 1.0,
        }
    }

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
            file_status: "normal".into(),
            width: 800,
            height: 600,
            file_size: 0,
            added_at: 1700000000,
            use_count: 0,
            tags: vec![],
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert!(
            json.get("thumbnailPath").is_some(),
            "should have thumbnailPath"
        );
        assert!(json.get("filePath").is_some(), "should have filePath");
        assert!(json.get("fileName").is_some(), "should have fileName");
        assert!(
            json.get("thumbnail_path").is_none(),
            "should NOT have thumbnail_path"
        );
    }

    #[test]
    fn test_search_result_serializes_camel_case() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/library/images/uuid-1.jpg".into(),
            thumbnail_path: "/library/thumbs/uuid-1.jpg".into(),
            file_format: "jpg".into(),
            file_status: "normal".into(),
            score: 0.9,
            tags: vec![],
            debug_info: None,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(
            json.get("thumbnailPath").is_some(),
            "should have thumbnailPath"
        );
        assert!(json.get("filePath").is_some(), "should have filePath");
        assert!(
            json.get("thumbnail_path").is_none(),
            "should NOT have thumbnail_path"
        );
        assert!(
            json.get("debugInfo").is_some(),
            "should have debugInfo (null)"
        );
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
    fn test_clear_gallery_progress_event_serializes() {
        let progress = ClearGalleryProgress {
            current: 3,
            total: 10,
        };
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json.get("current").and_then(|v| v.as_u64()), Some(3));
        assert_eq!(json.get("total").and_then(|v| v.as_u64()), Some(10));
    }

    #[test]
    fn test_load_image_for_clipboard_reads_rgba_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clipboard-test.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 128])
            }
        });
        img.save(&path).unwrap();

        let data = load_image_for_clipboard(path.to_str().unwrap()).unwrap();

        assert_eq!(data.width, 2);
        assert_eq!(data.height, 1);
        assert_eq!(data.bytes.as_ref(), &[255, 0, 0, 255, 0, 0, 255, 128]);
    }

    #[test]
    fn test_load_image_for_clipboard_rejects_missing_file() {
        let err = load_image_for_clipboard("/definitely/not/found.png").unwrap_err();
        assert!(err.contains("failed to open image"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_copy_to_clipboard_impl_increments_use_count_on_success(pool: SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copy-success.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(1, 1, Rgba([12, 34, 56, 255]));
        img.save(&path).unwrap();

        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-copy".into(),
                file_path: path.to_string_lossy().to_string(),
                file_name: "copy-success.png".into(),
                format: "png".into(),
                width: Some(1),
                height: Some(1),
                added_at: 1,
                use_count: 0,
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

        copy_to_clipboard_impl("img-copy", &pool, |_| Ok(()))
            .await
            .unwrap();

        let image = repo::get_image(&pool, "img-copy").await.unwrap().unwrap();
        assert_eq!(image.use_count, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_copy_to_clipboard_impl_does_not_increment_on_copy_failure(pool: SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copy-fail.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(1, 1, Rgba([78, 90, 12, 255]));
        img.save(&path).unwrap();

        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-copy-fail".into(),
                file_path: path.to_string_lossy().to_string(),
                file_name: "copy-fail.png".into(),
                format: "png".into(),
                width: Some(1),
                height: Some(1),
                added_at: 1,
                use_count: 0,
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

        let err =
            copy_to_clipboard_impl("img-copy-fail", &pool, |_| Err("clipboard failed".into()))
                .await
                .unwrap_err();
        assert_eq!(err, "clipboard failed");

        let image = repo::get_image(&pool, "img-copy-fail")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(image.use_count, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_copy_to_clipboard_impl_marks_missing_file(pool: SqlitePool) {
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-missing".into(),
                file_path: "/definitely/not/found.png".into(),
                file_name: "not-found.png".into(),
                format: "png".into(),
                width: Some(1),
                height: Some(1),
                added_at: 1,
                use_count: 0,
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

        let err = copy_to_clipboard_impl("img-missing", &pool, |_| Ok(()))
            .await
            .unwrap_err();
        assert_eq!(err, "原文件已丢失，无法复制");

        let image = repo::get_image(&pool, "img-missing")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(image.file_status, "missing");
        assert!(image.last_check_time.is_some());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_image_meta_impl_marks_missing_file(pool: SqlitePool) {
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-meta-missing".into(),
                file_path: "/definitely/not/found-meta.png".into(),
                file_name: "not-found-meta.png".into(),
                format: "png".into(),
                width: Some(1),
                height: Some(1),
                added_at: 1,
                use_count: 0,
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

        let meta = get_image_meta_impl("img-meta-missing".into(), &pool)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta.file_status, "missing");

        let image = repo::get_image(&pool, "img-meta-missing")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(image.file_status, "missing");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_relocate_image_impl_updates_metadata_and_embedding(pool: SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("replacement.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(3, 2, Rgba([22, 33, 44, 255]));
        img.save(&src).unwrap();

        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-relocate".into(),
                file_path: "/missing/original.png".into(),
                file_name: "original.png".into(),
                format: "png".into(),
                width: Some(1),
                height: Some(1),
                added_at: 1,
                use_count: 2,
                thumbnail_path: Some(
                    dir.path()
                        .join("thumbs")
                        .join("img-relocate.jpg")
                        .to_string_lossy()
                        .to_string(),
                ),
                file_hash: None,
                file_size: None,
                file_modified_time: None,
                file_status: "missing".to_string(),
                last_check_time: None,
            },
        )
        .await
        .unwrap();
        repo::insert_embedding(&pool, "img-relocate", &[0.1, 0.2, 0.3])
            .await
            .unwrap();

        let engine = make_engine(pool.clone()).await;
        let meta = relocate_image_impl(
            "img-relocate",
            src.to_str().unwrap(),
            &pool,
            &engine,
            dir.path(),
        )
        .await
        .unwrap();

        assert_eq!(meta.file_status, "normal");
        assert_eq!(meta.file_name, "replacement.png");
        assert_eq!(meta.width, 3);
        assert_eq!(meta.height, 2);

        let image = repo::get_image(&pool, "img-relocate")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(image.file_status, "normal");
        assert_eq!(image.file_path, src.to_string_lossy());
        assert!(Path::new(image.thumbnail_path.as_deref().unwrap()).exists());

        let embedding = repo::get_embedding(&pool, "img-relocate")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(embedding.len(), 512);
        assert_eq!(engine.vector_store_len(), 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_clear_missing_images_removes_only_missing_files(pool: SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let existing_path = dir.path().join("exists.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(2, 2, Rgba([100, 120, 140, 255]));
        img.save(&existing_path).unwrap();

        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-normal".into(),
                file_path: existing_path.to_string_lossy().to_string(),
                file_name: "exists.png".into(),
                format: "png".into(),
                width: Some(2),
                height: Some(2),
                added_at: 1,
                use_count: 0,
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
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img-missing".into(),
                file_path: "/definitely/not/found-clear-missing.png".into(),
                file_name: "missing.png".into(),
                format: "png".into(),
                width: Some(2),
                height: Some(2),
                added_at: 2,
                use_count: 0,
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

        let engine = make_engine(pool.clone()).await;
        engine.insert_vector("img-normal".into(), vec![0.1, 0.2, 0.3]);
        engine.insert_vector("img-missing".into(), vec![0.4, 0.5, 0.6]);

        let removed = clear_missing_images_impl(&pool, &engine).await.unwrap();

        assert_eq!(removed, 1);
        assert!(repo::get_image(&pool, "img-normal")
            .await
            .unwrap()
            .is_some());
        assert!(repo::get_image(&pool, "img-missing")
            .await
            .unwrap()
            .is_none());
        assert_eq!(engine.vector_store_len(), 1);
    }

    #[test]
    fn test_search_result_has_debug_info_field() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/path/img.jpg".into(),
            thumbnail_path: "/path/thumb.jpg".into(),
            file_format: "jpg".into(),
            file_status: "normal".into(),
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
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img1".into(),
                file_path: "/tmp/img1.jpg".into(),
                file_name: "img1.jpg".into(),
                format: "jpg".into(),
                width: None,
                height: None,
                added_at: 1,
                use_count: 0,
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
        repo::insert_tags(&pool, "img1", &[manual_tag("旧标签")])
            .await
            .unwrap();

        // update_tags 应替换旧标签
        let new_tags = vec!["新标签1".to_string(), "新标签2".to_string()];
        repo::delete_tags(&pool, "img1").await.unwrap();
        repo::insert_tags(
            &pool,
            "img1",
            &new_tags
                .iter()
                .map(|t| repo::TagRecord {
                    tag_text: t.clone(),
                    category: repo::TagCategory::Custom,
                    is_auto: false,
                    source_strategy: repo::TagSourceStrategy::Manual,
                    confidence: 1.0,
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();

        let tags = repo::get_tags_for_image(&pool, "img1").await.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(!tags.iter().any(|tag| tag.tag_text == "旧标签"));
        assert!(tags.iter().any(|tag| tag.tag_text == "新标签1"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_tag_suggestions_prefix(pool: SqlitePool) {
        repo::insert_image(
            &pool,
            &repo::ImageRecord {
                id: "img1".into(),
                file_path: "/tmp/img1.jpg".into(),
                file_name: "img1.jpg".into(),
                format: "jpg".into(),
                width: None,
                height: None,
                added_at: 1,
                use_count: 0,
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
        repo::insert_tags(
            &pool,
            "img1",
            &[manual_tag("搞笑"), manual_tag("搞怪"), manual_tag("可爱")],
        )
        .await
        .unwrap();

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
            file_status: "missing".into(),
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
        assert!(
            json.get("file_format").is_none(),
            "should NOT have snake_case field"
        );
    }

    // ── SearchResult 序列化测试（含 fileFormat）──────────────────────────────

    #[test]
    fn test_search_result_file_format_serializes() {
        let result = SearchResult {
            id: "uuid-1".into(),
            file_path: "/img.gif".into(),
            thumbnail_path: "/thumb.gif".into(),
            file_format: "gif".into(),
            file_status: "normal".into(),
            score: 0.9,
            tags: vec![],
            debug_info: None,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["fileFormat"].as_str().unwrap(), "gif");
        assert!(json.get("file_format").is_none());
    }
}
