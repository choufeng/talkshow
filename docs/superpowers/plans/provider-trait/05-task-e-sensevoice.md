# 子任务 E: SenseVoice Provider 实现

> **依赖**: B (Provider Trait 基础架构) | **阶段**: Phase 2 (可与 C, D 并行) | **复杂度**: 中

## 目标

实现 SenseVoice 本地推理 Provider，封装 `SenseVoiceEngine`，将音频 bytes 写入临时文件后调用本地 ONNX 推理。

## 涉及文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/providers/sensevoice.rs` | 修改 (替换 stub) |

## 背景知识

SenseVoice 是本地 ONNX 推理引擎，不走 HTTP API：
- 需要文件路径（非 bytes）作为输入
- 通过 `Arc<Mutex<Option<SenseVoiceEngine>>>` 共享引擎实例
- 引擎的 `transcribe(path, language)` 方法是同步调用
- `language` 参数为 `i32` 类型（0=auto, 3=中文, 等）
- **不支持文本补全** — `complete_text()` 返回 `UnsupportedOperation` 错误

## 步骤

- [ ] **Step 1: 替换 `providers/sensevoice.rs` 完整实现**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct SenseVoiceProvider {
    pub engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    pub language: Arc<Mutex<i32>>,
}

impl SenseVoiceProvider {
    pub fn new(engine: Arc<Mutex<Option<SenseVoiceEngine>>>) -> Self {
        Self {
            engine,
            language: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_language(self, language: Arc<Mutex<i32>>) -> Self {
        Self { language, ..self }
    }
}

#[async_trait]
impl Provider for SenseVoiceProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        let tmp_dir = std::env::temp_dir().join("talkshow_provider");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join(format!("audio_{}.wav", uuid()));

        std::fs::write(&tmp_path, audio_bytes)
            .map_err(|e| ProviderError::FileReadError(e.to_string()))?;

        let lang = self
            .language
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .to_owned();

        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(engine) => engine
                .transcribe(&tmp_path, lang)
                .map_err(|e| ProviderError::RequestError(e.to_string())),
            None => Err(ProviderError::RequestError(
                "SenseVoice engine not initialized".to_string(),
            )),
        }
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        Err(ProviderError::UnsupportedOperation(
            "SenseVoice does not support text completion".to_string(),
        ))
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "SenseVoice-Small".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        }]
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
```

- [ ] **Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep -E "(sensevoice|error)"
```

预期：sensevoice.rs 无编译错误。

## 注意事项

- `with_language()` 方法允许外部注入 language 参数（从 Tauri state 获取）
- 临时文件写入 `std::env::temp_dir()/talkshow_provider/` 目录
- `SenseVoiceEngine::transcribe()` 是同步方法，在 async 上下文中直接调用（足够快，不需要 spawn_blocking）
- `complete_text()` 始终返回错误 — SenseVoice 只做语音转写

## 提交

```bash
git add src-tauri/src/providers/sensevoice.rs
git commit -m "feat: implement SenseVoice provider wrapping local ONNX engine"
```
