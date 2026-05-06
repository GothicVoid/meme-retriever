use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use tauri::Manager;

static MODEL_DIR: OnceCell<PathBuf> = OnceCell::new();
static LIB_DIR: OnceCell<PathBuf> = OnceCell::new();

pub fn init_model_dir(app: &tauri::AppHandle) {
    let resolved = resolve_model_dir_with_app(app).or_else(resolve_model_dir_without_app);
    if let Some(path) = resolved {
        let _ = MODEL_DIR.set(path.clone());
        tracing::info!("runtime_paths: using model dir {:?}", path);
    } else {
        tracing::warn!("runtime_paths: model dir not found, runtime will use degraded fallbacks");
    }
}

pub fn init_runtime_library_dir(app: &tauri::AppHandle) {
    let resolved = resolve_library_dir_with_app(app).or_else(resolve_library_dir_without_app);
    if let Some(path) = resolved {
        let _ = LIB_DIR.set(path.clone());
        let dylib_path = path.join(runtime_library_name());
        if dylib_path.exists() {
            std::env::set_var("ORT_DYLIB_PATH", &dylib_path);
            tracing::info!("runtime_paths: using runtime library {:?}", dylib_path);
        } else {
            tracing::warn!(
                "runtime_paths: runtime library dir found but {} is missing in {:?}",
                runtime_library_name(),
                path
            );
        }
    } else {
        tracing::warn!("runtime_paths: runtime library dir not found");
    }
}

pub fn model_dir() -> PathBuf {
    MODEL_DIR
        .get_or_init(|| {
            resolve_model_dir_without_app().unwrap_or_else(|| PathBuf::from("./models"))
        })
        .clone()
}

pub fn library_dir() -> PathBuf {
    LIB_DIR
        .get_or_init(|| resolve_library_dir_without_app().unwrap_or_else(|| PathBuf::from("./libs")))
        .clone()
}

pub fn default_kb_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("knowledge_base.json")
}

pub fn runtime_kb_path(app_data_dir: &Path) -> PathBuf {
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

fn resolve_library_dir_with_app(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("ORT_DYLIB_PATH") {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return path.parent().map(Path::to_path_buf);
        }
    }

    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("libs"))
        .filter(|path| path.exists())
}

fn resolve_library_dir_without_app() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok();
    let candidates = [
        std::env::var("ORT_DYLIB_PATH")
            .ok()
            .map(PathBuf::from)
            .and_then(|path| path.parent().map(Path::to_path_buf)),
        cwd.as_ref().map(|dir| dir.join("libs")),
        cwd.as_ref().map(|dir| dir.join("src-tauri").join("libs")),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("libs"))),
    ];

    candidates.into_iter().flatten().find(|path| path.exists())
}

fn runtime_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    }
}
