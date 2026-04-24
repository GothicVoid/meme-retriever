use std::collections::HashMap;

use serde::Deserialize;

use super::provider::{KnowledgeBaseProvider, PrivateRoleMatch, QueryNormalization};
use crate::db::repo::TagCategory;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
struct KbEntry {
    #[serde(alias = "canonical")]
    name: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default, alias = "type")]
    r#type: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    match_mode: String,
    #[serde(default = "default_priority")]
    priority: f32,
    #[serde(default)]
    example_images: Vec<String>,
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
            alias_to_canonical.insert(normalize(&entry.name), entry.name.clone());
            for alias in &entry.aliases {
                alias_to_canonical.insert(normalize(alias), entry.name.clone());
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
}

impl KnowledgeBaseProvider for LocalKBProvider {
    fn expand_query(&self, query: &str) -> String {
        query.trim().to_string()
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
            expanded_query: tag_query,
        }
    }

    fn related_terms(&self, query: &str) -> Vec<String> {
        let normalized = normalize(query);
        for entry in &self.entries {
            if normalize(&entry.name) == normalized
                || entry
                    .aliases
                    .iter()
                    .any(|alias| normalize(alias) == normalized)
            {
                return entry
                    .aliases
                    .iter()
                    .cloned()
                    .chain(std::iter::once(entry.name.clone()))
                    .collect();
            }
        }
        Vec::new()
    }

    fn detect_private_role(&self, query: &str) -> Option<PrivateRoleMatch> {
        let normalized_query = normalize(query);
        if normalized_query.is_empty() {
            return None;
        }

        self.entries
            .iter()
            .filter(|entry| {
                matches!(Self::entry_category(entry), TagCategory::Person)
                    && !entry.example_images.is_empty()
            })
            .filter_map(|entry| {
                let terms = std::iter::once(entry.name.as_str())
                    .chain(entry.aliases.iter().map(String::as_str))
                    .collect::<Vec<_>>();
                let matched_term = terms
                    .into_iter()
                    .filter_map(|term| {
                        let normalized_term = normalize(term);
                        if normalized_term.is_empty() || !normalized_query.contains(&normalized_term)
                        {
                            return None;
                        }
                        Some((term.to_string(), normalized_term.chars().count()))
                    })
                    .max_by(|a, b| a.1.cmp(&b.1))?;

                Some((
                    matched_term.1,
                    entry.priority,
                    PrivateRoleMatch {
                        name: entry.name.clone(),
                        matched_term: matched_term.0,
                        related_terms: entry
                            .aliases
                            .iter()
                            .cloned()
                            .chain(std::iter::once(entry.name.clone()))
                            .collect(),
                    },
                ))
            })
            .max_by(|a, b| {
                a.0.cmp(&b.0).then_with(|| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            })
            .map(|(_, _, role)| role)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn provider() -> LocalKBProvider {
        LocalKBProvider::from_entries(vec![
            KbEntry {
                name: "蚌埠住了".into(),
                category: Some("meme".into()),
                aliases: vec!["绷不住了".into()],
                priority: 1.0,
                ..Default::default()
            },
            KbEntry {
                name: "甄嬛传".into(),
                category: Some("source".into()),
                aliases: vec!["后宫甄嬛传".into()],
                priority: 0.95,
                ..Default::default()
            },
        ])
    }

    #[test]
    fn test_normalize_query_alias_to_canonical() {
        let normalized = provider().normalize_query("绷不住了");
        assert_eq!(normalized.tag_query, "蚌埠住了");
        assert_eq!(normalized.expanded_query, "蚌埠住了");
    }

    #[test]
    fn test_detect_private_role_from_query_substring() {
        let provider = LocalKBProvider::from_entries(vec![
            KbEntry {
                name: "阿布".into(),
                category: Some("person".into()),
                aliases: vec!["布布".into()],
                priority: 10.0,
                example_images: vec!["kb_examples/abu-1.jpg".into()],
                ..Default::default()
            },
            KbEntry {
                name: "甄嬛传".into(),
                category: Some("source".into()),
                aliases: vec!["甄嬛".into()],
                priority: 5.0,
                ..Default::default()
            },
        ]);

        let matched = provider.detect_private_role("我想找阿布撇嘴那张").unwrap();
        assert_eq!(matched.name, "阿布");
        assert_eq!(matched.matched_term, "阿布");
        assert_eq!(matched.related_terms, vec!["布布".to_string(), "阿布".to_string()]);
    }

    #[test]
    fn test_detect_private_role_requires_example_images() {
        let provider = LocalKBProvider::from_entries(vec![KbEntry {
            name: "老板".into(),
            category: Some("person".into()),
            aliases: vec!["王总".into()],
            priority: 10.0,
            example_images: vec![],
            ..Default::default()
        }]);

        assert!(provider.detect_private_role("老板冷笑").is_none());
    }

    #[test]
    fn test_detect_private_role_ignores_action_only_terms() {
        let provider = LocalKBProvider::from_entries(vec![KbEntry {
            name: "阿布".into(),
            category: Some("person".into()),
            aliases: vec!["布布".into()],
            priority: 10.0,
            example_images: vec!["kb_examples/abu-1.jpg".into()],
            ..Default::default()
        }]);

        assert!(provider.detect_private_role("我想找撇嘴那张").is_none());
    }

    #[test]
    fn normalize_query_supports_legacy_schema() {
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

        let normalized = LocalKBProvider::load(&path).unwrap().normalize_query("绷不住了");

        assert_eq!(normalized.tag_query, "蚌埠住了");
        assert_eq!(normalized.expanded_query, "蚌埠住了");
    }

    #[test]
    fn normalize_query_supports_maintenance_schema() {
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
                  "description": "宫斗剧经典来源",
                  "priority": 20
                }
              ]
            }"#,
        )
        .unwrap();

        let normalized = LocalKBProvider::load(&path).unwrap().normalize_query("甄嬛");

        assert_eq!(normalized.tag_query, "甄嬛传");
        assert_eq!(normalized.expanded_query, "甄嬛传");
    }
}
