# 设计文档：模型选择功能迁移

**日期**: 2026-03-31  
**状态**: 待审核

## 概述

将"模型选择"功能从技能设置页面迁移至模型管理页面，并新增润色服务配置和启用开关。

## 变更范围

### 前端变更

#### 1. 数据结构 (`src/lib/stores/config.ts`)

**TranscriptionConfig 扩展**：
```typescript
export interface TranscriptionConfig {
  provider_id: string;
  model: string;
  polish_enabled: boolean;      // 新增
  polish_provider_id: string;   // 新增
  polish_model: string;         // 新增
}
```

**SkillsConfig 简化**：
```typescript
export interface SkillsConfig {
  enabled: boolean;
  skills: Skill[];
  // provider_id: string;  // 移除
  // model: string;        // 移除
}
```

**默认值更新**：
```typescript
features: {
  transcription: {
    provider_id: 'vertex',
    model: 'gemini-2.0-flash',
    polish_enabled: true,         // 新增
    polish_provider_id: '',       // 新增
    polish_model: ''              // 新增
  },
  skills: {
    enabled: true,
    skills: []
  }
}
```

#### 2. 模型管理页面 (`src/routes/models/+page.svelte`)

**Features 区域改为"转写服务"**：
- 区域标题从 "Features" 改为 "转写服务"
- 布局从 `grid-cols-2` 改为单列纵向布局
- 包含三个部分（纵向排列）：
  1. **转写模型**：现有 Transcription 分组下拉框
  2. **启用润色**：开关控件
  3. **润色模型**：新增分组下拉框（与转写模型使用相同的 `buildTranscriptionGroups()`）

**新增函数**：
- `getPolishValue()`: 获取润色模型选择值
- `handlePolishChange(val: string)`: 处理润色模型变更
- `handlePolishEnabled(enabled: boolean)`: 处理润色开关

#### 3. 技能设置页面 (`src/routes/skills/+page.svelte`)

**移除内容**：
- "LLM 服务" 整个 section（Provider 和 Model 选择框）
- `buildProviderGroups()` 函数
- `buildModelGroups()` 函数
- `handleProviderChange()` 函数
- `handleModelChange()` 函数
- 相关 import（如不再需要）

**保留内容**：
- Skills 全局开关
- 技能列表（增删改查）

### 后端变更

#### 1. 配置结构 (`src-tauri/src/config.rs`)

**TranscriptionConfig 扩展**：
```rust
pub struct TranscriptionConfig {
    pub provider_id: String,
    pub model: String,
    pub polish_enabled: bool,          // 新增
    pub polish_provider_id: String,    // 新增
    pub polish_model: String,          // 新增
}
```

**SkillsConfig 简化**：
```rust
pub struct SkillsConfig {
    pub enabled: bool,
    pub skills: Vec<Skill>,
    // pub provider_id: String,  // 移除
    // pub model: String,        // 移除
}
```

#### 2. 主流程调用 (`src-tauri/src/lib.rs`)

**当前逻辑** (`lib.rs:137-148`)：
```rust
let transcription = &app_config.features.transcription;
let provider = app_config.ai.providers.iter()
    .find(|p| p.id == transcription.provider_id).cloned();
let model_name = transcription.model.clone();
let skills_config = app_config.features.skills.clone();
let skills_providers = app_config.ai.providers.clone();
```

**变更后**：
- 转写仍用 `transcription.provider_id` 和 `transcription.model`（不变）
- 润色改用 `transcription.polish_provider_id` 和 `transcription.polish_model`
- 将 `transcription` 传给 `process_with_skills()`，而非仅 `skills_config`
- 当 `transcription.polish_enabled` 为 false 时，跳过 Skills 处理

#### 3. Skills 处理逻辑 (`src-tauri/src/skills.rs`)

**函数签名变更**：
```rust
// 之前
pub async fn process_with_skills(
    logger: &Logger,
    skills_config: &SkillsConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &VertexClientCache,
)

// 之后
pub async fn process_with_skills(
    logger: &Logger,
    transcription_config: &TranscriptionConfig,  // 改为传入完整 TranscriptionConfig
    skills_config: &SkillsConfig,
    providers: &[ProviderConfig],
    transcription: &str,
    vertex_cache: &VertexClientCache,
)
```

**内部逻辑变更**：
- 使用 `transcription_config.polish_provider_id` 和 `transcription_config.polish_model` 查找 LLM
- 使用 `transcription_config.polish_enabled` 判断是否启用润色
- 移除对 `skills_config.provider_id` 和 `skills_config.model` 的引用

## 数据迁移

- 现有 `skills.provider_id` 和 `skills.model` 迁移到 `transcription.polish_provider_id` 和 `transcription.polish_model`
- 迁移逻辑在 `config.rs` 的加载函数中处理

## UI 布局

```
模型管理页面
├── 转写服务
│   ├── 转写模型 [分组下拉框]
│   ├── 启用润色 [开关]
│   └── 润色模型 [分组下拉框]
└── Providers [...]

技能设置页面
├── 全局
│   └── Skills 功能 [开关]
└── 技能列表 [...]
```

## 影响范围

- 前端：`config.ts`, `models/+page.svelte`, `skills/+page.svelte`
- 后端：`config.rs`, `skills.rs`, `lib.rs`
- 数据：配置文件格式变更，需要迁移逻辑

## 调用链影响分析

**当前流程**：
1. `lib.rs` 从 `transcription` 获取转写模型 → 调用 AI 转写
2. `lib.rs` 从 `skills_config` 获取润色模型 → 调用 `process_with_skills()`
3. `skills.rs` 使用 `skills_config.provider_id/model` 查找 LLM

**变更后流程**：
1. `lib.rs` 从 `transcription` 获取转写模型 → 调用 AI 转写（不变）
2. `lib.rs` 从 `transcription` 获取润色模型 → 调用 `process_with_skills()`
3. `skills.rs` 使用 `transcription_config.polish_provider_id/model` 查找 LLM
4. `skills.rs` 使用 `transcription_config.polish_enabled` 判断是否跳过润色
