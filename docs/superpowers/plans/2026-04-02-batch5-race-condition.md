# Batch 5: P2 竞态条件修复

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复录音状态管理的竞态条件，使用原子操作替代非原子的读-判断-写模式。

**Architecture:** 将 `lib.rs` 中录音快捷键处理逻辑从 `load + if + store` 模式改为 `compare_exchange` 原子操作。将关键路径上的 `Ordering::Relaxed` 升级为 `Ordering::SeqCst`。

**Tech Stack:** Rust / std::sync::atomic

**前置依赖：** 无

---

## File Structure

| 文件 | 操作 |
|------|------|
| `src-tauri/src/lib.rs` | 修改 — 录音状态原子操作 + Ordering 升级 |

---

### Task 1: 使用 compare_exchange 重构录音状态切换

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Context:** `lib.rs:1163-1188` 中，录音状态切换使用 `RECORDING.load(Ordering::Relaxed)` 读取后，在多步操作后才 `RECORDING.store(...)`。期间另一个线程可能改变该值，导致两个快捷键同时触发录音或取消操作失效。

- [ ] **Step 1: 理解当前录音状态切换逻辑**

当前逻辑位于 `lib.rs` 的录音快捷键处理回调中（约 1160-1230 行）：

```
1. load RECORDING → 判断是否正在录音
2. 如果正在录音：store NONE → 停止录音 → 播放音效 → 通知前端
3. 如果未在录音：启动录音 → store TRANSCRIPTION → 播放音效 → 通知前端
```

问题在于步骤 1-2 或 1-3 之间，状态可能被其他线程改变。

- [ ] **Step 2: 使用 compare_exchange 重构**

将状态切换改为原子操作：

```rust
// 尝试从 NONE → TRANSCRIPTION（开始录音）
let start_result = RECORDING.compare_exchange(
    RECORDING_MODE_NONE,
    RECORDING_MODE_TRANSCRIPTION,
    Ordering::SeqCst,
    Ordering::SeqCst,
);

match start_result {
    Ok(_) => {
        // 成功获取录音权，开始录音
        // ... 原有的 start_recording 逻辑 ...
    }
    Err(current_mode) if current_mode != RECORDING_MODE_NONE => {
        // 当前正在录音，停止录音
        RECORDING.store(RECORDING_MODE_NONE, Ordering::SeqCst);
        // ... 原有的 stop_recording 逻辑 ...
    }
    Err(_) => unreachable!(),
}
```

**注意：** 原代码中的开始录音和停止录音逻辑较为复杂（涉及多个 Mutex 锁、osascript 调用、Tauri 事件发射等）。重构时需要确保：
- `start_result` 分支内的所有副作用（通知前端、播放音效、保存前台应用等）保留
- 停止录音分支内的所有副作用（停止录音、通知前端、恢复音量等）保留
- `should_ignore` 检查保留在 compare_exchange 之前

- [ ] **Step 3: 升级其他关键路径上的 Ordering**

搜索 `lib.rs` 中所有 `Ordering::Relaxed`：

- `RECORDING.load(Ordering::Relaxed)` 用于 `should_ignore` 检查 — 可保留 Relaxed（只是启发式过滤）
- `RECORDING.load(Ordering::Relaxed)` 用于判断是否正在翻译 — 应升级为 `SeqCst`
- `CANCELLED.load(Ordering::Relaxed)` / `store` — 应升级为 `SeqCst`

- [ ] **Step 4: 运行测试**

Run: `npm run test:rust`
Expected: 所有测试 PASS

- [ ] **Step 5: 手动测试录音快捷键**

Run: `npm run tauri dev`
验证：
- 按录音快捷键，正常开始录音
- 再按一次，正常停止录音
- 快速连续按多次，不会出现状态异常（双重录音、无法停止等）
- 翻译快捷键同样正常工作

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix(security): use atomic compare_exchange for recording state (M-5)"
```
