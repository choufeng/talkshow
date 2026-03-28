<script lang="ts">
  interface Props {
    tags: string[];
    onAdd: (tag: string) => void;
    onRemove: (tag: string) => void;
    placeholder?: string;
  }

  let { tags, onAdd, onRemove, placeholder = '添加...' }: Props = $props();

  let inputValue = $state('');
  let isInputVisible = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addTag();
    } else if (event.key === 'Escape') {
      isInputVisible = false;
      inputValue = '';
    }
  }

  function addTag() {
    const trimmed = inputValue.trim();
    if (trimmed && !tags.includes(trimmed)) {
      onAdd(trimmed);
      inputValue = '';
    }
  }

  function showInput() {
    isInputVisible = true;
  }
</script>

<div class="mt-1">
  <div class="flex flex-wrap gap-1 mb-1">
    {#each tags as tag}
      <span class="inline-flex items-center gap-1 rounded bg-accent px-2 py-0.5 text-[10px] text-accent-foreground">
        {tag}
        <button
          class="opacity-60 hover:opacity-100 transition-opacity"
          onclick={() => onRemove(tag)}
        >
          ✕
        </button>
      </span>
    {/each}
  </div>
  {#if isInputVisible}
    <input
      class="flex h-7 w-full rounded-md border border-border-input bg-background px-2 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
      type="text"
      {placeholder}
      bind:value={inputValue}
      onkeydown={handleKeydown}
      onblur={() => { addTag(); isInputVisible = false; }}
    />
  {:else}
    <button
      class="text-xs text-accent-foreground hover:underline"
      onclick={showInput}
    >
      + 添加模型
    </button>
  {/if}
</div>
