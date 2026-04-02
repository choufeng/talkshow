<script lang="ts">
  import { Select } from "bits-ui";
  import { ChevronDown, Check } from "lucide-svelte";

  interface Group {
    label: string;
    items: { value: string; label: string }[];
  }

  interface Props {
    value: string;
    groups: Group[];
    placeholder?: string;
    onChange: (value: string) => void;
    'aria-labelledby'?: string;
    'aria-label'?: string;
  }

  let { value, groups, placeholder = "请选择", onChange, 'aria-labelledby': ariaLabelledby, 'aria-label': ariaLabel }: Props = $props();

  function getDisplayLabel(): string {
    for (const group of groups) {
      for (const item of group.items) {
        if (item.value === value) return group.label ? `${group.label} — ${item.label}` : item.label;
      }
    }
    return placeholder;
  }
</script>

<Select.Root
  type="single"
  {value}
  onValueChange={(v) => { if (v) onChange(v); }}
>
  <Select.Trigger
    aria-labelledby={ariaLabelledby}
    aria-label={ariaLabel}
    class="flex h-10 w-full items-center justify-between rounded-md border border-border-input bg-background px-4 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-accent-foreground/20 focus:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-50"
  >
    <span class="truncate">{getDisplayLabel()}</span>
    <ChevronDown class="h-5 w-5 shrink-0 opacity-50" />
  </Select.Trigger>
  <Select.Portal>
    <Select.Content
      class="z-50 max-h-64 min-w-[var(--bits-select-anchor-width)] w-[var(--bits-select-anchor-width)] rounded-xl border border-border bg-background-alt p-1.5 text-foreground shadow-popover data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2"
      sideOffset={4}
    >
      <Select.ScrollUpButton class="flex h-4 items-center justify-center">
        <ChevronDown class="h-3 w-3 rotate-180" />
      </Select.ScrollUpButton>
      <Select.Viewport>
        {#each groups as group}
          <Select.Group>
            <Select.GroupHeading class="px-3 py-1.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/60">
              {group.label}
            </Select.GroupHeading>
            {#each group.items as item}
              <Select.Item
                value={item.value}
                label={item.label}
                class="relative flex w-full cursor-default select-none items-center rounded-md py-2 pl-3 pr-8 text-sm font-medium text-foreground outline-none data-highlighted:bg-gradient-to-b data-highlighted:from-btn-primary-from data-highlighted:to-btn-primary-to data-highlighted:text-white data-highlighted:shadow-btn-primary data-disabled:pointer-events-none data-disabled:opacity-50"
              >
                {#snippet children({ selected })}
                  <span class="absolute right-2 flex h-3.5 w-3.5 items-center justify-center">
                    {#if selected}
                      <Check class="h-4 w-4" />
                    {/if}
                  </span>
                  {item.label}
                {/snippet}
              </Select.Item>
            {/each}
          </Select.Group>
        {/each}
      </Select.Viewport>
      <Select.ScrollDownButton class="flex h-4 items-center justify-center">
        <ChevronDown class="h-3 w-3" />
      </Select.ScrollDownButton>
    </Select.Content>
  </Select.Portal>
</Select.Root>
