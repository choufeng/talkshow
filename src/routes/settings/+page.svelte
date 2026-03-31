<script lang="ts">
  import { onMount } from 'svelte';
  import { Lightbulb, Sun, Moon, Monitor } from 'lucide-svelte';
  import { config } from '$lib/stores/config';
  import { theme, type Theme } from '$lib/stores/theme';
  import ShortcutRecorder from '$lib/components/ui/shortcut-recorder/index.svelte';

  onMount(() => {
    config.load();
  });

  async function handleUpdateToggle(shortcut: string) {
    await config.updateShortcut('toggle', shortcut);
  }

  async function handleUpdateRecording(shortcut: string) {
    await config.updateShortcut('recording', shortcut);
  }

  async function handleUpdateTranslate(shortcut: string) {
    await config.updateShortcut('translate', shortcut);
  }

  const THEME_OPTIONS: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: '浅色', icon: Sun },
    { value: 'dark', label: '深色', icon: Moon },
    { value: 'system', label: '跟随系统', icon: Monitor },
  ];
</script>

<div class="max-w-[640px]">
  <h2 class="text-2xl font-semibold text-foreground m-0 mb-8">设置</h2>

  <section class="mb-10">
    <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">快捷键</div>
    <ShortcutRecorder
      label="窗口切换"
      description="显示或隐藏主窗口"
      value={$config.shortcut}
      onUpdate={handleUpdateToggle}
    />
    <ShortcutRecorder
      label="录音控制"
      description="开始或结束录音"
      value={$config.recording_shortcut}
      onUpdate={handleUpdateRecording}
    />
    <ShortcutRecorder
      label="AI 翻译"
      description="录音并翻译为目标语言"
      value={$config.translate_shortcut}
      onUpdate={handleUpdateTranslate}
    />
    <div class="rounded-lg bg-accent/50 border border-accent p-5 mt-6">
      <p class="text-sm text-accent-foreground m-0">
        <Lightbulb size={15} class="inline -align-[2px] mr-1" />
        提示：点击"修改"按钮后，直接按下键盘上的组合键即可完成设置。按 Esc 取消录制。
      </p>
    </div>
  </section>

  <section>
    <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">外观</div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="text-[15px] font-semibold text-foreground mb-1">主题模式</div>
      <div class="text-sm text-foreground-alt mb-4">选择界面的色彩主题</div>
      <div class="flex gap-3">
        {#each THEME_OPTIONS as opt}
          {@const Icon = opt.icon}
          <button
            class="flex-1 flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm transition-colors {$theme === opt.value ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium shadow-btn-primary' : 'border-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-foreground hover:bg-muted/50 shadow-btn-secondary'}"
            onclick={() => theme.set(opt.value)}
          >
            <Icon size={16} class="shrink-0" />
            <span>{opt.label}</span>
          </button>
        {/each}
      </div>
    </div>
  </section>
</div>
