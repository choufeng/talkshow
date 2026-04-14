use crate::ai;
use crate::audio_control;
use crate::clipboard;
use crate::config;
use crate::indicator::{destroy_indicator, emit_indicator, emit_indicator_paste_failed};
use crate::logger::Logger;
use crate::providers::ProviderContext;
use crate::recording;
use crate::recording::AudioRecorder;
use crate::sensevoice::SenseVoiceEngine;
use crate::shortcuts::{CANCELLED, RECORDING, RECORDING_MODE_NONE, RECORDING_MODE_TRANSLATION};
use crate::skills;
use crate::translation;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut};

pub struct SenseVoiceState {
    pub engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    pub language: Arc<Mutex<i32>>,
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
    recording_mode: u8,
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
                        emit_indicator(app_handle, "indicator:processing");

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
                        let skills_providers = app_config.ai.providers.clone();
                        let h = app_handle.clone();
                        let saved_target_app = clipboard::get_target_app();
                        CANCELLED.store(false, Ordering::SeqCst);
                        tauri::async_runtime::spawn(async move {
                            let pipeline_start = Instant::now();
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
                                    destroy_indicator(&h);
                                    if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                        let _ = h
                                            .global_shortcut()
                                            .unregister(Shortcut::new(None, Code::Escape));
                                    }
                                    return;
                                }
                            };

                            if CANCELLED.load(Ordering::SeqCst) {
                                if let Some(lg) = h.try_state::<Logger>() {
                                    lg.info("pipeline", "流水线已取消 (AI请求前)", None);
                                }
                                destroy_indicator(&h);
                                if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                    let _ = h
                                        .global_shortcut()
                                        .unregister(Shortcut::new(None, Code::Escape));
                                }
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

                                let sb_result = tokio::task::spawn_blocking(move || {
                                    {
                                        let guard =
                                            engine_arc.lock().unwrap_or_else(|e| e.into_inner());
                                        if guard.is_none() {
                                            drop(guard);
                                            match SenseVoiceEngine::new(&mdl_dir_sb) {
                                                Ok(eng) => {
                                                    let mut g = engine_arc
                                                        .lock()
                                                        .unwrap_or_else(|e| e.into_inner());
                                                    *g = Some(eng);
                                                }
                                                Err(e) => {
                                                    return Err(format!(
                                                        "SenseVoice 引擎加载失败: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                    }
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
                                    let mut final_text = skills::process_with_skills(
                                        &logger,
                                        &skills_config,
                                        &app_config.features.transcription,
                                        &skills_providers,
                                        &text,
                                        &h.state::<ProviderContext>(),
                                    )
                                    .await
                                    .unwrap_or_else(|e| {
                                        logger.error(
                                            "skills",
                                            &format!("Skills 处理异常，使用原始文字: {}", e),
                                            None,
                                        );
                                        text
                                    });
                                    let skills_elapsed = skills_start.elapsed().as_millis();

                                    let original_text =
                                        if recording_mode == RECORDING_MODE_TRANSLATION {
                                            Some(final_text.clone())
                                        } else {
                                            None
                                        };

                                    if recording_mode == RECORDING_MODE_TRANSLATION {
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
                                            )
                                            .await
                                            {
                                                Ok(translated) => final_text = translated,
                                                Err(e) => {
                                                    logger.error("translation", &e, None);
                                                    show_notification(&h, "翻译失败", &e);
                                                    destroy_indicator(&h);
                                                    if RECORDING.load(Ordering::SeqCst)
                                                        == RECORDING_MODE_NONE
                                                    {
                                                        let _ = h.global_shortcut().unregister(
                                                            Shortcut::new(None, Code::Escape),
                                                        );
                                                    }
                                                    return;
                                                }
                                            }
                                        } else {
                                            show_notification(
                                                &h,
                                                "翻译失败",
                                                "请先启用润色并配置润色模型",
                                            );
                                            destroy_indicator(&h);
                                            if RECORDING.load(Ordering::SeqCst)
                                                == RECORDING_MODE_NONE
                                            {
                                                let _ = h
                                                    .global_shortcut()
                                                    .unregister(Shortcut::new(None, Code::Escape));
                                            }
                                            return;
                                        }
                                    }

                                    if CANCELLED.load(Ordering::SeqCst) {
                                        logger.info("pipeline", "流水线已取消", None);
                                        destroy_indicator(&h);
                                        if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                            let _ = h
                                                .global_shortcut()
                                                .unregister(Shortcut::new(None, Code::Escape));
                                        }
                                        return;
                                    }

                                    if RECORDING.load(Ordering::SeqCst) != RECORDING_MODE_NONE {
                                        logger.info("ai", "录音已重新开始，丢弃当前 AI 结果", None);
                                        return;
                                    }

                                    let _ = h.emit(
                                        "pipeline:complete",
                                        serde_json::json!({
                                            "text": &final_text,
                                            "mode": recording_mode,
                                            "original_text": original_text,
                                        }),
                                    );

                                    let clipboard_start = Instant::now();
                                    let final_text_clone = final_text.clone();
                                    let target_app_for_paste = saved_target_app.clone();
                                    let clipboard_raw = tokio::time::timeout(
                                        std::time::Duration::from_secs(2),
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
                                            emit_indicator(&h, "indicator:done");
                                            if RECORDING.load(Ordering::SeqCst)
                                                == RECORDING_MODE_NONE
                                            {
                                                let _ = h
                                                    .global_shortcut()
                                                    .unregister(Shortcut::new(None, Code::Escape));
                                            }
                                        }
                                        Ok(Err(e)) => {
                                            logger.error(
                                                "clipboard",
                                                "剪贴板写入/粘贴失败",
                                                Some(serde_json::json!({ "error": e })),
                                            );
                                            emit_indicator_paste_failed(&h);
                                        }
                                        Err(_) => {
                                            logger.error(
                                                "clipboard",
                                                "剪贴板操作超时 (5s)",
                                                Some(serde_json::json!({
                                                    "text_length": final_text.len(),
                                                })),
                                            );
                                            emit_indicator_paste_failed(&h);
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
                                    destroy_indicator(&h);
                                    if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
                                        let _ = h
                                            .global_shortcut()
                                            .unregister(Shortcut::new(None, Code::Escape));
                                    }
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
                        destroy_indicator(app_handle);
                        let _ = app_handle
                            .global_shortcut()
                            .unregister(Shortcut::new(None, Code::Escape));
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
                        destroy_indicator(app_handle);
                        let _ = app_handle
                            .global_shortcut()
                            .unregister(Shortcut::new(None, Code::Escape));
                    }
                }
            }
            Err(_) => {
                let _ = app_handle.emit("recording:error", "Recording lock poisoned");
                destroy_indicator(app_handle);
                let _ = app_handle
                    .global_shortcut()
                    .unregister(Shortcut::new(None, Code::Escape));
            }
        },
        "recording:cancel" => {
            if let Ok(mut r) = recorder.lock() {
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
            destroy_indicator(app_handle);
        }
        _ => {}
    }
}
