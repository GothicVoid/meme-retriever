use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub default_limit: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_limit: 9,
        }
    }
}
