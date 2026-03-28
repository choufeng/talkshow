# RIG AI 集成实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 rig-core 替换 async-openai，实现录音完成后发送多模态请求到 AI，结果通过剪贴板粘贴到用户焦点位置。

**Architecture:** 新增 `ai.rs` 模块封装 RIG 交互，`clipboard.rs` 模块处理剪贴板写入和模拟粘贴。录音完成后在 `lib.rs` 的 `stop_recording` 中通过 `tokio::spawn` 异步调用 AI 处理管线。

**Tech Stack:** rig-core 0.33, rig-vertexai 0.3, arboard 3 (剪贴板), osascript (模拟粘贴)

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Modify | `src-tauri/Cargo.toml` | 替换依赖 |
| Create | `src-tauri/src/ai.rs` | RIG AI 交互：创建 client、发送多模态请求 |
| Create | `src-tauri/src/clipboard.rs` | 剪贴板写入 + 模拟 Cmd+V 粘贴 |
| Modify | `src-tauri/src/lib.rs:1-2` | 注册新模块 |
| Modify | `src-tauri/src/lib.rs:82-109` | 录音完成后调用 AI 管线 |

---

### Task 1: 替换 Cargo.toml 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml:28`

- [ ] **Step 1: 替换 async-openai 为 rig-core、rig-vertexai、arboard**

将 `src-tauri/Cargo.toml` 第 28 行：
```toml
async-openai = "0.20" # OpenAI API client, compatible with Vertex AI Gemini models via OpenAI-compatible endpoint
```

替换为：
```toml
rig-core = "0.33"
rig-vertexai = "0.3"
arboard = "3"
```

- [ ] **Step 2: 验证依赖能编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译成功（可能有 unused import 警告，无 error）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: replace async-openai with rig-core, rig-vertexai, arboard"
```

---

### Task 2: 创建 clipboard.rs 模块

**Files:**
- Create: `src-tauri/src/clipboard.rs`

- [ ] **Step 1: 创建 clipboard.rs**

创建文件 `src-tauri/src/clipboard.rs`：

```rust
pub fn write_and_paste(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard.set_text(text).map_err(|e| format!("Failed to write to clipboard: {}", e))?;
    simulate_paste();
    Ok(())
}

#[cfg(target_os = "macos")]
fn simulate_paste() {
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS（可能有 unused 警告）

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/clipboard.rs
git commit -m "feat: add clipboard write and paste simulation module"
```

---

### Task 3: 创建 ai.rs 模块

**Files:**
- Create: `src-tauri/src/ai.rs`

这是核心模块，负责根据配置创建 RIG client 并发送多模态请求。

- [ ] **Step 1: 创建 ai.rs**

创建文件 `src-tauri/src/ai.rs`：

```rust
use base64::Engine;
use rig::completion::{CompletionModel, Prompt};
use rig::providers::openai;
use std::path::Path;

use crate::config::ProviderConfig;

#[derive(Debug)]
pub enum AiError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            AiError::MissingApiKey(id) => write!(f, "Missing API key for provider: {}", id),
            AiError::FileReadError(e) => write!(f, "Failed to read audio file: {}", e),
            AiError::RequestError(e) => write!(f, "AI request failed: {}", e),
        }
    }
}

pub async fn send_audio_prompt(
    audio_path: &Path,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    let audio_data = std::fs::read(audio_path)
        .map_err(|e| AiError::FileReadError(e.to_string()))?;
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&audio_data);

    let extension = audio_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    let media_type = match extension {
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "mp3" => "audio/mp3",
        "ogg" => "audio/ogg",
        _ => "audio/wav",
    };

    match provider.provider_type.as_str() {
        "vertex" => send_via_vertex(&audio_b64, media_type, text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_via_openai_compatible(
                &audio_b64,
                media_type,
                text_prompt,
                model_name,
                api_key,
                &provider.endpoint,
            )
            .await
        }
        _ => Err(AiError::ProviderNotFound(format!(
            "Unknown provider type: {}",
            provider.provider_type
        ))),
    }
}

async fn send_via_vertex(
    audio_b64: &str,
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
    use rig::completion::message::{UserContent, AudioMediaType};
    use rig::OneOrMany;

    let client = rig_vertexai::Client::builder()
        .build()
        .map_err(|e| AiError::RequestError(format!("Vertex AI client init failed: {}", e)))?;

    let model = client.completion_model(model_name);

    let audio_content = UserContent::audio(audio_b64.to_string(), Some(AudioMediaType::from_mime(media_type)));

    let prompt_content = OneOrMany::many(vec![
        audio_content,
        UserContent::text(text_prompt.to_string()),
    ])
    .map_err(|e| AiError::RequestError(format!("Failed to build prompt: {}", e)))?;

    let message = rig::message::Message::User {
        content: prompt_content,
    };

    let response = model
        .prompt_message(message)
        .await
        .map_err(|e| AiError::RequestError(e.to_string()))?;

    let text = match response {
        rig::message::Message::Assistant { content, .. } => {
            content.into_iter().filter_map(|c| match c {
                rig::completion::message::AssistantContent::Text(t) => Some(t.text),
                _ => None,
            }).collect::<Vec<_>>().join("")
        }
        _ => return Err(AiError::RequestError("Unexpected response type".into())),
    };

    Ok(text)
}

