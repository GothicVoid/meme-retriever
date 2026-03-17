use tauri::State;
use crate::db::DbPool;

#[derive(serde::Serialize)]
pub struct SearchResult {
    pub id: String,
    pub file_path: String,
    pub thumbnail_path: String,
    pub score: f32,
    pub tags: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ImageMeta {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub thumbnail_path: String,
    pub width: i64,
    pub height: i64,
    pub added_at: i64,
    pub use_count: i64,
    pub tags: Vec<String>,
}

#[tauri::command]
pub async fn search(
    query: String,
    limit: usize,
    _db: State<'_, DbPool>,
) -> Result<Vec<SearchResult>, String> {
    tracing::info!("search: query={query}, limit={limit}");
    Ok(vec![])
}

#[tauri::command]
pub async fn add_images(
    paths: Vec<String>,
    _db: State<'_, DbPool>,
) -> Result<(), String> {
    tracing::info!("add_images: {} files", paths.len());
    Ok(())
}

#[tauri::command]
pub async fn delete_image(id: String, _db: State<'_, DbPool>) -> Result<(), String> {
    tracing::info!("delete_image: {id}");
    Ok(())
}

#[tauri::command]
pub async fn get_images(page: i64, _db: State<'_, DbPool>) -> Result<Vec<ImageMeta>, String> {
    tracing::info!("get_images: page={page}");
    Ok(vec![])
}

#[tauri::command]
pub async fn update_tags(
    image_id: String,
    tags: Vec<String>,
    _db: State<'_, DbPool>,
) -> Result<(), String> {
    tracing::info!("update_tags: image={image_id}, tags={tags:?}");
    Ok(())
}

#[tauri::command]
pub async fn get_tag_suggestions(
    prefix: String,
    _db: State<'_, DbPool>,
) -> Result<Vec<String>, String> {
    tracing::info!("get_tag_suggestions: prefix={prefix}");
    Ok(vec![])
}

#[tauri::command]
pub async fn copy_to_clipboard(id: String) -> Result<(), String> {
    tracing::info!("copy_to_clipboard: {id}");
    Ok(())
}

#[tauri::command]
pub async fn reveal_in_finder(id: String, _db: State<'_, DbPool>) -> Result<(), String> {
    tracing::info!("reveal_in_finder: {id}");
    Ok(())
}

#[tauri::command]
pub async fn increment_use_count(id: String, _db: State<'_, DbPool>) -> Result<(), String> {
    tracing::info!("increment_use_count: {id}");
    Ok(())
}
