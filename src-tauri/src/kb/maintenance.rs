use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const DEFAULT_VERSION: u32 = 1;
const DEFAULT_MATCH_MODE: &str = "contains";
const VALID_CATEGORIES: [&str; 3] = ["meme", "source", "person"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeBaseFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub entries: Vec<KnowledgeBaseEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KnowledgeBaseEntry {
    #[serde(alias = "canonical")]
    pub name: String,
    #[serde(alias = "type")]
    pub category: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(alias = "description")]
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub example_images: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub conflicts: Vec<TermConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermConflict {
    pub term: String,
    pub canonicals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchCandidate {
    pub name: String,
    pub category: String,
    pub match_type: MatchType,
    pub matched_term: String,
    pub score: i32,
    pub priority: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchType {
    CanonicalExact,
    AliasExact,
    CanonicalSubstring,
    AliasSubstring,
}

pub struct KnowledgeBaseStore {
    path: PathBuf,
    kb: KnowledgeBaseFile,
}

fn default_version() -> u32 {
    DEFAULT_VERSION
}

fn default_match_mode() -> String {
    DEFAULT_MATCH_MODE.to_string()
}

impl Default for KnowledgeBaseFile {
    fn default() -> Self {
        Self {
            version: DEFAULT_VERSION,
            entries: vec![],
        }
    }
}

impl KnowledgeBaseFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取私有角色库文件失败：{}", path.display()))?;
        Self::from_json_str(&content)
    }

    pub fn from_json_str(content: &str) -> anyhow::Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(content).context("私有角色库文件格式错误")?;
        let version = value["version"]
            .as_u64()
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_VERSION);

        let entries = value["entries"]
            .as_array()
            .ok_or_else(|| anyhow!("私有角色库文件格式错误"))?
            .iter()
            .cloned()
            .map(parse_entry)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self { version, entries })
    }

    pub fn to_pretty_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(self).context("序列化私有角色库失败")
    }

    pub fn validate(&self) -> ValidationReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut name_map: BTreeMap<String, String> = BTreeMap::new();
        let mut term_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for entry in &self.entries {
            let name = entry.name.trim();
            if name.is_empty() {
                errors.push("name 不能为空".to_string());
            }

            if !VALID_CATEGORIES.contains(&entry.category.as_str()) {
                errors.push(format!("非法分类：{}", entry.category));
            }

            if !matches!(
                entry.match_mode.as_str(),
                "exact" | "contains" | "exact_or_contains"
            ) {
                errors.push(format!(
                    "非法 match_mode：{}（仅支持 exact / contains / exact_or_contains）",
                    entry.match_mode
                ));
            }

            let name_key = normalize_text(name);
            if let Some(previous) = name_map.insert(name_key, name.to_string()) {
                errors.push(format!("name 已存在：{}", previous));
            }

            validate_term_collection("aliases", &entry.aliases, name, &mut errors);
            validate_example_images(entry, name, &mut errors, &mut warnings);

            for term in entry.all_terms() {
                let normalized = normalize_text(&term);
                if normalized.is_empty() {
                    continue;
                }
                term_map
                    .entry(normalized.clone())
                    .or_default()
                    .insert(name.to_string());

                if is_ambiguous_short_term(&normalized) {
                    warnings.push(format!("高歧义短词，请确认是否保留：{} -> {}", name, term));
                }
            }
        }

        let conflicts = term_map
            .into_iter()
            .filter_map(|(term, canonicals)| {
                if canonicals.len() > 1 {
                    Some(TermConflict {
                        term,
                        canonicals: canonicals.into_iter().collect(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        warnings.extend(conflicts.iter().map(|conflict| {
            format!(
                "检测到潜在冲突词：{} -> {}",
                conflict.term,
                conflict.canonicals.join("、")
            )
        }));

        ValidationReport {
            errors,
            warnings,
            conflicts,
        }
    }

    pub fn search_entries(
        &self,
        name: Option<&str>,
        category: Option<&str>,
        keyword: Option<&str>,
    ) -> Vec<&KnowledgeBaseEntry> {
        let keyword = keyword.map(normalize_text);
        self.entries
            .iter()
            .filter(|entry| name.map(|v| entry.name == v).unwrap_or(true))
            .filter(|entry| category.map(|v| entry.category == v).unwrap_or(true))
            .filter(|entry| {
                keyword
                    .as_ref()
                    .map(|kw| {
                        normalize_text(&entry.name).contains(kw)
                            || entry
                                .aliases
                                .iter()
                                .any(|term| normalize_text(term).contains(kw))
                    })
                    .unwrap_or(true)
            })
            .collect()
    }

    pub fn test_match(&self, text: &str) -> Vec<MatchCandidate> {
        let normalized_text = normalize_text(text);
        if normalized_text.is_empty() {
            return vec![];
        }

        let mut results = self
            .entries
            .iter()
            .filter_map(|entry| entry_match(entry, &normalized_text))
            .collect::<Vec<_>>();

        results.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.priority.cmp(&a.priority))
                .then_with(|| b.matched_term.chars().count().cmp(&a.matched_term.chars().count()))
                .then_with(|| a.name.cmp(&b.name))
        });
        results
    }

    pub fn format_in_place(&mut self) {
        self.entries
            .sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.name.cmp(&b.name)));
        for entry in &mut self.entries {
            entry.normalize();
        }
    }
}

