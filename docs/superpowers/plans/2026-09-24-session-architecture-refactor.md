# 录音会话架构重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将录音会话生命周期收敛到单一 owner（SessionManager），消除全局原子变量手工竞态协议，统一 osascript 系统调用入口，使项目恢复可持续开发。

**Architecture:** 新增 `session.rs`（会话状态机，单锁全字段原子转移）与 `sys.rs`（osascript 串行执行器，消除 AppleEvents 并发死锁）。`lib.rs` 快捷键 handler 从 ~400 行压缩为 ~80 行分发器，transcription/translation 重复分支合并。pipeline 的状态检查全部改走 manager 查询。前端与 IPC 契约零改动。

**Tech Stack:** Rust (edition 2024) / Tauri v2 / std 同步原语（无新增依赖）

---

## 背景：为什么重构

历史缺陷（`docs/superpowers/specs/2026-04-25-shortcut-freeze-fix-design.md` 记录 4 个竞态）的修复方式是 `SESSION_ID` 计数器 + 后台线程手工 checkpoint 比对。该协议要求：

- 每条 stop 路径必须记得 `SESSION_ID.fetch_add`（现有 4 条路径，已漏过）
- 每个后台副作用步骤前必须手工比对（现有 4 checkpoint × 2 分支）
- `CANCELLED` / `RECORDING` / `TARGET_APP` 独立全局变量，状态转移无原子性
- ESC unregister 逻辑在 pipeline.rs 重复 10 处

每加一个功能都要在这套手工协议上叠加。历史 git log 连续 4 个 fix 提交证明不可持续。

### 现状状态变量清单（全部待消除）

| 变量 | 位置 | 问题 |
|------|------|------|
| `RECORDING: AtomicU8` | shortcuts.rs | 会话活跃态，被 6 处读写 |
| `SESSION_ID: AtomicU64` | shortcuts.rs | 手工取消协议核心 |
| `CANCELLED: AtomicBool` | shortcuts.rs | pipeline 取消标志，4 文件读写 |
| `TARGET_APP: Mutex<Option<String>>` | clipboard.rs | 跨会话串扰 bug（读到上一会话的值） |
| `SHORTCUT_IDS: RwLock` | shortcuts.rs | 保留（快捷键 id 查询表，无竞态） |
| `LAST_REC_PRESS: Mutex` | shortcuts.rs | 保留（按键防抖，与会话无关） |

### 行为规格（重构必须逐条保持）

| 触发 | 现行为 | 新行为 |
|------|--------|--------|
| rec 键，空闲 | 开始转写录音 | `start(MODE_TRANSCRIPTION)` |
| rec 键，录音中 | 停止 → complete pipeline | `stop()` + spawn `stop_recording("recording:complete", s)` |
| translate 键，空闲 | 开始翻译录音 | `start(MODE_TRANSLATION)` |
| translate 键，录音中 | 停止 → complete pipeline | 同 rec 键停止路径 |
| ESC，录音中 | 停止 → cancel pipeline | `signal_cancel()` 返回 Some → cancel 路径 |
| ESC，处理中（indicator 在） | `CANCELLED=true`，销毁指示器 | `signal_cancel()` 返回 None（标记 last_finished）+ 销毁指示器 |
| `indicator:cancel` 事件 | 同 ESC 两条 | 同 ESC |
| pipeline 内取消检查 | `CANCELLED.load()` | `mgr.is_cancelled(session.id)` |
| pipeline 内"录音已重启，丢弃结果" | `RECORDING != NONE` | `mgr.has_active()` |
| 后台线程副作用 checkpoint | `SESSION_ID != snapshot` | `mgr.is_active(session.id)` |
| 停止时粘贴目标 | `clipboard::get_target_app()`（可能拿到上一会话的值——bug） | `session.target_app`（None 时粘贴降级为不激活直接 keystroke，与现状 None 分支一致） |
| 停止路径信号音 | complete=`Frog.aiff`，cancel=`Pop.aiff` | 不变 |
| 转写/翻译录音开始信号音 | `Tink.aiff` | 不变 |
| 500ms 按键防抖 | `LAST_REC_PRESS` | 不变（保留原变量） |

### 交付后删除清单

- `shortcuts.rs` 中 `RECORDING` / `CANCELLED` / `SESSION_ID` / `RECORDING_MODE_*`
- `clipboard.rs` 中 `TARGET_APP` / `save_target_app` / `get_target_app`
- `real_llm_client.rs`（并入 `llm_client.rs`）
- lib.rs 快捷键 handler 内联 osascript（~40 行 × 2）

---

## 文件结构

