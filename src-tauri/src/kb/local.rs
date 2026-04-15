use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::provider::{category_threshold, KnowledgeBaseProvider, QueryNormalization};
use crate::db::repo::{TagCategory, TagRecord, TagSourceStrategy};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
struct KbEntry {
    canonical: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default, alias = "type")]
    r#type: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    match_terms: Vec<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_match_mode")]
    match_mode: String,
    #[serde(default = "default_priority")]
    priority: f32,
    #[serde(default)]
    example_images: Vec<String>,
}

fn default_match_mode() -> String {
    "contains".to_string()
}

fn default_priority() -> f32 {
    1.0
}

#[derive(Default)]
pub struct LocalKBProvider {
    entries: Vec<KbEntry>,
    alias_to_canonical: HashMap<String, String>,
}

impl LocalKBProvider {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content =
            std::fs::read_to_string(path).unwrap_or_else(|_| r#"{"entries":[]}"#.to_string());
        let value: serde_json::Value = serde_json::from_str(&content)?;
        let entries: Vec<KbEntry> =
            serde_json::from_value(value["entries"].clone()).unwrap_or_default();
        Ok(Self::from_entries(entries))
    }

    pub fn empty() -> Self {
        Self::default()
    }

    fn from_entries(entries: Vec<KbEntry>) -> Self {
        let mut alias_to_canonical = HashMap::new();
        for entry in &entries {
            alias_to_canonical.insert(normalize(&entry.canonical), entry.canonical.clone());
            for alias in &entry.aliases {
                alias_to_canonical.insert(normalize(alias), entry.canonical.clone());
            }
        }
        Self {
            entries,
            alias_to_canonical,
        }
    }

    fn entry_category(entry: &KbEntry) -> TagCategory {
        TagCategory::from(
            entry
                .category
                .as_deref()
                .or(entry.r#type.as_deref())
                .unwrap_or("custom"),
        )
    }

    fn all_terms<'a>(entry: &'a KbEntry) -> impl Iterator<Item = &'a str> + 'a {
        std::iter::once(entry.canonical.as_str())
            .chain(entry.aliases.iter().map(String::as_str))
            .chain(entry.match_terms.iter().map(String::as_str))
    }

    fn match_candidates(&self, ocr_text: &str, file_name: &str) -> Vec<TagRecord> {
        let normalized_ocr = normalize(ocr_text);
        let normalized_file_name = normalize_file_name(file_name);
        let mut candidates = Vec::new();

        for entry in &self.entries {
            let mut score = 0.0f32;
            let mut matched_sources = HashSet::new();
            let mut strongest = 0.0f32;

            let canonical = normalize(&entry.canonical);
            if term_matches(&normalized_ocr, &canonical, &entry.match_mode) {
                strongest = strongest.max(if normalized_ocr == canonical { 1.0 } else { 0.8 });
                matched_sources.insert("ocr");
            }

            for alias in &entry.aliases {
                let normalized_alias = normalize(alias);
                if exact_or_contains(&normalized_ocr, &normalized_alias, &entry.match_mode) {
                    let base = if normalized_ocr == normalized_alias {
                        0.9
                    } else {
                        0.8
                    };
                    strongest = strongest.max(base);
                    matched_sources.insert("ocr");
                }
            }

            let mut matched_term_count = 0;
            for term in &entry.match_terms {
                let normalized_term = normalize(term);
                if allow_match_term(&normalized_term)
                    && term_matches(&normalized_ocr, &normalized_term, &entry.match_mode)
                {
                    strongest = strongest.max(0.6);
                    matched_term_count += 1;
                    matched_sources.insert("ocr");
                }
            }

            for name_term in Self::all_terms(entry) {
                let normalized_name_term = normalize(name_term);
                if allow_file_name_match(&normalized_name_term)
                    && normalized_file_name.contains(&normalized_name_term)
                {
                    strongest = strongest.max(0.3);
                    matched_sources.insert("file_name");
                    break;
                }
            }

            if strongest == 0.0 {
                continue;
            }

            score += strongest;
            if matched_sources.len() > 1 {
                score += 0.1;
            }
            if matched_term_count > 1 {
                score += 0.05 * (matched_term_count as f32 - 1.0);
            }
            score = (score * entry.priority).min(1.0);

            let category = Self::entry_category(entry);
            let threshold = category_threshold(&category);
            let file_name_only =
                matched_sources.len() == 1 && matched_sources.contains("file_name");
            if score < threshold || file_name_only {
                continue;
            }

            candidates.push(TagRecord {
                tag_text: entry.canonical.clone(),
                category,
                is_auto: true,
                source_strategy: match (
                    matched_sources.contains("ocr"),
                    matched_sources.contains("file_name"),
                ) {
                    (true, true) => TagSourceStrategy::OcrFileName,
                    (true, false) => TagSourceStrategy::Ocr,
                    (false, true) => TagSourceStrategy::FileName,
                    (false, false) => TagSourceStrategy::Fallback,
                },
                confidence: score.min(1.0),
            });
        }

        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.tag_text.len().cmp(&a.tag_text.len()))
        });

        let mut kept = Vec::new();
        let mut per_category: HashMap<&'static str, usize> = HashMap::new();
        let mut seen = HashSet::new();
        for candidate in candidates {
            let key = candidate.tag_text.to_lowercase();
            if seen.contains(&key) {
                continue;
            }
            let category_key = candidate.category.as_str();
            if *per_category.get(category_key).unwrap_or(&0) >= 2 || kept.len() >= 5 {
                continue;
            }
            seen.insert(key);
            *per_category.entry(category_key).or_insert(0) += 1;
            kept.push(candidate);
        }
        kept
    }
}

