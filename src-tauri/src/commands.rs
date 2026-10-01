use crate::ai;
use crate::config;
use crate::history;
use crate::logger::Logger;
use crate::providers::ProviderContext;
use crate::shortcuts::{SHORTCUT_IDS, parse_shortcuts, shortcut_ids};
use std::time::Instant;
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[tauri::command]
pub fn list_audio_devices() -> Vec<crate::recording::AudioDeviceInfo> {
    crate::recording::list_input_devices()
}

/// 切换首选麦克风:持久化配置并重建两路录音器(全局快捷键 + chat)
#[tauri::command]
pub fn set_audio_input_device(app_handle: tauri::AppHandle, device: String) -> Result<(), String> {
    if app_handle
        .state::<crate::session::SessionManager>()
        .has_active()
    {
        return Err("录音进行中,请先结束录音再切换麦克风".to_string());
    }
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut cfg = config::load_config(&app_data_dir);
    cfg.features.recording.input_device = device.clone();
    config::save_config(&app_data_dir, &cfg)?;

    let name = if device.trim().is_empty() {
        None
    } else {
        Some(device.as_str())
    };
    if let Some(state) = app_handle.try_state::<crate::RecorderState>()
        && let Ok(mut r) = state.recorder.lock()
    {
        *r = crate::recording::AudioRecorder::new_with_device(name);
    }
    // ponytail: chat 录音进行中切换会丢弃当前片段(TooShort),低概率不守卫
    if let Some(state) = app_handle.try_state::<crate::chat::ChatRecorderState>()
        && let Ok(mut r) = state.recorder.lock()
    {
        *r = crate::recording::AudioRecorder::new_with_device(name);
    }
    Ok(())
}

#[tauri::command]
pub fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::load_config(&app_data_dir)
}

#[tauri::command]
pub fn get_onboarding_status(app_handle: tauri::AppHandle) -> bool {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config = config::load_config(&app_data_dir);
    config.onboarding_completed
}

#[tauri::command]
pub fn set_onboarding_completed(
    app_handle: tauri::AppHandle,
    completed: bool,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut config = config::load_config(&app_data_dir);
    config.onboarding_completed = completed;
    config::save_config(&app_data_dir, &config)
}

