use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::db::{repo, DbPool};
use crate::indexer::{hash, index_features, thumbnail};
use crate::search::engine::SearchEngine;

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexProgress {
    pub id: String,
    pub file_name: String,
    pub status: String,      // "completed" | "error"
    pub result_kind: String, // "imported" | "duplicated" | "failed"
    pub message: Option<String>,
    pub elapsed_ms: u64,
    #[serde(skip_serializing, skip_deserializing)]
    pub embedding: Option<Vec<f32>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub(crate) metrics: Option<StageMetrics>,
}

#[derive(Debug, Clone)]
pub struct ResumeIndexTask {
    pub id: String,
    pub file_path: String,
}

enum IndexResult {
    Imported(ImportedImage),
    Duplicated(String),
}

struct ImportedImage {
    id: String,
    embedding: Vec<f32>,
    metrics: StageMetrics,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StageMetrics {
    pub queue_wait_ms: u64,
    pub hash_ms: u64,
    pub decode_ms: u64,
    pub thumb_ms: u64,
    pub ocr_ms: u64,
    pub clip_ms: u64,
    pub db_ms: u64,
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
    let tasks = paths
        .into_iter()
        .map(|path| ResumeIndexTask {
            id: Uuid::new_v4().to_string(),
            file_path: path,
        })
        .collect::<Vec<_>>();
    let task_rows = tasks
        .iter()
        .map(|task| (task.id.clone(), task.file_path.clone()))
        .collect::<Vec<_>>();
    crate::db::task_repo::insert_tasks_with_batch(pool, &task_rows, batch_id.unwrap_or("")).await?;
    Ok(tasks)
}

pub async fn create_index_task(
    pool: &DbPool,
    path: String,
    batch_id: Option<&str>,
) -> anyhow::Result<ResumeIndexTask> {
    let task = ResumeIndexTask {
        id: Uuid::new_v4().to_string(),
        file_path: path,
    };
    crate::db::task_repo::insert_task_with_batch(
        pool,
        &task.id,
        &task.file_path,
        batch_id.unwrap_or(""),
    )
    .await?;
    Ok(task)
}

pub fn index_images_with_batch(
    pool: DbPool,
    paths: Vec<String>,
    library_dir: PathBuf,
    engine: std::sync::Arc<SearchEngine>,
    batch_id: Option<String>,
) -> mpsc::Receiver<IndexProgress> {
    let (task_tx, rx) = start_index_task_stream(pool.clone(), library_dir.clone(), engine.clone());
    tokio::spawn(async move {
        let Ok(tasks) = create_index_tasks(&pool, paths, batch_id.as_deref()).await else {
            tracing::error!("index_images_with_batch: failed to create task queue");
            return;
        };
        for task in tasks {
            if task_tx.send(task).await.is_err() {
                break;
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
    let (task_tx, rx) = start_index_task_stream(pool, library_dir, engine);
    tokio::spawn(async move {
        for task in tasks {
            if task_tx.send(task).await.is_err() {
                break;
            }
        }
    });
    rx
}

pub fn start_index_task_stream(
    pool: DbPool,
    library_dir: PathBuf,
    engine: Arc<SearchEngine>,
) -> (mpsc::Sender<ResumeIndexTask>, mpsc::Receiver<IndexProgress>) {
    let (task_tx, task_rx) = mpsc::channel(64);
    let (progress_tx, progress_rx) = mpsc::channel(64);
    tokio::spawn(async move {
        run_task_stream(pool, task_rx, library_dir, engine, progress_tx).await;
    });
    (task_tx, progress_rx)
}

async fn run_task_stream(
    pool: DbPool,
    mut task_rx: mpsc::Receiver<ResumeIndexTask>,
    library_dir: PathBuf,
    engine: Arc<SearchEngine>,
    progress_tx: mpsc::Sender<IndexProgress>,
) {
    let semaphore = Arc::new(Semaphore::new(default_index_concurrency()));
    let mut join_set = JoinSet::new();

    while let Some(task) = task_rx.recv().await {
        let permit_pool = Arc::clone(&semaphore);
        let tx = progress_tx.clone();
        let worker_pool = pool.clone();
        let worker_library_dir = library_dir.clone();
        let worker_engine = Arc::clone(&engine);
        let queued_at = Instant::now();

        join_set.spawn(async move {
            let _permit = permit_pool.acquire_owned().await?;
            let progress = process_one(
                &worker_pool,
                task.file_path,
                worker_library_dir,
                worker_engine,
                None,
                Some(task.id),
                Some(queued_at),
            )
            .await;
            let _ = tx.send(progress).await;
            Ok::<_, anyhow::Error>(())
        });
    }

    drop(progress_tx);

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                tracing::warn!("index worker failed before processing: {err}");
            }
            Err(err) => {
                tracing::warn!("index worker join failed: {err}");
            }
        }
    }
}

async fn process_one(
    pool: &DbPool,
    src_path: String,
    library_dir: PathBuf,
    engine: Arc<SearchEngine>,
    batch_id: Option<&str>,
    existing_task_id: Option<String>,
    queued_at: Option<Instant>,
) -> IndexProgress {
    let start = Instant::now();
    let src = Path::new(&src_path);
    let file_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| src_path.clone());

    match do_index(
        pool,
        src,
        &library_dir,
        &engine,
        batch_id,
        existing_task_id.as_deref(),
        queued_at,
    )
    .await
    {
        Ok(result) => {
            let (id, result_kind, embedding, metrics) = match result {
                IndexResult::Imported(imported) => (
                    imported.id,
                    "imported",
                    Some(imported.embedding),
                    Some(imported.metrics),
                ),
                IndexResult::Duplicated(id) => (id, "duplicated", None, None),
            };
            if let Some(vec) = embedding.as_ref() {
                engine.insert_vector(id.clone(), vec.clone());
            }
            IndexProgress {
                id,
                file_name,
                status: "completed".into(),
                result_kind: result_kind.into(),
                message: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
                embedding,
                metrics,
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
                embedding: None,
                metrics: None,
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
    queued_at: Option<Instant>,
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

    let result = do_index_inner(pool, src, library_dir, engine, &task_id, queued_at).await;

    match &result {
        Ok(IndexResult::Imported(_)) => {}
        Ok(IndexResult::Duplicated(_)) => {
            let _ = crate::db::task_repo::update_task_status_with_result(
                pool,
                &task_id,
                "completed",
                Some("duplicated"),
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
    result
}

async fn do_index_inner(
    pool: &DbPool,
    src: &Path,
    library_dir: &Path,
    _engine: &SearchEngine,
    task_id: &str,
    queued_at: Option<Instant>,
) -> anyhow::Result<IndexResult> {
    if !src.exists() {
        anyhow::bail!("file not found: {:?}", src);
    }

    let mut metrics = StageMetrics {
        queue_wait_ms: queued_at
            .map(|queued_at| queued_at.elapsed().as_millis() as u64)
            .unwrap_or(0),
        ..StageMetrics::default()
    };
    let meta = std::fs::metadata(src)?;
    let file_size = meta.len() as i64;
    let file_modified_time = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    // 2.1 SHA-256 去重：内容相同的文件直接返回已有 ID
    let hash_start = Instant::now();
    let file_hash = hash::compute_sha256(src)?;
    metrics.hash_ms = hash_start.elapsed().as_millis() as u64;

    if let Some(existing) = repo::get_image_by_hash(pool, &file_hash).await? {
        let id = existing.id;
        return Ok(IndexResult::Duplicated(id));
    }

    let id = Uuid::new_v4().to_string();
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let thumb = library_dir.join("thumbs").join(format!("{id}.jpg"));
    let decode_start = Instant::now();
    let prepared = crate::indexer::index_features::prepare_index_frames(src)?;
    let (width, height) = (Some(prepared.width as i64), Some(prepared.height as i64));
    metrics.decode_ms = decode_start.elapsed().as_millis() as u64;

    // 1. 生成缩略图
    let t_thumb = Instant::now();
    thumbnail::generate_from_image(&prepared.thumbnail_image, &thumb, 150)?;
    metrics.thumb_ms = t_thumb.elapsed().as_millis() as u64;

    // 3. 并行：OCR + CLIP 图像编码
    let frame_count = prepared.sampled_frames.len();
    let (ocr_result, clip_result) = tokio::join!(
        index_features::aggregate_ocr_text_from_frames(prepared.sampled_frames.clone()),
        index_features::aggregate_embedding_from_frames(prepared.sampled_frames),
    );
    let (ocr_text, ocr_ms) = ocr_result?;
    let (embedding, clip_ms) = clip_result?;
    metrics.ocr_ms = ocr_ms;
    metrics.clip_ms = clip_ms;

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
    let db_start = Instant::now();
    let write_result =
        repo::insert_indexed_image(pool, &rec, &embedding, &ocr_text, Some(task_id)).await?;
    metrics.db_ms = db_start.elapsed().as_millis() as u64;

    tracing::info!(
        "[INDEX] {} processed: queue={}ms hash={}ms decode={}ms thumb={}ms ocr={}ms clip={}ms db={}ms ocr_chars={} embed={} sampled_frames={}",
        id,
        metrics.queue_wait_ms,
        metrics.hash_ms,
        metrics.decode_ms,
        metrics.thumb_ms,
        metrics.ocr_ms,
        metrics.clip_ms,
        metrics.db_ms,
        ocr_text.len(),
        embedding.len(),
        frame_count
    );

    match write_result {
        repo::IndexedImageWriteResult::Inserted => Ok(IndexResult::Imported(ImportedImage {
            id,
            embedding,
            metrics,
        })),
        repo::IndexedImageWriteResult::Duplicated(existing_id) => {
            let _ = std::fs::remove_file(&thumb);
            Ok(IndexResult::Duplicated(existing_id))
        }
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn default_index_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get().min(4))
        .unwrap_or(1)
        .max(1)
}

/// 递归扫描目录，收集所有支持格式（jpg/jpeg/png/gif/webp）的图片路径（已排序）。
pub fn scan_images_in_dir(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    scan_recursive(dir, &mut result)?;
    result.sort();
    Ok(result)
}

pub fn scan_images_in_dir_stream<F>(dir: &Path, visit: &mut F) -> anyhow::Result<()>
where
    F: FnMut(String) -> anyhow::Result<()>,
{
    scan_recursive_stream(dir, visit)
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

fn scan_recursive_stream<F>(dir: &Path, visit: &mut F) -> anyhow::Result<()>
where
    F: FnMut(String) -> anyhow::Result<()>,
{
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            scan_recursive_stream(&path, visit)?;
        } else if is_supported_image(&path) {
            visit(path.to_string_lossy().to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn is_supported_image(path: &Path) -> bool {
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
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Rgba, RgbaImage};
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

    fn write_test_gif(path: &Path, colors: &[[u8; 3]]) {
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = GifEncoder::new(file);
        encoder.set_repeat(Repeat::Infinite).unwrap();
        for color in colors {
            let frame = image::Frame::from_parts(
                RgbaImage::from_pixel(2, 2, Rgba([color[0], color[1], color[2], 255])),
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            );
            encoder.encode_frame(frame).unwrap();
        }
    }

    async fn collect(mut rx: mpsc::Receiver<IndexProgress>) -> Vec<IndexProgress> {
        let mut results = vec![];
        while let Ok(Some(p)) = tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
            results.push(p);
        }
        results
    }

    #[derive(Debug)]
    struct PercentileSummary {
        p50: u64,
        p95: u64,
    }

    #[derive(Debug)]
    struct BenchmarkReport {
        total_elapsed_ms: PercentileSummary,
        queue_wait_ms: PercentileSummary,
        hash_ms: PercentileSummary,
        decode_ms: PercentileSummary,
        thumb_ms: PercentileSummary,
        ocr_ms: PercentileSummary,
        clip_ms: PercentileSummary,
        db_ms: PercentileSummary,
        throughput_images_per_sec: f64,
        image_count: usize,
    }

    fn percentile(values: &[u64], numer: usize, denom: usize) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_unstable();
        let idx = ((sorted.len() - 1) * numer) / denom;
        sorted[idx]
    }

    fn summarize(values: &[u64]) -> PercentileSummary {
        PercentileSummary {
            p50: percentile(values, 50, 100),
            p95: percentile(values, 95, 100),
        }
    }

    fn build_benchmark_report(results: &[IndexProgress], wall_clock_ms: u64) -> BenchmarkReport {
        let completed = results
            .iter()
            .filter(|result| result.status == "completed")
            .collect::<Vec<_>>();
        let elapsed = completed
            .iter()
            .map(|item| item.elapsed_ms)
            .collect::<Vec<_>>();
        let queue_wait = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.queue_wait_ms))
            .collect::<Vec<_>>();
        let hash = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.hash_ms))
            .collect::<Vec<_>>();
        let decode = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.decode_ms))
            .collect::<Vec<_>>();
        let thumb = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.thumb_ms))
            .collect::<Vec<_>>();
        let ocr = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.ocr_ms))
            .collect::<Vec<_>>();
        let clip = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.clip_ms))
            .collect::<Vec<_>>();
        let db = completed
            .iter()
            .filter_map(|item| item.metrics.as_ref().map(|metrics| metrics.db_ms))
            .collect::<Vec<_>>();
        let throughput_images_per_sec = if wall_clock_ms == 0 {
            0.0
        } else {
            completed.len() as f64 / (wall_clock_ms as f64 / 1000.0)
        };

        BenchmarkReport {
            total_elapsed_ms: summarize(&elapsed),
            queue_wait_ms: summarize(&queue_wait),
            hash_ms: summarize(&hash),
            decode_ms: summarize(&decode),
            thumb_ms: summarize(&thumb),
            ocr_ms: summarize(&ocr),
            clip_ms: summarize(&clip),
            db_ms: summarize(&db),
            throughput_images_per_sec,
            image_count: completed.len(),
        }
    }

    fn benchmark_fixtures(dir: &Path, count: usize, duplicate_ratio: usize) -> Vec<String> {
        let seeds = [
            fixture("sample.jpg"),
            fixture("sample_blank.jpg"),
            fixture("sample_wide.jpg"),
        ];
        let duplicate_every = duplicate_ratio.max(1);
        let mut paths = Vec::with_capacity(count);

        for idx in 0..count {
            let source = if idx > 0 && idx % duplicate_every == 0 {
                &seeds[0]
            } else {
                &seeds[idx % seeds.len()]
            };
            let dest = dir.join(format!("bench-{idx}.jpg"));
            std::fs::copy(source, &dest).unwrap();
            paths.push(dest.to_string_lossy().to_string());
        }

        paths
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
    async fn test_pipeline_gif_uses_single_record_with_sampled_frames(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let input_dir = tempfile::tempdir().unwrap();
        let gif_path = input_dir.path().join("sample.gif");
        write_test_gif(&gif_path, &[[255, 0, 0], [0, 255, 0], [0, 0, 255]]);

        let engine = make_engine(pool.clone()).await;
        let results = collect(index_images(
            pool.clone(),
            vec![gif_path.to_string_lossy().to_string()],
            lib.path().to_path_buf(),
            engine,
        ))
        .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "completed");
        assert_eq!(results[0].result_kind, "imported");

        let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
        let embeddings = repo::get_all_embeddings(&pool).await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].format, "gif");
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].1.len(), 512);
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
        assert!(results.iter().any(|item| item.status == "error"));
        assert!(results.iter().any(|item| item.status == "completed"));

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
        assert!(results[0].elapsed_ms < 20_000, "should complete in < 20s");
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore = "用于手动性能基线采样"]
    async fn bench_pipeline_small_batch_reports_percentiles(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let inputs = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let paths = benchmark_fixtures(inputs.path(), 10, 4);
        let started_at = Instant::now();

        let results = collect(index_images(pool, paths, lib.path().to_path_buf(), engine)).await;

        let report = build_benchmark_report(&results, started_at.elapsed().as_millis() as u64);
        println!("bench small batch: {report:#?}");
        assert_eq!(report.image_count, 10);
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore = "用于手动性能基线采样"]
    async fn bench_pipeline_mixed_batch_reports_percentiles(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let inputs = tempfile::tempdir().unwrap();
        let engine = make_engine(pool.clone()).await;
        let paths = benchmark_fixtures(inputs.path(), 100, 3);
        let started_at = Instant::now();

        let results = collect(index_images(pool, paths, lib.path().to_path_buf(), engine)).await;

        let report = build_benchmark_report(&results, started_at.elapsed().as_millis() as u64);
        println!("bench mixed batch: {report:#?}");
        assert_eq!(report.image_count, 100);
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
