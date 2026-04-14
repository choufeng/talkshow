use super::*;
use std::fs;

pub fn config_file_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(CONFIG_FILE_NAME)
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
            if let Some(id) = skill.get("id").and_then(|v| v.as_str())
                && let Some(builtin) = skill.get("builtin").and_then(|v| v.as_bool())
                && builtin
            {
                if let Some(editable) = skill.get("editable").and_then(|v| v.as_bool())
                    && editable
                {
                    continue;
                }
                if let Some(default) = default_skills.iter().find(|s| s.id == id)
                    && let Some(current_prompt) = skill.get("prompt").and_then(|v| v.as_str())
                    && current_prompt != default.prompt
                {
                    *skill.get_mut("prompt").unwrap() = serde_json::json!(default.prompt);
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
            if let Some(models) = provider.get_mut("models")
                && let Some(arr) = models.as_array_mut()
            {
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

pub fn load_config(app_data_dir: &std::path::Path) -> AppConfig {
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

                if let Some((Some(provider_id), model)) = migration_target
                    && let Some(features) = raw.get_mut("features")
                    && let Some(transcription) = features.get_mut("transcription")
                {
                    if let Some(polish) = transcription.get_mut("polish_provider_id") {
                        *polish = serde_json::json!(provider_id);
                    }
                    if let Some(polish) = transcription.get_mut("polish_model")
                        && let Some(model) = model
                    {
                        *polish = serde_json::json!(model);
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

pub fn save_config(app_data_dir: &std::path::Path, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn validate_config(config: &AppConfig) -> Result<(), String> {
    for provider in &config.ai.providers {
        if provider.id.trim().is_empty() {
            return Err("Provider ID cannot be empty".to_string());
        }

        if provider.name.trim().is_empty() {
            return Err(format!(
                "Provider name cannot be empty for '{}'",
                provider.id
            ));
        }
    }

    if config.shortcut.len() > 100 {
        return Err("Shortcut string too long".to_string());
    }
    if config.recording_shortcut.len() > 100 {
        return Err("Recording shortcut string too long".to_string());
    }
    if config.translate_shortcut.len() > 100 {
        return Err("Translate shortcut string too long".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_models_string_to_object() {
        let mut value = serde_json::json!({
            "ai": {
                "providers": [
                    {
                        "id": "test",
                        "type": "openai-compatible",
                        "name": "Test",
                        "endpoint": "",
                        "models": ["old-model-1", "old-model-2"]
                    }
                ]
            }
        });
        migrate_models(&mut value);
        let models = value["ai"]["providers"][0]["models"].as_array().unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0]["name"], "old-model-1");
        assert_eq!(models[0]["capabilities"], serde_json::json!([]));
        assert_eq!(models[1]["name"], "old-model-2");
    }

    #[test]
    fn test_migrate_models_object_unchanged() {
        let mut value = serde_json::json!({
            "ai": {
                "providers": [
                    {
                        "id": "test",
                        "type": "openai-compatible",
                        "name": "Test",
                        "endpoint": "",
                        "models": [{"name": "model-a", "capabilities": ["transcription"]}]
                    }
                ]
            }
        });
        migrate_models(&mut value);
        let models = value["ai"]["providers"][0]["models"].as_array().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["name"], "model-a");
        assert_eq!(
            models[0]["capabilities"],
            serde_json::json!(["transcription"])
        );
    }

    #[test]
    fn test_migrate_builtin_skills_resets_modified_prompts() {
        let mut value = serde_json::json!({
            "features": {
                "skills": {
                    "enabled": true,
                    "skills": [
                        {
                            "id": "builtin-fillers",
                            "name": "语气词剔除",
                            "prompt": "MODIFIED PROMPT",
                            "builtin": true,
                            "editable": false,
                            "enabled": true,
                        }
                    ]
                }
            }
        });
        migrate_builtin_skills(&mut value);
        let skills = value["features"]["skills"]["skills"].as_array().unwrap();
        let filler = skills
            .iter()
            .find(|s| s["id"] == "builtin-fillers")
            .unwrap();
        assert_ne!(filler["prompt"].as_str().unwrap(), "MODIFIED PROMPT");
    }

    #[test]
    fn test_migrate_builtin_skills_adds_missing_defaults() {
        let mut value = serde_json::json!({
            "features": {
                "skills": {
                    "enabled": true,
                    "skills": []
                }
            }
        });
        migrate_builtin_skills(&mut value);
        let skills = value["features"]["skills"]["skills"].as_array().unwrap();
        let ids: Vec<&str> = skills.iter().map(|s| s["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"builtin-fillers"));
        assert!(ids.contains(&"builtin-typos"));
        assert!(ids.contains(&"builtin-polish"));
        assert!(ids.contains(&"builtin-formal"));
        assert!(ids.contains(&"builtin-translation"));
    }

    #[test]
    fn test_migrate_builtin_skills_preserves_editable() {
        let mut value = serde_json::json!({
            "features": {
                "skills": {
                    "enabled": true,
                    "skills": [
                        {
                            "id": "builtin-translation",
                            "name": "翻译优化",
                            "prompt": "CUSTOM TRANSLATION PROMPT",
                            "builtin": true,
                            "editable": true,
                            "enabled": true,
                        }
                    ]
                }
            }
        });
        migrate_builtin_skills(&mut value);
        let skills = value["features"]["skills"]["skills"].as_array().unwrap();
        let translation = skills
            .iter()
            .find(|s| s["id"] == "builtin-translation")
            .unwrap();
        assert_eq!(
            translation["prompt"].as_str().unwrap(),
            "CUSTOM TRANSLATION PROMPT"
        );
    }
}