```
src-tauri/src/
├── session.rs          # 新增：Session + SessionManager（唯一会话状态 owner）
├── sys.rs              # 新增：osascript 串行执行器 + frontmost_app_name
├── shortcuts.rs        # 修改：只留 parse_shortcut / ShortcutIds / LAST_REC_PRESS
├── lib.rs              # 修改：handler 重写 ~400 行 → ~80 行分发器
├── pipeline.rs         # 修改：stop_recording 接收 Session；cleanup_esc 提取
├── clipboard.rs        # 修改：删 TARGET_APP；simulate_paste 走 sys
├── audio_control.rs    # 修改：osascript 走 sys
├── skills.rs           # 修改：get_frontmost_app 走 sys
├── llm_client.rs       # 修改：吸收 real_llm_client.rs
└── real_llm_client.rs  # 删除
```

依赖顺序：Task 1、2 无依赖可并行 → Task 3 依赖 2 → Task 4 依赖 1 → Task 5 依赖 4 → Task 6、7 独立。

---

### Task 0: 基线验证与工作树

**Files:** 无代码改动

- [ ] **Step 1: 确认基线全绿**

```bash
cd /Users/jia.xia/development/talkshow
npm run check && npm test && cd src-tauri && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cd ..
```

Expected: 全部通过。任何失败先停下报告，不带病重构。

- [ ] **Step 2: 创建 worktree**

```bash
git check-ignore -q .worktrees || echo ".worktrees/" >> .gitignore
git worktree add .worktrees/refactor-session-arch -b refactor/session-architecture
cd .worktrees/refactor-session-arch
```

后续所有命令在 worktree 内执行。

---

### Task 1: session.rs — SessionManager 状态机

**Files:**
- Create: `src-tauri/src/session.rs`
- Modify: `src-tauri/src/lib.rs`（加 `mod session;`）

- [ ] **Step 1: 写失败测试**

创建 `src-tauri/src/session.rs`：

```rust
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub const MODE_NONE: u8 = 0;
pub const MODE_TRANSCRIPTION: u8 = 1;
pub const MODE_TRANSLATION: u8 = 2;

#[derive(Clone, Debug)]
pub struct Session {
    pub id: u64,
    pub mode: u8,
    pub started_at: Instant,
    pub target_app: Option<String>,
}

struct Inner {
    active: Option<Session>,
    last_finished: Option<u64>,
    cancelled: Option<u64>,
}

pub struct SessionManager {
    inner: Mutex<Inner>,
    next_id: AtomicU64,
}

impl SessionManager {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                active: None,
                last_finished: None,
                cancelled: None,
            }),
            next_id: AtomicU64::new(1),
        }
    }

    /// 开始新会话。已有活动会话时返回 None（调用方决定是否忽略）。
    /// 开始新会话同时清除旧取消标志（新 pipeline 不继承旧取消）。
    pub fn start(&self, mode: u8) -> Option<Session> {
        let mut g = self.lock();
        if g.active.is_some() {
            return None;
        }
        let s = Session {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            mode,
            started_at: Instant::now(),
            target_app: None,
        };
        g.active = Some(s.clone());
        g.cancelled = None;
        Some(s)
    }

    /// 结束当前会话并返回它（供后台清理 / pipeline 使用）。无活动会话返回 None。
    pub fn stop(&self) -> Option<Session> {
        let mut g = self.lock();
        let s = g.active.take();
        if let Some(ref s) {
            g.last_finished = Some(s.id);
        }
        s
    }

    /// ESC / 指示器取消：
    /// - 录音中 → 结束会话并标记取消，返回 Some(session)（走 cancel pipeline）
    /// - 处理中（无活动会话）→ 标记最近结束的会话为取消，返回 None（pipeline 自行检查）
    pub fn signal_cancel(&self) -> Option<Session> {
        let mut g = self.lock();
        let s = g.active.take();
        let id = match &s {
            Some(s) => Some(s.id),
            None => g.last_finished,
        }?;
        g.cancelled = Some(id);
        g.last_finished = Some(id);
        s
    }

    /// 后台线程 checkpoint：该会话是否仍是当前活动会话。
    pub fn is_active(&self, id: u64) -> bool {
        self.lock()
            .active
            .as_ref()
            .is_some_and(|s| s.id == id)
    }

    pub fn has_active(&self) -> bool {
        self.lock().active.is_some()
    }

    /// pipeline 取消检查：该会话是否被标记取消。
    pub fn is_cancelled(&self, id: u64) -> bool {
        self.lock().cancelled == Some(id)
    }

    /// 后台线程写入粘贴目标应用（仅对当前活动会话生效，过期 id 静默丢弃）。
    pub fn set_target_app(&self, id: u64, app: String) {
        let mut g = self.lock();
        if let Some(ref mut s) = g.active {
            if s.id == id {
                s.target_app = Some(app);
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_returns_session_and_stop_returns_same() {
        let m = SessionManager::new();
        let a = m.start(MODE_TRANSCRIPTION).unwrap();
        let b = m.stop().unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(b.mode, MODE_TRANSCRIPTION);
    }

    #[test]
    fn ids_monotonically_increase() {
        let m = SessionManager::new();
        let a = m.start(MODE_TRANSLATION).unwrap();
        m.stop();
        let c = m.start(MODE_TRANSLATION).unwrap();
        assert!(c.id > a.id);
    }

    #[test]
    fn start_while_active_returns_none() {
        let m = SessionManager::new();
        assert!(m.start(MODE_TRANSCRIPTION).is_some());
        assert!(m.start(MODE_TRANSLATION).is_none());
    }

    #[test]
    fn stop_clears_active() {
        let m = SessionManager::new();
        m.start(MODE_TRANSLATION).unwrap();
        assert!(m.stop().is_some());
        assert!(!m.has_active());
        assert!(m.stop().is_none());
    }

    #[test]
    fn signal_cancel_during_recording_ends_session() {
        let m = SessionManager::new();
        let s = m.start(MODE_TRANSCRIPTION).unwrap();
        assert!(m.is_active(s.id));
        let cancelled = m.signal_cancel().unwrap();
        assert_eq!(cancelled.id, s.id);
        assert!(!m.has_active());
        assert!(m.is_cancelled(s.id));
    }

    #[test]
    fn signal_cancel_after_stop_marks_last_pipeline() {
        let m = SessionManager::new();
        let s = m.start(MODE_TRANSLATION).unwrap();
        m.stop();
        // 无活动会话：仅标记，返回 None
        assert!(m.signal_cancel().is_none());
        assert!(m.is_cancelled(s.id));
    }

    #[test]
    fn new_session_clears_cancel_flag() {
        let m = SessionManager::new();
        let s = m.start(MODE_TRANSLATION).unwrap();
        m.stop();
        m.signal_cancel();
        let n = m.start(MODE_TRANSLATION).unwrap();
        assert!(!m.is_cancelled(n.id));
        assert!(!m.is_cancelled(s.id));
    }

    #[test]
    fn target_app_scoped_to_active_session() {
        let m = SessionManager::new();
        let a = m.start(MODE_TRANSCRIPTION).unwrap();
        m.set_target_app(a.id, "Safari".into());
        let stopped = m.stop().unwrap();
        assert_eq!(stopped.target_app.as_deref(), Some("Safari"));

        // 过期 id 写入被丢弃；错 id 写入被丢弃
        m.set_target_app(a.id, "stale".into());
        let b = m.start(MODE_TRANSLATION).unwrap();
        m.set_target_app(a.id, "wrong-id".into());
        let stopped_b = m.stop().unwrap();
        assert_eq!(stopped_b.target_app, None);
    }
}
```

