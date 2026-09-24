use crate::ai;
use crate::audio_control;
use crate::chat::{self, ChatOpenPayload};
use crate::clipboard;
use crate::config;
use crate::history::{self, HistoryEntry};
use crate::indicator::restore_default_tray;

/// 会话终态统一恢复托盘图标
fn restore_tray(h: &tauri::AppHandle) {
    let icons = h.state::<crate::TrayIcons>();
    restore_default_tray(h, icons.default.clone());
}
use crate::hooks;
use crate::intent::{self, Intent, IntentDecision};
use crate::logger::Logger;
use crate::providers::ProviderContext;
use crate::recording;
use crate::recording::AudioRecorder;
use crate::sensevoice::SenseVoiceEngine;
use crate::session::{Session, SessionManager};
use crate::translation;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut};

pub struct SenseVoiceState {
    pub engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    pub language: Arc<Mutex<i32>>,
}

/// pipeline 结束时清理 ESC 快捷键:仅当没有新的录音会话进行中。
/// (录音开始的后台线程会注册 ESC;有活动会话时不能注销)
fn cleanup_esc(h: &tauri::AppHandle) {
    if let Some(mgr) = h.try_state::<SessionManager>()
        && !mgr.has_active()
    {
        let _ = h
            .global_shortcut()
            .unregister(Shortcut::new(None, Code::Escape));
    }
}

pub fn show_notification(app_handle: &tauri::AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    app_handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .ok();
}

#[allow(unused_variables)]
pub fn play_sound(sound_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let sound_path = format!("/System/Library/Sounds/{}", sound_name);
        std::thread::spawn(move || {
            let _ = std::process::Command::new("afplay")
                .arg(&sound_path)
                .spawn();
        });
    }
}

