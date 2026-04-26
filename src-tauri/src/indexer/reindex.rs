use std::path::Path;
use std::sync::Arc;

use crate::db::{repo, DbPool};
use crate::search::engine::SearchEngine;

pub struct RebuiltIndexFeatures {
    pub ocr_text: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GifReindexFailure {
    pub image_id: String,
    pub file_path: String,
    pub error_message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GifReindexSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: Vec<GifReindexFailure>,
}

pub async fn reindex_gif_images<F>(
    pool: &DbPool,
    engine: Arc<SearchEngine>,
    mut on_progress: F,
) -> anyhow::Result<GifReindexSummary>
where
    F: FnMut(usize, usize, &str),
{
    let images = repo::get_all_images(pool).await?;
    let gif_images = images
        .into_iter()
        .filter(|image| image.format.eq_ignore_ascii_case("gif"))
        .collect::<Vec<_>>();
    let total = gif_images.len();
    let mut succeeded = 0usize;
    let mut failed = Vec::new();

    for (index, image) in gif_images.into_iter().enumerate() {
        on_progress(index, total, &image.id);

        match reindex_one_gif_image(pool, Arc::clone(&engine), &image.id, &image.file_path).await {
            Ok(()) => succeeded += 1,
            Err(err) => failed.push(GifReindexFailure {
                image_id: image.id.clone(),
                file_path: image.file_path.clone(),
                error_message: err.to_string(),
            }),
        }
    }

    on_progress(total, total, "");
    Ok(GifReindexSummary {
        total,
        succeeded,
        failed,
    })
}

async fn reindex_one_gif_image(
    pool: &DbPool,
    engine: Arc<SearchEngine>,
    image_id: &str,
    file_path: &str,
) -> anyhow::Result<()> {
    let rebuilt = rebuild_index_features(file_path).await?;

    repo::insert_embedding(pool, image_id, &rebuilt.embedding).await?;
    if rebuilt.ocr_text.is_empty() {
        repo::delete_ocr_for_image(pool, image_id).await?;
    } else {
        repo::insert_ocr(pool, image_id, &rebuilt.ocr_text).await?;
    }
    engine.insert_vector(image_id.to_string(), rebuilt.embedding);

    Ok(())
}

pub async fn rebuild_index_features(file_path: &str) -> anyhow::Result<RebuiltIndexFeatures> {
    let path = Path::new(file_path);
    anyhow::ensure!(path.exists(), "file not found: {:?}", path);

    let prepared = tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || crate::indexer::index_features::prepare_index_frames(&path)
    })
    .await??;

    let (ocr_result, clip_result) = tokio::join!(
        crate::indexer::index_features::aggregate_ocr_text_from_frames(
            prepared.sampled_frames.clone()
        ),
        crate::indexer::index_features::aggregate_embedding_from_frames(prepared.sampled_frames),
    );

    let (ocr_text, _) = ocr_result?;
    let (embedding, _) = clip_result?;
    Ok(RebuiltIndexFeatures {
        ocr_text,
        embedding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Rgba, RgbaImage};
    use sqlx::SqlitePool;

    use crate::db::repo::{self, ImageRecord, TagCategory, TagRecord, TagSourceStrategy};
    use crate::kb::local::LocalKBProvider;

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

    fn gif_record(id: &str, file_path: &str) -> ImageRecord {
        ImageRecord {
            id: id.to_string(),
            file_path: file_path.to_string(),
            file_name: format!("{id}.gif"),
            format: "gif".to_string(),
            width: Some(2),
            height: Some(2),
            added_at: 100,
            use_count: 3,
            thumbnail_path: Some(format!("/tmp/{id}.jpg")),
            file_hash: Some(format!("hash-{id}")),
            file_size: Some(12),
            file_modified_time: Some(34),
            file_status: "normal".to_string(),
            last_check_time: Some(56),
            last_used_at: Some(78),
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_reindex_gif_images_only_updates_embedding_and_ocr(pool: SqlitePool) {
        let path = std::env::temp_dir().join("codex_reindex_gif_images.gif");
        write_test_gif(&path, &[[255, 0, 0], [0, 255, 0], [0, 0, 255]]);

        let record = gif_record("gif-1", &path.to_string_lossy());
        repo::insert_image(&pool, &record).await.unwrap();
        repo::insert_embedding(&pool, &record.id, &[1.0, 0.0, 0.0])
            .await
            .unwrap();
        repo::insert_ocr(&pool, &record.id, "旧文字").await.unwrap();
        repo::insert_tags(
            &pool,
            &record.id,
            &[TagRecord {
                tag_text: "手动标签".to_string(),
                category: TagCategory::Custom,
                is_auto: false,
                source_strategy: TagSourceStrategy::Manual,
                confidence: 1.0,
            }],
        )
        .await
        .unwrap();

        let engine = Arc::new(
            SearchEngine::new(pool.clone(), Box::new(LocalKBProvider::empty()))
                .await
                .unwrap(),
        );

        let summary = reindex_gif_images(&pool, Arc::clone(&engine), |_current, _total, _id| {})
            .await
            .unwrap();

        let updated_record = repo::get_image(&pool, &record.id).await.unwrap().unwrap();
        let updated_embedding = repo::get_embedding(&pool, &record.id)
            .await
            .unwrap()
            .unwrap();
        let updated_tags = repo::get_tags_for_image(&pool, &record.id).await.unwrap();
        let updated_ocr = repo::get_ocr_texts(&pool, &[record.id.as_str()])
            .await
            .unwrap();

        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);
        assert!(summary.failed.is_empty());
        assert_eq!(updated_record.file_path, record.file_path);
        assert_eq!(updated_record.thumbnail_path, record.thumbnail_path);
        assert_eq!(updated_record.use_count, record.use_count);
        assert_eq!(updated_tags.len(), 1);
        assert_eq!(updated_tags[0].tag_text, "手动标签");
        assert_eq!(updated_embedding.len(), 512);
        assert_ne!(updated_embedding, vec![1.0, 0.0, 0.0]);
        assert!(updated_ocr.get(&record.id).map(|value| value.as_str()) != Some("旧文字"));

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn test_rebuild_index_features_supports_gif_multi_frame_path() {
        let path = std::env::temp_dir().join("codex_rebuild_index_features.gif");
        write_test_gif(&path, &[[10, 0, 0], [0, 20, 0], [0, 0, 30]]);

        let rebuilt = rebuild_index_features(&path.to_string_lossy())
            .await
            .unwrap();

        assert_eq!(rebuilt.embedding.len(), 512);
        assert!(rebuilt.embedding.iter().all(|value| value.is_finite()));

        let _ = std::fs::remove_file(path);
    }
}
