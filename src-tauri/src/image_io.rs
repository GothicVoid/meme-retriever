use image::AnimationDecoder;
use image::GenericImageView;
use image::ImageFormat;
use image::RgbImage;
use std::io::BufReader;
use std::path::Path;

pub struct IndexFrameSet {
    pub thumbnail_image: image::DynamicImage,
    pub sampled_frames: Vec<RgbImage>,
    pub width: u32,
    pub height: u32,
    pub used_multi_frame_sampling: bool,
}

pub fn open_image(path: &Path) -> anyhow::Result<image::DynamicImage> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.decode()?)
}

pub fn image_dimensions(path: &Path) -> anyhow::Result<(u32, u32)> {
    let img = open_image(path)?;
    Ok(img.dimensions())
}

pub fn load_index_frame_set(path: &Path) -> anyhow::Result<IndexFrameSet> {
    load_index_frame_set_with_sampler(path, decode_gif_index_frame_set)
}

fn load_index_frame_set_with_sampler<F>(
    path: &Path,
    gif_sampler: F,
) -> anyhow::Result<IndexFrameSet>
where
    F: FnOnce(&Path) -> anyhow::Result<IndexFrameSet>,
{
    if guessed_image_format(path)? == Some(ImageFormat::Gif) {
        match gif_sampler(path) {
            Ok(frame_set) => return Ok(frame_set),
            Err(err) => {
                tracing::warn!(
                    "gif multi-frame sampling failed for {:?}, falling back to single-frame decode: {err}",
                    path
                );
            }
        }
    }

    let decoded = open_image(path)?;
    let width = decoded.width();
    let height = decoded.height();
    let sampled_frames = vec![decoded.to_rgb8()];
    Ok(IndexFrameSet {
        thumbnail_image: decoded,
        sampled_frames,
        width,
        height,
        used_multi_frame_sampling: false,
    })
}

fn guessed_image_format(path: &Path) -> anyhow::Result<Option<ImageFormat>> {
    Ok(image::ImageReader::open(path)?
        .with_guessed_format()?
        .format())
}

fn decode_gif_index_frame_set(path: &Path) -> anyhow::Result<IndexFrameSet> {
    let file = std::fs::File::open(path)?;
    let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(file))?;
    let frames = decoder.into_frames().collect_frames()?;
    anyhow::ensure!(!frames.is_empty(), "gif has no frames");

    let indices = select_gif_sample_indices(frames.len());
    let first_frame = image::DynamicImage::ImageRgba8(frames[0].buffer().clone());
    let width = first_frame.width();
    let height = first_frame.height();
    let sampled_frames = indices
        .into_iter()
        .map(|idx| image::DynamicImage::ImageRgba8(frames[idx].buffer().clone()).to_rgb8())
        .collect::<Vec<_>>();

    Ok(IndexFrameSet {
        thumbnail_image: first_frame,
        sampled_frames,
        width,
        height,
        used_multi_frame_sampling: true,
    })
}

pub(crate) fn select_gif_sample_indices(frame_count: usize) -> Vec<usize> {
    if frame_count == 0 {
        return Vec::new();
    }

    let candidates = [0, frame_count / 2, frame_count - 1];
    let mut indices = Vec::with_capacity(3);
    for idx in candidates {
        if indices.last() == Some(&idx) || indices.contains(&idx) {
            continue;
        }
        indices.push(idx);
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, ImageBuffer, Rgba, RgbaImage};

    fn write_png_with_wrong_extension(path: &Path) {
        ImageBuffer::from_pixel(2, 3, Rgba([12u8, 34u8, 56u8, 255u8]))
            .save_with_format(path, image::ImageFormat::Png)
            .unwrap();
    }

    fn write_test_gif(path: &Path, colors: &[[u8; 3]]) {
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = GifEncoder::new(file);
        encoder.set_repeat(Repeat::Infinite).unwrap();
        for color in colors {
            let frame = image::Frame::from_parts(
                RgbaImage::from_pixel(2, 2, Rgba([color[0], color[1], color[2], 255])),
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            );
            encoder.encode_frame(frame).unwrap();
        }
    }

    #[test]
    fn open_image_supports_mismatched_extension() {
        let path = std::env::temp_dir().join("codex_misnamed_png_as_jpg.jpg");
        write_png_with_wrong_extension(&path);

        let img = open_image(&path).unwrap();
        assert_eq!(img.dimensions(), (2, 3));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn image_dimensions_supports_mismatched_extension() {
        let path = std::env::temp_dir().join("codex_misnamed_png_dimensions.jpg");
        write_png_with_wrong_extension(&path);

        let dims = image_dimensions(&path).unwrap();
        assert_eq!(dims, (2, 3));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn select_gif_sample_indices_deduplicates_short_gifs() {
        assert_eq!(select_gif_sample_indices(0), Vec::<usize>::new());
        assert_eq!(select_gif_sample_indices(1), vec![0]);
        assert_eq!(select_gif_sample_indices(2), vec![0, 1]);
        assert_eq!(select_gif_sample_indices(3), vec![0, 1, 2]);
        assert_eq!(select_gif_sample_indices(5), vec![0, 2, 4]);
    }

    #[test]
    fn load_index_frame_set_samples_first_middle_last_for_gif() {
        let path = std::env::temp_dir().join("codex_index_frame_set.gif");
        write_test_gif(
            &path,
            &[[10, 0, 0], [40, 0, 0], [90, 0, 0], [140, 0, 0], [200, 0, 0]],
        );

        let frame_set = load_index_frame_set(&path).unwrap();

        assert!(frame_set.used_multi_frame_sampling);
        assert_eq!(frame_set.sampled_frames.len(), 3);
        assert_eq!(frame_set.sampled_frames[0].get_pixel(0, 0).0, [10, 0, 0]);
        assert_eq!(frame_set.sampled_frames[1].get_pixel(0, 0).0, [90, 0, 0]);
        assert_eq!(frame_set.sampled_frames[2].get_pixel(0, 0).0, [200, 0, 0]);
        assert_eq!(
            frame_set.thumbnail_image.to_rgb8().get_pixel(0, 0).0,
            [10, 0, 0]
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_index_frame_set_falls_back_to_single_decode_when_gif_sampler_fails() {
        let path = std::env::temp_dir().join("codex_index_frame_set_fallback.gif");
        write_test_gif(&path, &[[55, 66, 77], [88, 99, 111]]);

        let frame_set =
            load_index_frame_set_with_sampler(&path, |_path| anyhow::bail!("forced failure"))
                .unwrap();

        assert!(!frame_set.used_multi_frame_sampling);
        assert_eq!(frame_set.sampled_frames.len(), 1);
        assert_eq!(frame_set.sampled_frames[0].get_pixel(0, 0).0, [55, 66, 77]);

        let _ = std::fs::remove_file(path);
    }
}
