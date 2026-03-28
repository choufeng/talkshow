use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const DEFAULT_RECORDING_SHORTCUT: &str = "Control+Backslash";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub shortcut: String,
    pub recording_shortcut: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            recording_shortcut: DEFAULT_RECORDING_SHORTCUT.to_string(),
        }
    }
}

pub fn config_file_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(CONFIG_FILE_NAME)
}

pub fn load_config(app_data_dir: &PathBuf) -> AppConfig {
    let path = config_file_path(app_data_dir);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    } else {
        let config = AppConfig::default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&path, content);
        }
        config
    }
}

pub fn save_config(app_data_dir: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}
