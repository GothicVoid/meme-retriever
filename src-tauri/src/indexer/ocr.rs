use once_cell::sync::OnceCell;
use std::path::Path;
use std::sync::Mutex;

static DET_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();
static REC_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();

/// 从图片中提取文字。
/// 需要 `models/ch_PP-OCRv4_det_infer.onnx`、`models/ch_PP-OCRv4_rec_infer.onnx`
/// 和 `models/ppocr_keys_v1.txt`。
/// 当模型或字典文件不存在时返回空字符串（降级处理）。
pub fn extract_text(image_path: &str) -> anyhow::Result<String> {
    let path = Path::new(image_path);
    if !path.exists() {
        anyhow::bail!("image not found: {image_path}");
    }

    let det_path = match find_model(&["ch_PP-OCRv4_det_infer.onnx"]) {
        Some(p) => p,
        None => {
            tracing::debug!("ocr: det model not found, skipping");
            return Ok(String::new());
        }
    };

    let rec_path = match find_model(&["ocr.onnx", "ch_PP-OCRv4_rec_infer.onnx"]) {
        Some(p) => p,
        None => {
            tracing::debug!("ocr: rec model not found, skipping");
            return Ok(String::new());
        }
    };

    let dict_path = model_dir().join("ppocr_keys_v1.txt");
    if !dict_path.exists() {
        tracing::debug!("ocr: dict not found ({:?}), skipping", dict_path);
        return Ok(String::new());
    }

    let start = std::time::Instant::now();
    let text = run_pipeline(path, &det_path, &rec_path, &dict_path)?;
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
    Ok(content.lines().map(|l| l.trim().to_string()).collect())
}

// ── Det 预处理 ─────────────────────────────────────────────────────────────

const DET_LIMIT: u32 = 960;
const DET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const DET_STD:  [f32; 3] = [0.229, 0.224, 0.225];

/// 返回 (scale, padded_w, padded_h, chw_data)
fn preprocess_det(img: &image::RgbImage) -> (f32, u32, u32, Vec<f32>) {
    let (orig_w, orig_h) = img.dimensions();

    // 长边 ≤ DET_LIMIT，等比缩放
    let scale = (DET_LIMIT as f32 / orig_w.max(orig_h) as f32).min(1.0);
    let new_w = ((orig_w as f32 * scale) as u32).max(1);
    let new_h = ((orig_h as f32 * scale) as u32).max(1);

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Triangle);

    // Pad 到 32 的倍数
    let pad_w = new_w.div_ceil(32) * 32;
    let pad_h = new_h.div_ceil(32) * 32;

    // CHW，pad 区域填 0
    let mut data = vec![0f32; 3 * pad_h as usize * pad_w as usize];
    for y in 0..new_h as usize {
        for x in 0..new_w as usize {
            let px = resized.get_pixel(x as u32, y as u32);
            for c in 0..3usize {
                let norm = (px[c] as f32 / 255.0 - DET_MEAN[c]) / DET_STD[c];
                data[c * pad_h as usize * pad_w as usize + y * pad_w as usize + x] = norm;
            }
        }
    }
    (scale, pad_w, pad_h, data)
}

// ── Det 后处理（DBNet）──────────────────────────────────────────────────────

const DET_THRESH:     f32 = 0.3;
const BOX_THRESH:     f32 = 0.6;
const UNCLIP_RATIO:   f32 = 1.5;
const MIN_BOX_SIZE:   f32 = 3.0;

