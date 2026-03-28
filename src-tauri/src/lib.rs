mod config;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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
    recording_start: &Arc<std::sync::Mutex<Option<Instant>>>,
    event_name: &str,
) {
    RECORDING.store(false, Ordering::Relaxed);
    let duration = recording_start
        .lock()
        .ok()
        .and_then(|mut start| start.take().map(|s| s.elapsed().as_secs()))
        .unwrap_or(0);
    let _ = app_handle.emit(event_name, duration);
}

fn restore_default_tray(app_handle: &tauri::AppHandle, default_icon: Image) {
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_icon(Some(default_icon));
        let _ = tray.set_tooltip(Some("TalkShow"));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            let _tray = TrayIconBuilder::with_id("main")
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

            // --- Global Shortcuts (single plugin instance) ---
            let toggle_shortcut = parse_shortcut(&shortcut_str);
            let rec_shortcut = parse_shortcut(&recording_shortcut_str);
            let esc_shortcut = Shortcut::new(None, Code::Escape);

            let rec_id = rec_shortcut.as_ref().map(|s| s.id());
            let esc_id = esc_shortcut.id();

            let app_handle = app.handle().clone();
            let recording_start_handler = recording_start.clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |_app, shortcut, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        let id = shortcut.id();
                        eprintln!(
                            "[DEBUG] Shortcut pressed: id={:?}, rec_id={:?}, esc_id={:?}",
                            id, rec_id, esc_id
                        );

                        if id == esc_id {
                            eprintln!("[DEBUG] Escape shortcut pressed");
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            eprintln!("[DEBUG] Current recording state: {}", is_recording);
                            if is_recording {
                                eprintln!("[DEBUG] Cancelling recording...");
                                stop_recording(
                                    &app_handle,
                                    &recording_start_handler,
                                    "recording:cancel",
                                );
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                                eprintln!("[DEBUG] Recording cancelled, tray restored");
                            }
                            return;
                        }

                        if rec_id == Some(id) {
                            eprintln!("[DEBUG] Recording shortcut pressed (Ctrl+\\)");
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            eprintln!("[DEBUG] Current recording state: {}", is_recording);
                            if is_recording {
                                eprintln!("[DEBUG] Stopping recording...");
                                stop_recording(
                                    &app_handle,
                                    &recording_start_handler,
                                    "recording:complete",
                                );
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                                eprintln!("[DEBUG] Recording stopped, tray restored");
                            } else {
                                eprintln!("[DEBUG] Starting recording...");
                                RECORDING.store(true, Ordering::Relaxed);
                                *recording_start_handler.lock().unwrap() = Some(Instant::now());
                                if let Some(tray) = app_handle.tray_by_id("main") {
                                    eprintln!("[DEBUG] Setting recording icon...");
                                    let _ = tray.set_icon(Some(recording_icon_owned.clone()));
                                    eprintln!("[DEBUG] Recording icon set");
                                } else {
                                    eprintln!("[DEBUG] Tray not found!");
                                }
                            }
                            return;
                        }

                        eprintln!("[DEBUG] Toggle window shortcut pressed");
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
                            if let Some(tray) = app_handle_tooltip.tray_by_id("main") {
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
