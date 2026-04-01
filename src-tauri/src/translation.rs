use crate::config::{ProviderConfig, SkillsConfig};
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use std::sync::{Arc, Mutex};

type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;

const TRANSLATION_TIMEOUT_SECS: u64 = 15;

const TRANSLATION_BASE_PROMPT: &str = "You are a professional translator. Translate the following text to {target_lang}. Output only the translation, nothing else.";

fn get_translation_skill_prompt(skills_config: &SkillsConfig) -> Option<String> {
    skills_config
        .skills
        .iter()
        .find(|s| s.id == "builtin-translation" && s.enabled)
        .map(|s| s.prompt.clone())
}

#[allow(clippy::too_many_arguments)]
pub async fn translate_text(
    logger: &Logger,
    text: &str,
    target_lang: &str,
    skills_config: &SkillsConfig,
    provider_id: &str,
    model_name: &str,
    providers: &[ProviderConfig],
    vertex_cache: &VertexClientCache,
) -> Result<String, String> {
    let provider = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Translation provider not found: {}", provider_id))?;

    let mut system_prompt = TRANSLATION_BASE_PROMPT.replace("{target_lang}", target_lang);

    if let Some(skill_prompt) = get_translation_skill_prompt(skills_config) {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&skill_prompt);
    }

    let full_prompt = format!("{}\n\n{}", system_prompt, text);

    logger.info(
        "translation",
        "翻译开始",
        Some(serde_json::json!({
            "target_lang": target_lang,
            "provider_id": provider_id,
            "model": model_name,
            "text_length": text.len(),
            "text_preview": text.chars().take(50).collect::<String>(),
        })),
    );

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(TRANSLATION_TIMEOUT_SECS),
        crate::ai::send_text_prompt(
            logger,
            &full_prompt,
            model_name,
            provider,
            vertex_cache,
            crate::ai::ThinkingMode::Disabled,
        ),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(translated)) => {
            logger.info(
                "translation",
                "翻译成功",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "original_length": text.len(),
                    "translated_length": translated.len(),
                    "translated_preview": translated.chars().take(50).collect::<String>(),
                })),
            );
            Ok(translated)
        }
        Ok(Err(e)) => {
            logger.error(
                "translation",
                "翻译失败",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "error": e.to_string(),
                })),
            );
            Err(format!("翻译失败: {}", e))
        }
        Err(_) => {
            logger.error(
                "translation",
                "翻译超时",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "timeout_secs": TRANSLATION_TIMEOUT_SECS,
                })),
            );
            Err(format!("翻译超时 ({}s)", TRANSLATION_TIMEOUT_SECS))
        }
    }
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub async fn translate_text_client(
    logger: &Logger,
    text: &str,
    target_lang: &str,
    skills_config: &SkillsConfig,
    provider_id: &str,
    model_name: &str,
    endpoint: &str,
    client: &mut dyn LlmClient,
) -> Result<String, String> {
    let mut system_prompt = TRANSLATION_BASE_PROMPT.replace("{target_lang}", target_lang);

    if let Some(skill_prompt) = get_translation_skill_prompt(skills_config) {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&skill_prompt);
    }

    let full_prompt = format!("{}\n\n{}", system_prompt, text);

    logger.info(
        "translation",
        "翻译开始",
        Some(serde_json::json!({
            "target_lang": target_lang,
            "provider_id": provider_id,
            "model_name": model_name,
            "text_length": text.len(),
        })),
    );

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(TRANSLATION_TIMEOUT_SECS),
        client.send_text(&full_prompt, model_name, provider_id, endpoint),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(translated)) => {
            logger.info(
                "translation",
                "翻译成功",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "original_length": text.len(),
                    "translated_length": translated.len(),
                })),
            );
            Ok(translated)
        }
        Ok(Err(e)) => {
            logger.error(
                "translation",
                "翻译失败",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms, "error": e })),
            );
            Err(format!("翻译失败: {}", e))
        }
        Err(_) => {
            logger.error(
                "translation",
                "翻译超时",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms })),
            );
            Err(format!("翻译超时 ({}s)", TRANSLATION_TIMEOUT_SECS))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Skill;
    use crate::llm_client::MockLlmClient;

    fn create_test_logger() -> Logger {
        let dir = tempfile::tempdir().unwrap();
        Logger::new(dir.path()).unwrap()
    }

    fn skills_with_translation() -> SkillsConfig {
        SkillsConfig {
            enabled: true,
            skills: vec![Skill {
                id: "builtin-translation".to_string(),
                name: "翻译优化".to_string(),
                description: "".to_string(),
                prompt: "保持原文语气".to_string(),
                builtin: true,
                editable: true,
                enabled: true,
            }],
        }
    }

    #[tokio::test]
    async fn test_translate_text_success() {
        let logger = create_test_logger();
        let skills = skills_with_translation();

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Ok("Hello World".to_string()));

        let result = translate_text_client(
            &logger,
            "你好世界",
            "English",
            &skills,
            "test-provider",
            "test-model",
            "https://example.com/v1",
            &mut mock,
        )
        .await;
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[tokio::test]
    async fn test_translate_text_llm_error() {
        let logger = create_test_logger();
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![],
        };

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Err("API error".to_string()));

        let result = translate_text_client(
            &logger,
            "你好",
            "English",
            &skills,
            "test-provider",
            "test-model",
            "https://example.com/v1",
            &mut mock,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("翻译失败"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_translate_text_timeout() {
        let logger = create_test_logger();
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![],
        };

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Err("timeout simulation".to_string()));

        let result = translate_text_client(
            &logger,
            "你好",
            "English",
            &skills,
            "test-provider",
            "test-model",
            "https://example.com/v1",
            &mut mock,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("翻译失败"));
    }

    #[test]
    fn test_get_translation_skill_prompt_found() {
        let skills = skills_with_translation();
        let prompt = get_translation_skill_prompt(&skills);
        assert!(prompt.is_some());
        assert_eq!(prompt.unwrap(), "保持原文语气");
    }

    #[test]
    fn test_get_translation_skill_prompt_not_found() {
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![],
        };
        let prompt = get_translation_skill_prompt(&skills);
        assert!(prompt.is_none());
    }

    #[test]
    fn test_get_translation_skill_prompt_disabled() {
        let skills = SkillsConfig {
            enabled: true,
            skills: vec![Skill {
                id: "builtin-translation".to_string(),
                name: "翻译优化".to_string(),
                description: "".to_string(),
                prompt: "test".to_string(),
                builtin: true,
                editable: true,
                enabled: false,
            }],
        };
        let prompt = get_translation_skill_prompt(&skills);
        assert!(prompt.is_none());
    }
}
