# 子任务 F: 后端集成 — 移除 rig 依赖并统一接口

> **依赖**: A (Config 简化) + B (Trait 基础) + C (DashScope) + D (Vertex) + E (SenseVoice) | **阶段**: Phase 3 | **复杂度**: 高

## 目标

1. 从 `Cargo.toml` 移除 `rig-core` 和 `rig-vertexai` 依赖
2. 重写 `ai.rs` 为 Provider trait 的薄包装
3. 更新 `lib.rs` 中所有调用点 (`VertexClientState` → `ProviderContext`)
4. 更新 `skills.rs`, `translation.rs`, `llm_client.rs`, `real_llm_client.rs`

## 涉及文件

| 文件 | 操作 | 变更摘要 |
|------|------|----------|
| `src-tauri/Cargo.toml` | 修改 | 移除 rig-core, rig-vertexai |
| `src-tauri/src/ai.rs` | 重写 | Provider trait 薄包装 |
| `src-tauri/src/lib.rs` | 修改 | VertexClientState → ProviderContext |
| `src-tauri/src/skills.rs` | 修改 | VertexClientCache → ProviderContext |
| `src-tauri/src/translation.rs` | 修改 | VertexClientCache → ProviderContext |
| `src-tauri/src/llm_client.rs` | 修改 | 移除 endpoint 参数 |
| `src-tauri/src/real_llm_client.rs` | 重写 | 使用 Provider trait |

## 步骤

- [ ] **Step 1: 移除 rig 依赖 (Cargo.toml)**

删除这两行：
```toml
rig-core = "0.33"
rig-vertexai = "0.3"
```

- [ ] **Step 2: 重写 `ai.rs`**

将整个文件替换为 Provider trait 的薄包装：

```rust
use crate::config::ProviderConfig;
use crate::logger::Logger;
use crate::providers::{self, ProviderContext, ProviderError, ThinkingMode};
use std::path::Path;

pub use providers::{ProviderError as AiError, ThinkingMode};

pub async fn send_audio_prompt(
    logger: &Logger,
    audio_path: &Path,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<String, AiError> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| AiError::FileReadError(e.to_string()))?;

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

    send_audio_prompt_from_bytes(logger, &audio_data, media_type, text_prompt, model_name, provider, ctx).await
}

pub async fn send_audio_prompt_from_bytes(
    logger: &Logger,
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<String, AiError> {
    let p = providers::create_provider(provider, ctx)?;
    p.transcribe(logger, audio_bytes, media_type, text_prompt, model_name)
        .await
}

pub async fn send_text_prompt(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
    let p = providers::create_provider(provider, ctx)?;
    p.complete_text(logger, text_prompt, model_name, thinking)
        .await
}
```

**关键变更：**
- 移除 `VertexClientCache` 类型别名
- 移除 `get_or_create_vertex_client()` 函数
- 移除所有 rig 相关 import (`rig_core::`, `rig_vertexai::`)
- 所有函数签名中 `VertexClientCache` → `ProviderContext`
- 不再有 `match provider_type` 分发逻辑 — 由 `create_provider()` 工厂处理

- [ ] **Step 3: 更新 `llm_client.rs`**

移除 `send_text` 和 `send_audio` 中的 `endpoint` 参数：

```rust
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)]
pub trait LlmClient: Send + Sync {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String>;

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String>;
}
```

- [ ] **Step 4: 重写 `real_llm_client.rs`**

