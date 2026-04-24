# Shortcut Freeze Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate app freeze caused by blocking calls in the shortcut callback thread and racing background threads from start/stop recording interactions across any shortcut key combination.

**Architecture:** Two coordinated changes — (1) a `SESSION_ID` atomic counter that lets any background thread detect it has been superseded and exit early, and (2) moving `stop_recording` out of the shortcut callback into a spawned thread so the callback returns in < 1ms.

**Tech Stack:** Rust, Tauri 2, `std::sync::atomic`, `std::thread::spawn`

---

## File Map

| File | Change |
|------|--------|
| `src-tauri/src/shortcuts.rs` | Add `static SESSION_ID: AtomicU64` and export it |
| `src-tauri/src/lib.rs` | Increment session on start/stop; add checkpoints to both background threads; spawn stop_recording; add getApp to translation background thread |
| `src-tauri/tests/shortcut_session.rs` | New integration tests for session ID logic |

---

## Task 1: Add `SESSION_ID` to `shortcuts.rs`

**Files:**
- Modify: `src-tauri/src/shortcuts.rs`
- Test: `src-tauri/src/shortcuts.rs` (inline unit test)

- [ ] **Step 1: Add `AtomicU64` import and `SESSION_ID` static**

In `src-tauri/src/shortcuts.rs`, change the first `use` line and add the new static:

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8};
use std::sync::{Mutex, RwLock};
use std::time::Instant;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub const RECORDING_MODE_NONE: u8 = 0;
pub const RECORDING_MODE_TRANSCRIPTION: u8 = 1;
pub const RECORDING_MODE_TRANSLATION: u8 = 2;

pub static RECORDING: AtomicU8 = AtomicU8::new(RECORDING_MODE_NONE);
pub static CANCELLED: AtomicBool = AtomicBool::new(false);
pub static SESSION_ID: AtomicU64 = AtomicU64::new(0);
pub static LAST_REC_PRESS: Mutex<Option<Instant>> = Mutex::new(None);
```

- [ ] **Step 2: Add unit test for SESSION_ID**

At the bottom of the `#[cfg(test)]` block in `shortcuts.rs`, add:

```rust
    #[test]
    fn test_session_id_increments() {
        use std::sync::atomic::Ordering;
        let before = SESSION_ID.load(Ordering::SeqCst);
        SESSION_ID.fetch_add(1, Ordering::SeqCst);
        let after = SESSION_ID.load(Ordering::SeqCst);
        assert_eq!(after, before + 1);
    }
```

- [ ] **Step 3: Run tests**

```bash
cd src-tauri && cargo test test_session_id_increments -- --nocapture
```

Expected output: `test test_session_id_increments ... ok`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shortcuts.rs
git commit -m "feat: add SESSION_ID atomic counter to shortcuts"
```

---

## Task 2: Increment Session on Recording Stop (All Stop Paths)

**Files:**
- Modify: `src-tauri/src/lib.rs`

Every code path that sets `RECORDING = RECORDING_MODE_NONE` must first increment `SESSION_ID`. There are four places:

- [ ] **Step 1: Add `SESSION_ID` to imports in `lib.rs`**

Find the shortcuts import line (around line 37):
```rust
use shortcuts::{
    CANCELLED, LAST_REC_PRESS, RECORDING, RECORDING_MODE_NONE, RECORDING_MODE_TRANSCRIPTION,
    RECORDING_MODE_TRANSLATION, SHORTCUT_IDS, parse_shortcut,
};
```
Change to:
```rust
use shortcuts::{
    CANCELLED, LAST_REC_PRESS, RECORDING, RECORDING_MODE_NONE, RECORDING_MODE_TRANSCRIPTION,
    RECORDING_MODE_TRANSLATION, SESSION_ID, SHORTCUT_IDS, parse_shortcut,
};
```

- [ ] **Step 2: ESC key handler — add session increment**

Find the ESC handler block (around line 228–250). It currently reads:
```rust
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
```
Change to:
```rust
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    SESSION_ID.fetch_add(1, Ordering::SeqCst);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
```

- [ ] **Step 3: Transcription key stop path — add session increment**

Find the transcription shortcut's `is_recording` branch (around line 270–285). It reads:
```rust
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
        &app_handle,
        &recorder_handler,
        &recording_start_handler,
        "recording:complete",
        mode,
    );
    play_sound("Frog.aiff");
    restore_default_tray(&app_handle, default_icon_owned.clone());
```
Change to:
```rust
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    SESSION_ID.fetch_add(1, Ordering::SeqCst);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
        &app_handle,
        &recorder_handler,
        &recording_start_handler,
        "recording:complete",
        mode,
    );
    play_sound("Frog.aiff");
    restore_default_tray(&app_handle, default_icon_owned.clone());
```

- [ ] **Step 4: Translation key stop path — add session increment**

Find the translation shortcut's `is_recording` branch (around line 400–415). Same pattern:
```rust
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
        &app_handle,
        &recorder_handler,
        &recording_start_handler,
        "recording:complete",
        mode,
    );
    play_sound("Frog.aiff");
    restore_default_tray(&app_handle, default_icon_owned.clone());