/// 返回原图坐标下的文字框列表，每个框为 [x0,y0,x1,y1]（水平轴对齐外接矩形）
fn dbnet_postprocess(
    prob: &[f32],
    prob_h: usize,
    prob_w: usize,
    scale: f32,
    orig_w: u32,
    orig_h: u32,
) -> Vec<[u32; 4]> {
    // 二值化
    let binary: Vec<bool> = prob.iter().map(|&v| v > DET_THRESH).collect();

    // 连通域标记（BFS）
    let mut label = vec![0i32; prob_h * prob_w];
    let mut component_id = 0i32;
    let mut components: Vec<Vec<usize>> = vec![vec![]]; // index 0 unused

    for start in 0..prob_h * prob_w {
        if !binary[start] || label[start] != 0 {
            continue;
        }
        component_id += 1;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        label[start] = component_id;
        let mut pixels = vec![start];

        while let Some(idx) = queue.pop_front() {
            let y = idx / prob_w;
            let x = idx % prob_w;
            for (dy, dx) in [(!0usize, 0usize), (1, 0), (0, !0), (0, 1)] {
                let ny = y.wrapping_add(dy);
                let nx = x.wrapping_add(dx);
                if ny < prob_h && nx < prob_w {
                    let ni = ny * prob_w + nx;
                    if binary[ni] && label[ni] == 0 {
                        label[ni] = component_id;
                        queue.push_back(ni);
                        pixels.push(ni);
                    }
                }
            }
        }
        components.push(pixels);
    }

    let mut boxes = Vec::new();

    for pixels in components.iter().skip(1) {
        if pixels.is_empty() { continue; }

        // 外接矩形（padded image 坐标）
        let xs: Vec<usize> = pixels.iter().map(|&i| i % prob_w).collect();
        let ys: Vec<usize> = pixels.iter().map(|&i| i / prob_w).collect();
        let x0 = *xs.iter().min().unwrap();
        let y0 = *ys.iter().min().unwrap();
        let x1 = *xs.iter().max().unwrap();
        let y1 = *ys.iter().max().unwrap();

        let bw = (x1 - x0 + 1) as f32;
        let bh = (y1 - y0 + 1) as f32;
        if bw < MIN_BOX_SIZE || bh < MIN_BOX_SIZE {
            continue;
        }

        // box score：prob_map 在外接矩形区域内的均值
        let mut sum = 0f32;
        let mut cnt = 0usize;
        for py in y0..=y1 {
            for px in x0..=x1 {
                sum += prob[py * prob_w + px];
                cnt += 1;
            }
        }
        let score = if cnt > 0 { sum / cnt as f32 } else { 0.0 };
        if score < BOX_THRESH {
            continue;
        }

        // Unclip：按面积/周长比例扩展外接矩形
        let area = bw * bh;
        let perimeter = 2.0 * (bw + bh);
        let dist = (area * UNCLIP_RATIO / perimeter) as u32;

        let ux0 = x0.saturating_sub(dist as usize);
        let uy0 = y0.saturating_sub(dist as usize);
        let ux1 = (x1 + dist as usize).min(prob_w - 1);
        let uy1 = (y1 + dist as usize).min(prob_h - 1);

        // 坐标映射回原图（除以缩放比，裁剪到原图边界）
        let map = |v: usize, max: u32| -> u32 {
            ((v as f32 / scale) as u32).min(max)
        };
        let rx0 = map(ux0, orig_w.saturating_sub(1));
        let ry0 = map(uy0, orig_h.saturating_sub(1));
        let rx1 = map(ux1, orig_w.saturating_sub(1));
        let ry1 = map(uy1, orig_h.saturating_sub(1));

        if rx1 > rx0 && ry1 > ry0 {
            boxes.push([rx0, ry0, rx1, ry1]);
        }
    }

    // 按 y 坐标排序（从上到下）
    boxes.sort_by_key(|b| b[1]);
    boxes
}

// ── Rec 推理 ────────────────────────────────────────────────────────────────

const REC_MEAN: f32 = 0.5;
const REC_STD:  f32 = 0.5;

