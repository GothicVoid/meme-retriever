use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::db::{repo, DbPool};
use crate::indexer::{hash, ocr, thumbnail};
use crate::ml::clip::ClipEncoder;
use crate::search::engine::SearchEngine;

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexProgress {
    pub id: String,
    pub file_name: String,
    pub status: String,      // "completed" | "error"
    pub result_kind: String, // "imported" | "duplicated" | "failed"
    pub message: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ResumeIndexTask {
    pub id: String,
    pub file_path: String,
}

enum IndexResult {
    Imported(String),
    Duplicated(String),
}

/// 入库流水线。返回进度接收端，调用方可监听每张图的处理结果。
/// 整体在后台 task 中运行，不阻塞调用方。
pub fn index_images(
    pool: DbPool,
    paths: Vec<String>,
    library_dir: PathBuf,
    engine: std::sync::Arc<SearchEngine>,
) -> mpsc::Receiver<IndexProgress> {
    index_images_with_batch(pool, paths, library_dir, engine, None)
}

pub async fn create_index_tasks(
    pool: &DbPool,
    paths: Vec<String>,
    batch_id: Option<&str>,
) -> anyhow::Result<Vec<ResumeIndexTask>> {
    let mut tasks = Vec::with_capacity(paths.len());
    for path in paths {
        let id = Uuid::new_v4().to_string();
        crate::db::task_repo::insert_task_with_batch(pool, &id, &path, batch_id.unwrap_or(""))
            .await?;
        tasks.push(ResumeIndexTask {
            id,
            file_path: path,
        });
    }
    Ok(tasks)
}

pub fn index_images_with_batch(
    pool: DbPool,
    paths: Vec<String>,
    library_dir: PathBuf,
    engine: std::sync::Arc<SearchEngine>,
    batch_id: Option<String>,
) -> mpsc::Receiver<IndexProgress> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        let Ok(tasks) = create_index_tasks(&pool, paths, batch_id.as_deref()).await else {
            tracing::error!("index_images_with_batch: failed to create task queue");
            return;
        };
        for task in tasks {
            let progress = process_one(
                &pool,
                &task.file_path,
                &library_dir,
                &engine,
                None,
                Some(&task.id),
            )
            .await;
            if tx.send(progress).await.is_err() {
                break; // 接收端已关闭
            }
        }
    });
    rx
}

pub fn resume_index_images(
    pool: DbPool,
    tasks: Vec<ResumeIndexTask>,
    library_dir: PathBuf,
    engine: std::sync::Arc<SearchEngine>,
) -> mpsc::Receiver<IndexProgress> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        for task in tasks {
            let progress = process_one(
                &pool,
                &task.file_path,
                &library_dir,
                &engine,
                None,
                Some(&task.id),
            )
            .await;
            if tx.send(progress).await.is_err() {
                break;
            }
        }
    });
    rx
}

async fn process_one(
    pool: &DbPool,
    src_path: &str,
    library_dir: &Path,
    engine: &SearchEngine,
    batch_id: Option<&str>,
    existing_task_id: Option<&str>,
) -> IndexProgress {
    let start = Instant::now();
    let src = Path::new(src_path);
    let file_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| src_path.to_string());

    match do_index(pool, src, library_dir, engine, batch_id, existing_task_id).await {
        Ok(result) => {
            let (id, result_kind) = match result {
                IndexResult::Imported(id) => (id, "imported"),
                IndexResult::Duplicated(id) => (id, "duplicated"),
            };
            IndexProgress {
                id,
                file_name,
                status: "completed".into(),
                result_kind: result_kind.into(),
                message: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
            }
        }
        Err(e) => {
            tracing::warn!("[INDEX] Failed to process {}: {e}", src_path);
            IndexProgress {
                id: String::new(),
                file_name,
                status: "error".into(),
                result_kind: "failed".into(),
                message: Some(e.to_string()),
                elapsed_ms: start.elapsed().as_millis() as u64,
            }
        }
    }
}

async fn do_index(
    pool: &DbPool,
    src: &Path,
    library_dir: &Path,
    engine: &SearchEngine,
    batch_id: Option<&str>,
    existing_task_id: Option<&str>,
) -> anyhow::Result<IndexResult> {
    // 2.3 任务队列：记录任务进度，支持断点续传
    let task_id = if let Some(task_id) = existing_task_id {
        task_id.to_string()
    } else {
        let task_id = Uuid::new_v4().to_string();
        crate::db::task_repo::insert_task_with_batch(
            pool,
            &task_id,
            &src.to_string_lossy(),
            batch_id.unwrap_or(""),
        )
        .await?;
        task_id
    };
    crate::db::task_repo::update_task_status(pool, &task_id, "processing", None).await?;

    let result = do_index_inner(pool, src, library_dir, engine).await;

    match &result {
        Ok((_, outcome)) => {
            let result_kind = match outcome {
                IndexResult::Imported(_) => Some("imported"),
                IndexResult::Duplicated(_) => Some("duplicated"),
            };
            let _ = crate::db::task_repo::update_task_status_with_result(
                pool,
                &task_id,
                "completed",
                result_kind,
                None,
            )
            .await;
        }
        Err(e) => {
            let _ = crate::db::task_repo::update_task_status_with_result(
                pool,
                &task_id,
                "failed",
                Some("failed"),
                Some(&e.to_string()),
            )
            .await;
        }
    }
    result.map(|(_, outcome)| outcome)
}

