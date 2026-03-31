# 关闭 AI 模型默认 Thinking 模式（按功能控制） Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 AI 请求链路中添加 per-call 的 thinking 模式控制参数，各调用点按需决定是否开启思考模式。

**Architecture:** 在 `send_text_prompt` 及其下游函数中新增 `ThinkingMode` 参数。OpenAI-compatible 路径通过 `additional_params` 传递 `enable_thinking`；Vertex 路径暂不支持（rig-vertexai 0.3.2 不处理 `additional_params`），默认不传。

**Tech Stack:** Rust, rig-core 0.33, rig-vertexai 0.3.2

---

## 关键发现

| 路径 | `additional_params` 支持 | thinking 控制方式 |
|------|------------------------|-----------------|
| **OpenAI-compatible** | 支持（`#[serde(flatten)]`） | `{"enable_thinking": false}` |
| **Vertex AI** | **不支持**（rig-vertexai 忽略） | 需后续升级 rig-vertexai |

## 调用点分析

| 调用点 | 文件:行号 | 当前场景 | 需要 thinking? |
|--------|----------|---------|---------------|
| Skills 文本润色 | `skills.rs:157` | 转写后文本处理 | **否** (已验证 45s→0.5s) |
| 连通性测试 | `lib.rs:546,560` | 测试模型是否可用 | **否** |
| 未来功能 | TBD | 按需决定 | 按需 |

---

### Task 1: 定义 ThinkingMode 枚举

**Files:**
- Modify: `src-tauri/src/ai.rs:1-25`

- [ ] **Step 1: 在 ai.rs 顶部添加 ThinkingMode 枚举**

