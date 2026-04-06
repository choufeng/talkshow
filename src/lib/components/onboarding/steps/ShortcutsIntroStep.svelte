<script lang="ts">
  import { onboarding } from '$lib/stores/onboarding';
  import { onMount } from 'svelte';
  import { Keyboard } from 'lucide-svelte';

  onMount(() => {
    onboarding.setStepValid(4, true);
  });

  const shortcuts = [
    { label: '窗口切换', desc: '显示或隐藏主窗口', default: 'Ctrl + Shift + \'' },
    { label: '录音控制', desc: '开始或结束录音', default: 'Ctrl + \\' },
    { label: 'AI 翻译', desc: '录音并翻译为目标语言', default: 'Ctrl + Shift + T' },
  ];
</script>

<div>
  <div class="text-center mb-6">
    <div class="w-14 h-14 rounded-full bg-accent/50 flex items-center justify-center mx-auto mb-4">
      <Keyboard size={28} class="text-accent-foreground" />
    </div>
    <h2 class="text-subheading font-semibold text-foreground mb-2">快捷键说明</h2>
    <p class="text-body text-foreground-alt">
      TalkShow 通过全局快捷键控制，你可以随时在设置中修改。
    </p>
  </div>

  <div class="space-y-3">
    {#each shortcuts as s}
      <div class="flex items-center gap-4 p-3 rounded-lg border border-border bg-background">
        <div class="flex-1">
          <div class="text-body font-medium text-foreground">{s.label}</div>
          <div class="text-caption text-foreground-alt">{s.desc}</div>
        </div>
        <kbd class="px-3 py-1.5 rounded-md text-caption font-mono bg-gradient-to-b from-key-bg-from to-key-bg-to border border-key-border text-key-text">
          {s.default}
        </kbd>
      </div>
    {/each}
  </div>
</div>
