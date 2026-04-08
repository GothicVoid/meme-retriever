use std::path::Path;

pub fn generate(src: &Path, dst: &Path, size: u32) -> anyhow::Result<()> {
    tracing::debug!("thumbnail: {:?} -> {:?} ({}px)", src, dst, size);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let img = image::open(src)?;
    img.thumbnail(size, size).save(dst)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn test_generate_jpeg() {
        let src = fixture("sample.jpg");
        let dst = std::env::temp_dir().join("thumb_test_jpeg.jpg");
        generate(&src, &dst, 150).unwrap();
        assert!(dst.exists());
        let img = image::open(&dst).unwrap();
        assert!(img.width() <= 150);
        assert!(img.height() <= 150);
    }

    #[test]
    fn test_generate_png() {
        // sample.jpg を PNG として保存してテスト
        let src = fixture("sample.jpg");
        let dst = std::env::temp_dir().join("thumb_test_png.png");
        generate(&src, &dst, 150).unwrap();
        assert!(dst.exists());
        let img = image::open(&dst).unwrap();
        assert!(img.width() <= 150);
        assert!(img.height() <= 150);
    }

    #[test]
    fn test_generate_preserves_aspect_ratio() {
        let src = fixture("sample_wide.jpg"); // 800×400
        let dst = std::env::temp_dir().join("thumb_test_wide.jpg");
        generate(&src, &dst, 150).unwrap();
        let img = image::open(&dst).unwrap();
        // 800×400 缩到 150px，宽=150，高=75
        assert_eq!(img.width(), 150);
        assert_eq!(img.height(), 75);
    }

    #[test]
    fn test_generate_nonexistent_src() {
        let src = PathBuf::from("/nonexistent/path/image.jpg");
        let dst = std::env::temp_dir().join("thumb_test_nonexistent.jpg");
        assert!(generate(&src, &dst, 150).is_err());
    }

    #[test]
    fn test_generate_creates_parent_dir() {
        let src = fixture("sample.jpg");
        let dst = std::env::temp_dir()
            .join("thumb_test_subdir_xyz")
            .join("thumb.jpg");
        generate(&src, &dst, 150).unwrap();
        assert!(dst.exists());
    }
}
