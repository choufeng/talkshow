use crate::config::{ProviderConfig, Skill, SkillsConfig};
use crate::logger::Logger;

const SKILLS_TIMEOUT_SECS: u64 = 10;

#[cfg(target_os = "macos")]
fn get_frontmost_app() -> Result<(String, String), String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(
            "tell application \"System Events\" to get {name, bundle identifier} of first process whose frontmost is true",
        )
        .output()
        .map_err(|e| format!("Failed to get frontmost app: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed: {}", stderr));
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = result.trim().split(", ").collect();

    let app_name = parts.first().unwrap_or(&"Unknown").to_string();
    let bundle_id = parts.get(1).unwrap_or(&"unknown").to_string();

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
) -> (String, String) {
    let mut system_prompt = String::from(
        "你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。\n\n",
    );

    system_prompt.push_str(&format!(
        "当前用户正在使用的应用是：{} ({})\n",
        app_name, bundle_id
    ));
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
    providers: &[ProviderConfig],
    transcription: &str,
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

    if skills_config.provider_id.is_empty() || skills_config.model.is_empty() {
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

    let skills_owned: Vec<Skill> = enabled_skills.into_iter().cloned().collect();
    let (system_prompt, user_message) =
        assemble_skills_prompt(&skills_owned, transcription, &app_name, &bundle_id);

    let provider = match providers
        .iter()
        .find(|p| p.id == skills_config.provider_id)
    {
        Some(p) => p,
        None => {
            logger.warn(
                "skills",
                "未找到 Skills Provider，回退原始文字",
                Some(serde_json::json!({
                    "provider_id": skills_config.provider_id,
                })),
            );
            return Ok(transcription.to_string());
        }
    };

    let full_prompt = format!("{}\n\n输入文本：\n{}", system_prompt, user_message);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(SKILLS_TIMEOUT_SECS),
        crate::ai::send_text_prompt(logger, &full_prompt, &skills_config.model, provider),
    )
    .await;

    match result {
        Ok(Ok(text)) => {
            logger.info(
                "skills",
                "LLM 调用成功",
                Some(serde_json::json!({
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
                Some(serde_json::json!({ "error": e.to_string() })),
            );
            Ok(transcription.to_string())
        }
        Err(_) => {
            logger.error(
                "skills",
                "LLM 调用超时，回退原始文字",
                Some(serde_json::json!({ "timeout_secs": SKILLS_TIMEOUT_SECS })),
            );
            Ok(transcription.to_string())
        }
    }
}
