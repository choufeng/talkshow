mod ai;
mod audio_control;
mod clipboard;
mod config;
mod keyring_store;
mod llm_client;
mod logger;
#[cfg(target_os = "macos")]
mod macos;
mod real_llm_client;
mod recording;
mod sensevoice;
mod skills;
mod translation;

use logger::Logger;
use recording::AudioRecorder;
use sensevoice::SenseVoiceEngine;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    Emitter, Listener, Manager, WebviewWindow, WebviewWindowBuilder, image::Image, window::Color,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const TRAY_ID: &str = "main";

const RECORDING_MODE_NONE: u8 = 0;
const RECORDING_MODE_TRANSCRIPTION: u8 = 1;
const RECORDING_MODE_TRANSLATION: u8 = 2;

static RECORDING: AtomicU8 = AtomicU8::new(RECORDING_MODE_NONE);
static CANCELLED: AtomicBool = AtomicBool::new(false);
static LAST_REC_PRESS: Mutex<Option<Instant>> = Mutex::new(None);

struct SenseVoiceState {
    engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    language: Arc<Mutex<i32>>,
}

struct VertexClientState {
    client: Arc<Mutex<Option<rig_vertexai::Client>>>,
}

struct ShortcutIds {
    toggle: u32,
    recording: u32,
    translate: u32,
}

static SHORTCUT_IDS: RwLock<ShortcutIds> = RwLock::new(ShortcutIds {
    toggle: 0,
    recording: 0,
    translate: 0,
});

fn toggle_window(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.center();
        let _ = window.set_focus();
    }
}

fn parse_shortcut(shortcut_str: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match *part {
            "Control" => modifiers |= Modifiers::CONTROL,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Alt" => modifiers |= Modifiers::ALT,
            "Command" | "Super" => modifiers |= Modifiers::SUPER,
            s => {
                if let Ok(code) = s.parse::<Code>() {
                    key_code = Some(code);
                }
            }
        }
    }

    key_code.map(|code| Shortcut::new(Some(modifiers), code))
}

fn format_elapsed(start: &Instant) -> String {
    let elapsed = start.elapsed().as_secs();
    let mins = elapsed / 60;
    let secs = elapsed % 60;
    format!("\u{5f55}\u{97f3}\u{4e2d} {:02}:{:02}", mins, secs)
}

