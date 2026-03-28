<script lang="ts">
  import { onMount } from 'svelte';
  import { Lightbulb } from 'lucide-svelte';
  import { config } from '$lib/stores/config';
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
</script>

<div class="max-w-[600px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">设置</h2>

  <section>
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
</div>
