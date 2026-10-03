//! 发送前人工确认:拦截 pipeline 产物(润色/译文/hook 整理/ask 问题),
//! 弹出校对浮窗编辑文本 + 选择去向,确认后才执行动作。
//!
//! 时序:窗口首次创建时 'review:open' 事件可能早于前端挂载监听 →
//! 前端 mount 后主动 invoke review_get 拉取,事件仅作增量更新。

use crate::chat::{self, ChatOpenPayload};
use crate::clipboard;
use crate::config;
use crate::history::{self, HistoryEntry};
use crate::hooks::{self, HookPayload};
use crate::logger::Logger;
use crate::translation;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, serde::Serialize)]
pub struct HookPreview {
    pub url: String,
    /// 模板渲染后的 body(按当前 text;编辑后确认时重渲染)
    pub body: String,
    /// LLM 提取的变量池(确认时保留原值参与重渲染)
    pub vars: HashMap<String, String>,
}

#[derive(Clone, serde::Serialize)]
pub struct ReviewRequest {
    pub id: u64,
    /// 原始转写(只读参考)
    pub raw_text: String,
    /// 主文本(润色后/译文/hook 整理后),可编辑
    pub text: String,
    /// 翻译产物时的原文
    pub original_text: Option<String>,
    /// 译文语言(展示用)
    pub target_lang: Option<String>,
    /// pipeline 判定动作 "paste"|"ask"|"hook"
    pub action: String,
    /// 粘贴目标应用
    pub target_app: Option<String>,
    /// hook 动作时的发送预览
    pub hook: Option<HookPreview>,
    /// Hook 去向可用(hooks 已启用且配置 URL)
    pub can_hook: bool,
    /// 音频文件路径(history 回溯用)
    pub audio_path: String,
}

#[derive(serde::Deserialize)]
pub struct ReviewConfirm {
    pub id: u64,
    /// 用户最终选择 "paste"|"translate"|"ask"|"hook"
    pub action: String,
    pub text: String,
    /// action=translate 时的目标语言
    pub target_lang: Option<String>,
}

#[derive(Default)]
pub struct ReviewState {
    pub current: Mutex<Option<ReviewRequest>>,
}

/// 存请求 → 指示条切"待确认" → 建窗(或复用) → 广播
pub fn open_review(h: &AppHandle, req: ReviewRequest) {
    let state = h.state::<ReviewState>();
    *state.current.lock().unwrap() = Some(req.clone());
    crate::indicator::show_review(h);

    match h.get_webview_window("review") {
        Some(w) => {
            let _ = w.show();
            let _ = w.set_focus();
        }
        None => {
            let _ = WebviewWindowBuilder::new(h, "review", WebviewUrl::App("index.html".into()))
                .title("校对与发送")
                .inner_size(560.0, 580.0)
                .min_inner_size(480.0, 420.0)
                .decorations(false)
                .always_on_top(true)
                .resizable(true)
                .skip_taskbar(true)
                .shadow(true)
                .focused(true)
                .build();
        }
    }
    let _ = h.emit("review:open", &req);
}

fn write_history(h: &AppHandle, mode: &str, req: &ReviewRequest, final_text: &str) {
    let logger = h.state::<Logger>();
    let data_dir = h.path().app_data_dir().unwrap_or_default();
    let entry = HistoryEntry {
        ts: chrono::Local::now().to_rfc3339(),
        mode: mode.to_string(),
        raw_text: req.raw_text.clone(),
        final_text: final_text.to_string(),
        audio_path: req.audio_path.clone(),
    };
    if let Err(e) = history::append(&data_dir, &entry) {
        logger.warn(
            "history",
            "历史记录写入失败",
            Some(serde_json::json!({ "error": e })),
        );
    }
}

async fn paste(_h: &AppHandle, text: &str, target_app: Option<String>) -> Result<(), String> {
    let text = text.to_string();
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(move || clipboard::write_and_paste(&text, target_app)),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => Err(format!("剪贴板任务异常: {}", e)),
        Err(_) => Err("剪贴板操作超时".to_string()),
    }
}

/// 执行确认:按用户选择分发动作。成功返回 Ok 并收尾(浮窗+指示条)。
pub async fn execute_confirm(h: &AppHandle, confirm: ReviewConfirm) -> Result<(), String> {
    let state = h.state::<ReviewState>();
    // 先比对 id 再取出,防止吞掉覆盖进来的新请求
    {
        let g = state.current.lock().unwrap();
        match g.as_ref() {
            Some(r) if r.id == confirm.id => {}
            Some(_) => return Err("内容已更新，请查看新内容".to_string()),
            None => return Err("没有待确认的内容".to_string()),
        }
    }
    let req = state.current.lock().unwrap().take().expect("checked above");

    let logger = h.state::<Logger>();
    let data_dir = h.path().app_data_dir().unwrap_or_default();
    let app_config = config::load_config(&data_dir);
    let result = dispatch(h, &app_config, &logger, &req, &confirm).await;

    match result {
        Ok(done_msg) => {
            logger.info(
                "review",
                "人工确认执行完成",
                Some(serde_json::json!({
                    "action": confirm.action,
                    "from": req.action,
                    "text_edited": confirm.text != req.text,
                })),
            );
            // 期间有新请求进来(指示条在待确认)则不动指示条与浮窗
            let superseded = state.current.lock().unwrap().is_some();
            if !superseded {
                if let Some(w) = h.get_webview_window("review") {
                    let _ = w.hide();
                }
                match done_msg {
                    Some(msg) => crate::indicator::show_result(h, &msg),
                    None => crate::indicator::hide_indicator(h),
                }
            }
            Ok(())
        }
        Err(e) => {
            // 失败放回请求,用户可改选去向或取消
            *state.current.lock().unwrap() = Some(req);
            Err(e)
        }
    }
}

