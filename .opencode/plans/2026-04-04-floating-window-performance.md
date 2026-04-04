# 浮窗性能优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将快捷键触发后录音浮窗的显示延迟从 ~260ms 降低到 < 10ms

**Architecture:** 采用"异步化 + 预加载"组合方案：1) 应用启动时预创建浮窗（隐藏状态），2) 快捷键 handler 拆分为立即响应阶段（显示浮窗）和后台异步阶段（执行耗时操作），3) 异步操作完成后通过事件更新浮窗内容

**Tech Stack:** Rust (Tauri v2), Svelte 5, macOS

---

### Task 1: 预创建浮窗 - 在应用启动时创建 indicator 窗口

**Files:**
- Modify: `src-tauri/src/lib.rs` (setup 函数，约第 1480-1490 行)

- [ ] **Step 1: 在 setup() 末尾、app.manage() 之后添加预创建浮窗代码**

在 `app.manage(logger);` 之后、`Ok(())` 之前添加：

```rust
// --- Pre-create indicator window for instant show ---
let indicator_url = tauri::WebviewUrl::App("/recording".into());
let indicator_window = WebviewWindowBuilder::new(app.handle(), INDICATOR_LABEL, indicator_url)
    .inner_size(180.0, 48.0)
    .position(620.0, 700.0)
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .focusable(false)
    .accept_first_mouse(true)
    .build();

if let Ok(w) = indicator_window {
    #[cfg(target_os = "macos")]
    {
        let _ = macos::floating_panel::make_window_nonactivating(&w);
    }
}
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过，无错误

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "perf: pre-create indicator window at startup for instant show"
```

---

### Task 2: 重构 show_indicator() - 简化为获取预创建窗口 + 显示

**Files:**
- Modify: `src-tauri/src/lib.rs` (show_indicator 函数，第 540-607 行)

- [ ] **Step 1: 替换 show_indicator() 函数**

将现有的 `show_indicator()` 函数（第 540-607 行）替换为：

```rust
fn show_indicator(app_handle: &tauri::AppHandle, selected_text: Option<&str>) {
    let payload = serde_json::json!({
        "replaceMode": selected_text.is_some(),
        "selectedPreview": selected_text.map(|t| t.chars().take(50).collect::<String>()).unwrap_or_default()
    });

    // Try to get the pre-created indicator window
    if let Some(window) = app_handle.get_webview_window(INDICATOR_LABEL) {
        // Dynamically adjust position based on current main window's monitor
        if let Some(main_window) = app_handle.get_webview_window("main") {
            if let Ok(Some(monitor)) = main_window.primary_monitor() {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let screen_w = size.width as f64 / scale;
                let screen_h = size.height as f64 / scale;
                let win_w = 180.0;
                let win_h = 48.0;
                let bottom_margin = 24.0;
                let _ = window.set_position(tauri::LogicalPosition::new(
                    (screen_w - win_w) / 2.0,
                    screen_h - win_h - bottom_margin,
                ));
            }
        }

        let _ = window.show();
        let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
        return;
    }

    // Fallback: create window if pre-created one doesn't exist
    let main_window = app_handle.get_webview_window("main");
    let monitor = main_window
        .as_ref()
        .and_then(|w| w.primary_monitor().ok().flatten());

    let (x, y) = match &monitor {
        Some(m) => {
            let size = m.size();
            let scale = m.scale_factor();
            let screen_w = size.width as f64 / scale;
            let screen_h = size.height as f64 / scale;
            let win_w = 180.0;
            let win_h = 48.0;
            let bottom_margin = 24.0;
            ((screen_w - win_w) / 2.0, screen_h - win_h - bottom_margin)
        }
        None => (620.0, 700.0),
    };

    let url = "/recording";

    let window = WebviewWindowBuilder::new(
        app_handle,
        INDICATOR_LABEL,
        tauri::WebviewUrl::App(url.into()),
    )
    .inner_size(180.0, 48.0)
    .position(x, y)
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .focusable(false)
    .accept_first_mouse(true)
    .build();

    match window {
        Ok(w) => {
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
}
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "perf: refactor show_indicator to use pre-created window with fallback"
```

---

### Task 3: 重构录音模式快捷键 handler - 异步化耗时操作

**Files:**
- Modify: `src-tauri/src/lib.rs` (录音模式 handler，约第 1232-1283 行)

- [ ] **Step 1: 替换录音启动成功分支的代码**

将 `Some(())` 分支中的代码（约第 1232-1283 行）替换为两阶段架构：