```
Change to:
```rust
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    SESSION_ID.fetch_add(1, Ordering::SeqCst);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
        &app_handle,
        &recorder_handler,
        &recording_start_handler,
        "recording:complete",
        mode,
    );
    play_sound("Frog.aiff");
    restore_default_tray(&app_handle, default_icon_owned.clone());
```

- [ ] **Step 5: `indicator:cancel` event listener — add session increment**

Find the `indicator:cancel` listener (around line 185–207). It reads:
```rust
let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
```
Change to:
```rust
let is_recording = RECORDING.load(Ordering::Relaxed) != RECORDING_MODE_NONE;
if is_recording {
    let mode = RECORDING.load(Ordering::Relaxed);
    SESSION_ID.fetch_add(1, Ordering::SeqCst);
    RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
    stop_recording(
```

- [ ] **Step 6: Compile check**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output (no errors).

- [ ] **Step 7: Run all tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all existing tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: increment SESSION_ID on all recording stop paths"
```

---

## Task 3: Add Session Checkpoints to Transcription Background Thread

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Capture session snapshot when transcription recording starts**

In the transcription key's `Ok(_)` start branch (inside `compare_exchange` success), after `show_indicator`:

Find the line just before `std::thread::spawn(move || {` in the transcription start path. Add snapshot capture:

```rust
// Show indicator immediately
show_indicator(&app_handle);
play_sound("Tink.aiff");

// === Phase 2: Background async operations (~300ms) ===
let app_handle_bg = app_handle.clone();
let esc_bg = esc_shortcut_handler;
let session_snapshot = SESSION_ID.fetch_add(1, Ordering::SeqCst) + 1;

std::thread::spawn(move || {
```

Note: `fetch_add` returns the old value, so `+ 1` gives the new (current) value.

- [ ] **Step 2: Add checkpoints inside the transcription background thread**

Replace the entire body of the transcription background thread spawn with:

```rust
std::thread::spawn(move || {
    use std::sync::atomic::Ordering;

    // Checkpoint 1: get frontmost app
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    let frontmost = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if let Some(ref app) = frontmost {
        clipboard::save_target_app(app);
    }

    // Checkpoint 2: auto mute
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    let app_data_dir_mute = app_handle_bg.path().app_data_dir().unwrap_or_default();
    let app_config_mute = config::load_config(&app_data_dir_mute);
    if app_config_mute.features.recording.auto_mute {
        let _ = audio_control::save_and_mute(
            &app_data_dir_mute,
            app_handle_bg.try_state::<Logger>().as_deref(),
        );
    }

    // Checkpoint 3: register ESC
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    let h = app_handle_bg.clone();
    let _ = h.global_shortcut().register(esc_bg);

    // Checkpoint 4: log
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    if let Some(logger) = app_handle_bg.try_state::<Logger>() {
        logger.info("recording", "录音开始", None);
    }
});
```

- [ ] **Step 3: Compile check**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 4: Run all tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: add session checkpoints to transcription background thread"
```

---

## Task 4: Add Session Checkpoints + getApp to Translation Background Thread

**Files:**
- Modify: `src-tauri/src/lib.rs`

The translation background thread currently skips the `getApp` step, leaving `TARGET_APP` stale when switching between shortcuts.

- [ ] **Step 1: Capture session snapshot when translation recording starts**

In the translation key's `Ok(_)` start branch, after `play_sound("Tink.aiff")`, add:

```rust
// Show indicator immediately
show_indicator(&app_handle);
play_sound("Tink.aiff");

// === Phase 2: Background async operations ===
let app_handle_bg = app_handle.clone();
let esc_bg = esc_shortcut_handler;
let session_snapshot = SESSION_ID.fetch_add(1, Ordering::SeqCst) + 1;

std::thread::spawn(move || {
```

- [ ] **Step 2: Replace translation background thread body with checkpointed version including getApp**

```rust
std::thread::spawn(move || {
    use std::sync::atomic::Ordering;

    // Checkpoint 1: get frontmost app (was missing in translation mode)
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    let frontmost = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if let Some(ref app) = frontmost {
        clipboard::save_target_app(app);
    }

    // Checkpoint 2: auto mute
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    let app_data_dir_mute = app_handle_bg.path().app_data_dir().unwrap_or_default();
    let app_config_mute = config::load_config(&app_data_dir_mute);
    if app_config_mute.features.recording.auto_mute {
        let _ = audio_control::save_and_mute(
            &app_data_dir_mute,
            app_handle_bg.try_state::<Logger>().as_deref(),
        );
    }

    // Checkpoint 3: register ESC
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    let h = app_handle_bg.clone();
    let _ = h.global_shortcut().register(esc_bg);

    // Checkpoint 4: log
    if SESSION_ID.load(Ordering::SeqCst) != session_snapshot {
        return;
    }
    if let Some(logger) = app_handle_bg.try_state::<Logger>() {
        logger.info("recording", "录音开始 (翻译模式)", None);
    }
});
```

- [ ] **Step 3: Compile check**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 4: Run all tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: add session checkpoints and getApp to translation background thread"
```

---

## Task 5: Move `stop_recording` Out of Shortcut Callback (Spawn It)

**Files:**
- Modify: `src-tauri/src/lib.rs`

This is the change that prevents FLAC encoding and `osascript restore` from blocking the shortcut callback thread (main thread).

There are four stop call sites: ESC handler, transcription stop, translation stop, and `indicator:cancel`. The `indicator:cancel` listener already runs on a separate thread (Tauri's event system), so it is lower priority — but we apply the same pattern for consistency.

- [ ] **Step 1: Spawn stop in ESC handler**

Find the ESC stop call (after the `SESSION_ID.fetch_add` we added in Task 2):

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
stop_recording(
    &app_handle,
    &recorder_handler,
    &recording_start_handler,
    "recording:cancel",
    mode,
);
play_sound("Pop.aiff");
```

Change to:

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
let app_stop = app_handle.clone();
let recorder_stop = recorder_handler.clone();
let rec_start_stop = recording_start_handler.clone();
std::thread::spawn(move || {
    stop_recording(&app_stop, &recorder_stop, &rec_start_stop, "recording:cancel", mode);
});
play_sound("Pop.aiff");
```

- [ ] **Step 2: Spawn stop in transcription key stop path**

Find the transcription stop call (after `SESSION_ID.fetch_add` from Task 2):

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
stop_recording(
    &app_handle,
    &recorder_handler,
    &recording_start_handler,
    "recording:complete",
    mode,
);
play_sound("Frog.aiff");
restore_default_tray(&app_handle, default_icon_owned.clone());
```

Change to:

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
let app_stop = app_handle.clone();
let recorder_stop = recorder_handler.clone();
let rec_start_stop = recording_start_handler.clone();
std::thread::spawn(move || {
    stop_recording(&app_stop, &recorder_stop, &rec_start_stop, "recording:complete", mode);
});
play_sound("Frog.aiff");
restore_default_tray(&app_handle, default_icon_owned.clone());
```

- [ ] **Step 3: Spawn stop in translation key stop path**

Same pattern as Step 2, for the translation stop call site:

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
let app_stop = app_handle.clone();
let recorder_stop = recorder_handler.clone();
let rec_start_stop = recording_start_handler.clone();
std::thread::spawn(move || {
    stop_recording(&app_stop, &recorder_stop, &rec_start_stop, "recording:complete", mode);
});
play_sound("Frog.aiff");
restore_default_tray(&app_handle, default_icon_owned.clone());
```

- [ ] **Step 4: Spawn stop in `indicator:cancel` listener**

Find the `indicator:cancel` stop call:

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
stop_recording(
    &app_handle_cancel,
    &recorder_cancel,
    &recording_start_cancel,
    "recording:cancel",
    mode,
);
```

Change to:

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
let app_stop = app_handle_cancel.clone();
let recorder_stop = recorder_cancel.clone();
let rec_start_stop = recording_start_cancel.clone();
std::thread::spawn(move || {
    stop_recording(&app_stop, &recorder_stop, &rec_start_stop, "recording:cancel", mode);
});
```

- [ ] **Step 5: Compile check**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output.

- [ ] **Step 6: Run all tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: spawn stop_recording off main thread to prevent UI freeze"
```

---

## Task 6: Verify with Manual Test Scenarios

- [ ] **Scenario 1 — Same key double press (fast, < 500ms)**
  1. Press transcription shortcut
  2. Immediately press it again (< 500ms)
  3. Expected: second press is debounced, no pipeline fires, no mute leak, app remains responsive

- [ ] **Scenario 2 — Same key double press (slow, > 500ms)**
  1. Press transcription shortcut (recording starts)
  2. Wait 600ms, press again (stop + pipeline starts)
  3. Immediately press again (< 500ms after stop) — debounced
  4. Expected: one pipeline runs, app responsive throughout

- [ ] **Scenario 3 — Transcription then translation**
  1. Press transcription shortcut (recording starts)
  2. Press translation shortcut (stops transcription, does NOT start translation)
  3. Press translation shortcut again (starts translation recording)
  4. Press translation shortcut again (stops, pipeline fires with translation mode)
  5. Expected: paste occurs in correct app, system not muted after

- [ ] **Scenario 4 — ESC before background thread registers it**
  1. Press transcription shortcut
  2. Immediately press ESC (before background thread has registered it)
  3. Expected: recording cancelled (or cancel has no effect), no permanent ESC registration

- [ ] **Scenario 5 — Rapid stop then start**
  1. Press transcription shortcut (start)
  2. Press it again after 600ms (stop — pipeline starts)
  3. Immediately press it again (start new recording while pipeline running)
  4. Expected: first pipeline sees `RECORDING != NONE`, discards its result; second recording proceeds normally

- [ ] **Final commit**

```bash
git add -A
git commit -m "test: verify freeze fix manual scenarios"
```
