mod ai;
mod clipboard;
mod config;
mod logger;
mod recording;

use logger::Logger;
use recording::AudioRecorder;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const TRAY_ID: &str = "main";

static RECORDING: AtomicBool = AtomicBool::new(false);
static LAST_REC_PRESS: Mutex<Option<Instant>> = Mutex::new(None);

struct ShortcutIds {
    toggle: u32,
    recording: u32,
}

static SHORTCUT_IDS: RwLock<ShortcutIds> = RwLock::new(ShortcutIds {
    toggle: 0,
    recording: 0,
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
) {
    RECORDING.store(false, Ordering::Relaxed);
    let duration = recording_start
        .lock()
        .ok()
        .and_then(|mut start| start.take().map(|s| s.elapsed().as_secs()))
        .unwrap_or(0);

    match event_name {
        "recording:complete" => match recorder.lock() {
            Ok(mut r) => match r.stop() {
                Ok(result) => {
                    println!(
                        "[TalkShow] Recording saved: {} ({}s, {})",
                        result.path.display(),
                        result.duration_secs,
                        result.format,
                    );
                    if result.format == "wav" {
                        show_notification(&app_handle, "FLAC 编码不可用", "已保存为 WAV 格式");
                    }
                    let _ = app_handle.emit("recording:complete", &result);

                    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
                    let app_config = config::load_config(&app_data_dir);
                    let transcription = &app_config.features.transcription;
                    let provider = app_config
                        .ai
                        .providers
                        .iter()
                        .find(|p| p.id == transcription.provider_id)
                        .cloned();

                    let audio_path = result.path.clone();
                    let model_name = transcription.model.clone();
                    let h = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let provider = match provider {
                            Some(p) => p,
                            None => {
                                show_notification(&h, "AI 处理失败", "未找到配置的 AI 提供商");
                                return;
                            }
                        };
                        let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
                        match ai::send_audio_prompt(&audio_path, prompt, &model_name, &provider).await {
                            Ok(text) => {
                                if let Err(e) = clipboard::write_and_paste(&text) {
                                    show_notification(&h, "剪贴板写入失败", &e);
                                }
                            }
                            Err(e) => {
                                show_notification(&h, "AI 处理失败", &e.to_string());
                            }
                        }
                    });
                }
                Err(recording::RecordingError::TooShort) => {
                    let cancelled = recording::RecordingCancelled {
                        duration_secs: duration,
                    };
                    let _ = app_handle.emit("recording:cancel", cancelled);
                }
                Err(e) => {
                    let _ = app_handle.emit("recording:error", e.to_string());
                }
            },
            Err(_) => {
                let _ = app_handle.emit("recording:error", "Recording lock poisoned");
            }
        },
        "recording:cancel" => {
            if let Ok(mut r) = recorder.lock() {
                let _duration = r.cancel();
            }
            println!("[TalkShow] Recording cancelled ({}s)", duration);
            let cancelled = recording::RecordingCancelled {
                duration_secs: duration,
            };
            let _ = app_handle.emit("recording:cancel", cancelled);
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

#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> config::AppConfig {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::load_config(&app_data_dir)
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

    match shortcut_type.as_str() {
        "toggle" => app_config.shortcut = shortcut,
        "recording" => app_config.recording_shortcut = shortcut,
        _ => return Err("Invalid shortcut type".to_string()),
    }

    config::save_config(&app_data_dir, &app_config)?;

    if let Some(sc) = parse_shortcut(&old_toggle) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    if let Some(sc) = parse_shortcut(&old_recording) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }

    let new_toggle = parse_shortcut(&app_config.shortcut);
    let new_rec = parse_shortcut(&app_config.recording_shortcut);

    {
        let mut ids = SHORTCUT_IDS.write().unwrap();
        ids.toggle = new_toggle.map(|s| s.id()).unwrap_or(0);
        ids.recording = new_rec.map(|s| s.id()).unwrap_or(0);
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

    Ok(())
}

#[tauri::command]
fn save_config_cmd(app_handle: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    config::save_config(&app_data_dir, &config)
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

    let test_audio: &[u8] = include_bytes!("../assets/test.wav");

    let start = Instant::now();
    let result = if is_transcription {
        ai::send_audio_prompt_from_bytes(
            test_audio,
            "audio/wav",
            "请将这段音频转录为文字",
            &model_name,
            &provider,
        )
        .await
    } else {
        ai::send_text_prompt("Hi", &model_name, &provider).await
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

    if let Some(p) = app_config.ai.providers.iter_mut().find(|p| p.id == provider_id) {
        if let Some(m) = p.models.iter_mut().find(|m| m.name == model_name) {
            m.verified = Some(verified);
        }
    }
    config::save_config(&app_data_dir, &app_config)?;

    Ok(TestResult {
        status,
        latency_ms: Some(latency),
        message,
    })
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
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_shortcut,
            save_config_cmd,
            test_model_connectivity,
            logger::get_log_sessions,
            logger::get_log_content
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().unwrap_or_default();
            let logger = Logger::new(&app_data_dir)
                .expect("Failed to initialize logger");
            let app_config = config::load_config(&app_data_dir);
            let shortcut_str = app_config.shortcut.clone();
            let recording_shortcut_str = app_config.recording_shortcut.clone();

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
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            toggle_window(&window);
                        }
                    }
                    _ => {}
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
            let esc_shortcut = Shortcut::new(None, Code::Escape);

            let toggle_id = toggle_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
            let rec_id = rec_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
            {
                let mut ids = SHORTCUT_IDS.write().unwrap();
                ids.toggle = toggle_id;
                ids.recording = rec_id;
            }
            let esc_id = esc_shortcut.id();

            let app_handle = app.handle().clone();
            let recording_start_handler = recording_start.clone();
            let recorder_handler = recorder.clone();
            let esc_shortcut_handler = esc_shortcut.clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |_app, shortcut, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        let id = shortcut.id();
                        let (_current_toggle_id, current_rec_id) = {
                            let ids = SHORTCUT_IDS.read().unwrap();
                            (ids.toggle, ids.recording)
                        };

                        if id == esc_id {
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:cancel",
                                );
                                play_sound("Pop.aiff");
                                let h = app_handle.clone();
                                let esc = esc_shortcut_handler.clone();
                                std::thread::spawn(move || {
                                    let _ = h.global_shortcut().unregister(esc);
                                });
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            }
                            return;
                        }

                        if current_rec_id != 0 && id == current_rec_id {
                            let now = Instant::now();
                            let should_ignore = LAST_REC_PRESS
                                .lock()
                                .ok()
                                .and_then(|mut last| {
                                    if let Some(t) = *last {
                                        if now.duration_since(t) < Duration::from_millis(500) {
                                            return Some(true);
                                        }
                                    }
                                    *last = Some(now);
                                    Some(false)
                                })
                                .unwrap_or(false);
                            if should_ignore {
                                return;
                            }
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                );
                                play_sound("Submarine.aiff");
                                let h = app_handle.clone();
                                let esc = esc_shortcut_handler.clone();
                                std::thread::spawn(move || {
                                    let _ = h.global_shortcut().unregister(esc);
                                });
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            } else {
                                let start_result =
                                    recorder_handler.lock().ok().and_then(|mut r| {
                                        let result = r.start();
                                        if result.is_ok() {
                                            Some(())
                                        } else {
                                            None
                                        }
                                    });
                                match start_result {
                                    Some(()) => {
                                        RECORDING.store(true, Ordering::Relaxed);
                                        if let Ok(mut start) = recording_start_handler.lock() {
                                            *start = Some(Instant::now());
                                        }
                                        if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                            let _ =
                                                tray.set_icon(Some(recording_icon_owned.clone()));
                                        }
                                        play_sound("Ping.aiff");
                                        let h = app_handle.clone();
                                        let esc = esc_shortcut_handler.clone();
                                        std::thread::spawn(move || {
                                            let _ = h.global_shortcut().register(esc);
                                        });
                                    }
                                    None => {
                                        let err_detail = recorder_handler
                                            .lock()
                                            .ok()
                                            .and_then(|mut r| r.start().err())
                                            .map(|e| e.to_string())
                                            .unwrap_or_else(|| "Unknown error".into());
                                        eprintln!("Failed to start recording: {}", err_detail);
                                        show_notification(&app_handle, "录音失败", &err_detail);
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

            if let Some(sc) = toggle_shortcut {
                if let Err(e) = app.global_shortcut().register(sc) {
                    eprintln!("Failed to register toggle shortcut: {}", e);
                }
            }
            if let Some(sc) = rec_shortcut {
                if let Err(e) = app.global_shortcut().register(sc) {
                    eprintln!("Failed to register recording shortcut: {}", e);
                }
            }

            // --- Tooltip update loop ---
            {
                let app_handle_tooltip = app.handle().clone();
                let recording_start_tooltip = recording_start.clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    let is_recording = RECORDING.load(Ordering::Relaxed);
                    if is_recording {
                        let tooltip = recording_start_tooltip
                            .lock()
                            .ok()
                            .and_then(|start| start.as_ref().map(format_elapsed));
                        if let Some(text) = tooltip {
                            if let Some(tray) = app_handle_tooltip.tray_by_id(TRAY_ID) {
                                let _ = tray.set_tooltip(Some(&text));
                            }
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

            app.manage(logger);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
