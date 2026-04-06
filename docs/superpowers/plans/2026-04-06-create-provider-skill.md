# Create Provider Skill 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建全局 Skill `create-provider`，自动化新增 AI Provider 到 TalkShow 应用中。

**Architecture:** 单文件 SKILL.md，内嵌 Provider trait 架构知识和代码模板，6 步流水线引导从 API 调研到代码生成到系统注册的全流程。

**Tech Stack:** Markdown (SKILL.md)、Rust (Provider trait)、TypeScript (前端 config)

---

### Task 1: 创建 Skill 目录和 SKILL.md 基础骨架

**Files:**
- Create: `~/.claude/skills/create-provider/SKILL.md`

- [ ] **Step 1: 创建 skill 目录**

```bash
mkdir -p ~/.claude/skills/create-provider
```

- [ ] **Step 2: 写入 SKILL.md 的 frontmatter 和 Overview 部分**

使用 Write 工具创建 `~/.claude/skills/create-provider/SKILL.md`，写入以下内容：

```markdown
---
name: create-provider
description: Use when adding a new AI provider to the TalkShow application, implementing the Provider trait and registering it across frontend and backend
---

# Create Provider

自动化新增 AI Provider 到 TalkShow 应用。用户只需提供 Provider 名称，AI 自动完成从 API 调研、代码生成到系统注册的全部工作。

## When to Use

当用户请求以下操作时使用此 skill：
- "添加 XXX Provider" / "添加 XXX 作为 Provider"
- "集成 XXX API" / "接入 XXX"
- "新增 XXX 支持"
- "XXX 作为一个新的 AI 服务商"

当用户说"添加 OpenAI Provider"时，此 skill 自动执行 6 步流程。

**不用于：**
- 修改现有 Provider 的配置
- 调试 Provider 连接问题
- 管理 Provider 的模型列表（这是 UI 操作）
```

- [ ] **Step 3: 验证文件创建成功**

```bash
ls -la ~/.claude/skills/create-provider/SKILL.md
```

Expected: 文件存在且非空

---

### Task 2: 写入 Provider 架构参考信息

**Files:**
- Modify: `~/.claude/skills/create-provider/SKILL.md` (在 Overview 之后追加)

- [ ] **Step 1: 在 "When to Use" 之后，追加 Provider 架构参考部分**

在 SKILL.md 中追加以下内容。这部分为 AI 提供架构上下文，确保生成的代码与系统一致。

```markdown
## Provider Architecture Reference

AI 在执行流程前必须理解以下架构知识。

### Provider Trait 签名

```rust
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
    fn default_models() -> Vec<ModelConfig> where Self: Sized;
}
```

### 关键类型

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingMode { Default, Enabled, Disabled }

#[derive(Debug)]
pub enum ProviderError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
    UnsupportedOperation(String),
}

pub struct ProviderContext {
    pub sensevoice_engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
}
```

### 数据模型

```rust
pub struct ModelConfig {
    pub name: String,
    pub capabilities: Vec<String>,  // "transcription" | "chat"
    pub verified: Option<ModelVerified>,
}

pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub models: Vec<ModelConfig>,
}
```

### 文件位置

| 文件 | 职责 |
|------|------|
| `src-tauri/src/providers/mod.rs` | Provider trait 定义、工厂函数 `create_provider()` |
| `src-tauri/src/providers/{id}.rs` | 具体 Provider 实现 |
| `src-tauri/src/config.rs` | `builtin_providers()` — 后端注册表 |
| `src/lib/stores/config.ts` | `BUILTIN_PROVIDERS` — 前端注册表 |
| `src/lib/stores/config.ts` | `PROVIDERS_REQUIRING_KEY` — 需要 API Key 的 Provider 列表 |
| `src-tauri/src/providers/mod.rs` | `ProviderContext` — 共享依赖容器 |

### 前端类型 (TypeScript)

