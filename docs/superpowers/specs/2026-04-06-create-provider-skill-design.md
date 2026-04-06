# Create Provider Skill 设计文档

**日期**: 2026-04-06
**状态**: 已批准

## 概述

创建一个全局 Skill `create-provider`，用于自动化新增 AI Provider 到 TalkShow 应用中。用户只需说出 Provider 名称（如"添加 OpenAI Provider"），AI 将自动完成从 API 调研、代码生成到系统注册的全部工作。

## 设计决策

| 决策项 | 选择 | 理由 |
|-------|------|------|
| 自动化程度 | 全自动执行 | 减少人工操作，AI 查文档、生成代码、注册系统 |
| API 信息获取 | 实时抓取官网文档 | 保证信息准确、无需维护本地缓存 |
| Skill 安装位置 | 全局 `~/.claude/skills/create-provider/` | 跨项目可复用 |
| 模型能力判定 | AI 分析 + 用户确认 | AI 推荐但人类把关 |
| 流程模式 | 6 步流水线 | 清晰可追溯，每步有检查点 |

## Skill 元信息

```yaml
---
name: create-provider
description: Use when adding a new AI provider to the TalkShow application, implementing the Provider trait and registering it across frontend and backend
---
```

**存放位置**: `~/.claude/skills/create-provider/SKILL.md`

**触发条件**: 用户说"添加 XXX Provider"、"集成 XXX API"、"新增 XXX 支持"、"XXX 作为 Provider"

## 6 步流程详细设计

### Step 1: 收集 Provider 信息

AI 从用户输入中提取：

- **Provider 名称**（如 "OpenAI"、"Azure Speech"）
- **是否为本地模型**（判断 needs_api_key）
- **是否有已知 API 文档 URL**（用户可能提供，也可能需要 AI 搜索）

此步**不查询网络**，仅确认基础信息。如果用户消息已包含足够信息（如"添加 OpenAI Provider"），可跳过提问直接进入 Step 2。

### Step 2: 查询 API 文档

AI 使用 `webfetch` 访问 Provider 官方 API 文档，提取：

- **Base URL / Endpoint** — 写死在实现中
- **认证方式** — Bearer Token / API Key / OAuth / ADC / 无
- **音频转写接口** — 请求格式（multipart/form-data vs JSON with base64）、响应格式
- **文本完成接口** — 请求格式、响应格式、是否支持 thinking/reasoning mode
- **支持的模型列表** — 每个模型的名称、适用场景

**抓取策略**:

1. 如果用户提供了文档 URL → 直接抓取
2. 如果没有 → AI 基于已知信息构造官网 URL 抓取（如 `https://platform.openai.com/docs/api-reference`）
3. 如果官网文档结构复杂，尝试搜索 `"{provider_name} API documentation audio transcription"`

**Red Flag**: 如果无法找到可靠的 API 文档，**停止**并告知用户，不要猜测 API 格式。

### Step 3: 分析模型与能力

AI 基于 Step 2 的信息整理模型能力矩阵：

| 模型 | transcription | chat | 备注 |
|------|:---:|:---:|------|
| 示例: whisper-1 | ✓ | | 语音转写专用 |
| 示例: gpt-4o | ✓ | ✓ | 多模态，支持音频输入 |
| 示例: gpt-4o-mini | | ✓ | 轻量文本模型 |

**向用户呈现**：

1. 候选模型列表 + AI 推荐的 capabilities
2. **推荐至少 1-2 个模型作为默认值**（标注推荐原因）
3. 用户确认或调整

**默认模型选择原则**:

- 转写优先选：速度快、成本低、支持多语言
- 文本对话优先选：通用能力强、响应快
- 至少选一个 transcription 模型（核心功能）

### Step 4: 生成 Rust 实现

创建 `src-tauri/src/providers/{id}.rs`，包含：

- `struct {Name}Provider` — 持有 api_key / auth config 等
- `impl Provider for {Name}Provider` — transcribe + complete_text
- 认证逻辑（Bearer token / ADC / 其他）
- 错误处理使用 `ProviderError` 变体
- 如果某操作不支持，返回 `UnsupportedOperation`

**生成规则**:

