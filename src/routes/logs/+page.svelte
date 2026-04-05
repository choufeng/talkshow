<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ScrollText } from 'lucide-svelte';
  import { formatTimestamp } from '$lib/utils';
  import { invokeWithError } from '$lib/ai/shared';

  interface LogEntry {
    ts: string;
    module: string;
    level: string;
    msg: string;
    meta?: Record<string, unknown>;
  }

  interface LogSession {
    filename: string;
    size_bytes: number;
    entry_count: number;
    first_ts?: string;
    is_current: boolean;
  }

  const MODULES = ['all', 'connectivity', 'recording', 'ai', 'system'] as const;
  const MODULE_COLORS: Record<string, string> = {
    connectivity: 'text-blue-400',
    recording: 'text-purple-400',
    ai: 'text-amber-400',
    system: 'text-muted-foreground',
  };

  let activeTab = $state<'current' | 'history'>('current');
  let selectedModule = $state<string>('all');
  let sessions = $state<LogSession[]>([]);
  let entries = $state<LogEntry[]>([]);
  let selectedSession = $state<string | null>(null);
  let loading = $state(false);
  let copied = $state(false);
  let selectMode = $state(false);
  let selectedIds = $state<Set<number>>(new Set());
  let lastClickedIndex = $state<number>(-1);

  let filteredEntries = $derived(
    selectedModule === 'all'
      ? entries
      : entries.filter((e) => e.module === selectedModule)
  );

  let currentSession = $derived(sessions.find((s) => s.is_current));

  onMount(async () => {
    await loadSessions();
    await loadCurrentLog();
  });

  async function loadSessions() {
    sessions = await invokeWithError<LogSession[]>('get_log_sessions') ?? [];
  }

  async function loadCurrentLog() {
    loading = true;
    entries = await invokeWithError<LogEntry[]>('get_log_content', { sessionFile: null }) ?? [];
    loading = false;
  }

  async function loadSessionLog(filename: string) {
    loading = true;
    selectedSession = filename;
    entries = await invokeWithError<LogEntry[]>('get_log_content', { sessionFile: filename }) ?? [];
    loading = false;
  }

  async function switchTab(tab: 'current' | 'history') {
    activeTab = tab;
    if (tab === 'current') {
      selectedSession = null;
      await loadCurrentLog();
    } else {
      entries = [];
      selectedSession = null;
    }
  }

  function formatSessionName(filename: string): string {
    const match = filename.match(/talkshow-(\d{4})-(\d{2})-(\d{2})_(\d{2})-(\d{2})-(\d{2})/);
    if (match) {
      return `${match[1]}-${match[2]}-${match[3]} ${match[4]}:${match[5]}:${match[6]}`;
    }
    return filename;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  }

  function levelColor(level: string): string {
    switch (level) {
      case 'error': return 'text-red-400';
      case 'warn': return 'text-yellow-400';
      default: return 'text-muted-foreground';
    }
  }

  function metaSummary(meta: Record<string, unknown> | undefined): string {
    if (!meta) return '';
    const parts: string[] = [];
    if (meta.provider_id) parts.push(String(meta.provider_id));
    if (meta.model_name) parts.push(String(meta.model_name));
    if (meta.latency_ms != null) parts.push(`${meta.latency_ms}ms`);
    if (meta.error) parts.push(String(meta.error));
    return parts.join(' · ');
  }

  function formatEntryForCopy(entry: LogEntry): string {
    const ts = formatTimestamp(entry.ts);
    const summary = metaSummary(entry.meta as Record<string, unknown> | undefined);
    let line = `[${ts}] [${entry.level}] [${entry.module}] ${entry.msg}`;
    if (summary) line += ` | ${summary}`;
    if (entry.meta) line += `\n${JSON.stringify(entry.meta, null, 2)}`;
    return line;
  }

  async function copyAll() {
    const text = filteredEntries.map(formatEntryForCopy).join('\n');
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => { copied = false; }, 2000);
  }

  function toggleSelectMode() {
    selectMode = !selectMode;
    if (!selectMode) {
      selectedIds.clear();
      lastClickedIndex = -1;
    }
  }

  function toggleEntrySelection(index: number, event: Event) {
    if ((event as MouseEvent).shiftKey && lastClickedIndex >= 0) {
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      for (let i = start; i <= end; i++) {
        selectedIds.add(i);
      }
    } else {
      if (selectedIds.has(index)) {
        selectedIds.delete(index);
      } else {
        selectedIds.add(index);
      }
      lastClickedIndex = index;
    }
  }

  async function copySelected() {
    const indices = Array.from(selectedIds).sort((a, b) => a - b).filter((i) => i < filteredEntries.length);
    const text = indices.map((i) => formatEntryForCopy(filteredEntries[i])).join('\n');
    await navigator.clipboard.writeText(text);
    copied = true;
    selectMode = false;
    selectedIds.clear();
    lastClickedIndex = -1;
    setTimeout(() => { copied = false; }, 2000);
  }
</script>

