<script lang="ts">
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

  function formatShortcut(shortcut: string): string {
    return shortcut
      .replace('Control', '⌃')
      .replace('Shift', '⇧')
      .replace('Alt', '⌥')
      .replace('Command', '⌘')
      .replace('Super', '⌘')
      .replace('Quote', "'")
      .replace('Backslash', '\\')
      .replace('Key', '')
      .replace('Digit', '')
      .replace('+', ' ');
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

<div class="rounded-lg border border-border bg-background p-5 mb-4">
  <div>
    <h4 class="text-sm font-semibold text-foreground m-0">{label}</h4>
    <p class="text-[13px] text-foreground-alt m-0 mt-1">{description}</p>
  </div>
  <div class="flex items-center gap-3 mt-3">
    <div class="rounded-md px-4 py-2 font-mono text-sm min-w-[120px] text-center {isRecording ? 'bg-accent text-accent-foreground' : 'bg-muted text-foreground'}">
      {#if isRecording}
        请按下快捷键...
      {:else}
        {formatShortcut(currentValue)}
      {/if}
    </div>
    {#if isRecording}
      <button
        class="inline-flex items-center justify-center rounded-md border border-accent-foreground bg-accent-foreground px-4 py-2 text-sm font-medium text-accent transition-colors hover:bg-accent-foreground/90"
        onclick={cancelRecording}
      >
        取消
      </button>
    {:else}
      <button
        class="inline-flex items-center justify-center rounded-md border border-border-input bg-background px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-muted hover:border-foreground/20"
        onclick={startRecording}
      >
        修改
      </button>
    {/if}
  </div>
  {#if error}
    <p class="text-xs text-destructive m-0 mt-2">{error}</p>
  {/if}
</div>
