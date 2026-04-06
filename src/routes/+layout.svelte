<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import type { Snippet } from 'svelte';
  import { House, Settings, Bot, ScrollText, Sparkles } from 'lucide-svelte';
  import { onboarding } from '$lib/stores/onboarding';
  import OnboardingWizard from '$lib/components/onboarding/OnboardingWizard.svelte';

  let { children }: { children: Snippet } = $props();

  let isLoading = $state(true);

  onMount(() => {
    onboarding.load().then(() => {
      isLoading = false;
    });
  });

  let isFloatingIndicator = $derived($page.url.pathname === '/recording');

  let activeMenu = $derived(
    $page.url.pathname === '/settings' ? 'settings' :
    $page.url.pathname === '/skills' ? 'skills' :
    $page.url.pathname === '/models' ? 'models' :
    $page.url.pathname === '/logs' ? 'logs' : 'home'
  );

  function navigateTo(path: string) {
    goto(path);
  }
</script>

{#if isLoading}
  <div class="flex h-screen w-screen items-center justify-center bg-background">
    <div class="text-muted-foreground text-body">加载中...</div>
  </div>
{:else if !$onboarding.completed}
  <OnboardingWizard />
{:else if isFloatingIndicator}
  {@render children()}
{:else}
<div class="flex h-screen w-screen overflow-hidden">
  <aside class="w-52 bg-background-alt border-r border-border flex flex-col">
    <div class="flex items-center gap-2.5 px-6 py-5 font-semibold text-subheading text-foreground border-b border-border">
      <img src="/logo.svg" alt="TalkShow" class="w-6 h-6 shrink-0" />
      <span>TalkShow</span>
    </div>
    <nav class="py-3">
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'home' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/')}
      >
        <House size={20} class="shrink-0" />
        <span>首页</span>
      </button>
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'models' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/models')}
      >
        <Bot size={20} class="shrink-0" />
        <span>模型</span>
      </button>
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'skills' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/skills')}
      >
        <Sparkles size={20} class="shrink-0" />
        <span>技能</span>
      </button>
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'settings' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/settings')}
      >
        <Settings size={20} class="shrink-0" />
        <span>设置</span>
      </button>
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'logs' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/logs')}
      >
        <ScrollText size={20} class="shrink-0" />
        <span>日志</span>
      </button>
    </nav>
  </aside>
  <main class="flex-1 p-8 overflow-y-auto bg-background">
    {@render children()}
  </main>
</div>
{/if}
