mod common;

use common::*;
use talkshow_lib::{Skill, SkillsConfig, TranscriptionConfig};
use talkshow_lib::{assemble_skills_prompt, process_with_skills_client};

#[tokio::test]
async fn test_skills_disabled_returns_original() {
    let (logger, _dir) = create_test_logger();
    let config = SkillsConfig {
        enabled: false,
        skills: vec![],
    };
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result =
        process_with_skills_client(&logger, &config, &tc, &providers, "原始文本", &mut mock).await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_empty_transcription_returns_original() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result = process_with_skills_client(&logger, &config, &tc, &providers, "", &mut mock).await;

    assert_eq!(result.unwrap(), "");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_no_enabled_skills_returns_original() {
    let (logger, _dir) = create_test_logger();
    let config = SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "disabled-skill".to_string(),
            name: "禁用技能".to_string(),
            description: "".to_string(),
            prompt: "".to_string(),
            builtin: false,
            editable: true,
            enabled: false,
        }],
    };
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result =
        process_with_skills_client(&logger, &config, &tc, &providers, "原始文本", &mut mock).await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_no_polish_provider_returns_original() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test".to_string(),
        polish_enabled: true,
        polish_provider_id: "".to_string(),
        polish_model: "".to_string(),
    };
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result =
        process_with_skills_client(&logger, &config, &tc, &providers, "原始文本", &mut mock).await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_empty_polish_model_returns_original() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test".to_string(),
        polish_enabled: true,
        polish_provider_id: "test-provider".to_string(),
        polish_model: "".to_string(),
    };
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result =
        process_with_skills_client(&logger, &config, &tc, &providers, "原始文本", &mut mock).await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_provider_not_found_returns_original() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = TranscriptionConfig {
        provider_id: "test".to_string(),
        model: "test".to_string(),
        polish_enabled: true,
        polish_provider_id: "nonexistent".to_string(),
        polish_model: "test-model".to_string(),
    };
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    let result =
        process_with_skills_client(&logger, &config, &tc, &providers, "原始文本", &mut mock).await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 0);
}

#[tokio::test]
async fn test_skills_calls_llm_and_returns_result() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _model, _provider| {
        assert!(prompt.contains("测试技能"));
        assert!(prompt.contains("测试 prompt"));
        Ok("处理后的文本".to_string())
    });

    let result = process_with_skills_client(
        &logger,
        &config,
        &tc,
        &providers,
        "嗯那个你好世界啊",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "处理后的文本");
    assert_eq!(mock.send_text_call_count(), 1);
}

#[tokio::test]
async fn test_skills_falls_back_on_llm_error() {
    let (logger, _dir) = create_test_logger();
    let config = enabled_skills_config();
    let tc = test_transcription_config();
    let providers = test_providers();
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|_, _, _| Err("LLM error".to_string()));

    let result =
        process_with_skills_client(&logger, &config, &tc, &providers, "原始文本", &mut mock).await;

    assert_eq!(result.unwrap(), "原始文本");
    assert_eq!(mock.send_text_call_count(), 1);
}

#[test]
fn test_assemble_skills_prompt_with_app_context() {
    let skills = vec![Skill {
        id: "test".to_string(),
        name: "测试技能".to_string(),
        description: "描述".to_string(),
        prompt: "测试 prompt".to_string(),
        builtin: false,
        editable: true,
        enabled: true,
    }];

    let (system, user) = assemble_skills_prompt(&skills, "转录文本", "Finder", "com.apple.finder");

    assert!(system.contains("测试技能"));
    assert!(system.contains("测试 prompt"));
    assert!(system.contains("Finder"));
    assert!(system.contains("com.apple.finder"));
    assert_eq!(user, "转录文本");
}

#[test]
fn test_assemble_skills_prompt_without_selected_text() {
    let skills: Vec<Skill> = vec![];
    let (_system, _user) = assemble_skills_prompt(&skills, "你好", "App", "com.app");

    assert_eq!(_user, "你好");
}
