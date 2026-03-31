# macOS 原生浮窗实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 macOS 上创建不抢占焦点的浮窗，使用 NSPanel 的 nonactivatingPanel 样式

**Architecture:** 
- 复用现有浮窗创建逻辑
- 在窗口创建后通过 objc2 修改 NSWindow 样式
- 仅影响 macOS 平台

**Tech Stack:** Rust, tauri, objc2, objc2-app-kit

---

## 文件结构

```
src-tauri/
├── src/lib.rs                    # 修改：在浮窗创建后调用样式修改
├── src/macos/
│   ├── mod.rs                     # 新增：macOS 模块入口
│   └── floating_panel.rs          # 新增：NSPanel 样式修改逻辑
```

---

## Task 1: 创建 macOS 模块结构

**Files:**
- Create: `src-tauri/src/macos/mod.rs`
- Create: `src-tauri/src/macos/floating_panel.rs`
- Modify: `src-tauri/src/lib.rs:9` (添加 `mod macos;`)

- [ ] **Step 1: 创建 macos/mod.rs**

```rust
#[cfg(target_os = "macos")]
pub mod floating_panel;
```

- [ ] **Step 2: 创建 macos/floating_panel.rs 空模块**

```rust
#[cfg(target_os = "macos")]
pub fn make_window_nonactivating(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}
```

- [ ] **Step 3: 在 lib.rs 添加 mod macos**

在 lib.rs 第 9 行后添加:
```rust
#[cfg(target_os = "macos")]
mod macos;
```

- [ ] **Step 4: 验证编译**

```bash
cd /Users/jia.xia/development/talkshow/src-tauri && cargo check
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/macos/ src-tauri/src/lib.rs
git commit -m "feat(macos): scaffold macos module for native integrations"
```

---

## Task 2: 实现 NSWindow 样式修改函数

**Files:**
- Modify: `src-tauri/src/macos/floating_panel.rs`

- [ ] **Step 1: 编写 make_window_nonactivating 函数**

```rust
#![cfg(target_os = "macos")]

use tauri::WebviewWindow;
use objc2::msg_send;
use objc2::foundation::NSWindow;
use objc2::app_kit::{NSWindowStyleMask, NSWindowStyleMaskExt};

pub fn make_window_nonactivating(window: &WebviewWindow) -> Result<(), String> {
    // 获取底层 NSWindow
    let ns_window = window
        .ns_window()
        .ok_or("Failed to get NSWindow handle")?;

    // 获取当前样式
    let style: NSWindowStyleMask = unsafe { msg_send![ns_window, styleMask] };

    // 添加 nonactivatingPanel 样式
    let new_style = style | NSWindowStyleMask::nonactivatingPanel;

    // 设置新样式
    unsafe {
        msg_send![ns_window, setStyleMask: new_style]
    }

    Ok(())
}
```

- [ ] **Step 2: 验证编译**

```bash
cd /Users/jia.xia/development/talkshow/src-tauri && cargo check 2>&1
```

预期可能有编译错误，需要根据实际 API 调整

- [ ] **Step 3: 修复编译问题（如有）**

根据错误信息调整代码，可能需要：
- 调整 import 路径
- 使用不同的方法获取 NSWindow

- [ ] **Step 4: 验证编译通过**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/macos/floating_panel.rs
git commit -m "feat(macos): add make_window_nonactivating function"
```

---

## Task 3: 集成到浮窗创建流程

**Files:**
- Modify: `src-tauri/src/lib.rs:469-477`

- [ ] **Step 1: 在浮窗创建成功后调用样式修改**

将 lib.rs 第 469-477 行:
```rust
match window {
    Ok(w) => {
        let _ = w.show();
        let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
    }
    Err(e) => {
        eprintln!("Failed to create indicator window: {}", e);
    }
}
```

修改为:
```rust
match window {
    Ok(w) => {
        // macOS: 将浮窗设置为 nonactivatingPanel
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = macos::floating_panel::make_window_nonactivating(&w) {
                eprintln!("Failed to make window nonactivating: {}", e);
            }
        }

        let _ = w.show();
        let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
    }
    Err(e) => {
        eprintln!("Failed to create indicator window: {}", e);
    }
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(recording-indicator): apply nonactivating panel style on macOS"
```

---

## Task 4: 测试验证

**Files:**
- 测试需要在实际 macOS 机器上运行

- [ ] **Step 1: 运行应用**

```bash
cd /Users/jia.xia/development/talkshow && npm run tauri dev
```

- [ ] **Step 2: 验证行为**

1. 打开另一个应用窗口（如 Finder）
2. 点击该窗口使其获得焦点
3. 开始录音，观察浮窗出现时其他窗口焦点是否保持
4. 浮窗显示时，点击浮窗区域，验证焦点是否保持不变

- [ ] **Step 3: 如有问题，修复后重新验证**

---

## 验证清单

- [ ] macOS 上浮窗弹出不抢占焦点
- [ ] 浮窗按钮仍然可点击
- [ ] 录音状态切换正常
- [ ] 浮窗消失行为正常