```rust
use crate::ai::{send_audio_prompt_from_bytes, send_text_prompt, ThinkingMode};
use crate::config::ProviderConfig;
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use crate::providers::ProviderContext;

#[allow(dead_code)]
pub struct RealLlmClient<'a> {
    logger: &'a Logger,
    ctx: &'a ProviderContext,
}

impl<'a> RealLlmClient<'a> {
    #[allow(dead_code)]
    pub fn new(logger: &'a Logger, ctx: &'a ProviderContext) -> Self {
        Self { logger, ctx }
    }
}

#[async_trait::async_trait]
impl LlmClient for RealLlmClient<'_> {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            name: provider_id.to_string(),
            api_key: None,
            models: vec![],
        };
        send_text_prompt(
            self.logger,
            prompt,
            model_name,
            &provider,
            self.ctx,
            ThinkingMode::Disabled,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            name: provider_id.to_string(),
            api_key: None,
            models: vec![],
        };
        send_audio_prompt_from_bytes(
            self.logger,
            audio_bytes,
            media_type,
            text_prompt,
            model_name,
            &provider,
            self.ctx,
        )
        .await
        .map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 5: 更新 `skills.rs`**

1. 将 `process_with_skills` 函数签名中的 `vertex_cache: &VertexClientCache` → `ctx: &crate::providers::ProviderContext`
2. 移除 `type VertexClientCache = ...` 类型别名
3. 内部调用 `ai::send_text_prompt` 时传递 `ctx` 而非 `vertex_cache`
4. 更新 `process_with_skills_client` 中 `client.send_text()` 调用，移除 endpoint 参数
5. 更新所有测试中的 `MockLlmClient` 期望：
   - `.expect_send_text().returning(|_, _, _, _| ...)` → `.expect_send_text().returning(|_, _, _| ...)`
   - 移除 endpoint 相关参数

- [ ] **Step 6: 更新 `translation.rs`**

1. 将 `translate_text` 签名中的 `vertex_cache: &VertexClientCache` → `ctx: &crate::providers::ProviderContext`
2. 移除 VertexClientCache 类型别名
3. 内部调用更新
4. 更新测试中的 MockLlmClient 期望

- [ ] **Step 7: 更新 `lib.rs`**

**7a. 移除 `VertexClientState` struct**

删除：
```rust
struct VertexClientState {
    client: crate::ai::VertexClientCache,
}
```

**7b. 替换所有 `VertexClientState` 使用为 `ProviderContext`**

搜索替换：
- `h.state::<VertexClientState>().client` → `h.state::<ProviderContext>()`（但注意参数类型变化，现在传 `&ProviderContext`）
- `VertexClientState { client: Arc::new(Mutex::new(None)) }` → `ProviderContext::new()`

**具体位置（需逐一确认行号）：**

1. **录音转写路径** (~line 248-289):
   - `provider.provider_type == "sensevoice"` → `provider.id == "sensevoice"`
   - sensevoice 分支：使用 `ProviderContext` 中的 sensevoice_engine
   - 其他 provider：调用 `ai::send_audio_prompt(&logger, &audio_path, prompt, &model_name, &provider, &provider_ctx)`

2. **Skills 处理** (~line 312-318):
   - `skills::process_with_skills(..., &h.state::<VertexClientState>().client)` → `..., &h.state::<ProviderContext>())`

3. **翻译** (~line 346-355):
   - `translation::translate_text(..., &h.state::<VertexClientState>().client)` → `..., &h.state::<ProviderContext>())`

4. **test_model_connectivity** (~line 824-866):
   - `provider.provider_type == "sensevoice"` → `provider.id == "sensevoice"`
   - 传递 `ProviderContext` 替代 `vertex_cache`

5. **初始化** (~line 1556-1559):
   - `VertexClientState { client: Arc::new(Mutex::new(None)) }` → `ProviderContext::new()`

- [ ] **Step 8: 验证编译和测试**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
cd src-tauri && cargo test 2>&1 | tail -30
```

预期：编译通过，所有测试通过。

## 提交

建议分为两个提交（可选）：

```bash
# 提交 1: 移除 rig 并重写 ai 层
git add src-tauri/Cargo.toml src-tauri/src/ai.rs src-tauri/src/llm_client.rs src-tauri/src/real_llm_client.rs
git commit -m "refactor: remove rig dependencies, rewrite ai.rs as Provider trait wrapper"

# 提交 2: 更新所有调用方
git add src-tauri/src/lib.rs src-tauri/src/skills.rs src-tauri/src/translation.rs
git commit -m "refactor: update lib.rs, skills, translation to use ProviderContext"
```
