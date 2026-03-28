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

<div class="shortcut-item">
  <div class="info">
    <h4>{label}</h4>
    <p>{description}</p>
  </div>
  <div class="controls">
    <div class="shortcut-display" class:recording={isRecording}>
      {#if isRecording}
        请按下快捷键...
      {:else}
        {formatShortcut(currentValue)}
      {/if}
    </div>
    {#if isRecording}
      <button class="btn cancel" onclick={cancelRecording}>
        取消
      </button>
    {:else}
      <button class="btn modify" onclick={startRecording}>
        修改
      </button>
    {/if}
  </div>
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .shortcut-item {
    background: white;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 16px;
  }

  .info h4 {
    margin: 0 0 4px 0;
    font-size: 14px;
    font-weight: 600;
    color: #333;
  }

  .info p {
    margin: 0;
    font-size: 13px;
    color: #666;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
  }

  .shortcut-display {
    background: #f0f0f0;
    padding: 8px 16px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 14px;
    min-width: 120px;
    text-align: center;
  }

  .shortcut-display.recording {
    background: #396cd8;
    color: white;
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    border: 1px solid #ccc;
    background: white;
    cursor: pointer;
    font-size: 13px;
  }

  .btn:hover {
    border-color: #396cd8;
  }

  .btn.cancel {
    border-color: #396cd8;
    background: #396cd8;
    color: white;
  }

  .error {
    margin: 8px 0 0 0;
    color: #d32f2f;
    font-size: 12px;
  }
</style>