- [ ] **Step 2: 挂载模块**

`src-tauri/src/lib.rs` 顶部 `mod recording;` 之后加一行：

```rust
mod session;
```

- [ ] **Step 3: 跑测试**

```bash
cd src-tauri && cargo test session:: -- --nocapture
```

Expected: `test result: ok. 8 passed`

- [ ] **Step 4: clippy + fmt**

```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo fmt
```

Expected: 无警告。

- [ ] **Step 5: Commit**

```bash
git add src/session.rs src/lib.rs
git commit -m "feat(session): add SessionManager as single owner of recording session lifecycle"
```

---

### Task 2: sys.rs — osascript 串行执行器

**Files:**
- Create: `src-tauri/src/sys.rs`
- Modify: `src-tauri/src/lib.rs`（加 `mod sys;`）

- [ ] **Step 1: 写失败测试**

创建 `src-tauri/src/sys.rs`：

```rust
//! macOS 系统交互收口层。
//!
//! macOS 的 AppleEvents 全局串行：多个 osascript 子进程并发执行会互相
//! 阻塞（历史上造成多秒死锁）。所有 osascript 调用必须走本模块的专用
//! 串行线程，保证任一时刻至多一个 osascript 在执行。
//!
//! 所有函数都会阻塞调用线程直至结果返回 —— 只在后台线程调用。

use std::sync::OnceLock;
use std::sync::mpsc;

struct Request {
    script: String,
    reply: mpsc::Sender<Result<String, String>>,
}

static QUEUE: OnceLock<mpsc::Sender<Request>> = OnceLock::new();

fn queue() -> mpsc::Sender<Request> {
    QUEUE
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel::<Request>();
            std::thread::Builder::new()
                .name("talkshow-osascript".into())
                .spawn(move || {
                    for req in rx {
                        let result = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(&req.script)
                            .output()
                            .map_err(|e| e.to_string())
                            .and_then(|o| {
                                if o.status.success() {
                                    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
                                } else {
                                    Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                                }
                            });
                        let _ = req.reply.send(result);
                    }
                })
                .expect("failed to spawn osascript worker thread");
            tx
        })
        .clone()
}

/// 串行执行一段 AppleScript，返回 stdout（trim 后）。
pub fn osascript(script: &str) -> Result<String, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    queue()
        .send(Request {
            script: script.to_string(),
            reply: reply_tx,
        })
        .map_err(|e| format!("osascript queue unavailable: {e}"))?;
    reply_rx
        .recv()
        .map_err(|e| format!("osascript worker died: {e}"))?
}

/// 当前最前台应用进程名（如 "Google Chrome"）。
pub fn frontmost_app_name() -> Option<String> {
    osascript(
        "tell application \"System Events\" to get name of first process whose frontmost is true",
    )
    .ok()
    .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn osascript_evaluates_expression() {
        assert_eq!(osascript("return 1 + 1").unwrap(), "2");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn osascript_reports_script_error() {
        assert!(osascript("syntax error here").is_err());
    }

    #[test]
    fn frontmost_app_name_is_plausible() {
        if cfg!(target_os = "macos") {
            let name = frontmost_app_name();
            // 非前台无权限时可能为 None，但不能 panic
            if let Some(n) = name {
                assert!(!n.is_empty());
            }
        }
    }
}
```

