//! 端到端集成测试：覆盖完整的入库 → 搜索 → 删除链路

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use meme_retriever_lib::{
    db::{repo, task_repo},
    indexer::pipeline,
    kb::local::LocalKBProvider,
    search::engine::SearchEngine,
};
use sqlx::Row;

/// 检测真实 CLIP 文本模型是否可用（与 ml/clip.rs 的 find_model 逻辑一致）
fn has_real_clip_model() -> bool {
    let dir = std::env::var("CLIP_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./models"));
    [
        "clip_text.onnx",
        "vit-b-16.txt.fp32.onnx",
        "vit-b-16.txt.fp16.onnx",
    ]
    .iter()
    .any(|name| dir.join(name).exists())
}
use tokio::sync::mpsc;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_string_lossy()
        .to_string()
}

async fn collect(mut rx: mpsc::Receiver<pipeline::IndexProgress>) -> Vec<pipeline::IndexProgress> {
    let mut results = vec![];
    while let Ok(Some(p)) = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await {
        results.push(p);
    }
    results
}

async fn make_engine(pool: sqlx::SqlitePool) -> Arc<SearchEngine> {
    let kb = Box::new(LocalKBProvider::empty());
    Arc::new(SearchEngine::new(pool, kb).await.unwrap())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_full_index_and_search(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    // 入库
    let rx = pipeline::index_images(
        pool.clone(),
        paths,
        lib.path().to_path_buf(),
        engine.clone(),
    );
    let results = collect(rx).await;
    assert_eq!(results.len(), 3);
    assert!(
        results.iter().all(|r| r.status == "completed"),
        "all should succeed"
    );

    // 搜索引擎预加载向量
    // 搜索不应报错；根据新评分公式，仅有 OCR/关键词命中的图片会出现在结果中
    // 测试图片不含 "test" 文字，因此此查询可能返回空（低相关性被过滤），这是预期行为
    let hits = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();

    // score 在合法范围
    for h in &hits {
        assert!(
            h.score >= 0.0 && h.score <= 1.0,
            "score out of range: {}",
            h.score
        );
    }

    // 无论有无结果，搜索调用本身成功即可
    if !has_real_clip_model() {
        eprintln!("注意：未找到真实 CLIP 模型，embedding 由 mock 实现生成");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_index_then_delete_then_search(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let paths = vec![fixture("sample.jpg"), fixture("sample_blank.jpg")];
    let engine = make_engine(pool.clone()).await;

    let rx = pipeline::index_images(
        pool.clone(),
        paths,
        lib.path().to_path_buf(),
        engine.clone(),
    );
    let indexed = collect(rx).await;
    assert_eq!(indexed.len(), 2);
    assert_eq!(engine.vector_store_len(), 2);

    // 删除第一张
    let id_to_delete = &indexed[0].id;
    repo::delete_image(&pool, id_to_delete).await.unwrap();
    engine.remove_vector(id_to_delete);

    // 向量索引应减少
    assert_eq!(engine.vector_store_len(), 1);

    // DB 中只剩 1 张
    let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
    assert_eq!(images.len(), 1);
    assert_ne!(images[0].id, *id_to_delete);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_use_count_increment(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;
    let rx = pipeline::index_images(
        pool.clone(),
        vec![fixture("sample.jpg")],
        lib.path().to_path_buf(),
        engine,
    );
    let indexed = collect(rx).await;
    assert_eq!(indexed[0].status, "completed");

    let id = &indexed[0].id;
    repo::increment_use_count(&pool, id, 1).await.unwrap();
    repo::increment_use_count(&pool, id, 2).await.unwrap();
    repo::increment_use_count(&pool, id, 3).await.unwrap();

    let img = repo::get_image(&pool, id).await.unwrap().unwrap();
    assert_eq!(img.use_count, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_add_images_and_list(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    // 1. 入库，全部成功
    let rx = pipeline::index_images(pool.clone(), paths, lib.path().to_path_buf(), engine);
    let results = collect(rx).await;
    assert_eq!(results.len(), 3);
    assert!(
        results.iter().all(|r| r.status == "completed"),
        "all images should index successfully"
    );

    // 2. 图库列表应包含 3 张
    let images = repo::get_images_paged(&pool, 0, 50).await.unwrap();
    assert_eq!(images.len(), 3, "library should contain 3 images after add");

    // 3. 每张图片字段完整
    for img in &images {
        assert!(!img.id.is_empty(), "id should not be empty");
        assert!(!img.file_name.is_empty(), "file_name should not be empty");
        assert!(
            img.width.unwrap_or(0) > 0,
            "width should be positive: {}",
            img.file_name
        );
        assert!(
            img.height.unwrap_or(0) > 0,
            "height should be positive: {}",
            img.file_name
        );
    }

    // 4. 缩略图文件实际存在于磁盘
    for img in &images {
        let thumb = img.thumbnail_path.as_deref().unwrap_or("");
        assert!(
            !thumb.is_empty(),
            "thumbnail_path should be set: {}",
            img.file_name
        );
        assert!(
            std::path::Path::new(thumb).exists(),
            "thumbnail should exist at: {thumb}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_search_performance(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    // 入库性能：每张 < 5s（mock 模式）
    let start = std::time::Instant::now();
    let rx = pipeline::index_images(
        pool.clone(),
        paths,
        lib.path().to_path_buf(),
        engine.clone(),
    );
    let results = collect(rx).await;
    let total_ms = start.elapsed().as_millis();
    assert!(results.iter().all(|r| r.status == "completed"));
    assert!(
        total_ms < 15_000,
        "indexing 3 images took too long: {total_ms}ms"
    );

    // 搜索性能：< 2000ms（真实 CLIP 模型推理）
    let start = std::time::Instant::now();
    let _ = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
    let search_ms = start.elapsed().as_millis();
    assert!(search_ms < 2000, "search took too long: {search_ms}ms");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_clear_all_images_integration(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    let rx = pipeline::index_images(
        pool.clone(),
        paths,
        lib.path().to_path_buf(),
        engine.clone(),
    );
    let indexed = collect(rx).await;
    assert_eq!(indexed.len(), 3);
    assert_eq!(engine.vector_store_len(), 3);

    let deleted = repo::clear_all_images(&pool).await.unwrap();
    assert_eq!(deleted, 3);
    engine.clear_all_vectors();

    assert!(repo::get_all_images(&pool).await.unwrap().is_empty());
    assert_eq!(engine.vector_store_len(), 0);

    let hits = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
    assert!(hits.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_import_batch_summary_with_imported_duplicated_and_failed_results(
    pool: sqlx::SqlitePool,
) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;

    let seed_rx = pipeline::index_images_with_batch(
        pool.clone(),
        vec![fixture("sample.jpg")],
        lib.path().to_path_buf(),
        engine.clone(),
        Some("batch-seed".into()),
    );
    let seed_results = collect(seed_rx).await;
    assert_eq!(seed_results.len(), 1);
    assert_eq!(seed_results[0].result_kind, "imported");

    let mixed_rx = pipeline::index_images_with_batch(
        pool.clone(),
        vec![
            fixture("sample.jpg"),
            fixture("sample_blank.jpg"),
            "/nonexistent/image.jpg".into(),
        ],
        lib.path().to_path_buf(),
        engine,
        Some("batch-mixed".into()),
    );
    let mixed_results = collect(mixed_rx).await;

    assert_eq!(mixed_results.len(), 3);
    assert!(mixed_results.iter().any(|item| item.result_kind == "duplicated"));
    assert!(mixed_results.iter().any(|item| item.result_kind == "imported"));
    assert!(mixed_results.iter().any(|item| item.result_kind == "failed"));

    let summary = task_repo::get_latest_import_batch_summary(&pool)
        .await
        .unwrap()
        .expect("should have latest import batch");
    assert_eq!(summary.batch_id, "batch-mixed");
    assert_eq!(summary.imported_count, 1);
    assert_eq!(summary.duplicated_count, 1);
    assert_eq!(summary.failed_count, 1);

    let failures = task_repo::get_import_batch_failures(&pool, "batch-mixed")
        .await
        .unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].file_path, "/nonexistent/image.jpg");
    assert!(
        failures[0]
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("file not found"))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_resume_unfinished_import_after_app_restart(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;

    task_repo::insert_task_with_batch(&pool, "resume-task-1", &fixture("sample.jpg"), "batch-r")
        .await
        .unwrap();
    task_repo::insert_task_with_batch(
        &pool,
        "resume-task-2",
        &fixture("sample_blank.jpg"),
        "batch-r",
    )
    .await
    .unwrap();
    task_repo::update_task_status(&pool, "resume-task-1", "processing", None)
        .await
        .unwrap();

    task_repo::reset_stale_tasks(&pool).await.unwrap();
    let pending_before_resume = task_repo::get_pending_tasks(&pool).await.unwrap();
    assert_eq!(pending_before_resume.len(), 2);
    assert!(pending_before_resume.iter().all(|task| task.status == "pending"));

    let resume_tasks = pending_before_resume
        .into_iter()
        .map(|task| pipeline::ResumeIndexTask {
            id: task.id,
            file_path: task.file_path,
        })
        .collect();

    let rx = pipeline::resume_index_images(
        pool.clone(),
        resume_tasks,
        lib.path().to_path_buf(),
        engine,
    );
    let resumed = collect(rx).await;

    assert_eq!(resumed.len(), 2);
    assert!(resumed.iter().all(|item| item.status == "completed"));
    assert!(resumed.iter().all(|item| item.result_kind == "imported"));

    let pending_after_resume = task_repo::get_pending_tasks(&pool).await.unwrap();
    assert!(pending_after_resume.is_empty());

    let task_rows = sqlx::query("SELECT id, status FROM task_queue ORDER BY id ASC")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(task_rows.len(), 2);
    assert_eq!(task_rows[0].get::<String, _>("id"), "resume-task-1");
    assert_eq!(task_rows[0].get::<String, _>("status"), "completed");
    assert_eq!(task_rows[1].get::<String, _>("id"), "resume-task-2");
    assert_eq!(task_rows[1].get::<String, _>("status"), "completed");

    let images = repo::get_images_paged(&pool, 0, 10).await.unwrap();
    assert_eq!(images.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_interrupted_import_keeps_all_remaining_tasks_in_queue(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let engine = make_engine(pool.clone()).await;
    let tasks = pipeline::create_index_tasks(
        &pool,
        vec![
            fixture("sample.jpg"),
            fixture("sample_blank.jpg"),
            fixture("sample_wide.jpg"),
        ],
        Some("batch-interrupted"),
    )
    .await
    .unwrap();

    assert_eq!(task_repo::get_pending_task_count(&pool).await.unwrap(), 3);

    let first_task = vec![tasks[0].clone()];
    let rx = pipeline::resume_index_images(pool.clone(), first_task, lib.path().to_path_buf(), engine);
    let resumed = collect(rx).await;

    assert_eq!(resumed.len(), 1);
    assert_eq!(resumed[0].status, "completed");

    let pending = task_repo::get_pending_tasks(&pool).await.unwrap();
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|task| task.status == "pending"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_clear_all_images_cleans_ocr_fts(pool: sqlx::SqlitePool) {
    repo::insert_image(
        &pool,
        &repo::ImageRecord {
            id: "ocr-img".into(),
            file_path: "/tmp/ocr-img.png".into(),
            file_name: "ocr-img.png".into(),
            format: "png".into(),
            width: Some(100),
            height: Some(100),
            added_at: 1000,
            use_count: 0,
            thumbnail_path: None,
            file_hash: None,
            file_size: None,
            file_modified_time: None,
            file_status: "normal".into(),
            last_check_time: None,
            last_used_at: None,
        },
    )
    .await
    .unwrap();
    repo::insert_ocr(&pool, "ocr-img", "这是一段 OCR 文本")
        .await
        .unwrap();

    let before: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM ocr_fts")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("cnt");
    assert_eq!(before, 1);

    repo::clear_all_images(&pool).await.unwrap();

    let after: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM ocr_fts")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("cnt");
    assert_eq!(after, 0);
}
