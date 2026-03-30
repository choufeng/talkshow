# Skills 文本处理系统设计

## 概述

在语音转写和剪贴板粘贴之间插入一个 LLM 文本处理管线。用户可以创建和管理"技能"（Skill），每个 Skill 是一段指令性 prompt，用于对转写文字进行修饰、纠错、格式化等处理。系统根据当前前台应用的 Bundle ID 作为上下文，由 LLM 智能判断应激活哪些 Skill 规则。

**核心原则**: 预置 + 自定义双模式，扁平 Skill 模型（无技能包），单次 LLM 调用合并所有 Skill。

## 架构

### 改造后的完整流程

```
全局快捷键录音
    ↓
音频录制 (recording.rs)                    [不变]
    ↓
AI 转写 (ai.rs / sensevoice.rs)            [不变]
    ↓
[新增] Skills 文本处理管线 (skills.rs)
    ├─ 检查 Skills 功能是否启用
    │   └─ 未启用 → 跳过，直接输出转写文字
    ├─ 获取前台应用信息 (macOS NSWorkspace)
    ├─ 收集所有已启用的 Skill
    ├─ 组装合并 Prompt:
    │   ├─ 框架 prompt（角色定义 + 输出格式约束）
    │   ├─ Bundle ID 上下文（当前应用信息）
    │   ├─ 各 Skill 指令（逐条嵌入）
    │   └─ 转写文字作为 user message
    ├─ 调用 Skills 专用 LLM（独立 Provider/Model）
    └─ 返回处理后的文本
    ↓
剪贴板写入 + 自动粘贴 (clipboard.rs)       [不变]
```

### 模块依赖

```
src-tauri/src/
├── lib.rs           # 修改：stop_recording 中插入 Skills 管线调用
├── skills.rs        # 新增：Skill 数据模型、Prompt 组装、Bundle ID 获取、LLM 调用
├── ai.rs            # 复用：send_text_prompt 用于 Skills LLM 调用
├── config.rs        # 修改：新增 SkillsConfig 数据模型
├── recording.rs     # 不变
├── sensevoice.rs    # 不变
├── clipboard.rs     # 不变
└── logger.rs        # 复用：Skills 管线的日志记录

src/lib/
├── stores/
│   └── config.ts    # 修改：新增 SkillsConfig 的前端 Store 管理
├── components/
│   └── ui/          # 复用现有组件
└── routes/
    ├── +layout.svelte         # 修改：侧边栏新增"技能"导航项
    └── skills/+page.svelte    # 新增：Skills 管理 UI 页面
```

## 数据模型

### Rust 端 (config.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,              // UUID
    pub name: String,            // 显示名称
    pub description: String,     // 简短描述（UI 展示 + LLM 路由参考）
    pub prompt: String,          // 发送给 LLM 的指令性 prompt
    pub builtin: bool,           // 是否为预置 Skill
    pub enabled: bool,           // 是否启用
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub enabled: bool,           // Skills 功能全局开关，默认 true
    pub skills: Vec<Skill>,      // 所有 Skill 列表
    pub provider_id: String,     // Skills 专用的 LLM Provider ID
    pub model: String,           // Skills 专用模型名称
}
```

`SkillsConfig` 嵌入到现有 `AppConfig.features` 中：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub transcription: TranscriptionConfig,
    pub skills: SkillsConfig,    // 新增
}
```

### 前端 (config.ts)

```typescript
interface Skill {
    id: string;
    name: string;
    description: string;
    prompt: string;
    builtin: boolean;
    enabled: boolean;
}

interface SkillsConfig {
    enabled: boolean;
    skills: Skill[];
    provider_id: string;
    model: string;
}

interface FeaturesConfig {
    transcription: TranscriptionConfig;
    skills: SkillsConfig;           // 新增
}
```

## Prompt 组装策略

### 合并 Prompt 模板

```
你是一个语音转文字的文本处理助手。请根据以下规则处理用户的输入文本。

当前用户正在使用的应用是：{app_name} ({bundle_id})

请仅应用与当前场景相关的规则，跳过不适用的规则。

{遍历所有 enabled 的 Skill，逐条嵌入：}

---
【{skill.name}】
{skill.prompt}

---

输入文本：
{transcription_text}

请只输出处理后的纯文本。不要添加任何解释、标注或前缀。
```

### 设计要点

1. **指令性 prompt**: 每个 Skill 的 prompt 必须告诉 LLM **怎么处理**，不是描述功能
2. **正交性**: 各 Skill 的职责不重叠（如"错别字修正"和"格式化"不同时处理标点）
3. **Bundle ID 作为上下文**: LLM 自行判断哪些规则与当前场景相关，无需显式绑定
4. **输出约束**: 明确要求只输出处理后的纯文本，避免 LLM 添加解释性内容

## 预置 Skill

首次启动时写入以下默认 Skill（`builtin: true`，用户可编辑，不可删除）：

