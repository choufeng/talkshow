<script lang="ts">
  import { onboarding } from '$lib/stores/onboarding';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { Download, CheckCircle2, AlertCircle, RotateCcw } from 'lucide-svelte';

  type DownloadProgress = {
    file: string;
    percent: number;
    downloaded: number;
    total: number;
  };

  let status = $state<'idle' | 'downloading' | 'ready' | 'error'>('idle');
  let progress = $state<DownloadProgress>({ file: '', percent: 0, downloaded: 0, total: 0 });
  let errorMsg = $state('');
  let errorHint = $state('');
  let unlisten: UnlistenFn | null = null;

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function classifyError(msg: string): { message: string; hint: string } {
    const lower = msg.toLowerCase();
    if (lower.includes('network') || lower.includes('connection') || lower.includes('timeout') || lower.includes('dns') || lower.includes('erefused') || lower.includes('enotfound')) {
      return { message: '网络连接失败，无法下载模型', hint: '请检查网络连接后重试' };
    }
    if (lower.includes('enospc') || lower.includes('disk') || lower.includes('space') || lower.includes('磁盘') || lower.includes('空间')) {
      return { message: '磁盘空间不足', hint: '请释放磁盘空间后重试' };
    }
    if (lower.includes('permission') || lower.includes('eacces') || lower.includes('权限')) {
      return { message: '没有写入权限', hint: '请检查应用是否有写入权限' };
    }
    return { message: msg || '模型下载失败，请重试', hint: '' };
  }

  async function checkStatus() {
    try {
      const result = await invoke<{ status: string; size_bytes?: number }>('get_sensevoice_status');
      if (result.status === 'ready') {
        status = 'ready';
        onboarding.setStepValid(2, true);
        return true;
      }
    } catch (e) {
      console.error('Failed to check sensevoice status:', e);
    }
    return false;
  }

  async function startDownload() {
    status = 'downloading';
    errorMsg = '';
    errorHint = '';
    onboarding.setStepValid(2, false);

    try {
      await invoke('download_sensevoice_model');
      status = 'ready';
      onboarding.setStepValid(2, true);
    } catch (e) {
      status = 'error';
      const raw = e instanceof Error ? e.message : String(e);
      const classified = classifyError(raw);
      errorMsg = classified.message;
      errorHint = classified.hint;
    }
  }

  async function retry() {
    await startDownload();
  }

  onMount(() => {
    (async () => {
      unlisten = await listen<DownloadProgress>('sensevoice:download-progress', (event) => {
        progress = event.payload;
      });

      const alreadyReady = await checkStatus();
      if (!alreadyReady) {
        await startDownload();
      }
    })();

    return () => {
      unlisten?.();
    };
  });
</script>

<div class="text-center">
  <div class="w-14 h-14 rounded-full bg-accent/50 flex items-center justify-center mx-auto mb-5">
    {#if status === 'ready'}
      <CheckCircle2 size={28} class="text-green-500" />
    {:else if status === 'error'}
      <AlertCircle size={28} class="text-red-400" />
    {:else}
      <Download size={28} class="text-accent-foreground" />
    {/if}
  </div>

  <h2 class="text-subheading font-semibold text-foreground mb-2">
    {#if status === 'ready'}
      模型已就绪
    {:else if status === 'error'}
      下载失败
    {:else}
      下载语音模型
    {/if}
  </h2>

  <p class="text-body text-foreground-alt mb-4">
    {#if status === 'ready'}
      SenseVoice 模型已下载完成，可以离线进行语音转写。
    {:else if status === 'error'}
      {errorMsg || '模型下载失败，请重试。'}
      {#if errorHint}
        <span class="block mt-1 text-caption text-muted-foreground">{errorHint}</span>
      {/if}
    {:else}
      正在下载 SenseVoice 本地模型，下载完成后可离线语音转写。
    {/if}
  </p>

  {#if status === 'downloading'}
    <div class="mt-4">
      <div class="text-caption text-muted-foreground mb-2">
        {#if progress.file}
          下载中: {progress.file}
        {:else}
          准备下载...
        {/if}
      </div>
      <div class="w-full bg-muted rounded-full h-2 mb-2">
        <div
          class="bg-gradient-to-r from-btn-primary-from to-btn-primary-to h-2 rounded-full transition-all duration-300"
          style="width: {progress.percent}%"
        ></div>
      </div>
      <div class="flex justify-between text-caption text-muted-foreground">
        <span>{progress.percent.toFixed(1)}%</span>
        <span>
          {#if progress.downloaded && progress.total}
            {formatSize(progress.downloaded)} / {formatSize(progress.total)}
          {/if}
        </span>
      </div>
    </div>
  {:else if status === 'error'}
    <button
      class="mt-4 inline-flex items-center gap-2 px-5 py-2 rounded-lg text-body font-medium transition-colors border border-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-foreground shadow-btn-secondary hover:bg-muted/50"
      onclick={retry}
    >
      <RotateCcw size={16} />
      重试
    </button>
  {/if}
</div>
