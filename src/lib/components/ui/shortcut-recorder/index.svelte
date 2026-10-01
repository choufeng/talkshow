<script lang="ts">
  import KeyBadge from '$lib/components/ui/key-badge/index.svelte';
  import { parseKeys, parseShortcutList, joinShortcuts } from '$lib/utils/shortcut';
  import { Plus, X } from 'lucide-svelte';

  interface Props {
    label: string;
    description: string;
    value: string;
    onUpdate: (shortcut: string) => Promise<void>;
  }

  let { label, description, value, onUpdate }: Props = $props();

  /** 录制目标: -1 = 新增一组, >=0 = 替换该 index */
  let recordingIndex = $state<number | null>(null);
  let error = $state<string | null>(null);
  let groups = $state<string[]>([]);

  $effect(() => {
    groups = parseShortcutList(value);
  });

  function handleKeydown(event: KeyboardEvent) {
    if (recordingIndex === null) return;

    event.preventDefault();
    event.stopPropagation();

    if (event.key === 'Escape') {
      recordingIndex = null;
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

    saveShortcut([...modifiers, key].join('+'));
  }

  async function saveShortcut(shortcut: string) {
    const index = recordingIndex;
    if (index === null) return;

    // 同事件内去重
    if (groups.some((g, i) => i !== index && g === shortcut)) {
      error = '该快捷键已绑定';
      recordingIndex = null;
      return;
    }

    const next =
      index === -1 ? [...groups, shortcut] : groups.map((g, i) => (i === index ? shortcut : g));

    try {
      error = null;
      await onUpdate(joinShortcuts(next));
      recordingIndex = null;
    } catch (e) {
      error = e instanceof Error ? e.message : '保存失败';
      recordingIndex = null;
    }
  }

  async function removeGroup(index: number) {
    if (groups.length <= 1) return;
    try {
      error = null;
      await onUpdate(joinShortcuts(groups.filter((_, i) => i !== index)));
    } catch (e) {
      error = e instanceof Error ? e.message : '保存失败';
    }
  }

  function startRecording(index: number) {
    recordingIndex = index;
    error = null;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="rounded-xl border border-border bg-background-alt p-6 mb-5">
  <div>
    <h4 class="text-subheading font-semibold text-foreground m-0">{label}</h4>
    <p class="text-body text-foreground-alt m-0 mt-1">{description}</p>
  </div>
  <div class="flex flex-wrap items-center gap-3 mt-4">
    {#each groups as group, index (index)}
      {#if recordingIndex === index}
        <div class="rounded-md px-5 py-2.5 text-subheading min-w-[130px] text-center bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white shadow-btn-primary">
          请按下快捷键...
        </div>
      {:else}
        <div class="flex items-center gap-1">
          {#each parseKeys(group) as key (key + index)}
            <KeyBadge label={key} />
          {/each}
        </div>
        <button
          class="text-caption text-foreground-alt hover:text-foreground transition-colors"
          onclick={() => startRecording(index)}
          title="点击此组重新录制"
        >
          修改
        </button>
        {#if groups.length > 1}
          <button
            class="text-foreground-alt hover:text-destructive transition-colors"
            onclick={() => removeGroup(index)}
            aria-label="删除此组快捷键"
            title="删除此组"
          >
            <X size={14} />
          </button>
        {/if}
      {/if}
      {#if index < groups.length - 1}
        <span class="text-caption text-muted-foreground">或</span>
      {/if}
    {/each}
    {#if recordingIndex === -1}
      <div class="rounded-md px-5 py-2.5 text-subheading min-w-[130px] text-center bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white shadow-btn-primary">
        请按下快捷键...
      </div>
    {:else}
      <button
        class="inline-flex items-center justify-center rounded-md bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to border border-btn-secondary-border px-4 py-2.5 text-body font-medium text-accent-foreground transition-colors hover:opacity-90 shadow-btn-secondary"
        onclick={() => startRecording(-1)}
        title="添加一组快捷键"
      >
        <Plus size={15} class="inline -align-[2px]" />
      </button>
    {/if}
    {#if recordingIndex !== null}
      <button
        class="inline-flex items-center justify-center rounded-md bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to border border-btn-secondary-border px-5 py-2.5 text-body font-medium text-accent-foreground transition-colors hover:opacity-90 shadow-btn-secondary"
        onclick={() => (recordingIndex = null)}
      >
        取消
      </button>
    {/if}
  </div>
  {#if error}
    <p class="text-body text-destructive m-0 mt-2">{error}</p>
  {/if}
</div>
