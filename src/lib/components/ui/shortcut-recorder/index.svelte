<script lang="ts">
  import KeyBadge from '$lib/components/ui/key-badge/index.svelte';

  interface Props {
    label: string;
    description: string;
    value: string;
    onUpdate: (shortcut: string) => Promise<void>;
  }

  let { label, description, value, onUpdate }: Props = $props();

  let isRecording = $state(false);
  let currentValue = $state(value);
  let error = $state<string | null>(null);

  $effect(() => {
    currentValue = value;
  });

  const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent);

  const MODIFIER_DISPLAY: Record<string, string> = isMac
    ? { Control: '⌃', Shift: '⇧', Alt: '⌥', Command: '⌘', Super: '⌘' }
    : { Control: 'Ctrl', Shift: 'Shift', Alt: 'Alt', Command: 'Cmd', Super: 'Cmd' };

  const KEY_DISPLAY: Record<string, string> = {
    Quote: "'",
    Backslash: '\\',
    Space: 'Space',
  };

  function parseKeys(shortcut: string): string[] {
    return shortcut.split('+').map((key) => {
      if (MODIFIER_DISPLAY[key]) return MODIFIER_DISPLAY[key];
      if (KEY_DISPLAY[key]) return KEY_DISPLAY[key];
      return key.replace('Key', '').replace('Digit', '');
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    if (event.key === 'Escape') {
      isRecording = false;
      currentValue = value;
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push('Control');
    if (event.shiftKey) modifiers.push('Shift');
    if (event.altKey) modifiers.push('Alt');
    if (event.metaKey) modifiers.push('Command');

    let key = event.code;
    if (key.startsWith('Key')) {
      key = key;
    } else if (key.startsWith('Digit')) {
      key = key;
    } else if (key === 'Quote') {
      key = 'Quote';
    } else if (key === 'Backslash') {
      key = 'Backslash';
    } else if (key === 'Space') {
      key = 'Space';
    } else {
      return;
    }

    if (modifiers.length === 0) {
      error = '请至少使用一个修饰键 (Ctrl, Shift, Alt, Command)';
      return;
    }

    const shortcut = [...modifiers, key].join('+');
    saveShortcut(shortcut);
  }

  async function saveShortcut(shortcut: string) {
    try {
      error = null;
      await onUpdate(shortcut);
      currentValue = shortcut;
      isRecording = false;
    } catch (e) {
      error = e instanceof Error ? e.message : '保存失败';
    }
  }

  function startRecording() {
    isRecording = true;
    error = null;
  }

  function cancelRecording() {
    isRecording = false;
    currentValue = value;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="rounded-xl border border-border bg-background-alt p-6 mb-5">
  <div>
    <h4 class="text-base font-semibold text-foreground m-0">{label}</h4>
    <p class="text-sm text-foreground-alt m-0 mt-1">{description}</p>
  </div>
  <div class="flex items-center gap-3 mt-4">
    {#if isRecording}
      <div         class="rounded-md px-5 py-2.5 text-base min-w-[130px] text-center bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white shadow-btn-primary">
        请按下快捷键...
      </div>
    {:else}
      <div class="flex items-center gap-2">
        {#each parseKeys(currentValue) as key}
          <KeyBadge label={key} />
        {/each}
      </div>
    {/if}
    {#if isRecording}
      <button
        class="inline-flex items-center justify-center rounded-md bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to border border-btn-secondary-border px-5 py-2.5 text-sm font-medium text-accent-foreground transition-colors hover:opacity-90 shadow-btn-secondary"
        onclick={cancelRecording}
      >
        取消
      </button>
    {:else}
      <button
        class="inline-flex items-center justify-center rounded-md bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-5 py-2.5 text-sm font-medium text-white transition-colors hover:opacity-90 shadow-btn-primary"
        onclick={startRecording}
      >
        修改
      </button>
    {/if}
  </div>
  {#if error}
    <p class="text-sm text-destructive m-0 mt-2">{error}</p>
  {/if}
</div>