| Skill | prompt 摘要 |
|-------|-------------|
| 语气词剔除 | 去除"嗯"、"啊"、"那个"、"就是"、"然后"等无意义口头语气词 |
| 错别字修正 | 识别并修正文本中的错别字、同音错误和常见输入错误 |
| 口语润色 | 保持口语化风格，但使表达更流畅自然 |
| 书面格式化 | 将口语化的表达转换为规范的书面表达，适合邮件和文档场景 |

## UI 设计

### 侧边栏

在"模型"和"设置"之间新增"技能"导航项，使用 `Sparkles` (lucide-svelte) 图标。

### Skills 管理页面 (`/skills`)

```
┌─────────────────────────────────────────────────┐
│  技能设置                                         │
│                                                   │
│  [全局开关: Skills 功能 ON/OFF]                    │
│                                                   │
│  ── LLM 服务 ──────────────────────────────────   │
│  Provider: [选择 Provider ▾]   Model: [选择 ▾]    │
│                                                   │
│  ── 技能列表 ──────────────────────────────────   │
│  [ + 添加自定义 Skill ]                           │
│                                                   │
│  ┌───────────────────────────────────────────┐   │
│  │ [开关] 语气词剔除                    [预置] │   │
│  │ 去除嗯、啊等无意义口头语气词         [编辑] │   │
│  └───────────────────────────────────────────┘   │
│  ┌───────────────────────────────────────────┐   │
│  │ [开关] 错别字修正                    [预置] │   │
│  │ 识别并修正文本中的错别字             [编辑] │   │
│  └───────────────────────────────────────────┘   │
│  ┌───────────────────────────────────────────┐   │
│  │ [开关] 口语润色                      [预置] │   │
│  │ 保持口语化但更流畅                   [编辑] │   │
│  └───────────────────────────────────────────┘   │
│  ┌───────────────────────────────────────────┐   │
│  │ [开关] 书面格式化                    [预置] │   │
│  │ 口语转书面表达                       [编辑] │   │
│  └───────────────────────────────────────────┘   │
│                                                   │
└─────────────────────────────────────────────────┘
```

### Skill 编辑对话框

点击"编辑"弹出对话框，包含：

- **名称** (text input)
- **描述** (text input)
- **Prompt 内容** (textarea，主要编辑区，支持多行)
- **保存 / 取消**

预置 Skill 名称旁显示 `[预置]` 标签，不可删除但可编辑内容。

自定义 Skill 显示 `[自定义]` 标签，可编辑和删除。

## LLM 调用

Skills 管线复用现有 `ai.rs` 的 `send_text_prompt` 能力，但使用独立的 Provider/Model 配置：

- 用户在 Skills 页面选择一个已有的 Provider 和 Model
- 调用时用 system prompt = 合并后的 Skill 指令，user message = 转写文字
- 仅支持 OpenAI-Compatible 和 Vertex AI 两种 provider（SenseVoice 是本地 ASR，不支持文本对话）

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| Skills LLM 调用失败 | 回退输出原始转写文字，不阻塞粘贴流程 |
| Skills LLM 超时 | 默认 10 秒超时，超时后回退原始文字 |
| 未配置 Skills Provider | 跳过 Skills 处理，使用原始转写文字 |
| 所有 Skill 均未启用 | 跳过 LLM 调用，使用原始转写文字 |
| 转写文字为空 | 跳过 Skills 处理 |

所有错误均记录到日志系统（复用 `logger.rs`），不弹窗不打断用户。

## 配置迁移

`FeaturesConfig` 已标注 `#[serde(default)]`，因此当用户从旧版本升级时，`config.json` 中不存在 `features.skills` 字段，serde 会自动调用 `SkillsConfig::default()` 填充默认值。无需额外的迁移逻辑。

需确保 `SkillsConfig` 也标注 `#[serde(default)]` 并实现合理的 `Default`：
- `enabled: true`
- `skills`: 4 个预置 Skill（语气词剔除、错别字修正、口语润色、书面格式化）
- `provider_id: ""` （空字符串，表示未配置）
- `model: ""`

## 任务拆分

本功能拆分为 6 个独立子任务，每个子任务有独立的设计文档和实施计划，可并行或串行执行：

| # | 任务 | 依赖 | 说明 |
|---|------|------|------|
| T1 | 配置层扩展 | 无 | 数据模型定义、配置持久化、前后端接口对齐 |
| T2 | Skills 核心引擎 | T1 | Prompt 组装、Bundle ID 获取、LLM 调用 |
| T3 | 录音流程集成 | T2 | lib.rs 改造，插入 Skills 管线 |
| T4 | Skills UI 页面 | T1 | Skill 的 CRUD 操作界面 |
| T5 | 导航集成 | T4 | 侧边栏导航、全局开关 UI |
| T6 | 预置 Skill 与验证 | T3, T5 | 内置默认 Skill、端到端验证 |

任务依赖关系：
```
T1 (配置层) ──→ T2 (核心引擎) ──→ T3 (流程集成) ──┐
   │                                              ├──→ T6 (预置与验证)
   └──→ T4 (UI 页面) ──→ T5 (导航集成) ──────────┘
```
