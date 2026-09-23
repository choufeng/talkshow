mod ai;
mod audio_control;
mod clipboard;
mod commands;
mod config;
mod indicator;
mod llm_client;
mod logger;
#[cfg(target_os = "macos")]
mod macos;
mod pipeline;
mod providers;
mod real_llm_client;
mod recording;
mod sensevoice;
mod session;
mod shortcuts;
mod skills;
mod sys;
mod translation;

pub use config::{
    AiConfig, AppConfig, FeaturesConfig, ModelConfig, ModelVerified, ProviderConfig,
    RecordingFeaturesConfig, Skill, SkillsConfig, TranscriptionConfig, TranslationConfig,
    load_config, save_config, validate_config,
};
pub use llm_client::LlmClient;
pub use logger::Logger;
pub use skills::{assemble_skills_prompt, process_with_skills_client};
pub use translation::translate_text_client;

use indicator::{
    INDICATOR_LABEL, TRAY_ID, destroy_indicator, restore_default_tray, show_indicator,
};
use pipeline::{SenseVoiceState, play_sound, stop_recording};
use providers::ProviderContext;
use recording::AudioRecorder;
use session::{MODE_TRANSCRIPTION, MODE_TRANSLATION, SessionManager};
use shortcuts::{LAST_REC_PRESS, SHORTCUT_IDS, parse_shortcut};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Listener, Manager, WebviewWindow, WebviewWindowBuilder, image::Image, window::Color};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

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

/// Tray 图标集(default / recording),managed state 供各处取用。
struct TrayIcons {
    default: Image<'static>,
    recording: Image<'static>,
}

/// 按键去抖:500ms 内重复按压忽略。
fn debounced() -> bool {
    let now = Instant::now();
    LAST_REC_PRESS
        .lock()
        .ok()
        .map(|mut last| {
            if let Some(t) = *last
                && now.duration_since(t) < Duration::from_millis(500)
            {
                return true;
            }
            *last = Some(now);
            false
        })
        .unwrap_or(false)
}

/// 开始一次录音会话(转写/翻译共用)。成功时立即返回(UI 反馈已发出),
/// 副作用在后台线程执行,以 session.id 为 checkpoint。
fn begin_session(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<Mutex<AudioRecorder>>,
    recording_start: &Arc<Mutex<Option<Instant>>>,
    mode: u8,
    esc_shortcut: Shortcut,
    recording_icon: Image,
) {
    let manager = app_handle.state::<SessionManager>();
    let Some(session) = manager.start(mode) else {
        return;
    };

    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let started = recorder
        .lock()
        .map(|mut r| {
            r.set_output_dir(app_data_dir);
            r.start().is_ok()
        })
        .unwrap_or(false);

    if !started {
        manager.stop(); // 回滚会话,允许重试
        let err_detail = recorder
            .lock()
            .ok()
            .and_then(|mut r| r.start().err())
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".into());
        eprintln!("[TalkShow] Failed to start recording: {}", err_detail);
        if let Some(logger) = app_handle.try_state::<Logger>() {
            logger.error(
                "recording",
                "录音启动失败",
                Some(serde_json::json!({ "error": err_detail })),
            );
        }
        return;
    }

    // === Phase 1: 立即响应 ===
    if let Ok(mut s) = recording_start.lock() {
        *s = Some(Instant::now());
    }
    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(recording_icon));
    }
    show_indicator(app_handle);
    play_sound("Tink.aiff");

    // === Phase 2: 后台副作用(checkpoint = manager.is_active(session.id)) ===
    let h = app_handle.clone();
    std::thread::spawn(move || {
        let mgr = h.state::<SessionManager>();
        let logger = h.try_state::<Logger>();

        // Checkpoint 1: 取前台应用并写入会话
        if !mgr.is_active(session.id) {
            return;
        }
        if let Some(app_name) = sys::frontmost_app_name() {
            mgr.set_target_app(session.id, app_name);
        }

        // Checkpoint 2: 自动静音
        if !mgr.is_active(session.id) {
            return;
        }
        let dir = h.path().app_data_dir().unwrap_or_default();
        if config::load_config(&dir).features.recording.auto_mute {
            let _ = audio_control::save_and_mute(&dir, logger.as_deref());
        }

        // Checkpoint 3: 注册 ESC
        if !mgr.is_active(session.id) {
            return;
        }
        let _ = h.global_shortcut().register(esc_shortcut);

        // Checkpoint 4: 日志
        if mgr.is_active(session.id)
            && let Some(logger) = logger
        {
            let label = if mode == MODE_TRANSLATION {
                "录音开始 (翻译模式)"
            } else {
                "录音开始"
            };
            logger.info("recording", label, None);
        }
    });
}

