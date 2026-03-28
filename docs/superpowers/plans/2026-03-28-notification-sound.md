# 录音提示音 & ESC 键修复 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为录音的开始/停止/取消添加系统提示音反馈，并修复 ESC 键全局拦截问题。

**Architecture:** 在 Rust 后端新增 `play_sound()` 函数，通过 `afplay` 播放 macOS 系统音效；将 ESC 快捷键从"始终注册"改为"仅录音期间动态注册/注销"。

**Tech Stack:** Rust, tauri-plugin-global-shortcut, macOS afplay

---

### Task 1: 新增 play_sound 函数

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 `show_notification` 函数后新增 `play_sound` 函数**

在 `lib.rs` 第 169 行（`show_notification` 函数结束后）添加：

```rust
fn play_sound(sound_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let sound_path = format!("/System/Library/Sounds/{}", sound_name);
        std::thread::spawn(move || {
            let _ = std::process::Command::new("afplay")
                .arg(&sound_path)
                .spawn();
        });
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过，无错误

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add play_sound function for macOS system sounds"
```

---

### Task 2: 在录音流程中添加提示音

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 开始录音成功后播放 Tink 提示音**

在 handler 中开始录音成功的 `Some(()) => {` 块（约第 295 行）的末尾，`tray.set_icon` 之后添加 `play_sound("Tink.aiff");`：

将第 295-303 行：
```rust
                                    Some(()) => {
                                        RECORDING.store(true, Ordering::Relaxed);
                                        if let Ok(mut start) = recording_start_handler.lock() {
                                            *start = Some(Instant::now());
                                        }
                                        if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                            let _ =
                                                tray.set_icon(Some(recording_icon_owned.clone()));
                                        }
                                    }
```

改为：
```rust
                                    Some(()) => {
                                        RECORDING.store(true, Ordering::Relaxed);
                                        if let Ok(mut start) = recording_start_handler.lock() {
                                            *start = Some(Instant::now());
                                        }
                                        if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
                                            let _ =
                                                tray.set_icon(Some(recording_icon_owned.clone()));
                                        }
                                        play_sound("Tink.aiff");
                                    }
```

- [ ] **Step 2: 停止录音后播放 Tink 提示音**

在 handler 中停止录音的 `stop_recording` 调用之后、`restore_default_tray` 调用之前添加提示音。

将第 276-283 行：
```rust
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                );
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            }
```

改为：
```rust
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:complete",
                                );
                                play_sound("Tink.aiff");
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            }
```

- [ ] **Step 3: 取消录音后播放 Pop 提示音**

将第 261-271 行：
```rust
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:cancel",
                                );
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            }
```

改为：
```rust
                            let is_recording = RECORDING.load(Ordering::Relaxed);
                            if is_recording {
                                stop_recording(
                                    &app_handle,
                                    &recorder_handler,
                                    &recording_start_handler,
                                    "recording:cancel",
                                );
                                play_sound("Pop.aiff");
                                restore_default_tray(&app_handle, default_icon_owned.clone());
                            }
```

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: play notification sounds on recording start/stop/cancel"
```

---

### Task 3: ESC 键动态注册/注销

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 移除应用启动时 ESC 的初始注册**

删除第 337-339 行：
```rust
            if let Err(e) = app.global_shortcut().register(esc_shortcut) {
                eprintln!("Failed to register escape shortcut: {}", e);
            }
```

- [ ] **Step 2: 在 handler 闭包中 clone esc_shortcut**

在第 251 行 `let recorder_handler = recorder.clone();` 之后添加：
```rust
            let esc_shortcut_handler = esc_shortcut.clone();
```

- [ ] **Step 3: 开始录音时注册 ESC**

在 handler 中开始录音成功后（`play_sound("Tink.aiff");` 之后）添加 ESC 注册：

```rust
                                        play_sound("Tink.aiff");
                                        let _ = app_handle.global_shortcut().register(esc_shortcut_handler.clone());
```

- [ ] **Step 4: 停止录音时注销 ESC**

在 handler 中停止录音的 `play_sound("Tink.aiff");` 之后添加 ESC 注销：

```rust
                                play_sound("Tink.aiff");
                                let _ = app_handle.global_shortcut().unregister(esc_shortcut_handler.clone());
```

- [ ] **Step 5: 取消录音时注销 ESC**

在 handler 中取消录音的 `play_sound("Pop.aiff");` 之后添加 ESC 注销：

```rust
                                play_sound("Pop.aiff");
                                let _ = app_handle.global_shortcut().unregister(esc_shortcut_handler.clone());
```

- [ ] **Step 6: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix: dynamically register/unregister ESC shortcut only during recording"
```

---

### Task 4: 构建并手动验证

- [ ] **Step 1: 构建应用**

Run: `cd src-tauri && cargo build`

- [ ] **Step 2: 验证清单**

1. 启动应用后，在其他应用中按 ESC 能正常使用（如关闭对话框）
2. 按录音快捷键开始录音 → 听到 "Tink" 提示音
3. 录音期间按 ESC → 听到 "Pop" 提示音，录音取消
4. 取消后，在其他应用中按 ESC 能正常使用
5. 按录音快捷键开始录音 → 听到 "Tink" 提示音
6. 再次按录音快捷键停止录音 → 听到 "Tink" 提示音，录音完成
7. 停止后，在其他应用中按 ESC 能正常使用
