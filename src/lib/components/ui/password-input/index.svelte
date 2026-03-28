<script lang="ts">
  import { Eye, EyeOff } from 'lucide-svelte';

  interface Props {
    value: string;
    placeholder?: string;
    onChange: (value: string) => void;
  }

  let { value, placeholder = '', onChange }: Props = $props();

  let visible = $state(false);

  function mask(val: string): string {
    if (!val) return '';
    return val.slice(0, 3) + '•'.repeat(Math.max(val.length - 3, 6));
  }
</script>

<div class="flex items-center gap-1">
  {#if visible}
    <input
      class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type="text"
      {placeholder}
      {value}
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
  {:else}
    <div class="flex h-8 flex-1 items-center rounded-md border border-border-input bg-background px-3 text-xs text-muted-foreground select-none">
      {mask(value)}
    </div>
  {/if}
  <button
    class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border-input bg-background text-muted-foreground transition-colors hover:bg-muted"
    onclick={() => visible = !visible}
    title={visible ? '隐藏' : '显示'}
  >
    {#if visible}
      <Eye class="h-3.5 w-3.5" />
    {:else}
      <EyeOff class="h-3.5 w-3.5" />
    {/if}
  </button>
</div>