async fn send_via_openai_compatible(
    audio_b64: &str,
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
    use rig::completion::message::{UserContent, AudioMediaType};
    use rig::OneOrMany;

    let client = openai::CompletionsClient::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| AiError::RequestError(format!("Client init failed: {}", e)))?;

    let model = client.completion_model(model_name);

    let audio_content = UserContent::audio(audio_b64.to_string(), Some(AudioMediaType::from_mime(media_type)));

    let prompt_content = OneOrMany::many(vec![
        audio_content,
        UserContent::text(text_prompt.to_string()),
    ])
    .map_err(|e| AiError::RequestError(format!("Failed to build prompt: {}", e)))?;

    let message = rig::message::Message::User {
        content: prompt_content,
    };

    let response = model
        .prompt_message(message)
        .await
        .map_err(|e| AiError::RequestError(e.to_string()))?;

    let text = match response {
        rig::message::Message::Assistant { content, .. } => {
            content.into_iter().filter_map(|c| match c {
                rig::completion::message::AssistantContent::Text(t) => Some(t.text),
                _ => None,
            }).collect::<Vec<_>>().join("")
        }
        _ => return Err(AiError::RequestError("Unexpected response type".into())),
    };

    Ok(text)
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ai.rs
git commit -m "feat: add AI module with RIG multimodal request support"
```

---

### Task 4: 集成到 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs:1-2` (添加模块声明)
- Modify: `src-tauri/src/lib.rs:82-109` (录音完成后调用 AI)

- [ ] **Step 1: 在 lib.rs 顶部注册新模块**

将 `src-tauri/src/lib.rs` 第 1-2 行：
```rust
mod config;
mod recording;
```

替换为：
```rust
mod ai;
mod clipboard;
mod config;
mod recording;
```

- [ ] **Step 2: 在 stop_recording 的 recording:complete 分支中添加 AI 处理逻辑**

将 `src-tauri/src/lib.rs` 中 `stop_recording` 函数的 `"recording:complete"` 分支（第 82-109 行）：

```rust
        "recording:complete" => match recorder.lock() {
            Ok(mut r) => match r.stop() {
                Ok(result) => {
                    println!(
                        "[TalkShow] Recording saved: {} ({}s, {})",
                        result.path.display(),
                        result.duration_secs,
                        result.format,
                    );
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
```

替换为：

```rust
        "recording:complete" => match recorder.lock() {
            Ok(mut r) => match r.stop() {
                Ok(result) => {
                    println!(
                        "[TalkShow] Recording saved: {} ({}s, {})",
                        result.path.display(),
                        result.duration_secs,
                        result.format,
                    );
                    if result.format == "wav" {
                        show_notification(&app_handle, "FLAC 编码不可用", "已保存为 WAV 格式");
                    }
                    let _ = app_handle.emit("recording:complete", &result);

                    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
                    let app_config = config::load_config(&app_data_dir);
                    let transcription = &app_config.features.transcription;
                    let provider = app_config
                        .ai
                        .providers
                        .iter()
                        .find(|p| p.id == transcription.provider_id)
                        .cloned();

                    let audio_path = result.path.clone();
                    let model_name = transcription.model.clone();
                    let h = app_handle.clone();
                    tokio::spawn(async move {
                        let provider = match provider {
                            Some(p) => p,
                            None => {
                                show_notification(&h, "AI 处理失败", "未找到配置的 AI 提供商");
                                return;
                            }
                        };
                        let prompt = "请将这段音频转录为文字，只输出转录结果，不要添加任何解释。";
                        match ai::send_audio_prompt(&audio_path, prompt, &model_name, &provider).await {
                            Ok(text) => {
                                if let Err(e) = clipboard::write_and_paste(&text) {
                                    show_notification(&h, "剪贴板写入失败", &e);
                                }
                            }
                            Err(e) => {
                                show_notification(&h, "AI 处理失败", &e.to_string());
                            }
                        }
                    });
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
```

- [ ] **Step 3: 验证编译**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: integrate AI processing into recording completion flow"
```

---

### Task 5: 端到端编译验证与清理

- [ ] **Step 1: 完整编译检查**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 无 error，修复任何编译问题

- [ ] **Step 2: 如有编译错误，逐一修复**

常见的可能问题：
- `base64` import：rig-core 可能已经 re-export，如果没有需要在 Cargo.toml 添加 `base64` 依赖
- `AudioMediaType::from_mime` 方法签名可能与预期不同，需根据实际 API 调整
- `prompt_message` 方法可能需要 `Completion` trait 而非 `Prompt` trait

- [ ] **Step 3: 最终 Commit**

如果有修复：
```bash
git add -A
git commit -m "fix: resolve compilation issues for RIG integration"
```
