use image::DynamicImage;
use image::RgbImage;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

use crate::ml::clip::ClipEncoder;

static CLIP_PERMITS: Lazy<Arc<Semaphore>> =
    Lazy::new(|| Arc::new(Semaphore::new(default_ml_concurrency())));
static OCR_PERMITS: Lazy<Arc<Semaphore>> =
    Lazy::new(|| Arc::new(Semaphore::new(default_ml_concurrency())));

pub struct PreparedIndexFrames {
    pub thumbnail_image: DynamicImage,
    pub sampled_frames: Vec<RgbImage>,
    pub width: u32,
    pub height: u32,
    pub used_multi_frame_sampling: bool,
}

pub fn prepare_index_frames(path: &Path) -> anyhow::Result<PreparedIndexFrames> {
    let frame_set = crate::image_io::load_index_frame_set(path)?;
    Ok(PreparedIndexFrames {
        thumbnail_image: frame_set.thumbnail_image,
        sampled_frames: frame_set.sampled_frames,
        width: frame_set.width,
        height: frame_set.height,
        used_multi_frame_sampling: frame_set.used_multi_frame_sampling,
    })
}

pub async fn aggregate_ocr_text_from_frames(
    frames: Vec<RgbImage>,
) -> anyhow::Result<(String, u64)> {
    let _permit = Arc::clone(&OCR_PERMITS).acquire_owned().await?;
    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let texts = frames
            .iter()
            .map(crate::indexer::ocr::extract_text_from_rgb_image)
            .collect::<anyhow::Result<Vec<_>>>()?;
        let aggregated = aggregate_ocr_texts(&texts);
        Ok::<_, anyhow::Error>((aggregated, start.elapsed().as_millis() as u64))
    })
    .await?
}

pub async fn aggregate_embedding_from_frames(
    frames: Vec<RgbImage>,
) -> anyhow::Result<(Vec<f32>, u64)> {
    let _permit = Arc::clone(&CLIP_PERMITS).acquire_owned().await?;
    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let vectors = frames
            .iter()
            .map(ClipEncoder::encode_rgb_image)
            .collect::<anyhow::Result<Vec<_>>>()?;
        let aggregated = aggregate_embeddings(&vectors)?;
        Ok::<_, anyhow::Error>((aggregated, start.elapsed().as_millis() as u64))
    })
    .await?
}

pub fn aggregate_ocr_texts(texts: &[String]) -> String {
    let mut seen = HashSet::new();
    let mut segments = Vec::new();

    for text in texts {
        for normalized in normalize_ocr_segments(text) {
            if seen.insert(normalized.clone()) {
                segments.push(normalized);
            }
        }
    }

    segments.join(" ")
}

pub fn aggregate_embeddings(vectors: &[Vec<f32>]) -> anyhow::Result<Vec<f32>> {
    anyhow::ensure!(!vectors.is_empty(), "cannot aggregate empty embeddings");
    let dims = vectors[0].len();
    anyhow::ensure!(dims > 0, "embedding dims must be positive");

    let mut aggregated = vec![0.0f32; dims];
    for vector in vectors {
        anyhow::ensure!(
            vector.len() == dims,
            "embedding dims mismatch: expected {dims}, got {}",
            vector.len()
        );
        for (slot, value) in aggregated.iter_mut().zip(vector.iter()) {
            *slot += *value;
        }
    }

    let count = vectors.len() as f32;
    for slot in &mut aggregated {
        *slot /= count;
    }
    l2_normalize(&mut aggregated);

    anyhow::ensure!(
        aggregated.iter().all(|value| value.is_finite()),
        "aggregated embedding contains non-finite values"
    );
    Ok(aggregated)
}

fn normalize_ocr_segments(text: &str) -> Vec<String> {
    text.lines()
        .flat_map(|line| line.split('\u{000c}'))
        .map(|segment| segment.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|segment| segment.trim().to_string())
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn l2_normalize(vector: &mut [f32]) {
    let norm: f32 = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for value in vector {
            *value /= norm;
        }
    }
}

fn default_ml_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get().min(4))
        .unwrap_or(1)
        .clamp(1, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_ocr_texts_deduplicates_and_normalizes_segments() {
        let texts = vec![
            "  第一行 \n第二行  ".to_string(),
            "".to_string(),
            "第二行\n第三 行".to_string(),
        ];

        let aggregated = aggregate_ocr_texts(&texts);

        assert_eq!(aggregated, "第一行 第二行 第三 行");
    }

    #[test]
    fn aggregate_embeddings_averages_and_normalizes() {
        let vectors = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];

        let aggregated = aggregate_embeddings(&vectors).unwrap();
        let norm: f32 = aggregated
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();

        assert_eq!(aggregated.len(), 3);
        assert!((norm - 1.0).abs() < 1e-5);
        assert!(aggregated.iter().all(|value| value.is_finite()));
    }
}
