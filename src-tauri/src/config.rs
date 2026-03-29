use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const DEFAULT_RECORDING_SHORTCUT: &str = "Control+Backslash";
const CONFIG_FILE_NAME: &str = "config.json";

fn builtin_providers() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            id: "vertex".to_string(),
            provider_type: "vertex".to_string(),
            name: "Vertex AI".to_string(),
            endpoint: String::new(),
            api_key: None,
            models: vec![ModelConfig {
                name: "gemini-2.0-flash".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "dashscope".to_string(),
            provider_type: "openai-compatible".to_string(),
            name: "阿里云".to_string(),
            endpoint: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            api_key: Some(String::new()),
            models: vec![ModelConfig {
                name: "qwen2-audio-instruct".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "sensevoice".to_string(),
            provider_type: "sensevoice".to_string(),
            name: "SenseVoice (本地)".to_string(),
            endpoint: String::new(),
            api_key: None,
            models: vec![ModelConfig {
                name: "SenseVoice-Small".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
    ]
}

fn merge_builtin_providers(mut providers: Vec<ProviderConfig>) -> Vec<ProviderConfig> {
    let builtins = builtin_providers();
    let builtin_map: std::collections::HashMap<String, ProviderConfig> =
        builtins.into_iter().map(|p| (p.id.clone(), p)).collect();

    let builtin_ids: std::collections::HashSet<String> = builtin_map.keys().cloned().collect();
    let user_ids: std::collections::HashSet<String> =
        providers.iter().map(|p| p.id.clone()).collect();

    let missing: Vec<ProviderConfig> = builtin_map
        .values()
        .filter(|p| !user_ids.contains(&p.id))
        .cloned()
        .collect();

    for provider in &mut providers {
        if let Some(builtin) = builtin_map.get(&provider.id) {
            if builtin_ids.contains(&provider.id) {
                provider.provider_type = builtin.provider_type.clone();
                provider.endpoint = builtin.endpoint.clone();
            }
        }
    }

    let mut result = missing;
    result.append(&mut providers);
    result
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModelVerified {
    pub status: String,
    pub tested_at: String,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModelConfig {
    pub name: String,
    pub capabilities: Vec<String>,
    pub verified: Option<ModelVerified>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ProviderConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AiConfig {
    pub providers: Vec<ProviderConfig>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TranscriptionConfig {
    pub provider_id: String,
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
                providers: builtin_providers(),
            },
            features: FeaturesConfig {
                transcription: TranscriptionConfig {
                    provider_id: "vertex".to_string(),
                    model: "gemini-2.0-flash".to_string(),
                },
            },
        }
    }
}

pub fn config_file_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(CONFIG_FILE_NAME)
}

fn dedup_models(models: &mut Vec<ModelConfig>) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < models.len() {
        if let Some(&prev) = seen.get(&models[i].name) {
            let caps_to_merge: Vec<String> = models[i]
                .capabilities
                .iter()
                .filter(|c| !models[prev].capabilities.contains(c))
                .cloned()
                .collect();
            models[prev].capabilities.extend(caps_to_merge);
            models.remove(i);
        } else {
            seen.insert(models[i].name.clone(), i);
            i += 1;
        }
    }
}

fn migrate_models(value: &mut serde_json::Value) {
    if let Some(providers) = value
        .get_mut("ai")
        .and_then(|ai| ai.get_mut("providers"))
        .and_then(|p| p.as_array_mut())
    {
        for provider in providers.iter_mut() {
            if let Some(models) = provider.get_mut("models") {
                if let Some(arr) = models.as_array_mut() {
                    let migrated: Vec<serde_json::Value> = arr
                        .drain(..)
                        .map(|m| {
                            if m.is_string() {
                                serde_json::json!({
                                    "name": m,
                                    "capabilities": []
                                })
                            } else {
                                m
                            }
                        })
                        .collect();
                    *arr = migrated;
                }
            }
        }
    }
}

pub fn load_config(app_data_dir: &PathBuf) -> AppConfig {
    let path = config_file_path(app_data_dir);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                let mut raw: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
                migrate_models(&mut raw);
                let mut config: AppConfig = serde_json::from_value(raw).unwrap_or_default();
                config.ai.providers = merge_builtin_providers(config.ai.providers);
                for provider in &mut config.ai.providers {
                    dedup_models(&mut provider.models);
                }
                config
            }
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
