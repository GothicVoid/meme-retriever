use image::GenericImageView;
use std::path::Path;

pub fn open_image(path: &Path) -> anyhow::Result<image::DynamicImage> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.decode()?)
}

pub fn image_dimensions(path: &Path) -> anyhow::Result<(u32, u32)> {
    let img = open_image(path)?;
    Ok(img.dimensions())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn write_png_with_wrong_extension(path: &Path) {
        ImageBuffer::from_pixel(2, 3, Rgba([12u8, 34u8, 56u8, 255u8]))
            .save_with_format(path, image::ImageFormat::Png)
            .unwrap();
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
}
