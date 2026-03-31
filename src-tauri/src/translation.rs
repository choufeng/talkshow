use crate::config::{ProviderConfig, Skill, SkillsConfig};
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
