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

<!-- svelte-ignore a11y_no_static_element_interactions -->
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
