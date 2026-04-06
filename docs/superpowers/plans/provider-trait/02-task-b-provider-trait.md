# 子任务 B: Provider Trait 基础架构

> **依赖**: A (ProviderConfig 简化) | **阶段**: Phase 1 | **复杂度**: 中

## 目标

创建 `providers/` 模块，定义 `Provider` trait、错误类型、工厂函数、`ProviderContext`。创建三个 stub provider 文件并注册到 `lib.rs`。

## 涉及文件

| 文件 | 操作 |
|------|------|
| `src-tauri/src/providers/mod.rs` | 新建 |
| `src-tauri/src/providers/dashscope.rs` | 新建 (stub) |
| `src-tauri/src/providers/vertex.rs` | 新建 (stub) |
| `src-tauri/src/providers/sensevoice.rs` | 新建 (stub) |
| `src-tauri/src/lib.rs` | 修改 (添加 `mod providers;`) |

## 前置条件

- 子任务 A 已完成 (`ProviderConfig` 已简化，无 `provider_type`/`endpoint`)

## 步骤

- [ ] **Step 1: 创建 `providers/mod.rs`**

```rust
pub mod dashscope;
pub mod sensevoice;
pub mod vertex;

use crate::config::{ModelConfig, ProviderConfig};
use crate::logger::Logger;
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode {
    Default,
    Enabled,
    Disabled,
}

#[derive(Debug)]
pub enum ProviderError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
    UnsupportedOperation(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            ProviderError::MissingApiKey(id) => {
                write!(f, "Missing API key for provider: {}", id)
            }
            ProviderError::FileReadError(e) => write!(f, "Failed to read audio file: {}", e),
            ProviderError::RequestError(e) => write!(f, "AI request failed: {}", e),
            ProviderError::UnsupportedOperation(op) => {
                write!(f, "Unsupported operation: {}", op)
            }
        }
    }
}

impl std::error::Error for ProviderError {}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError>;

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        thinking: ThinkingMode,
    ) -> Result<String, ProviderError>;

    fn needs_api_key(&self) -> bool;
    fn default_models() -> Vec<ModelConfig>;
}

pub struct ProviderContext {
    pub sensevoice_engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
}

impl ProviderContext {
    pub fn new() -> Self {
        ProviderContext {
            sensevoice_engine: Arc::new(Mutex::new(None)),
        }
    }
}

pub fn create_provider(
    config: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<Box<dyn Provider>, ProviderError> {
    match config.id.as_str() {
        "dashscope" => Ok(Box::new(dashscope::DashScopeProvider::new(
            config.api_key.clone(),
        ))),
        "vertex" => Ok(Box::new(vertex::VertexAIProvider::new())),
        "sensevoice" => Ok(Box::new(sensevoice::SenseVoiceProvider::new(
            ctx.sensevoice_engine.clone(),
        ))),
        _ => Err(ProviderError::ProviderNotFound(config.id.clone())),
    }
}

pub const PROVIDERS_REQUIRING_KEY: &[&str] = &["dashscope"];

pub fn provider_needs_api_key(id: &str) -> bool {
    PROVIDERS_REQUIRING_KEY.contains(&id)
}

pub const BUILTIN_PROVIDER_IDS: &[&str] = &["dashscope", "vertex", "sensevoice"];

pub fn is_builtin_provider(id: &str) -> bool {
    BUILTIN_PROVIDER_IDS.contains(&id)
}
```

- [ ] **Step 2: 创建 stub `providers/dashscope.rs`**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;

pub struct DashScopeProvider {
    pub api_key: Option<String>,
}

impl DashScopeProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for DashScopeProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        todo!("Implement in sub-task C")
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        todo!("Implement in sub-task C")
    }

    fn needs_api_key(&self) -> bool {
        true
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "qwen2-audio-instruct".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        }]
    }
}
```

- [ ] **Step 3: 创建 stub `providers/vertex.rs`**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;

pub struct VertexAIProvider {}

impl VertexAIProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Provider for VertexAIProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        todo!("Implement in sub-task D")
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        todo!("Implement in sub-task D")
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "gemini-2.0-flash".to_string(),
            capabilities: vec!["transcription".to_string(), "chat".to_string()],
            verified: None,
        }]
    }
}
```

- [ ] **Step 4: 创建 stub `providers/sensevoice.rs`**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct SenseVoiceProvider {
    pub engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
}

impl SenseVoiceProvider {
    pub fn new(engine: Arc<Mutex<Option<SenseVoiceEngine>>>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Provider for SenseVoiceProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        _audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        todo!("Implement in sub-task E")
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
```

- [ ] **Step 5: 在 `lib.rs` 中注册模块**

在 `src-tauri/src/lib.rs` 的 `mod` 声明区域添加：
```rust
mod providers;
```

- [ ] **Step 6: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```

预期：providers 模块编译通过。可能会有其他文件引用旧类型的编译错误（这些在子任务 F 中处理），但 providers 模块本身不应有错误。

## 提交

```bash
git add src-tauri/src/providers/ src-tauri/src/lib.rs
git commit -m "feat: add Provider trait and module structure with stubs"
```