#[tauri::command]
pub fn update_shortcut(
    app_handle: tauri::AppHandle,
    shortcut_type: String,
    shortcut: String,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);

    let old_toggle = app_config.shortcut.clone();
    let old_recording = app_config.recording_shortcut.clone();
    let old_translate = app_config.translate_shortcut.clone();

    match shortcut_type.as_str() {
        "toggle" => app_config.shortcut = shortcut,
        "recording" => app_config.recording_shortcut = shortcut,
        "translate" => app_config.translate_shortcut = shortcut,
        _ => return Err("Invalid shortcut type".to_string()),
    }

    config::save_config(&app_data_dir, &app_config)?;

    for sc in parse_shortcuts(&old_toggle) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    for sc in parse_shortcuts(&old_recording) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    for sc in parse_shortcuts(&old_translate) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }

    let new_toggle = parse_shortcuts(&app_config.shortcut);
    let new_rec = parse_shortcuts(&app_config.recording_shortcut);
    let new_translate = parse_shortcuts(&app_config.translate_shortcut);

    {
        let mut ids = SHORTCUT_IDS.write().unwrap();
        ids.toggle = shortcut_ids(&new_toggle);
        ids.recording = shortcut_ids(&new_rec);
        ids.translate = shortcut_ids(&new_translate);
    }

    for sc in new_toggle {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }
    for sc in new_rec {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }
    for sc in new_translate {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn save_config_cmd(
    app_handle: tauri::AppHandle,
    config: config::AppConfig,
) -> Result<(), String> {
    config::validate_config(&config)?;
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::save_config(&app_data_dir, &config)
}

#[derive(serde::Serialize, Clone)]
pub struct TestResult {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub message: String,
}

#[tauri::command]
pub async fn test_model_connectivity(
    app_handle: tauri::AppHandle,
    provider_id: String,
    model_name: String,
) -> Result<TestResult, String> {
    let logger = app_handle.state::<Logger>();
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);

    let provider = app_config
        .ai
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?
        .clone();

    let model = provider
        .models
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model not found: {}", model_name))?
        .clone();

    let is_transcription = model.capabilities.iter().any(|c| c == "transcription");

    logger.info(
        "connectivity",
        &format!("开始测试模型连通性: {}/{}", provider_id, model_name),
        Some(serde_json::json!({
            "provider_id": provider_id,
            "model_name": model_name,
            "is_transcription": is_transcription,
        })),
    );

    if provider.id == "sensevoice" {
        return Ok(TestResult {
            status: "ok".to_string(),
            latency_ms: Some(0),
            message: "本地模型，无需连通性测试".to_string(),
        });
    }

    let start = Instant::now();
    let provider_ctx = app_handle.state::<ProviderContext>();
    let result = if provider.id == "vertex" {
        ai::send_text_prompt(
            &logger,
            "Hi",
            &model_name,
            &provider,
            &provider_ctx,
            ai::ThinkingMode::Disabled,
        )
        .await
    } else if is_transcription {
        let test_audio: &[u8] = include_bytes!("../assets/test.wav");
        ai::send_audio_prompt_from_bytes(
            &logger,
            test_audio,
            "audio/wav",
            "请将这段音频转录为文字",
            &model_name,
            &provider,
            &provider_ctx,
        )
        .await
    } else {
        ai::send_text_prompt(
            &logger,
            "Hi",
            &model_name,
            &provider,
            &provider_ctx,
            ai::ThinkingMode::Disabled,
        )
        .await
    };
    let latency = start.elapsed().as_millis() as u64;

    let (status, message) = match &result {
        Ok(text) => {
            let summary: String = text.chars().take(50).collect();
            logger.info(
                "connectivity",
                &format!("测试成功: {}/{}", provider_id, model_name),
                Some(serde_json::json!({
                    "provider_id": provider_id,
                    "model_name": model_name,
                    "latency_ms": latency,
                    "response_summary": summary,
                })),
            );
            ("ok".to_string(), summary)
        }
        Err(e) => {
            let error_str = e.to_string();
            logger.error(
                "connectivity",
                &format!("测试失败: {}/{}", provider_id, model_name),
                Some(serde_json::json!({
                    "provider_id": provider_id,
                    "model_name": model_name,
                    "latency_ms": latency,
                    "error": error_str,
                })),
            );
            ("error".to_string(), error_str)
        }
    };

    let verified = config::ModelVerified {
        status: status.clone(),
        tested_at: chrono::Utc::now().to_rfc3339(),
        latency_ms: Some(latency),
        message: if status == "error" {
            Some(message.clone())
        } else {
            None
        },
    };

    if let Some(p) = app_config
        .ai
        .providers
        .iter_mut()
        .find(|p| p.id == provider_id)
        && let Some(m) = p.models.iter_mut().find(|m| m.name == model_name)
    {
        m.verified = Some(verified);
    }

    config::save_config(&app_data_dir, &app_config)?;

    Ok(TestResult {
        status,
        latency_ms: Some(latency),
        message,
    })
}

#[derive(serde::Serialize, Clone)]
pub struct VertexEnvInfo {
    pub project: String,
    pub location: String,
}

#[tauri::command]
pub fn get_vertex_env_info() -> VertexEnvInfo {
    let project = crate::providers::vertex::get_vertex_project().unwrap_or_default();
    let location = crate::providers::vertex::get_vertex_location();
    VertexEnvInfo { project, location }
}

#[tauri::command]
pub fn get_skills_config(app_handle: tauri::AppHandle) -> config::SkillsConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let app_config = config::load_config(&app_data_dir);
    app_config.features.skills
}

