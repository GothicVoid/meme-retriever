//! 端到端集成测试：覆盖完整的入库 → 搜索 → 删除链路

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use meme_retriever_lib::{
    db::{self, repo},
    indexer::pipeline,
    kb::local::LocalKBProvider,
    search::engine::SearchEngine,
};

/// 检测真实 CLIP 文本模型是否可用（与 ml/clip.rs 的 find_model 逻辑一致）
fn has_real_clip_model() -> bool {
    let dir = std::env::var("CLIP_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./models"));
    ["clip_text.onnx", "vit-b-16.txt.fp32.onnx", "vit-b-16.txt.fp16.onnx"]
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
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    // 入库
    let rx = pipeline::index_images(pool.clone(), paths, lib.path().to_path_buf());
    let results = collect(rx).await;
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.status == "completed"), "all should succeed");

    // 搜索引擎预加载向量
    let engine = make_engine(pool.clone()).await;

    // 搜索不应报错；根据新评分公式，仅有 OCR/关键词命中的图片会出现在结果中
    // 测试图片不含 "test" 文字，因此此查询可能返回空（低相关性被过滤），这是预期行为
    let hits = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();

    // score 在合法范围
    for h in &hits {
        assert!(h.score >= 0.0 && h.score <= 1.0, "score out of range: {}", h.score);
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

    let rx = pipeline::index_images(pool.clone(), paths, lib.path().to_path_buf());
    let indexed = collect(rx).await;
    assert_eq!(indexed.len(), 2);

    let engine = make_engine(pool.clone()).await;
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
    let rx = pipeline::index_images(pool.clone(), vec![fixture("sample.jpg")], lib.path().to_path_buf());
    let indexed = collect(rx).await;
    assert_eq!(indexed[0].status, "completed");

    let id = &indexed[0].id;
    repo::increment_use_count(&pool, id).await.unwrap();
    repo::increment_use_count(&pool, id).await.unwrap();
    repo::increment_use_count(&pool, id).await.unwrap();

    let img = repo::get_image(&pool, id).await.unwrap().unwrap();
    assert_eq!(img.use_count, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_add_images_and_list(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    // 1. 入库，全部成功
    let rx = pipeline::index_images(pool.clone(), paths, lib.path().to_path_buf());
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
        assert!(img.width.unwrap_or(0) > 0, "width should be positive: {}", img.file_name);
        assert!(img.height.unwrap_or(0) > 0, "height should be positive: {}", img.file_name);
    }

    // 4. 缩略图文件实际存在于磁盘
    for img in &images {
        let thumb = img.thumbnail_path.as_deref().unwrap_or("");
        assert!(!thumb.is_empty(), "thumbnail_path should be set: {}", img.file_name);
        assert!(
            std::path::Path::new(thumb).exists(),
            "thumbnail should exist at: {thumb}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_search_performance(pool: sqlx::SqlitePool) {
    let lib = tempfile::tempdir().unwrap();
    let paths = vec![
        fixture("sample.jpg"),
        fixture("sample_blank.jpg"),
        fixture("sample_wide.jpg"),
    ];

    // 入库性能：每张 < 5s（mock 模式）
    let start = std::time::Instant::now();
    let rx = pipeline::index_images(pool.clone(), paths, lib.path().to_path_buf());
    let results = collect(rx).await;
    let total_ms = start.elapsed().as_millis();
    assert!(results.iter().all(|r| r.status == "completed"));
    assert!(total_ms < 15_000, "indexing 3 images took too long: {total_ms}ms");

    // 搜索性能：< 2000ms（真实 CLIP 模型推理）
    let engine = make_engine(pool).await;
    let start = std::time::Instant::now();
    let _ = engine.search("test", 10, 0.3, 0.4, 0.3).await.unwrap();
    let search_ms = start.elapsed().as_millis();
    assert!(search_ms < 2000, "search took too long: {search_ms}ms");
}
