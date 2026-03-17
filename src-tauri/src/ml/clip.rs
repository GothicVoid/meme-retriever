use once_cell::sync::OnceCell;

static _TEXT_SESSION: OnceCell<()> = OnceCell::new();
static _IMAGE_SESSION: OnceCell<()> = OnceCell::new();

pub struct ClipEncoder;

impl ClipEncoder {
    /// Encode text query → 512-dim f32 vector.
    pub fn encode_text(_text: &str) -> anyhow::Result<Vec<f32>> {
        // TODO: load clip_text.onnx via ort, tokenize, run inference, L2-normalize
        Ok(vec![0.0f32; 512])
    }

    /// Encode image → 512-dim f32 vector.
    pub fn encode_image(_image_path: &str) -> anyhow::Result<Vec<f32>> {
        // TODO: load clip_image.onnx via ort, preprocess, run inference, L2-normalize
        Ok(vec![0.0f32; 512])
    }
}
