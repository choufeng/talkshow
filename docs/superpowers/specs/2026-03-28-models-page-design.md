# 模型页面 UI 设计

日期：2026-03-28

## 概述

在 TalkShow 应用中新增"模型"页面（`/models`），用于配置 AI Provider 和 Features 功能选择。该页面独立于现有的"设置"页面，专门承载 AI 相关的配置项。

## 背景

上一个提交（`a53938a`）已在 `AppConfig` 中添加了 `ai` 和 `features` 配置结构，但尚未在 UI 中暴露。当前应用只有"首页"和"设置"两个页面，设置页仅包含快捷键配置。

### 数据模型

采用泛化 Provider 结构，通过 `type` 字段区分连接/认证模式（如 Vertex AI 用 Google Cloud 认证，OpenAI 兼容的用 API Key），新增 Provider 无需修改类型定义。

```typescript
interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
  ai: AiConfig;
  features: FeaturesConfig;
}

interface ProviderConfig {
  id: string;            // 唯一标识 "vertex", "dashscope", "deepseek"...
  type: "vertex" | "openai-compatible";  // 连接/认证模式
  name: string;          // 显示名 "VTX", "阿里云"...
  endpoint: string;
  api_key?: string;      // vertex 不需要，openai-compatible 需要
  models: string[];
}

interface AiConfig {
  providers: ProviderConfig[];
}

interface TranscriptionConfig {
  provider_id: string;   // 引用 providers[].id
  model: string;
}

interface FeaturesConfig {
  transcription: TranscriptionConfig;
}
```

**相比旧结构的变更：**
- `AiConfig` 从固定字段（`vertex`, `dashscope`）改为 `ProviderConfig[]` 数组
- 新增 `ProviderConfig.type` 区分认证模式，控制 UI 是否显示 API Key 字段
- `TranscriptionConfig.provider` 改为 `provider_id`，通过 id 引用 Provider
- Rust 端 `config.rs` 需同步重构：移除 `VertexConfig`/`DashScopeConfig` 独立结构体，统一为 `ProviderConfig`

## 设计决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 菜单项名称 | 🤖 模型 | 简洁，直接表达页面用途 |
| 菜单位置 | 首页和设置之间 | 将 AI 配置作为核心功能突出 |
| Features 布局 | 3 列网格卡片 | 每个 Feature 内容少，3 列适配 3-4 个 Feature |
| Feature 下拉框 | 单级分组下拉（按 Provider 分组显示 Model） | 一步完成选择，数据模型直接映射 |
| Feature 占位 | 不显示占位卡片 | Feature 有时自然出现，无需预留 |
| Provider 布局 | 2 列网格卡片，内联编辑 | 所有字段直接可见，操作路径短 |
| Provider 扩展 | 动态可扩展，底部"+ 添加 Provider" | 新增 Provider 只需往数组加数据，无需改类型 |
| Provider 结构 | 泛化 ProviderConfig 数组 | type 字段区分认证模式，可扩展 |
| Provider 认证区分 | `type: "vertex" | "openai-compatible"` | vertex 无 API Key，openai-compatible 有 |
| Models 管理 | Tag 标签式（✕ 删除 + 底部"+ 添加模型"） | 紧凑、直观、适合 string[] 结构 |
| API Key 显示 | 密码遮罩 + 👁 切换明文 | 安全考虑，桌面应用本地存储 |
| 样式 | 沿用现有灰白主题 | 保持一致性 |

## 页面结构

### 导航变更

侧边栏新增一项，顺序为：首页 → 🤖 模型 → ⚙️ 设置

### 页面布局（从上到下）

**1. 页面标题**："模型"

**2. Features 区域**
- 区域标题：`FEATURES`（大写、小号、灰色）
- 3 列 CSS Grid 布局
- 每个 Feature 一张卡片，包含：
  - Feature 名称（加粗）
  - Feature 描述（灰色小字）
  - 分组下拉选择器：按 Provider 分组显示所有可用 Model
  - 选中项格式：`{Provider 名称} — {model 名称}`
  - 选择后同时设置 `provider` 和 `model` 字段

**3. Providers 区域**
- 区域标题：`PROVIDERS`（大写、小号、灰色）
- 2 列 CSS Grid 布局
- 每个 Provider 一张卡片，内联编辑，字段包括：
  - **头部**：Provider 名称（加粗）+ 副标题 + ✕ 删除按钮
  - **API Key**（仅 DashScope 等需要 key 的 Provider）：遮罩输入框 + 👁 切换按钮
  - **Endpoint**：可编辑输入框，预填默认值
  - **Models**：Tag 标签列表，每个 tag 带 ✕ 删除，底部"+ 添加模型"链接
- 底部：虚线边框"+ 添加 Provider"按钮

### 交互细节

**分组下拉框行为：**
- 点击 Features 卡片中的选择器，展开下拉列表
- 列表按 Provider 分组，组头显示 Provider 名称
- 当前选中项高亮标记 ✓
- 选择后自动收起

**Tag 标签操作：**
- 点击 tag 上的 ✕ 移除该 model
- 点击"+ 添加模型"出现输入框，回车确认添加

**API Key 切换：**
- 默认显示 `sk-••••••••` 遮罩
- 点击 👁 图标切换为明文
- 再次点击恢复遮罩

**Provider 动态扩展：**
- 点击"+ 添加 Provider"弹出配置表单或对话框
- 具体交互待定（当前只需预留 UI 位置）

## 组件规划

需要新建的基础组件（项目目前无组件库，手写 CSS）：

| 组件 | 用途 |
|------|------|
| `GroupedSelect.svelte` | 分组下拉选择器（Features 用） |
| `TagInput.svelte` | Tag 标签输入（Provider 的 Models 管理） |
| `PasswordInput.svelte` | 带遮罩切换的密码输入框 |

需要新建的页面：

| 文件 | 用途 |
|------|------|
| `src/routes/models/+page.svelte` | 模型页面 |

需要修改的文件：

| 文件 | 修改内容 |
|------|----------|
| `src/routes/+layout.svelte` | 侧边栏新增"🤖 模型"菜单项 |
| `src/lib/stores/config.ts` | 重构 AiConfig 为 ProviderConfig[] 数组 |
| `src-tauri/src/config.rs` | 同步重构 Rust 端配置结构体 |

## 样式规范

沿用现有主题：
- 背景：`#f5f5f5`
- 边框：`#e0e0e0`，圆角 `8px`
- 强调色：`#396cd8`（蓝色，用于选中态、链接、tag 背景）
- 字号：标题 20px，区域标题 11px uppercase，正文 13-14px，辅助 11px
- 卡片内间距：14px padding
- 网格间距：12px gap
