mod common;

use common::*;
use talkshow_lib::{ModelConfig, ProviderConfig, Skill, load_config, save_config};

fn find_provider<'a>(providers: &'a [ProviderConfig], id: &str) -> &'a ProviderConfig {
    providers
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("Provider '{}' not found", id))
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_app_config();

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.shortcut, config.shortcut);
    assert_eq!(loaded.recording_shortcut, config.recording_shortcut);
    assert_eq!(loaded.translate_shortcut, config.translate_shortcut);
    // load_config merges builtin providers, so count may be higher
    assert!(loaded.ai.providers.len() >= config.ai.providers.len());
    let saved_provider = find_provider(&loaded.ai.providers, "test-provider");
    assert_eq!(saved_provider.name, "Test Provider");
}

#[test]
fn test_save_and_load_with_api_keys() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers[0].api_key = Some("sk-secret-key-123".to_string());

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    let provider = find_provider(&loaded.ai.providers, "test-provider");
    assert_eq!(provider.api_key, Some("sk-secret-key-123".to_string()));
}

#[test]
fn test_save_and_load_without_api_keys() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    // test_providers() sets api_key to Some("sk-test-key"), clear it
    config.ai.providers[0].api_key = None;

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    let provider = find_provider(&loaded.ai.providers, "test-provider");
    assert_eq!(provider.api_key, None);
}

#[test]
fn test_load_nonexistent_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let config = load_config(dir.path());

    // 应返回默认配置，不报错
    assert!(!config.shortcut.is_empty());
}

#[test]
fn test_save_and_load_features_config() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.features.transcription.provider_id = "vertex".to_string();
    config.features.transcription.model = "gemini-pro".to_string();
    config.features.translation.target_lang = "Japanese".to_string();
    config.features.skills.enabled = true;
    config.features.skills.skills = vec![Skill {
        id: "custom-skill".to_string(),
        name: "自定义技能".to_string(),
        description: "描述".to_string(),
        prompt: "prompt".to_string(),
        builtin: false,
        editable: true,
        enabled: true,
    }];
    config.features.recording.auto_mute = true;

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    assert_eq!(loaded.features.transcription.provider_id, "vertex");
    assert_eq!(loaded.features.transcription.model, "gemini-pro");
    assert_eq!(loaded.features.translation.target_lang, "Japanese");
    assert!(loaded.features.skills.enabled);
    // load_config migrates builtin skills, so custom non-builtin skills are preserved
    let custom_skill = loaded
        .features
        .skills
        .skills
        .iter()
        .find(|s| s.id == "custom-skill")
        .expect("custom skill not found");
    assert!(custom_skill.enabled);
    assert!(loaded.features.recording.auto_mute);
}

#[test]
fn test_save_and_load_multiple_providers() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers.push(ProviderConfig {
        id: "second-provider".to_string(),
        provider_type: "vertex".to_string(),
        name: "Second Provider".to_string(),
        endpoint: "https://vertex.example.com".to_string(),
        api_key: None,
        models: vec![],
    });

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    // load_config merges builtin providers, so count may be higher
    let second = find_provider(&loaded.ai.providers, "second-provider");
    assert_eq!(second.name, "Second Provider");
    assert_eq!(second.provider_type, "vertex");
}

#[test]
fn test_save_and_load_with_models() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers[0].models = vec![
        ModelConfig {
            name: "gpt-4".to_string(),
            capabilities: vec!["chat".to_string()],
            verified: None,
        },
        ModelConfig {
            name: "whisper-1".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        },
    ];

    save_config(dir.path(), &config).unwrap();
    let loaded = load_config(dir.path());

    let provider = find_provider(&loaded.ai.providers, "test-provider");
    assert_eq!(provider.models.len(), 2);
    assert_eq!(provider.models[0].name, "gpt-4");
    assert_eq!(provider.models[1].name, "whisper-1");
}

#[test]
fn test_corrupted_config_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    std::fs::write(&config_path, "not valid json{{{").unwrap();

    // load_config 应优雅处理损坏的文件，返回默认配置
    let config = load_config(dir.path());
    assert!(!config.shortcut.is_empty());
}
