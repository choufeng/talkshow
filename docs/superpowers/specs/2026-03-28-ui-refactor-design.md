# TalkShow UI 重构设计文档

## 概述

引入 bits-ui + TailwindCSS 对 TalkShow 桌面应用进行全量 UI 重构，建立完整的 Design Token 体系，支持亮色/暗色双模式，采用简约现代的视觉风格。

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| TailwindCSS | v4 | 样式框架 |
| @tailwindcss/vite | v4 | Vite 插件集成 |
| bits-ui | v2 | Headless 组件库 |
| tailwindcss-animate | - | 动画工具类 |
| lucide-svelte | 已有 | 图标库 |

## 项目结构

```
src/
├── app.css                    # Design Tokens + Tailwind 导入
├── app.html
├── lib/
│   ├── components/
│   │   └── ui/                # bits-ui 封装层
│   │       ├── select/        # GroupedSelect → bits-ui Select
│   │       ├── tag-input/     # 自定义组件，Tailwind 样式
│   │       ├── password-input/# 自定义组件，Tailwind 样式
│   │       └── shortcut-recorder/ # 自定义组件，Tailwind 样式
│   └── stores/
│       ├── config.ts          # 已有
│       └── theme.ts           # 新增：主题状态管理
└── routes/
    ├── +layout.svelte         # Tailwind 重构
    ├── +page.svelte           # Tailwind 重构
    ├── models/+page.svelte    # Tailwind 重构
    └── settings/+page.svelte  # Tailwind 重构 + 外观设置入口
```

## Design Token 体系

在 `app.css` 中定义 CSS 变量，分为亮色和暗色两套。

### 颜色 Token

| Token | 亮色值 | 暗色值 | 用途 |
|-------|--------|--------|------|
| `--background` | `hsl(0 0% 100%)` | `hsl(0 0% 5%)` | 页面/内容区背景 |
| `--background-alt` | `hsl(0 0% 98%)` | `hsl(0 0% 8%)` | 侧边栏背景 |
| `--foreground` | `hsl(0 0% 9%)` | `hsl(0 0% 95%)` | 主文字 |
| `--foreground-alt` | `hsl(0 0% 32%)` | `hsl(0 0% 70%)` | 次要文字 |
| `--muted` | `hsl(240 5% 96%)` | `hsl(240 4% 16%)` | 弱化背景 |
| `--muted-foreground` | `hsla(0 0% 9% / 0.4)` | `hsla(0 0% 100% / 0.4)` | 弱化文字 |
| `--border` | `hsla(240 6% 10% / 0.1)` | `hsla(0 0% 96% / 0.1)` | 卡片边框 |
| `--border-input` | `hsla(240 6% 10% / 0.17)` | `hsla(0 0% 96% / 0.17)` | 输入框边框 |
| `--accent` | `hsl(204 94% 94%)` | `hsl(204 90% 90%)` | 强调色背景 |
| `--accent-foreground` | `hsl(204 80% 16%)` | `hsl(204 94% 94%)` | 强调色文字 |
| `--destructive` | `hsl(347 77% 50%)` | `hsl(350 89% 60%)` | 破坏性操作 |

### 阴影 Token

| Token | 用途 |
|-------|------|
| `--shadow-card` | 卡片阴影 |
| `--shadow-popover` | 下拉浮层阴影 |
| `--shadow-mini` | 轻量阴影（按钮等） |

### 字体

- 正文：系统字体栈（`-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`）
- 等宽：系统 monospace 字体栈
- 不额外加载网络字体，保持桌面应用的轻量与快速

### 圆角

| 元素 | 圆角 | Tailwind 类 |
|------|------|-------------|
| 卡片/面板 | 8px | `rounded-lg` |
| 输入框/按钮 | 6px | `rounded-md` |
| 下拉浮层 | 8px | `rounded-lg` |
| 标签(Tag) | 4px | `rounded` |

### 间距

基于 Tailwind 默认（4px 基数）：
- 页面内容区 padding：`24px` (`p-6`)
- 卡片内 padding：`16px` (`p-4`)
- 表单元素间距：`12px` (`gap-3`)

## 组件映射

### GroupedSelect → bits-ui Select

用 bits-ui Select 原语完全重写：

- `Select.Root` → 状态管理与受控值
- `Select.Trigger` → 触发按钮
- `Select.Portal` + `Select.Content` → Floating UI 定位的浮层
- `Select.Group` + `Select.GroupHeading` → 分组展示
- `Select.Item` → 选项，通过 `{#snippet children({ selected })}` 渲染选中标记

新增能力（相比手写组件）：
- 键盘 typeahead 搜索
- `Select.ScrollUpButton` / `Select.ScrollDownButton` 长列表滚动
- 进出动画（fade + zoom）

### PasswordInput（保留自定义）

bits-ui 无对应组件，保留业务逻辑，样式改用 Tailwind：
- 👁 emoji 替换为 lucide-svelte 的 `Eye` / `EyeOff` 图标
- 蒙版/明文切换逻辑不变
- 输入框统一样式：`bg-muted border-border-input rounded-md`

### TagInput（保留自定义）

保留业务逻辑，样式改用 Tailwind：
- 标签用 accent 色系：`bg-accent text-accent-foreground rounded`
- 输入框统一样式
- 添加按钮：`text-accent-foreground` + hover 下划线

### ShortcutRecorder（保留自定义）

保留业务逻辑，样式改用 Tailwind：
- 录制态：`bg-accent text-accent-foreground`
- 快捷键显示区：`bg-muted font-mono rounded-md`
- 错误信息：`text-destructive`

### 布局（+layout.svelte）

保持 flex 布局结构，全部改为 Tailwind 类：
- 侧边栏：`w-40 bg-background-alt border-r border-border`
- Logo：`px-5 py-4 font-semibold text-sm border-b border-border`
- 菜单项：`flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground`
- 激活态：`bg-muted border-l-[3px] border-l-accent-foreground`
- 内容区：`flex-1 p-6 overflow-y-auto bg-background`

## 主题切换

### 机制

- `<html>` 标签上切换 `dark` class
- 新增 `theme` store（`src/lib/stores/theme.ts`），支持 `light` / `dark` / `system`
- 通过 `window.matchMedia('(prefers-color-scheme: dark)')` 监听系统偏好变化
- 主题偏好持久化到 localStorage

### UI 入口

在 settings 页新增"外观"设置区域：
- 三个选项：浅色 / 深色 / 跟随系统
- 使用按钮组切换

## 动画

克制使用，保持桌面工具的专业感：
- 下拉浮层：`fade-in/fade-out` + `zoom-in-95/zoom-out-95`（150-200ms）
- 侧边栏激活态：`border-left` 无动画（即时切换）
- 不在其他元素上添加动画

## 不在本次范围内

- 不引入新的页面或功能
- 不修改 Rust 后端逻辑
- 不修改数据 store 的接口（`config.ts` 的 API 保持不变）
- 不做国际化
