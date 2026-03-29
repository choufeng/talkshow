# 修复快捷键动态注册 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复设置页面修改快捷键后无法正确生效的问题——确保旧快捷键被注销、新快捷键被注册，且 handler 能正确匹配新快捷键。

**Architecture:** 核心问题是 handler 闭包在启动时捕获了固定的 `rec_id`，导致运行时更新快捷键后 handler 无法匹配新的快捷键 ID。方案是将快捷键 ID 存入 `Arc<Mutex<u32>>` 共享状态，handler 动态读取当前 ID 进行匹配，同时 `update_shortcut` 正确注销旧快捷键并更新共享状态。同时用 `Code::from_str` 替换硬编码的键名映射。

**Tech Stack:** Rust, Tauri 2, tauri-plugin-global-shortcut 2, keyboard-types (Code 的 FromStr)

---

### Task 1: 用 `Code::from_str` 替换 `parse_shortcut` 中的硬编码键名映射

**Files:**
- Modify: `src-tauri/src/lib.rs:29-53`

当前 `parse_shortcut` 只支持 8 个硬编码键名，前端允许录制任意 `KeyA-Z` 和 `Digit0-9`，导致大多数键解析返回 `None`。`keyboard-types::Code` 实现了 `FromStr`，可以直接用字符串解析。

- [ ] **Step 1: 重写 `parse_shortcut` 函数**

将 `lib.rs:29-53` 的 `parse_shortcut` 替换为：

```rust
fn parse_shortcut(shortcut_str: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match *part {
            "Control" => modifiers |= Modifiers::CONTROL,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Alt" => modifiers |= Modifiers::ALT,
            "Command" | "Super" => modifiers |= Modifiers::SUPER,
            s => {
                if let Ok(code) = s.parse::<Code>() {
                    key_code = Some(code);
                }
            }
        }
    }

    key_code.map(|code| Shortcut::new(Some(modifiers), code))
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: 编译成功，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: use Code::from_str for comprehensive key parsing in parse_shortcut"
```

---

### Task 2: 引入共享状态存储当前快捷键 ID，使 handler 动态匹配

**Files:**
- Modify: `src-tauri/src/lib.rs` (setup 闭包 + handler + update_shortcut)

这是核心修复。添加 `Arc<Mutex<u32>>` 存储 toggle/rec 快捷键 ID，handler 闭包动态读取匹配，`update_shortcut` 同步更新。

- [ ] **Step 1: 添加共享状态类型**

在 `lib.rs` 顶部 static 声明区域（约第 15-16 行后），添加共享状态：

```rust
use std::sync::RwLock;

struct ShortcutIds {
    toggle: u32,
    recording: u32,
}

static SHORTCUT_IDS: RwLock<ShortcutIds> = RwLock::new(ShortcutIds {
    toggle: 0,
    recording: 0,
});
```

- [ ] **Step 2: 在 setup 中初始化共享状态**

在 `lib.rs` 的 `setup` 闭包中，`let rec_id = ...` 之后（约 269 行后），添加：

```rust
let toggle_id = toggle_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
let rec_id_val = rec_shortcut.as_ref().map(|s| s.id()).unwrap_or(0);
{
    let mut ids = SHORTCUT_IDS.write().unwrap();
    ids.toggle = toggle_id;
    ids.recording = rec_id_val;
}
```

注意：删除原来的 `let rec_id = rec_shortcut.as_ref().map(|s| s.id());` 行，替换为 `let rec_id_val`。

- [ ] **Step 3: 修改 handler 闭包中的匹配逻辑**

在 handler 闭包内，将原来通过捕获变量 `rec_id` 匹配的逻辑，改为从共享状态读取。

将以下代码段（约 282-284 行）：

```rust
let id = shortcut.id();

if id == esc_id {
```

替换为：

```rust
let id = shortcut.id();
let (current_toggle_id, current_rec_id) = {
    let ids = SHORTCUT_IDS.read().unwrap();
    (ids.toggle, ids.recording)
};

if id == esc_id {
```

然后，将原来的：

```rust
if rec_id == Some(id) {
```

替换为：

```rust
if current_rec_id != 0 && id == current_rec_id {
```

最后的 fallback 分支（toggle window）保持不变。

- [ ] **Step 4: 修复 `update_shortcut` 函数——正确注销旧快捷键并更新共享状态**

将 `update_shortcut` 函数（`lib.rs:132-159`）替换为：

```rust
#[tauri::command]
fn update_shortcut(
    app_handle: tauri::AppHandle,
    shortcut_type: String,
    shortcut: String,
) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let mut app_config = config::load_config(&app_data_dir);

    let old_toggle = app_config.shortcut.clone();
    let old_recording = app_config.recording_shortcut.clone();

    match shortcut_type.as_str() {
        "toggle" => app_config.shortcut = shortcut,
        "recording" => app_config.recording_shortcut = shortcut,
        _ => return Err("Invalid shortcut type".to_string()),
    }

    config::save_config(&app_data_dir, &app_config)?;

    // Unregister old shortcuts
    if let Some(sc) = parse_shortcut(&old_toggle) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }
    if let Some(sc) = parse_shortcut(&old_recording) {
        let _ = app_handle.global_shortcut().unregister(sc);
    }

    // Register new shortcuts
    let new_toggle = parse_shortcut(&app_config.shortcut);
    let new_rec = parse_shortcut(&app_config.recording_shortcut);

    if let Some(sc) = new_toggle {
        let _ = app_handle.global_shortcut().register(sc);
    }
    if let Some(sc) = new_rec {
        let _ = app_handle.global_shortcut().register(sc);
    }

    // Update shared state so handler matches new IDs
    {
        let mut ids = SHORTCUT_IDS.write().unwrap();
        ids.toggle = new_toggle.map(|s| s.id()).unwrap_or(0);
        ids.recording = new_rec.map(|s| s.id()).unwrap_or(0);
    }

    Ok(())
}
```

- [ ] **Step 5: 编译验证**

Run: `cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: 编译成功，无错误

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: dynamically match shortcuts via shared state, properly unregister old shortcuts on update"
```

---

### Task 3: 手动集成验证

**Files:** 无代码修改

- [ ] **Step 1: 构建并运行应用**

Run: `cargo tauri dev --manifest-path src-tauri/Cargo.toml`
Expected: 应用正常启动，系统托盘图标出现

- [ ] **Step 2: 验证默认快捷键**

1. 按 `Ctrl+Shift+'` 验证 toggle window 功能正常
2. 按 `Ctrl+\` 验证录音开始/停止功能正常
3. 按 `Esc` 验证取消录音功能正常

- [ ] **Step 3: 验证快捷键修改后生效**

1. 打开设置页面，修改 toggle 快捷键（如改为 `Ctrl+Shift+A`）
2. 按 `Ctrl+Shift+'`（旧快捷键）应不再触发任何操作
3. 按 `Ctrl+Shift+A`（新快捷键）应正确 toggle 窗口
4. 修改录音快捷键（如改为 `Ctrl+Shift+R`）
5. 按 `Ctrl+\`（旧快捷键）应不再触发录音
6. 按 `Ctrl+Shift+R` 应正确开始/停止录音
7. 重启应用，验证快捷键设置已持久化且仍然生效

- [ ] **Step 4: 最终 Commit（如有遗漏修复）**

```bash
git add -A
git commit -m "fix: shortcut registration verification complete"
```
