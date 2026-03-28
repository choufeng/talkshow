<script lang="ts">
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

<div class="password-input">
  {#if visible}
    <input
      class="field"
      type="text"
      {placeholder}
      {value}
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
  {:else}
    <div class="field masked">{mask(value)}</div>
  {/if}
  <button class="toggle-btn" onclick={() => visible = !visible} title={visible ? '隐藏' : '显示'}>
    👁
  </button>
</div>

<style>
  .password-input {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .field {
    flex: 1;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 11px;
    outline: none;
  }

  .field:focus {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }

  .masked {
    color: #666;
    user-select: none;
  }

  .toggle-btn {
    padding: 4px 8px;
    font-size: 12px;
    cursor: pointer;
    color: #888;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  .toggle-btn:hover {
    background: #f0f0f0;
  }
</style>
