<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { X, ClipboardPaste, Languages, MessageCircle, Webhook, ChevronDown } from 'lucide-svelte';
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
    action: 'paste' | 'ask' | 'hook';
    target_app: string | null;
    hook: HookPreview | null;
    can_hook: boolean;
    audio_path: string;
  }

  const LANGUAGES = [
    'English', '中文', '日本語', '한국어', 'Français', 'Deutsch',
    'Español', 'Português', 'Italiano', 'Русский', 'العربية', 'हिन्दी'
  ];

  let req = $state<ReviewRequest | null>(null);
  let text = $state('');
  let action = $state<'paste' | 'translate' | 'ask' | 'hook'>('paste');
  let targetLang = $state('English');
  let busy = $state(false);
  let error = $state('');
  let showRaw = $state(false);

  function load(r: ReviewRequest) {
    req = r;
    text = r.text;
    action = r.action === 'paste' || r.action === 'ask' || r.action === 'hook' ? r.action : 'paste';
    if (r.target_lang) targetLang = r.target_lang;
    error = '';
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
    })();
    const kd = (e: KeyboardEvent) => {
      if (e.key === 'Escape') dismiss();
    };
    window.addEventListener('keydown', kd);
    let un: (() => void) | null = null;
    let unClose: (() => void) | null = null;
    (async () => {
      // mock 环境返回 {unlisten},真实环境返回函数;两者兼容
      const l = (await listen<ReviewRequest>('review:open', (e) => load(e.payload))) as unknown as
        | (() => void)
        | { unlisten: () => void };
      un = typeof l === 'function' ? l : () => l.unlisten();
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
      unClose?.();
      window.removeEventListener('keydown', kd);
    };
  });

  async function confirm() {
    if (!req || busy) return;
    busy = true;
    error = '';
    try {
      await invoke('review_confirm', {
        confirm: {
          id: req.id,
          action,
          text,
          target_lang: action === 'translate' ? targetLang : null
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

  const ACTIONS = $derived([
    {
      key: 'paste' as const,
      icon: ClipboardPaste,
      label: '粘贴',
      desc: req?.target_app ? `粘贴到 ${req.target_app}` : '粘贴到原应用',
      available: true
    },
    {
      key: 'translate' as const,
      icon: Languages,
      label: '翻译',
      desc: '翻译为目标语言后粘贴',
      available: true
    },
    {
      key: 'ask' as const,
      icon: MessageCircle,
      label: '问询',
      desc: '作为问题打开对话',
      available: true
    },
    {
      key: 'hook' as const,
      icon: Webhook,
      label: 'Hook',
      desc: '整理后发送到后端',
      available: req?.can_hook ?? false
    }
  ]);

  const confirmLabel = $derived(
    action === 'paste' ? (req?.target_app ? `粘贴到 ${req.target_app}` : '确认粘贴')
    : action === 'translate' ? `翻译为${targetLang}并粘贴`
    : action === 'ask' ? '转入对话'
    : '发送到 Hook'
  );

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
          {action === 'hook' ? '待整理内容' : req.original_text || req.target_lang ? '译文(可修改)' : '识别结果(可修改)'}
        </div>
        <textarea
          class="min-h-[96px] w-full flex-1 resize-none rounded-lg border border-border-input bg-background px-3 py-2 text-body leading-relaxed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
          bind:value={text}
          placeholder="内容为空"
        ></textarea>
      </div>

      <!-- Hook 预览 -->
      {#if action === 'hook'}
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
      {#if action === 'translate'}
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

    <!-- 去向选择 + 操作 -->
    <div class="border-t border-border p-4">
      <div class="mb-3 grid grid-cols-4 gap-2">
        {#each ACTIONS as a (a.key)}
          {@const Icon = a.icon}
          <button
            class="flex flex-col items-center gap-1 rounded-lg border px-1 py-2.5 text-center transition-colors {action === a.key
              ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from/15 to-btn-primary-to/10 text-foreground'
              : a.available
                ? 'border-border bg-background text-foreground-alt hover:bg-muted/50'
                : 'cursor-not-allowed border-border bg-background text-muted-foreground opacity-40'}"
            onclick={() => a.available && (action = a.key)}
            disabled={!a.available}
            title={a.desc}
          >
            <Icon size={17} />
            <span class="text-caption font-medium">{a.label}</span>
          </button>
        {/each}
      </div>
      <div class="flex gap-2.5">
        <button
          class="h-10 flex-1 rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-body font-medium text-white shadow-btn-primary transition-opacity disabled:opacity-50"
          disabled={busy || !text.trim()}
          onclick={confirm}
        >
          {busy ? '执行中…' : confirmLabel}
        </button>
        <button
          class="h-10 rounded-lg border border-border px-5 text-body text-foreground transition-colors hover:bg-muted/50 disabled:opacity-50"
          disabled={busy}
          onclick={dismiss}
        >
          取消
        </button>
      </div>
    </div>
  {/if}
</div>