```typescript
interface ModelConfig {
  name: string;
  capabilities: string[];  // "transcription" | "chat"
  verified?: { status: "ok" | "error"; tested_at: string; latency_ms?: number; message?: string };
}

interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  models: ModelConfig[];
}
```
```

---

### Task 3: 写入 6 步流程 (Step 1-2: 收集信息 + 查询 API)

**Files:**
- Modify: `~/.claude/skills/create-provider/SKILL.md`

- [ ] **Step 1: 在架构参考之后追加 Step 1-2**

```markdown
## The Process

### Step 1: 收集 Provider 信息

从用户输入中提取基础信息：

1. **Provider 名称** — 如 "OpenAI"、"Azure Speech"、"Groq"
2. **是否为本地模型** — 判断是否需要 API Key
3. **已知 API 文档 URL** — 用户可能提供

如果用户消息已包含足够信息（如"添加 OpenAI Provider"），跳过提问直接进入 Step 2。

如果信息不足，向用户确认：
- 该 Provider 是否为本地模型（不需要 API Key）？
- 是否有 API 文档链接？

### Step 2: 查询 API 文档

**必须查询官方文档，不可猜测 API 格式。**

使用 `webfetch` 工具访问 Provider 官方 API 文档，提取以下信息：

1. **Base URL** — API 的基础地址（写死在实现中）
2. **认证方式** — Bearer Token / API Key Header / OAuth / ADC / 无
3. **音频转写接口**：
   - Endpoint 路径（如 `/v1/audio/transcriptions`）
   - 请求格式（multipart/form-data vs JSON with base64）
   - 响应格式（如何提取转写文本）
   - 支持的音频格式
4. **文本完成接口**：
   - Endpoint 路径（如 `/v1/chat/completions`）
   - 请求格式（messages 结构）
   - 响应格式（如何提取回复文本）
   - 是否支持 thinking/reasoning mode
5. **可用模型列表** — 每个模型的名称和适用场景

**抓取策略**（按优先级）：

1. 用户提供了文档 URL → 直接用 `webfetch` 抓取
2. 用户未提供 → 基于 Provider 名称构造官网 URL：
   - OpenAI → `https://platform.openai.com/docs/api-reference`
   - Groq → `https://console.groq.com/docs/api`
   - 其他 → 搜索 `"{provider_name} API documentation"`
3. 如果主要文档页面是索引页，尝试访问子页面：
   - 音频转写 → 搜索 `"audio transcription"` 相关链接
   - 聊天完成 → 搜索 `"chat completions"` 相关链接

**STOP**: 如果无法找到可靠的 API 文档，停止并告知用户。不要猜测 API 格式。

将提取的信息整理成以下格式呈现给用户确认：

```
Provider: {name}
Base URL: {url}
认证方式: {auth_method}
音频转写: {endpoint} ({request_format})
文本完成: {endpoint} ({request_format})
ThinkingMode: 支持/不支持
可用模型: {model_list}
```
```

---

### Task 4: 写入 6 步流程 (Step 3-4: 模型分析 + 生成代码)

**Files:**
- Modify: `~/.claude/skills/create-provider/SKILL.md`

- [ ] **Step 1: 在 Step 2 之后追加 Step 3-4**

```markdown
### Step 3: 分析模型与能力

基于 Step 2 提取的 API 信息，整理模型能力矩阵。

向用户呈现以下表格（示例格式）：

| 模型 | transcription | chat | 备注 |
|------|:---:|:---:|------|
| whisper-1 | ✓ | | 语音转写专用模型 |
| gpt-4o | ✓ | ✓ | 多模态，支持音频输入 |
| gpt-4o-mini | | ✓ | 轻量级文本模型 |

**推荐默认模型（至少 1-2 个）：**
- **transcription**: 推荐 `{model}` — {推荐原因，如"速度快、成本低、多语言支持"}
- **chat**: 推荐 `{model}` — {推荐原因，如"通用能力强、响应快"}

**默认模型选择原则：**
- 转写模型优先：速度快 > 成本低 > 多语言支持 > 准确率高
- 聊天模型优先：通用能力 > 响应速度 > 成本
- 必须至少选择一个 transcription 模型（核心功能）
- 如果模型同时支持 transcription 和 chat，标注两个 capability