impl KnowledgeBaseEntry {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.category = self.category.trim().to_string();
        self.notes = self.notes.trim().to_string();
        self.match_mode = self.match_mode.trim().to_string();
        self.aliases = dedup_terms(&self.aliases);
        self.example_images = dedup_paths(&self.example_images);
    }

    pub fn all_terms(&self) -> Vec<String> {
        let mut terms = vec![self.name.clone()];
        terms.extend(self.aliases.clone());
        terms
    }
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl KnowledgeBaseStore {
    pub fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let kb = if path.exists() {
            KnowledgeBaseFile::load(&path)?
        } else {
            KnowledgeBaseFile::default()
        };
        Ok(Self { path, kb })
    }

    pub fn kb(&self) -> &KnowledgeBaseFile {
        &self.kb
    }

    pub fn replace_all(&mut self, mut kb: KnowledgeBaseFile) -> anyhow::Result<()> {
        kb.format_in_place();
        let report = kb.validate();
        if !report.is_valid() {
            bail!(report.errors.join("；"));
        }
        self.kb = kb;
        Ok(())
    }

    pub fn add_entry(&mut self, mut entry: KnowledgeBaseEntry) -> anyhow::Result<()> {
        entry.normalize();
        if self
            .kb
            .entries
            .iter()
            .any(|current| normalize_text(&current.name) == normalize_text(&entry.name))
        {
            bail!("name 已存在");
        }
        self.kb.entries.push(entry);
        self.kb.format_in_place();
        self.ensure_valid()?;
        Ok(())
    }

    pub fn edit_entry(
        &mut self,
        name: &str,
        mut replacement: KnowledgeBaseEntry,
    ) -> anyhow::Result<()> {
        replacement.normalize();
        let target = normalize_text(name);
        let index = self
            .kb
            .entries
            .iter()
            .position(|entry| normalize_text(&entry.name) == target)
            .ok_or_else(|| anyhow!("未找到私有角色条目：{}", name))?;
        self.kb.entries[index] = replacement;
        self.kb.format_in_place();
        self.ensure_valid()?;
        Ok(())
    }

    pub fn delete_entry(&mut self, name: &str) -> anyhow::Result<()> {
        let before = self.kb.entries.len();
        let target = normalize_text(name);
        self.kb
            .entries
            .retain(|entry| normalize_text(&entry.name) != target);
        if self.kb.entries.len() == before {
            bail!("未找到私有角色条目：{}", name);
        }
        Ok(())
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.kb.format_in_place();
        self.ensure_valid()?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建目录失败：{}", parent.display()))?;
        }
        std::fs::write(&self.path, self.kb.to_pretty_json()?)
            .with_context(|| format!("写回私有角色库文件失败：{}", self.path.display()))?;
        Ok(())
    }

    fn ensure_valid(&self) -> anyhow::Result<()> {
        let report = self.kb.validate();
        if report.is_valid() {
            Ok(())
        } else {
            bail!(report.errors.join("；"))
        }
    }
}