#[tauri::command]
pub fn save_skills_config(
    app_handle: tauri::AppHandle,
    mut skills_config: config::SkillsConfig,
) -> Result<(), String> {
    skills_config.enabled = true;
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.skills = skills_config;
    config::save_config(&app_data_dir, &app_config)
}

#[tauri::command]
pub fn save_transcription_config(
    app_handle: tauri::AppHandle,
    transcription: config::TranscriptionConfig,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.transcription = transcription;
    config::save_config(&app_data_dir, &app_config)
}

#[tauri::command]
pub fn add_skill(app_handle: tauri::AppHandle, skill: config::Skill) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.skills.skills.push(skill);
    app_config.features.skills.enabled = true;
    config::save_config(&app_data_dir, &app_config)
}

#[tauri::command]
pub fn update_skill(app_handle: tauri::AppHandle, skill: config::Skill) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    if let Some(existing) = app_config
        .features
        .skills
        .skills
        .iter_mut()
        .find(|s| s.id == skill.id)
    {
        *existing = skill;
        app_config.features.skills.enabled = true;
        config::save_config(&app_data_dir, &app_config)
    } else {
        Err(format!("Skill not found: {}", skill.id))
    }
}

#[tauri::command]
pub fn delete_skill(app_handle: tauri::AppHandle, skill_id: String) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    let skill = app_config
        .features
        .skills
        .skills
        .iter()
        .find(|s| s.id == skill_id);
    if skill.is_none() {
        return Err(format!("Skill not found: {}", skill_id));
    }
    if skill.unwrap().builtin {
        return Err("Cannot delete builtin skill".to_string());
    }
    app_config
        .features
        .skills
        .skills
        .retain(|s| s.id != skill_id);
    app_config.features.skills.enabled = true;
    config::save_config(&app_data_dir, &app_config)
}

// --- TCC 权限(粘贴能力依赖 Accessibility + Automation)---

#[tauri::command]
pub fn get_history(app_handle: tauri::AppHandle, n: Option<usize>) -> Vec<history::HistoryEntry> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    history::load_recent(&app_data_dir, n.unwrap_or(50))
}

#[tauri::command]
pub async fn analyze_history(
    app_handle: tauri::AppHandle,
    n: Option<usize>,
) -> Result<history::AnalyzeResult, String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let app_config = config::load_config(&app_data_dir);
    let transcription = app_config.features.transcription;

    if transcription.polish_provider_id.is_empty() || transcription.polish_model.is_empty() {
        return Err("请先在设置中配置润色模型（用于分析）".to_string());
    }

    let logger = app_handle.state::<Logger>();
    history::analyze_history(
        &logger,
        &app_data_dir,
        n.unwrap_or(30),
        &transcription.polish_provider_id,
        &transcription.polish_model,
        &app_config.ai.providers,
        &app_handle.state::<ProviderContext>(),
    )
    .await
}

#[tauri::command]
pub fn check_accessibility_permission() -> bool {
    crate::permissions::is_trusted()
}

#[tauri::command]
pub fn reset_tcc_permissions() -> Result<String, String> {
    crate::permissions::reset_tcc()
}

#[tauri::command]
pub fn open_accessibility_settings(automation: Option<bool>) {
    crate::permissions::open_accessibility_settings(automation.unwrap_or(false));
}

#[tauri::command]
pub fn probe_automation_permission() -> Result<(), String> {
    crate::permissions::probe_automation()
}

// ============ Chat 对话窗体 ============

