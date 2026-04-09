use sha2::{Digest, Sha256};
use std::path::Path;

pub fn compute_sha256(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn test_compute_sha256_consistent() {
        let path = fixture("sample.jpg");
        let h1 = compute_sha256(&path).unwrap();
        let h2 = compute_sha256(&path).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_compute_sha256_different_files() {
        let h1 = compute_sha256(&fixture("sample.jpg")).unwrap();
        let h2 = compute_sha256(&fixture("sample_blank.jpg")).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_compute_sha256_nonexistent() {
        let result = compute_sha256(Path::new("/nonexistent/file.jpg"));
        assert!(result.is_err());
    }
}
