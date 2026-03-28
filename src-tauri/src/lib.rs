mod config;
mod recording;

use recording::AudioRecorder;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const TRAY_ID: &str = "main";

static RECORDING: AtomicBool = AtomicBool::new(false);

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
    let mut key_code = None;

    for part in &parts {
        match *part {
            "Control" => modifiers |= Modifiers::CONTROL,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Alt" => modifiers |= Modifiers::ALT,
            "Command" | "Super" => modifiers |= Modifiers::SUPER,
            "Quote" => key_code = Some(Code::Quote),
            "Space" => key_code = Some(Code::Space),
            "KeyN" => key_code = Some(Code::KeyN),
            "KeyR" => key_code = Some(Code::KeyR),
            "KeyS" => key_code = Some(Code::KeyS),
            "KeyQ" => key_code = Some(Code::KeyQ),
            "Backslash" => key_code = Some(Code::Backslash),
            "Escape" => key_code = Some(Code::Escape),
            _ => {}
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
                    if result.format == "wav" {
                        show_notification(&app_handle, "FLAC 编码不可用", "已保存为 WAV 格式");
                    }
                    let _ = app_handle.emit("recording:complete", result);
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

    match shortcut_type.as_str() {
        "toggle" => app_config.shortcut = shortcut,
        "recording" => app_config.recording_shortcut = shortcut,
        _ => return Err("Invalid shortcut type".to_string()),
    }

    config::save_config(&app_data_dir, &app_config)?;

    // Re-register shortcuts
    if let Some(sc) = parse_shortcut(&app_config.shortcut) {
        let _ = app_handle.global_shortcut().unregister(sc.clone());
        let _ = app_handle.global_shortcut().register(sc);
    }
    if let Some(sc) = parse_shortcut(&app_config.recording_shortcut) {
        let _ = app_handle.global_shortcut().unregister(sc.clone());
        let _ = app_handle.global_shortcut().register(sc);
    }

    Ok(())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![get_config, update_shortcut])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().unwrap_or_default();
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

            let rec_id = rec_shortcut.as_ref().map(|s| s.id());
            let esc_id = esc_shortcut.id();

            let app_handle = app.handle().clone();
            let recording_start_handler = recording_start.clone();
            let recorder_handler = recorder.clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |_app, shortcut, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        let id = shortcut.id();

                        if id == esc_id {
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:cancel",
                                );
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            }
                            return;
                        }

                        if rec_id == Some(id) {
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                );
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
            if let Err(e) = app.global_shortcut().register(esc_shortcut) {
                eprintln!("Failed to register escape shortcut: {}", e);
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
