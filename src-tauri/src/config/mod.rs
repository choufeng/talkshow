use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod io;
pub use io::{load_config, save_config, validate_config};

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const DEFAULT_RECORDING_SHORTCUT: &str = "Control+Backslash";
const DEFAULT_TRANSLATE_SHORTCUT: &str = "Control+Shift+T";
const CONFIG_FILE_NAME: &str = "config.json";

fn builtin_providers() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            api_key: Some(String::new()),
            endpoint: Some("https://api.openai.com/v1".to_string()),
            models: vec![ModelConfig {
                name: "gpt-4o-transcribe".to_string(),
                capabilities: vec!["transcription".to_string(), "chat".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "dashscope".to_string(),
            name: "阿里云".to_string(),
            api_key: Some(String::new()),
            endpoint: None,
            models: vec![ModelConfig {
                name: "qwen2-audio-instruct".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "vertex".to_string(),
            name: "Vertex AI".to_string(),
            api_key: None,
            endpoint: None,
            models: vec![ModelConfig {
                name: "gemini-2.0-flash".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "sensevoice".to_string(),
            name: "SenseVoice (本地)".to_string(),
            api_key: None,
            endpoint: None,
            models: vec![ModelConfig {
                name: "SenseVoice-Small".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            }],
        },
        ProviderConfig {
            id: "zhipu".to_string(),
            name: "智谱 AI".to_string(),
            api_key: Some(String::new()),
            endpoint: Some("https://open.bigmodel.cn/api/paas/v4".to_string()),
            models: vec![
                ModelConfig {
                    name: "glm-4.7-flash".to_string(),
                    capabilities: vec!["chat".to_string()],
                    verified: None,
                },
                ModelConfig {
                    name: "glm-4.7".to_string(),
                    capabilities: vec!["chat".to_string()],
                    verified: None,
                },
                ModelConfig {
                    name: "glm-5".to_string(),
                    capabilities: vec!["chat".to_string()],
                    verified: None,
                },
            ],
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
        if let Some(builtin) = builtin_map.get(&provider.id)
            && builtin_ids.contains(&provider.id)
        {
            provider.name = builtin.name.clone();
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
    pub name: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
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
    pub polish_enabled: bool,
    pub polish_provider_id: String,
    pub polish_model: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct TranslationConfig {
    pub target_lang: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub builtin: bool,
    pub editable: bool,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SkillsConfig {
    pub enabled: bool,
    pub skills: Vec<Skill>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        SkillsConfig {
            enabled: true,
            skills: vec![
                Skill {
                    id: "builtin-fillers".to_string(),
                    name: "语气词剔除".to_string(),
                    description: "去除嗯、啊、那个、就是等无意义口头语气词".to_string(),
                    prompt: "去除中文口语中常见的无意义语气词和填充词，包括但不限于：\n\"嗯\"、\"啊\"、\"额\"、\"呃\"、\"那个\"、\"就是\"、\"然后\"、\"对吧\"、\"的话\"、\"怎么说呢\"。\n注意保留有实际语义的词语，例如\"然后\"在表示时间顺序时应保留。不要改变原文的语义和语气。".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: true,
                },
                Skill {
                    id: "builtin-typos".to_string(),
                    name: "错别字修正".to_string(),
                    description: "修正错别字、同音错误和输入法错误".to_string(),
                    prompt: "识别并修正文本中的错别字、同音错误和常见输入法导致的文字错误。\n只修正明确的错误，不要对有歧义的内容做主观改动。\n常见的同音错误示例：\"的/地/得\"、\"做/作\"、\"在/再\"、\"已/以\"、\"即/既\"。".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: true,
                },
                Skill {
                    id: "builtin-polish".to_string(),
                    name: "口语润色".to_string(),
                    description: "保持口语化风格，使表达更流畅自然".to_string(),
                    prompt: "保持口语化的表达风格，但使语句更流畅自然。\n具体做法：去除重复表达、调整语序使其更通顺、适当添加标点使句子结构更清晰。\n不要改变原文的口语化特征，不要转换为书面语。".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: false,
                },
                Skill {
                    id: "builtin-formal".to_string(),
                    name: "书面格式化".to_string(),
                    description: "口语转书面表达，适合邮件和文档场景".to_string(),
                    prompt: "将口语化的表达转换为规范的书面表达，适合邮件、文档、报告等正式场景。\n\n具体做法：\n- 词汇替换：将口语化词汇替换为正式表达（如\u{201c}搞定了\u{201d}→\u{201c}已完成\u{201d}）\n- 列表结构化：将\u{201c}第一/第二/第三\u{201d}、\u{201c}首先/其次/最后\u{201d}、\u{201c}一二三\u{201d}等序列词转换为规范的有序列表格式\n- 段落重组：识别话题转换，合理分段；将碎片化短句合并为完整句子\n- 标点规范：统一使用全角标点，消除重复标点，合理使用冒号、分号等结构化标点\n- 句子结构：调整语序使其符合书面语法，消除冗余和重复表达\n- 层级关系：识别\u{201c}总-分\u{201d}、因果、递进等逻辑关系，用合适的连接词明确表达\n\n约束：\n- 保持原文的完整语义，不添加或删除信息\n- 输出纯文本，可使用 Markdown 列表格式\n- 不要添加解释性文字".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: false,
                },
                Skill {
                    id: "builtin-translation".to_string(),
                    name: "翻译优化".to_string(),
                    description: "自定义翻译规则，如术语、风格和行业特定要求".to_string(),
                    prompt: "保持原文的语气和风格。确保技术术语翻译准确。如果某个术语没有标准翻译，保留原文。".to_string(),
                    builtin: true,
                    editable: true,
                    enabled: true,
                },
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct RecordingFeaturesConfig {
    pub auto_mute: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub skills: SkillsConfig,
    pub recording: RecordingFeaturesConfig,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub shortcut: String,
    pub recording_shortcut: String,
    pub translate_shortcut: String,
    pub ai: AiConfig,
    pub features: FeaturesConfig,
    pub onboarding_completed: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            recording_shortcut: DEFAULT_RECORDING_SHORTCUT.to_string(),
            translate_shortcut: DEFAULT_TRANSLATE_SHORTCUT.to_string(),
            ai: AiConfig {
                providers: builtin_providers(),
            },
            features: FeaturesConfig {
                transcription: TranscriptionConfig {
                    provider_id: "vertex".to_string(),
                    model: "gemini-2.0-flash".to_string(),
                    polish_enabled: true,
                    polish_provider_id: String::new(),
                    polish_model: String::new(),
                },
                translation: TranslationConfig {
                    target_lang: "English".to_string(),
                },
                skills: SkillsConfig::default(),
                recording: RecordingFeaturesConfig::default(),
            },
            onboarding_completed: false,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_models_removes_duplicate_and_merges_capabilities() {
        let mut models = vec![
            ModelConfig {
                name: "model-a".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            },
            ModelConfig {
                name: "model-a".to_string(),
                capabilities: vec!["chat".to_string()],
                verified: None,
            },
            ModelConfig {
                name: "model-b".to_string(),
                capabilities: vec!["transcription".to_string()],
                verified: None,
            },
        ];
        dedup_models(&mut models);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "model-a");
        assert!(
            models[0]
                .capabilities
                .contains(&"transcription".to_string())
        );
        assert!(models[0].capabilities.contains(&"chat".to_string()));
        assert_eq!(models[1].name, "model-b");
    }

    #[test]
    fn test_dedup_models_no_duplicates() {
        let mut models = vec![
            ModelConfig {
                name: "a".to_string(),
                capabilities: vec!["t".to_string()],
                verified: None,
            },
            ModelConfig {
                name: "b".to_string(),
                capabilities: vec!["c".to_string()],
                verified: None,
            },
        ];
        dedup_models(&mut models);
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_dedup_models_empty() {
        let mut models: Vec<ModelConfig> = vec![];
        dedup_models(&mut models);
        assert!(models.is_empty());
    }

    #[test]
    fn test_merge_builtin_providers_adds_missing() {
        let providers = vec![ProviderConfig {
            id: "custom".to_string(),
            name: "Custom".to_string(),
            api_key: Some("key".to_string()),
            models: vec![],
            endpoint: None,
        }];
        let result = merge_builtin_providers(providers);
        let ids: Vec<&str> = result.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"vertex"));
        assert!(ids.contains(&"dashscope"));
        assert!(ids.contains(&"sensevoice"));
        assert!(ids.contains(&"custom"));
    }

    #[test]
    fn test_merge_builtin_providers_corrects_existing() {
        let providers = vec![ProviderConfig {
            id: "dashscope".to_string(),
            name: "阿里云".to_string(),
            api_key: Some("key".to_string()),
            models: vec![],
            endpoint: None,
        }];
        let result = merge_builtin_providers(providers);
        let dashscope = result.iter().find(|p| p.id == "dashscope").unwrap();
        assert_eq!(dashscope.name, "阿里云");
    }

    #[test]
    fn test_load_and_save_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let app_data_dir = dir.path().to_path_buf();

        let mut config = load_config(&app_data_dir);
        assert_eq!(config.shortcut, DEFAULT_SHORTCUT);
        assert!(config.ai.providers.len() >= 3);

        config.ai.providers[0].api_key = Some("sk-roundtrip-test".to_string());
        save_config(&app_data_dir, &config).unwrap();

        let loaded = load_config(&app_data_dir);
        let p0 = loaded
            .ai
            .providers
            .iter()
            .find(|p| p.id == config.ai.providers[0].id)
            .unwrap();
        assert_eq!(p0.api_key, Some("sk-roundtrip-test".to_string()));
    }

    #[test]
    fn test_load_config_creates_default_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app_data_dir = dir.path().to_path_buf();
        let config = load_config(&app_data_dir);
        assert_eq!(config.shortcut, DEFAULT_SHORTCUT);
        assert!(app_data_dir.join("config.json").exists());
    }

    #[test]
    fn test_default_app_config() {
        let config = AppConfig::default();
        assert_eq!(config.shortcut, "Control+Shift+Quote");
        assert_eq!(config.features.transcription.provider_id, "vertex");
        assert!(config.features.skills.enabled);
        assert!(!config.features.recording.auto_mute);
    }

    #[test]
    fn test_validate_config_allows_valid_provider() {
        let mut config = AppConfig::default();
        config.ai.providers = vec![ProviderConfig {
            id: "vertex".to_string(),
            name: "Vertex AI".to_string(),
            api_key: None,
            models: vec![],
            endpoint: None,
        }];
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_rejects_empty_id() {
        let mut config = AppConfig::default();
        config.ai.providers = vec![ProviderConfig {
            id: "".to_string(),
            name: "Test".to_string(),
            api_key: None,
            models: vec![],
            endpoint: None,
        }];
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_rejects_too_long_shortcut() {
        let config = AppConfig {
            shortcut: "A".repeat(101),
            ..Default::default()
        };
        assert!(validate_config(&config).is_err());
    }
}
