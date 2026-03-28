<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import ShortcutRecorder from '$lib/components/ShortcutRecorder.svelte';

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

<div class="settings-page">
  <h2>快捷键设置</h2>

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

  <div class="hint">
    <p>💡 提示：点击"修改"按钮后，直接按下键盘上的组合键即可完成设置。按 Esc 取消录制。</p>
  </div>
</div>

<style>
  .settings-page {
    max-width: 600px;
  }

  h2 {
    margin: 0 0 24px 0;
    font-size: 20px;
    font-weight: 600;
    color: #333;
  }

  .hint {
    background: #fff9e6;
    border: 1px solid #ffd666;
    border-radius: 8px;
    padding: 16px;
    margin-top: 20px;
  }

  .hint p {
    margin: 0;
    color: #d48806;
    font-size: 13px;
  }
</style>