- [ ] **Step 2: 挂载模块**

`src-tauri/src/lib.rs` 加：

```rust
mod sys;
```

- [ ] **Step 3: 跑测试**

```bash
cargo test sys:: 
```

Expected: `test result: ok. 3 passed`

- [ ] **Step 4: clippy + fmt + Commit**

```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo fmt
git add src/sys.rs src/lib.rs
git commit -m "feat(sys): add serial osascript executor to eliminate AppleEvents contention"
```

---

### Task 3: 迁移既有 osascript 调用到 sys

**Files:**
- Modify: `src-tauri/src/audio_control.rs`
- Modify: `src-tauri/src/clipboard.rs`
- Modify: `src-tauri/src/skills.rs`

调用语义不变（同步阻塞拿结果），仅执行路径统一。本 Task 不动 lib.rs 内联两处（Task 5 重写时自然消失）。

- [ ] **Step 1: audio_control.rs 迁移**

`get_current_volume` 内替换：

```rust
fn get_current_volume() -> Result<f64, String> {
    let stdout = crate::sys::osascript("output volume of (get volume settings)")?;
    stdout
        .parse::<f64>()
        .map_err(|_| format!("Failed to parse volume: {}", stdout))
}
```

`set_volume` 内替换：

```rust
fn set_volume(volume: f64) -> Result<(), String> {
    let vol = volume.round() as i64;
    crate::sys::osascript(&format!("set volume output volume {}", vol)).map(|_| ())
}
```

注意：`sys::osascript` 已做 stdout/stderr trim，原 `.trim()` 逻辑被吸收。

- [ ] **Step 2: clipboard.rs 迁移**

`simulate_paste` 中 `Command::new("osascript")` 块替换为：

```rust
    match crate::sys::osascript(&script) {
        Ok(_) => {}
        Err(stderr) => {
            if stderr.contains("1002") || stderr.contains("not allowed to send keystrokes") {
                // Surface accessibility permission error clearly
                eprintln!(
                    "[TalkShow] Paste blocked — grant Accessibility permission to this \
                     app in System Settings → Privacy & Security → Accessibility. \
                     Error: {stderr}"
                );
            } else {
                eprintln!("[TalkShow] osascript failed: {stderr}");
            }
        }
    }
```

同时删除文件顶部 `use std::process::Command;`（若仅此函数使用）。

粘贴脚本内的 `delay 0.3` 保留：串行队列中它会延迟后续系统调用 ~300ms，可接受（并发时代的替代品是死锁）。

- [ ] **Step 3: skills.rs 迁移**

`get_frontmost_app`（macOS 分支）替换为：

```rust
#[cfg(target_os = "macos")]
fn get_frontmost_app() -> Result<(String, String), String> {
    let app_name = crate::sys::frontmost_app_name()
        .ok_or_else(|| "Failed to get frontmost app".to_string())?;

    let bundle_id = crate::sys::osascript(&format!(
        "tell application \"System Events\" to get bundle identifier of process \"{}\"",
        escape_applescript_string(&app_name)
    ))
    .unwrap_or_else(|_| "unknown".to_string());

    Ok((app_name, bundle_id))
}
```

- [ ] **Step 4: 编译 + 测试**