async fn do_index_inner(
    pool: &DbPool,
    src: &Path,
    library_dir: &Path,
    engine: &SearchEngine,
) -> anyhow::Result<(String, IndexResult)> {
    if !src.exists() {
        anyhow::bail!("file not found: {:?}", src);
    }

    // 2.1 SHA-256 去重：内容相同的文件直接返回已有 ID
    let file_hash = hash::compute_sha256(src)?;
    let meta = std::fs::metadata(src)?;
    let file_size = meta.len() as i64;
    let file_modified_time = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    if let Some(existing) = repo::get_image_by_hash(pool, &file_hash).await? {
        let id = existing.id;
        return Ok((id.clone(), IndexResult::Duplicated(id)));
    }

    let id = Uuid::new_v4().to_string();
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let thumb = library_dir.join("thumbs").join(format!("{id}.jpg"));

    // 1. 生成缩略图
    let t_thumb = Instant::now();
    thumbnail::generate(src, &thumb, 150)?;
    let thumb_ms = t_thumb.elapsed().as_millis();

    // 3. 并行：OCR + CLIP 图像编码
    let src_str = src.to_string_lossy().to_string();
    let (ocr_result, clip_result) = tokio::join!(
        tokio::task::spawn_blocking({
            let s = src_str.clone();
            move || ocr::extract_text(&s)
        }),
        tokio::task::spawn_blocking({
            let s = src_str.clone();
            move || ClipEncoder::encode_image(&s)
        }),
    );
    let ocr_text = ocr_result??;
    let embedding = clip_result??;

    tracing::info!(
        "[INDEX] {} processed: thumb={}ms ocr={}chars embed={}",
        id,
        thumb_ms,
        ocr_text.len(),
        embedding.len()
    );

    // 4. 读取图片尺寸
    let (width, height) = image_dimensions(src);

    // 5. 写入数据库
    let rec = repo::ImageRecord {
        id: id.clone(),
        file_path: src.to_string_lossy().to_string(),
        file_name: src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        format: ext.to_string(),
        width,
        height,
        added_at: now_secs(),
        use_count: 0,
        thumbnail_path: Some(thumb.to_string_lossy().to_string()),
        file_hash: Some(file_hash),
        file_size: Some(file_size),
        file_modified_time,
        file_status: "normal".to_string(),
        last_check_time: None,
        last_used_at: None,
    };
    repo::insert_image(pool, &rec).await?;
    repo::insert_embedding(pool, &id, &embedding).await?;
    engine.insert_vector(id.clone(), embedding.clone());
    if !ocr_text.is_empty() {
        repo::insert_ocr(pool, &id, &ocr_text).await?;
    }
    let auto_tags = engine.build_auto_tags(&ocr_text, &rec.file_name, Some(&rec.file_path));
    if !auto_tags.is_empty() {
        repo::insert_tags(pool, &id, &auto_tags).await?;
    }

    Ok((id.clone(), IndexResult::Imported(id)))
}