**capabilities 取值：**
- `"transcription"` — 语音转文字能力
- `"chat"` — 文本对话能力

等待用户确认或调整选择后，记录最终模型列表和 capabilities。

### Step 4: 生成 Rust 实现

创建 `src-tauri/src/providers/{id}.rs`。

**生成规则：**
- 结构体持有认证所需字段（api_key / token_cache 等）
- 使用 `reqwest` 做 HTTP 请求
- 所有日志使用 `logger.info()` / `logger.error()` 格式（参考现有 Provider）
- 错误统一使用 `ProviderError` 变体
- `ThinkingMode::Default` 表示不传思考参数；`Enabled`/`Disabled` 根据 API 实际支持处理
- 不支持的操作返回 `ProviderError::UnsupportedOperation`

**代码模板 — Bearer Token 认证型 Provider：**

```rust
use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;

const {PROVIDER_UPPER}_BASE_URL: &str = "{base_url}";

pub struct {Name}Provider {
    pub api_key: Option<String>,
}

impl {Name}Provider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    fn get_api_key(&self) -> Result<&str, ProviderError> {
        self.api_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| ProviderError::MissingApiKey("{id}".to_string()))
    }
}

#[async_trait]
impl Provider for {Name}Provider {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError> {
        let api_key = self.get_api_key()?;

        // 根据 Step 2 查询的 API 格式构建请求：
        // - multipart/form-data（参考 dashscope.rs）
        // - JSON with base64（参考 vertex.rs）
        // 使用实际 API 的 endpoint、请求格式、响应格式

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{{}}{{}}", {PROVIDER_UPPER}_BASE_URL, "{endpoint}"))
            .header("Authorization", format!("Bearer {{}}", api_key))
            // ... 构建请求体
            .send()
            .await
            .map_err(|e| ProviderError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestError(format!("HTTP {{}} - {{}}", status, body)));
        }

        // 根据 API 实际响应结构解析
        // ...
        Ok(text)
    }

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        // 如果 API 不支持文本完成：
        // return Err(ProviderError::UnsupportedOperation(
        //     "{Name} does not support text completion".to_string(),
        // ));

        // 如果支持，根据 API 格式构建请求
        // 根据 API 是否支持 thinking mode 处理 ThinkingMode 参数
        // ...
        Ok(text)
    }

    fn needs_api_key(&self) -> bool {
        {true_or_false}  // 根据 Step 2 确定的认证方式
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![
            // 根据 Step 3 用户确认的模型列表生成
            ModelConfig {
                name: "{model_name}".to_string(),
                capabilities: vec!["{capability}".to_string()],
                verified: None,
            },
        ]
    }
}
```

**如果 Provider 使用非 Bearer Token 认证**（如 ADC、OAuth），参考 `vertex.rs` 的 token 缓存模式。在结构体中添加 `Arc<Mutex<...>>` 管理认证状态。

**关键：生成的代码必须使用 Step 2 查询到的实际 API 格式，不可照搬模板中的占位符。**
```

---

### Task 5: 写入 6 步流程 (Step 5-6: 注册 + 验证)

**Files:**
- Modify: `~/.claude/skills/create-provider/SKILL.md`

- [ ] **Step 1: 在 Step 4 之后追加 Step 5-6**

```markdown
### Step 5: 注册到系统

按以下顺序修改 4 个文件。每修改一个文件后立即检查一致性。

**5.1 修改 `src-tauri/src/providers/mod.rs`**

添加模块声明：
```rust
pub mod {id};  // 在已有的 pub mod 声明之后添加
```

添加工厂分支（在 `create_provider()` 函数的 `match` 中添加）：
```rust
"{id}" => Ok(Box::new({id}::{Name}Provider::new(config.api_key.clone()))),
```

如果 Provider 需要共享状态（如本地引擎），需要修改 `ProviderContext` 并传递参数。

