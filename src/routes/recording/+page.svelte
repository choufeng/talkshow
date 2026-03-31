<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";

  const appWindow = getCurrentWindow();

  type Phase = "recording" | "processing";

  let phase = $state<Phase>("recording");
  let seconds = $state(0);
  let intervalId: ReturnType<typeof setInterval> | null = null;
  let visible = $state(true);
  let closeTimeoutId: ReturnType<typeof setTimeout> | null = null;

  function formatTime(totalSeconds: number): string {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }

  function startTimer() {
    if (intervalId) clearInterval(intervalId);
    if (closeTimeoutId) {
      clearTimeout(closeTimeoutId);
      closeTimeoutId = null;
    }
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

  function scheduleClose() {
    visible = false;
    closeTimeoutId = setTimeout(() => appWindow.close(), 200);
  }

  $effect(() => {
    const unsubs: Array<() => void> = [];

    (async () => {
      unsubs.push(
        await listen("indicator:recording", () => {
          phase = "recording";
          visible = true;
          startTimer();
        }),
      );
      unsubs.push(
        await listen("indicator:processing", () => {
          phase = "processing";
          stopTimer();
        }),
      );
      unsubs.push(await listen("indicator:done", scheduleClose));
      unsubs.push(await listen("indicator:error", () => appWindow.close()));
    })();

    return () => unsubs.forEach((fn) => fn());
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
      <svg class="neon-ring" width="24" height="24" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="8" fill="none" stroke="#ff0055" stroke-width="1.5">
          <animate attributeName="r" values="7;9;7" dur="1.2s" repeatCount="indefinite"/>
          <animate attributeName="opacity" values="1;0.5;1" dur="1.2s" repeatCount="indefinite"/>
        </circle>
        <circle cx="12" cy="12" r="3" fill="#ff0055"/>
      </svg>
      <span class="timer">{formatTime(seconds)}</span>
      <span class="rec-label">REC</span>
    </div>
  {:else}
    <div class="status">
      <svg class="jellyfish" width="28" height="28" viewBox="0 0 28 28" style="overflow:visible">
        <defs>
          <filter id="jg-bell">
            <feGaussianBlur stdDeviation="2.5" result="b"/>
            <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>
          <filter id="jg-tent">
            <feGaussianBlur stdDeviation="1.5" result="b"/>
            <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
          </filter>
          <radialGradient id="jbell-grad" cx="50%" cy="40%" r="50%">
            <stop offset="0%" style="stop-color:#00c8ff;stop-opacity:0.3"/>
            <stop offset="100%" style="stop-color:#00c8ff;stop-opacity:0.05"/>
          </radialGradient>
        </defs>
        <g class="jellyfish-body">
          <path class="jelly-bell" d="M5,16 Q5,5 14,5 Q23,5 23,16 Q19,19 14,17 Q9,19 5,16 Z"
                fill="url(#jbell-grad)" stroke="#00c8ff" stroke-width="1.2" filter="url(#jg-bell)">
            <animate attributeName="d"
              values="M5,16 Q5,5 14,5 Q23,5 23,16 Q19,19 14,17 Q9,19 5,16 Z;
                      M5,15 Q6,7 14,7 Q22,7 23,15 Q19,17 14,15 Q9,17 5,15 Z;
                      M5,16 Q5,5 14,5 Q23,5 23,16 Q19,19 14,17 Q9,19 5,16 Z"
              dur="2s" repeatCount="indefinite"/>
          </path>
          <ellipse cx="14" cy="12" rx="4" ry="3" fill="#00c8ff" opacity="0.15" filter="url(#jg-tent)">
            <animate attributeName="ry" values="3;2;3" dur="2s" repeatCount="indefinite"/>
          </ellipse>
          <path d="M8,16.5 Q6,22 8,27" fill="none" stroke="#00c8ff" stroke-width="0.8" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.5">
            <animate attributeName="d"
              values="M8,16.5 Q6,22 8,27;M8,15.5 Q5,21 7,26;M8,16.5 Q6,22 8,27"
              dur="2.5s" repeatCount="indefinite"/>
          </path>
          <path d="M11,17 Q9,23 10,28" fill="none" stroke="#00c8ff" stroke-width="1" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.7">
            <animate attributeName="d"
              values="M11,17 Q9,23 10,28;M11,16 Q8,22 9,27;M11,17 Q9,23 10,28"
              dur="2.2s" repeatCount="indefinite"/>
          </path>
          <path d="M14,17.5 Q13,24 14,28" fill="none" stroke="#00c8ff" stroke-width="1.2" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.8">
            <animate attributeName="d"
              values="M14,17.5 Q13,24 14,28;M14,16.5 Q12,23 13,27;M14,17.5 Q13,24 14,28"
              dur="2s" repeatCount="indefinite"/>
          </path>
          <path d="M17,17 Q19,23 18,28" fill="none" stroke="#00c8ff" stroke-width="1" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.7">
            <animate attributeName="d"
              values="M17,17 Q19,23 18,28;M17,16 Q20,22 19,27;M17,17 Q19,23 18,28"
              dur="2.2s" repeatCount="indefinite"/>
          </path>
          <path d="M20,16.5 Q22,22 20,27" fill="none" stroke="#00c8ff" stroke-width="0.8" stroke-linecap="round" filter="url(#jg-tent)" opacity="0.5">
            <animate attributeName="d"
              values="M20,16.5 Q22,22 20,27;M20,15.5 Q23,21 21,26;M20,16.5 Q22,22 20,27"
              dur="2.5s" repeatCount="indefinite"/>
          </path>
        </g>
      </svg>
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
  :global(html), :global(body) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    border: none !important;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
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

  .neon-ring {
    filter: drop-shadow(0 0 4px rgba(255, 0, 85, 0.6));
    flex-shrink: 0;
  }

  .timer {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 13px;
    color: #ff0055;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: center;
    text-shadow: 0 0 6px rgba(255, 0, 85, 0.4);
  }

  .rec-label {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 8px;
    color: rgba(255, 0, 85, 0.4);
    letter-spacing: 2px;
  }

  .indicator.processing {
    width: 160px;
  }

  .jellyfish {
    flex-shrink: 0;
  }

  .jellyfish-body {
    animation: jelly-swim 2s ease-in-out infinite;
  }

  @keyframes jelly-swim {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-2px); }
  }

  .label {
    font-family: -apple-system, BlinkMacSystemFont, "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 13px;
    color: #00c8ff;
    font-weight: 500;
    text-shadow: 0 0 6px rgba(0, 200, 255, 0.4);
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
    background: linear-gradient(to bottom, rgba(255,255,255,0.08), rgba(255,255,255,0.03));
    color: #64748b;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
    box-shadow: 0 1px 3px rgba(0,0,0,0.2);
  }

  .btn:hover {
    background: linear-gradient(to bottom, rgba(255, 95, 87, 0.25), rgba(255, 95, 87, 0.15));
    color: #ff5f57;
    box-shadow: 0 1px 4px rgba(255, 95, 87, 0.2);
  }
</style>
