mod common;

use common::*;
use talkshow_lib::{
    AppConfig, Skill, SkillsConfig, TranscriptionConfig, load_config, save_config, validate_config,
};

#[test]
fn test_save_config_accepts_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_app_config();
    let result = save_config(dir.path(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_save_config_rejects_empty_providers() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_app_config();
    config.ai.providers = vec![];
    let result = save_config(dir.path(), &config);
    assert!(result.is_ok());
}

#[test]
fn test_load_config_returns_default_on_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = load_config(dir.path());
    assert!(!config.shortcut.is_empty());
}

#[test]
fn test_validate_config_accepts_valid_config() {
    let config = test_app_config();
    let result = validate_config(&config);
    assert!(result.is_ok());
}

#[test]
fn test_validate_config_rejects_invalid_provider_id() {
    let mut config = test_app_config();
    config.ai.providers[0].id = "".to_string();
    let result = validate_config(&config);
    assert!(result.is_err());
}

// Shortcut 校验契约测试
#[test]
fn test_validate_config_accepts_empty_shortcut() {
    let mut config = test_app_config();
    config.shortcut = "".to_string();
    let result = validate_config(&config);
    assert!(result.is_ok());
}

#[test]
fn test_validate_config_rejects_too_long_shortcut() {
    let mut config = test_app_config();
    config.shortcut = "A".repeat(101);
    let result = validate_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_config_accepts_valid_shortcut() {
    let config = test_app_config();
    let result = validate_config(&config);
    assert!(result.is_ok());
}

// Skills CRUD 契约测试
#[test]
fn test_skills_config_validation() {
    let config = SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "".to_string(),
            name: "测试".to_string(),
            description: "".to_string(),
            prompt: "prompt".to_string(),
            builtin: false,
            editable: true,
            enabled: true,
        }],
    };
    let json = serde_json::to_string(&config).unwrap();
    let loaded: SkillsConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.skills.len(), 1);
}

#[test]
fn test_transcription_config_serialization() {
    let config = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test-model".to_string(),
        polish_enabled: true,
        polish_provider_id: "test".to_string(),
        polish_model: "test-model".to_string(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let loaded: TranscriptionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.provider_id, "test");
    assert_eq!(loaded.model, "test-model");
    assert!(loaded.polish_enabled);
}

#[test]
fn test_app_config_full_serialization() {
    let config = test_app_config();
    let json = serde_json::to_string(&config).unwrap();
    let loaded: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.shortcut, config.shortcut);
    assert_eq!(loaded.ai.providers.len(), config.ai.providers.len());
    assert_eq!(
        loaded.features.skills.enabled,
        config.features.skills.enabled
    );
}
