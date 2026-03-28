<script lang="ts">
  import { onMount } from 'svelte';
  import { Lightbulb, Sun, Moon, Monitor } from 'lucide-svelte';
  import { config } from '$lib/stores/config';
  import { theme, type Theme } from '$lib/stores/theme';
  import ShortcutRecorder from '$lib/components/ui/shortcut-recorder/index.svelte';

  onMount(() => {
    config.load();
  });

  let currentTheme = $state<Theme>('system');

  onMount(() => {
    const stored = localStorage.getItem('theme');
    if (stored === 'light' || stored === 'dark' || stored === 'system') {
      currentTheme = stored;
    }
  });

  function setTheme(t: Theme) {
    currentTheme = t;
    theme.set(t);
  }

  async function handleUpdateToggle(shortcut: string) {
    await config.updateShortcut('toggle', shortcut);
  }

  async function handleUpdateRecording(shortcut: string) {
    await config.updateShortcut('recording', shortcut);
  }

  const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: '浅色', icon: Sun },
    { value: 'dark', label: '深色', icon: Moon },
    { value: 'system', label: '系统', icon: Monitor },
  ];
</script>

<div class="max-w-[600px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">设置</h2>

  <section class="mb-8">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">快捷键</div>
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
    <div class="rounded-lg bg-accent/50 border border-accent p-4 mt-5">
      <p class="text-xs text-accent-foreground m-0">
        <Lightbulb size={14} class="inline -align-[2px] mr-1" />
        提示：点击"修改"按钮后，直接按下键盘上的组合键即可完成设置。按 Esc 取消录制。
      </p>
    </div>
  </section>

  <section>
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">外观</div>
    <div class="flex gap-2">
      {#each themeOptions as opt}
        {@const Icon = opt.icon}
        <button
          class="flex items-center gap-2 px-4 py-2.5 rounded-md border text-sm transition-colors {currentTheme === opt.value ? 'border-accent-foreground bg-accent text-accent-foreground' : 'border-border bg-background text-foreground hover:bg-muted'}"
          onclick={() => setTheme(opt.value)}
        >
          <Icon size={16} />
          {opt.label}
        </button>
      {/each}
    </div>
  </section>
</div>
