use super::provider::KnowledgeBaseProvider;
use serde::Deserialize;

#[derive(Deserialize, Default)]
struct KbEntry {
    canonical: String,
    aliases: Vec<String>,
    description: String,
    tags: Vec<String>,
}

pub struct LocalKBProvider {
    entries: Vec<KbEntry>,
}

impl LocalKBProvider {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|_| r#"{"entries":[]}"#.to_string());
        let v: serde_json::Value = serde_json::from_str(&content)?;
        let entries = serde_json::from_value(v["entries"].clone()).unwrap_or_default();
        Ok(Self { entries })
    }
}

impl KnowledgeBaseProvider for LocalKBProvider {
    fn expand_query(&self, query: &str) -> String {
        for e in &self.entries {
            if e.canonical == query || e.aliases.iter().any(|a| a == query) {
                return format!("{} {} {}", e.canonical, e.description, e.tags.join(" "));
            }
        }
        query.to_string()
    }
}
