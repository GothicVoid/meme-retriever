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
    pub canonical: String,
    #[serde(alias = "type")]
    pub category: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub match_terms: Vec<String>,
    #[serde(default)]
    pub description: String,
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
    pub canonical: String,
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
    MatchTermExact,
    MatchTermSubstring,
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
        let mut canonical_map: BTreeMap<String, String> = BTreeMap::new();
        let mut term_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for entry in &self.entries {
            let canonical = entry.canonical.trim();
            if canonical.is_empty() {
                errors.push("canonical 不能为空".to_string());
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

            let canonical_key = normalize_text(canonical);
            if let Some(previous) = canonical_map.insert(canonical_key, canonical.to_string()) {
                errors.push(format!("canonical 已存在：{}", previous));
            }

            validate_term_collection("aliases", &entry.aliases, canonical, &mut errors);
            validate_term_collection("match_terms", &entry.match_terms, canonical, &mut errors);
            validate_example_images(entry, canonical, &mut errors, &mut warnings);

            for term in entry.all_terms() {
                let normalized = normalize_text(&term);
                if normalized.is_empty() {
                    continue;
                }
                term_map
                    .entry(normalized.clone())
                    .or_default()
                    .insert(canonical.to_string());

                if is_ambiguous_short_term(&normalized) {
                    warnings.push(format!("高歧义短词，请确认是否保留：{} -> {}", canonical, term));
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
        canonical: Option<&str>,
        category: Option<&str>,
        keyword: Option<&str>,
    ) -> Vec<&KnowledgeBaseEntry> {
        let keyword = keyword.map(normalize_text);
        self.entries
            .iter()
            .filter(|entry| canonical.map(|v| entry.canonical == v).unwrap_or(true))
            .filter(|entry| category.map(|v| entry.category == v).unwrap_or(true))
            .filter(|entry| {
                keyword
                    .as_ref()
                    .map(|kw| {
                        normalize_text(&entry.canonical).contains(kw)
                            || entry
                                .aliases
                                .iter()
                                .chain(entry.match_terms.iter())
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
                .then_with(|| a.canonical.cmp(&b.canonical))
        });
        results
    }

    pub fn format_in_place(&mut self) {
        self.entries
            .sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.canonical.cmp(&b.canonical)));
        for entry in &mut self.entries {
            entry.normalize();
        }
    }
}

impl KnowledgeBaseEntry {
    pub fn normalize(&mut self) {
        self.canonical = self.canonical.trim().to_string();
        self.category = self.category.trim().to_string();
        self.description = self.description.trim().to_string();
        self.match_mode = self.match_mode.trim().to_string();
        self.aliases = dedup_terms(&self.aliases);
        self.match_terms = dedup_terms(&self.match_terms);
        self.example_images = dedup_paths(&self.example_images);
    }

    pub fn all_terms(&self) -> Vec<String> {
        let mut terms = vec![self.canonical.clone()];
        terms.extend(self.aliases.clone());
        terms.extend(self.match_terms.clone());
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
            .any(|current| normalize_text(&current.canonical) == normalize_text(&entry.canonical))
        {
            bail!("canonical 已存在");
        }
        self.kb.entries.push(entry);
        self.kb.format_in_place();
        self.ensure_valid()?;
        Ok(())
    }

    pub fn edit_entry(
        &mut self,
        canonical: &str,
        mut replacement: KnowledgeBaseEntry,
    ) -> anyhow::Result<()> {
        replacement.normalize();
        let target = normalize_text(canonical);
        let index = self
            .kb
            .entries
            .iter()
            .position(|entry| normalize_text(&entry.canonical) == target)
            .ok_or_else(|| anyhow!("未找到私有角色条目：{}", canonical))?;
        self.kb.entries[index] = replacement;
        self.kb.format_in_place();
        self.ensure_valid()?;
        Ok(())
    }

    pub fn delete_entry(&mut self, canonical: &str) -> anyhow::Result<()> {
        let before = self.kb.entries.len();
        let target = normalize_text(canonical);
        self.kb
            .entries
            .retain(|entry| normalize_text(&entry.canonical) != target);
        if self.kb.entries.len() == before {
            bail!("未找到私有角色条目：{}", canonical);
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

pub fn execute_cli(args: &[String]) -> anyhow::Result<String> {
    let Some(command) = args.first().map(|value| value.as_str()) else {
        return Ok(cli_help());
    };

    match command {
        "list" => {
            let path = parse_optional_path(args, "--file");
            let kb = KnowledgeBaseFile::load(&path)?;
            let entries = kb.search_entries(
                parse_optional_value(args, "--canonical").as_deref(),
                parse_optional_value(args, "--category").as_deref(),
                parse_optional_value(args, "--keyword").as_deref(),
            );
            if entries.is_empty() {
                return Ok("未找到任何私有角色条目".to_string());
            }
            Ok(entries
                .into_iter()
                .map(|entry| {
                    format!(
                        "{} [{}] aliases={} match_terms={} priority={}",
                        entry.canonical,
                        entry.category,
                        entry.aliases.len(),
                        entry.match_terms.len(),
                        entry.priority
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "add" => {
            let path = parse_optional_path(args, "--file");
            let mut store = KnowledgeBaseStore::open(path)?;
            let entry = parse_entry_from_args(args)?;
            let canonical = entry.canonical.clone();
            store.add_entry(entry)?;
            store.save()?;
            Ok(format!("已创建私有角色条目：{}", canonical))
        }
        "edit" => {
            let path = parse_optional_path(args, "--file");
            let target = require_value(args, "--target")?;
            let mut store = KnowledgeBaseStore::open(path)?;
            let entry = parse_entry_from_args(args)?;
            let canonical = entry.canonical.clone();
            store.edit_entry(&target, entry)?;
            store.save()?;
            Ok(format!("已更新私有角色条目：{}", canonical))
        }
        "delete" => {
            let path = parse_optional_path(args, "--file");
            let canonical = require_value(args, "--canonical")?;
            let mut store = KnowledgeBaseStore::open(path)?;
            store.delete_entry(&canonical)?;
            store.save()?;
            Ok(format!("已删除私有角色条目：{}", canonical))
        }
        "validate" => {
            let path = parse_optional_path(args, "--file");
            let kb = KnowledgeBaseFile::load(&path)?;
            let report = kb.validate();
            if report.errors.is_empty() && report.warnings.is_empty() {
                return Ok("私有角色库校验通过".to_string());
            }

            let mut lines = Vec::new();
            if !report.errors.is_empty() {
                lines.push("错误:".to_string());
                lines.extend(report.errors.iter().map(|error| format!("- {}", error)));
            }
            if !report.warnings.is_empty() {
                lines.push("警告:".to_string());
                lines.extend(report.warnings.iter().map(|warning| format!("- {}", warning)));
            }
            Ok(lines.join("\n"))
        }
        "test-match" => {
            let path = parse_optional_path(args, "--file");
            let text = require_value(args, "--text")?;
            let kb = KnowledgeBaseFile::load(&path)?;
            let matches = kb.test_match(&text);
            if matches.is_empty() {
                return Ok("未命中任何私有角色条目".to_string());
            }
            let recommended = &matches[0].canonical;
            let mut lines = vec![format!(
                "输入文本命中 {} 个候选，最终推荐角色：{}",
                matches.len(),
                recommended
            )];
            lines.extend(matches.into_iter().map(|candidate| {
                format!(
                    "- {} | {:?} | 命中词={} | 分值={}",
                    candidate.canonical, candidate.match_type, candidate.matched_term, candidate.score
                )
            }));
            Ok(lines.join("\n"))
        }
        "format" => {
            let path = parse_optional_path(args, "--file");
            let mut store = KnowledgeBaseStore::open(path)?;
            store.save()?;
            Ok("已写回格式化后的私有角色库文件".to_string())
        }
        _ => Ok(cli_help()),
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
    let normalized_canonical = normalize_text(&entry.canonical);
    let mut best = score_term(
        &entry.canonical,
        &normalized_canonical,
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

    for term in &entry.match_terms {
        let candidate = score_term(
            term,
            &normalize_text(term),
            normalized_text,
            MatchType::MatchTermExact,
            MatchType::MatchTermSubstring,
            entry.priority,
            &entry.match_mode,
        );
        best = select_better(best, candidate);
    }

    best.map(|(match_type, matched_term, score)| MatchCandidate {
        canonical: entry.canonical.clone(),
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
        MatchType::MatchTermExact => 300,
        MatchType::MatchTermSubstring => 240,
    }
}

fn parse_entry(value: serde_json::Value) -> anyhow::Result<KnowledgeBaseEntry> {
    let mut entry: KnowledgeBaseEntry =
        serde_json::from_value(value.clone()).context("私有角色条目格式错误")?;
    if entry.match_terms.is_empty() {
        entry.match_terms = value["tags"]
            .as_array()
            .map(|array| {
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(|value| value.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }
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
    canonical: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in &entry.example_images {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            errors.push(format!("example_images 存在空值：{}", canonical));
            continue;
        }
        let key = trimmed.to_lowercase();
        if !seen.insert(key) {
            errors.push(format!("检测到重复 example_images：{}", canonical));
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
                canonical, trimmed
            ));
        }
    }

    if matches!(entry.category.as_str(), "person" | "source") && entry.example_images.is_empty() {
        warnings.push(format!("{} 分类建议补充示例图：{}", entry.category, canonical));
    }
}

fn validate_term_collection(
    field_name: &str,
    values: &[String],
    canonical: &str,
    errors: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        let normalized = normalize_text(value);
        if normalized.is_empty() {
            errors.push(format!("{} 存在空值：{}", field_name, canonical));
            continue;
        }
        if !seen.insert(normalized) {
            errors.push(format!("检测到重复 {}：{}", field_name, canonical));
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

fn parse_optional_value(args: &[String], key: &str) -> Option<String> {
    parse_optional_value_by_aliases(args, &[key])
}

fn parse_optional_value_by_aliases(args: &[String], keys: &[&str]) -> Option<String> {
    args.windows(2)
        .find(|window| keys.iter().any(|key| window[0] == *key))
        .map(|window| window[1].clone())
}

fn parse_optional_path(args: &[String], key: &str) -> PathBuf {
    let aliases = param_aliases(key);
    parse_optional_value_by_aliases(args, &aliases)
        .map(PathBuf::from)
        .unwrap_or_else(default_kb_path)
}

fn require_value(args: &[String], key: &str) -> anyhow::Result<String> {
    let aliases = param_aliases(key);
    parse_optional_value_by_aliases(args, &aliases)
        .ok_or_else(|| anyhow!("缺少参数：{}", key))
}

fn parse_csv(args: &[String], key: &str) -> Vec<String> {
    let aliases = param_aliases(key);
    parse_optional_value_by_aliases(args, &aliases)
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn param_aliases(key: &str) -> Vec<&str> {
    match key {
        "--file" => vec!["--file", "-f"],
        "--canonical" => vec!["--canonical", "-c"],
        "--category" => vec!["--category", "-g"],
        "--keyword" => vec!["--keyword", "-k"],
        "--aliases" => vec!["--aliases", "-a"],
        "--match-terms" => vec!["--match-terms", "-m"],
        "--description" => vec!["--description", "-d"],
        "--match-mode" => vec!["--match-mode", "-M"],
        "--priority" => vec!["--priority", "-p"],
        "--target" => vec!["--target", "-t"],
        "--text" => vec!["--text", "-x"],
        _ => vec![key],
    }
}

fn parse_entry_from_args(args: &[String]) -> anyhow::Result<KnowledgeBaseEntry> {
    Ok(KnowledgeBaseEntry {
        canonical: require_value(args, "--canonical")?,
        category: require_value(args, "--category")?,
        aliases: parse_csv(args, "--aliases"),
        match_terms: parse_csv(args, "--match-terms"),
        description: parse_optional_value(args, "--description").unwrap_or_default(),
        match_mode: parse_optional_value(args, "--match-mode").unwrap_or_else(default_match_mode),
        priority: parse_optional_value(args, "--priority")
            .map(|value| value.parse::<i32>().context("priority 必须为整数"))
            .transpose()?
            .unwrap_or_default(),
        example_images: vec![],
    })
}

fn cli_help() -> String {
    [
        "kb list [--file path] [--canonical 名称] [--category 分类] [--keyword 关键词]",
        "  别名: -f -c -g -k",
        "kb add --canonical 名称 --category 分类 [--aliases a,b] [--match-terms a,b] [--description 文本] [--match-mode exact|contains|exact_or_contains] [--priority 数值]",
        "  别名: -c -g -a -m -d -M -p",
        "kb edit --target 旧名称 --canonical 新名称 --category 分类 [其他参数同 add]",
        "  别名: -t -c -g -a -m -d -M -p",
        "kb delete --canonical 名称 [--file path]",
        "  别名: -c -f",
        "kb validate [--file path]",
        "  别名: -f",
        "kb test-match --text OCR文本 [--file path]",
        "  别名: -x -f",
        "kb format [--file path]",
        "  别名: -f",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_entry(canonical: &str) -> KnowledgeBaseEntry {
        KnowledgeBaseEntry {
            canonical: canonical.to_string(),
            category: "meme".to_string(),
            aliases: vec!["绷不住了".to_string()],
            match_terms: vec!["忍不住笑".to_string()],
            description: "测试条目".to_string(),
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
                    canonical: "蚌埠住了".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["绷不住了".to_string(), "绷不住了".to_string()],
                    match_terms: vec!["皇上".to_string()],
                    description: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 10,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    canonical: "甄嬛传".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["笑死".to_string()],
                    match_terms: vec!["皇上".to_string()],
                    description: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 5,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    canonical: "蚌埠住了".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["蚌住了".to_string()],
                    match_terms: vec!["笑到不行".to_string()],
                    description: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 1,
                    example_images: vec![],
                },
            ],
        };

        let report = kb.validate();

        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|error| error.contains("canonical 已存在")));
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
                    canonical: "甄嬛传".to_string(),
                    category: "source".to_string(),
                    aliases: vec!["甄嬛".to_string()],
                    match_terms: vec!["皇上".to_string()],
                    description: String::new(),
                    match_mode: "contains".to_string(),
                    priority: 20,
                    example_images: vec![],
                },
            ],
        };

        let result = kb.test_match("皇上看到这个真的绷不住了！！");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].canonical, "蚌埠住了");
        assert_eq!(result[0].match_type, MatchType::AliasSubstring);
        assert_eq!(result[1].canonical, "甄嬛传");
        assert_eq!(result[1].match_type, MatchType::MatchTermSubstring);
    }

    #[test]
    fn match_mode_exact_or_contains_and_exact_follow_spec() {
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![
                KnowledgeBaseEntry {
                    canonical: "马云".to_string(),
                    category: "person".to_string(),
                    aliases: vec!["杰克马".to_string()],
                    match_terms: vec!["马云".to_string()],
                    description: String::new(),
                    match_mode: "exact_or_contains".to_string(),
                    priority: 10,
                    example_images: vec![],
                },
                KnowledgeBaseEntry {
                    canonical: "臣妾".to_string(),
                    category: "meme".to_string(),
                    aliases: vec![],
                    match_terms: vec!["臣妾".to_string()],
                    description: String::new(),
                    match_mode: "exact".to_string(),
                    priority: 10,
                    example_images: vec![],
                },
            ],
        };

        let contains_result = kb.test_match("今天又看到杰克马的截图");
        assert_eq!(contains_result.len(), 1);
        assert_eq!(contains_result[0].canonical, "马云");
        assert_eq!(contains_result[0].match_type, MatchType::AliasSubstring);

        let exact_result = kb.test_match("臣妾");
        assert_eq!(exact_result.len(), 1);
        assert_eq!(exact_result[0].canonical, "臣妾");
        assert_eq!(exact_result[0].match_type, MatchType::CanonicalExact);

        let no_contains_for_exact = kb.test_match("臣妾做不到啊");
        assert!(no_contains_for_exact.iter().all(|item| item.canonical != "臣妾"));
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
                    canonical: "蚌埠住了".to_string(),
                    category: "meme".to_string(),
                    aliases: vec!["绷不住了".to_string(), "笑死".to_string()],
                    match_terms: vec!["忍不住笑".to_string()],
                    description: "更新说明".to_string(),
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

        let mut reopened = KnowledgeBaseStore::open(&path).unwrap();
        reopened.delete_entry("蚌埠住了").unwrap();
        reopened.save().unwrap();

        let emptied = KnowledgeBaseFile::load(&path).unwrap();
        assert!(emptied.entries.is_empty());
    }

    #[test]
    fn load_legacy_schema_and_map_type_and_tags() {
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
        assert_eq!(kb.entries[0].match_terms, vec!["搞笑", "表情包"]);
        assert_eq!(kb.entries[0].match_mode, "contains");
    }

    #[test]
    fn cli_validate_and_test_match_output_are_human_readable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knowledge_base.json");
        let kb = KnowledgeBaseFile {
            version: 1,
            entries: vec![sample_entry("蚌埠住了")],
        };
        std::fs::write(&path, kb.to_pretty_json().unwrap()).unwrap();

        let validate_output = execute_cli(&[
            "validate".to_string(),
            "--file".to_string(),
            path.display().to_string(),
        ])
        .unwrap();
        assert_eq!(validate_output, "私有角色库校验通过");

        let test_output = execute_cli(&[
            "test-match".to_string(),
            "--file".to_string(),
            path.display().to_string(),
            "--text".to_string(),
            "这个图我真的绷不住了".to_string(),
        ])
        .unwrap();
        assert!(test_output.contains("最终推荐角色：蚌埠住了"));
        assert!(test_output.contains("AliasSubstring"));
    }

    #[test]
    fn cli_supports_short_flag_aliases() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knowledge_base.json");

        let add_output = execute_cli(&[
            "add".to_string(),
            "-f".to_string(),
            path.display().to_string(),
            "-c".to_string(),
            "蚌埠住了".to_string(),
            "-g".to_string(),
            "meme".to_string(),
            "-a".to_string(),
            "绷不住了,蚌住了".to_string(),
            "-m".to_string(),
            "忍不住笑,破防了".to_string(),
            "-d".to_string(),
            "测试说明".to_string(),
            "-M".to_string(),
            "contains".to_string(),
            "-p".to_string(),
            "8".to_string(),
        ])
        .unwrap();
        assert_eq!(add_output, "已创建私有角色条目：蚌埠住了");

        let list_output = execute_cli(&[
            "list".to_string(),
            "-f".to_string(),
            path.display().to_string(),
            "-k".to_string(),
            "绷不住".to_string(),
        ])
        .unwrap();
        assert!(list_output.contains("蚌埠住了 [meme]"));

        let test_output = execute_cli(&[
            "test-match".to_string(),
            "-f".to_string(),
            path.display().to_string(),
            "-x".to_string(),
            "这个图我真的绷不住了".to_string(),
        ])
        .unwrap();
        assert!(test_output.contains("最终推荐角色：蚌埠住了"));

        let delete_output = execute_cli(&[
            "delete".to_string(),
            "-f".to_string(),
            path.display().to_string(),
            "-c".to_string(),
            "蚌埠住了".to_string(),
        ])
        .unwrap();
        assert_eq!(delete_output, "已删除私有角色条目：蚌埠住了");
    }
}
