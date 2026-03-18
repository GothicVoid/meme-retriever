use once_cell::sync::OnceCell;
use std::path::Path;

static TEXT_SESSION: OnceCell<ort::session::Session> = OnceCell::new();
static IMAGE_SESSION: OnceCell<ort::session::Session> = OnceCell::new();

pub struct ClipEncoder;

impl ClipEncoder {
    /// 文本编码：tokenize → ONNX 推理 → L2 归一化 → 512 维向量
    /// 模型不存在时返回确定性 mock 向量（L2 归一化后的均匀向量）
    pub fn encode_text(text: &str) -> anyhow::Result<Vec<f32>> {
        let start = std::time::Instant::now();
        let model_path = model_dir().join("clip_text.onnx");

        let result = if model_path.exists() {
            let session = TEXT_SESSION.get_or_try_init(|| load_session(&model_path))?;
            run_text_inference(session, text)?
        } else {
            tracing::debug!("clip: text model not found, using mock");
            mock_vector(text.len())
        };

        tracing::debug!("encode_text: {}ms", start.elapsed().as_millis());
        Ok(result)
    }

    /// 图像编码：预处理 → ONNX 推理 → L2 归一化 → 512 维向量
    /// 模型不存在时返回确定性 mock 向量
    pub fn encode_image(image_path: &str) -> anyhow::Result<Vec<f32>> {
        let path = Path::new(image_path);
        if !path.exists() {
            anyhow::bail!("image not found: {image_path}");
        }

        let start = std::time::Instant::now();
        let model_path = model_dir().join("clip_image.onnx");

        let result = if model_path.exists() {
            let session = IMAGE_SESSION.get_or_try_init(|| load_session(&model_path))?;
            run_image_inference(session, path)?
        } else {
            tracing::debug!("clip: image model not found, using mock");
            // mock：基于文件名生成确定性向量
            let seed = image_path.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
            mock_vector(seed as usize)
        };

        tracing::debug!("encode_image: {}ms", start.elapsed().as_millis());
        Ok(result)
    }
}

// ── 内部实现 ────────────────────────────────────────────────────────────────

fn model_dir() -> std::path::PathBuf {
    std::env::var("CLIP_MODEL_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./models"))
}

fn load_session(path: &Path) -> anyhow::Result<ort::session::Session> {
    tracing::info!("clip: loading model {:?}", path);
    let session = ort::session::Session::builder()?.commit_from_file(path)?;
    Ok(session)
}

fn run_text_inference(_session: &ort::session::Session, text: &str) -> anyhow::Result<Vec<f32>> {
    // TODO: 真实推理（P2-B 真实模型阶段）
    // 1. tokenize(text) → Vec<i64>
    // 2. ndarray 转换 → session.run()
    // 3. 取输出 → l2_normalize
    let _ = text;
    Ok(mock_vector(text.len()))
}

fn run_image_inference(_session: &ort::session::Session, image_path: &Path) -> anyhow::Result<Vec<f32>> {
    // TODO: 真实推理（P2-B 真实模型阶段）
    // 1. image::open → resize 224×224 → 归一化(mean/std) → CHW ndarray
    // 2. session.run() → 取输出 → l2_normalize
    let seed = image_path.to_string_lossy().bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
    Ok(mock_vector(seed as usize))
}

/// 生成确定性的 L2 归一化 512 维向量（用于无模型时的 mock）
fn mock_vector(seed: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..512)
        .map(|i| ((i + seed) as f32 * 0.017_453_3).sin())
        .collect();
    l2_normalize(&mut v);
    v
}

fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
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
    fn test_encode_text_shape() {
        let v = ClipEncoder::encode_text("hello").unwrap();
        assert_eq!(v.len(), 512);
    }

    #[test]
    fn test_encode_text_normalized() {
        let v = ClipEncoder::encode_text("hello world").unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "L2 norm should be ≈1.0, got {norm}");
    }

    #[test]
    fn test_encode_image_shape() {
        let v = ClipEncoder::encode_image(&fixture("sample.jpg")).unwrap();
        assert_eq!(v.len(), 512);
    }

    #[test]
    fn test_encode_image_normalized() {
        let v = ClipEncoder::encode_image(&fixture("sample.jpg")).unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "L2 norm should be ≈1.0, got {norm}");
    }

    #[test]
    fn test_encode_text_deterministic() {
        let v1 = ClipEncoder::encode_text("蚌埠住了").unwrap();
        let v2 = ClipEncoder::encode_text("蚌埠住了").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_encode_text_different_inputs() {
        let v1 = ClipEncoder::encode_text("hello").unwrap();
        let v2 = ClipEncoder::encode_text("goodbye").unwrap();
        let cosine: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        assert!(cosine < 1.0 - 1e-5, "different inputs should have cosine < 1.0, got {cosine}");
    }

    #[test]
    fn test_encode_image_nonexistent() {
        let result = ClipEncoder::encode_image("/nonexistent/path.jpg");
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "需要 models/clip_text.onnx 和 models/clip_image.onnx"]
    fn test_encode_text_real_model() {
        let v = ClipEncoder::encode_text("一只可爱的猫").unwrap();
        assert_eq!(v.len(), 512);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
}
