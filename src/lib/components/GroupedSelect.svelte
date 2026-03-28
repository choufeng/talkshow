<script lang="ts">
  interface Group {
    label: string;
    items: { value: string; label: string }[];
  }

  interface Props {
    value: string;
    groups: Group[];
    placeholder?: string;
    onChange: (value: string) => void;
  }

  let { value, groups, placeholder = '请选择', onChange }: Props = $props();

  let isOpen = $state(false);

  function toggle() {
    isOpen = !isOpen;
  }

  function select(val: string) {
    onChange(val);
    isOpen = false;
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.grouped-select')) {
      isOpen = false;
    }
  }

  function getDisplayLabel(): string {
    for (const group of groups) {
      for (const item of group.items) {
        if (item.value === value) return `${group.label} — ${item.label}`;
      }
    }
    return placeholder;
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="grouped-select">
  <div class="select-trigger" class:open={isOpen} onclick={toggle}>
    <span class="select-value">{getDisplayLabel()}</span>
    <span class="select-arrow">▾</span>
  </div>
  {#if isOpen}
    <div class="select-dropdown">
      {#each groups as group}
        <div class="select-group">
          <div class="select-group-label">{group.label}</div>
          {#each group.items as item}
            <div
              class="select-option"
              class:selected={item.value === value}
              onclick={() => select(item.value)}
            >
              {item.label}
              {#if item.value === value}
                <span class="check">✓</span>
              {/if}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .grouped-select {
    position: relative;
  }

  .select-trigger {
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
  }

  .select-trigger:hover {
    border-color: #ccc;
  }

  .select-trigger.open {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }

  .select-arrow {
    color: #888;
    font-size: 12px;
  }

  .select-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    z-index: 100;
    max-height: 240px;
    overflow-y: auto;
  }

  .select-group-label {
    padding: 6px 10px;
    font-size: 10px;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid #eee;
  }

  .select-option {
    padding: 6px 10px;
    font-size: 12px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .select-option:hover {
    background: #f5f5f5;
  }

  .select-option.selected {
    background: #e8f0fe;
    color: #396cd8;
  }

  .check {
    color: #396cd8;
    font-size: 11px;
  }
</style>
