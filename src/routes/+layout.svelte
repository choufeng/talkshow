<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import type { Snippet } from 'svelte';
  import { House, Settings, Bot, ScrollText, Sparkles } from 'lucide-svelte';

  let { children }: { children: Snippet } = $props();

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

<div class="flex h-screen w-screen overflow-hidden">
  <aside class="w-40 bg-background-alt border-r border-border flex flex-col">
    <div class="px-5 py-4 font-semibold text-sm text-foreground border-b border-border">
      TalkShow
    </div>
    <nav class="py-2">
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'home' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/')}
      >
        <House size={18} class="shrink-0" />
        <span>首页</span>
      </button>
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'models' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/models')}
      >
        <Bot size={18} class="shrink-0" />
        <span>模型</span>
      </button>
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'skills' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/skills')}
      >
        <Sparkles size={18} class="shrink-0" />
        <span>技能</span>
      </button>
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'settings' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/settings')}
      >
        <Settings size={18} class="shrink-0" />
        <span>设置</span>
      </button>
      <button
        class="flex items-center gap-2 px-5 py-2.5 w-full text-sm text-foreground text-left transition-colors {activeMenu === 'logs' ? 'bg-muted border-l-[3px] border-l-accent-foreground' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/logs')}
      >
        <ScrollText size={18} class="shrink-0" />
        <span>日志</span>
      </button>
    </nav>
  </aside>
  <main class="flex-1 p-6 overflow-y-auto bg-background">
    {@render children()}
  </main>
</div>