pub fn default_kb_path() -> PathBuf {
    PathBuf::from("app_data/knowledge_base.json")
}

pub fn resolve_default_kb_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_relative = default_kb_path();
    let candidates = [
        manifest_dir
            .parent()
            .map(|parent| parent.join(&default_relative)),
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(&default_relative)),
        std::env::current_dir().ok().and_then(|cwd| {
            cwd.parent()
                .map(|parent| parent.join(&default_relative))
        }),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return candidate;
        }
    }

    manifest_dir
        .parent()
        .map(|parent| parent.join(default_relative))
        .unwrap_or_else(default_kb_path)
}

fn entry_match(entry: &KnowledgeBaseEntry, normalized_text: &str) -> Option<MatchCandidate> {
    let normalized_name = normalize_text(&entry.name);
    let mut best = score_term(
        &entry.name,
        &normalized_name,
        normalized_text,
        MatchType::CanonicalExact,
        MatchType::CanonicalSubstring,
        entry.priority,
        &entry.match_mode,
    );

    for alias in &entry.aliases {
        let candidate = score_term(
            alias,
            &normalize_text(alias),
            normalized_text,
            MatchType::AliasExact,
            MatchType::AliasSubstring,
            entry.priority,
            &entry.match_mode,
        );
        best = select_better(best, candidate);
    }

    best.map(|(match_type, matched_term, score)| MatchCandidate {
        name: entry.name.clone(),
        category: entry.category.clone(),
        match_type,
        matched_term,
        score,
        priority: entry.priority,
    })
}

fn score_term(
    original_term: &str,
    normalized_term: &str,
    normalized_text: &str,
    exact_type: MatchType,
    substring_type: MatchType,
    priority: i32,
    match_mode: &str,
) -> Option<(MatchType, String, i32)> {
    if normalized_term.is_empty() {
        return None;
    }

    if normalized_text == normalized_term {
        let score = base_score(exact_type) + normalized_term.chars().count() as i32 * 5 + priority;
        return Some((exact_type, original_term.to_string(), score));
    }

    if matches!(match_mode, "contains" | "exact_or_contains")
        && normalized_text.contains(normalized_term)
    {
        let score =
            base_score(substring_type) + normalized_term.chars().count() as i32 * 3 + priority;
        return Some((substring_type, original_term.to_string(), score));
    }

    None
}

fn select_better(
    current: Option<(MatchType, String, i32)>,
    next: Option<(MatchType, String, i32)>,
) -> Option<(MatchType, String, i32)> {
    match (current, next) {
        (None, value) => value,
        (value, None) => value,
        (Some(current), Some(next)) => {
            if next.2 > current.2 {
                Some(next)
            } else {
                Some(current)
            }
        }
    }
}

fn base_score(match_type: MatchType) -> i32 {
    match match_type {
        MatchType::CanonicalExact => 600,
        MatchType::AliasExact => 520,
        MatchType::CanonicalSubstring => 420,
        MatchType::AliasSubstring => 360,
    }
}

fn parse_entry(value: serde_json::Value) -> anyhow::Result<KnowledgeBaseEntry> {
    let mut entry: KnowledgeBaseEntry =
        serde_json::from_value(value.clone()).context("私有角色条目格式错误")?;
    if entry.category.is_empty() {
        entry.category = value["type"]
            .as_str()
            .unwrap_or_default()
            .to_string();
    }
    if entry.match_mode.is_empty() {
        entry.match_mode = default_match_mode();
    }
    if entry.example_images.is_empty() {
        entry.example_images = value["example_images"]
            .as_array()
            .map(|array| {
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(|value| value.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }
    entry.normalize();
    Ok(entry)
}

fn validate_example_images(
    entry: &KnowledgeBaseEntry,
    name: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in &entry.example_images {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            errors.push(format!("example_images 存在空值：{}", name));
            continue;
        }
        let key = trimmed.to_lowercase();
        if !seen.insert(key) {
            errors.push(format!("检测到重复 example_images：{}", name));
            break;
        }
        let path = Path::new(trimmed);
        let extension_ok = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif"))
            .unwrap_or(false);
        if !extension_ok {
            errors.push(format!(
                "示例图格式不受支持：{} -> {}",
                name, trimmed
            ));
        }
    }

    if entry.category != "person" {
        warnings.push(format!("当前条目不是私有角色卡片：{}", name));
    } else if entry.example_images.is_empty() {
        warnings.push(format!("缺少示例图，将降级为普通搜索：{}", name));
    }
}

fn validate_term_collection(
    field_name: &str,
    values: &[String],
    name: &str,
    errors: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        let normalized = normalize_text(value);
        if normalized.is_empty() {
            errors.push(format!("{} 存在空值：{}", field_name, name));
            continue;
        }
        if !seen.insert(normalized) {
            errors.push(format!("检测到重复 {}：{}", field_name, name));
            break;
        }
    }
}

fn dedup_terms(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = normalize_text(trimmed);
        if seen.insert(normalized) {
            result.push(trimmed.to_string());
        }
    }
    result
}

