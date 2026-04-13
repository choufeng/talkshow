# OpenAI Provider 设计文档

## 概述

为 TalkShow 新增 OpenAI Provider，支持语音转写和文本补全功能，并支持自定义 base URL 以兼容第三方 OpenAI 兼容服务（Azure OpenAI、Groq 等）。

## 数据模型变更

### ProviderConfig 新增 endpoint 字段

**Rust (`config.rs`)**：

```rust
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

**TypeScript (`config.ts`)**：

```typescript
export interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  endpoint?: string;
  models: ModelConfig[];
}
```

### OpenAI 内置 Provider 默认配置

```rust
ProviderConfig {
    id: "openai",
    name: "OpenAI",
    api_key: Some(""),
    endpoint: Some("https://api.openai.com/v1"),
    models: [ModelConfig {
        name: "gpt-4o-transcribe",
        capabilities: ["transcription", "chat"],
        verified: None,
    }],
}
```

### 数据迁移

`ProviderConfig` 使用 `#[serde(default)]`，新增的 `endpoint` 字段在反序列化时自动为 `None`，不需要显式迁移逻辑。

## Rust 后端实现

### 新建 `providers/openai.rs`

`OpenAIProvider` 结构体：

```rust
pub struct OpenAIProvider {
    api_key: Option<String>,
    endpoint: String,
}
```

实现 `Provider` trait：

- **`transcribe()`**: POST `{endpoint}/audio/transcriptions`
  - multipart form: `file` (音频 bytes) + `model` + `prompt`
  - 响应解析: `{ "text": "..." }`

- **`complete_text()`**: POST `{endpoint}/chat/completions`
  - JSON body: `{ "model": "...", "messages": [{"role": "user", "content": "..."}] }`
  - 支持 `ThinkingMode`（通过额外字段控制）
  - 响应解析: `{ "choices": [{"message": {"content": "..."}}] }`

- **`needs_api_key()`**: 返回 `true`

- **`default_models()`**: 还认 `[gpt-4o-transcribe]`

### 注册到 Provider 工厂

在 `providers/mod.rs` 的 `create_provider()` 中添加 `"openai"` 分支，从 `ProviderConfig.endpoint` 读取 base URL（默认 `https://api.openai.com/v1`），传入 `OpenAIProvider::new()`。

## 前端变更

### `config.ts`

- `BUILTIN_PROVIDERS` 新增 OpenAI 条目
- `PROVIDERS_REQUIRING_KEY` 新增 `'openai'`
- 新增 `PROVIDERS_WITH_ENDPOINT = ['openai']` 常量

### `+page.svelte` (models 页面)

- 为 `PROVIDERS_WITH_ENDPOINT` 中的 provider 显示 endpoint 编辑框（使用 `EditableField` 组件）
- 保存时将 endpoint 写入 `ProviderConfig.endpoint`

OpenAI Provider 卡片包含：API Key 输入框 + Endpoint 输入框 + Models 列表，无需其他特殊渲染。

## 涉及文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/providers/openai.rs` | 新建：OpenAI Provider 实现 |
| `src-tauri/src/providers/mod.rs` | 添加 `openai` 模块声明和 `create_provider` 分支 |
| `src-tauri/src/config.rs` | `ProviderConfig` 新增 `endpoint` 字段 + `builtin_providers()` 新增 OpenAI |
| `src/lib/stores/config.ts` | `ProviderConfig` 类型新增 `endpoint` + `BUILTIN_PROVIDERS` 新增 OpenAI |
| `src/routes/models/+page.svelte` | 添加 endpoint 编辑框渲染逻辑 |
