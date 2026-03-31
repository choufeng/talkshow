<script lang="ts">
  import { Dialog } from "bits-ui";
  import { X } from "lucide-svelte";

  interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    title: string;
    description?: string;
    children?: import('svelte').Snippet;
    footer?: import('svelte').Snippet;
  }

  let { open, onOpenChange, title, description, children, footer }: Props = $props();
</script>

<Dialog.Root {open} {onOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
    <Dialog.Content class="fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-xl border border-border bg-background-alt p-6 shadow-popover data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95">
      <div class="flex justify-between items-center mb-4">
        <div>
          <Dialog.Title class="text-base font-semibold text-foreground">{title}</Dialog.Title>
          {#if description}
            <Dialog.Description class="text-sm text-foreground-alt mt-0.5">{description}</Dialog.Description>
          {/if}
        </div>
        <Dialog.Close class="rounded-md p-1 text-muted-foreground hover:text-foreground transition-colors">
          <X class="h-5 w-5" />
        </Dialog.Close>
      </div>

      {#if children}
        <div class="space-y-4">
          {@render children()}
        </div>
      {/if}

      {#if footer}
        <div class="flex justify-end gap-2 mt-5 pt-4 border-t border-border">
          {@render footer()}
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