在 `AiError` 枚举之后（约第 22 行），添加：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode {
    Default,
    Enabled,
    Disabled,
}
```

`Default` 表示不干预模型默认行为；`Enabled` / `Disabled` 显式控制。

- [ ] **Step 2: 编译验证**

Run: `cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ai.rs
git commit -m "feat(ai): add ThinkingMode enum for per-call thinking control"
```

---

### Task 2: OpenAI-compatible 路径支持 ThinkingMode

**Files:**
- Modify: `src-tauri/src/ai.rs` — `send_text_prompt` 函数签名、`send_text_via_openai_compatible` 函数

- [ ] **Step 1: 修改 `send_text_prompt` 函数签名，添加 `thinking` 参数**

将 `src-tauri/src/ai.rs` 第 244-275 行的函数签名从：

```rust
pub async fn send_text_prompt(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    vertex_cache: &VertexClientCache,
) -> Result<String, AiError> {
```

改为：

```rust
pub async fn send_text_prompt(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    vertex_cache: &VertexClientCache,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
```

同时在函数体内部，将两处调用 `send_text_via_openai_compatible` 和 `send_text_via_vertex` 时透传 `thinking` 参数（此时函数签名还未更新，先传参，后续 Task 会更新下游函数）：

vertex 分支（约第 254 行）改为：
```rust
"vertex" => {
    let client = get_or_create_vertex_client(logger, vertex_cache)?;
    send_text_via_vertex(logger, &client, text_prompt, model_name, thinking).await
}
```

openai-compatible 分支（约第 257 行）改为：
```rust
"openai-compatible" => {
    let api_key = provider
        .api_key
        .as_deref()
        .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
    send_text_via_openai_compatible(
        logger,
        text_prompt,
        model_name,
        api_key,
        &provider.endpoint,
        thinking,
    )
    .await
}
```

- [ ] **Step 2: 修改 `send_text_via_openai_compatible` 函数签名和请求构建**

将 `src-tauri/src/ai.rs` 第 321 行的函数签名从：

```rust
async fn send_text_via_openai_compatible(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
```

改为：

```rust
async fn send_text_via_openai_compatible(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
```

在请求构建处（约第 359 行），将：

```rust
let request = model.completion_request(message).build();
```

改为：

```rust
let request = match thinking {
    ThinkingMode::Default => model.completion_request(message).build(),
    ThinkingMode::Enabled => model
        .completion_request(message)
        .additional_params(serde_json::json!({"enable_thinking": true}))
        .build(),
    ThinkingMode::Disabled => model
        .completion_request(message)
        .additional_params(serde_json::json!({"enable_thinking": false}))
        .build(),
};
```

- [ ] **Step 3: 编译验证（预期失败，因为下游函数签名不匹配）**

Run: `cargo check`
Expected: 编译错误 — `send_text_via_vertex` 签名不匹配（这是正常的，Task 3 会修复）

---

### Task 3: Vertex 路径支持 ThinkingMode

**Files:**
- Modify: `src-tauri/src/ai.rs` — `send_text_via_vertex` 函数

- [ ] **Step 1: 修改 `send_text_via_vertex` 函数签名**

将 `src-tauri/src/ai.rs` 第 277 行的函数签名从：

```rust
async fn send_text_via_vertex(
    logger: &Logger,
    client: &rig_vertexai::Client,
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
```

改为：

```rust
async fn send_text_via_vertex(
    logger: &Logger,
    client: &rig_vertexai::Client,
    text_prompt: &str,
    model_name: &str,
    _thinking: ThinkingMode,
) -> Result<String, AiError> {
```

注意：参数名前加 `_` 前缀表示暂时未使用。rig-vertexai 0.3.2 的 `additional_params` 不被处理，所以暂时忽略。当 rig-vertexai 升级后可移除下划线并实现。

- [ ] **Step 2: 编译验证**

Run: `cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/ai.rs
git commit -m "feat(ai): add ThinkingMode parameter to text prompt functions"
```

---

### Task 4: 更新所有调用点

**Files:**
- Modify: `src-tauri/src/skills.rs:157`
- Modify: `src-tauri/src/lib.rs:546,560`

- [ ] **Step 1: Skills 调用点 — 关闭 thinking**

将 `src-tauri/src/skills.rs` 第 157 行从：

```rust
crate::ai::send_text_prompt(logger, &full_prompt, &transcription_config.polish_model, provider, vertex_cache),
```

改为：

```rust
crate::ai::send_text_prompt(logger, &full_prompt, &transcription_config.polish_model, provider, vertex_cache, crate::ai::ThinkingMode::Disabled),
```

- [ ] **Step 2: 连通性测试调用点 — 关闭 thinking**

将 `src-tauri/src/lib.rs` 第 546 行从：

```rust
ai::send_text_prompt(&logger, "Hi", &model_name, &provider, &vertex_cache).await
```

改为：

```rust
ai::send_text_prompt(&logger, "Hi", &model_name, &provider, &vertex_cache, ai::ThinkingMode::Disabled).await
```

将 `src-tauri/src/lib.rs` 第 560 行从：

```rust
ai::send_text_prompt(&logger, "Hi", &model_name, &provider, &vertex_cache).await
```

改为：

```rust
ai::send_text_prompt(&logger, "Hi", &model_name, &provider, &vertex_cache, ai::ThinkingMode::Disabled).await
```

- [ ] **Step 3: 编译验证**

Run: `cargo check`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/skills.rs src-tauri/src/lib.rs
git commit -m "feat(ai): disable thinking mode for skills polish and connectivity test"
```

---

### Task 5: 验证

- [ ] **Step 1: 完整编译**

Run: `cargo check`
Expected: 零错误零警告

- [ ] **Step 2: 运行应用测试**

手动测试流程：
1. 启动应用
2. 测试模型连通性（应快速返回，不再因思考模式延迟）
3. 录音并触发 Skills 文本润色（应从 35s 超时降到 <2s）

---

## Self-Review

1. **Spec 覆盖**: per-call thinking 控制 ✓，Skills 关闭 ✓，连通性测试关闭 ✓，Vertex 路径标记未使用 ✓
2. **Placeholder 扫描**: 无 TBD/TODO
3. **类型一致性**: `ThinkingMode` 在所有函数签名中一致 ✓
