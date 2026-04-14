use crate::ai;
use crate::config;
use crate::logger::Logger;
use crate::providers::ProviderContext;
use crate::shortcuts::{SHORTCUT_IDS, parse_shortcut};
use std::time::Instant;
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

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
pub fn set_onboarding_completed(app_handle: tauri::AppHandle, completed: bool) -> Result<(), String> {
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

    if let Some(sc) = parse_shortcut(&old_toggle) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    if let Some(sc) = parse_shortcut(&old_recording) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    if let Some(sc) = parse_shortcut(&old_translate) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }

    let new_toggle = parse_shortcut(&app_config.shortcut);
    let new_rec = parse_shortcut(&app_config.recording_shortcut);
    let new_translate = parse_shortcut(&app_config.translate_shortcut);

    {
        let mut ids = SHORTCUT_IDS.write().unwrap();
        ids.toggle = new_toggle.map(|s| s.id()).unwrap_or(0);
        ids.recording = new_rec.map(|s| s.id()).unwrap_or(0);
        ids.translate = new_translate.map(|s| s.id()).unwrap_or(0);
    }

    if let Some(sc) = new_toggle {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }
    if let Some(sc) = new_rec {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }
    if let Some(sc) = new_translate {
        app_handle
            .global_shortcut()
            .register(sc)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
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
    let project = std::env::var("GOOGLE_CLOUD_PROJECT").unwrap_or_default();
    let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string());
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
