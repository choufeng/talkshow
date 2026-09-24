//! Ask 对话:语义路由命中问询意图时,唤起主窗体并导航到对话栏目。
//! (v1 曾用独立悬浮窗,因体验不佳改为主窗体内栏目)

use tauri::{Emitter, Manager};

#[derive(Clone, serde::Serialize)]
pub struct ChatOpenPayload {
    pub question: String,
    /// 首轮回答;合并调用未产出时为空,由前端调 chat_send 兜底
    pub answer: String,
}

/// 唤起主窗体并下发初始问答(layout 监听后导航到对话栏目)
pub fn show_chat(app_handle: &tauri::AppHandle, payload: ChatOpenPayload) {
    if let Some(w) = app_handle.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    let _ = app_handle.emit_to("main", "chat:open", &payload);
}

/// chat 栏目语音专用录音器(独立实例,不与全局快捷键管线共享)
// ponytail: 与全局录音同时开麦依赖系统共享模式,极低概率场景不守卫
pub struct ChatRecorderState {
    pub recorder: std::sync::Arc<std::sync::Mutex<crate::recording::AudioRecorder>>,
}
