# T4: Skills UI 页面 — Skill 管理界面

## 所属项目
[Skills 文本处理系统](../../specs/2026-03-30-skills-system-design.md)

## 依赖
- T1: 配置层扩展（需要前端 SkillsConfig 类型和 Tauri command）

## 目标
创建 `/skills` 页面，提供 Skill 的 CRUD 操作界面，包括列表展示、编辑对话框、添加/删除功能。

## 任务详情

### 1. 创建 Skills 页面路由

新建 `src/routes/skills/+page.svelte`。

### 2. 页面布局

```
┌─────────────────────────────────────────────────┐
│  技能设置                                         │
│                                                   │
│  [全局开关 ON/OFF]                                │
│                                                   │
│  ── LLM 服务 ─────────────────────────────────   │
│  Provider: [选择 Provider ▾]   Model: [选择 ▾]    │
│                                                   │
│  ── 技能列表 ─────────────────────────────────   │
│  [ + 添加自定义 Skill ]                           │
│                                                   │
│  ┌─ Skill 卡片 ──────────────────────────────┐   │
│  │ [Switch] 名称               [预置/自定义]  │   │
│  │ 描述文字                     [编辑] [删除] │   │
│  └───────────────────────────────────────────┘   │
│  ...（更多 Skill 卡片）                            │
│                                                   │
└─────────────────────────────────────────────────┘
```

### 3. 页面功能

#### 全局开关
- 复用现有 Switch 组件（或 bits-ui Toggle）
- 绑定到 `skills.enabled`
- 切换后调用 `save_skills_config` 持久化

#### LLM 服务选择
- Provider 下拉选择：从 `config.ai.providers` 中列出所有 provider（排除 sensevoice 类型，因为其不支持文本对话）
- Model 下拉选择：根据选中的 Provider 显示其 models 列表
- 选择后调用 `save_skills_config` 持久化

#### Skill 列表
- 遍历 `skills.skills` 数组，每个 Skill 渲染为一个卡片
- 每个卡片包含：
  - **Switch 开关**: 绑定 `skill.enabled`，切换后保存
  - **名称**: `skill.name`
  - **描述**: `skill.description`
  - **标签**: `skill.builtin ? "预置" : "自定义"`
  - **编辑按钮**: 打开编辑对话框
  - **删除按钮**: 仅在 `!skill.builtin` 时显示

#### 添加 Skill
- 点击"添加自定义 Skill"按钮打开对话框
- 创建新 Skill 对象（生成 UUID，`builtin: false`，`enabled: true`）
- 调用 `add_skill` 保存

#### 编辑对话框
- 复用现有 `dialog` 组件或新建
- 包含字段：
  - 名称 (text input, required)
  - 描述 (text input, required)
  - Prompt 内容 (textarea, required, 多行)
- 保存时调用 `update_skill`
- 预置 Skill 可编辑内容但不可删除

#### 删除确认
- 复用现有确认对话框
- 仅允许删除自定义 Skill（`!skill.builtin`）
- 确认后调用 `delete_skill`

### 4. 样式

- 复用项目现有的 Tailwind CSS 4 + CSS 变量主题
- Skill 卡片样式参考现有的 Provider 卡片设计（参见 models/+page.svelte）
- 支持亮暗主题

## 验收标准

- [ ] `/skills` 路由可正常访问
- [ ] 全局开关切换正常，状态持久化
- [ ] Provider 和 Model 下拉选择正常工作
- [ ] Skill 列表正确展示所有 Skill
- [ ] 预置 Skill 显示"预置"标签，不可删除
- [ ] 自定义 Skill 显示"自定义"标签，可编辑和删除
- [ ] 编辑对话框可正常编辑名称、描述、Prompt
- [ ] 添加自定义 Skill 正常工作
- [ ] 删除自定义 Skill 有确认提示
- [ ] 亮暗主题适配正常
