# Provider Trait Architecture Design

Date: 2026-04-06

## 背景

当前 TalkShow 的 AI Provider 架构存在以下问题：

1. **依赖 rig-core/rig-vertexai 抽象层**：通义走 rig 的 OpenAI 兼容客户端，Vertex AI 走 rig-vertexai。这些中间层限制了我们对各家 API 特性的直接控制，且更新依赖上游。
2. **Add Provider 功能复杂度高**：UI 需要支持用户手动填写 Provider 类型、Endpoint 等信息，增加维护成本和出错概率。
3. **Endpoint 可编辑**：用户可修改 Provider 的 API URL，但官方 Endpoint 极少变动，无需暴露给用户。

## 目标

1. 移除 rig-core 和 rig-vertexai 依赖，每家服务商独立封装为 Rust trait 实现。
2. 移除 "Add Provider" 功能 — 所能支持的 Provider 写死在代码中，新增服务商通过版本升级引入。
3. Provider 的 Endpoint 写死在各自的实现内部，UI 不再显示。
4. 用户仍可编辑每个 Provider 的模型列表。
5. API Key 输入保留（需要密钥的服务商仍然显示）。
6. 已有用户配置自动迁移到新格式。

## 设计

### 1. Provider Trait 架构（后端）

#### 1.1 Trait 定义

文件：`src-tauri/src/providers/mod.rs`

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn transcribe(&self, logger: &Logger, audio: &[u8], media_type: &str, prompt: &str, model: &str) -> Result<String, AiError>;
    async fn complete_text(&self, logger: &Logger, prompt: &str, model: &str, thinking: ThinkingMode) -> Result<String, AiError>;
    fn needs_api_key(&self) -> bool;
    fn default_models() -> Vec<ModelConfig>;
}
```

#### 1.2 Provider 注册工厂

```rust
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn Provider>, AiError> {
    match config.id.as_str() {
        "dashscope" => Ok(Box::new(DashScopeProvider::new(config.api_key.clone()))),
        "vertex" => Ok(Box::new(VertexAIProvider::new())),
        "sensevoice" => Ok(Box::new(SenseVoiceProvider::new())),
        _ => Err(AiError::ProviderNotFound(config.id.clone())),
    }
}
```

新增服务商只需：添加新 struct + impl Provider，在 `create_provider` 中注册一行。

#### 1.3 三个实现

| Provider | 文件 | HTTP 方式 | 需要 API Key |
|----------|------|-----------|-------------|
| DashScopeProvider | `providers/dashscope.rs` | reqwest 直调 DashScope OpenAI 兼容 API | 是 |
| VertexAIProvider | `providers/vertex.rs` | reqwest 直调 Vertex AI REST API（ADC 认证） | 否 |
| SenseVoiceProvider | `providers/sensevoice.rs` | 本地 ONNX 推理（从 ai.rs 迁移） | 否 |

#### 1.4 DashScope 实现

直调 DashScope OpenAI 兼容 API：

- 转写：`POST https://dashscope.aliyuncs.com/compatible-mode/v1/audio/transcriptions`
- 文本：`POST https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions`
- 认证：`Authorization: Bearer {api_key}`
- 使用 reqwest 构建 multipart/form-data（转写）和 JSON body（文本）

#### 1.5 Vertex AI 实现

使用 Google Cloud ADC 获取 access token，直调 Vertex AI REST API：

- 端点：`POST https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:generateContent`
- 认证：先通过 ADC 获取 token → `Authorization: Bearer {token}`
- ADC token 获取逻辑：复用当前 `get_vertex_env_info` 的环境检测，通过 gcloud CLI 或环境变量获取 token

#### 1.6 SenseVoice 实现

从 `ai.rs` 迁移本地 ONNX 推理逻辑到 `SenseVoiceProvider` struct。逻辑不变，只是包装到 trait 实现中。

#### 1.7 依赖变更

Cargo.toml：
- 移除：`rig-core = "0.33"`、`rig-vertexai = "0.3"`
- 保留：`reqwest`（已有）、`async-trait`（已有）、`base64`（已有）

### 2. 配置模型变更

#### 2.1 ProviderConfig 简化

当前：
```rust
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: String,
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

变更后：
```rust
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

- 移除 `provider_type`（id 直接映射到 trait 实现）
- 移除 `endpoint`（写死在各 Provider 内部）

#### 2.2 前端类型同步

TypeScript `ProviderConfig`：
```typescript
export interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  models: ModelConfig[];
}
```

#### 2.3 内置 Provider 定义

前后端 `BUILTIN_PROVIDERS` 简化为 id + name + 默认模型，不再包含 type 和 endpoint。