fn stop_recording(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<std::sync::Mutex<AudioRecorder>>,
    recording_start: &Arc<std::sync::Mutex<Option<Instant>>>,
    event_name: &str,
    recording_mode: u8,
) {
    let app_data_dir_restore = app_handle.path().app_data_dir().unwrap_or_default();
    let _ = audio_control::restore(
        &app_data_dir_restore,
        app_handle.try_state::<Logger>().as_deref(),
    );

    let duration = recording_start
        .lock()
        .ok()
        .and_then(|mut start| start.take().map(|s| s.elapsed().as_secs()))
        .unwrap_or(0);

    let logger = app_handle.try_state::<Logger>();

    if let Some(ref lg) = logger {
        lg.info(
            "recording",
            &format!("录音停止 ({})", event_name),
            Some(serde_json::json!({ "duration_secs": duration })),
        );
    }

    match event_name {
        "recording:complete" => match recorder.lock() {
            Ok(mut r) => {
                let save_start = Instant::now();
                let stop_result = r.stop();
                let save_elapsed = save_start.elapsed().as_millis();
                match stop_result {
                    Ok(result) => {
                        println!(
                            "[TalkShow] Recording saved: {} ({}s, {})",
                            result.path.display(),
                            result.duration_secs,
                            result.format,
                        );
                        if result.format == "wav" {
                            show_notification(app_handle, "FLAC 编码不可用", "已保存为 WAV 格式");
                        }
                        let _ = app_handle.emit("recording:complete", &result);
                        emit_indicator(app_handle, "indicator:processing");

                        if let Some(ref lg) = logger {
                            lg.info(
                                "recording",
                                "录音文件已保存",
                                Some(serde_json::json!({
                                    "path": result.path.display().to_string(),
                                    "duration_secs": result.duration_secs,
                                    "format": result.format,
                                    "save_ms": save_elapsed,
                                })),
                            );
                        }

                        let config_start = Instant::now();
                        let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
                        let app_config = config::load_config(&app_data_dir);
                        let config_elapsed = config_start.elapsed().as_millis();
                        let transcription = app_config.features.transcription.clone();
                        let provider = app_config
                            .ai
                            .providers
                            .iter()
                            .find(|p| p.id == transcription.provider_id)
                            .cloned();

                        let audio_path = result.path.clone();
                        let model_name = transcription.model.clone();
                        let skills_config = app_config.features.skills.clone();
                        let skills_providers = app_config.ai.providers.clone();
                        let h = app_handle.clone();
                        CANCELLED.store(false, Ordering::SeqCst);
                        tauri::async_runtime::spawn(async move {
                            let pipeline_start = Instant::now();
                            let _ = config_elapsed;

                            if let Some(lg) = h.try_state::<Logger>() {
                                lg.info(
                                    "pipeline",
                                    "流水线启动",
                                    Some(serde_json::json!({
                                        "config_load_ms": config_elapsed,
                                    })),
                                );
                            }

                            let provider = match provider {
                                Some(p) => p,
                                None => {
                                    show_notification(&h, "AI 处理失败", "未找到配置的 AI 提供商");
                                    if let Some(lg) = h.try_state::<Logger>() {
                                        lg.error("ai", "未找到配置的 AI 提供商", None);
                                    }
                                    destroy_indicator(&h);
                                    if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                        let _ = h
                                            .global_shortcut()
                                            .unregister(Shortcut::new(None, Code::Escape));
                                    }
                                    return;
                                }
                            };

                            if CANCELLED.load(Ordering::SeqCst) {
                                if let Some(lg) = h.try_state::<Logger>() {
                                    lg.info("pipeline", "流水线已取消 (AI请求前)", None);
                                }
                                destroy_indicator(&h);
                                if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                    let _ = h
                                        .global_shortcut()
                                        .unregister(Shortcut::new(None, Code::Escape));
                                }
                                return;
                            }

                            if let Some(lg) = h.try_state::<Logger>() {
                                lg.info(
                                    "ai",
                                    "开始发送 AI 转写请求",
                                    Some(serde_json::json!({
                                        "provider_id": provider.id,
                                        "provider_type": provider.provider_type,
                                        "model": model_name,
                                        "audio_path": audio_path.display().to_string(),
                                    })),
                                );
                            }

                            let logger = h.state::<Logger>();
                            let transcribe_start = Instant::now();
                            let text_result = if provider.provider_type == "sensevoice" {
                                let sv_state = h.state::<SenseVoiceState>();
                                let lang =
                                    *sv_state.language.lock().unwrap_or_else(|e| e.into_inner());
                                let app_data_dir = h.path().app_data_dir().unwrap_or_default();
                                let mdl_dir = app_data_dir.join("models").join("sensevoice");
                                {
                                    let guard =
                                        sv_state.engine.lock().unwrap_or_else(|e| e.into_inner());
                                    if guard.is_none() {
                                        drop(guard);
                                        match SenseVoiceEngine::new(&mdl_dir) {
                                            Ok(e) => {
                                                let mut g = sv_state
                                                    .engine
                                                    .lock()
                                                    .unwrap_or_else(|e| e.into_inner());
                                                *g = Some(e);
                                                logger.info(
                                                    "sensevoice",
                                                    "SenseVoice 引擎加载完成",
                                                    None,
                                                );
                                            }
                                            Err(e) => {
                                                logger.error("sensevoice", "SenseVoice 引擎加载失败", Some(serde_json::json!({ "error": e.to_string() })));
                                            }
                                        }
                                    }
                                }
                                let mut guard =
                                    sv_state.engine.lock().unwrap_or_else(|e| e.into_inner());
                                match guard.as_mut() {
                                    Some(engine) => engine
                                        .transcribe(&audio_path, lang)
                                        .map_err(|e| e.to_string()),
                                    None => Err("SenseVoice 引擎未初始化".to_string()),
                                }
                            } else {
                                let prompt =
                                    "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
                                ai::send_audio_prompt(
                                    &logger,
                                    &audio_path,
                                    prompt,
                                    &model_name,
                                    &provider,
                                    &h.state::<VertexClientState>().client,
                                )
                                .await
                                .map_err(|e| e.to_string())
                            };
                            let transcribe_elapsed = transcribe_start.elapsed().as_millis();
                            match text_result {
                                Ok(text) => {
                                    logger.info("ai", "AI 转写成功", Some(serde_json::json!({
                                    "transcribe_ms": transcribe_elapsed,
                                    "response_length": text.len(),
                                    "response_preview": text.chars().take(100).collect::<String>(),
                                })));

                                    let skills_start = Instant::now();
                                    let selected_text = clipboard::get_saved_selected_text();
                                    let selected_text_ref = selected_text.as_deref();
                                    let mut final_text = skills::process_with_skills(
                                        &logger,
                                        &skills_config,
                                        &app_config.features.transcription,
                                        &skills_providers,
                                        &text,
                                        &h.state::<VertexClientState>().client,
                                        selected_text_ref,
                                    )
                                    .await
                                    .unwrap_or_else(|e| {
                                        logger.error(
                                            "skills",
                                            &format!("Skills 处理异常，使用原始文字: {}", e),
                                            None,
                                        );
                                        text
                                    });
                                    let skills_elapsed = skills_start.elapsed().as_millis();

                                    if recording_mode == RECORDING_MODE_TRANSLATION {
                                        if transcription.polish_enabled
                                            && !transcription.polish_provider_id.is_empty()
                                            && !transcription.polish_model.is_empty()
                                        {
                                            let translate_config =
                                                app_config.features.translation.clone();
                                            match translation::translate_text(
                                                &logger,
                                                &final_text,
                                                &translate_config.target_lang,
                                                &skills_config,
                                                &transcription.polish_provider_id,
                                                &transcription.polish_model,
                                                &skills_providers,
                                                &h.state::<VertexClientState>().client,
                                            )
                                            .await
                                            {
                                                Ok(translated) => final_text = translated,
                                                Err(e) => {
                                                    logger.error("translation", &e, None);
                                                    show_notification(&h, "翻译失败", &e);
                                                    destroy_indicator(&h);
                                                    if RECORDING.load(Ordering::SeqCst)
                                                        == RECORDING_MODE_NONE
                                                    {
                                                        let _ = h.global_shortcut().unregister(
                                                            Shortcut::new(None, Code::Escape),
                                                        );
                                                    }
                                                    return;
                                                }
                                            }
                                        } else {
                                            show_notification(
                                                &h,
                                                "翻译失败",
                                                "请先启用润色并配置润色模型",
                                            );
                                            destroy_indicator(&h);
                                            if RECORDING.load(Ordering::SeqCst)
                                                == RECORDING_MODE_NONE
                                            {
                                                let _ = h
                                                    .global_shortcut()
                                                    .unregister(Shortcut::new(None, Code::Escape));
                                            }
                                            return;
                                        }
                                    }

                                    if CANCELLED.load(Ordering::SeqCst) {
                                        logger.info("pipeline", "流水线已取消", None);
                                        destroy_indicator(&h);
                                        if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                            let _ = h
                                                .global_shortcut()
                                                .unregister(Shortcut::new(None, Code::Escape));
                                        }
                                        return;
                                    }

                                    if RECORDING.load(Ordering::SeqCst) != RECORDING_MODE_NONE {
                                        logger.info("ai", "录音已重新开始，丢弃当前 AI 结果", None);
                                        return;
                                    }

                                    let clipboard_start = Instant::now();
                                    match clipboard::write_and_paste(&final_text) {
                                        Ok(()) => {
                                            let clipboard_elapsed =
                                                clipboard_start.elapsed().as_millis();
                                            let total_elapsed =
                                                pipeline_start.elapsed().as_millis();
                                            logger.info(
                                                "clipboard",
                                                "剪贴板写入并粘贴成功",
                                                Some(serde_json::json!({
                                                    "text_length": final_text.len(),
                                                })),
                                            );
                                            logger.info(
                                                "pipeline",
                                                "流水线完成",
                                                Some(serde_json::json!({
                                                    "total_ms": total_elapsed,
                                                    "transcribe_ms": transcribe_elapsed,
                                                    "skills_ms": skills_elapsed,
                                                    "clipboard_ms": clipboard_elapsed,
                                                })),
                                            );
                                            emit_indicator(&h, "indicator:done");
                                            if RECORDING.load(Ordering::SeqCst)
                                                == RECORDING_MODE_NONE
                                            {
                                                let _ = h
                                                    .global_shortcut()
                                                    .unregister(Shortcut::new(None, Code::Escape));
                                            }
                                        }
                                        Err(e) => {
                                            logger.error(
                                                "clipboard",
                                                "剪贴板写入/粘贴失败",
                                                Some(serde_json::json!({ "error": e })),
                                            );
                                            show_notification(&h, "剪贴板写入失败", &e);
                                            destroy_indicator(&h);
                                            if RECORDING.load(Ordering::SeqCst)
                                                == RECORDING_MODE_NONE
                                            {
                                                let _ = h
                                                    .global_shortcut()
                                                    .unregister(Shortcut::new(None, Code::Escape));
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    logger.error(
                                        "ai",
                                        "AI 转写失败",
                                        Some(serde_json::json!({
                                            "transcribe_ms": transcribe_elapsed,
                                            "error": e.to_string(),
                                        })),
                                    );
                                    show_notification(&h, "AI 处理失败", &e);
                                    destroy_indicator(&h);
                                    if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                        let _ = h
                                            .global_shortcut()
                                            .unregister(Shortcut::new(None, Code::Escape));
                                    }
                                }
                            }
                        });
                    }
                    Err(recording::RecordingError::TooShort) => {
                        if let Some(ref lg) = logger {
                            lg.info(
                                "recording",
                                "录音时间过短，已丢弃",
                                Some(serde_json::json!({ "duration_secs": duration })),
                            );
                        }
                        let cancelled = recording::RecordingCancelled {
                            duration_secs: duration,
                        };
                        let _ = app_handle.emit("recording:cancel", cancelled);
                        destroy_indicator(app_handle);
                        let _ = app_handle
                            .global_shortcut()
                            .unregister(Shortcut::new(None, Code::Escape));
                    }
                    Err(e) => {
                        if let Some(ref lg) = logger {
                            lg.error(
                                "recording",
                                "录音停止失败",
                                Some(serde_json::json!({ "error": e.to_string() })),
                            );
                        }
                        let _ = app_handle.emit("recording:error", e.to_string());
                        destroy_indicator(app_handle);
                        let _ = app_handle
                            .global_shortcut()
                            .unregister(Shortcut::new(None, Code::Escape));
                    }
                }
            }
            Err(_) => {
                let _ = app_handle.emit("recording:error", "Recording lock poisoned");
                destroy_indicator(app_handle);
                let _ = app_handle
                    .global_shortcut()
                    .unregister(Shortcut::new(None, Code::Escape));
            }
        },
        "recording:cancel" => {
            if let Ok(mut r) = recorder.lock() {
                let _duration = r.cancel();
            }
            println!("[TalkShow] Recording cancelled ({}s)", duration);
            if let Some(ref lg) = logger {
                lg.info(
                    "recording",
                    "录音已取消",
                    Some(serde_json::json!({ "duration_secs": duration })),
                );
            }
            let cancelled = recording::RecordingCancelled {
                duration_secs: duration,
            };
            let _ = app_handle.emit("recording:cancel", cancelled);
            destroy_indicator(app_handle);
        }
        _ => {}
    }
}

