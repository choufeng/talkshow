<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import type { Snippet } from 'svelte';
  import { House, Settings, Bot, ScrollText, Sparkles, History, MessageCircle } from 'lucide-svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onboarding } from '$lib/stores/onboarding';
  import { chatOpen, type ChatOpenPayload } from '$lib/stores/chat';
  import OnboardingWizard from '$lib/components/onboarding/OnboardingWizard.svelte';
  import PermissionBanner from '$lib/components/PermissionBanner.svelte';

  let { children }: { children: Snippet } = $props();

  let isLoading = $state(true);

  onMount(() => {
    onboarding.load().then(() => {
      isLoading = false;
    });
  });

  // 语义路由 ask:唤起主窗体后导航到对话栏目,payload 存 store 供对话页消费
  onMount(() => {
    // Tauri v2 坑:前端 listen 默认 target=Any,emit_to 定向事件会投给所有
    // 注册了同名监听器的窗口。不加守卫,indicator 窗口也会 goto('/chat'),
    // route 被污染后录音指示条变成主窗体侧栏。仅主窗体消费 chat:open。
    if (getCurrentWindow().label !== 'main') return;
    let unsub: (() => void) | null = null;
    listen<ChatOpenPayload>('chat:open', (e) => {
      chatOpen.set(e.payload);
      goto('/chat');
    }).then((fn) => (unsub = fn));
    return () => unsub?.();
  });

  let activeMenu = $derived(
    $page.url.pathname === '/settings' ? 'settings' :
    $page.url.pathname === '/skills' ? 'skills' :
    $page.url.pathname === '/models' ? 'models' :
    $page.url.pathname === '/logs' ? 'logs' :
    $page.url.pathname === '/history' ? 'history' :
    $page.url.pathname === '/chat' ? 'chat' : 'home'
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
{:else}
<PermissionBanner />
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
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'chat' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/chat')}
      >
        <MessageCircle size={20} class="shrink-0" />
        <span>对话</span>
      </button>
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'history' ? 'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
        onclick={() => navigateTo('/history')}
      >
        <History size={20} class="shrink-0" />
        <span>历史回溯</span>
      </button>
      <button
        class="flex items-center gap-3 px-6 py-3 w-full text-[15px] text-foreground text-left transition-colors {activeMenu === 'settings' ?'bg-gradient-to-r from-btn-primary-from/15 to-btn-primary-from/5 border-l-[3px] border-l-accent-foreground font-medium shadow-[0_1px_3px_oklch(38%_0.1_160/0.1)]' : 'hover:bg-muted/50 border-l-[3px] border-l-transparent'}"
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
