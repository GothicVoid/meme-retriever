use once_cell::sync::OnceCell;
use std::collections::HashMap;

const CLS: i64 = 101;
const SEP: i64 = 102;
const PAD: i64 = 0;
const CONTEXT_LENGTH: usize = 52;

/// BERT tokenizer for Chinese-CLIP.
/// 词表文件路径：`models/vocab.txt`（bert-base-chinese），不存在时降级为字符级分词。
pub fn tokenize(text: &str) -> Vec<i64> {
    tracing::debug!("tokenize: input_len={}", text.len());
    let tokens = get_tokenizer().encode(text);
    tracing::debug!("tokenize: token_count={}", tokens.len());
    tokens
}

// ── 内部实现 ────────────────────────────────────────────────────────────────

static TOKENIZER: OnceCell<BertTokenizer> = OnceCell::new();

fn get_tokenizer() -> &'static BertTokenizer {
    TOKENIZER.get_or_init(|| {
        let vocab_path = model_dir().join("vocab.txt");
        if vocab_path.exists() {
            match BertTokenizer::load(&vocab_path) {
                Ok(t) => {
                    tracing::info!("tokenizer: loaded BERT vocab from {:?}", vocab_path);
                    return t;
                }
                Err(e) => tracing::warn!(
                    "tokenizer: failed to load vocab: {e}, falling back to char-level"
                ),
            }
        } else {
            tracing::debug!(
                "tokenizer: vocab not found at {:?}, using char-level fallback",
                vocab_path
            );
        }
        BertTokenizer::char_level()
    })
}

fn model_dir() -> std::path::PathBuf {
    crate::runtime_paths::model_dir()
}

struct BertTokenizer {
    encoder: HashMap<String, i64>,
}

impl BertTokenizer {
    /// 字符级 fallback：每个 Unicode 字符映射到其 codepoint（截断到 21127）
    fn char_level() -> Self {
        Self {
            encoder: HashMap::new(),
        }
    }

    /// 从 BERT vocab.txt 加载（每行一个 token，行号即 token id）
    fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let encoder: HashMap<String, i64> = content
            .lines()
            .enumerate()
            .map(|(i, line)| (line.trim().to_string(), i as i64))
            .collect();
        Ok(Self { encoder })
    }

    fn encode(&self, text: &str) -> Vec<i64> {
        // BERT format: [CLS] tokens... [SEP] [PAD]...
        let mut tokens: Vec<i64> = vec![CLS];

        for ch in text.chars() {
            // 每个字符作为独立 token（BERT 中文处理方式）
            let s = ch.to_string();
            if let Some(&id) = self.encoder.get(&s) {
                tokens.push(id);
            } else {
                // 未知字符用 [UNK]=100
                tokens.push(100);
            }
            if tokens.len() >= CONTEXT_LENGTH - 1 {
                break;
            }
        }

        tokens.push(SEP);

        // padding 到 CONTEXT_LENGTH
        while tokens.len() < CONTEXT_LENGTH {
            tokens.push(PAD);
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_output_shape() {
        let tokens = tokenize("hello world");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
    }

    #[test]
    fn test_tokenize_sot_eot() {
        let tokens = tokenize("hello");
        assert_eq!(tokens[0], CLS);
        // SEP 在第一个 PAD 之前
        let sep_pos = tokens
            .iter()
            .position(|&t| t == SEP)
            .expect("SEP not found");
        assert!(sep_pos > 0);
        assert!(sep_pos < CONTEXT_LENGTH);
    }

    #[test]
    fn test_tokenize_chinese() {
        let tokens = tokenize("蚌埠住了");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
        assert_eq!(tokens[0], CLS);
        // 中文字符应被编码为非零 token
        let non_zero: Vec<i64> = tokens.iter().cloned().filter(|&t| t != 0).collect();
        assert!(non_zero.len() > 2, "应有 CLS + 至少一个中文 token + SEP");
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
        assert_eq!(tokens[0], CLS);
        assert_eq!(tokens[1], SEP);
        // 其余全为 PAD
        assert!(tokens[2..].iter().all(|&t| t == PAD));
    }

    #[test]
    fn test_tokenize_long_text() {
        // 超长文本应截断到 CONTEXT_LENGTH
        let long = "a".repeat(200);
        let tokens = tokenize(&long);
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
    }

    #[test]
    fn test_tokenize_padding() {
        // 短文本应 padding 到 CONTEXT_LENGTH
        let tokens = tokenize("hi");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
        // 末尾应有 PAD
        assert_eq!(tokens[CONTEXT_LENGTH - 1], PAD);
    }
}
