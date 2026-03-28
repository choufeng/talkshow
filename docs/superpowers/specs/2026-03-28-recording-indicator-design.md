# 录音悬浮指示器设计

## 概述

在 Tauri 桌面应用中实现一个录音状态的悬浮指示器，作为独立的小窗口悬浮在所有窗口上方，显示录音状态并提供取消/完成操作。

## 技术栈

- Tauri v2（多窗口 + always_on_top）
- SvelteKit + Svelte 5
- 纯 CSS 动画

## 窗口规格

| 属性 | 值 |
|------|-----|
| 尺寸 | 140 × 44 px |
| transparent | true |
| decorations | false |
| always_on_top | true |
| resizable | false |
| skip_taskbar | true |
| 圆角 | 10px（CSS） |

## 布局

```
┌──────────────────────────────┐
│  ✕  │ 🔴 录音 00:23 │  ✓   │
│ 28px│    中间区域     │ 28px │
└──────────────────────────────┘
```

横排三栏布局：

- **左栏（28px）**：✕ 取消按钮，红色/灰色图标，hover 时高亮
- **中栏**：红色脉冲圆点 + "录音" 文字 + MM:SS 格式计时器
- **右栏（28px）**：✓ 完成按钮，绿色图标，hover 时高亮

## 外观

- 半透明深色背景（`rgba(30, 30, 30, 0.9)`）
- 圆角 `border-radius: 10px`
- 无系统标题栏，无边框

## 脉冲动画

中间红色圆点使用 CSS `@keyframes`：

- 从 `opacity: 1; transform: scale(1)` 到 `opacity: 0.5; transform: scale(1.4)`
- 循环周期：1.5s
- `infinite` 循环

## 计时器

- 使用 `setInterval` 每秒更新
- 格式：`MM:SS`
- Svelte 5 `$state` 响应式变量驱动
- 窗口打开时开始计时，取消/完成时停止

## 交互

- **取消（✕）**：关闭窗口，发出取消录音事件（Tauri event）
- **完成（✓）**：关闭窗口，发出录音完成事件（Tauri event）
- **拖拽移动**：通过 Tauri `startDragging` API，拖拽中间区域移动窗口位置

## 窗口间通信

主窗口与录音指示器窗口通过 Tauri 的 `emit` / `listen` 事件系统通信：

- `recording:start` — 开始录音，打开指示器窗口
- `recording:cancel` — 取消录音
- `recording:complete` — 录音完成

## 文件结构

```
src/
  routes/
    recording/
      +page.svelte          # 录音指示器页面（独立窗口加载）
src-tauri/
  src/
    lib.rs                   # 添加录音窗口创建逻辑
  tauri.conf.json           # 新增录音窗口配置
```

## 实现要点

1. Tauri 配置中新增一个 label 为 `recording-indicator` 的窗口条目
2. 录音指示器页面使用独立路由 `/recording`
3. 主窗口通过 Tauri `WebviewWindow` API 创建/控制录音窗口
4. 窗口背景透明需要同时设置 Tauri 的 `transparent: true` 和 CSS 的 `background: transparent`（`<html>` 和 `<body>`）
