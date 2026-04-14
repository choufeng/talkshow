export interface DialogStateOptions {
  initialOpen?: boolean;
  onReset?: () => void;
}

export function createDialogState(options: DialogStateOptions = {}) {
  let isOpen = $state(options.initialOpen ?? false);
  let resetFn = $state<(() => void) | null>(options.onReset ?? null);

  return {
    get isOpen() {
      return isOpen;
    },
    open() {
      if (isOpen) {
        isOpen = false;
      }
      isOpen = true;
    },
    close() {
      isOpen = false;
      resetFn?.();
    },
    onOpenChange(open: boolean) {
      if (open) {
        isOpen = true;
      } else {
        this.close();
      }
    },
    setReset(fn: () => void) {
      resetFn = fn;
    }
  };
}
