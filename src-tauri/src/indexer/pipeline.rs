use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::db::{DbPool, repo};
use crate::indexer::{thumbnail, ocr};
use crate::ml::clip::ClipEncoder;

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexProgress {
    pub id: String,
    pub file_name: String,
    pub status: String,   // "completed" | "error"
    pub message: Option<String>,
    pub elapsed_ms: u64,
}

/// 入库流水线。返回进度接收端，调用方可监听每张图的处理结果。
/// 整体在后台 task 中运行，不阻塞调用方。
pub fn index_images(
    pool: DbPool,
    paths: Vec<String>,
    library_dir: PathBuf,
) -> mpsc::Receiver<IndexProgress> {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        for path in paths {
            let progress = process_one(&pool, &path, &library_dir).await;
            if tx.send(progress).await.is_err() {
                break; // 接收端已关闭
            }
        }
    });
    rx
}

async fn process_one(pool: &DbPool, src_path: &str, library_dir: &Path) -> IndexProgress {
    let start = Instant::now();
    let src = Path::new(src_path);
    let file_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| src_path.to_string());

    match do_index(pool, src, library_dir).await {
        Ok(id) => IndexProgress {
            id,
            file_name,
            status: "completed".into(),
            message: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        Err(e) => {
            tracing::warn!("pipeline: failed {src_path}: {e}");
            IndexProgress {
                id: String::new(),
                file_name,
                status: "error".into(),
                message: Some(e.to_string()),
                elapsed_ms: start.elapsed().as_millis() as u64,
            }
        }
    }
}

async fn do_index(pool: &DbPool, src: &Path, library_dir: &Path) -> anyhow::Result<String> {
    if !src.exists() {
        anyhow::bail!("file not found: {:?}", src);
    }

    let id = Uuid::new_v4().to_string();
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let dest = library_dir.join(format!("{id}.{ext}"));
    let thumb = library_dir.join("thumbs").join(format!("{id}.jpg"));

    // 1. 复制文件
    tokio::fs::create_dir_all(library_dir).await?;
    tokio::fs::copy(src, &dest).await?;

    // 2. 生成缩略图
    let t_thumb = Instant::now();
    thumbnail::generate(src, &thumb, 256)?;
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
        "pipeline: id={id} thumb={}ms ocr_len={} embed_dims={}",
        thumb_ms, ocr_text.len(), embedding.len()
    );

    // 4. 读取图片尺寸
    let (width, height) = image_dimensions(src);

    // 5. 写入数据库
    let rec = repo::ImageRecord {
        id: id.clone(),
        file_path: dest.to_string_lossy().to_string(),
        file_name: src.file_name().unwrap_or_default().to_string_lossy().to_string(),
        format: ext.to_string(),
        width,
        height,
        added_at: now_secs(),
        use_count: 0,
        thumbnail_path: Some(thumb.to_string_lossy().to_string()),
    };
    repo::insert_image(pool, &rec).await?;
    repo::insert_embedding(pool, &id, &embedding).await?;
    if !ocr_text.is_empty() {
        repo::insert_ocr(pool, &id, &ocr_text).await?;
    }

    Ok(id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
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
        while let Ok(Some(p)) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            results.push(p);
        }
        results
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_pipeline_single_image(pool: SqlitePool) {
        let lib = tempfile::tempdir().unwrap();
        let mut rx = index_images(pool.clone(), vec![fixture("sample.jpg")], lib.path().to_path_buf());
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
        let paths = vec![
            fixture("sample.jpg"),
            fixture("sample_blank.jpg"),
            fixture("sample_wide.jpg"),
        ];
        let rx = index_images(pool.clone(), paths, lib.path().to_path_buf());
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
        let paths = vec![
            "/nonexistent/image.jpg".to_string(),
            fixture("sample.jpg"),
        ];
        let rx = index_images(pool.clone(), paths, lib.path().to_path_buf());
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
        let rx = index_images(pool.clone(), vec![fixture("sample.jpg")], lib.path().to_path_buf());
        let results = collect(rx).await;
        assert!(results[0].elapsed_ms < 10_000, "should complete in < 10s");
    }
}
