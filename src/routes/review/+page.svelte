<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { X, ChevronDown } from 'lucide-svelte';
  import '$lib/stores/theme'; // 触发主题应用(浮窗独立于主窗体)

  interface HookPreview {
    url: string;
    body: string;
    vars: Record<string, string>;
  }

  interface ReviewRequest {
    id: number;
    raw_text: string;
    text: string;
    original_text: string | null;
    target_lang: string | null;
    primary: 'paste' | 'ask' | 'hook';
    alternatives: string[];
    available: string[];
    target_app: string | null;
    hook: HookPreview | null;
    audio_path: string;
  }

  type ActionKind = 'paste' | 'translate' | 'ask' | 'hook';
  const ALL_KINDS: ActionKind[] = ['paste', 'translate', 'ask', 'hook'];
  const KIND_LABEL: Record<ActionKind, string> = {
    paste: '转写粘贴',
    translate: '翻译',
    ask: '问询',
    hook: '发送到 Hook'
  };

  const LANGUAGES = [
    'English', '中文', '日本語', '한국어', 'Français', 'Deutsch',
    'Español', 'Português', 'Italiano', 'Русский', 'العربية', 'हिन्दी'
  ];

  let req = $state<ReviewRequest | null>(null);
  let text = $state('');
  let currentAction = $state<ActionKind>('paste');
  let targetLang = $state('English');
  let busy = $state(false);
  let error = $state('');
  let showRaw = $state(false);
  let showMore = $state(false);
  let sendKey = $state<'enter' | 'cmd_enter'>('enter');
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  function load(r: ReviewRequest) {
    req = r;
    text = r.text;
    currentAction = r.primary === 'paste' || r.primary === 'ask' || r.primary === 'hook' ? r.primary : 'paste';
    if (r.target_lang) targetLang = r.target_lang;
    error = '';
    showMore = false;
  }

  async function confirm() {
    if (!req || busy) return;
    busy = true;
    error = '';
    try {
      await invoke('review_confirm', {
        confirm: {
          id: req.id,
          action: currentAction,
          text,
          target_lang: currentAction === 'translate' ? targetLang : null
        }
      });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function dismiss() {
    if (busy) return;
    await invoke('review_dismiss').catch(() => {});
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      dismiss();
      return;
    }
    if (e.key === 'Enter' && !e.shiftKey && e.metaKey === (sendKey === 'cmd_enter')) {
      e.preventDefault();
      confirm();
      return;
    }
    if (e.metaKey && ['1', '2', '3', '4'].includes(e.key)) {
      const kind = ALL_KINDS[Number(e.key) - 1];
      if (req?.available.includes(kind)) {
        e.preventDefault();
        currentAction = kind;
        showMore = false;
      }
    }
  }

  function onTextareaKeydown(e: KeyboardEvent) {
    // 非发送键变体的 Enter → 手动插入换行(native Cmd+Enter 不换行)
    const isSendCombo = e.metaKey === (sendKey === 'cmd_enter');
    if (e.key === 'Enter' && !e.shiftKey && !isSendCombo) {
      e.preventDefault();
      insertAtCursor('\n');
    }
  }

  function insertAtCursor(insert: string) {
    const ta = textareaEl;
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    text = text.slice(0, start) + insert + text.slice(end);
    requestAnimationFrame(() => {
      ta.selectionStart = ta.selectionEnd = start + insert.length;
      ta.focus();
    });
  }

  function insertVoice(payload: { text: string }) {
    const ta = textareaEl;
    if (!ta) {
      text += payload.text;
      return;
    }
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    text = text.slice(0, start) + payload.text + text.slice(end);
    requestAnimationFrame(() => {
      ta.selectionStart = ta.selectionEnd = start + payload.text.length;
      ta.focus();
    });
  }

  onMount(() => {
    (async () => {
      try {
        const r = await invoke<ReviewRequest | null>('review_get');
        if (r) load(r);
        else await invoke('review_dismiss').catch(() => {});
      } catch {
        /* 非 Tauri 环境 */
      }
      try {
        const cfg = await invoke<{ features: { review: { send_key: string } } }>('get_config');
        const k = cfg?.features?.review?.send_key;
        if (k === 'cmd_enter') sendKey = 'cmd_enter';
      } catch {
        /* fallback: enter */
      }
    })();
    window.addEventListener('keydown', onKeydown);
    let un: (() => void) | null = null;
    let unInsert: (() => void) | null = null;
    let unClose: (() => void) | null = null;
    (async () => {
      // mock 环境返回 {unlisten},真实环境返回函数;两者兼容
      const l = (await listen<ReviewRequest>('review:open', (e) => load(e.payload))) as unknown as
        | (() => void)
        | { unlisten: () => void };
      un = typeof l === 'function' ? l : () => l.unlisten();
      const ins = (await listen<{ text: string }>('review:insert', (e) => insertVoice(e.payload))) as unknown as
        | (() => void)
        | { unlisten: () => void };
      unInsert = typeof ins === 'function' ? ins : () => ins.unlisten();
      try {
        unClose = await getCurrentWindow().onCloseRequested(async () => {
          await invoke('review_dismiss').catch(() => {});
        });
      } catch {
        /* 非 Tauri 环境 */
      }
    })();
    return () => {
      un?.();
      unInsert?.();
      unClose?.();
      window.removeEventListener('keydown', onKeydown);
    };
  });

  const primaryLabel = $derived.by(() => {
    if (currentAction === 'paste') return req?.target_app ? `粘贴到 ${req.target_app}` : '确认粘贴';
    if (currentAction === 'translate') return `翻译为${targetLang}并粘贴`;
    if (currentAction === 'ask') return '转入对话';
    return '发送到 Hook';
  });

  const ambiguous = $derived(!!req && req.alternatives.length > 0);
  const altKind = $derived(req?.alternatives[0] as ActionKind | undefined);
  const sendHint = $derived(sendKey === 'enter' ? '⏎' : '⌘⏎');

  const prettyBody = $derived.by(() => {
    if (!req?.hook) return '';
    try {
      return JSON.stringify(JSON.parse(req.hook.body), null, 2);
    } catch {
      return req.hook.body;
    }
  });
</script>

<div class="flex h-screen w-screen flex-col bg-background text-foreground shadow-popover">
  <!-- 标题栏(拖动区) -->
  <div class="flex items-center justify-between border-b border-border px-4 py-3" data-tauri-drag-region>
    <div class="flex items-center gap-2 text-[15px] font-semibold">
      <span class="inline-block h-2 w-2 rounded-full bg-[#ffb020]"></span>
      校对与发送
    </div>
    <button
      class="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      onclick={dismiss}
      aria-label="取消"
    >
      <X size={16} />
    </button>
  </div>

  {#if !req}
    <div class="flex flex-1 items-center justify-center text-body text-muted-foreground">
      没有待确认的内容
    </div>
  {:else}
    <div class="flex flex-1 flex-col gap-4 overflow-y-auto p-4">
      <!-- 原始转写(折叠) -->
      <div>
        <button
          class="flex items-center gap-1 text-caption text-muted-foreground transition-colors hover:text-foreground"
          onclick={() => (showRaw = !showRaw)}
        >
          <ChevronDown size={13} class="transition-transform {showRaw ? 'rotate-180' : ''}" />
          原始转写
        </button>
        {#if showRaw}
          <div class="mt-1.5 rounded-lg bg-background-alt px-3 py-2 text-caption leading-relaxed text-muted-foreground">
            {req.raw_text}
          </div>
        {/if}
      </div>

      <!-- 翻译原文 -->
      {#if req.original_text}
        <div>
          <div class="mb-1.5 text-caption text-muted-foreground">原文</div>
          <div class="rounded-lg bg-background-alt px-3 py-2 text-caption leading-relaxed text-foreground-alt">
            {req.original_text}
          </div>
        </div>
      {/if}

      <!-- 主文本(可编辑) -->
      <div class="flex min-h-0 flex-1 flex-col">
        <div class="mb-1.5 text-caption text-muted-foreground">
          {currentAction === 'hook' ? '待整理内容' : req.original_text || req.target_lang ? '译文(可修改)' : '识别结果(可修改)'}
        </div>
        <textarea
          class="min-h-[96px] w-full flex-1 resize-none rounded-lg border border-border-input bg-background px-3 py-2 text-body leading-relaxed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
          bind:value={text}
          bind:this={textareaEl}
          onkeydown={onTextareaKeydown}
          placeholder="内容为空"
        ></textarea>
      </div>

      <!-- Hook 预览 -->
      {#if currentAction === 'hook'}
        {#if req.hook}
          <div class="rounded-lg border border-border bg-background-alt p-3">
            <div class="mb-1.5 truncate font-mono text-caption text-muted-foreground" title={req.hook.url}>
              POST {req.hook.url}
            </div>
            {#if Object.keys(req.hook.vars).length > 0}
              <div class="mb-2 flex flex-wrap gap-1.5">
                {#each Object.entries(req.hook.vars) as [k, v]}
                  <span class="rounded bg-muted px-1.5 py-0.5 font-mono text-[11px] text-foreground-alt" title={v}>
                    {k}: {v.length > 24 ? v.slice(0, 24) + '…' : v}
                  </span>
                {/each}
              </div>
            {/if}
            <pre class="max-h-40 overflow-auto rounded bg-background px-2.5 py-2 font-mono text-[12px] leading-relaxed text-foreground-alt">{prettyBody}</pre>
            {#if text !== req.text}
              <div class="mt-1.5 text-[11px] text-muted-foreground">正文已修改，发送前将按新正文重新渲染</div>
            {/if}
          </div>
        {:else}
          <div class="rounded-lg border border-dashed border-border px-3 py-2.5 text-caption text-muted-foreground">
            确认后将按 Hook 整理指令处理此内容并发送
          </div>
        {/if}
      {/if}

      <!-- 翻译语言 -->
      {#if currentAction === 'translate'}
        <div>
          <div class="mb-1.5 text-caption text-muted-foreground">目标语言</div>
          <select
            class="h-9 w-full rounded-lg border border-border-input bg-background px-2.5 text-body focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
            bind:value={targetLang}
          >
            {#each LANGUAGES as lang (lang)}
              <option value={lang}>{lang}</option>
            {/each}
          </select>
        </div>
      {/if}

      {#if error}
        <div class="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-caption text-destructive">
          {error}
        </div>
      {/if}
    </div>

    <!-- 智能去向选择 + 操作 -->
    <div class="border-t border-border p-4">
      {#if ambiguous}
        <div class="mb-3 rounded-lg border border-[#ffb020]/40 bg-[#ffb020]/10 px-3 py-2 text-caption text-[#8a5a00] dark:text-[#ffb020]">
          ⚠ 语义路由不确定，请选择去向
        </div>
      {/if}
      {#if ambiguous && altKind}
        <div class="mb-3 flex flex-col gap-2">
          <button
            class="h-10 rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-body font-medium text-white shadow-btn-primary transition-opacity disabled:opacity-50"
            disabled={busy || !text.trim()}
            onclick={confirm}
          >
            {primaryLabel} <span class="ml-1 text-caption opacity-80">{sendHint}</span>
          </button>
          <button
            class="h-10 rounded-lg border border-border bg-background text-body text-foreground transition-colors hover:bg-muted/50"
            disabled={busy}
            onclick={() => (currentAction = altKind)}
          >
            {KIND_LABEL[altKind]}
          </button>
        </div>
      {:else}
        <button
          class="mb-3 h-10 w-full rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-body font-medium text-white shadow-btn-primary transition-opacity disabled:opacity-50"
          disabled={busy || !text.trim()}
          onclick={confirm}
        >
          {primaryLabel} <span class="ml-1 text-caption opacity-80">{sendHint}</span>
        </button>
      {/if}

      {#if showMore}
        <div class="mb-3 grid grid-cols-4 gap-2">
          {#each ALL_KINDS as kind, i (kind)}
            <button
              class="rounded-lg border px-1 py-2 text-center text-caption transition-colors {currentAction === kind
                ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from/15 to-btn-primary-to/10 text-foreground font-medium'
                : req?.available.includes(kind)
                  ? 'border-border bg-background text-foreground-alt hover:bg-muted/50'
                  : 'cursor-not-allowed border-border bg-background text-muted-foreground opacity-40'}"
              onclick={() => {
                if (req?.available.includes(kind)) {
                  currentAction = kind;
                  showMore = false;
                }
              }}
              disabled={!req?.available.includes(kind)}
            >
              {KIND_LABEL[kind]}<span class="ml-1 opacity-60">⌘{i + 1}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="flex items-center justify-between">
        <button
          class="text-caption text-muted-foreground underline-offset-2 transition-colors hover:text-foreground hover:underline"
          onclick={() => (showMore = !showMore)}
        >
          其他去向…
        </button>
        <button
          class="rounded-lg border border-border px-4 py-2 text-caption text-foreground transition-colors hover:bg-muted/50 disabled:opacity-50"
          disabled={busy}
          onclick={dismiss}
        >
          取消
        </button>
      </div>
    </div>
  {/if}
</div>
