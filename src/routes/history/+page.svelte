<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { History, Sparkles, Trash2, Plus, AlertTriangle, ArrowRight } from 'lucide-svelte';
  import { config } from '$lib/stores/config';
  import { updateFeature, invokeWithError } from '$lib/ai/shared';
  import { formatTimestamp } from '$lib/utils';
  import type { GlossaryEntry } from '$lib/stores/config';

  interface HistoryEntry {
    ts: string;
    mode: string;
    raw_text: string;
    final_text: string;
    audio_path: string;
  }

  interface Suspect {
    snippet: string;
    issue: string;
  }

  interface AnalyzeResult {
    suspects: Suspect[];
    glossary_suggestions: { wrong: string; correct: string }[];
  }

  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(false);
  let analyzing = $state(false);
  let analyzeError = $state('');
  let result = $state<AnalyzeResult | null>(null);
  // 勾选的术语建议索引
  let selectedSuggestions = $state<Set<number>>(new Set());
  let newEntry = $state<GlossaryEntry>({ wrong: '', correct: '' });
  let expanded = $state<number>(-1);

  onMount(async () => {
    await loadHistory();
    await config.load();
  });

  async function loadHistory() {
    loading = true;
    try {
      entries = await invoke<HistoryEntry[]>('get_history', { n: 100 });
    } catch (e) {
      console.error('Failed to load history:', e);
    } finally {
      loading = false;
    }
  }

  async function runAnalyze() {
    analyzing = true;
    analyzeError = '';
    result = null;
    selectedSuggestions = new Set();
    try {
      result = await invokeWithError<AnalyzeResult>('analyze_history', { n: 30 });
      if (result) {
        // 默认全选术语建议
        selectedSuggestions = new Set(
          (result.glossary_suggestions || []).map((_, i) => i)
        );
      }
    } catch (e) {
      analyzeError = String(e);
    } finally {
      analyzing = false;
    }
  }

  function toggleSuggestion(i: number) {
    const next = new Set(selectedSuggestions);
    if (next.has(i)) {
      next.delete(i);
    } else {
      next.add(i);
    }
    selectedSuggestions = next;
  }

  function saveGlossary(next: GlossaryEntry[]) {
    config.save(updateFeature($config, 'glossary', () => next));
  }

  function adoptSuggestions() {
    if (!result) return;
    const existing = $config.features.glossary || [];
    const adopted = result.glossary_suggestions
      .filter((_, i) => selectedSuggestions.has(i))
      .filter((s) => s.wrong && s.correct);
    // 去重：同 wrong 已存在则跳过
    const wrongs = new Set(existing.map((g) => g.wrong));
    const merged = [
      ...existing,
      ...adopted.filter((s) => !wrongs.has(s.wrong))
    ];
    saveGlossary(merged);
    // 从结果中移除已采纳项
    const adoptedWrongs = new Set(adopted.map((s) => s.wrong));
    result.glossary_suggestions = result.glossary_suggestions.filter(
      (s) => !adoptedWrongs.has(s.wrong)
    );
    selectedSuggestions = new Set();
  }

  function removeGlossary(index: number) {
    const next = [...($config.features.glossary || [])];
    next.splice(index, 1);
    saveGlossary(next);
  }

  function addManualEntry() {
    if (!newEntry.wrong.trim() || !newEntry.correct.trim()) return;
    const existing = $config.features.glossary || [];
    if (existing.some((g) => g.wrong === newEntry.wrong.trim())) return;
    saveGlossary([...existing, { wrong: newEntry.wrong.trim(), correct: newEntry.correct.trim() }]);
    newEntry = { wrong: '', correct: '' };
  }

  let modeLabel = (mode: string) => (mode === 'translation' ? '翻译' : '转写');
</script>

