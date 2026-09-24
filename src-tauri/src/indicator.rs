//! 托盘图标管理(录音指示窗已移除,状态反馈 = 托盘图标 + 音效 + 系统通知)

use tauri::image::Image;

pub const TRAY_ID: &str = "main";

pub fn restore_default_tray(app_handle: &tauri::AppHandle, default_icon: Image) {
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_icon(Some(default_icon));
        let _ = tray.set_tooltip(Some("TalkShow"));
    }
}
