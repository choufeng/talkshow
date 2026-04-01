<script lang="ts">
  interface Props {
    checked: boolean;
    onCheckedChange?: (checked: boolean) => void;
    size?: 'sm' | 'md';
    disabled?: boolean;
    ariaLabel?: string;
  }

  let { checked, onCheckedChange, size = 'md', disabled = false, ariaLabel }: Props = $props();

  const sizes = {
    sm: { button: 'h-5 w-9', thumb: 'h-3.5 w-3.5', translate: 'translate-x-4' },
    md: { button: 'h-6 w-11', thumb: 'h-4 w-4', translate: 'translate-x-5' }
  };

  function toggle() {
    if (disabled) return;
    onCheckedChange?.(!checked);
  }
</script>

<button
  class="relative inline-flex shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {checked ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to shadow-btn-primary' : 'bg-gradient-to-b from-toggle-off-from to-toggle-off-to shadow-btn-secondary'} {disabled ? 'cursor-not-allowed opacity-50' : ''} {sizes[size].button}"
  role="switch"
  aria-checked={checked}
  aria-label={ariaLabel}
  disabled={disabled}
  onclick={toggle}
>
  <span class="pointer-events-none inline-block transform rounded-full bg-gradient-to-b from-toggle-thumb-from to-toggle-thumb-to shadow ring-0 transition duration-200 ease-in-out {checked ? sizes[size].translate : 'translate-x-0'} {sizes[size].thumb}"></span>
</button>
