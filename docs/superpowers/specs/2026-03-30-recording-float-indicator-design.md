# 录音悬浮状态浮窗设计

**日期:** 2026-03-30
**状态:** 已批准

## 背景与动机

当前 TalkShow 的录音反馈仅通过系统托盘图标变化、tooltip 文字和提示音实现。从录音结束到 AI 转写完成再到剪贴板粘贴的整个过程中，用户无法获得任何视觉反馈。这导致用户不确定应用是否仍在工作，体验割裂。

需要一个始终悬浮在屏幕最上层的小型浮窗，覆盖录音和 AI 处理两个阶段的状态表达。

## 需求摘要

| 需求 | 决策 |
|------|------|
| 录音状态 | 红色脉冲动画 + 计时器 |
| 处理状态 | 靛蓝色旋转动画 + "处理中"文字 |
| 浮窗特性 | alwaysOnTop + skipTaskbar，不受窗口平铺影响 |
| 尺寸 | 160 x 44px 药丸形 |
| 位置 | 屏幕右上角固定位置 |
| 交互 | 取消按钮（录音中）、中止按钮（处理中） |
| 完成反馈 | 静默消失，无额外提示 |
| 错误反馈 | 浮窗消失 + 系统通知（保持现有行为） |
| 并发处理 | 新录音中断当前处理，丢弃结果 |
| 目标平台 | macOS / Windows / Linux |

## 技术方案

**选择方案：Tauri 动态子窗口（方案 A）**

通过 Rust 后端使用 `WebviewWindowBuilder` 动态创建独立的无边框 WebviewWindow，Svelte 渲染浮窗 UI，Tauri 事件系统驱动状态切换。

选择理由：
- 与项目现有 Svelte 技术栈一致，开发效率高
- `capabilities/default.json` 已预留 `recording-indicator` 窗口标识
- 与现有 `emit`/`listen` 事件系统无缝集成
- Tauri v2 的窗口属性（alwaysOnTop、skipTaskbar、transparent）开箱支持

## 架构设计

### 窗口生命周期管理

**创建者：Rust 后端**（非前端）。原因：
1. 录音流程完全由 Rust 侧驱动（快捷键 → 录音 → AI → 剪贴板）
2. 主窗口可能处于隐藏状态，不适合依赖前端创建子窗口
3. 后端拥有完整的录音状态，是窗口生命周期的天然管理者

### 窗口属性

```
label: "recording-indicator"
width: 160
height: 44
transparent: true
decorations: false
resizable: false
alwaysOnTop: true
skipTaskbar: true
visible: false (创建后由代码控制显示)
```

### 状态机

```
IDLE ──(快捷键:开始录音)──→ RECORDING ──(快捷键:结束录音)──→ PROCESSING ──(完成)──→ IDLE
                                ↑                               │
                                └────(快捷键:新录音,中断)────────┘
                                                                 ├──(错误)──→ IDLE + 系统通知
RECORDING ──(取消按钮/ESC)──→ IDLE
PROCESSING ──(中止按钮)──→ IDLE
```

### 事件协议

**Rust 后端 → 浮窗窗口**（通过 `emit_to` 定向发送到 `recording-indicator` 窗口）：

| 事件 | payload | 时机 | 浮窗行为 |
|------|---------|------|----------|
| `indicator:recording` | `{}` | 录音开始 | 显示录音状态（红色脉冲 + 计时） |
| `indicator:processing` | `{}` | 录音结束，AI 开始处理 | 切换到处理状态（旋转动画） |
| `indicator:done` | `{}` | AI 完成 + 剪贴板粘贴成功 | 窗口淡出后销毁 |
| `indicator:error` | `{ message }` | 处理失败 | 窗口销毁，系统通知由后端发送 |

**浮窗窗口 → Rust 后端**（通过 `emit` 发送到全局）：

| 事件 | payload | 时机 | 后端行为 |
|------|---------|------|----------|
| `indicator:cancel` | `{ phase: "recording" \| "processing" }` | 用户点击取消/中止按钮 | 取消录音或中断处理任务 |

### 位置计算

浮窗固定在屏幕右上角，不同平台需要考虑顶部栏高度：