/// 取消:清状态 + 收指示条 + 藏浮窗
pub fn dismiss(h: &AppHandle) {
    let state = h.state::<ReviewState>();
    *state.current.lock().unwrap() = None;
    crate::indicator::hide_indicator(h);
    if let Some(w) = h.get_webview_window("review") {
        let _ = w.hide();
    }
}

#[allow(clippy::too_many_lines)]
async fn dispatch(
    h: &AppHandle,
    app_config: &config::AppConfig,
    logger: &Logger,
    req: &ReviewRequest,
    confirm: &ReviewConfirm,
) -> Result<Option<String>, String> {
    let text = confirm.text.trim().to_string();
    if text.is_empty() {
        return Err("内容为空".to_string());
    }

    match confirm.action.as_str() {
        "paste" => {
            paste(h, &text, req.target_app.clone()).await?;
            write_history(
                h,
                if req.original_text.is_some() {
                    "translation"
                } else {
                    "transcription"
                },
                req,
                &text,
            );
            let _ = h.emit(
                "pipeline:complete",
                serde_json::json!({
                    "text": &text,
                    "mode": if req.original_text.is_some() { crate::session::MODE_TRANSLATION } else { crate::session::MODE_TRANSCRIPTION },
                    "original_text": req.original_text,
                }),
            );
            Ok(Some("已粘贴".to_string()))
        }
        "translate" => {
            let lang = confirm
                .target_lang
                .clone()
                .unwrap_or_else(|| app_config.features.translation.target_lang.clone());
            let skills_config = app_config.features.skills.clone();
            let transcription = app_config.features.transcription.clone();
            let providers = app_config.ai.providers.clone();
            let glossary = app_config.features.glossary.clone();
            let translated = translation::translate_text(
                logger,
                &text,
                &lang,
                &skills_config,
                &transcription.polish_provider_id,
                &transcription.polish_model,
                &providers,
                &h.state::<crate::providers::ProviderContext>(),
                &glossary,
            )
            .await
            .map_err(|e| format!("翻译失败: {}", e))?;
            paste(h, &translated, req.target_app.clone()).await?;
            write_history(h, "translation", req, &translated);
            let _ = h.emit(
                "pipeline:complete",
                serde_json::json!({
                    "text": &translated,
                    "mode": crate::session::MODE_TRANSLATION,
                    "original_text": &text,
                }),
            );
            Ok(Some(format!("已翻译并粘贴 ({})", lang)))
        }
        "ask" => {
            chat::show_chat(
                h,
                ChatOpenPayload {
                    question: text.clone(),
                    answer: String::new(),
                },
            );
            write_history(h, "chat", req, &text);
            let _ = h.emit(
                "pipeline:complete",
                serde_json::json!({
                    "text": &text,
                    "mode": crate::session::MODE_TRANSCRIPTION,
                    "original_text": null,
                }),
            );
            Ok(Some("已转对话".to_string()))
        }
        "hook" => {
            let hooks_cfg = app_config.features.hooks.clone();
            if !hooks_cfg.enabled || hooks_cfg.url.trim().is_empty() {
                return Err("Hook 未启用或未配置 URL".to_string());
            }
            let app_name = req
                .target_app
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());

            // 原 pipeline 动作即 hook → 已有 refine 产物与变量池,用编辑后 text 重渲染;
            // 其他动作切换来 → 现场补跑整理
            let (refined_text, vars) = match &req.hook {
                Some(_) => (
                    text.clone(),
                    req.hook
                        .as_ref()
                        .map(|p| p.vars.clone())
                        .unwrap_or_default(),
                ),
                None => {
                    let r = hooks::refine_content(
                        logger,
                        &hooks_cfg,
                        &app_config.features.skills,
                        &app_config.features.transcription,
                        &app_config.ai.providers,
                        &text,
                        &h.state::<crate::providers::ProviderContext>(),
                    )
                    .await
                    .map_err(|e| format!("内容整理失败: {}", e))?;
                    (r.text, r.vars)
                }
            };

            let payload = HookPayload {
                raw: req.raw_text.clone(),
                text: refined_text.clone(),
                app: app_name,
                vars,
            };
            hooks::send_to_hook(logger, &hooks_cfg, &payload).await?;
            write_history(h, "hook", req, &refined_text);
            let _ = h.emit(
                "pipeline:complete",
                serde_json::json!({
                    "text": &refined_text,
                    "mode": crate::session::MODE_TRANSCRIPTION,
                    "original_text": null,
                }),
            );
            Ok(Some("已发送至 Hook".to_string()))
        }
        other => Err(format!("未知去向: {}", other)),
    }
}
