# 录音悬浮指示器实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现一个录音状态的悬浮指示器，作为 Tauri 独立小窗口，显示脉冲动画和计时，提供取消/完成操作。

**Architecture:** 在 SvelteKit 中新增 `/recording` 路由作为指示器窗口的前端页面。Tauri 通过 `tauri.conf.json` 中的额外窗口配置创建独立悬浮窗口。窗口间通过 Tauri 的 `emit`/`listen` 事件系统通信。主窗口通过 `WebviewWindow` API 创建和控制录音窗口。

**Tech Stack:** Tauri v2, Svelte 5 (SvelteKit), CSS keyframes, Tauri event system

---

### Task 1: Tauri 配置 — 添加录音指示器窗口

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 在 tauri.conf.json 中添加录音指示器窗口配置**

在 `app.windows` 数组中追加一个新窗口条目：

```json
{
  "label": "recording-indicator",
  "title": "Recording",
  "url": "/recording",
  "width": 140,
  "height": 44,
  "transparent": true,
  "decorations": false,
  "resizable": false,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "visible": false,
  "center": true
}
```

注意：`visible: false` 表示窗口创建时不可见，由前端代码控制显示。

- [ ] **Step 2: 在 capabilities/default.json 中为录音窗口添加权限**

将 `windows` 数组从 `["main"]` 改为 `["main", "recording-indicator"]`。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "feat: add recording indicator window config"
```

---

### Task 2: 录音指示器前端页面

**Files:**
- Create: `src/routes/recording/+page.svelte`

- [ ] **Step 1: 创建录音指示器页面组件**

```svelte
<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();

  let seconds = $state(0);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  function formatTime(totalSeconds: number): string {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }

  async function startTimer() {
    seconds = 0;
    intervalId = setInterval(() => {
      seconds++;
    }, 1000);
  }

  async function cancel() {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
    await appWindow.emit("recording:cancel");
    await appWindow.close();
  }

  async function complete() {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
    await appWindow.emit("recording:complete", { duration: seconds });
    await appWindow.close();
  }

  async function handleDrag(event: MouseEvent) {
    if ((event.target as HTMLElement).closest("button")) return;
    await appWindow.startDragging();
  }

  startTimer();
</script>

<div class="indicator" onmousedown={handleDrag}>
  <button class="btn cancel" onclick={cancel} aria-label="Cancel recording">
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
    </svg>
  </button>

  <div class="status">
    <span class="pulse-dot"></span>
    <span class="timer">{formatTime(seconds)}</span>
  </div>

  <button class="btn complete" onclick={complete} aria-label="Complete recording">
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <polyline points="1,5 4,8 9,2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
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
  }

  .indicator {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 140px;
    height: 44px;
    padding: 0 8px;
    background: rgba(30, 30, 30, 0.9);
    border-radius: 10px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    background: transparent;
    transition: background 0.15s;
  }

  .btn.cancel {
    color: #ff5f57;
  }

  .btn.cancel:hover {
    background: rgba(255, 95, 87, 0.2);
  }

  .btn.complete {
    color: #28c840;
  }

  .btn.complete:hover {
    background: rgba(40, 200, 64, 0.2);
  }

  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    justify-content: center;
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ff3b30;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.4);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }

  .timer {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", monospace;
    font-size: 13px;
    color: #ffffff;
    font-variant-numeric: tabular-nums;
  }
</style>
```

- [ ] **Step 2: 验证 TypeScript 类型检查**

Run: `npm run check`
Expected: 无类型错误

- [ ] **Step 3: Commit**

```bash
git add src/routes/recording/+page.svelte
git commit -m "feat: add recording indicator page with pulse animation and timer"
```

---

### Task 3: 主窗口 — 添加打开录音窗口的触发入口

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: 在主页面添加"开始录音"按钮和录音事件监听**

在 `+page.svelte` 中添加一个按钮，点击后创建并显示录音指示器窗口：

```svelte
<script lang="ts">
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { listen } from "@tauri-apps/api/event";

  let name = $state("");
  let greetMsg = $state("");
  let recordingStatus = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    greetMsg = await invoke("greet", { name });
  }

  async function startRecording() {
    const existing = await WebviewWindow.getByLabel("recording-indicator");
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }

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

    recordingWindow.once("tauri://created", () => {
      recordingStatus = "recording";
    });

    recordingWindow.once("tauri://error", (e) => {
      console.error("Failed to create recording window:", e);
    });
  }

  listen<{ duration: number }>("recording:complete", (event) => {
    recordingStatus = `completed (${Math.floor(event.payload.duration / 60)}:${String(event.payload.duration % 60).padStart(2, "0")})`;
  });

  listen("recording:cancel", () => {
    recordingStatus = "cancelled";
  });
</script>

<main class="container">
  <h1>Welcome to Tauri + Svelte</h1>

  <div class="row">
    <a href="https://vite.dev" target="_blank">
      <img src="/vite.svg" class="logo vite" alt="Vite Logo" />
    </a>
    <a href="https://tauri.app" target="_blank">
      <img src="/tauri.svg" class="logo tauri" alt="Tauri Logo" />
    </a>
    <a href="https://svelte.dev" target="_blank">
      <img src="/svelte.svg" class="logo svelte-kit" alt="SvelteKit Logo" />
    </a>
  </div>
  <p>Click on the Tauri, Vite, and SvelteKit logos to learn more.</p>

  <form class="row" onsubmit={greet}>
    <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
    <button type="submit">Greet</button>
  </form>
  <p>{greetMsg}</p>

  <div class="row" style="margin-top: 20px">
    <button class="record-btn" onclick={startRecording}>Start Recording</button>
    {#if recordingStatus}
      <p class="recording-status">Recording {recordingStatus}</p>
    {/if}
  </div>
</main>
```

在 `<style>` 中追加：

```css
.record-btn {
  background: #ff3b30;
  color: white;
  border: none;
  border-radius: 8px;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

.recording-status {
  margin-left: 10px;
  font-size: 0.9em;
}
```

- [ ] **Step 2: 验证 TypeScript 类型检查**

Run: `npm run check`
Expected: 无类型错误

- [ ] **Step 3: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: add start recording button with event listeners in main window"
```

---

### Task 4: 构建验证

**Files:** 无新建/修改

- [ ] **Step 1: 运行 Vite 前端构建**

Run: `npm run build`
Expected: 构建成功，无错误

- [ ] **Step 2: 运行 Tauri 开发模式验证**

Run: `npm run tauri dev`
Expected:
1. 主窗口正常显示
2. 点击 "Start Recording" 按钮
3. 录音指示器窗口出现在屏幕中央，140×44px，无边框，半透明深色背景
4. 红色脉冲圆点动画正常播放
5. 计时器从 00:00 开始递增
6. 点击 ✕ 按钮：窗口关闭，主窗口显示 "cancelled"
7. 再次点击 "Start Recording"，再点击 ✓ 按钮：窗口关闭，主窗口显示 "completed" 及时长

- [ ] **Step 3: 最终 Commit（如有修复）**

```bash
git add -A
git commit -m "fix: adjust recording indicator based on manual testing"
```