**5.2 修改 `src-tauri/src/config.rs`**

在 `builtin_providers()` 函数的 `vec![]` 中添加：
```rust
ProviderConfig {
    id: "{id}".to_string(),
    name: "{显示名称}".to_string(),
    api_key: Some(String::new()),  // 或 None（如不需要 API Key）
    models: vec![
        ModelConfig {
            name: "{model_name}".to_string(),
            capabilities: vec!["{capability}".to_string()],
            verified: None,
        },
    ],
},
```

**5.3 修改 `src/lib/stores/config.ts`**

在 `BUILTIN_PROVIDERS` 数组中添加：
```typescript
{
  id: '{id}',
  name: '{显示名称}',
  api_key: '',  // 如果不需要 api_key，删除此行
  models: [
    { name: '{model_name}', capabilities: ['{capability}'] }
  ]
}
```

如果需要 API Key，更新 `PROVIDERS_REQUIRING_KEY` 数组：
```typescript
export const PROVIDERS_REQUIRING_KEY = ['dashscope', '{id}'];
```

**5.4 一致性检查（必须立即执行）**

逐项核对：
- [ ] 后端 `builtin_providers()` 的 `id` 与前端 `BUILTIN_PROVIDERS` 一致
- [ ] 后端 `builtin_providers()` 的 `name` 与前端 `BUILTIN_PROVIDERS` 一致
- [ ] 后端 `models` 列表与前端 `models` 列表一致
- [ ] `needs_api_key()` 返回 `true` ↔ `PROVIDERS_REQUIRING_KEY` 包含该 id
- [ ] `needs_api_key()` 返回 `false` ↔ `api_key` 字段为 `None`（后端）/ 不存在（前端）
- [ ] `default_models()` 返回值与 `builtin_providers()` 中的 models 一致
- [ ] 工厂函数中的 `new()` 参数与结构体定义匹配

**如果不一致，立即修复再继续。**

### Step 6: 验证

**6.1 编译检查**

```bash
cargo check
```
在 `src-tauri/` 目录下执行。

如果失败，根据错误信息修复代码后重新 `cargo check`。常见问题：
- 缺少 `use` 导入
- 方法签名与 trait 不匹配
- 字段类型不一致

**6.2 前后端最终一致性复查**

再次核对 Step 5.4 的检查清单，确认所有修改正确。

**6.3 完成**

告知用户新 Provider 已添加成功，总结以下信息：
- Provider ID 和名称
- 支持的功能（transcription / chat）
- 默认模型
- 是否需要 API Key
- 提醒用户：前端 UI 无需改动，会自动渲染新 Provider
```

---

### Task 6: 写入 Red Flags 和注意事项

**Files:**
- Modify: `~/.claude/skills/create-provider/SKILL.md`

- [ ] **Step 1: 在 Step 6 之后追加 Red Flags 和注意事项**

```markdown
## Red Flags

| 如果你在想... | 现实是... |
|--------------|---------|
| "API 格式我大概知道，不用查文档" | **必须查官方文档**。API 格式各不相同，猜测会导致运行时错误 |
| "模型能力差不多，随便标就行" | capabilities 决定模型是否出现在 UI 下拉框中。标错 = 用户选了不能用的模型 |
| "前后端注册表差一点没关系" | `builtin_providers()` 和 `BUILTIN_PROVIDERS` 必须严格一致。`merge_builtin_providers()` 会根据 id 做 merge，不一致会导致重复或丢失 |
| "先写完代码，验证最后再说" | 每步完成后立即验证。积累的错误更难定位 |
| "这个 Provider 和 XXX 很像，直接复制改改" | 认证方式、请求格式、响应结构通常不同。必须基于官方文档逐项确认 |
| "用户没提模型，我就不选默认模型" | 必须推荐至少 1 个默认模型。没有默认模型 = 用户无法使用该 Provider |
| "先不加到前端注册表，后端能编译就行" | 前后端必须同步注册。只改后端 = 前端看不到该 Provider |
| "ThinkingMode 先不管，以后再加" | `complete_text` 必须处理 `ThinkingMode` 参数。不支持就忽略，不能遗漏 match 分支 |

