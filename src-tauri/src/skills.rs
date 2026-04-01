use crate::config::{ProviderConfig, Skill, SkillsConfig};
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use std::sync::{Arc, Mutex};

type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;

const SKILLS_BASE_TIMEOUT_SECS: u64 = 15;
const SKILLS_PER_SKILL_TIMEOUT_SECS: u64 = 5;

#[cfg(target_os = "macos")]
fn get_frontmost_app() -> Result<(String, String), String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output()
        .map_err(|e| format!("Failed to get frontmost app: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed: {}", stderr));
    }

    let app_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let bundle_output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!("tell application \"System Events\" to get bundle identifier of process \"{}\"", app_name))
        .output()
        .map_err(|e| format!("Failed to get bundle id: {}", e))?;

    let bundle_id = if bundle_output.status.success() {
        String::from_utf8_lossy(&bundle_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    Ok((app_name, bundle_id))
}

#[cfg(not(target_os = "macos"))]
fn get_frontmost_app() -> Result<(String, String), String> {
    Ok(("Unknown".to_string(), "unknown".to_string()))
}

fn assemble_skills_prompt(
    skills: &[Skill],
    transcription: &str,
    app_name: &str,
    bundle_id: &str,
    selected_text: Option<&str>,
) -> (String, String) {
    let mut system_prompt = String::from(
        "你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。\n\n基础规则：\n1. 如果输入是短内容（如单词、短语、简短回答）或非完整句子，不要添加句尾标点符号。只有当输入明显构成完整句子时才添加标点。\n2. 当输入包含多种语言（如中英文混用）时，保留各语言的原文表达，不要尝试翻译或统一为某一种语言。\n",
    );

    system_prompt.push_str(&format!(
        "当前用户正在使用的应用是：{} ({})\n",
        app_name, bundle_id
    ));
    if let Some(selected) = selected_text {
        system_prompt.push_str(&format!(
            "用户选中了以下文字，准备用语音替换它。请在处理转写结果时考虑这个上下文，使替换后的文本自然衔接。\n选中的原文：「{}」\n",
            selected
        ));
    }
    system_prompt.push_str("请仅应用与当前场景相关的规则，跳过不适用的规则。");

    for skill in skills {
        system_prompt.push_str(&format!("\n---\n【{}】\n{}", skill.name, skill.prompt));
    }

    system_prompt.push_str("\n\n请只输出处理后的纯文本。不要添加任何解释、标注或前缀。");

    let user_message = transcription.to_string();

    (system_prompt, user_message)
}

pub async fn process_with_skills(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &VertexClientCache,
    selected_text: Option<&str>,
) -> Result<String, String> {
    if !skills_config.enabled {
        return Ok(transcription.to_string());
    }

    let enabled_skills: Vec<&Skill> = skills_config
        .skills
        .iter()
        .filter(|s| s.enabled)
        .collect();

    if enabled_skills.is_empty() {
        logger.info("skills", "没有启用的 Skill，跳过处理", None);
        return Ok(transcription.to_string());
    }

    if transcription.is_empty() {
        logger.info("skills", "转写文字为空，跳过处理", None);
        return Ok(transcription.to_string());
    }

    if transcription_config.polish_provider_id.is_empty() || transcription_config.polish_model.is_empty() {
        logger.warn("skills", "Skills Provider 未配置，跳过处理", None);
        return Ok(transcription.to_string());
    }

    let (app_name, bundle_id) = match get_frontmost_app() {
        Ok(info) => info,
        Err(e) => {
            logger.warn(
                "skills",
                "获取前台应用信息失败",
                Some(serde_json::json!({ "error": e })),
            );
            ("Unknown".to_string(), "unknown".to_string())
        }
    };

    logger.info(
        "skills",
        "Skills 处理开始",
        Some(serde_json::json!({
            "enabled_skills": enabled_skills.iter().map(|s| &s.name).collect::<Vec<_>>(),
            "app_name": app_name,
            "bundle_id": bundle_id,
        })),
    );

    let start = std::time::Instant::now();

    let skills_owned: Vec<Skill> = enabled_skills.into_iter().cloned().collect();
    let (system_prompt, user_message) =
        assemble_skills_prompt(&skills_owned, transcription, &app_name, &bundle_id, selected_text);

    let provider = match providers
        .iter()
        .find(|p| p.id == transcription_config.polish_provider_id)
    {
        Some(p) => p,
        None => {
            logger.warn(
                "skills",
                "未找到 Skills Provider，回退原始文字",
                Some(serde_json::json!({
                    "provider_id": transcription_config.polish_provider_id,
                })),
            );
            return Ok(transcription.to_string());
        }
    };

    let full_prompt = format!("{}\n\n输入文本：\n{}", system_prompt, user_message);

    let timeout_secs = SKILLS_BASE_TIMEOUT_SECS
        + (skills_owned.len() as u64) * SKILLS_PER_SKILL_TIMEOUT_SECS;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        crate::ai::send_text_prompt(logger, &full_prompt, &transcription_config.polish_model, provider, vertex_cache, crate::ai::ThinkingMode::Disabled),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(text)) => {
            logger.info(
                "skills",
                "LLM 调用成功",
                Some(serde_json::json!({
                    "elapsed_ms": elapsed_ms,
                    "original_length": transcription.len(),
                    "processed_length": text.len(),
                    "original_preview": transcription.chars().take(50).collect::<String>(),
                    "processed_preview": text.chars().take(50).collect::<String>(),
                })),
            );
            Ok(text)
        }
        Ok(Err(e)) => {
            logger.error(
                "skills",
                "LLM 调用失败，回退原始文字",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms, "error": e.to_string() })),
            );
            Ok(transcription.to_string())
        }
        Err(_) => {
            logger.error(
                "skills",
                "LLM 调用超时，回退原始文字",
                Some(serde_json::json!({ "elapsed_ms": elapsed_ms, "timeout_secs": timeout_secs })),
            );
            Ok(transcription.to_string())
        }
    }
}