pub fn stop_recording(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<std::sync::Mutex<AudioRecorder>>,
    recording_start: &Arc<std::sync::Mutex<Option<Instant>>>,
    event_name: &str,
    session: Session,
) {
    let app_data_dir_restore = app_handle.path().app_data_dir().unwrap_or_default();
    let _ = audio_control::restore(
        &app_data_dir_restore,
        app_handle.try_state::<Logger>().as_deref(),
    );

    let duration = recording_start
        .lock()
        .ok()
        .and_then(|mut start| start.take().map(|s| s.elapsed().as_secs()))
        .unwrap_or(0);

    let logger = app_handle.try_state::<Logger>();

    if let Some(ref lg) = logger {
        lg.info(
            "recording",
            &format!("录音停止 ({})", event_name),
            Some(serde_json::json!({ "duration_secs": duration })),
        );
    }

    match event_name {
        "recording:complete" => match recorder.lock() {
            Ok(mut r) => {
                // 快速重启守卫:必须持 recorder 锁时检查(与 begin_session 的 start() 抢锁互斥)。
                // 新会话已接管 recorder → 跳过 stop,避免拿新 start_time 算出 duration≈0
                // 触发 TooShort 误销毁新会话指示器。锁序 recorder→manager 单向。
                if let Some(mgr) = app_handle.try_state::<SessionManager>()
                    && mgr.has_active()
                {
                    if let Some(ref lg) = logger {
                        lg.info("recording", "新会话已接管，跳过旧会话停止", None);
                    }
                    return;
                }
                let save_start = Instant::now();
                let stop_result = r.stop();
                let save_elapsed = save_start.elapsed().as_millis();
                match stop_result {
                    Ok(result) => {
                        println!(
                            "[TalkShow] Recording saved: {} ({}s, {})",
                            result.path.display(),
                            result.duration_secs,
                            result.format,
                        );
                        if result.format == "wav" {
                            show_notification(app_handle, "FLAC 编码不可用", "已保存为 WAV 格式");
                        }
                        let _ = app_handle.emit("recording:complete", &result);

                        if let Some(ref lg) = logger {
                            lg.info(
                                "recording",
                                "录音文件已保存",
                                Some(serde_json::json!({
                                    "path": result.path.display().to_string(),
                                    "duration_secs": result.duration_secs,
                                    "format": result.format,
                                    "save_ms": save_elapsed,
                                })),
                            );
                        }

                        let config_start = Instant::now();
                        let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
                        let app_config = config::load_config(&app_data_dir);
                        let config_elapsed = config_start.elapsed().as_millis();
                        let transcription = app_config.features.transcription.clone();
                        let provider = app_config
                            .ai
                            .providers
                            .iter()
                            .find(|p| p.id == transcription.provider_id)
                            .cloned();

                        let audio_path = result.path.clone();
                        let model_name = transcription.model.clone();
                        let skills_config = app_config.features.skills.clone();
                        let glossary = app_config.features.glossary.clone();
                        let skills_providers = app_config.ai.providers.clone();
                        let h = app_handle.clone();
                        let saved_target_app = session.target_app.clone();
                        tauri::async_runtime::spawn(async move {
                            let pipeline_start = Instant::now();
                            let mgr = h.state::<SessionManager>();
                            let _ = config_elapsed;

                            if let Some(lg) = h.try_state::<Logger>() {
                                lg.info(
                                    "pipeline",
                                    "流水线启动",
                                    Some(serde_json::json!({
                                        "config_load_ms": config_elapsed,
                                    })),
                                );
                            }

                            let provider = match provider {
                                Some(p) => p,
                                None => {
                                    show_notification(&h, "AI 处理失败", "未找到配置的 AI 提供商");
                                    if let Some(lg) = h.try_state::<Logger>() {
                                        lg.error("ai", "未找到配置的 AI 提供商", None);
                                    }
                                    restore_tray(&h);
                                    cleanup_esc(&h);
                                    return;
                                }
                            };

                            if mgr.is_cancelled(session.id) {
                                if let Some(lg) = h.try_state::<Logger>() {
                                    lg.info("pipeline", "流水线已取消 (AI请求前)", None);
                                }
                                restore_tray(&h);
                                cleanup_esc(&h);
                                return;
                            }

                            if let Some(lg) = h.try_state::<Logger>() {
                                lg.info(
                                    "ai",
                                    "开始发送 AI 转写请求",
                                    Some(serde_json::json!({
                                        "provider_id": provider.id,
                                        "model": model_name,
                                        "audio_path": audio_path.display().to_string(),
                                    })),
                                );
                            }

                            let logger = h.state::<Logger>();
                            let transcribe_start = Instant::now();
                            let text_result = if provider.id == "sensevoice" {
                                let sv_state = h.state::<SenseVoiceState>();
                                let engine_arc = sv_state.engine.clone();
                                let lang_val =
                                    *sv_state.language.lock().unwrap_or_else(|e| e.into_inner());
                                let mdl_dir_sb = h
                                    .path()
                                    .app_data_dir()
                                    .unwrap_or_default()
                                    .join("models")
                                    .join("sensevoice");
                                let audio_path_sb = audio_path.clone();
                                let app_handle_for_sv = h.clone();

                                let sb_result = tokio::task::spawn_blocking(move || {
                                    let lg = app_handle_for_sv.state::<Logger>();
                                    {
                                        let guard =
                                            engine_arc.lock().unwrap_or_else(|e| e.into_inner());
                                        if guard.is_none() {
                                            drop(guard);
                                            lg.info(
                                                "sensevoice",
                                                "开始加载 SenseVoice 引擎（首次初始化，可能需要 30-120 秒）",
                                                Some(serde_json::json!({
                                                    "model_dir": mdl_dir_sb.display().to_string(),
                                                })),
                                            );
                                            let load_start = std::time::Instant::now();
                                            match SenseVoiceEngine::new(&mdl_dir_sb, &app_handle_for_sv) {
                                                Ok(eng) => {
                                                    lg.info(
                                                        "sensevoice",
                                                        "SenseVoice 引擎加载完成",
                                                        Some(serde_json::json!({
                                                            "load_ms": load_start.elapsed().as_millis(),
                                                        })),
                                                    );
                                                    let mut g = engine_arc
                                                        .lock()
                                                        .unwrap_or_else(|e| e.into_inner());
                                                    *g = Some(eng);
                                                }
                                                Err(e) => {
                                                    lg.error(
                                                        "sensevoice",
                                                        &format!("SenseVoice 引擎加载失败: {}", e),
                                                        None,
                                                    );
                                                    return Err(format!(
                                                        "SenseVoice 引擎加载失败: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                    lg.info("sensevoice", "开始转写音频", None);
                                    let mut guard =
                                        engine_arc.lock().unwrap_or_else(|e| e.into_inner());
                                    match guard.as_mut() {
                                        Some(engine) => engine
                                            .transcribe(&audio_path_sb, lang_val)
                                            .map_err(|e| e.to_string()),
                                        None => Err("SenseVoice 引擎未初始化".to_string()),
                                    }
                                })
                                .await;

                                match sb_result {
                                    Ok(Ok(text)) => Ok(text),
                                    Ok(Err(e)) => {
                                        logger.error("sensevoice", &e, None);
                                        Err(e)
                                    }
                                    Err(e) => Err(format!("SenseVoice 推理任务失败: {}", e)),
                                }
                            } else {
                                let prompt =
                                    "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
                                ai::send_audio_prompt(
                                    &logger,
                                    &audio_path,
                                    prompt,
                                    &model_name,
                                    &provider,
                                    &h.state::<ProviderContext>(),
                                )
                                .await
                                .map_err(|e| e.to_string())
                            };
                            let transcribe_elapsed = transcribe_start.elapsed().as_millis();
                            match text_result {
                                Ok(text) => {
                                    logger.info("ai", "AI 转写成功", Some(serde_json::json!({
                                    "transcribe_ms": transcribe_elapsed,
                                    "response_length": text.len(),
                                    "response_preview": text.chars().take(100).collect::<String>(),
                                })));

                                    let skills_start = Instant::now();
                                    let raw_text_for_history = text.clone();
                                    let mut decision = intent::process_with_intent(
                                        &logger,
                                        &app_config.features.intent_routing,
                                        &skills_config,
                                        &app_config.features.transcription,
                                        &skills_providers,
                                        &text,
                                        &h.state::<ProviderContext>(),
                                        &glossary,
                                    )
                                    .await
                                    .unwrap_or_else(|e| {
                                        logger.error(
                                            "intent",
                                            &format!("语义路由异常，使用原始文字: {}", e),
                                            None,
                                        );
                                        IntentDecision::passthrough(text.clone())
                                    });
                                    let mut final_text = decision.text.clone();
                                    let skills_elapsed = skills_start.elapsed().as_millis();

                                    // hook 未启用/未配置 URL → 回退普通转写粘贴,内容不丢
                                    if decision.intent == Intent::Hook {
                                        let hooks_cfg = &app_config.features.hooks;
                                        if !hooks_cfg.enabled || hooks_cfg.url.trim().is_empty() {
                                            logger.warn(
                                                "hook",
                                                "Hook 未启用或未配置 URL，回退普通转写",
                                                None,
                                            );
                                            decision.intent = Intent::Transcribe;
                                        }
                                    }

                                    let mut routed_translation = false;

                                    let original_text = if session.mode
                                        == crate::session::MODE_TRANSLATION
                                        || matches!(decision.intent, Intent::Translate { .. })
                                    {
                                        Some(final_text.clone())
                                    } else {
                                        None
                                    };

                                    if session.mode == crate::session::MODE_TRANSLATION {
                                        if transcription.polish_enabled
                                            && !transcription.polish_provider_id.is_empty()
                                            && !transcription.polish_model.is_empty()
                                        {
                                            let translate_config =
                                                app_config.features.translation.clone();
                                            match translation::translate_text(
                                                &logger,
                                                &final_text,
                                                &translate_config.target_lang,
                                                &skills_config,
                                                &transcription.polish_provider_id,
                                                &transcription.polish_model,
                                                &skills_providers,
                                                &h.state::<ProviderContext>(),
                                                &glossary,
                                            )
                                            .await
                                            {
                                                Ok(translated) => final_text = translated,
                                                Err(e) => {
                                                    logger.error("translation", &e, None);
                                                    show_notification(&h, "翻译失败", &e);
                                                    restore_tray(&h);
                                                    cleanup_esc(&h);
                                                    return;
                                                }
                                            }
                                        } else {
                                            show_notification(
                                                &h,
                                                "翻译失败",
                                                "请先启用润色并配置润色模型",
                                            );
                                            restore_tray(&h);
                                            cleanup_esc(&h);
                                            return;
                                        }
                                    } else if let Intent::Translate { target_lang } =
                                        &decision.intent
                                    {
                                        // 语义路由命中翻译:优先用合并调用的译文,缺失时独立翻译兑底
                                        routed_translation = true;
                                        match decision.result.take() {
                                            Some(translated) => final_text = translated,
                                            None => {
                                                match translation::translate_text(
                                                    &logger,
                                                    &final_text,
                                                    target_lang,
                                                    &skills_config,
                                                    &transcription.polish_provider_id,
                                                    &transcription.polish_model,
                                                    &skills_providers,
                                                    &h.state::<ProviderContext>(),
                                                    &glossary,
                                                )
                                                .await
                                                {
                                                    Ok(translated) => final_text = translated,
                                                    Err(e) => {
                                                        logger.error("translation", &e, None);
                                                        show_notification(&h, "翻译失败", &e);
                                                        restore_tray(&h);
                                                        cleanup_esc(&h);
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    } else if decision.intent == Intent::Ask {
                                        // ask:开 chat 窗体呈现首轮问答,不走剪贴板
                                        // 首答统一由前端 chat_send(思考模式)生成,
                                        // 不用合并调用的快速 result(无思考,质量弱)
                                        chat::show_chat(
                                            &h,
                                            ChatOpenPayload {
                                                question: final_text.clone(),
                                                answer: String::new(),
                                            },
                                        );

                                        {
                                            let data_dir =
                                                h.path().app_data_dir().unwrap_or_default();
                                            let entry = HistoryEntry {
                                                ts: chrono::Local::now().to_rfc3339(),
                                                mode: "chat".to_string(),
                                                raw_text: raw_text_for_history.clone(),
                                                final_text: final_text.clone(),
                                                audio_path: audio_path.display().to_string(),
                                            };
                                            if let Err(e) = history::append(&data_dir, &entry) {
                                                logger.warn(
                                                    "history",
                                                    "历史记录写入失败",
                                                    Some(serde_json::json!({ "error": e })),
                                                );
                                            }
                                        }

                                        let _ = h.emit(
                                            "pipeline:complete",
                                            serde_json::json!({
                                                "text": &final_text,
                                                "mode": session.mode,
                                                "original_text": null,
                                            }),
                                        );
                                        logger.info(
                                            "pipeline",
                                            "ask 路由完成,chat 窗体已打开",
                                            Some(serde_json::json!({
                                                "total_ms": pipeline_start.elapsed().as_millis(),
                                                "has_answer": false,
                                            })),
                                        );
                                        cleanup_esc(&h);
                                        return;
                                    } else if decision.intent == Intent::Hook {
                                        // hook:整理正文并 POST 到配置的后端,不走剪贴板
                                        let hooks_cfg = app_config.features.hooks.clone();

                                        // 1) LLM 整理(未配置整理指令 → 用润色后文本)
                                        let refined = match hooks::refine_content(
                                            &logger,
                                            &hooks_cfg,
                                            &skills_config,
                                            &app_config.features.transcription,
                                            &skills_providers,
                                            &final_text,
                                            &h.state::<ProviderContext>(),
                                        )
                                        .await
                                        {
                                            Ok(t) => t,
                                            Err(e) => {
                                                logger.error(
                                                    "hook",
                                                    "内容整理失败，使用润色后文本",
                                                    Some(serde_json::json!({ "error": e })),
                                                );
                                                final_text.clone()
                                            }
                                        };

                                        // 2) 发送
                                        let payload = hooks::HookPayload {
                                            raw: raw_text_for_history.clone(),
                                            text: refined.clone(),
                                            app: saved_target_app
                                                .clone()
                                                .unwrap_or_else(|| "Unknown".to_string()),
                                        };
                                        let send_result =
                                            hooks::send_to_hook(&logger, &hooks_cfg, &payload)
                                                .await;

                                        // 3) 通知
                                        match &send_result {
                                            Ok(()) => {
                                                let preview: String =
                                                    refined.chars().take(40).collect();
                                                show_notification(&h, "已记录", &preview);
                                            }
                                            Err(e) => {
                                                logger.error(
                                                    "hook",
                                                    "Hook 发送失败",
                                                    Some(serde_json::json!({ "error": e })),
                                                );
                                                show_notification(&h, "记录发送失败", e);
                                            }
                                        }

                                        // 4) 历史记录(发送失败也可回溯重发)
                                        {
                                            let data_dir =
                                                h.path().app_data_dir().unwrap_or_default();
                                            let entry = HistoryEntry {
                                                ts: chrono::Local::now().to_rfc3339(),
                                                mode: "hook".to_string(),
                                                raw_text: raw_text_for_history.clone(),
                                                final_text: refined.clone(),
                                                audio_path: audio_path.display().to_string(),
                                            };
                                            if let Err(e) = history::append(&data_dir, &entry) {
                                                logger.warn(
                                                    "history",
                                                    "历史记录写入失败",
                                                    Some(serde_json::json!({ "error": e })),
                                                );
                                            }
                                        }

                                        let _ = h.emit(
                                            "pipeline:complete",
                                            serde_json::json!({
                                                "text": &refined,
                                                "mode": session.mode,
                                                "original_text": null,
                                            }),
                                        );
                                        logger.info(
                                            "pipeline",
                                            "hook 路由完成",
                                            Some(serde_json::json!({
                                                "total_ms": pipeline_start.elapsed().as_millis(),
                                                "sent": send_result.is_ok(),
                                            })),
                                        );
                                        restore_tray(&h);
                                        cleanup_esc(&h);
                                        return;
                                    }

                                    if mgr.is_cancelled(session.id) {
                                        logger.info("pipeline", "流水线已取消", None);
                                        restore_tray(&h);
                                        cleanup_esc(&h);
                                        return;
                                    }

                                    if mgr.has_active() {
                                        logger.info("ai", "录音已重新开始，丢弃当前 AI 结果", None);
                                        return;
                                    }

                                    let _ = h.emit(
                                        "pipeline:complete",
                                        serde_json::json!({
                                            "text": &final_text,
                                            "mode": session.mode,
                                            "original_text": original_text,
                                        }),
                                    );

                                    // 写入历史记录（供回溯分析与术语沉淀）
                                    {
                                        let data_dir = h.path().app_data_dir().unwrap_or_default();
                                        let entry = HistoryEntry {
                                            ts: chrono::Local::now().to_rfc3339(),
                                            mode: if session.mode
                                                == crate::session::MODE_TRANSLATION
                                                || routed_translation
                                            {
                                                "translation".to_string()
                                            } else {
                                                "transcription".to_string()
                                            },
                                            raw_text: raw_text_for_history.clone(),
                                            final_text: final_text.clone(),
                                            audio_path: audio_path.display().to_string(),
                                        };
                                        if let Err(e) = history::append(&data_dir, &entry) {
                                            logger.warn(
                                                "history",
                                                "历史记录写入失败",
                                                Some(serde_json::json!({ "error": e })),
                                            );
                                        }
                                    }

                                    let clipboard_start = Instant::now();
                                    let final_text_clone = final_text.clone();
                                    let target_app_for_paste = saved_target_app.clone();
                                    let clipboard_raw = tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        tokio::task::spawn_blocking(move || {
                                            clipboard::write_and_paste(
                                                &final_text_clone,
                                                target_app_for_paste,
                                            )
                                        }),
                                    )
                                    .await;
                                    let clipboard_result: Result<
                                        Result<(), String>,
                                        tokio::time::error::Elapsed,
                                    > = match clipboard_raw {
                                        Ok(Ok(r)) => Ok(r),
                                        Ok(Err(e)) => Ok(Err(format!("剪贴板任务异常: {}", e))),
                                        Err(e) => Err(e),
                                    };

                                    match clipboard_result {
                                        Ok(Ok(())) => {
                                            let clipboard_elapsed =
                                                clipboard_start.elapsed().as_millis();
                                            let total_elapsed =
                                                pipeline_start.elapsed().as_millis();
                                            logger.info(
                                                "clipboard",
                                                "剪贴板写入并粘贴成功",
                                                Some(serde_json::json!({
                                                    "text_length": final_text.len(),
                                                })),
                                            );
                                            logger.info(
                                                "pipeline",
                                                "流水线完成",
                                                Some(serde_json::json!({
                                                    "total_ms": total_elapsed,
                                                    "transcribe_ms": transcribe_elapsed,
                                                    "skills_ms": skills_elapsed,
                                                    "clipboard_ms": clipboard_elapsed,
                                                })),
                                            );
                                            cleanup_esc(&h);
                                        }
                                        Ok(Err(e)) => {
                                            logger.error(
                                                "clipboard",
                                                "剪贴板写入/粘贴失败",
                                                Some(serde_json::json!({ "error": e })),
                                            );
                                            show_notification(&h, "粘贴失败", "请手动 Cmd+V 粘贴");
                                            restore_tray(&h);
                                        }
                                        Err(_) => {
                                            logger.error(
                                                "clipboard",
                                                "剪贴板操作超时",
                                                Some(serde_json::json!({
                                                    "text_length": final_text.len(),
                                                })),
                                            );
                                            show_notification(&h, "粘贴失败", "请手动 Cmd+V 粘贴");
                                            restore_tray(&h);
                                        }
                                    }
                                }
                                Err(e) => {
                                    logger.error(
                                        "ai",
                                        "AI 转写失败",
                                        Some(serde_json::json!({
                                            "transcribe_ms": transcribe_elapsed,
                                            "error": e.to_string(),
                                        })),
                                    );
                                    show_notification(&h, "AI 处理失败", &e);
                                    restore_tray(&h);
                                    cleanup_esc(&h);
                                }
                            }
                        });
                    }
                    Err(recording::RecordingError::TooShort) => {
                        if let Some(ref lg) = logger {
                            lg.info(
                                "recording",
                                "录音时间过短，已丢弃",
                                Some(serde_json::json!({ "duration_secs": duration })),
                            );
                        }
                        let cancelled = recording::RecordingCancelled {
                            duration_secs: duration,
                        };
                        let _ = app_handle.emit("recording:cancel", cancelled);
                        restore_tray(app_handle);
                        cleanup_esc(app_handle);
                    }
                    Err(e) => {
                        if let Some(ref lg) = logger {
                            lg.error(
                                "recording",
                                "录音停止失败",
                                Some(serde_json::json!({ "error": e.to_string() })),
                            );
                        }
                        let _ = app_handle.emit("recording:error", e.to_string());
                        restore_tray(app_handle);
                        cleanup_esc(app_handle);
                    }
                }
            }
            Err(_) => {
                let _ = app_handle.emit("recording:error", "Recording lock poisoned");
                restore_tray(app_handle);
                cleanup_esc(app_handle);
            }
        },
        "recording:cancel" => {
            if let Ok(mut r) = recorder.lock() {
                // 同 recording:complete 的快速重启守卫,持锁检查新会话是否已接管
                if let Some(mgr) = app_handle.try_state::<SessionManager>()
                    && mgr.has_active()
                {
                    if let Some(ref lg) = logger {
                        lg.info("recording", "新会话已接管，跳过旧会话取消", None);
                    }
                    return;
                }
                let _duration = r.cancel();
            }
            println!("[TalkShow] Recording cancelled ({}s)", duration);
            if let Some(ref lg) = logger {
                lg.info(
                    "recording",
                    "录音已取消",
                    Some(serde_json::json!({ "duration_secs": duration })),
                );
            }
            let cancelled = recording::RecordingCancelled {
                duration_secs: duration,
            };
            let _ = app_handle.emit("recording:cancel", cancelled);
            restore_tray(app_handle);
        }
        _ => {}
    }
}
