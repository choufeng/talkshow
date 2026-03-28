use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const DEFAULT_RECORDING_SHORTCUT: &str = "Control+Backslash";
const CONFIG_FILE_NAME: &str = "config.json";

const DEFAULT_VERTEX_ENDPOINT: &str = "https://aiplatform.googleapis.com/v1";
const DEFAULT_DASHSCOPE_ENDPOINT: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct VertexConfig {
    pub endpoint: String,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DashScopeConfig {
    pub api_key: String,
    pub endpoint: String,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AiConfig {
    pub vertex: VertexConfig,
    pub dashscope: DashScopeConfig,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TranscriptionConfig {
    pub provider: String,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub shortcut: String,
    pub recording_shortcut: String,
    pub ai: AiConfig,
    pub features: FeaturesConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            recording_shortcut: DEFAULT_RECORDING_SHORTCUT.to_string(),
            ai: AiConfig {
                vertex: VertexConfig {
                    endpoint: DEFAULT_VERTEX_ENDPOINT.to_string(),
                    models: vec!["gemini-2.0-flash".to_string()],
                },
                dashscope: DashScopeConfig {
                    api_key: String::new(),
                    endpoint: DEFAULT_DASHSCOPE_ENDPOINT.to_string(),
                    models: vec!["qwen2-audio-instruct".to_string()],
                },
            },
            features: FeaturesConfig {
                transcription: TranscriptionConfig {
                    provider: "vertex".to_string(),
                    model: "gemini-2.0-flash".to_string(),
                },
            },
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