/// 结束活动会话并在后台线程执行 stop pipeline。
fn end_session(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<Mutex<AudioRecorder>>,
    recording_start: &Arc<Mutex<Option<Instant>>>,
    event_name: &'static str,
) {
    let manager = app_handle.state::<SessionManager>();
    let Some(session) = manager.stop() else {
        return;
    };
    let h = app_handle.clone();
    let rec = recorder.clone();
    let start = recording_start.clone();
    std::thread::spawn(move || {
        stop_recording(&h, &rec, &start, event_name, session);
    });
    play_sound(if event_name == "recording:complete" {
        "Frog.aiff"
    } else {
        "Pop.aiff"
    });
    let icons = app_handle.state::<TrayIcons>();
    restore_default_tray(app_handle, icons.default.clone());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
            let logger = Logger::new(&app_data_dir).expect("Failed to initialize logger");
            app.manage(SessionManager::new());
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
            app.manage(TrayIcons {
                default: default_icon_owned.clone(),
                recording: recording_icon_owned.clone(),
            });

            // --- System Tray ---
            let show_i = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(default_icon_owned.clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
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
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            toggle_window(&window);
                        }
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
                let _ = app.listen("indicator:cancel", move |_event| {
                    let mgr = app_handle_cancel.state::<SessionManager>();
                    if let Some(session) = mgr.signal_cancel() {
                        // 录音中取消
                        let h = app_handle_cancel.clone();
                        let rec = recorder_cancel.clone();
                        let start = recording_start_cancel.clone();
                        std::thread::spawn(move || {
                            stop_recording(&h, &rec, &start, "recording:cancel", session);
                        });
                    }
                    destroy_indicator(&app_handle_cancel);
                    play_sound("Pop.aiff");
                    let h = app_handle_cancel.clone();
                    let esc = esc_cancel;
                    std::thread::spawn(move || {
                        let _ = h.global_shortcut().unregister(esc);
                    });
                    let icons = app_handle_cancel.state::<TrayIcons>();
                    restore_default_tray(&app_handle_cancel, icons.default.clone());
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
                        let (current_toggle_id, current_rec_id, current_translate_id) = {
                            let ids = SHORTCUT_IDS.read().unwrap();
                            (ids.toggle, ids.recording, ids.translate)
                        };

                        // --- ESC ---
                        if id == esc_id {
                            let mgr = app_handle.state::<SessionManager>();
                            match mgr.signal_cancel() {
                                Some(session) => {
                                    // 录音中取消
                                    let h = app_handle.clone();
                                    let rec = recorder_handler.clone();
                                    let start = recording_start_handler.clone();
                                    std::thread::spawn(move || {
                                        stop_recording(
                                            &h,
                                            &rec,
                                            &start,
                                            "recording:cancel",
                                            session,
                                        );
                                    });
                                    play_sound("Pop.aiff");
                                }
                                None if app_handle
                                    .get_webview_window(INDICATOR_LABEL)
                                    .is_some() =>
                                {
                                    // 处理中取消:signal_cancel 已标记 last_finished
                                    destroy_indicator(&app_handle);
                                    play_sound("Pop.aiff");
                                }
                                None => return,
                            }
                            let h = app_handle.clone();
                            let esc = esc_shortcut_handler;
                            std::thread::spawn(move || {
                                let _ = h.global_shortcut().unregister(esc);
                            });
                            let icons = app_handle.state::<TrayIcons>();
                            restore_default_tray(&app_handle, icons.default.clone());
                            return;
                        }

                        // --- 录音键(转写/翻译合并) ---
                        let mode = if current_rec_id != 0 && id == current_rec_id {
                            MODE_TRANSCRIPTION
                        } else if current_translate_id != 0 && id == current_translate_id {
                            MODE_TRANSLATION
                        } else if current_toggle_id != 0 && id == current_toggle_id {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                toggle_window(&window);
                            }
                            return;
                        } else {
                            return;
                        };

                        if debounced() {
                            return;
                        }

                        let mgr = app_handle.state::<SessionManager>();
                        if mgr.has_active() {
                            end_session(
                                &app_handle,
                                &recorder_handler,
                                &recording_start_handler,
                                "recording:complete",
                            );
                        } else {
                            let icons = app_handle.state::<TrayIcons>();
                            begin_session(
                                &app_handle,
                                &recorder_handler,
                                &recording_start_handler,
                                mode,
                                esc_shortcut_handler,
                                icons.recording.clone(),
                            );
                        }
                    })
                    .build(),
            )?;

            if let Some(sc) = toggle_shortcut
                && let Err(e) = app.global_shortcut().register(sc)
            {
                eprintln!("Failed to register toggle shortcut: {}", e);
            }
            if let Some(sc) = rec_shortcut
                && let Err(e) = app.global_shortcut().register(sc)
            {
                eprintln!("Failed to register recording shortcut: {}", e);
            }
            if let Some(sc) = translate_shortcut
                && let Err(e) = app.global_shortcut().register(sc)
            {
                eprintln!("Failed to register translate shortcut: {}", e);
            }

            // --- Tooltip update loop ---
            {
                let app_handle_tooltip = app.handle().clone();
                let recording_start_tooltip = recording_start.clone();
                std::thread::spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        let is_recording = app_handle_tooltip
                            .try_state::<SessionManager>()
                            .is_some_and(|mgr| mgr.has_active());
                        if is_recording {
                            let tooltip = recording_start_tooltip
                                .lock()
                                .ok()
                                .and_then(|start| start.as_ref().map(format_elapsed));
                            if let Some(text) = tooltip
                                && let Some(tray) = app_handle_tooltip.tray_by_id(TRAY_ID)
                            {
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

            let sensevoice_state = SenseVoiceState {
                engine: Arc::new(Mutex::new(None)),
                language: Arc::new(Mutex::new(0)),
            };
            app.manage(sensevoice_state);

            let provider_ctx = ProviderContext::new();
            app.manage(provider_ctx);

            // Pre-initialise the ONNX Runtime dylib on the main thread.
            // Doing this here avoids a macOS CoreML/dyld deadlock that occurs
            // when dlopen(libonnxruntime.dylib) is first called from a
            // spawn_blocking background thread.
            {
                let app_handle_for_ort = app.handle().clone();
                if let Err(e) = sensevoice::ensure_ort_initialized_pub(&app_handle_for_ort) {
                    eprintln!("[startup] ORT pre-init failed (non-fatal): {}", e);
                } else {
                    eprintln!("[startup] ORT pre-init succeeded");
                }
            }

            app.manage(logger);

            // --- Pre-create indicator window for instant show ---
            let indicator_url = tauri::WebviewUrl::App("/recording".into());
            let indicator_window =
                WebviewWindowBuilder::new(app.handle(), INDICATOR_LABEL, indicator_url)
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

            if let Ok(w) = indicator_window {
                let indicator_close = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = indicator_close.hide();
                    }
                });
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
