use crate::db::DbPool;

pub async fn index_images(pool: &DbPool, paths: Vec<String>) -> anyhow::Result<()> {
    for path in paths {
        // TODO: copy/ref file → thumbnail → OCR → CLIP encode → write db
        tracing::info!("indexing: {path}");
        let _ = pool;
    }
    Ok(())
}