fn run_rec(
    line_img: &image::RgbImage,
    rec_path: &Path,
    dict: &[String],
) -> anyhow::Result<String> {
    use ort::value::Tensor;

    let (orig_w, orig_h) = line_img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Ok(String::new());
    }

    const TARGET_H: u32 = 48;
    let target_w = ((orig_w as f32 * TARGET_H as f32 / orig_h as f32) as u32).clamp(1, 1920);
    let resized = image::imageops::resize(line_img, target_w, TARGET_H, image::imageops::FilterType::Triangle);
    let (w, h) = resized.dimensions();

    let mut data = vec![0f32; 3 * h as usize * w as usize];
    for (idx, pixel) in resized.pixels().enumerate() {
        for c in 0..3 {
            data[c * h as usize * w as usize + idx] =
                (pixel[c] as f32 / 255.0 - REC_MEAN) / REC_STD;
        }
    }

    let tensor = Tensor::from_array(([1usize, 3, h as usize, w as usize], data.into_boxed_slice()))?;

    let session = REC_SESSION.get_or_try_init(|| {
        tracing::info!("ocr: loading rec model {:?}", rec_path);
        ort::session::Session::builder()?.commit_from_file(rec_path)
            .map(Mutex::new)
            .map_err(|e| anyhow::anyhow!(e))
    })?;

    let mut guard = session.lock().unwrap();
    let outputs = guard.run(ort::inputs!["x" => tensor])?;
    let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;
    if shape.len() < 3 {
        anyhow::bail!("unexpected rec output shape: {:?}", shape);
    }
    let seq_len = shape[1] as usize;
    let num_classes = shape[2] as usize;

    let mut result = String::new();
    let mut prev_idx = 0usize;
    for t in 0..seq_len {
        let offset = t * num_classes;
        let frame = &data[offset..offset + num_classes];
        let best_idx = frame.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        if best_idx != 0 && best_idx != prev_idx {
            let char_idx = best_idx - 1;
            if char_idx < dict.len() {
                result.push_str(&dict[char_idx]);
            }
        }
        prev_idx = best_idx;
    }
    Ok(result)
}

// ── 主流水线 ────────────────────────────────────────────────────────────────

fn run_pipeline(
    image_path: &Path,
    det_path: &Path,
    rec_path: &Path,
    dict_path: &Path,
) -> anyhow::Result<String> {
    use ort::value::Tensor;

    let dict = load_dict(dict_path)?;
    let img = image::open(image_path)?.to_rgb8();
    let (orig_w, orig_h) = img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Ok(String::new());
    }

    // 1. Det 预处理
    let (scale, pad_w, pad_h, det_data) = preprocess_det(&img);
    let det_tensor = Tensor::from_array((
        [1usize, 3, pad_h as usize, pad_w as usize],
        det_data.into_boxed_slice(),
    ))?;

    // 2. Det 推理
    let det_session = DET_SESSION.get_or_try_init(|| {
        tracing::info!("ocr: loading det model {:?}", det_path);
        ort::session::Session::builder()?.commit_from_file(det_path)
            .map(Mutex::new)
            .map_err(|e| anyhow::anyhow!(e))
    })?;

    let prob_map: Vec<f32> = {
        let mut guard = det_session.lock().unwrap();
        let outputs = guard.run(ort::inputs!["x" => det_tensor])?;
        let (_, data) = outputs[0].try_extract_tensor::<f32>()?;
        data.to_vec()
    };

    // 3. DBNet 后处理 → 文字框列表
    let boxes = dbnet_postprocess(
        &prob_map,
        pad_h as usize,
        pad_w as usize,
        scale,
        orig_w,
        orig_h,
    );
    tracing::debug!("ocr: det found {} boxes", boxes.len());

    if boxes.is_empty() {
        return Ok(String::new());
    }

    // 4. 逐框裁剪 + Rec 推理
    let mut texts: Vec<String> = Vec::new();
    for [x0, y0, x1, y1] in &boxes {
        let crop = image::imageops::crop_imm(&img, *x0, *y0, x1 - x0, y1 - y0).to_image();
        let text = run_rec(&crop, rec_path, &dict)?;
        if !text.is_empty() {
            texts.push(text);
        }
    }

    Ok(texts.join(" "))
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
    }

    #[test]
    fn test_extract_text_nonexistent() {
        let result = extract_text("/nonexistent/path/image.jpg");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_extract_text_with_real_model() {
        if find_model(&["ch_PP-OCRv4_det_infer.onnx"]).is_none() {
            eprintln!("跳过：找不到 OCR det 模型（ch_PP-OCRv4_det_infer.onnx）");
            return;
        }
        if find_model(&["ocr.onnx", "ch_PP-OCRv4_rec_infer.onnx"]).is_none() {
            eprintln!("跳过：找不到 OCR rec 模型（ch_PP-OCRv4_rec_infer.onnx）");
            return;
        }
        // sample_text_line.png 是 320×48 单行文字条带
        let result = extract_text(&fixture("sample_text_line.png")).unwrap();
        assert!(!result.is_empty(), "OCR 应识别出文字条带中的文字，实际返回空字符串");
    }
}
