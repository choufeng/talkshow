# 录音悬浮状态浮窗实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现一个始终悬浮在屏幕最上层的小型药丸浮窗，覆盖「录音中」和「AI 处理中」两种状态，支持取消/中止操作。

**Architecture:** Rust 后端通过 `WebviewWindowBuilder` 动态创建独立的无边框 WebviewWindow（label: `recording-indicator`），Svelte 页面渲染 UI。后端通过 `emit_to` 向浮窗定向发送状态事件，浮窗通过 `emit` 向后端发送取消/中止事件。浮窗创建、状态切换、销毁全由 Rust 后端驱动。

**Tech Stack:** Tauri v2 (WebviewWindowBuilder, emit_to/emit, EventTarget), Svelte 5, CSS keyframes/animations

**Spec:** `docs/superpowers/specs/2026-03-30-recording-float-indicator-design.md`

---

## Task 1: 后端 — 浮窗窗口管理与事件集成

**Files:**
- Modify: `src-tauri/src/lib.rs`

**目标：** 在 Rust 后端中实现浮窗的创建、状态事件发射、取消/中止处理、中断机制。

- [ ] **Step 1: 添加浮窗窗口创建辅助函数**

在 `lib.rs` 中添加导入和辅助函数。在文件顶部已有的 `use tauri::{image::Image, Emitter, Manager, WebviewWindow};` 行中添加 `WebviewWindowBuilder`：

```rust
use tauri::{image::Image, Emitter, Manager, WebviewWindow, WebviewWindowBuilder};
```

在 `restore_default_tray` 函数之后（约第 270 行），添加浮窗管理函数：

```rust
const INDICATOR_LABEL: &str = "recording-indicator";

fn show_indicator(app_handle: &tauri::AppHandle) {
    let existing = app_handle.get_webview_window(INDICATOR_LABEL);
    if existing.is_some() {
        let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", ());
        return;
    }

    let main_window = app_handle.get_webview_window("main");
    let monitor = main_window.as_ref().and_then(|w| w.primary_monitor().ok().flatten());

    let (x, y) = match &monitor {
        Some(m) => {
            let size = m.size();
            let scale = m.scale_factor();
            let screen_w = size.width as f64 / scale;
            let win_w = 160.0;
            #[cfg(target_os = "macos")]
            let top_offset = 33.0;
            #[cfg(not(target_os = "macos"))]
            let top_offset = 12.0;
            (screen_w - win_w - 12.0, top_offset)
        }
        None => (800.0, 33.0),
    };

    let window = WebviewWindowBuilder::new(
        app_handle,
        INDICATOR_LABEL,
        tauri::WebviewUrl::App("/recording".into()),
    )
    .inner_size(160.0, 44.0)
    .position(x, y)
    .transparent(true)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build();

    match window {
        Ok(w) => {
            let _ = w.show();
            let _ = app_handle.emit_to(INDICATOR_LABEL, "indicator:recording", ());
        }
        Err(e) => {
            eprintln!("Failed to create indicator window: {}", e);
        }
    }
}

fn emit_indicator(app_handle: &tauri::AppHandle, event: &str) {
    let _ = app_handle.emit_to(INDICATOR_LABEL, event, ());
}

fn destroy_indicator(app_handle: &tauri::AppHandle) {
    if let Some(w) = app_handle.get_webview_window(INDICATOR_LABEL) {
        let _ = w.close();
    }
}
```

- [ ] **Step 2: 在录音开始处调用 show_indicator**

在 `lib.rs` 快捷键 handler 中，录音成功开始的分支里（搜索 `RECORDING.store(true, Ordering::Relaxed);`），在 `play_sound("Ping.aiff");` 之后添加：

```rust
show_indicator(&app_handle);
```

注意：这一行位于 `Some(()) => {` 分支内。该分支的上下文大约在第 731-748 行。找到 `play_sound("Ping.aiff");` 这一行，在其后插入 `show_indicator(&app_handle);`。

- [ ] **Step 3: 在 stop_recording 中发射 indicator:processing 事件**

在 `stop_recording` 函数中，`"recording:complete"` 分支里，找到 `let _ = app_handle.emit("recording:complete", &result);` 这一行，在其后添加：

```rust
emit_indicator(app_handle, "indicator:processing");
```

- [ ] **Step 4: 在 AI 处理完成后发射 indicator:done 或 indicator:error**

