# 录音指示器 Bug 修复计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复点击 "Start Recording" 无反应的问题

**Root Cause:** 三个问题叠加：
1. 录音窗口已在 `tauri.conf.json` 中预定义（`visible: false`），`getByLabel` 返回已有窗口走了 `existing` 分支，`recordingStatus` 未更新，用户看不到任何反馈
2. capabilities 缺少 `core:window:allow-start-dragging` 权限
3. macOS 上 `visible: false` 有已知 bug（#13452），窗口可能启动时就显示了但被主窗口遮挡

**Fix Strategy:** 不在配置中预定义窗口，改为完全由前端动态创建；添加缺失权限；修复状态反馈

---

### Fix 1: 移除 tauri.conf.json 中的预定义窗口

**Files:**
- Modify: `src-tauri/tauri.conf.json:13-35`

- [ ] **Step 1: 从 windows 数组中删除 recording-indicator 条目**

将 windows 数组恢复为只包含 main 窗口。

- [ ] **Step 2: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix: remove pre-defined recording window from tauri config"
```

---

### Fix 2: 添加缺失的 capabilities 权限

**Files:**
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 添加窗口拖拽和事件权限**

在 permissions 数组中添加：
- `core:window:allow-start-dragging`
- `core:window:allow-show`
- `core:window:allow-close`
- `core:window:allow-set-focus`
- `core:window:allow-center`

- [ ] **Step 2: Commit**

```bash
git add src-tauri/capabilities/default.json
git commit -m "fix: add missing window and event permissions for recording indicator"
```

---

### Fix 3: 修复主窗口 startRecording 逻辑

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: 重写 startRecording 函数**

由于窗口不再预定义，`getByLabel` 只在动态创建后才会返回窗口。简化逻辑：直接创建窗口，`tauri://created` 事件中 show + 更新状态。

```typescript
async function startRecording() {
  recordingStatus = "recording";

  const recordingWindow = new WebviewWindow("recording-indicator", {
    url: "/recording",
    width: 140,
    height: 44,
    transparent: true,
    decorations: false,
    resizable: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    center: true,
  });

  recordingWindow.once("tauri://created", async () => {
    await recordingWindow.show();
    await recordingWindow.setFocus();
  });

  recordingWindow.once("tauri://error", (e) => {
    console.error("Failed to create recording window:", e);
    recordingStatus = "";
  });
}
```

移除 `getByLabel` 的 existing 分支，因为窗口不再预定义。

- [ ] **Step 2: 运行类型检查**

Run: `npm run check`
Expected: 0 errors, 0 warnings

- [ ] **Step 3: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "fix: simplify startRecording to always create window dynamically"
```

---

### Fix 4: 构建验证

**Files:** 无修改

- [ ] **Step 1: 运行前端构建**

Run: `npm run build`
Expected: 构建成功

- [ ] **Step 2: 运行 Tauri dev 验证**

Run: `npm run tauri dev`
Expected:
1. 主窗口显示 "Start Recording" 按钮
2. 点击后主窗口立即显示 "Recording recording" 状态
3. 录音指示器窗口出现在屏幕中央，140×44px
4. 红色脉冲圆点动画播放，计时器递增
5. 点击 ✕ → 窗口关闭，主窗口显示 "cancelled"
6. 点击 ✓ → 窗口关闭，主窗口显示 "completed" + 时长
