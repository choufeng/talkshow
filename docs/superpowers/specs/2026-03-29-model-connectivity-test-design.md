# 模型连通性测试设计

## 概述

在模型配置页面上，为每个模型提供连通性和可用性测试功能。测试结果持久化到配置中，页面重载后仍可见。

## 需求

- **粒度**：Model 级别，每个模型独立测试
- **触发方式**：手动触发（单个模型 + "测试全部"）
- **验证范围**：API 连通性 + 模型可用性（非端到端音频转写）
- **结果持久化**：测试结果保存到 config.json
- **统一策略**：所有 provider 类型（vertex、openai-compatible、stubbed）统一发送实际请求，以 API 响应判断结果，无跳过逻辑

## 架构

```
用户点击测试按钮/标签
        │
        ▼
前端 invoke('test_model_connectivity', { provider_id, model_name })
        │
        ▼
后端 Rust 命令
  - 加载配置 → 查找 provider + model
  - 检查 model.capabilities
  - 包含 "transcription" → 发送内嵌测试音频 + 转写提示词
  - 其他 → 发送纯文本 completion
  - 统一以 API 响应判断结果
  - 更新 config 中 model.verified 字段并保存
  - 返回 TestResult
        │
        ▼
前端更新 UI 状态
```

## 后端变更

### 1. config.rs — 新增 ModelVerified 结构体

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ModelVerified {
    pub status: String,           // "ok" | "error"
    pub tested_at: String,        // ISO 8601
    pub latency_ms: Option<u64>,
    pub message: Option<String>,  // error 详情
}

pub struct ModelConfig {
    pub name: String,
    pub capabilities: Vec<String>,
    pub verified: Option<ModelVerified>,  // 新增，serde default = None
}
```

### 2. ai.rs — 新增文本测试 + 音频字节测试函数

```rust
// 新增：纯文本测试（用于非 transcription 模型）
pub async fn send_text_prompt(
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError>

// 新增：从字节发送音频测试（用于 transcription 模型）
pub async fn send_audio_prompt_from_bytes(
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError>
```

### 3. lib.rs — 新增 Tauri 命令

```rust
#[tauri::command]
async fn test_model_connectivity(
    app_handle: tauri::AppHandle,
    provider_id: String,
    model_name: String,
) -> Result<TestResult, String>
```

逻辑：
1. 加载配置，查找 provider 和 model
2. 根据 `model.capabilities` 包含 `"transcription"` 决定测试方式
3. 发送实际请求（15 秒超时）
4. 构造 `ModelVerified`，更新 config 并保存
5. 返回 `TestResult`

### 4. 测试音频文件

- 使用 `include_bytes!("../assets/test.wav")` 编译时内嵌
- 约为 0.5 秒短音频，几 KB 大小
- 用 macOS `say` 命令生成或录制

## 前端变更

### 1. config.ts — 类型更新

```typescript
interface ModelVerified {
  status: 'ok' | 'error';
  tested_at: string;
  latency_ms?: number;
  message?: string;
}

interface ModelConfig {
  name: string;
  capabilities: string[];
  verified?: ModelVerified;  // 新增
}
```

### 2. +page.svelte — UI 变更

**模型标签状态（4 种）：**

| 状态 | 视觉 | 来源 |
|------|------|------|
| idle | 默认标签样式，无指示器 | `verified` 不存在 |
| testing | 旋转图标 ⟳ | 测试进行中 |
| ok | 绿色边框 + ✓ + 测试日期 | `verified.status === 'ok'` |
| error | 红色边框 + ✕ + 测试日期 | `verified.status === 'error'`，hover 显示详情 |

**新增元素：**
- 每个 Provider 卡片底部添加"测试全部模型"按钮
- 点击模型标签触发单个测试
- 测试全部时顺序执行（非并行），避免 rate limit

**新增状态管理：**
```typescript
let testingModels = $state<Set<string>>(new Set());  // key = "provider_id::model_name"

async function testModel(providerId: string, modelName: string) { ... }
async function testAllModels(provider: ProviderConfig) { ... }
```

## 响应判断

| 响应 | status | 含义 |
|------|--------|------|
| 成功 + 响应非空 | ok | 连通正常，模型可用 |
| 401/403 | error | API Key 无效或无权限 |
| 404 / model not found | error | 模型在 Provider 上不存在 |
| 网络错误 / 超时 | error | Endpoint 不可达 |
| 不支持音频输入 | error | 模型不支持转写能力 |

## 迁移兼容

`verified` 字段为 `Option<ModelVerified>`，serde default 为 `None`。旧配置文件中无此字段时自动反序列化为 `None`，无需额外迁移逻辑。

## 涉及文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/config.rs` | 新增 `ModelVerified`，`ModelConfig` 加 `verified` 字段 |
| `src-tauri/src/ai.rs` | 新增 `send_text_prompt`、`send_audio_prompt_from_bytes` |
| `src-tauri/src/lib.rs` | 新增 `test_model_connectivity` 命令并注册 |
| `src-tauri/assets/test.wav` | 新增测试音频文件 |
| `src/lib/stores/config.ts` | 类型定义更新 |
| `src/routes/models/+page.svelte` | UI 变更（标签状态、测试按钮、测试逻辑） |