在 `stop_recording` 函数的 `tauri::async_runtime::spawn` 块中：

(a) 在剪贴板粘贴成功的分支（`clipboard::write_and_paste(&final_text)` 的 `Ok(())` 分支）中，在 `logger.info("clipboard", ...)` 之后添加：

```rust
emit_indicator(&h, "indicator:done");
```

(b) 在 AI 转写失败的分支（`Err(e)` 中 `logger.error("ai", ...)` 之后）中，在 `show_notification(&h, "AI 处理失败", &e);` 之后添加：

```rust
destroy_indicator(&h);
```

(c) 在「未找到配置的 AI 提供商」的提前 return 分支中，在 `show_notification` 之后添加：

```rust
destroy_indicator(&h);
```

(d) 在剪贴板写入失败的分支中，在 `show_notification(&h, "剪贴板写入失败", &e);` 之后添加：

```rust
destroy_indicator(&h);
```

- [ ] **Step 5: 在录音取消处销毁浮窗**

在 `stop_recording` 函数的 `"recording:cancel"` 分支中，在 `let _ = app_handle.emit("recording:cancel", cancelled);` 之后添加：

```rust
destroy_indicator(app_handle);
```

- [ ] **Step 6: 在录音错误处销毁浮窗**

在 `stop_recording` 函数中，两个 `recording:error` emit 的地方（`let _ = app_handle.emit("recording:error", ...)` 之后），各添加：

```rust
destroy_indicator(app_handle);
```

- [ ] **Step 7: 处理浮窗的取消/中止事件**

在 `setup` 闭包内，在快捷键 handler 注册之前（`let app_handle = app.handle().clone();` 之前），添加浮窗事件监听：

```rust
{
    let app_handle_cancel = app.handle().clone();
    let recorder_cancel = recorder.clone();
    let recording_start_cancel = recording_start.clone();
    let esc_cancel = esc_shortcut.clone();
    let default_icon_cancel = default_icon_owned.clone();
    let _ = app.listen("indicator:cancel", move |_event| {
        let is_recording = RECORDING.load(Ordering::Relaxed);
        if is_recording {
            stop_recording(
                &app_handle_cancel,
                &recorder_cancel,
                &recording_start_cancel,
                "recording:cancel",
            );
            play_sound("Pop.aiff");
            let h = app_handle_cancel.clone();
            let esc = esc_cancel.clone();
            std::thread::spawn(move || {
                let _ = h.global_shortcut().unregister(esc);
            });
            restore_default_tray(&app_handle_cancel, default_icon_cancel.clone());
        } else {
            destroy_indicator(&app_handle_cancel);
        }
    });
}
```

注意：此处使用 `app.listen`（而非 `app_handle.listen`），因为 `listen` 需要在 setup 阶段用 `app` 引用调用。`recorder`、`recorder_start`、`esc_shortcut`、`default_icon_owned` 在 setup 闭包中已经存在，可以直接 clone。`app.listen` 返回 `EventId`，赋值给 `_` 忽略。

- [ ] **Step 8: 处理中断场景 — 处理中触发新录音**

当用户在 PROCESSING 状态按下录音快捷键时，`show_indicator` 检测到浮窗已存在，会直接发送 `indicator:recording` 事件让浮窗切回录音状态，无需额外处理创建/销毁。

但 spawn 出的异步 AI 任务仍在运行，其结果需要被丢弃。方案：在 spawn 块内，skills 处理完成之后、`clipboard::write_and_paste` 之前，检查 `RECORDING` 是否已变为 true（意味着新录音已开始），如果是则丢弃结果：

```rust
if RECORDING.load(Ordering::Relaxed) {
    logger.info("ai", "录音已重新开始，丢弃当前 AI 结果", None);
    emit_indicator(&h, "indicator:done");
    return;
}
```

将这段代码插入到 `stop_recording` 函数的 spawn 块中，`match text_result` 的 `Ok(text)` 分支内、`clipboard::write_and_paste(&final_text)` 调用之前。

