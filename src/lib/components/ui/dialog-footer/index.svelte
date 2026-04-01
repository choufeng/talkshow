<script lang="ts">
  interface Props {
    onCancel: () => void;
    onConfirm: () => void;
    cancelText?: string;
    confirmText?: string;
    confirmDisabled?: boolean;
    confirmVariant?: 'primary' | 'danger';
  }

  let {
    onCancel,
    onConfirm,
    cancelText = '取消',
    confirmText = '确认',
    confirmDisabled = false,
    confirmVariant = 'primary'
  }: Props = $props();

  const confirmClasses = $derived(confirmVariant === 'danger'
    ? 'rounded-md bg-red-600 px-4 py-2 text-body text-white hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed'
    : 'rounded-md bg-gradient-to-b from-btn-primary-from to-btn-primary-to px-4 py-2 text-body text-white hover:opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-btn-primary');
</script>

<div class="flex justify-end gap-2 mt-4">
  <button
    class="rounded-md border border-btn-secondary-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to px-4 py-2 text-body text-accent-foreground hover:bg-muted transition-colors shadow-btn-secondary"
    onclick={onCancel}
  >
    {cancelText}
  </button>
  <button
    class={confirmClasses}
    disabled={confirmDisabled}
    onclick={onConfirm}
  >
    {confirmText}
  </button>
</div>