fn restore_default_tray(app_handle: &tauri::AppHandle, default_icon: Image) {
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_icon(Some(default_icon));
        let _ = tray.set_tooltip(Some("TalkShow"));
    }
}

const INDICATOR_LABEL: &str = "recording-indicator";

fn show_indicator(app_handle: &tauri::AppHandle, selected_text: Option<&str>) {
    let payload = serde_json::json!({
        "replaceMode": selected_text.is_some(),
        "selectedPreview": selected_text.map(|t| t.chars().take(50).collect::<String>()).unwrap_or_default()
    });
    let existing = app_handle.get_webview_window(INDICATOR_LABEL);
    if existing.is_some() {
        let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
        return;
    }

    let main_window = app_handle.get_webview_window("main");
    let monitor = main_window
        .as_ref()
        .and_then(|w| w.primary_monitor().ok().flatten());

    let (x, y) = match &monitor {
        Some(m) => {
            let size = m.size();
            let scale = m.scale_factor();
            let screen_w = size.width as f64 / scale;
            let screen_h = size.height as f64 / scale;
            let win_w = 180.0;
            let win_h = 48.0;
            let bottom_margin = 24.0;
            ((screen_w - win_w) / 2.0, screen_h - win_h - bottom_margin)
        }
        None => (620.0, 700.0),
    };

    let url = "/recording";

    let window = WebviewWindowBuilder::new(
        app_handle,
        INDICATOR_LABEL,
        tauri::WebviewUrl::App(url.into()),
    )
    .inner_size(180.0, 48.0)
    .position(x, y)
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .focusable(false)
    .accept_first_mouse(true)
    .build();

    match window {
        Ok(w) => {
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = macos::floating_panel::make_window_nonactivating(&w) {
                    eprintln!("Failed to make window nonactivating: {}", e);
                }
            }

            let _ = w.show();
            let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
        }
        Err(e) => {
            eprintln!("Failed to create indicator window: {}", e);
        }
    }
}

