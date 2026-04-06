mod common;

use common::*;
use talkshow_lib::translate_text_client;
use talkshow_lib::{Skill, SkillsConfig};

#[tokio::test]
async fn test_translate_success() {
    let (logger, _dir) = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, model, provider| {
        assert!(prompt.contains("professional translator"));
        assert!(prompt.contains("English"));
        assert_eq!(model, "gpt-4");
        assert_eq!(provider, "test-provider");
        Ok("Hello World".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好世界",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "Hello World");
    assert_eq!(mock.send_text_call_count(), 1);
}

#[tokio::test]
async fn test_translate_with_skill_prompt() {
    let (logger, _dir) = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "builtin-translation".to_string(),
            name: "翻译优化".to_string(),
            description: "".to_string(),
            prompt: "保持原文语气和风格".to_string(),
            builtin: true,
            editable: true,
            enabled: true,
        }],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _| {
        assert!(prompt.contains("保持原文语气和风格"));
        Ok("Translated with style".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好世界",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "Translated with style");
}

#[tokio::test]
async fn test_translate_llm_error() {
    let (logger, _dir) = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|_, _, _| Err("API error".to_string()));

    let result = translate_text_client(
        &logger,
        "你好",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        &mut mock,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("翻译失败"));
}

#[tokio::test]
async fn test_translate_empty_text() {
    let (logger, _dir) = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _| {
        assert!(!prompt.is_empty());
        Ok("".to_string())
    });

    let result = translate_text_client(
        &logger,
        "",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "");
}

#[tokio::test]
async fn test_translate_chinese_to_japanese() {
    let (logger, _dir) = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _| {
        assert!(prompt.contains("Japanese"));
        Ok("こんにちは世界".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好世界",
        "Japanese",
        &skills,
        "test-provider",
        "gpt-4",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "こんにちは世界");
}

#[tokio::test]
async fn test_translate_chinese_to_english() {
    let (logger, _dir) = create_test_logger();
    let skills = SkillsConfig {
        enabled: true,
        skills: vec![],
    };
    let mut mock = MockLlmClientIntegration::new();

    mock.expect_send_text(|prompt, _, _| {
        assert!(prompt.contains("English"));
        Ok("Hello, how are you?".to_string())
    });

    let result = translate_text_client(
        &logger,
        "你好吗",
        "English",
        &skills,
        "test-provider",
        "gpt-4",
        &mut mock,
    )
    .await;

    assert_eq!(result.unwrap(), "Hello, how are you?");
}
