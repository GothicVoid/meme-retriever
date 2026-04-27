use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use tauri::Manager;

static MODEL_DIR: OnceCell<PathBuf> = OnceCell::new();

pub fn init_model_dir(app: &tauri::AppHandle) {
    let resolved = resolve_model_dir_with_app(app).or_else(resolve_model_dir_without_app);
    if let Some(path) = resolved {
        let _ = MODEL_DIR.set(path.clone());
        tracing::info!("runtime_paths: using model dir {:?}", path);
    } else {
        tracing::warn!("runtime_paths: model dir not found, runtime will use degraded fallbacks");
    }
}

pub fn model_dir() -> PathBuf {
    MODEL_DIR
        .get_or_init(|| {
            resolve_model_dir_without_app().unwrap_or_else(|| PathBuf::from("./models"))
        })
        .clone()
}

pub fn default_kb_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("knowledge_base.json")
}

pub fn runtime_kb_path(app_data_dir: &Path) -> PathBuf {
    if cfg!(debug_assertions) {
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|dir| dir.join("app_data").join("knowledge_base.json"));
        if let Some(path) = repo_path.filter(|path| path.exists()) {
            return path;
        }
    }
    default_kb_path(app_data_dir)
}

fn resolve_model_dir_with_app(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("CLIP_MODEL_DIR") {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Some(path);
        }
    }

    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("models"))
        .filter(|path| path.exists())
}

fn resolve_model_dir_without_app() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok();
    let candidates = [
        std::env::var("CLIP_MODEL_DIR").ok().map(PathBuf::from),
        cwd.as_ref().map(|dir| dir.join("models")),
        cwd.as_ref().map(|dir| dir.join("src-tauri").join("models")),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("models"))),
    ];

    candidates.into_iter().flatten().find(|path| path.exists())
}
