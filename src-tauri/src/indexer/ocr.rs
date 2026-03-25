use once_cell::sync::OnceCell;
use std::path::Path;
use std::sync::Mutex;

static OCR_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();

/// 从图片中提取文字。
/// 需要 `models/ch_PP-OCRv4_rec_infer.onnx` 和 `models/ppocr_keys_v1.txt`。
/// 当模型或字典文件不存在时返回空字符串（降级处理）。
pub fn extract_text(image_path: &str) -> anyhow::Result<String> {
    let path = Path::new(image_path);
    if !path.exists() {
        anyhow::bail!("image not found: {image_path}");
    }

    let model_path = match find_model(&["ocr.onnx", "ch_PP-OCRv4_rec_infer.onnx"]) {
        Some(p) => p,
        None => {
            tracing::debug!("ocr: model not found, skipping");
            return Ok(String::new());
        }
    };

    let dict_path = model_dir().join("ppocr_keys_v1.txt");
    if !dict_path.exists() {
        tracing::debug!("ocr: dict not found ({:?}), skipping", dict_path);
        return Ok(String::new());
    }

    let start = std::time::Instant::now();
    let text = run_inference(path, &model_path, &dict_path)?;
    tracing::debug!("ocr: {}ms, text_len={}", start.elapsed().as_millis(), text.len());
    Ok(text)
}

fn model_dir() -> std::path::PathBuf {
    std::env::var("CLIP_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./models"))
}

fn find_model(candidates: &[&str]) -> Option<std::path::PathBuf> {
    let dir = model_dir();
    candidates.iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
}

fn load_dict(dict_path: &Path) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(dict_path)?;
    // 每行一个字符，行号即类别索引（从 1 开始，0 为 blank）
    let chars: Vec<String> = content.lines().map(|l| l.trim().to_string()).collect();
    Ok(chars)
}

fn run_inference(image_path: &Path, model_path: &Path, dict_path: &Path) -> anyhow::Result<String> {
    use ort::value::Tensor;

    // 1. 加载字典
    let dict = load_dict(dict_path)?;

    // 2. 加载图像，resize 到高度 48，等比缩放宽度
    let img = image::open(image_path)?.to_rgb8();
    let (orig_w, orig_h) = img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Ok(String::new());
    }

    const TARGET_H: u32 = 48;
    let target_w = ((orig_w as f32 * TARGET_H as f32 / orig_h as f32) as u32).clamp(1, 1920);

    let img = image::imageops::resize(&img, target_w, TARGET_H, image::imageops::FilterType::Triangle);
    let (w, h) = img.dimensions();

    // 3. 归一化：(pixel/255.0 - 0.5) / 0.5，CHW 格式
    let mut data = vec![0f32; 3 * h as usize * w as usize];
    for (idx, pixel) in img.pixels().enumerate() {
        for c in 0..3 {
            data[c * h as usize * w as usize + idx] =
                (pixel[c] as f32 / 255.0 - 0.5) / 0.5;
        }
    }

    // 4. shape [1, 3, H, W]
    let tensor = Tensor::from_array(([1usize, 3, h as usize, w as usize], data.into_boxed_slice()))?;

    // 5. 加载 session 并推理
    let session = OCR_SESSION.get_or_try_init(|| {
        tracing::info!("ocr: loading model {:?}", model_path);
        ort::session::Session::builder()?.commit_from_file(model_path)
            .map(Mutex::new)
            .map_err(|e| anyhow::anyhow!(e))
    })?;

    let mut guard = session.lock().unwrap();
    let outputs = guard.run(ort::inputs!["x" => tensor])?;
    // 输出 shape: [1, seq_len, num_classes]
    let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;
    if shape.len() < 3 {
        anyhow::bail!("unexpected OCR output shape: {:?}", shape);
    }
    let seq_len = shape[1] as usize;
    let num_classes = shape[2] as usize;

    // 6. CTC greedy decode
    // PaddleOCR 约定：index 0 = blank，dict[i-1] = 第 i 类字符
    let mut result = String::new();
    let mut prev_idx: usize = 0;

    for t in 0..seq_len {
        let offset = t * num_classes;
        let frame = &data[offset..offset + num_classes];
        let best_idx = frame.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // CTC collapse：跳过 blank（0）和与上一步相同的符号
        if best_idx != 0 && best_idx != prev_idx {
            let char_idx = best_idx - 1; // dict 从 index 1 开始
            if char_idx < dict.len() {
                result.push_str(&dict[char_idx]);
            }
        }
        prev_idx = best_idx;
    }

    Ok(result)
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
    fn test_extract_text_with_real_model() {
        if find_model(&["ocr.onnx", "ch_PP-OCRv4_rec_infer.onnx"]).is_none() {
            eprintln!("跳过：找不到 OCR 模型（ch_PP-OCRv4_rec_infer.onnx）");
            return;
        }
        // sample_text_line.png 是专为识别模型准备的单行文字条带（320×48），
        // 内容为 "test meme text"。recognition 模型不含 detection，需要预裁剪的文字行。
        let result = extract_text(&fixture("sample_text_line.png")).unwrap();
        assert!(!result.is_empty(), "OCR 应识别出文字条带中的文字，实际返回空字符串");
    }
}