fn image_dimensions(path: &Path) -> (Option<i64>, Option<i64>) {
    match image::image_dimensions(path) {
        Ok((w, h)) => (Some(w as i64), Some(h as i64)),
        Err(_) => (None, None),
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 递归扫描目录，收集所有支持格式（jpg/jpeg/png/gif/webp）的图片路径（已排序）。
pub fn scan_images_in_dir(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    scan_recursive(dir, &mut result)?;
    result.sort();
    Ok(result)
}

fn scan_recursive(dir: &Path, result: &mut Vec<String>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            scan_recursive(&path, result)?;
        } else if is_supported_image(&path) {
            result.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

fn is_supported_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("jpg" | "jpeg" | "png" | "gif" | "webp")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::local::LocalKBProvider;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use std::time::Duration;

    fn fixture(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    async fn collect(mut rx: mpsc::Receiver<IndexProgress>) -> Vec<IndexProgress> {
        let mut results = vec![];
        while let Ok(Some(p)) = tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
            results.push(p);
        }
        results
    }

    async fn make_engine(pool: SqlitePool) -> Arc<SearchEngine> {
        Arc::new(
            SearchEngine::new(pool, Box::new(LocalKBProvider::empty()))
                .await
                .unwrap(),
        )
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_single_image(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let rx = index_images(
            pool.clone(),
            vec![fixture("sample.jpg")],
            lib.path().to_path_buf(),
            engine,
        );
        let results = collect(rx).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "completed");
        assert!(!results[0].id.is_empty());

        // DB 有记录
        let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
        assert_eq!(images.len(), 1);

        // embeddings 有记录
        let embs = repo::get_all_embeddings(&pool).await.unwrap();
        assert_eq!(embs.len(), 1);
        assert_eq!(embs[0].1.len(), 512);

        // thumbnail 文件存在
        let thumb = images[0].thumbnail_path.as_ref().unwrap();
        assert!(Path::new(thumb).exists());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_multiple_images(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let paths = vec![
            fixture("sample.jpg"),
            fixture("sample_blank.jpg"),
            fixture("sample_wide.jpg"),
        ];
        let rx = index_images(pool.clone(), paths, lib.path().to_path_buf(), engine);
        let results = collect(rx).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.status == "completed"));

        // id 不重复
        let ids: std::collections::HashSet<_> = results.iter().map(|r| &r.id).collect();
        assert_eq!(ids.len(), 3);

        let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
        assert_eq!(images.len(), 3);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_invalid_path(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let paths = vec!["/nonexistent/image.jpg".to_string(), fixture("sample.jpg")];
        let rx = index_images(pool.clone(), paths, lib.path().to_path_buf(), engine);
        let results = collect(rx).await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].status, "error");
        assert_eq!(results[1].status, "completed");

        // 只有 1 张成功入库
        let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
        assert_eq!(images.len(), 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_progress_elapsed(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let rx = index_images(
            pool.clone(),
            vec![fixture("sample.jpg")],
            lib.path().to_path_buf(),
            engine,
        );
        let results = collect(rx).await;
        assert!(results[0].elapsed_ms < 10_000, "should complete in < 10s");
    }

    // ── scan_images_in_dir 测试 ─────────────────────────────────────────────

    #[test]
    fn test_scan_dir_finds_images() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(fixture("sample.jpg"), dir.path().join("a.jpg")).unwrap();
        let result = scan_images_in_dir(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].ends_with("a.jpg"));
    }

    #[test]
    fn test_scan_dir_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::copy(fixture("sample.jpg"), sub.join("deep.jpg")).unwrap();
        let result = scan_images_in_dir(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("deep.jpg"));
    }

    #[test]
    fn test_scan_dir_filters_non_images() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.txt"), b"hello").unwrap();
        std::fs::write(dir.path().join("video.mp4"), b"data").unwrap();
        std::fs::copy(fixture("sample.jpg"), dir.path().join("img.png")).unwrap();
        let result = scan_images_in_dir(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].ends_with("img.png"));
    }

    #[test]
    fn test_scan_dir_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_images_in_dir(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_dir_not_found() {
        let result = scan_images_in_dir(std::path::Path::new("/nonexistent/dir/xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_dir_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        // 创建大写扩展名的假图片文件（内容不重要，scan 只按扩展名过滤）
        std::fs::copy(fixture("sample.jpg"), dir.path().join("A.JPG")).unwrap();
        std::fs::copy(fixture("sample.jpg"), dir.path().join("B.PNG")).unwrap();
        let result = scan_images_in_dir(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_stores_original_path(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let src = fixture("sample.jpg");
        let engine = make_engine(pool.clone()).await;
        let rx = index_images(
            pool.clone(),
            vec![src.clone()],
            lib.path().to_path_buf(),
            engine,
        );
        let results = collect(rx).await;
        assert_eq!(results[0].status, "completed");

        let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
        // file_path 必须等于原始路径，不是 library_dir 下的副本
        assert_eq!(images[0].file_path, src);
        // library_dir 下不应存在原文件的副本（只有 thumbs/）
        let copied = lib.path().join(format!("{}.jpg", results[0].id));
        assert!(!copied.exists(), "不应复制文件到 library_dir");
    }

    // ── Phase B：SHA-256 去重 ──────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_dedup_same_file(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let src = fixture("sample.jpg");
        let engine = make_engine(pool.clone()).await;

        let rx1 = index_images(
            pool.clone(),
            vec![src.clone()],
            lib.path().to_path_buf(),
            Arc::clone(&engine),
        );
        let r1 = collect(rx1).await;
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].status, "completed");
        let first_id = r1[0].id.clone();

        // 同一文件再次入库，应返回已有 ID，不新增记录
        let rx2 = index_images(
            pool.clone(),
            vec![src.clone()],
            lib.path().to_path_buf(),
            engine,
        );
        let r2 = collect(rx2).await;
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].status, "completed");
        assert_eq!(r2[0].id, first_id);
        assert_eq!(r1[0].result_kind, "imported");
        assert_eq!(r2[0].result_kind, "duplicated");

        let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
        assert_eq!(images.len(), 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_invalid_path_reports_failed_result_kind(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let paths = vec!["/nonexistent/image.jpg".to_string()];

        let rx = index_images(pool.clone(), paths, lib.path().to_path_buf(), engine);
        let results = collect(rx).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "error");
        assert_eq!(results[0].result_kind, "failed");
    }
}