## Provider 特性自动识别

在 Step 2 查询 API 后，自动分析以下特性维度：

| 特性维度 | 判断方式 | 影响范围 |
|---------|---------|---------|
| 认证方式 | API 文档 Auth 章节 | `needs_api_key()` 返回值、`PROVIDERS_REQUIRING_KEY` |
| API 协议 | OpenAI 兼容 vs 自有协议 | 请求构建方式（是否可用 OpenAI 兼容格式简化） |
| 音频上传格式 | multipart vs base64 JSON | `transcribe()` 的请求构建 |
| ThinkingMode | API 是否有 thinking/reasoning 参数 | `complete_text()` 中对 ThinkingMode 的处理 |
| 文本完成支持 | 是否有 chat/completions 类接口 | `complete_text()` 实现或返回 `UnsupportedOperation` |
| 本地/云端 | 是否需要网络请求 | 是否使用 `reqwest`、是否需要 API Key |

## 已知 Provider 的 API 文档入口

以下是常见 AI Provider 的 API 文档 URL，供 Step 2 查询时参考：

| Provider | API 文档 URL |
|----------|-------------|
| OpenAI | https://platform.openai.com/docs/api-reference |
| Groq | https://console.groq.com/docs/api |
| Anthropic | https://docs.anthropic.com/en/api |
| Google AI (Gemini) | https://ai.google.dev/api |
| AWS Bedrock | https://docs.aws.amazon.com/bedrock/ |
| Azure OpenAI | https://learn.microsoft.com/en-us/azure/ai-services/openai/ |
| DeepSeek | https://api-docs.deepseek.com/ |
| Together AI | https://docs.together.ai/ |
| Fireworks AI | https://docs.fireworks.ai/ |

## 范围边界

**此 Skill 负责：**
- 查询 Provider 官方 API 文档
- 生成 Provider trait 的 Rust 实现
- 注册到前后端注册表
- 编译验证

**此 Skill 不负责：**
- 前端 UI 定制（Provider 卡片自动渲染，无需改动）
- Provider 的单元测试编写
- 复杂认证流程实现（OAuth 回调等需要人工介入）
- git commit（由用户决定）
```

---

### Task 7: 验证 Skill 结构并提交

**Files:**
- Read: `~/.claude/skills/create-provider/SKILL.md`

- [ ] **Step 1: 验证 SKILL.md 结构完整性**

确认文件包含以下所有部分：
- [ ] YAML frontmatter（`name` + `description`）
- [ ] # Create Provider 标题
- [ ] ## When to Use
- [ ] ## Provider Architecture Reference
- [ ] ## The Process（含 Step 1-6）
- [ ] ## Red Flags
- [ ] ## Provider 特性自动识别
- [ ] ## 已知 Provider 的 API 文档入口
- [ ] ## 范围边界

- [ ] **Step 2: 验证 frontmatter 格式**

```bash
head -5 ~/.claude/skills/create-provider/SKILL.md
```

Expected: 以 `---` 开头，包含 `name: create-provider` 和 `description:` 字段

- [ ] **Step 3: 验证 description 符合规范**

确认 description：
- 以 "Use when" 开头
- 描述触发条件，不总结工作流程
- 不超过 500 字符

- [ ] **Step 4: 统计文件字数**

```bash
wc -w ~/.claude/skills/create-provider/SKILL.md
```

Expected: 大约 800-1500 词（这是流程型 skill，不在高频加载范围内，可以适当详细）

- [ ] **Step 5: Commit**

```bash
cd /Users/jia.xia/development/talkshow
git add -A ~/.claude/skills/create-provider/SKILL.md
git commit -m "feat: add create-provider skill for automating provider integration"
```

注意：如果 `~/.claude/skills/` 不在 talkshow 的 git 仓库内，则无需 git commit，仅确认文件存在即可。
