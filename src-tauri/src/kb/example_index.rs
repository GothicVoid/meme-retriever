use std::path::{Path, PathBuf};

use crate::db::repo::{TagCategory, TagRecord, TagSourceStrategy};
use crate::kb::maintenance::{KnowledgeBaseEntry, KnowledgeBaseFile};
use crate::kb::provider::category_threshold;
use crate::ml::clip::ClipEncoder;
use crate::search::vector_store::VectorStore;

#[derive(Debug, Clone, Default)]
pub struct ExampleImageIndex {
    entries: Vec<ExampleImageEntry>,
}

#[derive(Debug, Clone)]
struct ExampleImageEntry {
    canonical: String,
    category: TagCategory,
    embeddings: Vec<Vec<f32>>,
}

impl ExampleImageIndex {
    pub fn from_knowledge_base(kb: &KnowledgeBaseFile, kb_path: &Path) -> Self {
        let base_dir = kb_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut entries = Vec::new();

        for entry in &kb.entries {
            let Some(indexed) = ExampleImageEntry::from_kb_entry(entry, &base_dir) else {
                continue;
            };
            entries.push(indexed);
        }

        Self { entries }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn match_image(&self, image_path: &str) -> Vec<TagRecord> {
        if self.entries.is_empty() {
            return vec![];
        }

        let image_embedding = match ClipEncoder::encode_image(image_path) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!("[KB] 示例图匹配编码失败 {}: {}", image_path, error);
                return vec![];
            }
        };

        let mut candidates = self
            .entries
            .iter()
            .filter_map(|entry| {
                let score = entry.best_score(&image_embedding)?;
                let threshold = example_threshold(&entry.category);
                if score < threshold {
                    return None;
                }
                Some(TagRecord {
                    tag_text: entry.canonical.clone(),
                    category: entry.category.clone(),
                    is_auto: true,
                    source_strategy: TagSourceStrategy::ExampleImage,
                    confidence: score,
                })
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.tag_text.len().cmp(&a.tag_text.len()))
        });
        candidates
    }

    pub fn query_role_candidates(
        &self,
        canonical: &str,
        vector_store: &VectorStore,
        top_k: usize,
    ) -> Vec<(String, f32)> {
        let Some(entry) = self.entries.iter().find(|item| item.canonical == canonical) else {
            return vec![];
        };

        let mut merged = std::collections::HashMap::<String, f32>::new();
        for embedding in &entry.embeddings {
            for (id, score) in vector_store.query(embedding, top_k * 2) {
                let normalized_score = ((score + 1.0) / 2.0).clamp(0.0, 1.0);
                let threshold = example_threshold(&entry.category);
                if normalized_score < threshold {
                    continue;
                }
                merged
                    .entry(id)
                    .and_modify(|current| {
                        if normalized_score > *current {
                            *current = normalized_score;
                        }
                    })
                    .or_insert(normalized_score);
            }
        }

        let mut results = merged.into_iter().collect::<Vec<_>>();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }
}

impl ExampleImageEntry {
    fn from_kb_entry(entry: &KnowledgeBaseEntry, base_dir: &Path) -> Option<Self> {
        if entry.example_images.is_empty() {
            return None;
        }

        let category = TagCategory::from(entry.category.as_str());
        if matches!(category, TagCategory::Custom) {
            return None;
        }

        let embeddings = entry
            .example_images
            .iter()
            .filter_map(|value| {
                let path = resolve_example_path(base_dir, value);
                if !path.exists() {
                    tracing::warn!(
                        "[KB] 示例图不存在，已跳过 {} -> {}",
                        entry.canonical,
                        path.display()
                    );
                    return None;
                }
                match ClipEncoder::encode_image(path.to_string_lossy().as_ref()) {
                    Ok(embedding) => Some(embedding),
                    Err(error) => {
                        tracing::warn!(
                            "[KB] 示例图编码失败，已跳过 {} -> {}: {}",
                            entry.canonical,
                            path.display(),
                            error
                        );
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        if embeddings.is_empty() {
            return None;
        }

        Some(Self {
            canonical: entry.canonical.clone(),
            category,
            embeddings,
        })
    }

    fn best_score(&self, image_embedding: &[f32]) -> Option<f32> {
        self.embeddings
            .iter()
            .map(|embedding| cosine_similarity(embedding, image_embedding))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|cosine| ((cosine + 1.0) / 2.0).clamp(0.0, 1.0))
    }
}

fn resolve_example_path(base_dir: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut dot = 0.0f32;
    for idx in 0..len {
        dot += a[idx] * b[idx];
    }
    dot
}

fn example_threshold(category: &TagCategory) -> f32 {
    match category {
        TagCategory::Person => 0.9,
        TagCategory::Source => 0.88,
        TagCategory::Meme => category_threshold(category).max(0.92),
        TagCategory::Custom => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_match_image_returns_example_candidate() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![KnowledgeBaseEntry {
                canonical: "测试人物".into(),
                category: "person".into(),
                aliases: vec![],
                match_terms: vec![],
                description: String::new(),
                match_mode: "contains".into(),
                priority: 1,
                example_images: vec![fixture("sample.jpg")],
            }],
        };
        let index = ExampleImageIndex::from_knowledge_base(&kb, Path::new("."));
        let tags = index.match_image(&fixture("sample.jpg"));

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_text, "测试人物");
        assert_eq!(tags[0].source_strategy, TagSourceStrategy::ExampleImage);
        assert!(tags[0].confidence >= 0.99);
    }

    #[test]
    fn test_query_role_candidates_returns_matching_gallery_images() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![KnowledgeBaseEntry {
                canonical: "测试人物".into(),
                category: "person".into(),
                aliases: vec![],
                match_terms: vec![],
                description: String::new(),
                match_mode: "contains".into(),
                priority: 1,
                example_images: vec![fixture("sample.jpg")],
            }],
        };
        let index = ExampleImageIndex::from_knowledge_base(&kb, Path::new("."));
        let mut store = VectorStore::new();
        let sample_embedding = ClipEncoder::encode_image(&fixture("sample.jpg")).unwrap();
        store.insert("same".into(), sample_embedding.clone());
        store.insert(
            "different".into(),
            sample_embedding.iter().map(|value| -*value).collect(),
        );

        let results = index.query_role_candidates("测试人物", &store, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "same");
        assert!(results[0].1 >= 0.99);
    }
}
