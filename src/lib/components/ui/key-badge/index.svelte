<script lang="ts">
  interface Props {
    label: string;
  }

  let { label }: Props = $props();
</script>

<span
  class="inline-flex items-center justify-center min-w-[40px] h-[40px] px-3
         rounded-md border-b-[2px]
         bg-gradient-to-b from-key-bg-from to-key-bg-to
         border-key-border
         shadow-[0_1px_2px_rgba(0,0,0,0.12),inset_0_1px_0_rgba(255,255,255,0.08)]
         dark:shadow-[0_1px_3px_rgba(0,0,0,0.3),inset_0_1px_0_rgba(255,255,255,0.05)]
         font-sans text-base text-key-text select-none"
>
  {label}
</span>
