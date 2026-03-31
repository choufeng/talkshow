use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const DEFAULT_RECORDING_SHORTCUT: &str = "Control+Backslash";
const DEFAULT_TRANSLATE_SHORTCUT: &str = "Control+Shift+T";
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
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub translation: TranslationConfig,
    pub skills: SkillsConfig,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub shortcut: String,
    pub recording_shortcut: String,
    pub translate_shortcut: String,
    pub ai: AiConfig,
    pub features: FeaturesConfig,
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

fn migrate_builtin_skills(value: &mut serde_json::Value) {
    if let Some(skills) = value
        .get_mut("features")
        .and_then(|f| f.get_mut("skills"))
        .and_then(|s| s.get_mut("skills"))
        .and_then(|s| s.as_array_mut())
    {
        let default_skills = SkillsConfig::default().skills;
        for skill in skills.iter_mut() {
            if let Some(id) = skill.get("id").and_then(|v| v.as_str()) {
                if let Some(builtin) = skill.get("builtin").and_then(|v| v.as_bool()) {
                    if builtin {
                        if let Some(editable) = skill.get("editable").and_then(|v| v.as_bool()) {
                            if editable {
                                continue;
                            }
                        }
                        if let Some(default) = default_skills.iter().find(|s| s.id == id) {
                            if let Some(current_prompt) =
                                skill.get("prompt").and_then(|v| v.as_str())
                            {
                                if current_prompt != default.prompt {
                                    *skill.get_mut("prompt").unwrap() =
                                        serde_json::json!(default.prompt);
                                }
                            }
                        }
                    }
                }
            }
        }

        let builtin_ids: std::collections::HashSet<String> = skills
            .iter()
            .filter_map(|s| {
                if s.get("builtin").and_then(|v| v.as_bool()).unwrap_or(false) {
                    s.get("id").and_then(|v| v.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect();

        for default in &default_skills {
            if !builtin_ids.contains(&default.id) {
                skills.push(serde_json::to_value(default).unwrap_or_default());
            }
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
                migrate_builtin_skills(&mut raw);

                // 数据迁移：将 skills.provider_id/model 迁移到 transcription.polish_*
                let migration_target = if let Some(features) = raw.get_mut("features") {
                    if let Some(skills) = features.get_mut("skills") {
                        let provider_id = skills
                            .get("provider_id")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from);
                        let model = skills
                            .get("model")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        Some((provider_id, model))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((Some(provider_id), model)) = migration_target {
                    if let Some(features) = raw.get_mut("features") {
                        if let Some(transcription) = features.get_mut("transcription") {
                            if let Some(polish) = transcription.get_mut("polish_provider_id") {
                                *polish = serde_json::json!(provider_id);
                            }
                            if let Some(polish) = transcription.get_mut("polish_model") {
                                if let Some(model) = model {
                                    *polish = serde_json::json!(model);
                                }
                            }
                        }
                    }
                }

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