- [ ] **Step 9: 编译验证**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 无编译错误

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add recording float indicator window management in backend"
```

---

## Task 2: 前端 — 浮窗 UI 页面

**Files:**
- Create: `src/routes/recording/+page.svelte`

**目标：** 创建浮窗的 Svelte 页面，实现录音/处理两种状态的 UI、动画过渡、取消/中止按钮。

- [ ] **Step 1: 创建浮窗页面**

创建 `src/routes/recording/+page.svelte`：

```svelte
<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";

  const appWindow = getCurrentWindow();

  type Phase = "recording" | "processing";

  let phase = $state<Phase>("recording");
  let seconds = $state(0);
  let intervalId: ReturnType<typeof setInterval> | null = null;
  let visible = $state(true);

  function formatTime(totalSeconds: number): string {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }

  function startTimer() {
    if (intervalId) clearInterval(intervalId);
    seconds = 0;
    intervalId = setInterval(() => {
      seconds++;
    }, 1000);
  }

  function stopTimer() {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  function cancel() {
    appWindow.emit("indicator:cancel", { phase });
  }

  listen("indicator:recording", () => {
    phase = "recording";
    visible = true;
    startTimer();
  });

  listen("indicator:processing", () => {
    phase = "processing";
    stopTimer();
  });

  listen("indicator:done", () => {
    visible = false;
    setTimeout(() => appWindow.close(), 200);
  });

  listen("indicator:error", () => {
    appWindow.close();
  });

  startTimer();
</script>

<div
  class="indicator"
  class:processing={phase === "processing"}
  class:fade-out={!visible}
>
  {#if phase === "recording"}
    <div class="status">
      <span class="pulse-dot"></span>
      <span class="timer">{formatTime(seconds)}</span>
    </div>
  {:else}
    <div class="status">
      <span class="spinner"></span>
      <span class="label">处理中</span>
    </div>
  {/if}

  <button class="btn cancel" onclick={cancel} aria-label={phase === "recording" ? "取消录音" : "中止处理"}>
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
    </svg>
  </button>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
  }

  :global(html) {
    background: transparent;
  }

  .indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 160px;
    height: 44px;
    padding: 0 8px;
    background: rgba(30, 30, 30, 0.92);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border-radius: 22px;
    box-shadow:
      0 4px 20px rgba(0, 0, 0, 0.3),
      0 0 0 0.5px rgba(255, 255, 255, 0.1);
    transition: opacity 200ms ease;
  }

  .indicator.fade-out {
    opacity: 0;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    justify-content: center;
  }

  .pulse-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #ef4444;
    box-shadow: 0 0 8px rgba(239, 68, 68, 0.5);
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.4;
      transform: scale(0.85);
    }
  }

  .timer {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 13px;
    color: #f1f5f9;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: center;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(99, 102, 241, 0.3);
    border-top-color: #6366f1;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .label {
    font-size: 13px;
    color: #a5b4fc;
    font-weight: 500;
  }

  .btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 50%;
    cursor: pointer;
    background: transparent;
    color: #64748b;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
  }

  .btn:hover {
    background: rgba(255, 95, 87, 0.2);
    color: #ff5f57;
  }
</style>
```

- [ ] **Step 2: 运行类型检查**

Run: `npm run check`
Expected: 无类型错误

- [ ] **Step 3: Commit**

```bash
git add src/routes/recording/+page.svelte
git commit -m "feat: add recording float indicator page with recording/processing states"
```

---

## Task 3: 构建与集成验证

**Files:** 无新建/修改

**目标：** 端到端验证浮窗在录音流程中的完整行为。

- [ ] **Step 1: 运行前端构建**

Run: `npm run build`
Expected: 构建成功，无错误

- [ ] **Step 2: 运行 Tauri 开发模式端到端验证**

Run: `npm run tauri dev`

手动验证以下场景：

1. 按录音快捷键 → 浮窗出现在屏幕右上角，红色脉冲 + 计时器开始
2. 再次按录音快捷键 → 浮窗切换到处理状态（旋转动画 + "处理中"）
3. 等待处理完成 → 浮窗淡出消失，转写结果已粘贴到当前应用
4. 重复场景 1，按 ESC → 浮窗消失，录音取消
5. 重复场景 1-2，在处理中再次按录音快捷键 → 浮窗切回录音状态（中断场景）
6. 断开网络后重复场景 1-2 → 处理失败，浮窗消失 + 系统通知
7. 验证浮窗不被窗口平铺管理器纳入布局

- [ ] **Step 3: 修复 Commit（如有调整）**

```bash
git add -A
git commit -m "fix: adjust float indicator based on manual testing"
```
