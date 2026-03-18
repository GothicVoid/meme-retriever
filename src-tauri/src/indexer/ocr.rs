use std::path::Path;

/// 从图片中提取文字。
/// 真实推理需要 `src-tauri/models/ocr.onnx`，通过 `#[ignore]` 测试验证。
/// 当模型文件不存在时返回空字符串（降级处理）。
pub fn extract_text(image_path: &str) -> anyhow::Result<String> {
    let path = Path::new(image_path);
    if !path.exists() {
        anyhow::bail!("image not found: {image_path}");
    }

    let model_path = model_dir().join("ocr.onnx");
    if !model_path.exists() {
        tracing::debug!("ocr: model not found, skipping ({:?})", model_path);
        return Ok(String::new());
    }

    let start = std::time::Instant::now();
    let text = run_inference(path, &model_path)?;
    tracing::debug!("ocr: {}ms, text_len={}", start.elapsed().as_millis(), text.len());
    Ok(text)
}

fn model_dir() -> std::path::PathBuf {
    std::env::var("CLIP_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./models"))
}

fn run_inference(_image_path: &Path, _model_path: &Path) -> anyhow::Result<String> {
    // TODO: 实现真实 ONNX 推理（P2-C 真实模型阶段）
    // 需要：图像预处理 → ort::Session 推理 → CTC 解码
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_extract_text_returns_ok() {
        // 无模型时应降级返回 Ok("")
        let result = extract_text(&fixture("sample.jpg"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_text_blank_image() {
        let result = extract_text(&fixture("sample_blank.jpg"));
        assert!(result.is_ok());
        // 纯白图无论有无模型都不应 panic
    }

    #[test]
    fn test_extract_text_nonexistent() {
        let result = extract_text("/nonexistent/path/image.jpg");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    #[ignore = "需要 models/ocr.onnx"]
    fn test_extract_text_with_text() {
        let result = extract_text(&fixture("sample.jpg")).unwrap();
        assert!(!result.is_empty(), "含文字图片应返回非空文本");
    }
}
