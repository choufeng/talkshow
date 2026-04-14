#[cfg(target_os = "macos")]
use crate::macos;

use tauri::{Emitter, Manager, WebviewWindowBuilder, image::Image, window::Color};

pub const TRAY_ID: &str = "main";

pub const INDICATOR_LABEL: &str = "recording-indicator";

pub fn restore_default_tray(app_handle: &tauri::AppHandle, default_icon: Image) {
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_icon(Some(default_icon));
        let _ = tray.set_tooltip(Some("TalkShow"));
    }
}

pub fn show_indicator(app_handle: &tauri::AppHandle) {
    let payload = serde_json::json!({
        "replaceMode": false,
        "selectedPreview": ""
    });

    if let Some(window) = app_handle.get_webview_window(INDICATOR_LABEL) {
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(180.0, 48.0)));
        if let Ok(Some(monitor)) = window.primary_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let screen_w = size.width as f64 / scale;
            let screen_h = size.height as f64 / scale;
            let win_w = 180.0;
            let win_h = 48.0;
            let bottom_margin = 24.0;
            let _ = window.set_position(tauri::LogicalPosition::new(
                (screen_w - win_w) / 2.0,
                screen_h - win_h - bottom_margin,
            ));
        }

        #[cfg(target_os = "macos")]
        {
            let _ = macos::floating_panel::show_without_activating(&window);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = window.show();
        }
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
                if let Err(e) = macos::floating_panel::show_without_activating(&w) {
                    eprintln!("Failed to show indicator: {}", e);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = w.show();
            }
            let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
        }
        Err(e) => {
            eprintln!("Failed to create indicator window: {}", e);
        }
    }
}

pub fn emit_indicator(app_handle: &tauri::AppHandle, event: &str) {
    let _ = app_handle.emit_to(INDICATOR_LABEL, event, ());
}

pub fn emit_indicator_paste_failed(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window(INDICATOR_LABEL) {
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(280.0, 48.0)));
        if let Ok(Some(monitor)) = window.primary_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let screen_w = size.width as f64 / scale;
            let screen_h = size.height as f64 / scale;
            let win_w = 280.0;
            let win_h = 48.0;
            let bottom_margin = 24.0;
            let _ = window.set_position(tauri::LogicalPosition::new(
                (screen_w - win_w) / 2.0,
                screen_h - win_h - bottom_margin,
            ));
        }
    }
    let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:paste-failed", ());
}

pub fn destroy_indicator(app_handle: &tauri::AppHandle) {
    if let Some(w) = app_handle.get_webview_window(INDICATOR_LABEL) {
        let _ = w.hide();
    }
}
