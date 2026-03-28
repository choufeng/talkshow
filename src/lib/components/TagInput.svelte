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

<div class="tag-input">
  <div class="tags">
    {#each tags as tag}
      <span class="tag">
        {tag}
        <button class="tag-remove" onclick={() => onRemove(tag)}>✕</button>
      </span>
    {/each}
  </div>
  {#if isInputVisible}
    <input
      class="tag-field"
      type="text"
      {placeholder}
      bind:value={inputValue}
      onkeydown={handleKeydown}
      onblur={() => { addTag(); isInputVisible = false; }}
    />
  {:else}
    <button class="add-tag-btn" onclick={showInput}>+ 添加模型</button>
  {/if}
</div>

<style>
  .tag-input {
    margin-top: 4px;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 4px;
  }

  .tag {
    background: #e8f0fe;
    color: #396cd8;
    border-radius: 4px;
    padding: 3px 8px;
    font-size: 10px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .tag-remove {
    background: none;
    border: none;
    color: #396cd8;
    cursor: pointer;
    font-size: 10px;
    padding: 0;
    opacity: 0.6;
  }

  .tag-remove:hover {
    opacity: 1;
  }

  .tag-field {
    width: 100%;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 5px 8px;
    font-size: 11px;
    outline: none;
  }

  .tag-field:focus {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }

  .add-tag-btn {
    background: none;
    border: none;
    color: #396cd8;
    cursor: pointer;
    font-size: 10px;
    padding: 0;
  }

  .add-tag-btn:hover {
    text-decoration: underline;
  }
</style>