<div class="flex flex-col h-full">
  <header class="px-8 pt-6 pb-4 border-b border-border">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2.5">
        <History size={22} class="text-muted-foreground" />
        <h1 class="text-title font-semibold text-foreground">历史与回溯</h1>
      </div>
      <button
        class="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-body transition-opacity {analyzing
          ? 'opacity-60 cursor-wait'
          : 'hover:opacity-90'}"
        onclick={runAnalyze}
        disabled={analyzing || entries.length === 0}
      >
        <Sparkles size={16} />
        {analyzing ? '分析中…' : '回溯分析'}
      </button>
    </div>
    <p class="text-muted-foreground text-caption mt-1.5">
      分析最近 30 条转写/翻译记录，找出疑点并沉淀专有名词术语表，后续转写自动纠正
    </p>
  </header>

  <div class="flex-1 overflow-y-auto px-8 py-5 space-y-6">
    {#if analyzeError}
      <div class="flex items-start gap-2 p-3 rounded-lg bg-red-500/10 text-red-500 text-body">
        <AlertTriangle size={16} class="mt-0.5 shrink-0" />
        <span>{analyzeError}</span>
      </div>
    {/if}

    <!-- 分析结果 -->
    {#if result}
      <section class="rounded-xl border border-border bg-background-alt p-5 space-y-4">
        <h2 class="text-subheading font-semibold text-foreground">分析结果</h2>

        {#if result.suspects?.length}
          <div class="space-y-2">
            <h3 class="text-body font-medium text-foreground">疑点（{result.suspects.length}）</h3>
            {#each result.suspects as s}
              <div class="p-3 rounded-lg bg-background border border-border">
                <div class="text-body text-foreground break-all">“{s.snippet}”</div>
                <div class="text-caption text-muted-foreground mt-1">{s.issue}</div>
              </div>
            {/each}
          </div>
        {/if}

        {#if result.glossary_suggestions?.length}
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <h3 class="text-body font-medium text-foreground">
                术语建议（{result.glossary_suggestions.length}）
              </h3>
              <button
                class="px-3 py-1.5 rounded-lg bg-primary text-primary-foreground text-caption transition-opacity hover:opacity-90"
                onclick={adoptSuggestions}
                disabled={selectedSuggestions.size === 0}
              >
                采纳选中（{selectedSuggestions.size}）
              </button>
            </div>
            {#each result.glossary_suggestions as s, i}
              <label
                class="flex items-center gap-3 p-3 rounded-lg bg-background border border-border cursor-pointer"
              >
                <input
                  type="checkbox"
                  class="accent-blue-500 w-4 h-4"
                  checked={selectedSuggestions.has(i)}
                  onchange={() => toggleSuggestion(i)}
                />
                <span class="text-body text-red-400 line-through">{s.wrong}</span>
                <ArrowRight size={14} class="text-muted-foreground shrink-0" />
                <span class="text-body text-green-500 font-medium">{s.correct}</span>
              </label>
            {/each}
          </div>
        {/if}

        {#if !result.suspects?.length && !result.glossary_suggestions?.length}
          <div class="text-body text-muted-foreground py-4 text-center">未发现问题，质量良好</div>
        {/if}
      </section>
    {/if}

    <!-- 术语表管理 -->
    <section class="rounded-xl border border-border bg-background-alt p-5 space-y-4">
      <h2 class="text-subheading font-semibold text-foreground">
        个人术语表（{$config.features.glossary?.length || 0}）
      </h2>
      <p class="text-caption text-muted-foreground">
        条目会注入转写、润色和翻译环节，遇到相同或发音相近的错误写法自动纠正
      </p>

      {#if $config.features.glossary?.length}
        <div class="space-y-2">
          {#each $config.features.glossary as g, i}
            <div class="flex items-center gap-3 p-3 rounded-lg bg-background border border-border">
              <span class="text-body text-red-400 line-through flex-1 break-all">{g.wrong}</span>
              <ArrowRight size={14} class="text-muted-foreground shrink-0" />
              <span class="text-body text-green-500 font-medium flex-1 break-all">{g.correct}</span>
              <button
                class="p-1.5 rounded-md hover:bg-muted transition-colors text-muted-foreground hover:text-red-500"
                onclick={() => removeGlossary(i)}
                aria-label="删除"
              >
                <Trash2 size={15} />
              </button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="text-body text-muted-foreground py-2">暂无术语。回溯分析后采纳建议，或手动添加。</div>
      {/if}

      <div class="flex items-center gap-2">
        <input
          class="flex-1 px-3 py-2 rounded-lg bg-background border border-border text-body text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-accent"
          placeholder="错误写法"
          bind:value={newEntry.wrong}
        />
        <ArrowRight size={14} class="text-muted-foreground shrink-0" />
        <input
          class="flex-1 px-3 py-2 rounded-lg bg-background border border-border text-body text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-accent"
          placeholder="正确写法"
          bind:value={newEntry.correct}
          onkeydown={(e) => e.key === 'Enter' && addManualEntry()}
        />
        <button
          class="p-2 rounded-lg bg-primary text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
          onclick={addManualEntry}
          disabled={!newEntry.wrong.trim() || !newEntry.correct.trim()}
          aria-label="添加"
        >
          <Plus size={16} />
        </button>
      </div>
    </section>

    <!-- 历史列表 -->
    <section class="space-y-2">
      <h2 class="text-subheading font-semibold text-foreground">历史记录（{entries.length}）</h2>
      {#if loading}
        <div class="text-body text-muted-foreground py-4 text-center">加载中…</div>
      {:else if entries.length === 0}
        <div class="text-body text-muted-foreground py-4 text-center">
          暂无历史。完成几次语音转写后即可回溯分析。
        </div>
      {:else}
        {#each entries as e, i}
          <button
            class="w-full text-left p-4 rounded-xl border border-border bg-background-alt hover:border-accent/50 transition-colors"
            onclick={() => (expanded = expanded === i ? -1 : i)}
          >
            <div class="flex items-center gap-2 text-caption text-muted-foreground">
              <span class="px-1.5 py-0.5 rounded bg-muted text-foreground">{modeLabel(e.mode)}</span>
              <span>{formatTimestamp(e.ts)}</span>
            </div>
            <div class="text-body text-foreground mt-1.5 line-clamp-2 break-all">
              {e.final_text || e.raw_text}
            </div>
            {#if expanded === i}
              <div class="mt-3 pt-3 border-t border-border space-y-2">
                {#if e.final_text !== e.raw_text}
                  <div>
                    <div class="text-caption text-muted-foreground mb-1">最终输出</div>
                    <div class="text-body text-foreground break-all whitespace-pre-wrap">{e.final_text}</div>
                  </div>
                  <div>
                    <div class="text-caption text-muted-foreground mb-1">原始转写</div>
                    <div class="text-body text-muted-foreground break-all whitespace-pre-wrap">{e.raw_text}</div>
                  </div>
                {:else}
                  <div class="text-body text-foreground break-all whitespace-pre-wrap">{e.raw_text}</div>
                {/if}
              </div>
            {/if}
          </button>
        {/each}
      {/if}
    </section>
  </div>
</div>
