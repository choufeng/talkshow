# Batch 3: P1 录音路径迁移 + 命令注入修复

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 2 个 High 级别安全问题 — 将录音文件从全局可读的临时目录迁移到受权限保护的 app_data_dir，以及修复 osascript 调用中的字符串转义问题防止命令注入。

**Architecture:** 修改 `recording.rs` 添加新的路径函数，修改 `lib.rs` 调用新路径。修改 `clipboard.rs` 和 `skills.rs` 添加 AppleScript 字符串转义。

**Tech Stack:** Rust / Tauri 2

**前置依赖：** 无

---

## File Structure

| 文件 | 操作 |
|------|------|
| `src-tauri/src/recording.rs` | 修改 — 添加 app_data_dir 版本的路径函数 |
| `src-tauri/src/lib.rs` | 修改 — 使用新路径函数 |
| `src-tauri/src/clipboard.rs` | 修改 — 添加 AppleScript 字符串转义 |
| `src-tauri/src/skills.rs` | 修改 — 添加 AppleScript 字符串转义 |

---

### Task 1: 录音文件迁移到 app_data_dir

**Files:**
- Modify: `src-tauri/src/recording.rs:107-115`
- Modify: `src-tauri/src/lib.rs`

**Context:** `recordings_dir()` 返回 `std::env::temp_dir().join("talkshow")`。在 macOS 上 `/tmp` 对所有用户可读，录音可能包含敏感对话内容。应存储到 `app_data_dir`（`~/Library/Application Support/com.jiaxia.talkshow/recordings/`），该目录有 700 权限。

- [ ] **Step 1: 在 `recording.rs` 中添加新路径函数**

在现有 `ensure_recordings_dir` 函数之后添加：

```rust
pub fn recordings_dir_in(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(RECORDINGS_DIR_NAME)
}

pub fn ensure_recordings_dir_in(app_data_dir: &std::path::Path) -> Result<PathBuf, String> {
    let dir = recordings_dir_in(app_data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings dir: {}", e))?;
    Ok(dir)
}
```

保留旧函数 `recordings_dir()` 和 `ensure_recordings_dir()` 不变，用于向后兼容访问旧录音。

- [ ] **Step 2: 修改 `lib.rs` 中所有录音目录创建调用**

搜索 `lib.rs` 中所有 `recording::ensure_recordings_dir()` 调用，将它们替换为传入 `app_data_dir` 的版本：

```rust
// 旧：
let rec_dir = recording::ensure_recordings_dir()?;

// 新：
let rec_dir = recording::ensure_recordings_dir_in(&app_data_dir)?;
```

**注意：** 需要在每个调用点确认 `app_handle.path().app_data_dir()` 可用。如果调用点在闭包中，可能需要提前获取 app_data_dir 的所有权。

- [ ] **Step 3: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 4: 手动验证录音功能**

Run: `npm run tauri dev`
验证：
- 录音能正常启动和停止
- 录音文件保存在 `~/Library/Application Support/com.jiaxia.talkshow/recordings/` 下
- 旧录音（如果存在于 `/tmp/talkshow/`）仍可通过日志页面查看

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/recording.rs src-tauri/src/lib.rs
git commit -m "fix(security): move recordings to app_data_dir (H-5)"
```

---

### Task 2: 修复 osascript 字符串转义（命令注入）

**Files:**
- Modify: `src-tauri/src/clipboard.rs:41-55`
- Modify: `src-tauri/src/skills.rs:11-44`

**Context:** 多处 `osascript` 调用将外部获取的字符串（如前台应用进程名）直接拼入 AppleScript。如果恶意应用将进程名设置为包含双引号的字符串，将破坏 AppleScript 语法。需要转义 `\` 和 `"` 字符。

- [ ] **Step 1: 在 `clipboard.rs` 中添加转义函数**

在 `simulate_paste` 函数之前添加：

```rust
#[cfg(target_os = "macos")]
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
```

- [ ] **Step 2: 修改 `clipboard.rs` 中的 `simulate_paste`**

将第 47 行：
```rust
.arg(format!("tell application \"{}\" to activate", app))
```
改为：
```rust
.arg(format!("tell application \"{}\" to activate", escape_applescript_string(&app)))
```

- [ ] **Step 3: 在 `skills.rs` 中添加同样的转义函数**

在 `get_frontmost_app` 函数之前添加：

```rust
#[cfg(target_os = "macos")]
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
```

- [ ] **Step 4: 修改 `skills.rs` 中的 `get_frontmost_app`**

将第 28-31 行：
```rust
.arg(format!(
    "tell application \"System Events\" to get bundle identifier of process \"{}\"",
    app_name
))
```
改为：
```rust
.arg(format!(
    "tell application \"System Events\" to get bundle identifier of process \"{}\"",
    escape_applescript_string(&app_name)
))
```

- [ ] **Step 5: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/clipboard.rs src-tauri/src/skills.rs
git commit -m "fix(security): escape strings in osascript calls (H-2)"
```