```rust
Some(()) => {
    // === Phase 1: Immediate response (< 10ms) ===
    if let Ok(mut start) = recording_start_handler.lock() {
        *start = Some(Instant::now());
    }
    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(recording_icon_owned.clone()));
    }

    // Show indicator immediately (without selected_text, will update later)
    show_indicator(&app_handle, None);
    play_sound("Ping.aiff");

    // === Phase 2: Background async operations (~300ms) ===
    let app_handle_bg = app_handle.clone();
    let recording_icon_bg = recording_icon_owned.clone();
    let esc_bg = esc_shortcut_handler;

    // Capture values needed for async thread before moving
    let logger_bg = app_handle.try_state::<Logger>();

    std::thread::spawn(move || {
        // Get frontmost app name
        let frontmost = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Detect selected text
        let selected_text = clipboard::detect_selected_text(frontmost.as_deref().unwrap_or(""));

        // Save target app and selected text
        if let Some(ref app) = frontmost {
            clipboard::save_target_app(app);
        }
        if let Some(ref text) = selected_text {
            clipboard::save_selected_text(text);
            if let Some(ref lg) = logger_bg {
                lg.info("recording", "检测到选中文本，进入替换模式", Some(serde_json::json!({
                    "selected_length": text.len(),
                    "selected_preview": text.chars().take(100).collect::<String>()
                })));
            }
            // Update indicator with replaceMode
            let payload = serde_json::json!({
                "replaceMode": true,
                "selectedPreview": text.chars().take(50).collect::<String>()
            });
            let _ = app_handle_bg.emit_to(INDICATOR_LABEL, "indicator:recording", &payload);
        }

        // Auto mute
        let app_data_dir_mute = app_handle_bg.path().app_data_dir().unwrap_or_default();
        let app_config_mute = config::load_config(&app_data_dir_mute);
        if app_config_mute.features.recording.auto_mute {
            let _ = audio_control::save_and_mute(
                &app_data_dir_mute,
                app_handle_bg.try_state::<Logger>().as_deref(),
            );
        }

        // Hide main window
        if let Some(mw) = app_handle_bg.get_webview_window("main")
            && mw.is_visible().unwrap_or(false) {
                let _ = mw.hide();
            }

        // Register ESC shortcut
        let h = app_handle_bg.clone();
        let _ = h.global_shortcut().register(esc_bg);

        // Log
        if let Some(ref lg) = logger_bg {
            lg.info("recording", "录音开始", None);
        }
    });
}
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "perf: async recording shortcut handler for instant indicator show"
```

---

### Task 4: 重构翻译模式快捷键 handler - 异步化耗时操作

**Files:**
- Modify: `src-tauri/src/lib.rs` (翻译模式 handler，约第 1360-1401 行)

- [ ] **Step 1: 替换翻译启动成功分支的代码**

将 `Some(())` 分支中的代码（约第 1360-1401 行）替换为：

```rust
Some(()) => {
    // === Phase 1: Immediate response (< 10ms) ===
    if let Ok(mut start) = recording_start_handler.lock() {
        *start = Some(Instant::now());
    }
    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(recording_icon_owned.clone()));
    }

    // Show indicator immediately
    show_indicator(&app_handle, None);
    play_sound("Ping.aiff");

    // === Phase 2: Background async operations ===
    let app_handle_bg = app_handle.clone();
    let esc_bg = esc_shortcut_handler;
    let logger_bg = app_handle.try_state::<Logger>();

    std::thread::spawn(move || {
        // Auto mute
        let app_data_dir_mute = app_handle_bg.path().app_data_dir().unwrap_or_default();
        let app_config_mute = config::load_config(&app_data_dir_mute);
        if app_config_mute.features.recording.auto_mute {
            let _ = audio_control::save_and_mute(
                &app_data_dir_mute,
                app_handle_bg.try_state::<Logger>().as_deref(),
            );
        }

        // Hide main window
        if let Some(mw) = app_handle_bg.get_webview_window("main")
            && mw.is_visible().unwrap_or(false) {
                let _ = mw.hide();
            }

        // Register ESC shortcut
        let h = app_handle_bg.clone();
        let _ = h.global_shortcut().register(esc_bg);

        // Log
        if let Some(ref lg) = logger_bg {
            lg.info("recording", "录音开始 (翻译模式)", None);
        }
    });
}
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: 编译通过

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "perf: async translate shortcut handler for instant indicator show"
```

---

### Task 5: 运行测试 + 完整验证

**Files:**
- No changes

- [ ] **Step 1: 运行 Rust 单元测试**

```bash
cd src-tauri && cargo test
```

Expected: 所有测试通过

- [ ] **Step 2: 运行 cargo clippy 检查**

```bash
cd src-tauri && cargo clippy -- -D warnings
```

Expected: 无警告

- [ ] **Step 3: 运行 cargo fmt 格式化**

```bash
cd src-tauri && cargo fmt
```

- [ ] **Step 4: 提交格式化变更（如有）**

```bash
git add -A
git commit -m "style: apply cargo fmt"
```

---

## 自检验证

### 规范覆盖检查

| 规范要求 | 对应 Task | 状态 |
|----------|-----------|------|
| 启动时预创建浮窗 | Task 1 | ✓ |
| show_indicator 重构 | Task 2 | ✓ |
| 录音模式异步化 | Task 3 | ✓ |
| 翻译模式异步化 | Task 4 | ✓ |
| 测试验证 | Task 5 | ✓ |

### 占位符扫描

计划中无 TBD、TODO、"implement later" 等占位符。每个步骤都有完整代码。

### 类型一致性

- `INDICATOR_LABEL` 常量在所有任务中一致使用
- `app_handle.get_webview_window()` 返回类型一致
- `serde_json::json!` payload 结构一致
- `Logger` 通过 `try_state` 获取的方式一致