<div class="max-w-[960px]">
  <h2 class="text-title font-semibold text-foreground m-0 mb-8">日志</h2>

  <div class="flex items-center gap-3 mb-5">
    <div class="flex gap-0.5 bg-muted rounded-md p-0.5">
      <button
        class="px-4 py-1.5 rounded text-body transition-colors {activeTab === 'current' ? 'bg-background text-foreground font-medium' : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => switchTab('current')}
      >
        当前会话
      </button>
      <button
        class="px-4 py-1.5 rounded text-body transition-colors {activeTab === 'history' ? 'bg-background text-foreground font-medium' : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => switchTab('history')}
      >
        历史记录
      </button>
    </div>

    <div class="flex gap-1">
      {#each MODULES as mod}
        <button
          class="px-3 py-1.5 rounded text-caption transition-colors {selectedModule === mod ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium shadow-btn-primary' : 'border border-border text-muted-foreground hover:text-foreground'}"
          onclick={() => (selectedModule = mod)}
        >
          {mod === 'all' ? '全部' : mod}
        </button>
      {/each}
    </div>

    {#if currentSession && activeTab === 'current'}
      <span class="ml-auto text-caption text-muted-foreground">
        {filteredEntries.length} 条日志
        {#if selectMode && selectedIds.size > 0}
          · 已选 {selectedIds.size} 条
        {/if}
      </span>
      {#if selectMode}
        <button
          class="px-3 py-1 rounded text-caption border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-accent-foreground hover:opacity-90 transition-colors shadow-btn-secondary disabled:opacity-50"
          onclick={copySelected}
          disabled={selectedIds.size === 0 || copied}
        >
          {copied ? '已复制' : `复制选中 (${selectedIds.size})`}
        </button>
        <button
          class="px-3 py-1 rounded text-caption border border-border text-muted-foreground hover:text-foreground transition-colors"
          onclick={toggleSelectMode}
        >
          取消
        </button>
      {:else}
        <button
          class="px-3 py-1 rounded text-caption border border-border text-muted-foreground hover:text-foreground transition-colors"
          onclick={toggleSelectMode}
        >
          选择
        </button>
        <button
          class="px-3 py-1 rounded text-caption border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-accent-foreground hover:opacity-90 transition-colors shadow-btn-secondary"
          onclick={copyAll}
          disabled={copied}
        >
          {copied ? '已复制' : '复制'}
        </button>
      {/if}
    {/if}
  </div>

  {#if activeTab === 'history' && !selectedSession}
    <div class="space-y-2">
      {#if sessions.length === 0}
        <div class="text-base text-muted-foreground py-12 text-center">暂无历史日志</div>
      {:else}
        {#each sessions as session}
          <button
            class="w-full flex items-center gap-5 px-5 py-4 rounded-xl border border-border bg-background-alt text-left transition-colors hover:bg-muted/50"
            onclick={() => loadSessionLog(session.filename)}
          >
            <ScrollText size={16} class="shrink-0 text-muted-foreground" />
            <div class="flex-1 min-w-0">
              <div class="text-base text-foreground font-medium">
                {formatSessionName(session.filename)}
                {#if session.is_current}
                  <span class="text-[11px] text-green-400 ml-2 font-normal">当前</span>
                {/if}
              </div>
              <div class="text-body text-muted-foreground mt-0.5">
                {session.entry_count} 条 · {formatSize(session.size_bytes)}
              </div>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  {:else}
    {#if loading}
      <div class="text-base text-muted-foreground py-12 text-center">加载中...</div>
    {:else if filteredEntries.length === 0}
      <div class="text-base text-muted-foreground py-12 text-center">暂无日志</div>
    {:else}
      <div class="border border-border rounded-xl overflow-hidden">
        <div class="max-h-[calc(100vh-200px)] overflow-y-auto font-mono text-body">
          {#each filteredEntries as entry, i}
            <div
              class="flex gap-3 px-5 py-2 border-b border-border last:border-b-0 hover:bg-muted/30 {selectMode && selectedIds.has(i) ? 'bg-muted/50' : ''} {selectMode ? 'cursor-pointer' : ''}"
              onclick={(e) => selectMode && toggleEntrySelection(i, e)}
            >
              {#if selectMode}
                <input
                  type="checkbox"
                  checked={selectedIds.has(i)}
                  class="mt-0.5 pointer-events-none"
                />
              {/if}
              <span class="text-muted-foreground whitespace-nowrap shrink-0">{formatTimestamp(entry.ts)}</span>
              <span class="{levelColor(entry.level)} shrink-0 w-4 text-center">
                {entry.level === 'error' ? '✕' : entry.level === 'warn' ? '!' : '·'}
              </span>
              <span class="{MODULE_COLORS[entry.module] || 'text-muted-foreground'} shrink-0 w-28">{entry.module}</span>
              <span class="{entry.level === 'error' ? 'text-red-400' : 'text-foreground'} truncate">{entry.msg}</span>
              <span class="text-muted-foreground text-[11px] ml-auto whitespace-nowrap shrink-0">
                {metaSummary(entry.meta as Record<string, unknown> | undefined)}
              </span>
            </div>
          {/each}
        </div>
      </div>

      {#if activeTab === 'history' && selectedSession}
        <button
          class="mt-3 text-body text-accent-foreground hover:opacity-80 transition-colors"
          onclick={() => { selectedSession = null; entries = []; }}
        >
          ← 返回历史列表
        </button>
      {/if}
    {/if}
  {/if}
</div>
