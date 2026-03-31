# AI 翻译功能设计

## 概述

为 TalkShow 新增 AI 翻译功能。用户通过独立的翻译快捷键触发录音，录音内容经转录和润色后，额外调用 LLM 将文本翻译为目标语言，最终粘贴翻译结果。

## 核心设计决策

| 决策 | 选择 | 理由 |
|---|---|---|
| 流水线位置 | Skills 润色之后、粘贴之前 | 翻译清理过的文本质量更高 |
| 实现方式 | 独立 LLM 调用 | 独立性强、容错边界清晰 |
| 模型配置 | 复用润色模型（polish_provider_id / polish_model） | 避免额外配置复杂度 |
| 触发方式 | 独立快捷键，无开关 | 快捷键本身决定流水线，简单直接 |
| 目标语言 | 单一默认语言，固定预设列表 | 满足大部分场景，实现简洁 |
| UI 位置 | 模型页面（目标语言）+ 设置页面（快捷键）+ Skills 页面（翻译 Skill） | 沿用现有页面布局 |

## 配置层变更

### 新增顶层字段

```rust
translate_shortcut: String  // 默认 "Control+Shift+T"
```

### 新增 features.translation

与 `features.transcription` 同级：

```rust
features: {
    transcription: {
        provider_id,
        model,
        polish_enabled,
        polish_provider_id,
        polish_model,
    },
    translation: {
        target_lang: String,  // 默认 "English"，无 enabled 开关
    },
    skills: { ... },
}
```

翻译不设 `enabled` 开关，由快捷键触发决定是否走翻译流程。

### 固定预设语言列表

用户从以下列表中选择 `target_lang`：

English、中文、日本語、한국어、Français、Deutsch、Español、Português、Italiano、Русский、العربية、हिन्दी

### 向后兼容

老配置加载时自动补上：
- `translate_shortcut: "Control+Shift+T"`
- `translation.target_lang: "English"`

遵循项目现有的迁移模式（`config.rs` 中的 `migrate` 逻辑）。

## 快捷键系统

| 快捷键 | 配置项 | 默认值 | 触发的流水线 |
|---|---|---|---|
| `recording_shortcut` | 现有 | Control+Backslash | 转录 → 润色 → 粘贴 |
| `translate_shortcut` | 新增 | Control+Shift+T | 转录 → 润色 → 翻译 → 粘贴 |

后端需要追踪当前录音由哪个快捷键触发，用于决定是否执行翻译步骤。

## 内置翻译 Skill

新增内置 Skill `builtin-translation`，在 Skills 页面管理。

与现有内置 Skill 的区别：**允许用户编辑 prompt**。

默认 prompt 提供通用翻译指导，用户可自定义加入术语、风格、行业要求等。

### 翻译调用时的 Prompt 组装

```
System: [固定基础 prompt] + [翻译 Skill 的自定义 prompt]
User:   [润色后的文本]
```

固定基础 prompt（不可编辑）：

> You are a professional translator. Translate the following text to {target_lang}. Output only the translation, nothing else.

翻译 Skill 的自定义 prompt（可编辑）默认值：

> Preserve the original tone and style. Keep technical terms accurate. If a term has no standard translation, keep it in the original language.

## 后端流水线

```
用户按下翻译快捷键
  → 录音（标记为翻译模式）
  → 再次按下快捷键结束录音
  → 转录
  → Skills 润色
  → 翻译（ThinkingMode::Disabled, 15s 超时）
      ├─ 成功 → 粘贴翻译结果到前台应用
      └─ 失败 → 不粘贴 + 系统通知告知用户翻译失败
```

### 翻译步骤细节

1. 检查录音是否由翻译快捷键触发
2. 使用 `polish_provider_id` + `polish_model` 调用 `send_text_prompt`
3. 强制使用 `ThinkingMode::Disabled`（避免超时）
4. 超时保护：15s
5. 失败时：不粘贴任何内容，通过系统通知告知用户翻译失败

### 前置条件

翻译快捷键触发时，要求润色功能已启用（`polish_enabled == true` 且 `polish_provider_id` + `polish_model` 已配置）。若润色未启用，翻译快捷键按下时通过系统通知提示用户先配置润色模型。

### 录音状态追踪

使用 `AtomicU8` 枚举记录当前录音模式（替换现有的 `RECORDING: AtomicBool`）：

- `0` — 未在录音
- `1` — 普通转录模式（recording_shortcut 触发）
- `2` — 翻译模式（translate_shortcut 触发）

## UI 变更

### 设置页面（`/settings`）

在现有录音快捷键配置下方，新增翻译快捷键配置，使用相同的快捷键录制器组件。

### 模型页面（`/models`）

在润色区块下方新增「AI 翻译」区块：
- 目标语言下拉选择（从固定预设列表中选择）
- 显示当前使用的翻译模型（即润色模型，只读提示）

### Skills 页面（`/skills`）

新增内置 Skill `builtin-translation`：
- 标记为可编辑（与其他内置 Skill 不同）
- 默认启用
- 用户可查看和修改 prompt

## 代码组织

| 变更点 | 文件 | 说明 |
|---|---|---|
| 配置结构 | `config.rs` | 新增 `translate_shortcut`、`TranslationConfig`、迁移逻辑 |
| 流水线编排 | `lib.rs` | 新增翻译快捷键注册、录音模式追踪、翻译步骤调用 |
| 翻译逻辑 | `lib.rs` 或新建 `translation.rs` | 翻译 LLM 调用、prompt 组装 |
| AI 调用 | `ai.rs` | 无变更，复用 `send_text_prompt` |
| 前端配置 Store | `stores/config.ts` | 新增 `translate_shortcut`、`translation` 字段 |
| 设置页面 | `routes/settings/+page.svelte` | 新增翻译快捷键配置 UI |
| 模型页面 | `routes/models/+page.svelte` | 新增目标语言选择 UI |
| Skills 页面 | `routes/skills/+page.svelte` | 新增 `builtin-translation` 展示 |

## 未来扩展

当前设计为「快捷键决定功能」模式。未来保留以下扩展能力：

- 通过某组快捷键调出功能选择界面，在录音前切换模式
- 同一套快捷键实现不同功能的切换
- 翻译结果回退到录音流程的起点
