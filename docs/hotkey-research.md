# macOS 全局快捷键方案调研

> 调研日期：2026-03-31
> 背景：当前使用 `tauri-plugin-global-shortcut`（底层 `global-hotkey`），无法区分左右修饰键，也不支持单个修饰键作为快捷键。

## 当前方案的限制

`global-hotkey` 使用 Carbon API `RegisterEventHotKey`：

- 修饰键参数只接受**通用标志位**（如 `cmdKey = 0x0100`），不区分左右
- **必须**指定一个非修饰键作为主键，不支持单独修饰键触发
- `Modifiers` 枚举只有 `SHIFT` / `CONTROL` / `ALT` / `SUPER`，没有左右之分

## 候选方案

### 1. `rdev` crate

最成熟的 Rust 跨平台键盘监听库（720+ GitHub stars）。

`Key` 枚举已区分左右修饰键：

```rust
MetaLeft,     // 左 Command
MetaRight,    // 右 Command
ShiftLeft, ShiftRight,
ControlLeft, ControlRight,
Alt, AltGr,
```

- **区分左右修饰键**: ✅（Command/Shift/Control 均有左右）
- **单键触发**: ✅（可以监听 `KeyPress(Key::MetaRight)` 事件）
- **拦截/消费事件**: ✅（`grab()` 功能，标记为 `unstable_grab`）
- **macOS 底层**: 使用 `CGEventTap`
- **权限**: 需要辅助功能权限（Accessibility）
- **限制**:
  - `listen()` 是阻塞调用，需要独占线程
  - macOS 上**必须在主线程运行**（与 Tauri 冲突）
  - 没有"注册热键"的高层 API，需自行实现状态跟踪

### 2. `core-graphics` CGEventTap

macOS 原生 Core Graphics 事件监听 API 的 Rust 绑定。

`KeyCode` 已定义所有虚拟键码：

```rust
COMMAND = 55,        // 左 Command
RIGHT_COMMAND = 54,  // 右 Command
SHIFT = 56, RIGHT_SHIFT = 60,
OPTION = 58, RIGHT_OPTION = 61,
CONTROL = 59, RIGHT_CONTROL = 62,
```

关键点：
- 修饰键按下/释放时，事件类型是 `kCGEventFlagsChanged`（不是 KeyDown/KeyUp）
- 通过 `keyCode` 字段区分具体是哪个修饰键变化
- 可以消费事件（拦截不让传递）

- **区分左右修饰键**: ✅
- **单键触发**: ✅
- **拦截/消费事件**: ✅
- **权限**: 需要 `AXIsProcessTrusted()` 辅助功能权限
- **限制**: 需要手动创建 `CGEventTap`、管理 `CFRunLoop`、处理回调，代码量大
- **优势**: 可以与 Tauri 现有的 RunLoop 集成

### 3. `NSEvent` addGlobalMonitorForEvents

通过 `objc2-app-kit` crate 在 Rust 中调用 macOS AppKit API。

- **区分左右修饰键**: 部分（`keyCode` 可以，`modifierFlags` 不行）
- **单键触发**: ✅
- **拦截/消费事件**: ❌（只读监听）
- **限制**: 无法拦截事件，不适合需要消费事件的场景

### 4. IOKit / HID

直接访问 HID 设备，获取原始键盘数据。

- **结论**: 太底层，需要 root 或特殊驱动权限，**不推荐**

## 方案对比

| 维度 | `rdev` | `core-graphics` CGEventTap | `NSEvent` global monitor | `global-hotkey` (现有) |
|------|--------|---------------------------|-------------------------|----------------------|
| **区分左右修饰键** | ✅ | ✅ | 部分 | ❌ |
| **单键触发** | ✅ | ✅ | ✅ | ❌ |
| **拦截/消费事件** | ✅(unstable) | ✅ | ❌(只读) | ❌ |
| **需要辅助功能权限** | 是 | 是 | 是 | 否(用Carbon API) |
| **API 层次** | 高层(监听回调) | 低层(手动建Tap) | 高层(ObjC回调) | 高层(注册即用) |
| **Rust 集成难度** | 低(add依赖即可) | 中(需写FFI胶水) | 中(需objc2调用) | 已集成 |
| **与 Tauri 兼容性** | 主线程冲突 ⚠️ | 可接入RunLoop | 可接入RunLoop | 原生兼容 |
| **维护状态** | 活跃(720 stars) | servo项目维护 | 苹果API | Tauri官方维护 |
| **跨平台** | macOS/Win/Linux | macOS only | macOS only | macOS/Win/Linux(X11) |

## 推荐方案：混合架构

保留 `global-hotkey` 处理普通组合键，新增底层监听处理单修饰键场景。

```
┌─────────────────────────────────────────────┐
│             快捷键处理层                      │
├──────────────────┬──────────────────────────┤
│  global-hotkey   │    CGEventTap / rdev      │
│  (现有方案)       │    (新增)                 │
│                  │                          │
│  · Cmd+Shift+A   │  · 右 Command 单独按下    │
│  · Ctrl+\        │  · 修饰键状态跟踪         │
│  · 普通组合键     │                          │
└──────────────────┴──────────────────────────┘
```

### 路径 A：使用 `rdev` crate

```rust
use rdev::{listen, Event, EventType, Key};
use std::sync::mpsc;

enum HotkeyAction {
    RightCommandPressed,
    RightCommandReleased,
}

fn start_hotkey_listener(tx: mpsc::Sender<HotkeyAction>) {
    std::thread::spawn(move || {
        let mut modifier_state = ModifierState::default();

        listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(Key::MetaRight) => {
                    modifier_state.right_cmd = true;
                    if !modifier_state.any_other_key_pressed() {
                        let _ = tx.send(HotkeyAction::RightCommandPressed);
                    }
                }
                EventType::KeyRelease(Key::MetaRight) => {
                    modifier_state.right_cmd = false;
                    let _ = tx.send(HotkeyAction::RightCommandReleased);
                }
                EventType::KeyPress(key) if !is_modifier(key) => {
                    modifier_state.other_key_pressed = true;
                }
                EventType::KeyRelease(key) if !is_modifier(key) => {
                    modifier_state.other_key_pressed = false;
                }
                _ => {}
            }
        })
        .unwrap();
    });
}
```

**风险**: `rdev::listen` 在 macOS 上必须在主线程运行，与 Tauri 冲突。

### 路径 B：使用 `core-graphics` CGEventTap（推荐）

手动创建 `CGEventTap`，将 event source 添加到 Tauri 的 CFRunLoop。

**优点**:
- 与 Tauri RunLoop 深度集成，无需额外线程
- 完全掌控事件处理逻辑

**缺点**:
- 需要写更多底层代码
- 仅 macOS 可用

## 辅助功能权限

无论选择哪条路径，都需要用户授予辅助功能权限：

- 首次使用时调用 `AXIsProcessTrusted()` 检查
- 未授权时引导用户前往「系统设置 → 隐私与安全性 → 辅助功能」
- 需要在 UI 中添加权限引导流程

## 后续步骤

1. 验证 `rdev` 在 Tauri 非 macOS 主线程下的行为
2. 如不可行，走路径 B（CGEventTap）
3. 设计辅助功能权限引导 UI
4. 实现修饰键状态机（按下/释放追踪，防误触）
