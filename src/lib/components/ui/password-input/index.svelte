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
      class="flex h-10 w-full rounded-md border border-border-input bg-background px-4 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-xs file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type="text"
      {placeholder}
      {value}
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
  {:else}
    <div class="flex h-10 flex-1 items-center rounded-md border border-border-input bg-background px-4 text-sm text-muted-foreground select-none">
      {mask(value)}
    </div>
  {/if}
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
</div>