impl KnowledgeBaseProvider for LocalKBProvider {
    fn expand_query(&self, query: &str) -> String {
        let normalized = normalize(query);
        for entry in &self.entries {
            if normalize(&entry.canonical) == normalized
                || entry
                    .aliases
                    .iter()
                    .any(|alias| normalize(alias) == normalized)
            {
                let extra_terms = if entry.tags.is_empty() {
                    entry.match_terms.join(" ")
                } else {
                    entry.tags.join(" ")
                };
                return format!("{} {} {}", entry.canonical, entry.description, extra_terms)
                    .trim()
                    .to_string();
            }
        }
        query.to_string()
    }

    fn normalize_query(&self, query: &str) -> QueryNormalization {
        let normalized = normalize(query);
        let tag_query = self
            .alias_to_canonical
            .get(&normalized)
            .cloned()
            .unwrap_or_else(|| query.trim().to_string());
        QueryNormalization {
            tag_query: tag_query.clone(),
            expanded_query: self.expand_query(&tag_query),
        }
    }

    fn related_terms(&self, query: &str) -> Vec<String> {
        let normalized = normalize(query);
        for entry in &self.entries {
            if normalize(&entry.canonical) == normalized
                || entry
                    .aliases
                    .iter()
                    .any(|alias| normalize(alias) == normalized)
            {
                return entry
                    .aliases
                    .iter()
                    .cloned()
                    .chain(std::iter::once(entry.canonical.clone()))
                    .chain(entry.match_terms.iter().cloned())
                    .collect();
            }
        }
        Vec::new()
    }

    fn auto_tag(&self, ocr_text: &str, file_name: &str) -> Vec<TagRecord> {
        self.match_candidates(ocr_text, file_name)
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '，' | ',' | '。' | '.' | '！' | '!' | '？' | '?' | '、' | '_' | '-' => ' ',
            _ => ch,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn normalize_file_name(file_name: &str) -> String {
    let stem = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);
    normalize(stem)
}

fn allow_match_term(term: &str) -> bool {
    let len = term.chars().count();
    len > 2 || (len == 2 && !term.contains(' '))
}

fn allow_file_name_match(term: &str) -> bool {
    term.chars().count() >= 3
}

fn exact_or_contains(haystack: &str, needle: &str, mode: &str) -> bool {
    match mode {
        "exact" => haystack == needle,
        "exact_or_contains" => haystack == needle || haystack.contains(needle),
        _ => haystack.contains(needle),
    }
}

fn term_matches(haystack: &str, needle: &str, mode: &str) -> bool {
    if needle.chars().count() <= 1 {
        return false;
    }
    if needle.chars().count() == 2 && mode != "exact" && mode != "exact_or_contains" {
        return haystack == needle;
    }
    exact_or_contains(haystack, needle, mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn provider() -> LocalKBProvider {
        LocalKBProvider::from_entries(vec![
            KbEntry {
                canonical: "蚌埠住了".into(),
                category: Some("meme".into()),
                aliases: vec!["绷不住了".into()],
                match_terms: vec!["笑死".into()],
                priority: 1.0,
                match_mode: "contains".into(),
                ..Default::default()
            },
            KbEntry {
                canonical: "甄嬛传".into(),
                category: Some("source".into()),
                aliases: vec!["后宫甄嬛传".into()],
                match_terms: vec!["皇上".into(), "臣妾".into()],
                priority: 0.95,
                match_mode: "contains".into(),
                ..Default::default()
            },
        ])
    }

    #[test]
    fn test_normalize_query_alias_to_canonical() {
        let normalized = provider().normalize_query("绷不住了");
        assert_eq!(normalized.tag_query, "蚌埠住了");
    }

    #[test]
    fn test_auto_tag_from_ocr() {
        let tags = provider().auto_tag("这个我真的绷不住了", "sample.jpg");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_text, "蚌埠住了");
        assert!(tags[0].confidence >= 0.6);
    }

    #[test]
    fn test_auto_tag_ignores_file_name_only() {
        let tags = provider().auto_tag("", "绷不住了_sample.jpg");
        assert!(tags.is_empty());
    }

    #[test]
    fn expand_query_supports_legacy_schema() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knowledge_base.json");
        std::fs::write(
            &path,
            r#"{
              "version": 1,
              "entries": [
                {
                  "canonical": "蚌埠住了",
                  "aliases": ["绷不住了"],
                  "description": "表示忍不住笑了",
                  "tags": ["搞笑", "表情包"],
                  "type": "meme"
                }
              ]
            }"#,
        )
        .unwrap();

        let provider = LocalKBProvider::load(&path).unwrap();
        let expanded = provider.expand_query("绷不住了");

        assert_eq!(expanded, "蚌埠住了 表示忍不住笑了 搞笑 表情包");
    }

    #[test]
    fn expand_query_supports_maintenance_schema() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knowledge_base.json");
        std::fs::write(
            &path,
            r#"{
              "version": 1,
              "entries": [
                {
                  "canonical": "甄嬛传",
                  "category": "source",
                  "aliases": ["甄嬛"],
                  "match_terms": ["皇上", "娘娘"],
                  "description": "宫斗剧经典来源",
                  "match_mode": "contains",
                  "priority": 20
                }
              ]
            }"#,
        )
        .unwrap();

        let provider = LocalKBProvider::load(&path).unwrap();
        let expanded = provider.expand_query("甄嬛");

        assert_eq!(expanded, "甄嬛传 宫斗剧经典来源 皇上 娘娘");
    }
}
