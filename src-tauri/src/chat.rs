//! Ask 对话窗体:语义路由命中问询意图时弹出,展示首轮问答并支持持续对话。
//!
//! 窗口策略:首次创建后常驻(隐藏不销毁,保留对话状态);
//! 每次新 ask 重置对话;用户 Esc/关闭按钮 → 隐藏。

use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewWindowBuilder, window::Color};

pub const CHAT_LABEL: &str = "chat";

/// 待拉取的初始问答:show_chat 时存入,前端就绪后 chat_take_latest 拉取。
/// 双通道(事件 + 拉取)避免窗口加载时序导致事件丢失。
pub struct ChatState {
    pub pending: Mutex<Option<ChatOpenPayload>>,
}

/// chat 窗体语音专用录音器(独立实例,不与全局快捷键管线共享)
// ponytail: 与全局录音同时开麦依赖系统共享模式,极低概率场景不守卫
pub struct ChatRecorderState {
    pub recorder: std::sync::Arc<Mutex<crate::recording::AudioRecorder>>,
}

#[derive(Clone, serde::Serialize)]
pub struct ChatOpenPayload {
    pub question: String,
    /// 首轮回答;合并调用未产出时为空,由前端调 chat_send 兜底
    pub answer: String,
}

/// 打开(或复用)chat 窗体并下发初始问答
pub fn show_chat(app_handle: &tauri::AppHandle, payload: ChatOpenPayload) {
    // 存 pending:前端就绪后主动拉取,不依赖事件时序
    if let Some(state) = app_handle.try_state::<ChatState>()
        && let Ok(mut p) = state.pending.lock()
    {
        *p = Some(payload.clone());
    }

    if let Some(window) = app_handle.get_webview_window(CHAT_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app_handle.emit_to(CHAT_LABEL, "chat:open", &payload);
        return;
    }

    // 参照 indicator:取主显示器尺寸,窗口居中偏上
    let main_window = app_handle.get_webview_window("main");
    let monitor = main_window
        .as_ref()
        .and_then(|w| w.primary_monitor().ok().flatten());
    let (win_w, win_h) = (440.0, 640.0);
    let (x, y) = match &monitor {
        Some(m) => {
            let size = m.size();
            let scale = m.scale_factor();
            let screen_w = size.width as f64 / scale;
            let screen_h = size.height as f64 / scale;
            ((screen_w - win_w) / 2.0, (screen_h - win_h) / 2.0 - 40.0)
        }
        None => (620.0, 300.0),
    };

    let builder = WebviewWindowBuilder::new(
        app_handle,
        CHAT_LABEL,
        tauri::WebviewUrl::App("/chat".into()),
    )
    .inner_size(win_w, win_h)
    .position(x, y)
    .min_inner_size(360.0, 480.0)
    .transparent(true)
    .decorations(false)
    .shadow(true)
    .background_color(Color(0, 0, 0, 0))
    .resizable(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false);

    match builder.build() {
        Ok(w) => {
            let _ = w.show();
            let _ = w.set_focus();
            // 事件给已就绪页面;未就绪时由前端 mount 后 chat_take_latest 兜底
            let _ = app_handle.emit_to(CHAT_LABEL, "chat:open", &payload);
        }
        Err(e) => {
            eprintln!("Failed to create chat window: {}", e);
        }
    }
}

/// 隐藏 chat 窗体(保留状态)
pub fn hide_chat(app_handle: &tauri::AppHandle) {
    if let Some(w) = app_handle.get_webview_window(CHAT_LABEL) {
        let _ = w.hide();
    }
}