Rust：
```rust
fn builtin_providers() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            id: "dashscope".to_string(),
            name: "阿里云".to_string(),
            api_key: Some(String::new()),
            models: DashScopeProvider::default_models(),
        },
        ProviderConfig {
            id: "vertex".to_string(),
            name: "Vertex AI".to_string(),
            api_key: None,
            models: VertexAIProvider::default_models(),
        },
        ProviderConfig {
            id: "sensevoice".to_string(),
            name: "SenseVoice (本地)".to_string(),
            api_key: None,
            models: SenseVoiceProvider::default_models(),
        },
    ]
}
```

#### 2.4 配置迁移

`load_config` 迁移逻辑：
- 旧配置中的 `type` 和 `endpoint` 字段通过 serde 忽略（struct 中不再定义，serde 默认跳过未知字段）
- `merge_builtin_providers` 简化为：只补缺失的内置 Provider，修正 name
- 用户自定义的模型列表保留

#### 2.5 校验简化

`validate_config`：
- 移除 provider_type 校验
- 移除 endpoint URL 格式校验
- 保留 id 非空、name 非空校验

### 3. 前端 UI 变更

#### 3.1 模型页面

文件：`src/routes/models/+page.svelte`

移除：
- "添加 Provider" 虚线卡片按钮
- Add Provider 对话框（Name/Type/ID/Endpoint 表单）
- `PROVIDER_TYPES` 常量
- `handleAddProvider()` 函数
- Endpoint 可编辑字段（EditableField）
- `validateForm()` 中与 type/endpoint 相关的校验
- `handleProviderFieldChange()` 中对 endpoint 的处理

保留：
- Provider 卡片展示（id + name）
- API Key 输入（仅需要密钥的 Provider 显示）
- 模型列表编辑（添加/删除模型、修改能力）
- 连通性测试

#### 3.2 Onboarding 向导简化

文件：`src/lib/components/onboarding/steps/ProviderConfigStep.svelte`

简化为：
1. 自动检测 Vertex AI 环境（保留现有逻辑）
2. 有 Vertex ADC → 直接通过
3. 没有 → 引导用户输入通义 API Key
4. 连通性测试
5. 完成

移除：手动添加 Provider 的弹窗（Name/Type/ID/Endpoint 表单）。

#### 3.3 API Key 判断逻辑

不再通过 `provider.type` 判断，改为查询内置元数据：
```typescript
const PROVIDERS_REQUIRING_KEY = ['dashscope'];
function needsApiKey(providerId: string): boolean {
    return PROVIDERS_REQUIRING_KEY.includes(providerId);
}
```

### 4. 调用链路变更

当前：
```
lib.rs → ai.rs::send_audio_prompt() → match provider_type
    "vertex" → rig_vertexai::Client
    "openai-compatible" → rig::providers::openai::Client
```

变更后：
```
lib.rs → providers::create_provider(&config) → provider.transcribe(...)
```

`ai.rs` 大幅简化为对 `Provider` trait 的薄包装层。`VertexClientCache` 被移除，各 Provider 自行管理内部状态（如 Vertex AI 的 token 缓存）。

### 5. 错误处理

复用现有 `AiError` 枚举：
- `ProviderNotFound(id)` — 未知的 provider id
- `MissingApiKey(id)` — 需要 key 但未配置
- `RequestError(msg)` — HTTP 错误、超时、响应解析失败
- `FileReadError(msg)` — 音频文件读取失败

### 6. 文件变更清单

新增：
- `src-tauri/src/providers/mod.rs`
- `src-tauri/src/providers/dashscope.rs`
- `src-tauri/src/providers/vertex.rs`
- `src-tauri/src/providers/sensevoice.rs`

主要修改：
- `src-tauri/Cargo.toml` — 移除 rig 依赖
- `src-tauri/src/ai.rs` — 大幅简化
- `src-tauri/src/config.rs` — ProviderConfig 简化
- `src-tauri/src/lib.rs` — 适配新调用链路
- `src-tauri/src/real_llm_client.rs` — 适配新 trait
- `src-tauri/src/skills.rs` — 适配新 trait
- `src-tauri/src/translation.rs` — 适配新 trait
- `src/lib/stores/config.ts` — 类型简化
- `src/routes/models/+page.svelte` — 移除 Add Provider、Endpoint
- `src/lib/components/onboarding/steps/ProviderConfigStep.svelte` — 简化

## 不涉及

- SenseVoice 本地推理逻辑不变
- Skills 功能逻辑不变
- 快捷键、录音、剪贴板模块不变
- 通知、托盘、浮动指示器不变

## 成功标准

1. `cargo build` 编译通过，无 rig-core/rig-vertexai 依赖
2. 通义转写和文本润色功能正常工作
3. Vertex AI 转写和文本润色功能正常工作
4. SenseVoice 本地转写功能正常工作
5. UI 不显示 Endpoint，不显示 Add Provider 按钮
6. 旧配置文件自动迁移，用户无需手动重置
7. Onboarding 简化后流程顺畅
