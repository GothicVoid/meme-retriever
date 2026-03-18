use once_cell::sync::OnceCell;
use std::collections::HashMap;

const SOT: i64 = 49406;
const EOT: i64 = 49407;
const CONTEXT_LENGTH: usize = 77;

/// BPE tokenizer for Chinese-CLIP.
/// 词表文件路径：`models/bpe_simple_vocab_16e6.txt`，不存在时降级为字符级分词。
pub fn tokenize(text: &str) -> Vec<i64> {
    tracing::debug!("tokenize: input_len={}", text.len());
    let tokens = get_tokenizer().encode(text);
    tracing::debug!("tokenize: token_count={}", tokens.len());
    tokens
}

// ── 内部实现 ────────────────────────────────────────────────────────────────

static TOKENIZER: OnceCell<BpeTokenizer> = OnceCell::new();

fn get_tokenizer() -> &'static BpeTokenizer {
    TOKENIZER.get_or_init(|| {
        let vocab_path = model_dir().join("bpe_simple_vocab_16e6.txt");
        if vocab_path.exists() {
            match BpeTokenizer::load(&vocab_path) {
                Ok(t) => {
                    tracing::info!("tokenizer: loaded BPE vocab from {:?}", vocab_path);
                    return t;
                }
                Err(e) => tracing::warn!("tokenizer: failed to load vocab: {e}, falling back to char-level"),
            }
        } else {
            tracing::debug!("tokenizer: vocab not found at {:?}, using char-level fallback", vocab_path);
        }
        BpeTokenizer::char_level()
    })
}

fn model_dir() -> std::path::PathBuf {
    std::env::var("CLIP_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./models"))
}

struct BpeTokenizer {
    encoder: HashMap<String, i64>,
}

impl BpeTokenizer {
    /// 字符级 fallback：每个 Unicode 字符映射到其 codepoint（截断到合法范围）
    fn char_level() -> Self {
        Self { encoder: HashMap::new() }
    }

    /// 从 BPE vocab 文件加载（格式：每行一个 token，行号即 token id）
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
        // 简单实现：按空格分词，查表，未知词用字符 codepoint
        let mut tokens: Vec<i64> = vec![SOT];

        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if let Some(&id) = self.encoder.get(word) {
                tokens.push(id);
            } else {
                // 字符级 fallback
                for ch in word.chars() {
                    let id = (ch as i64).min(49405); // 不超过 SOT-1
                    tokens.push(id);
                }
            }
            if tokens.len() >= CONTEXT_LENGTH - 1 {
                break;
            }
        }

        tokens.push(EOT);

        // 截断到 CONTEXT_LENGTH
        tokens.truncate(CONTEXT_LENGTH);

        // padding 到 CONTEXT_LENGTH
        while tokens.len() < CONTEXT_LENGTH {
            tokens.push(0);
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
        assert_eq!(tokens[0], SOT);
        // EOT 在第一个 0 之前
        let eot_pos = tokens.iter().position(|&t| t == EOT).expect("EOT not found");
        assert!(eot_pos > 0);
        assert!(eot_pos < CONTEXT_LENGTH);
    }

    #[test]
    fn test_tokenize_chinese() {
        let tokens = tokenize("蚌埠住了");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
        assert_eq!(tokens[0], SOT);
        // 中文字符应被编码为非零 token
        let non_zero: Vec<i64> = tokens.iter().cloned().filter(|&t| t != 0).collect();
        assert!(non_zero.len() > 2, "应有 SOT + 至少一个中文 token + EOT");
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
        assert_eq!(tokens[0], SOT);
        assert_eq!(tokens[1], EOT);
        // 其余全为 0
        assert!(tokens[2..].iter().all(|&t| t == 0));
    }

    #[test]
    fn test_tokenize_long_text() {
        // 超长文本应截断到 77
        let long = "a ".repeat(200);
        let tokens = tokenize(&long);
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
    }

    #[test]
    fn test_tokenize_padding() {
        // 短文本应 padding 到 77
        let tokens = tokenize("hi");
        assert_eq!(tokens.len(), CONTEXT_LENGTH);
        // 末尾应有 0 padding
        assert_eq!(tokens[CONTEXT_LENGTH - 1], 0);
    }
}