- **macOS:** 菜单栏高度约 25px + 8px 间距 = top 33px
- **Windows:** 无顶部栏，top 12px
- **Linux (GNOME):** 顶部面板约 28px + 8px = top 36px
- **Linux (KDE):** 可能无面板或不同高度，top 12px 作为默认

通过 Tauri 的 `availableMonitors()` + `currentMonitor()` API 获取主显示器尺寸，计算 `x = screen_width - window_width - 12`, `y = platform_offset`。创建窗口时使用 `setPosition` 设置。

## UI 设计

### 录音状态（RECORDING）

```
┌──────────────────────────────────────┐
│  ●  01:23                    ✕      │
└──────────────────────────────────────┘
```

- 背景：`rgba(30, 30, 30, 0.92)` + `backdrop-filter: blur(20px)`
- 圆角：22px（药丸形）
- 红色脉冲圆点：`#ef4444`，1.5s ease-in-out 无限脉冲，带 0.5px 发光边框
- 计时器：等宽字体（SF Mono / Menlo），`font-variant-numeric: tabular-nums`，白色
- 取消按钮（✕）：hover 时 `rgba(255, 95, 87, 0.2)` 底色

### 处理状态（PROCESSING）

```
┌──────────────────────────────────────┐
│  ⟳  处理中                   ✕      │
└──────────────────────────────────────┘
```

- 背景：与录音状态相同
- 旋转动画：靛蓝色 `#6366f1`，0.8s 线性旋转
- 文字：`#a5b4fc`（靛蓝浅色），"处理中"
- 中止按钮（✕）：hover 时同样变红底

### 状态过渡

录音 → 处理：300ms CSS transition，红色脉冲淡出，旋转器淡入，计时器文字变为"处理中"。

完成/消失：200ms opacity 淡出后窗口销毁。

## Rust 后端集成点

### 录音开始处（lib.rs 快捷键 handler）

在 `recorder.start()` 成功后：
1. 创建 `recording-indicator` WebviewWindow
2. emit `indicator:recording` 到该窗口

### 录音结束处（stop_recording 函数）

在 `recording:complete` 分支中：
1. emit `indicator:processing` 到浮窗窗口
2. 在 AI 处理 spawn 的异步任务中：
   - 成功：emit `indicator:done`，销毁窗口
   - 失败：emit `indicator:error`，销毁窗口 + 系统通知

### 取消/中止处理

浮窗前端 emit `indicator:cancel` 后，Rust 后端需要：
- 录音中：执行取消录音逻辑（等同 ESC 键）
- 处理中：设置取消标志，AI spawn 的异步任务检查该标志并提前退出

### 中断处理

当用户在 PROCESSING 状态按下录音快捷键：
1. 设置取消标志，当前 AI spawn 的 tokio task 检测到标志后提前退出并丢弃结果
2. 浮窗窗口不销毁，直接 emit `indicator:recording` 切回录音状态
3. 开始新的录音

## 前端实现

### Svelte 路由

创建 `src/routes/recording/+page.svelte` 作为浮窗页面。

### 浮窗组件结构

```
src/routes/recording/+page.svelte
├── 监听 indicator:recording/processing/done/error 事件
├── 根据 current state 渲染对应 UI
├── 取消/中止按钮 → emit indicator:cancel
└── 录音状态下的计时器（前端自管理，事件触发时 start/clear interval）
```

### 暗色模式

浮窗本身使用固定的半透明深色背景，不受应用主题影响。

## 跨平台注意事项

1. **Wayland (Linux):** `alwaysOnTop` 不一定被所有 compositor 支持。GNOME/Mutter 对 overlay 窗口有限制。降级方案：浮窗仍可显示，但可能被其他窗口覆盖。
2. **透明窗口:** macOS 和 Windows 原生支持。Linux X11 需要 compositor 运行。Wayland 原生支持。
3. **窗口不接收平铺:** `skipTaskbar: true` + 无装饰窗口通常不会被 tiling WM 纳入布局。X11 下可额外设置窗口 type 为 dock/utility。

## 不在范围内

- 浮窗拖拽（固定位置）
- 处理完成后的结果预览
- 浮窗内错误详情展示
- 多显示器位置记忆
