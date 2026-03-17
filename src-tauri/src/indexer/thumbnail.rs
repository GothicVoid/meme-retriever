use std::path::Path;

pub fn generate(_src: &Path, _dst: &Path, _size: u32) -> anyhow::Result<()> {
    // TODO: image::open(src)?.thumbnail(size, size).save(dst)?
    Ok(())
}