pub async fn process_with_skills_client(
    logger: &Logger,
    skills_config: &SkillsConfig,
    transcription_config: &crate::config::TranscriptionConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    client: &mut dyn LlmClient,
    selected_text: Option<&str>,
) -> Result<String, String> {
    if !skills_config.enabled {
        return Ok(transcription.to_string());
    }

    let enabled_skills: Vec<&Skill> = skills_config
        .skills
        .iter()
        .filter(|s| s.enabled)
        .collect();

    if enabled_skills.is_empty() {
        logger.info("skills", "没有启用的 Skill，跳过处理", None);
        return Ok(transcription.to_string());
    }

    if transcription.is_empty() {
        logger.info("skills", "转写文字为空，跳过处理", None);
        return Ok(transcription.to_string());
    }

    if transcription_config.polish_provider_id.is_empty() || transcription_config.polish_model.is_empty() {
        logger.warn("skills", "Skills Provider 未配置，跳过处理", None);
        return Ok(transcription.to_string());
    }

    let provider = match providers
        .iter()
        .find(|p| p.id == transcription_config.polish_provider_id)
    {
        Some(p) => p,
        None => {
            logger.warn(
                "skills",
                "未找到 Skills Provider，回退原始文字",
                Some(serde_json::json!({
                    "provider_id": transcription_config.polish_provider_id,
                })),
            );
            return Ok(transcription.to_string());
        }
    };

    let skills_owned: Vec<Skill> = enabled_skills.into_iter().cloned().collect();
    let (app_name, bundle_id) = get_frontmost_app().unwrap_or(("Unknown".to_string(), "unknown".to_string()));
    let (system_prompt, user_message) =
        assemble_skills_prompt(&skills_owned, transcription, &app_name, &bundle_id, selected_text);

    let full_prompt = format!("{}\n\n输入文本：\n{}", system_prompt, user_message);

    let timeout_secs = SKILLS_BASE_TIMEOUT_SECS
        + (skills_owned.len() as u64) * SKILLS_PER_SKILL_TIMEOUT_SECS;

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        client.send_text(&full_prompt, &transcription_config.polish_model, &provider.id, &provider.endpoint),
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(Ok(text)) => {
            logger.info("skills", "LLM 调用成功", Some(serde_json::json!({
                "elapsed_ms": elapsed_ms,
                "original_length": transcription.len(),
                "processed_length": text.len(),
            })));
            Ok(text)
        }
        Ok(Err(e)) => {
            logger.error("skills", "LLM 调用失败，回退原始文字", Some(serde_json::json!({
                "elapsed_ms": elapsed_ms, "error": e,
            })));
            Ok(transcription.to_string())
        }
        Err(_) => {
            logger.error("skills", "LLM 调用超时，回退原始文字", Some(serde_json::json!({
                "elapsed_ms": elapsed_ms, "timeout_secs": timeout_secs,
            })));
            Ok(transcription.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TranscriptionConfig;
    use crate::llm_client::MockLlmClient;

    fn enabled_skills_config() -> SkillsConfig {
        SkillsConfig {
            enabled: true,
            skills: vec![
                Skill {
                    id: "builtin-fillers".to_string(),
                    name: "语气词剔除".to_string(),
                    description: "去除语气词".to_string(),
                    prompt: "去除语气词".to_string(),
                    builtin: true,
                    editable: false,
                    enabled: true,
                },
            ],
        }
    }

    fn test_transcription_config() -> TranscriptionConfig {
        TranscriptionConfig {
            provider_id: "test-provider".to_string(),
            model: "test-model".to_string(),
            polish_enabled: true,
            polish_provider_id: "test-provider".to_string(),
            polish_model: "test-model".to_string(),
        }
    }

    fn test_providers() -> Vec<ProviderConfig> {
        vec![ProviderConfig {
            id: "test-provider".to_string(),
            provider_type: "openai-compatible".to_string(),
            name: "Test".to_string(),
            endpoint: "https://example.com/v1".to_string(),
            api_key: Some("sk-test".to_string()),
            models: vec![],
        }]
    }

    fn create_test_logger() -> Logger {
        let dir = tempfile::tempdir().unwrap();
        Logger::new(dir.path()).unwrap()
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_disabled() {
        let logger = create_test_logger();
        let mut config = enabled_skills_config();
        config.enabled = false;
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_empty() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_no_skills_enabled() {
        let logger = create_test_logger();
        let config = SkillsConfig { enabled: true, skills: vec![] };
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_no_provider_configured() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = TranscriptionConfig {
            provider_id: "".to_string(),
            model: "".to_string(),
            polish_enabled: true,
            polish_provider_id: "".to_string(),
            polish_model: "".to_string(),
        };
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_returns_original_when_provider_not_found() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = TranscriptionConfig {
            provider_id: "nonexistent".to_string(),
            model: "test-model".to_string(),
            polish_enabled: true,
            polish_provider_id: "nonexistent".to_string(),
            polish_model: "test-model".to_string(),
        };
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[tokio::test]
    async fn test_process_with_skills_calls_llm_and_returns_result() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Ok("处理后的文本".to_string()));

        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "嗯那个你好世界啊", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "处理后的文本");
    }

    #[tokio::test]
    async fn test_process_with_skills_falls_back_on_llm_error() {
        let logger = create_test_logger();
        let config = enabled_skills_config();
        let tc = test_transcription_config();
        let providers = test_providers();

        let mut mock = MockLlmClient::new();
        mock.expect_send_text()
            .returning(|_, _, _, _| Err("LLM error".to_string()));

        let result = process_with_skills_client(
            &logger, &config, &tc, &providers,
            "你好世界", &mut mock, None,
        ).await;
        assert_eq!(result.unwrap(), "你好世界");
    }

    #[test]
    fn test_assemble_skills_prompt_includes_skill_name() {
        let skills = vec![Skill {
            id: "test".to_string(),
            name: "测试技能".to_string(),
            description: "".to_string(),
            prompt: "测试 prompt".to_string(),
            builtin: false,
            editable: true,
            enabled: true,
        }];
        let (system, user) = assemble_skills_prompt(&skills, "你好", "App", "com.app", None);
        assert!(system.contains("测试技能"));
        assert!(system.contains("测试 prompt"));
        assert_eq!(user, "你好");
    }

    #[test]
    fn test_assemble_skills_prompt_includes_app_context() {
        let skills = vec![];
        let (system, _) = assemble_skills_prompt(&skills, "你好", "Finder", "com.apple.finder", Some("选中文本"));
        assert!(system.contains("Finder"));
        assert!(system.contains("com.apple.finder"));
        assert!(system.contains("选中文本"));
    }
}
