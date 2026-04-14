mod ai;
mod audio_control;
mod clipboard;
mod commands;
mod config;
mod llm_client;
mod logger;
#[cfg(target_os = "macos")]
mod macos;
mod providers;
mod real_llm_client;
mod recording;
mod sensevoice;
mod skills;
mod translation;
mod indicator;
mod pipeline;
mod shortcuts;

use indicator::{INDICATOR_LABEL, TRAY_ID, destroy_indicator, restore_default_tray, show_indicator};

use pipeline::{SenseVoiceState, play_sound, stop_recording};

// Re-export types and functions for integration tests
pub use config::{
    AiConfig, AppConfig, FeaturesConfig, ModelConfig, ModelVerified, ProviderConfig,
    RecordingFeaturesConfig, Skill, SkillsConfig, TranscriptionConfig, TranslationConfig,
    load_config, save_config, validate_config,
};
pub use llm_client::LlmClient;
pub use logger::Logger;
pub use skills::{assemble_skills_prompt, process_with_skills_client};
pub use translation::translate_text_client;

use providers::ProviderContext;
use recording::AudioRecorder;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    Listener, Manager, WebviewWindow, WebviewWindowBuilder, image::Image, window::Color,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

use shortcuts::{
    parse_shortcut, CANCELLED, LAST_REC_PRESS, RECORDING, RECORDING_MODE_NONE,
    RECORDING_MODE_TRANSCRIPTION, RECORDING_MODE_TRANSLATION, SHORTCUT_IDS,
};

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

fn format_elapsed(start: &Instant) -> String {
    let elapsed = start.elapsed().as_secs();
    let mins = elapsed / 60;
    let secs = elapsed % 60;
    format!("\u{5f55}\u{97f3}\u{4e2d} {:02}:{:02}", mins, secs)
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
            commands::get_config,
            commands::get_onboarding_status,
            commands::set_onboarding_completed,
            commands::update_shortcut,
            commands::save_config_cmd,
            commands::test_model_connectivity,
            commands::get_vertex_env_info,
            commands::get_skills_config,
            commands::save_skills_config,
            commands::save_transcription_config,
            commands::add_skill,
            commands::update_skill,
            commands::delete_skill,
            sensevoice::get_sensevoice_status,
            sensevoice::download_sensevoice_model,
            sensevoice::delete_sensevoice_model,
            logger::get_log_sessions,
            logger::get_log_content
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
                                                // === Phase 1: Immediate response (< 10ms) ===
                                                if let Ok(mut start) = recording_start_handler.lock() {
                                                    *start = Some(Instant::now());
                                                }
                                                if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                                    let _ =
                                                        tray.set_icon(Some(recording_icon_owned.clone()));
                                                }

                                                // Show indicator immediately
                                                show_indicator(&app_handle);
                                                play_sound("Ping.aiff");

                                                // === Phase 2: Background async operations (~300ms) ===
                                                let app_handle_bg = app_handle.clone();
                                                let esc_bg = esc_shortcut_handler;

                                                std::thread::spawn(move || {
                                                    // Get frontmost app name
                                                    let frontmost = std::process::Command::new("osascript")
                                                        .arg("-e")
                                                        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
                                                        .output()
                                                        .ok()
                                                        .filter(|o| o.status.success())
                                                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

                                                    // Save target app for later paste
                                                    if let Some(ref app) = frontmost {
                                                        clipboard::save_target_app(app);
                                                    }

                                                    // Auto mute
                                                    let app_data_dir_mute = app_handle_bg.path().app_data_dir().unwrap_or_default();
                                                    let app_config_mute = config::load_config(&app_data_dir_mute);
                                                    if app_config_mute.features.recording.auto_mute {
                                                        let _ = audio_control::save_and_mute(
                                                            &app_data_dir_mute,
                                                            app_handle_bg.try_state::<Logger>().as_deref(),
                                                        );
                                                    }

                                                    // Register ESC shortcut
                                                    let h = app_handle_bg.clone();
                                                    let _ = h.global_shortcut().register(esc_bg);

                                                    // Log
                                                    if let Some(logger) = app_handle_bg.try_state::<Logger>() {
                                                        logger.info("recording", "录音开始", None);
                                                    }
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
                                                // === Phase 1: Immediate response (< 10ms) ===
                                                if let Ok(mut start) = recording_start_handler.lock() {
                                                    *start = Some(Instant::now());
                                                }
                                                if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                                    let _ =
                                                        tray.set_icon(Some(recording_icon_owned.clone()));
                                                }

                                                // Show indicator immediately
                                                show_indicator(&app_handle);
                                                play_sound("Ping.aiff");

                                                // === Phase 2: Background async operations ===
                                                let app_handle_bg = app_handle.clone();
                                                let esc_bg = esc_shortcut_handler;

                                                std::thread::spawn(move || {
                                                    // Auto mute
                                                    let app_data_dir_mute = app_handle_bg.path().app_data_dir().unwrap_or_default();
                                                    let app_config_mute = config::load_config(&app_data_dir_mute);
                                                    if app_config_mute.features.recording.auto_mute {
                                                        let _ = audio_control::save_and_mute(
                                                            &app_data_dir_mute,
                                                            app_handle_bg.try_state::<Logger>().as_deref(),
                                                        );
                                                    }

                                                    // Register ESC shortcut
                                                    let h = app_handle_bg.clone();
                                                    let _ = h.global_shortcut().register(esc_bg);

                                                    // Log
                                                    if let Some(logger) = app_handle_bg.try_state::<Logger>() {
                                                        logger.info("recording", "录音开始 (翻译模式)", None);
                                                    }
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

            let provider_ctx = ProviderContext::new();
            app.manage(provider_ctx);

            app.manage(logger);

            // --- Pre-create indicator window for instant show ---
            let indicator_url = tauri::WebviewUrl::App("/recording".into());
            let indicator_window = WebviewWindowBuilder::new(app.handle(), INDICATOR_LABEL, indicator_url)
                .inner_size(180.0, 48.0)
                .position(620.0, 700.0)
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

            #[cfg(target_os = "macos")]
            {
                if let Ok(w) = &indicator_window {
                    let _ = macos::floating_panel::make_window_nonactivating(w);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = indicator_window;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_elapsed() {
        let start = Instant::now();
        assert!(format_elapsed(&start).contains("录音中"));
    }
}