fn emit_indicator(app_handle: &tauri::AppHandle, event: &str) {
    let _ = app_handle.emit_to(INDICATOR_LABEL, event, ());
}

fn destroy_indicator(app_handle: &tauri::AppHandle) {
    if let Some(w) = app_handle.get_webview_window(INDICATOR_LABEL) {
        let _ = w.close();
    }
}

#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut config = config::load_config(&app_data_dir);

    // 从 keyring 读取 api_key
    let provider_ids: Vec<String> = config.ai.providers.iter().map(|p| p.id.clone()).collect();
    let keyring_keys = keyring_store::load_all_api_keys(&provider_ids);

    for provider in &mut config.ai.providers {
        if let Some(key) = keyring_keys.get(&provider.id) {
            provider.api_key = Some(key.clone());
        }
    }

    config::mask_api_keys(config)
}

#[tauri::command]
fn update_shortcut(
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
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::validate_config(&config)?;

    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();

    // 首次保存时迁移旧密钥到 keyring
    let existing_config = config::load_config(&app_data_dir);
    for provider in &existing_config.ai.providers {
        if let Some(ref key) = provider.api_key {
            if !key.is_empty() {
                let _ = keyring_store::store_api_key(&provider.id, key);
            }
        }
    }

    // 保存 api_key 到 keyring，从 JSON 中剥离
    let (clean_config, keys) = config::strip_api_keys(config);
    for (provider_id, api_key) in keys {
        if let Some(key) = api_key {
            if !key.is_empty() && !key.contains("...") {
                keyring_store::store_api_key(&provider_id, &key)?;
            }
        }
    }

    config::save_config(&app_data_dir, &clean_config)
}

