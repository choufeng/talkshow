# Design: Fix Shortcut-Triggered App Freeze

**Date**: 2026-04-25  
**Status**: Approved

---

## Problem

The app freezes (假死) when any shortcut key triggers `stop_recording` while a background thread from a prior `start_recording` is still running. This is not limited to pressing the same key twice — pressing different shortcut keys (e.g., transcription then translation, or ESC at the wrong time) can trigger the same failure.

### Root Cause 1: `stop_recording` Blocks the Shortcut Callback Thread

`stop_recording` is called synchronously inside the global shortcut callback. It contains blocking operations:

- `audio_control::restore()` → spawns `osascript` process, waits for it (~200ms)
- `recorder.lock()` + `r.stop()` → FLAC audio encoding (1–3 seconds for long recordings)

If the shortcut callback runs on the main thread (which it does in Tauri's global shortcut plugin), these blocking calls freeze the entire UI.

Any shortcut that triggers stop is affected: the transcription key (2nd press), the translation key (when recording is active), and the ESC key.

### Root Cause 2: Background Thread from `start_recording` Has No Cancellation Signal

When recording starts, a background thread is spawned to perform side effects:
1. `osascript` → get frontmost app name
2. `audio_control::save_and_mute()` → two more `osascript` calls
3. `global_shortcut().register(ESC)`
4. Log

This background thread has no awareness that recording was stopped. If the shortcut is pressed again (same or different key) before the thread finishes, the following races occur:

**Race A — Restore-before-Save (permanent mute)**:  
`stop_recording` calls `restore()` before the background thread has written `mute_state.json`. Restore finds no file and returns OK. The background thread then writes the file and mutes the system permanently.

**Race B — Concurrent osascript processes (System Events deadlock)**:  
Background thread (start) and pipeline (paste) each spawn multiple `osascript` processes concurrently. macOS serializes AppleScript via AppleEvents, so contention on `System Events.app` causes multi-second hangs.

**Race C — ESC shortcut leak**:  
If `stop_recording` unregisters ESC before the background thread registers it, the ESC registration happens after cleanup and remains permanently active. Subsequent ESC presses trigger unexpected cancel behavior.

**Race D — Stale target app for paste**:  
The translation shortcut's background thread does not call `getApp` (unlike the transcription shortcut). Switching between shortcuts leaves `TARGET_APP` stale, causing paste to go to the wrong application.

---

## Solution: Session ID + Async Stop

Two changes, working together:

### Change 1: Session ID for Background Thread Cancellation

Add a global atomic counter `SESSION_ID` to `shortcuts.rs`:

```rust
pub static SESSION_ID: AtomicU64 = AtomicU64::new(0);
```

**On recording start (both transcription and translation)**:  
Increment `SESSION_ID` and capture the new value as `snapshot`. The background thread checks `SESSION_ID == snapshot` before each side-effect step. If they differ, it returns immediately without executing the step.

```rust
let snapshot = SESSION_ID.fetch_add(1, Ordering::SeqCst) + 1;

std::thread::spawn(move || {
    // Checkpoint 1: get frontmost app
    if SESSION_ID.load(Ordering::SeqCst) != snapshot { return; }
    let frontmost = /* osascript */;
    clipboard::save_target_app(...);

    // Checkpoint 2: auto mute
    if SESSION_ID.load(Ordering::SeqCst) != snapshot { return; }
    audio_control::save_and_mute(...);

    // Checkpoint 3: register ESC
    if SESSION_ID.load(Ordering::SeqCst) != snapshot { return; }
    h.global_shortcut().register(esc);

    // Checkpoint 4: log
    if SESSION_ID.load(Ordering::SeqCst) != snapshot { return; }
    logger.info(...);
});
```

**On recording stop or cancel (all code paths)**:  
Increment `SESSION_ID` before changing `RECORDING` state. This ensures the background thread's next checkpoint fails and it exits without executing further side effects.

```rust
SESSION_ID.fetch_add(1, Ordering::SeqCst);  // invalidate background thread
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
// then proceed with stop
```

Apply to all stop paths:
- Transcription key (2nd press)
- Translation key (when is_recording)
- ESC key handler
- `indicator:cancel` event listener

### Change 2: Move `stop_recording` into a Spawned Thread

Instead of calling `stop_recording` synchronously in the shortcut callback, spawn a thread for it. The callback returns immediately after setting state.

```rust
// In shortcut callback (main thread) — returns in < 1ms:
SESSION_ID.fetch_add(1, Ordering::SeqCst);
RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
let mode = /* current mode */;
let app = app_handle.clone();
let recorder = recorder_handler.clone();
let rec_start = recording_start_handler.clone();
std::thread::spawn(move || {
    stop_recording(&app, &recorder, &rec_start, "recording:complete", mode);
});
play_sound("Frog.aiff");         // non-blocking
restore_default_tray(...);       // non-blocking
```

The sound and tray update happen immediately (good UX). The slow work (osascript restore, FLAC encode, pipeline) happens off the main thread.

### Change 3: Add `getApp` to Translation Shortcut Background Thread

The translation shortcut's background thread currently skips the frontmost-app step. Add it, matching the transcription shortcut's behavior, so that `TARGET_APP` is always fresh when recording starts.

---

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/shortcuts.rs` | Add `static SESSION_ID: AtomicU64` |
| `src-tauri/src/lib.rs` | Increment session on start and stop; add checkpoints in background threads; spawn `stop_recording` instead of calling it inline; add `getApp` to translation background thread |
| `src-tauri/src/pipeline.rs` | No change |
| `src-tauri/src/audio_control.rs` | No change |

**Estimated new code**: ~30 lines added, ~10 lines restructured. No new dependencies.

---

## Race Condition Coverage

| Race | Before | After |
|------|--------|-------|
| Restore-before-Save (permanent mute) | ❌ mute step skipped by checkpoint | ✅ |
| Concurrent osascript / System Events hang | ❌ multiple processes pile up | ✅ background thread exits early |
| ESC shortcut leak | ❌ ESC registered after cleanup | ✅ checkpoint before register(ESC) |
| Stale target app for paste | ❌ translation skips getApp | ✅ getApp added to translation thread |
| UI freeze on stop (FLAC encode / osascript) | ❌ blocks shortcut callback thread | ✅ stop runs in spawned thread |

---

## What Is Not Changed

- Debounce logic (`LAST_REC_PRESS`, 500ms) — kept as-is, complementary protection
- `pipeline.rs` async logic — unchanged
- `audio_control.rs` — unchanged
- ESC unregistration in pipeline — unchanged (still correct, as ESC will only be registered if checkpoint 3 passed)
