use once_cell::sync::OnceCell;
use std::path::Path;
use std::sync::Mutex;

static TEXT_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();
static IMAGE_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();

pub struct ClipEncoder;

impl ClipEncoder {
    /// 文本编码：tokenize → ONNX 推理 → L2 归一化 → 512 维向量
    /// 模型不存在时返回确定性 mock 向量（L2 归一化后的均匀向量）
    pub fn encode_text(text: &str) -> anyhow::Result<Vec<f32>> {
        let start = std::time::Instant::now();

        let result = match find_model(&[
            "clip_text.onnx",
            "vit-b-16.txt.fp32.onnx",
            "vit-b-16.txt.fp16.onnx",
        ]) {
            Some(model_path) => {
                let session =
                    TEXT_SESSION.get_or_try_init(|| load_session(&model_path).map(Mutex::new))?;
                run_text_inference(&mut session.lock().unwrap(), text)?
            }
            None => {
                tracing::debug!("clip: text model not found, using mock");
                let seed = text
                    .bytes()
                    .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                mock_vector(seed as usize)
            }
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

        let result = match find_model(&[
            "clip_image.onnx",
            "vit-b-16.img.fp32.onnx",
            "vit-b-16.img.fp16.onnx",
        ]) {
            Some(model_path) => {
                let session =
                    IMAGE_SESSION.get_or_try_init(|| load_session(&model_path).map(Mutex::new))?;
                run_image_inference(&mut session.lock().unwrap(), path)?
            }
            None => {
                tracing::debug!("clip: image model not found, using mock");
                let seed = image_path
                    .bytes()
                    .fold(0u64, |acc, b| acc.wrapping_add(b as u64));
                mock_vector(seed as usize)
            }
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

/// 按候选文件名顺序查找第一个存在的模型文件
fn find_model(candidates: &[&str]) -> Option<std::path::PathBuf> {
    let dir = model_dir();
    candidates
        .iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
}

fn load_session(path: &Path) -> anyhow::Result<ort::session::Session> {
    tracing::info!("clip: loading model {:?}", path);
    let session = ort::session::Session::builder()?.commit_from_file(path)?;
    Ok(session)
}

fn run_text_inference(session: &mut ort::session::Session, text: &str) -> anyhow::Result<Vec<f32>> {
    use crate::ml::tokenizer;
    use ort::value::Tensor;
    let tokens = tokenizer::tokenize(text); // Vec<i64>
    let seq_len = tokens.len();
    let tensor = Tensor::from_array(([1usize, seq_len], tokens.into_boxed_slice()))?;
    let outputs = session.run(ort::inputs!["text" => tensor])?;
    let (_, data) = outputs[0].try_extract_tensor::<f32>()?;
    let mut vec: Vec<f32> = data.to_vec();
    l2_normalize(&mut vec);
    Ok(vec)
}

fn run_image_inference(
    session: &mut ort::session::Session,
    image_path: &Path,
) -> anyhow::Result<Vec<f32>> {
    use ort::value::Tensor;
    // 1. 加载图像：短边缩放到 224，再中心裁剪 224×224（Chinese-CLIP 标准预处理）
    let img = crate::image_io::open_image(image_path)?.to_rgb8();
    let (orig_w, orig_h) = img.dimensions();
    let (new_w, new_h) = if orig_w <= orig_h {
        (
            224u32,
            (orig_h as f32 * 224.0 / orig_w as f32).round() as u32,
        )
    } else {
        (
            (orig_w as f32 * 224.0 / orig_h as f32).round() as u32,
            224u32,
        )
    };
    let img = image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Triangle);
    let x = (new_w - 224) / 2;
    let y = (new_h - 224) / 2;
    let img = image::imageops::crop_imm(&img, x, y, 224, 224).to_image();

    // 2. 归一化：CLIP 专用 mean/std（非 ImageNet），转换为 CHW f32
    const MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
    const STD: [f32; 3] = [0.26862954, 0.261_302_6, 0.275_777_1];
    let mut data = vec![0f32; 3 * 224 * 224];
    for (idx, pixel) in img.pixels().enumerate() {
        for c in 0..3 {
            data[c * 224 * 224 + idx] = (pixel[c] as f32 / 255.0 - MEAN[c]) / STD[c];
        }
    }

    // 3. shape [1, 3, 224, 224]
    let tensor = Tensor::from_array(([1usize, 3, 224, 224], data.into_boxed_slice()))?;
    let outputs = session.run(ort::inputs!["image" => tensor])?;
    let (_, data) = outputs[0].try_extract_tensor::<f32>()?;
    let mut vec: Vec<f32> = data.to_vec();
    l2_normalize(&mut vec);
    Ok(vec)
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
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "L2 norm should be ≈1.0, got {norm}"
        );
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
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "L2 norm should be ≈1.0, got {norm}"
        );
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
        assert!(
            cosine < 1.0 - 1e-5,
            "different inputs should have cosine < 1.0, got {cosine}"
        );
    }

    #[test]
    fn test_encode_image_nonexistent() {
        let result = ClipEncoder::encode_image("/nonexistent/path.jpg");
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_text_real_model() {
        if find_model(&[
            "clip_text.onnx",
            "vit-b-16.txt.fp32.onnx",
            "vit-b-16.txt.fp16.onnx",
        ])
        .is_none()
        {
            eprintln!("跳过：找不到 CLIP 文本模型（vit-b-16.txt.fp32.onnx 等）");
            return;
        }
        let v = ClipEncoder::encode_text("一只可爱的猫").unwrap();
        assert_eq!(v.len(), 512);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
}