fn dedup_paths(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            result.push(trimmed.to_string());
        }
    }
    result
}

pub fn normalize_text(input: &str) -> String {
    let lowered = input.trim().to_lowercase();
    let mut result = String::new();
    let mut previous_space = false;

    for ch in lowered.chars() {
        let mapped = match ch {
            '\u{3000}' => ' ',
            '\u{ff01}'..='\u{ff5e}' => char::from_u32(ch as u32 - 0xfee0).unwrap_or(ch),
            _ => ch,
        };

        if mapped.is_whitespace() {
            if !previous_space {
                result.push(' ');
            }
            previous_space = true;
            continue;
        }

        previous_space = false;
        if is_noise_punctuation(mapped) {
            continue;
        }
        result.push(mapped);
    }

    result.trim().to_string()
}

fn is_noise_punctuation(ch: char) -> bool {
    matches!(
        ch,
        ',' | '.'
            | '!'
            | '?'
            | ':'
            | ';'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '，'
            | '。'
            | '！'
            | '？'
            | '：'
            | '；'
            | '、'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '（'
            | '）'
            | '【'
            | '】'
        )
}

fn is_ambiguous_short_term(term: &str) -> bool {
    term.chars().count() <= 1 || (term.chars().count() <= 2 && !term.is_ascii())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_entry(name: &str) -> KnowledgeBaseEntry {
        KnowledgeBaseEntry {
            name: name.to_string(),
            category: "meme".to_string(),
            aliases: vec!["绷不住了".to_string()],
            notes: "测试条目".to_string(),
            match_mode: "contains".to_string(),
            priority: 10,
            example_images: vec![],
        }
    }

    #[test]
    fn validate_reports_duplicate_canonical_and_conflicts() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![
                KnowledgeBaseEntry {
                    name: "蚌埠住了".to_string(),
                    category: "meme".to_string(),
                    aliases: vec![
                        "绷不住了".to_string(),
                        "绷不住了".to_string(),
                        "皇上".to_string(),
                    ],
                    notes: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 10,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    name: "甄嬛传".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["笑死".to_string(), "皇上".to_string()],
                    notes: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 5,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    name: "蚌埠住了".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["蚌住了".to_string()],
                    notes: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 1,
                    example_images: vec![],
                },
            ],
        };

        let report = kb.validate();

        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|error| error.contains("name 已存在")));
        assert!(report.errors.iter().any(|error| error.contains("检测到重复 aliases")));
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].term, "皇上");
    }

    #[test]
    fn test_match_returns_best_candidate_and_hit_type() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![
                sample_entry("蚌埠住了"),
                KnowledgeBaseEntry {
                    name: "甄嬛传".to_string(),
                    category: "source".to_string(),
                    aliases: vec!["甄嬛".to_string()],
                    notes: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 20,
                    example_images: vec![],
                },
            ],
        };

        let result = kb.test_match("皇上看到这个真的绷不住了！！");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "蚌埠住了");
        assert_eq!(result[0].match_type, MatchType::AliasSubstring);
    }

    #[test]
    fn match_mode_exact_or_contains_and_exact_follow_spec() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![
                KnowledgeBaseEntry {
                    name: "马云".to_string(),
                    category: "person".to_string(),
                    aliases: vec!["杰克马".to_string()],
                    notes: String::new(),
                    match_mode: "exact_or_contains".to_string(),
                    priority: 10,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    name: "臣妾".to_string(),
                    category: "meme".to_string(),
                    aliases: vec![],
                    notes: String::new(),
                    match_mode: "exact".to_string(),
                    priority: 10,
                    example_images: vec![],
                },
            ],
        };

        let contains_result = kb.test_match("今天又看到杰克马的截图");
        assert_eq!(contains_result.len(), 1);
        assert_eq!(contains_result[0].name, "马云");
        assert_eq!(contains_result[0].match_type, MatchType::AliasSubstring);

        let exact_result = kb.test_match("臣妾");
        assert_eq!(exact_result.len(), 1);
        assert_eq!(exact_result[0].name, "臣妾");
        assert_eq!(exact_result[0].match_type, MatchType::CanonicalExact);

        let no_contains_for_exact = kb.test_match("臣妾做不到啊");
        assert!(no_contains_for_exact.iter().all(|item| item.name != "臣妾"));
    }

    #[test]
    fn store_add_edit_delete_and_save_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knowledge_base.json");
        let mut store = KnowledgeBaseStore::open(&path).unwrap();

        store.add_entry(sample_entry("蚌埠住了")).unwrap();
        store
            .edit_entry(
                "蚌埠住了",
                KnowledgeBaseEntry {
                    name: "蚌埠住了".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["绷不住了".to_string(), "笑死".to_string()],
                    notes: "更新说明".to_string(),
                    match_mode: "contains".to_string(),
                    priority: 30,
                    example_images: vec![],
                },
            )
            .unwrap();
        store.save().unwrap();

        let reloaded = KnowledgeBaseFile::load(&path).unwrap();
        assert_eq!(reloaded.entries.len(), 1);
        assert_eq!(reloaded.entries[0].aliases, vec!["绷不住了", "笑死"]);
        assert_eq!(reloaded.entries[0].name, "蚌埠住了");

        let mut reopened = KnowledgeBaseStore::open(&path).unwrap();
        reopened.delete_entry("蚌埠住了").unwrap();
        reopened.save().unwrap();

        let emptied = KnowledgeBaseFile::load(&path).unwrap();
        assert!(emptied.entries.is_empty());
    }

    #[test]
    fn load_legacy_schema_and_map_type_and_notes() {
        let kb = KnowledgeBaseFile::from_json_str(
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

        assert_eq!(kb.entries[0].category, "meme");
        assert_eq!(kb.entries[0].match_mode, "contains");
        assert_eq!(kb.entries[0].name, "蚌埠住了");
        assert_eq!(kb.entries[0].notes, "表示忍不住笑了");
    }

    #[test]
    fn load_new_role_schema_maps_name_and_notes() {
        let kb = KnowledgeBaseFile::from_json_str(
            r#"{
              "version": 1,
              "entries": [
                {
                  "name": "阿布",
                  "category": "person",
                  "aliases": ["布布"],
                  "notes": "私有角色",
                  "example_images": ["kb_examples/abu/sample-1.jpg"]
                }
              ]
            }"#,
        )
        .unwrap();

        assert_eq!(kb.entries[0].name, "阿布");
        assert_eq!(kb.entries[0].notes, "私有角色");
    }

    #[test]
    fn validate_warns_with_runtime_language_for_private_role_gating() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![
                KnowledgeBaseEntry {
                    name: "阿布".to_string(),
                    category: "person".to_string(),
                    aliases: vec!["布布".to_string()],
                    notes: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 0,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    name: "甄嬛传".to_string(),
                    category: "source".to_string(),
                    aliases: vec!["甄嬛".to_string()],
                    notes: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 0,
                    example_images: vec!["kb_examples/zhenhuan/sample-1.jpg".to_string()],
                },
            ],
        };

        let report = kb.validate();

        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning == "缺少示例图，将降级为普通搜索：阿布")
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning == "当前条目不是私有角色卡片：甄嬛传")
        );
    }

}
