# 按钮渐变立体风格升级设计

## 背景

当前项目的按钮在 Light/Dark 模式下均为平铺的单色效果：
- Light 模式：主要按钮用 `bg-foreground`（近黑背景 + 白字）
- Dark 模式：主要按钮用 `bg-foreground`（浅灰背景 + 深字）

这种平铺风格与项目整体的微绿色调（oklch hue=160）设计语言不协调，缺乏视觉层次和品牌辨识度。

## 目标

将所有按钮从平铺单色升级为绿色渐变 + 微阴影的立体风格，统一使用项目已有的绿色系色板，提升按钮的视觉存在感和交互暗示。

## 设计方案

### 色彩体系

项目使用 oklch 色彩空间，整体偏青绿色调（hue=160）。升级后按钮将复用现有 `--accent` / `--accent-foreground` 变量所在的色系，但通过 CSS 新增渐变和阴影变量来实现立体效果。

#### 新增 CSS 变量

**Light 模式 (`:root`)：**

| 变量 | 值 | 用途 |
|------|-----|------|
| `--btn-primary-from` | 绿色高光 | 主要按钮渐变上沿 |
| `--btn-primary-to` | 绿色深色 | 主要按钮渐变下沿 |
| `--btn-primary-shadow` | 绿色阴影 | 主要按钮外阴影 |
| `--btn-primary-inset` | 白色内发光 | 主要按钮 inset 高光 |
| `--btn-secondary-from` | 浅色高光 | 次要按钮渐变上沿 |
| `--btn-secondary-to` | 浅色底色 | 次要按钮渐变下沿 |
| `--btn-secondary-border` | 绿色半透明边框 | 次要按钮边框色 |
| `--btn-secondary-shadow` | 淡阴影 | 次要按钮外阴影 |
| `--btn-destructive-from` | 红色高光 | 危险按钮渐变上沿 |
| `--btn-destructive-to` | 红色深色 | 危险按钮渐变下沿 |
| `--btn-destructive-shadow` | 红色阴影 | 危险按钮外阴影 |
| `--btn-destructive-inset` | 白色内发光 | 危险按钮 inset 高光 |

**Dark 模式 (`.dark`)：** 对应的暗色调变体，额外增加微弱的彩色光晕（glow）效果。

### 按钮类型规范

#### 1. 主要操作按钮（保存、添加、重置）

- **Light**：深绿色渐变（从亮绿到深绿）+ 绿色外阴影 + 顶部 inset 白色高光
- **Dark**：亮绿色渐变 + 深色外阴影 + 绿色微弱光晕 + 顶部 inset 半透明白色高光
- 文字：白色
- 无边框

#### 2. 次要操作按钮（取消）

- **Light**：从白到微灰的渐变 + 绿色半透明边框 + 绿色文字 + 淡阴影
- **Dark**：从深灰到更深灰的渐变 + 绿色半透明边框 + 绿色文字 + 深色阴影
- 有 1px 边框

#### 3. 危险操作按钮（删除）

- **Light**：红色渐变（从亮红到深红）+ 红色外阴影 + 顶部 inset 白色高光
- **Dark**：红色渐变 + 深色外阴影 + 红色微弱光晕 + 顶部 inset 高光
- 文字：白色
- 无边框

#### 4. 开关 Toggle

- **激活态**：绿色渐变轨道 + 白色渐变圆形滑块 + 阴影
- **未激活态**：灰色渐变轨道 + 白色/灰色渐变滑块 + 阴影
- 滑块带有微阴影增加立体感

#### 5. 主题切换按钮

- **选中态**：绿色渐变背景 + 白色文字 + 阴影（与主要按钮同风格）
- **未选中态**：淡色渐变背景 + 灰色文字 + 边框 + 微阴影（与次要按钮同风格）

#### 6. 导航按钮（侧边栏）

- **激活态**：绿色渐变背景（从左到右，左浓右淡）+ 左侧 3px 绿色边框指示器 + 绿色文字 + 绿色微阴影
- **未激活态**：透明背景 + 透明边框 + 灰色文字（保持不变）

#### 7. 图标按钮（编辑/删除）

- **编辑图标**：淡色渐变背景 + 淡边框 + 微阴影
- **删除图标**：淡红色渐变背景 + 红色半透明边框 + 红色微阴影

### 影响范围

#### 需要修改的文件

| 文件 | 变更内容 |
|------|----------|
| `src/app.css` | 新增渐变/阴影 CSS 变量（Light + Dark）+ `@theme inline` 映射 |
| `src/routes/settings/+page.svelte` | 主题切换按钮样式 |
| `src/routes/skills/+page.svelte` | Toggle 开关、添加/编辑/删除/取消/保存按钮样式 |
| `src/routes/models/+page.svelte` | Toggle 开关、添加/重置/删除/取消按钮样式 |
| `src/routes/logs/+page.svelte` | Tab 切换、模块过滤、拷贝按钮样式 + 修复 `bg-primary` 未定义引用 |
| `src/routes/+layout.svelte` | 侧边栏导航按钮样式 |
| `src/routes/recording/+page.svelte` | 取消按钮样式 |

#### 不修改的文件

- `src/lib/components/ui/dialog/index.svelte` — Dialog 关闭按钮（X 图标）保持原样
- `src/lib/components/ui/select/index.svelte` — Select 触发器保持原样
- `src/lib/components/ui/shortcut-recorder/index.svelte` — 快捷键按钮保持原样
- `src/lib/components/ui/password-input/index.svelte` — 密码切换保持原样
- `src/lib/components/ui/tag-input/index.svelte` — 标签按钮保持原样

### 实现策略

1. 在 `src/app.css` 中新增所有渐变/阴影 CSS 变量，分别定义 Light 和 Dark 模式值
2. 将新增变量映射到 Tailwind v4 的 `@theme inline` 中
3. 逐一更新各页面中的按钮 Tailwind class，将 `bg-foreground` 替换为 `bg-gradient-to-b from-btn-primary-from to-btn-primary-to` + 对应 shadow class
4. 由于 Tailwind v4 的 `shadow-*` 是固定值，自定义阴影需要通过 `shadow-[var(--btn-primary-shadow)]` 或在 `@theme inline` 中定义自定义 shadow token 来实现
5. 对于渐变，优先使用 Tailwind 的 `bg-gradient-to-b from-xxx to-xxx` 工具类

### Bug 修复

在 `src/routes/logs/+page.svelte` 中发现使用了未定义的 `bg-primary text-primary-foreground`，本次升级中一并修复为使用新的按钮变量体系。

### 不做的事

- 不修改 UI 原语组件（Dialog、Select 等）的内部按钮
- 不修改 recording 页面的录音指示器独立样式系统
- 不引入独立的 Button 组件（保持当前内联 Tailwind class 的模式）
- 不修改 `--accent` / `--accent-foreground` 等已有变量的值
