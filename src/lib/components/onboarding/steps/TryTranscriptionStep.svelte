<script lang="ts">
  import { onboarding } from '$lib/stores/onboarding';
  import { onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { Mic, Loader2, CheckCircle2, AlertCircle, RotateCcw } from 'lucide-svelte';

  type StepState = 'waiting' | 'processing' | 'done' | 'error';

  let stepState = $state<StepState>('waiting');
  let resultText = $state('');
  let errorMsg = $state('');
  let timedOut = $state(false);
  let unlisteners: UnlistenFn[] = [];
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  const TIMEOUT_MS = 30_000;

  function clearTimeout_() {
    if (timeoutId) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  }

  function startTimeout() {
    clearTimeout_();
    timeoutId = setTimeout(() => {
      if (stepState === 'waiting' || stepState === 'processing') {
        timedOut = true;
        stepState = 'error';
        errorMsg = '未检测到操作';
      }
    }, TIMEOUT_MS);
  }

  function reset() {
    stepState = 'waiting';
    resultText = '';
    errorMsg = '';
    timedOut = false;
    onboarding.setStepValid(5, false);
    startTimeout();
  }

  function skip() {
    stepState = 'done';
    resultText = '';
    onboarding.setStepValid(5, true);
  }

  onMount(() => {
    (async () => {
      unlisteners.push(
        await listen<{ path: string; duration_secs: number; format: string }>(
          'recording:complete',
          () => {
            if (stepState === 'waiting') {
              stepState = 'processing';
            }
          },
        ),
      );

      unlisteners.push(
        await listen<{ text: string; mode: number }>('pipeline:complete', (event) => {
          if (event.payload.mode === 1) {
            stepState = 'done';
            resultText = event.payload.text;
            onboarding.setStepValid(5, true);
            clearTimeout_();
          }
        }),
      );

      unlisteners.push(
        await listen<string>('recording:error', () => {
          stepState = 'error';
          errorMsg = '录音或处理出错';
          clearTimeout_();
        }),
      );

      unlisteners.push(
        await listen<{ duration_secs: number }>('recording:cancel', () => {
          stepState = 'error';
          errorMsg = '录音已取消';
          clearTimeout_();
        }),
      );

      startTimeout();
    })();

    return () => {
      unlisteners.forEach((fn) => fn());
      clearTimeout_();
    };
  });
</script>

<div class="text-center">
  <div class="w-14 h-14 rounded-full bg-accent/50 flex items-center justify-center mx-auto mb-5">
    {#if stepState === 'done'}
      <CheckCircle2 size={28} class="text-green-500" />
    {:else if stepState === 'error'}
      <AlertCircle size={28} class="text-red-400" />
    {:else if stepState === 'processing'}
      <Loader2 size={28} class="text-accent-foreground animate-spin" />
    {:else}
      <Mic size={28} class="text-accent-foreground" />
    {/if}
  </div>

  <h2 class="text-subheading font-semibold text-foreground mb-2">
    {#if stepState === 'done'}
      转写成功
    {:else if stepState === 'error'}
      {timedOut ? '未检测到操作' : '转写出错'}
    {:else if stepState === 'processing'}
      AI 处理中...
    {:else}
      试用转写
    {/if}
  </h2>

  <p class="text-body text-foreground-alt mb-4">
    {#if stepState === 'waiting'}
      按下录音快捷键，说一段话后再次按下停止，AI 将自动转写。
    {:else if stepState === 'processing'}
      正在处理转写结果，请稍候...
    {:else if stepState === 'done'}
      转写已完成，以下是识别结果：
    {:else}
      {errorMsg || '请重试或跳过此步骤'}
    {/if}
  </p>

  {#if stepState === 'waiting'}
    <div class="flex items-center justify-center gap-2 mt-4">
      <span class="text-caption text-foreground-alt">按下</span>
      <kbd class="px-3 py-1.5 rounded-md text-caption font-mono bg-gradient-to-b from-key-bg-from to-key-bg-to border border-key-border text-key-text">
        Ctrl + \
      </kbd>
      <span class="text-caption text-foreground-alt">开始录音，再次按下停止</span>
    </div>
  {/if}

  {#if stepState === 'processing'}
    <div class="mt-4 flex items-center justify-center gap-2 text-caption text-muted-foreground">
      <Loader2 size={14} class="animate-spin" />
      <span>等待 AI 处理结果...</span>
    </div>
  {/if}

  {#if stepState === 'done' && resultText}
    <div class="mt-4 p-4 rounded-lg bg-muted/50 text-left">
      <p class="text-body text-foreground whitespace-pre-wrap">{resultText}</p>
    </div>
  {/if}

  {#if stepState === 'error'}
    <div class="flex items-center justify-center gap-3 mt-4">
      <button
        class="inline-flex items-center gap-2 px-5 py-2 rounded-lg text-body font-medium transition-colors border border-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-foreground shadow-btn-secondary hover:bg-muted/50"
        onclick={reset}
      >
        <RotateCcw size={16} />
        重试
      </button>
      <div class="flex flex-col items-center">
        <button
          class="px-5 py-2 rounded-lg text-body font-medium transition-colors text-muted-foreground hover:text-foreground"
          onclick={skip}
        >
          跳过
        </button>
        <span class="text-[11px] text-muted-foreground/60 mt-0.5">稍后可在设置中测试</span>
      </div>
    </div>
  {/if}
</div>