#[derive(serde::Serialize, Clone)]
struct TestResult {
    status: String,
    latency_ms: Option<u64>,
    message: String,
}

#[tauri::command]
async fn test_model_connectivity(
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

    if provider.provider_type == "sensevoice" {
        return Ok(TestResult {
            status: "ok".to_string(),
            latency_ms: Some(0),
            message: "本地模型，无需连通性测试".to_string(),
        });
    }

    let start = Instant::now();
    let vertex_cache = app_handle.state::<VertexClientState>().client.clone();
    let result = if provider.provider_type == "vertex" {
        ai::send_text_prompt(
            &logger,
            "Hi",
            &model_name,
            &provider,
            &vertex_cache,
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
            &vertex_cache,
        )
        .await
    } else {
        ai::send_text_prompt(
            &logger,
            "Hi",
            &model_name,
            &provider,
            &vertex_cache,
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
struct VertexEnvInfo {
    project: String,
    location: String,
}

#[tauri::command]
fn get_vertex_env_info() -> VertexEnvInfo {
    let project = std::env::var("GOOGLE_CLOUD_PROJECT").unwrap_or_default();
    let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string());
    VertexEnvInfo { project, location }
}

#[tauri::command]
fn get_skills_config(app_handle: tauri::AppHandle) -> config::SkillsConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let app_config = config::load_config(&app_data_dir);
    app_config.features.skills
}

#[tauri::command]
fn save_skills_config(
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
fn save_transcription_config(
    app_handle: tauri::AppHandle,
    transcription: config::TranscriptionConfig,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.transcription = transcription;
    config::save_config(&app_data_dir, &app_config)
}

#[tauri::command]
fn add_skill(app_handle: tauri::AppHandle, skill: config::Skill) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);
    app_config.features.skills.skills.push(skill);
    app_config.features.skills.enabled = true;
    config::save_config(&app_data_dir, &app_config)
}

