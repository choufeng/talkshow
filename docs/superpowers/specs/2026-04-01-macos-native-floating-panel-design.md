# macOS 原生浮窗实现

**日期:** 2026-04-01
**状态:** 已批准

## 背景

当前录音浮窗使用 Tauri 的 `WebviewWindowBuilder` 创建，在 macOS 上会抢占窗口焦点。用户期望浮窗弹出时不影响其他窗口的焦点状态。

问题根源：Tauri 的 WebviewWindow 是标准 NSWindow，在 macOS 上默认会执行焦点获取行为。

解决方案：使用 macOS 的 `nonactivatingPanel` 窗口样式，使浮窗在不激活的情况下显示。

## 需求

| 需求 | 描述 |
|------|------|
| 不抢占焦点 | 浮窗弹出时，下层窗口保持原有焦点 |
| 按钮可点击 | 用户可以点击浮窗上的取消/中止按钮 |
| 复用现有 UI | 浮窗 UI 保持现有 Svelte 实现 |
| 位置固定 | 屏幕右上角 |
| 生命周期不变 | 录音开始显示，处理完成/出错消失 |

## 技术方案

### 方案 A：运行时修改 NSWindow 样式（采用）

核心思路：
1. 使用现有 `WebviewWindowBuilder` 创建浮窗
2. 浮窗创建后，通过 `objc2` 获取底层 `NSWindow` 指针
3. 调用 `[window setStyleMask:NSWindowStyleMask::nonactivatingPanel]` 修改窗口样式

### 实现细节

**依赖：**
- `objc2` (已有)
- `objc2-app-kit` (已有)
- Tauri 的 macOS 私有 API 或通过窗口句柄获取底层窗口

**关键代码流程：**

```rust
// 1. 创建浮窗（保持现有代码）
let window = WebviewWindowBuilder::new(...)
    .build()?;

// 2. 获取 NSWindow 指针
// 通过 WebviewWindow 的 raw_handle() 或类似方法获取

// 3. 修改窗口样式为 nonactivatingPanel
unsafe {
    let ns_window = /* 获取 NSWindow */;
    let style = ns_window.styleMask();
    ns_window.setStyleMask(style | NSWindowStyleMask::nonactivatingPanel);
}
```

**需要调研的事项：**
1. 如何从 Tauri WebviewWindow 获取底层 NSWindow 指针
2. `nonactivatingPanel` 是否与现有窗口属性（transparent、decorations）兼容
3. 是否需要在窗口显示前修改样式

## 架构

### 模块结构

```
src-tauri/
├── lib.rs                    # 修改：浮窗创建后调用样式修改
├── macos/
│   └── floating_panel.rs     # 新增：封装 NSPanel 样式修改逻辑
```

### 职责划分

| 模块 | 职责 |
|------|------|
| `lib.rs` | 浮窗创建，调用样式修改函数 |
| `macos/floating_panel.rs` | NSWindow 样式修改逻辑 |

## 行为

| 场景 | 行为 |
|------|------|
| 浮窗显示 | 浮窗出现，但不抢占任何窗口焦点 |
| 用户点击浮窗按钮 | 按钮可响应，焦点不变化 |
| 用户点击桌面其他位置 | 正常聚焦到点击的窗口 |
| 浮窗消失 | 正常销毁 |

## 限制

- **仅 macOS**：此修改仅影响 macOS 平台
- **窗口仍可见**：浮窗会覆盖在下层窗口之上，但不会激活它们

## 不在范围内

- Windows/Linux 的类似实现
- 浮窗拖拽功能
- 多显示器支持改进