#[derive(serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 多轮对话:messages 格式化为对话 prompt,返回回答
#[tauri::command]
pub async fn chat_send(
    app_handle: tauri::AppHandle,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    let logger = app_handle.state::<Logger>();
    let data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let app_config = config::load_config(&data_dir);
    let transcription = &app_config.features.transcription;

    if transcription.polish_provider_id.is_empty() || transcription.polish_model.is_empty() {
        return Err("请先在设置中配置润色模型".to_string());
    }
    let provider = app_config
        .ai
        .providers
        .iter()
        .find(|p| p.id == transcription.polish_provider_id)
        .cloned()
        .ok_or_else(|| "未找到润色 Provider".to_string())?;

    let mut prompt = String::from(
        "你是一个语音问答助手。用户通过语音向你提问,回答要简洁实用,直接给出答案。使用与提问相同的语言回答。",
    );
    for m in &messages {
        match m.role.as_str() {
            "user" => prompt.push_str(&format!("\n\n用户:{}", m.content)),
            "assistant" => prompt.push_str(&format!("\n\n助手:{}", m.content)),
            _ => {}
        }
    }
    prompt.push_str("\n\n助手:");

    logger.info(
        "chat",
        "chat_send 开始",
        Some(serde_json::json!({
            "turns": messages.len(),
            "model": transcription.polish_model,
        })),
    );

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        ai::send_text_prompt(
            &logger,
            &prompt,
            &transcription.polish_model,
            &provider,
            &app_handle.state::<ProviderContext>(),
            // 对话需要推理质量;转写/润色/翻译等延迟敏感路径保持 Disabled
            ai::ThinkingMode::Enabled,
        ),
    )
    .await;

    match result {
        Ok(Ok(text)) => {
            logger.info("chat", "chat_send 成功", None);
            Ok(text)
        }
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("回答超时,请重试".to_string()),
    }
}

/// chat 窗体语音:开始录音(独立于全局快捷键管线)
#[tauri::command]
pub fn chat_record_start(
    app_handle: tauri::AppHandle,
    recorder: tauri::State<'_, crate::chat::ChatRecorderState>,
) -> Result<(), String> {
    // 并发守卫:全局录音进行中则拒绝
    if app_handle
        .state::<crate::session::SessionManager>()
        .has_active()
    {
        return Err("全局录音进行中,请稍候".to_string());
    }
    let dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut r = recorder.recorder.lock().map_err(|e| e.to_string())?;
    if r.is_recording() {
        return Ok(());
    }
    r.set_output_dir(dir);
    r.start().map_err(|e| e.to_string())
}

/// chat 窗体语音:停止录音并转写,返回文本(不粘贴、不过意图路由)
#[tauri::command]
pub async fn chat_record_stop(
    app_handle: tauri::AppHandle,
    recorder: tauri::State<'_, crate::chat::ChatRecorderState>,
) -> Result<String, String> {
    let logger = app_handle.state::<Logger>();
    let result = {
        let mut r = recorder.recorder.lock().map_err(|e| e.to_string())?;
        if !r.is_recording() {
            return Err("未在录音".to_string());
        }
        r.stop().map_err(|e| e.to_string())?
    };

    logger.info(
        "chat",
        "chat 语音录音完成",
        Some(serde_json::json!({
            "path": result.path.display().to_string(),
            "duration_secs": result.duration_secs,
            "format": result.format,
        })),
    );

    let data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let app_config = config::load_config(&data_dir);
    let transcription = &app_config.features.transcription;
    let provider = app_config
        .ai
        .providers
        .iter()
        .find(|p| p.id == transcription.provider_id)
        .cloned()
        .ok_or_else(|| "未找到转写 Provider".to_string())?;

    let audio_bytes = std::fs::read(&result.path).map_err(|e| e.to_string())?;
    let media_type = if result.format == "wav" {
        "audio/wav"
    } else {
        "audio/flac"
    };
    let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";

    let ctx = app_handle.state::<ProviderContext>();
    let model = transcription.model.clone();
    let text = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        ai::send_audio_prompt_from_bytes(
            &logger,
            &audio_bytes,
            media_type,
            prompt,
            &model,
            &provider,
            &ctx,
        ),
    )
    .await
    .map_err(|_| "转写超时,请重试".to_string())?
    .map_err(|e| e.to_string())?;

    let _ = std::fs::remove_file(&result.path);

    logger.info(
        "chat",
        "chat 语音转写完成",
        Some(serde_json::json!({ "text_preview": text.chars().take(50).collect::<String>() })),
    );
    Ok(text)
}