```bash
cargo test && cargo clippy --all-targets --all-features -- -D warnings
```

Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add src/audio_control.rs src/clipboard.rs src/skills.rs
git commit -m "refactor(sys): route all osascript calls through serial executor"
```

---

### Task 4: pipeline.rs 会话化

**Files:**
- Modify: `src-tauri/src/pipeline.rs`

核心变更：`stop_recording` 接收 `Session`；全部 `RECORDING`/`CANCELLED` 全局读取换 manager 查询；ESC 清理 10 处重复提取为 `cleanup_esc`；粘贴目标从 `session.target_app` 取。

- [ ] **Step 1: 改签名与导入**

`pipeline.rs` 顶部：

```rust
use crate::session::{Session, SessionManager};
```

删除 `use crate::shortcuts::{CANCELLED, RECORDING, RECORDING_MODE_NONE, RECORDING_MODE_TRANSLATION};`

新增 helper（放在 `SenseVoiceState` 定义后）：

```rust
/// pipeline 结束时清理 ESC 快捷键：仅当没有新的录音会话进行中。
/// （录音开始的后台线程会注册 ESC；有活动会话时不能注销）
fn cleanup_esc(h: &tauri::AppHandle) {
    if !h.state::<SessionManager>().has_active() {
        let _ = h
            .global_shortcut()
            .unregister(Shortcut::new(None, Code::Escape));
    }
}
```

`stop_recording` 签名改为：

```rust
pub fn stop_recording(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<std::sync::Mutex<AudioRecorder>>,
    recording_start: &Arc<std::sync::Mutex<Option<Instant>>>,
    event_name: &str,
    session: Session,
)
```

函数内 `recording_mode` 全部改用 `session.mode`（类型 u8 不变，比较常量改为 `session::MODE_TRANSLATION`）。

- [ ] **Step 2: pipeline async 块状态检查替换**

在 async 块开头（`let pipeline_start = ...` 附近）获取依赖：

```rust
let mgr = h.state::<SessionManager>();
let session = session.clone(); // 供 async 块使用（外层 session 已被 event_name 分支使用）
```

逐点替换（对照行为规格表）：

1. 删除 `CANCELLED.store(false, Ordering::SeqCst);`（原 129 行附近）
2. `if CANCELLED.load(Ordering::SeqCst) {`（AI 请求前检查，原 161 行）→
   ```rust
   if mgr.is_cancelled(session.id) {
   ```
3. `if CANCELLED.load(Ordering::SeqCst) {`（粘贴前检查，原 367 行）→ 同上
4. `if RECORDING.load(Ordering::SeqCst) != RECORDING_MODE_NONE {`（丢弃结果判断，原 ~375 行）→
   ```rust
   if mgr.has_active() {
   ```
5. `let saved_target_app = clipboard::get_target_app();`（原 ~137 行）→
   ```rust
   let saved_target_app = session.target_app.clone();
   ```
6. `if recording_mode == RECORDING_MODE_TRANSLATION {`（两处）→
   ```rust
   if session.mode == crate::session::MODE_TRANSLATION {
   ```
7. 全部 10 处 ESC 清理块：
   ```rust
   if RECORDING.load(Ordering::SeqCst) == RECORDING_MODE_NONE {
       let _ = h.global_shortcut().unregister(Shortcut::new(None, Code::Escape));
   }
   ```
   → 单行 `cleanup_esc(&h);`
   （`"recording:cancel"` / `TooShort` / `Err` 分支里的无条件 unregister 同样换 `cleanup_esc(app_handle);` —— 有活动会话时不注销是行为改进，且与新录音后台线程的注册时序一致：注册前有 `is_active` checkpoint）

- [ ] **Step 3: 编译（此时 lib.rs 仍传旧参数，会报错——预期）**

```bash
cargo check 2>&1 | head -30
```

Expected: 仅 lib.rs 处 stop_recording 调用参数不匹配错误。pipeline.rs 自身无错误。

- [ ] **Step 4: Commit（WIP，Task 5 完成后一起变绿）**

```bash
git add src/pipeline.rs
git commit -m "refactor(pipeline): stop_recording takes Session; state checks via SessionManager"
```

说明：此提交点 `cargo check` 不通过（lib.rs 未迁移），属计划内中间态；Task 5 Step 完成后必须恢复全绿。

---

### Task 5: lib.rs 快捷键 handler 重写

**Files:**
- Modify: `src-tauri/src/lib.rs`（大改）
- Modify: `src-tauri/src/shortcuts.rs`（删全局状态）
- Modify: `src-tauri/src/clipboard.rs`（删 TARGET_APP）

- [ ] **Step 1: shortcuts.rs 瘦身**

删除：`RECORDING` / `CANCELLED` / `SESSION_ID` 三个 static、`RECORDING_MODE_*` 三个常量、引用它们的测试（`test_recording_mode_constants` / `test_session_id_increments`）。

保留：`parse_shortcut` / `ShortcutIds` / `SHORTCUT_IDS` / `LAST_REC_PRESS` 及其余测试。

`use` 清理后顶部：

```rust
use std::sync::Mutex;
use std::time::Instant;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
```

- [ ] **Step 2: clipboard.rs 删全局**

删除：

```rust
static TARGET_APP: Mutex<Option<String>> = Mutex::new(None);

pub fn save_target_app(app_name: &str) { ... }

pub fn get_target_app() -> Option<String> { ... }
```

及顶部 `use std::sync::Mutex;`（若无其他使用）。

- [ ] **Step 3: lib.rs 重写核心区**

3a. 导入区改为：

```rust
use indicator::{
    INDICATOR_LABEL, TRAY_ID, destroy_indicator, restore_default_tray, show_indicator,
};
use pipeline::{SenseVoiceState, play_sound, stop_recording};
use providers::ProviderContext;
use recording::AudioRecorder;
use session::{Session, SessionManager, MODE_TRANSLATION, MODE_TRANSCRIPTION};
use shortcuts::{LAST_REC_PRESS, SHORTCUT_IDS, parse_shortcut};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
```

（`std::sync::atomic::Ordering` 与 `tauri::Listener` 若仍被使用则保留；`commands` 等 pub use 不动。）

3b. setup() 开头（Logger 初始化后）manage manager：

```rust
app.manage(SessionManager::new());
```

3c. 新增两个模块级函数（放在 `format_elapsed` 后）：

```rust
/// 按键去抖：500ms 内重复按压忽略。
fn debounced() -> bool {
    let now = Instant::now();
    LAST_REC_PRESS
        .lock()
        .ok()
        .map(|mut last| {
            if let Some(t) = *last
                && now.duration_since(t) < Duration::from_millis(500)
            {
                return true;
            }
            *last = Some(now);
            false
        })
        .unwrap_or(false)
}

/// 开始一次录音会话（转写/翻译共用）。成功时立即返回（UI 反馈已发出），
/// 副作用在后台线程执行，以 session.id 为 checkpoint。
fn begin_session(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<Mutex<AudioRecorder>>,
    recording_start: &Arc<Mutex<Option<Instant>>>,
    mode: u8,
    esc_shortcut: Shortcut,
    recording_icon: Image,
) {
    let manager = app_handle.state::<SessionManager>();
    let Some(session) = manager.start(mode) else {
        return;
    };

    let app_data_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let started = recorder.lock().ok().and_then(|mut r| {
        r.set_output_dir(app_data_dir);
        r.start().is_ok()
    });

    if !started {
        manager.stop(); // 回滚会话，允许重试
        let err_detail = recorder
            .lock()
            .ok()
            .and_then(|mut r| r.start().err())
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".into());
        eprintln!("[TalkShow] Failed to start recording: {}", err_detail);
        if let Some(logger) = app_handle.try_state::<Logger>() {
            logger.error(
                "recording",
                "录音启动失败",
                Some(serde_json::json!({ "error": err_detail })),
            );
        }
        return;
    }

    // === Phase 1: 立即响应 ===
    if let Ok(mut s) = recording_start.lock() {
        *s = Some(Instant::now());
    }
    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(recording_icon));
    }
    show_indicator(app_handle);
    pipeline::play_sound("Tink.aiff");

    // === Phase 2: 后台副作用（checkpoint = manager.is_active(session.id)） ===
    let h = app_handle.clone();
    std::thread::spawn(move || {
        let mgr = h.state::<SessionManager>();
        let logger = h.try_state::<Logger>();

        if !mgr.is_active(session.id) {
            return;
        }
        if let Some(app_name) = sys::frontmost_app_name() {
            mgr.set_target_app(session.id, app_name);
        }

        if !mgr.is_active(session.id) {
            return;
        }
        let dir = h.path().app_data_dir().unwrap_or_default();
        if config::load_config(&dir).features.recording.auto_mute {
            let _ = audio_control::save_and_mute(&dir, logger.as_deref());
        }

        if !mgr.is_active(session.id) {
            return;
        }
        let _ = h.global_shortcut().register(esc_shortcut);

        if mgr.is_active(session.id)
            && let Some(logger) = logger
        {
            let label = if mode == MODE_TRANSLATION {
                "录音开始 (翻译模式)"
            } else {
                "录音开始"
            };
            logger.info("recording", label, None);
        }
    });
}

/// 结束活动会话并在后台线程执行 stop pipeline。
fn end_session(
    app_handle: &tauri::AppHandle,
    recorder: &Arc<Mutex<AudioRecorder>>,
    recording_start: &Arc<Mutex<Option<Instant>>>,
    event_name: &'static str,
) {
    let manager = app_handle.state::<SessionManager>();
    let Some(session) = manager.stop() else {
        return;
    };
    let h = app_handle.clone();
    let rec = recorder.clone();
    let start = recording_start.clone();
    std::thread::spawn(move || {
        stop_recording(&h, &rec, &start, event_name, session);
    });
    pipeline::play_sound(if event_name == "recording:complete" {
        "Frog.aiff"
    } else {
        "Pop.aiff"
    });
    restore_default_tray(app_handle, app_handle.state::<TrayIcons>().default.clone());
}
```

注意：`restore_default_tray` 需要 `Image` 参数。现有 handler 闭包里靠捕获 `default_icon_owned` 实现。重构时在 setup 里 `app.manage(TrayIcons { default: default_icon_owned.clone(), recording: recording_icon_owned.clone() });`，其中：

```rust
struct TrayIcons {
    default: Image,
    recording: Image,
}
```

（定义放 lib.rs 顶部；`begin_session` 内 `tray.set_icon(Some(recording_icon))` 改从 `app_handle.state::<TrayIcons>().recording.clone()` 取。）

3d. ESC / indicator:cancel 统一处理。`indicator:cancel` 监听器替换为：

```rust
{
    let app_handle_cancel = app.handle().clone();
    let recorder_cancel = recorder.clone();
    let recording_start_cancel = recording_start.clone();
    let esc_cancel = esc_shortcut;
    let _ = app.listen("indicator:cancel", move |_event| {
        let mgr = app_handle_cancel.state::<SessionManager>();
        if let Some(session) = mgr.signal_cancel() {
            // 录音中取消
            let h = app_handle_cancel.clone();
            let rec = recorder_cancel.clone();
            let start = recording_start_cancel.clone();
            std::thread::spawn(move || {
                stop_recording(&h, &rec, &start, "recording:cancel", session);
            });
        }
        destroy_indicator(&app_handle_cancel);
        pipeline::play_sound("Pop.aiff");
        let h = app_handle_cancel.clone();
        let esc = esc_cancel;
        std::thread::spawn(move || {
            let _ = h.global_shortcut().unregister(esc);
        });
        restore_default_tray(
            &app_handle_cancel,
            app_handle_cancel.state::<TrayIcons>().default.clone(),
        );
    });
}
```

3e. 快捷键 handler 主体（替换原 ~400 行闭包内 transcription/translation 两大分支与 ESC 分支）：

```rust
.with_handler(move |_app, shortcut, event| {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let id = shortcut.id();
    let (current_toggle_id, current_rec_id, current_translate_id) = {
        let ids = SHORTCUT_IDS.read().unwrap();
        (ids.toggle, ids.recording, ids.translate)
    };

    // --- ESC ---
    if id == esc_id {
        let mgr = app_handle.state::<SessionManager>();
        match mgr.signal_cancel() {
            Some(session) => {
                // 录音中取消
                let h = app_handle.clone();
                let rec = recorder_handler.clone();
                let start = recording_start_handler.clone();
                std::thread::spawn(move || {
                    stop_recording(&h, &rec, &start, "recording:cancel", session);
                });
                pipeline::play_sound("Pop.aiff");
            }
            None if app_handle.get_webview_window(INDICATOR_LABEL).is_some() => {
                // 处理中取消：signal_cancel 已标记 last_finished
                destroy_indicator(&app_handle);
                pipeline::play_sound("Pop.aiff");
            }
            None => return,
        }
        let h = app_handle.clone();
        let esc = esc_shortcut_handler;
        std::thread::spawn(move || {
            let _ = h.global_shortcut().unregister(esc);
        });
        restore_default_tray(&app_handle, app_handle.state::<TrayIcons>().default.clone());
        return;
    }

    // --- 录音键（转写/翻译合并） ---
    let mode = if current_rec_id != 0 && id == current_rec_id {
        MODE_TRANSCRIPTION
    } else if current_translate_id != 0 && id == current_translate_id {
        MODE_TRANSLATION
    } else if current_toggle_id != 0 && id == current_toggle_id {
        if let Some(window) = app_handle.get_webview_window("main") {
            toggle_window(&window);
        }
        return;
    } else {
        return;
    };

    if debounced() {
        return;
    }

    let mgr = app_handle.state::<SessionManager>();
    if mgr.has_active() {
        end_session(&app_handle, &recorder_handler, &recording_start_handler, "recording:complete");
    } else {
        let icons = app_handle.state::<TrayIcons>();
        begin_session(
            &app_handle,
            &recorder_handler,
            &recording_start_handler,
            mode,
            esc_shortcut_handler,
            icons.recording.clone(),
        );
    }
})
```

3f. tooltip 轮询线程中 `RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE` → `h.state::<SessionManager>().has_active()`（保留 200ms 轮询，`h` 为已 clone 的 app_handle）。

- [ ] **Step 4: 全量验证**

```bash
cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt
cd .. && npm run check && npm test
```

Expected: 全绿。`grep -rn "SESSION_ID\|CANCELLED\|RECORDING_MODE_\|save_target_app\|get_target_app" src-tauri/src --include="*.rs"` 返回空。

- [ ] **Step 5: 手工冒烟（关键——竞态修复的核心验证）**

```bash
npm run tauri:dev
```

逐项验证行为规格表 13 条。重点：
1. 按 rec 键开始 → 浮窗即时出现（<100ms）→ 再按 → 转写粘贴
2. 录音中按 ESC → Pop 音 → 无转写
3. 处理中（浮窗显示处理中时）按 ESC → 浮窗消失、无粘贴
4. 录音中按 translate 键 → 按 translate 停止 → 翻译粘贴
5. 开启自动静音 → 录音 → 停止 → 音量恢复（原 Race A 场景）
6. 快速连按 rec 键 3 次（<500ms 间隔）→ 防抖生效

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(core): rewrite shortcut handler on SessionManager, remove global session atomics

- RECORDING/SESSION_ID/CANCELLED/TARGET_APP 全局状态删除
- 转写/翻译重复分支合并为 begin_session(mode)
- 4 条 stop 路径统一经 manager 状态转移
- osascript 内联调用改走 sys 串行执行器
- 修复 TARGET_APP 跨会话串扰"
```

---

### Task 6: AI 客户端文件合并

**Files:**
- Modify: `src-tauri/src/llm_client.rs`（吸收 real_llm_client.rs）
- Delete: `src-tauri/src/real_llm_client.rs`
- Modify: 引用处

- [ ] **Step 1: 合并**

`real_llm_client.rs` 全文（`use` 之后的 `RealLlmClient` 定义与 impl）移入 `llm_client.rs`，调整 use：

```rust
use crate::ai::{ThinkingMode, send_audio_prompt_from_bytes, send_text_prompt};
use crate::config::ProviderConfig;
use crate::logger::Logger;
use crate::providers::ProviderContext;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)]
pub trait LlmClient: Send + Sync {
    // ... 原 trait 定义不变 ...
}

pub struct RealLlmClient<'a> {
    logger: &'a Logger,
    ctx: &'a ProviderContext,
}

// ... 原 real_llm_client.rs 的 impl 原样移入 ...
```

删除 `real_llm_client.rs`，lib.rs 删 `mod real_llm_client;`。

- [ ] **Step 2: 全局替换引用**

```bash
grep -rn "real_llm_client" src-tauri/src src-tauri/tests
```

对每处把 `real_llm_client::RealLlmClient` 改为 `llm_client::RealLlmClient`（含测试文件）。

- [ ] **Step 3: 验证 + Commit**

```bash
cargo test && cargo clippy --all-targets --all-features -- -D warnings
git add -A && git commit -m "refactor(ai): merge RealLlmClient into llm_client.rs"
```

---

### Task 7: 工程清理

**Files:**
- Modify: `README.md`
- 删除残留目录

- [ ] **Step 1: README 架构节更新**

「技术架构」与「项目结构」两节替换为反映现状的版本：`session.rs`（会话状态机）、`sys.rs`（系统调用串行层）、`providers/` 目录、`config/` 目录、`sensevoice/` 目录。数据流图更新为：

```
快捷键 → SessionManager(状态机) → pipeline(编排)
                                     ├─ providers/(转写/润色/翻译)
                                     ├─ skills/(后处理)
                                     └─ sys/(osascript 串行: 静音/粘贴/前台应用)
```

- [ ] **Step 2: 残留清理（主仓库，非 worktree）**

```bash
cd /Users/jia.xia/development/talkshow
rm -rf TalkFlow .venv .ruff_cache coverage .build
git status   # 确认这些目录未被 git 追踪（应无变更；若被追踪则 git rm 并提交）
```

`TalkFlow/`（空 Xcode 工程）删除前知会用户一次。

- [ ] **Step 3: 分支清理**

```bash
git branch --merged main | grep -v "^\*\|main"   # 已合并的本地分支
git branch -d <每个已合并分支>
git fetch --prune
```

未合并分支（`feature/v2`、`feat/chat-design` 等）**不删**，列清单交用户决策。

- [ ] **Step 4: Commit（若有 README 变更）**

```bash
git add README.md && git commit -m "docs: update architecture to session-manager design"
```

---

### Task 8: 合并回 main

- [ ] **Step 1: 终验**

worktree 内：

```bash
npm run ci   # check + test + lint:rust + test:rust
```

- [ ] **Step 2: 合并**

```bash
cd /Users/jia.xia/development/talkshow
git checkout main && git pull
git merge --no-ff refactor/session-architecture
git worktree remove .worktrees/refactor-session-arch
git branch -d refactor/session-architecture
```

- [ ] **Step 3: 发布构建验证**

```bash
npm run tauri:build
```

Expected: dmg 产出成功。

---

## 风险与回滚

| 风险 | 缓解 |
|------|------|
| Task 5 改动面大，引入新竞态 | 行为规格表 13 条逐条冒烟；每 Task 独立提交可单点 revert |
| Task 4 提交点编译不过 | 计划内中间态，Task 5 恢复；若需中断可用 `git rebase -i` 合并 4+5 |
| 串行 osascript 引入排队延迟 | 上限 = 队列长度 × 单次耗时；录音开始只有 2 次调用（前台应用+静音），排队 <300ms，远优于死锁 |
| signal_cancel 语义遗漏路径 | handler 中 ESC/indicator:cancel/rec/translate 四入口全部走 manager，无旁路；`grep SESSION_ID` 为空即证 |

## 明确不做（YAGNI）

- 不换前台应用检测为 CGWindowListCopyWindowInfo 原生 API（需 objc 绑定依赖；串行队列已解决死锁，延迟 ~50ms 可接受。真需要时在 `sys.rs` 单点替换）
- 不动前端（健康）
- 不动 IPC 契约（commands.rs 签名零变化）
- 不改 zhipu provider（用户确认保留，未来单独调整 API 格式）
- 不引入 tokio 重构录音线程模型（std thread 足够，行为已验证）