- 参考现有 provider 实现的代码风格
- 使用 `reqwest` 做 HTTP 请求
- `complete_text` 中根据 API 实际支持情况处理 `ThinkingMode`
- 如果 Provider 不支持文本完成，`complete_text` 返回 `UnsupportedOperation`

### Step 5: 注册到系统

按顺序修改以下文件：

| 文件 | 修改内容 |
|------|---------|
| `src-tauri/src/providers/mod.rs` | 添加 `pub mod {id};` 和 `create_provider()` 工厂分支 |
| `src-tauri/src/config.rs` | 在 `builtin_providers()` 添加 ProviderConfig 条目 |
| `src/lib/stores/config.ts` | 在 `BUILTIN_PROVIDERS` 添加对应条目 |
| `src/lib/stores/config.ts` | 如需 API Key，更新 `PROVIDERS_REQUIRING_KEY` 数组 |
| `src-tauri/src/providers/mod.rs` | 如需共享状态，在 `ProviderContext` 添加新字段 |

**一致性检查**（必须在注册完成后立即执行）:

- 后端 `builtin_providers()` 的 id/name/models 必须与前端 `BUILTIN_PROVIDERS` 一致
- `needs_api_key()` 返回值必须与 `PROVIDERS_REQUIRING_KEY` 一致
- `default_models()` 返回值必须与配置中的 models 一致

### Step 6: 验证

1. `cargo check` — 编译检查（在 `src-tauri/` 目录下执行）
2. 前后端一致性检查 — 对比 Rust 和 TypeScript 的 Provider 定义
3. 如项目有 lint/typecheck 命令，一并运行

**Red Flag**: 如果 `cargo check` 失败，**不要继续**，修复后再验证。

## Provider 特性自动识别

AI 在 Step 2 查询 API 后，自动填充以下特性矩阵：

| 特性维度 | 判断方式 | 影响范围 |
|---------|---------|---------|
| **认证方式** | 从 API 文档 Auth 章节提取 | `needs_api_key()` 返回值、`PROVIDERS_REQUIRING_KEY` |
| **API 协议** | OpenAI 兼容 vs 自有协议 | 请求构建方式 |
| **音频上传格式** | multipart vs base64 JSON | `transcribe()` 实现 |
| **是否支持 ThinkingMode** | API 是否有 enable_thinking / reasoning 参数 | `complete_text()` 中对 ThinkingMode 的处理 |
| **是否支持文本完成** | 是否有 chat/completions 类接口 | 是否让 `complete_text()` 返回 `UnsupportedOperation` |
| **本地/云端** | 是否需要网络请求 | 是否需要 `reqwest`、是否需要 API Key |

## Red Flags

| 如果你在想... | 现实是... |
|--------------|---------|
| "API 格式我大概知道，不用查文档" | 必须查官方文档，API 格式各不相同 |
| "模型能力差不多，随便标就行" | capabilities 决定模型是否出现在 UI 下拉框中，标错会导致用户选了不能用的模型 |
| "前后端注册表差一点没关系" | `builtin_providers()` 和 `BUILTIN_PROVIDERS` 必须严格一致，否则 merge 逻辑会出错 |
| "先写代码，验证最后再说" | 每步完成后立即验证，不要积累错误 |
| "这个 Provider 和 XXX 很像，直接复制改改" | 认证方式、请求格式、响应结构通常不同，必须逐项确认 |
| "用户没提模型，我就不选默认模型" | 必须推荐至少 1 个默认模型，否则用户无法使用该 Provider |

## 集成说明

- **不依赖其他 Skill**，独立运行
- **生成的代码必须通过 `cargo check`** 才算完成
- **不处理前端 UI 改动** — 前端通过 `$config.ai.providers` 动态渲染，无需额外改动 UI 代码
- **不处理 git commit** — 由用户决定何时提交

## 范围边界

**在范围内**:
- 查询 API 文档并提取接口信息
- 生成 Provider trait 的 Rust 实现
- 注册到前后端注册表
- 编译验证

**不在范围内**:
- 前端 UI 定制（Provider 卡片样式、特殊配置界面）
- Provider 的单元测试编写
- 发布流程（版本号、changelog）
- 复杂认证流程（OAuth 回调、多步认证）— 需要人工介入