#[tauri::command]
fn update_skill(app_handle: tauri::AppHandle, skill: config::Skill) -> Result<(), String> {
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
fn delete_skill(app_handle: tauri::AppHandle, skill_id: String) -> Result<(), String> {
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

fn show_notification(app_handle: &tauri::AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    app_handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .ok();
}

#[allow(unused_variables)]
fn play_sound(sound_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let sound_path = format!("/System/Library/Sounds/{}", sound_name);
        std::thread::spawn(move || {
            let _ = std::process::Command::new("afplay")
                .arg(&sound_path)
                .spawn();
        });
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let window = app.get_webview_window("main").unwrap();
            window.set_focus().unwrap();
        }))
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_shortcut,
            save_config_cmd,
            test_model_connectivity,
            get_vertex_env_info,
            get_skills_config,
            save_skills_config,
            save_transcription_config,
            add_skill,
            update_skill,
            delete_skill,
            sensevoice::get_sensevoice_status,
            sensevoice::download_sensevoice_model,
            sensevoice::delete_sensevoice_model,
            logger::get_log_sessions,
            logger::get_log_content,
            clipboard::get_replace_mode_state
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().unwrap_or_default();
            let _ = audio_control::cleanup_stale_state(&app_data_dir);
            let logger = Logger::new(&app_data_dir)
                .expect("Failed to initialize logger");
            let app_config = config::load_config(&app_data_dir);
            let shortcut_str = app_config.shortcut.clone();
            let recording_shortcut_str = app_config.recording_shortcut.clone();
            let translate_shortcut_str = app_config.translate_shortcut.clone();

            let default_icon = app.default_window_icon().unwrap().clone();
            let recording_bytes = include_bytes!("../icons/recording.png");
            let img = image::load_from_memory(recording_bytes)
                .expect("failed to decode recording icon")
                .to_rgba8();
            let (w, h) = (img.width(), img.height());
            let recording_icon = Image::new_owned(img.into_raw(), w, h);

            let default_icon_rgba = default_icon.rgba();
            let (dw, dh) = (default_icon.width(), default_icon.height());
            let default_icon_owned = Image::new_owned(default_icon_rgba.to_vec(), dw, dh);
            let recording_icon_rgba = recording_icon.rgba();
            let (rw, rh) = (recording_icon.width(), recording_icon.height());
            let recording_icon_owned = Image::new_owned(recording_icon_rgba.to_vec(), rw, rh);

            // --- System Tray ---
            let show_i = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(default_icon_owned.clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("TalkShow")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            toggle_window(&window);
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        toggle_window(&window);
                    }
                })
                .build(app)?;

            // --- Recording State ---
            let recording_start: Arc<std::sync::Mutex<Option<Instant>>> =
                Arc::new(std::sync::Mutex::new(None));

            let recorder: Arc<std::sync::Mutex<AudioRecorder>> =
                Arc::new(std::sync::Mutex::new(AudioRecorder::new()));

            // --- Global Shortcuts (single plugin instance) ---
            let toggle_shortcut = parse_shortcut(&shortcut_str);
            let rec_shortcut = parse_shortcut(&recording_shortcut_str);
            let translate_shortcut = parse_shortcut(&translate_shortcut_str);
            let esc_shortcut = Shortcut::new(None, Code::Escape);

            let toggle_id = toggle_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
            let rec_id = rec_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
            let translate_id = translate_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
            {
                let mut ids = SHORTCUT_IDS.write().unwrap();
                ids.toggle = toggle_id;
                ids.recording = rec_id;
                ids.translate = translate_id;
            }
            let esc_id = esc_shortcut.id();

            {
                let app_handle_cancel = app.handle().clone();
                let recorder_cancel = recorder.clone();
                let recording_start_cancel = recording_start.clone();
                let esc_cancel = esc_shortcut;
                let default_icon_cancel = default_icon_owned.clone();
                let _ = app.listen("indicator:cancel", move |_event| {
                    let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
                    if is_recording {
                        let mode = RECORDING.load(Ordering::Relaxed);
                        RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
                        stop_recording(
                            &app_handle_cancel,
                            &recorder_cancel,
                            &recording_start_cancel,
                            "recording:cancel",
                            mode,
                        );
                    } else {
                        CANCELLED.store(true, Ordering::SeqCst);
                    }
                    destroy_indicator(&app_handle_cancel);
                    play_sound("Pop.aiff");
                    let h = app_handle_cancel.clone();
                    let esc = esc_cancel;
                    std::thread::spawn(move || {
                        let _ = h.global_shortcut().unregister(esc);
                    });
                    restore_default_tray(&app_handle_cancel, default_icon_cancel.clone());
                });
            }

            let app_handle = app.handle().clone();
            let recording_start_handler = recording_start.clone();
            let recorder_handler = recorder.clone();
            let esc_shortcut_handler = esc_shortcut;
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |_app, shortcut, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        let id = shortcut.id();
                        let (_current_toggle_id, current_rec_id, current_translate_id) = {
                            let ids = SHORTCUT_IDS.read().unwrap();
                            (ids.toggle, ids.recording, ids.translate)
                        };

                        if id == esc_id {
                            let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
                            if is_recording {
                                let mode = RECORDING.load(Ordering::Relaxed);
                                RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:cancel",
                                    mode,
                                );
                                play_sound("Pop.aiff");
                            } else if app_handle.get_webview_window(INDICATOR_LABEL).is_some() {
                                CANCELLED.store(true, Ordering::SeqCst);
                                destroy_indicator(&app_handle);
                                play_sound("Pop.aiff");
                            } else {
                                return;
                            }
                            let h = app_handle.clone();
                            let esc = esc_shortcut_handler;
                            std::thread::spawn(move || {
                                let _ = h.global_shortcut().unregister(esc);
                            });
                            restore_default_tray(&app_handle, default_icon_owned.clone());
                            return;
                        }

                        if current_rec_id != 0 && id == current_rec_id {
                            let now = Instant::now();
                            let should_ignore = LAST_REC_PRESS
                                .lock()
                                .ok()
                                .map(|mut last| {
                                    if let Some(t) = *last
                                        && now.duration_since(t) < Duration::from_millis(500) {
                                            return true;
                                        }
                                    *last = Some(now);
                                    false
                                })
                                .unwrap_or(false);
                            if should_ignore {
                                return;
                            }
                            let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
                            if is_recording {
                                let mode = RECORDING.load(Ordering::Relaxed);
                                RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                    mode,
                                );
                                play_sound("Submarine.aiff");
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            } else {
                                let start_result =
                                    RECORDING.compare_exchange(
                                        RECORDING_MODE_NONE,
                                        RECORDING_MODE_TRANSCRIPTION,
                                        Ordering::SeqCst,
                                        Ordering::SeqCst,
                                    );
                                match start_result {
                                    Ok(_) => {
                                        let app_data_dir_rec = app_handle.path().app_data_dir().unwrap_or_default();
                                        let rec_result =
                                            recorder_handler.lock().ok().and_then(|mut r| {
                                                r.set_output_dir(app_data_dir_rec);
                                                let result = r.start();
                                                if result.is_ok() {
                                                    Some(())
                                                } else {
                                                    None
                                                }
                                            });
                                        match rec_result {
                                            Some(()) => {
                                                if let Ok(mut start) = recording_start_handler.lock() {
                                                    *start = Some(Instant::now());
                                                }
                                                if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                                    let _ =
                                                        tray.set_icon(Some(recording_icon_owned.clone()));
                                                }
                                                let frontmost = std::process::Command::new("osascript")
                                                    .arg("-e")
                                                    .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
                                                    .output()
                                                    .ok()
                                                    .filter(|o| o.status.success())
                                                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
                                                if let Some(ref app) = frontmost {
                                                    clipboard::save_target_app(app);
                                                }
                                                let selected_text = clipboard::detect_selected_text(frontmost.as_deref().unwrap_or(""));
                                                if let Some(ref text) = selected_text {
                                                    clipboard::save_selected_text(text);
                                                    if let Some(logger) = app_handle.try_state::<Logger>() {
                                                        logger.info("recording", "检测到选中文本，进入替换模式", Some(serde_json::json!({
                                                            "selected_length": text.len(),
                                                            "selected_preview": text.chars().take(100).collect::<String>()
                                                        })));
                                                    }
                                                }
                                                play_sound("Ping.aiff");
                                                {
                                                    let app_data_dir_mute = app_handle.path().app_data_dir().unwrap_or_default();
                                                    let app_config_mute = config::load_config(&app_data_dir_mute);
                                                    if app_config_mute.features.recording.auto_mute {
                                                        let _ = audio_control::save_and_mute(
                                                            &app_data_dir_mute,
                                                            app_handle.try_state::<Logger>().as_deref(),
                                                        );
                                                    }
                                                }
                                                show_indicator(&app_handle, selected_text.as_deref());
                                                if let Some(mw) = app_handle.get_webview_window("main")
                                                    && mw.is_visible().unwrap_or(false) {
                                                        let _ = mw.hide();
                                                    }
                                                if let Some(logger) = app_handle.try_state::<Logger>() {
                                                    logger.info("recording", "录音开始", None);
                                                }
                                                let h = app_handle.clone();
                                                let esc = esc_shortcut_handler;
                                                std::thread::spawn(move || {
                                                    let _ = h.global_shortcut().register(esc);
                                                });
                                            }
                                            None => {
                                                RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
                                                let err_detail = recorder_handler
                                                    .lock()
                                                    .ok()
                                                    .and_then(|mut r| r.start().err())
                                                    .map(|e| e.to_string())
                                                    .unwrap_or_else(|| "Unknown error".into());
                                                eprintln!("Failed to start recording: {}", err_detail);
                                                if let Some(logger) = app_handle.try_state::<Logger>() {
                                                    logger.error("recording", "录音启动失败", Some(serde_json::json!({ "error": err_detail })));
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        return;
                                    }
                                }
                            }
                            return;
                        }

                        if current_translate_id != 0 && id == current_translate_id {
                            let now = Instant::now();
                            let should_ignore = LAST_REC_PRESS
                                .lock()
                                .ok()
                                .map(|mut last| {
                                    if let Some(t) = *last
                                        && now.duration_since(t) < Duration::from_millis(500) {
                                            return true;
                                        }
                                    *last = Some(now);
                                    false
                                })
                                .unwrap_or(false);
                            if should_ignore {
                                return;
                            }
                            let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
                            if is_recording {
                                let mode = RECORDING.load(Ordering::Relaxed);
                                RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                    mode,
                                );
                                play_sound("Submarine.aiff");
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            } else {
                                let start_result =
                                    RECORDING.compare_exchange(
                                        RECORDING_MODE_NONE,
                                        RECORDING_MODE_TRANSLATION,
                                        Ordering::SeqCst,
                                        Ordering::SeqCst,
                                    );
                                match start_result {
                                    Ok(_) => {
                                        let app_data_dir_rec = app_handle.path().app_data_dir().unwrap_or_default();
                                        let rec_result =
                                            recorder_handler.lock().ok().and_then(|mut r| {
                                                r.set_output_dir(app_data_dir_rec);
                                                let result = r.start();
                                                if result.is_ok() {
                                                    Some(())
                                                } else {
                                                    None
                                                }
                                            });
                                        match rec_result {
                                            Some(()) => {
                                                if let Ok(mut start) = recording_start_handler.lock() {
                                                    *start = Some(Instant::now());
                                                }
                                                if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                                    let _ =
                                                        tray.set_icon(Some(recording_icon_owned.clone()));
                                                }
                                                let frontmost = std::process::Command::new("osascript")
                                                    .arg("-e")
                                                    .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
                                                    .output()
                                                    .ok()
                                                    .filter(|o| o.status.success())
                                                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
                                                if let Some(ref app) = frontmost {
                                                    clipboard::save_target_app(app);
                                                }
                                                play_sound("Ping.aiff");
                                                {
                                                    let app_data_dir_mute = app_handle.path().app_data_dir().unwrap_or_default();
                                                    let app_config_mute = config::load_config(&app_data_dir_mute);
                                                    if app_config_mute.features.recording.auto_mute {
                                                        let _ = audio_control::save_and_mute(
                                                            &app_data_dir_mute,
                                                            app_handle.try_state::<Logger>().as_deref(),
                                                        );
                                                    }
                                                }
                                                show_indicator(&app_handle, None);
                                                if let Some(mw) = app_handle.get_webview_window("main")
                                                    && mw.is_visible().unwrap_or(false) {
                                                        let _ = mw.hide();
                                                    }
                                                if let Some(logger) = app_handle.try_state::<Logger>() {
                                                    logger.info("recording", "录音开始 (翻译模式)", None);
                                                }
                                                let h = app_handle.clone();
                                                let esc = esc_shortcut_handler;
                                                std::thread::spawn(move || {
                                                    let _ = h.global_shortcut().register(esc);
                                                });
                                            }
                                            None => {
                                                RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
                                                let err_detail = recorder_handler
                                                    .lock()
                                                    .ok()
                                                    .and_then(|mut r| r.start().err())
                                                    .map(|e| e.to_string())
                                                    .unwrap_or_else(|| "Unknown error".into());
                                                eprintln!("Failed to start recording: {}", err_detail);
                                                if let Some(logger) = app_handle.try_state::<Logger>() {
                                                    logger.error("recording", "录音启动失败", Some(serde_json::json!({ "error": err_detail })));
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        return;
                                    }
                                }
                            }
                            return;
                        }

                        if let Some(window) = app_handle.get_webview_window("main") {
                            toggle_window(&window);
                        }
                    })
                    .build(),
            )?;

            if let Some(sc) = toggle_shortcut
                && let Err(e) = app.global_shortcut().register(sc) {
                    eprintln!("Failed to register toggle shortcut: {}", e);
                }
            if let Some(sc) = rec_shortcut
                && let Err(e) = app.global_shortcut().register(sc) {
                    eprintln!("Failed to register recording shortcut: {}", e);
                }
            if let Some(sc) = translate_shortcut
                && let Err(e) = app.global_shortcut().register(sc) {
                    eprintln!("Failed to register translate shortcut: {}", e);
                }

            // --- Tooltip update loop ---
            {
                let app_handle_tooltip = app.handle().clone();
                let recording_start_tooltip = recording_start.clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
                    if is_recording {
                        let tooltip = recording_start_tooltip
                            .lock()
                            .ok()
                            .and_then(|start| start.as_ref().map(format_elapsed));
                        if let Some(text) = tooltip
                            && let Some(tray) = app_handle_tooltip.tray_by_id(TRAY_ID) {
                                let _ = tray.set_tooltip(Some(&text));
                            }
                    }
                });
            }

            // --- Close window -> hide ---
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            let sensevoice_state = SenseVoiceState {
                engine: Arc::new(Mutex::new(None)),
                language: Arc::new(Mutex::new(0)),
            };
            app.manage(sensevoice_state);

            let vertex_state = VertexClientState {
                client: Arc::new(Mutex::new(None)),
            };
            app.manage(vertex_state);

            app.manage(logger);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_control_shift_quote() {
        let result = parse_shortcut("Control+Shift+Quote");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_control_backslash() {
        let result = parse_shortcut("Control+Backslash");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_single_key() {
        let result = parse_shortcut("KeyA");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_empty_string() {
        let result = parse_shortcut("");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_shortcut_only_modifiers() {
        let result = parse_shortcut("Control+Shift");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_shortcut_command_super_alias() {
        let cmd_result = parse_shortcut("Command+KeyA");
        let super_result = parse_shortcut("Super+KeyA");
        assert_eq!(cmd_result.is_some(), super_result.is_some());
    }

    #[test]
    fn test_format_elapsed() {
        let start = Instant::now();
        assert!(format_elapsed(&start).contains("录音中"));
    }

    #[test]
    fn test_recording_mode_constants() {
        assert_eq!(RECORDING_MODE_NONE, 0);
        assert_eq!(RECORDING_MODE_TRANSCRIPTION, 1);
        assert_eq!(RECORDING_MODE_TRANSLATION, 2);
    }
}
