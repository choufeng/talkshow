<script lang="ts">
  import { Eye, EyeOff, Check, X } from 'lucide-svelte';

  interface Props {
    value: string;
    placeholder?: string;
    mode?: 'password' | 'text';
    onChange: (value: string) => void | Promise<void>;
    'aria-labelledby'?: string;
    'aria-label'?: string;
  }

  let { value, placeholder = '', mode = 'password', onChange, 'aria-labelledby': ariaLabelledby, 'aria-label': ariaLabel }: Props = $props();

  let visible = $state(false);
  let editing = $state(false);
  let editValue = $state('');

  $effect(() => {
    if (!editing) {
      editValue = value;
    }
  });

  function mask(val: string): string {
    if (!val) return '';
    return val.slice(0, 3) + '•'.repeat(Math.max(val.length - 3, 6));
  }

  function startEdit() {
    editValue = value;
    editing = true;
  }

  async function confirm() {
    await onChange(editValue);
    editing = false;
  }

  function cancel() {
    editValue = value;
    editing = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') cancel();
    if (e.key === 'Enter') confirm();
  }
</script>

<div class="flex items-center gap-1 min-w-0">
  {#if editing}
    <input
      class="flex h-10 min-w-0 flex-1 rounded-md border border-border-input bg-background px-4 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type={mode === 'password' && !visible ? 'password' : 'text'}
      bind:value={editValue}
      {placeholder}
      onkeydown={handleKeyDown}
    />
    <button
      class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-input bg-accent/10 text-accent-foreground transition-colors hover:bg-accent/20"
      onclick={confirm}
      title="确认"
    >
      <Check class="h-4 w-4" />
    </button>
    <button
      class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-input bg-destructive/10 text-destructive transition-colors hover:bg-destructive/20"
      onclick={cancel}
      title="取消"
    >
      <X class="h-4 w-4" />
    </button>
  {:else}
    <div
      role="button"
      tabindex="0"
      aria-labelledby={ariaLabelledby}
      aria-label={ariaLabel}
      class="flex h-10 min-w-0 flex-1 items-center truncate rounded-md border border-border-input bg-background px-4 text-body text-muted-foreground cursor-pointer select-none hover:bg-muted/50 transition-colors"
      onclick={startEdit}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') startEdit(); }}
      title="点击编辑"
    >
      {#if mode === 'password'}
        {visible ? value : mask(value)}
      {:else}
        {value || placeholder}
      {/if}
    </div>
    {#if mode === 'password'}
      <button
        class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border-input bg-background text-muted-foreground transition-colors hover:bg-muted"
        onclick={() => visible = !visible}
        title={visible ? '隐藏' : '显示'}
      >
        {#if visible}
          <Eye class="h-4 w-4" />
        {:else}
          <EyeOff class="h-4 w-4" />
        {/if}
      </button>
    {/if}
  {/if}
</div>