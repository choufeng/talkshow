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
