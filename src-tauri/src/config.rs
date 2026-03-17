use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub default_limit: usize,
    pub delete_original_file: bool,
    pub library_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_limit: 9,
            delete_original_file: false,
            library_path: String::new(),
        }
    }
}
