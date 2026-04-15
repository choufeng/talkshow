<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeWithError } from '$lib/ai/shared';
  import { TriangleAlert, X } from 'lucide-svelte';

  interface HealthCheckResult {
    id: string;
    name: string;
    status: { status: string; message?: string; fix_hint?: string };
  }

  let warnings = $state<HealthCheckResult[]>([]);
  let dismissed = $state(false);

  onMount(async () => {
    const result = await invokeWithError<HealthCheckResult[]>('get_health_status');
    if (result) {
      warnings = result.filter((r) => r.status.status === 'warning');
    }
  });

  function dismiss() {
    dismissed = true;
  }
</script>

{#if warnings.length > 0 && !dismissed}
  <div class="bg-yellow-500/10 border-b border-yellow-500/30 px-6 py-3">
    {#each warnings as warning (warning.id)}
      <div class="flex items-start gap-3">
        <TriangleAlert size={18} class="text-yellow-500 shrink-0 mt-0.5" />
        <div class="flex-1 min-w-0">
          <p class="text-sm text-foreground">{warning.status.message}</p>
          <p class="text-xs text-muted-foreground mt-1">{warning.status.fix_hint}</p>
        </div>
        <button onclick={dismiss} class="text-muted-foreground hover:text-foreground shrink-0">
          <X size={16} />
        </button>
      </div>
    {/each}
  </div>
{/if}
