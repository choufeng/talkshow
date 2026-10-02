//! 托盘图标管理 + 录音指示条入口(macOS 走原生 NSPanel,其他平台 no-op)

use tauri::AppHandle;
use tauri::image::Image;

pub const TRAY_ID: &str = "main";

pub fn restore_default_tray(app_handle: &tauri::AppHandle, default_icon: Image) {
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_icon(Some(default_icon));
        let _ = tray.set_tooltip(Some("TalkShow"));
    }
}

pub fn show_indicator(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    crate::macos::indicator_panel::show_recording(app_handle);
    #[cfg(not(target_os = "macos"))]
    let _ = app_handle;
}

pub fn show_processing(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    crate::macos::indicator_panel::show_processing(app_handle);
    #[cfg(not(target_os = "macos"))]
    let _ = app_handle;
}

pub fn show_paste_failed(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    crate::macos::indicator_panel::show_paste_failed(app_handle);
    #[cfg(not(target_os = "macos"))]
    let _ = app_handle;
}

/// 路由结果提示:浮窗末段显示去向(语义路由/hook 分发结果)后自动隐藏
pub fn show_result(app_handle: &AppHandle, text: &str) {
    #[cfg(target_os = "macos")]
    crate::macos::indicator_panel::show_result(app_handle, text);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app_handle;
        let _ = text;
    }
}

pub fn hide_indicator(app_handle: &AppHandle) {
    #[cfg(target_os = "macos")]
    crate::macos::indicator_panel::hide(app_handle);
    #[cfg(not(target_os = "macos"))]
    let _ = app_handle;
}
