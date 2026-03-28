# System Tray + Global Shortcut + Window Toggle Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** TalkShow 启动后常驻系统托盘，通过全局快捷键 Ctrl+" 呼出/隐藏主窗口，快捷键可配置。

**Architecture:** Rust 侧完成所有系统级能力（tray、shortcut、window toggle），配置文件存于 app_data_dir，应用启动时读取，不存在则创建默认配置。

**Tech Stack:** Tauri v2, tauri-plugin-global-shortcut, Rust serde/serde_json

---

## File Structure

| 文件 | 职责 |
|------|------|
| `src-tauri/Cargo.toml` | 添加 tray-icon feature + global-shortcut 插件依赖 |
| `package.json` | 添加 @tauri-apps/plugin-global-shortcut |
| `src-tauri/capabilities/default.json` | 添加 global-shortcut 权限 |
| `src-tauri/src/config.rs` (新增) | 配置结构体定义 + 读写逻辑 |
| `src-tauri/src/lib.rs` | 重写：setup 中初始化 tray、shortcut、config |

---

### Task 1: 添加依赖和权限

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 更新 Cargo.toml — 添加 tray-icon feature 和 global-shortcut 插件**

```toml
[package]
name = "talkshow"
version = "0.1.0"
description = "A Tauri App"
authors = ["you"]
edition = "2021"

[lib]
name = "talkshow_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-global-shortcut = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-openai = "0.20"
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: 更新 package.json — 添加 global-shortcut 插件**

```json
"dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "@tauri-apps/plugin-global-shortcut": "^2"
}
```

- [ ] **Step 3: 更新 capabilities/default.json — 添加 global-shortcut 权限**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered"
  ]
}
```

- [ ] **Step 4: 安装 npm 依赖**

Run: `npm install`

---

### Task 2: 创建配置模块

**Files:**
- Create: `src-tauri/src/config.rs`

- [ ] **Step 1: 创建 config.rs — 配置结构体和读写函数**

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Control+Shift+Quote";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub shortcut: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            shortcut: DEFAULT_SHORTCUT.to_string(),
        }
    }
}

pub fn config_file_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join(CONFIG_FILE_NAME)
}

pub fn load_config(app_data_dir: &PathBuf) -> AppConfig {
    let path = config_file_path(app_data_dir);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    } else {
        let config = AppConfig::default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&path, content);
        }
        config
    }
}

pub fn save_config(app_data_dir: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: 验证编译通过**

Run: `cargo check`
Expected: 编译通过（无错误）

---

### Task 3: 更新窗口初始状态

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 设置主窗口初始隐藏**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "TalkShow",
  "version": "0.1.0",
  "identifier": "com.jiaxia.talkshow",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../build"
  },
  "app": {
    "windows": [
      {
        "title": "TalkShow",
        "width": 800,
        "height": 600,
        "visible": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

---

### Task 4: 重写 lib.rs — 核心逻辑

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 重写 lib.rs — 集成 tray、global shortcut、config**

```rust
mod config;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn toggle_window(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.center();
        let _ = window.set_focus();
    }
}

fn parse_shortcut(shortcut_str: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code = None;

    for part in &parts {
        match *part {
            "Control" => modifiers |= Modifiers::CONTROL,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Alt" => modifiers |= Modifiers::ALT,
            "Command" | "Super" => modifiers |= Modifiers::SUPER,
            "Quote" => key_code = Some(Code::Quote),
            "Space" => key_code = Some(Code::Space),
            "KeyN" => key_code = Some(Code::KeyN),
            "KeyS" => key_code = Some(Code::KeyS),
            "KeyQ" => key_code = Some(Code::KeyQ),
            _ => {}
        }
    }

    key_code.map(|code| Shortcut::new(Some(modifiers), code))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().unwrap_or_default();
            let app_config = config::load_config(&app_data_dir);
            let shortcut_str = app_config.shortcut.clone();

            // --- 系统托盘 ---
            let show_i = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(false)
                .tooltip("TalkShow")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            toggle_window(&window);
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            toggle_window(&window);
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // --- 全局快捷键 ---
            let app_handle = app.handle().clone();
            if let Some(shortcut) = parse_shortcut(&shortcut_str) {
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    toggle_window(&window);
                                }
                            }
                        })
                        .build(),
                )?;

                app.global_shortcut().register(shortcut)?;
            }

            // --- 关闭窗口 → 隐藏 ---
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(|event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = event.window().hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: 验证编译通过**

Run: `cargo check`
Expected: 编译通过

---

### Task 5: 全量构建验证

**Files:**
- 无新增文件

- [ ] **Step 1: 前端构建**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 2: 完整 Tauri 构建**

Run: `npm run tauri build`
Expected: 编译通过，生成 TalkShow.app

- [ ] **Step 3: 功能验证清单**

手动运行 `src-tauri/target/release/talkshow` 验证：
- [ ] 应用启动后窗口不显示
- [ ] 系统托盘出现 TalkShow 图标
- [ ] 按 Ctrl+Shift+Quote 呼出窗口（居中、置顶）
- [ ] 再按一次隐藏窗口
- [ ] 点击托盘图标呼出/隐藏窗口
- [ ] 右键托盘图标 → Show/Hide 菜单 → 呼出/隐藏
- [ ] 右键托盘图标 → Quit → 退出应用
- [ ] 关闭按钮只隐藏不退出
- [ ] 配置文件 `~/Library/Application Support/com.jiaxia.talkshow/config.json` 已创建且内容正确
