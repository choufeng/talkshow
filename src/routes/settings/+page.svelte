<script lang="ts">
  import { onMount } from 'svelte';
  import { Lightbulb, Sun, Moon, Monitor } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { config } from '$lib/stores/config';
  import { theme, type Theme } from '$lib/stores/theme';
  import ShortcutRecorder from '$lib/components/ui/shortcut-recorder/index.svelte';
  import Toggle from '$lib/components/ui/toggle/index.svelte';
  import { updateFeature } from '$lib/ai/shared';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';

  interface AudioDevice {
    name: string;
    is_default: boolean;
  }

  let audioDevices: AudioDevice[] = [];
  let deviceError = '';

  onMount(async () => {
    config.load();
    await refreshDevices();
  });

  async function refreshDevices() {
    try {
      audioDevices = (await invoke<AudioDevice[]>('list_audio_devices')) ?? [];
      deviceError = '';
    } catch (e) {
      deviceError = String(e);
    }
  }

  async function handleSelectDevice(name: string) {
    // 空 = 跟随系统默认;命令内部负责持久化 + 重建录音器,前端只需回拉
    try {
      await invoke('set_audio_input_device', { device: name });
      await config.load();
      deviceError = '';
    } catch (e) {
      deviceError = String(e);
    }
  }

  function deviceGroups() {
    const items = [
      { value: '', label: '跟随系统默认' },
      ...audioDevices.map((d) => ({
        value: d.name,
        label: d.is_default ? `${d.name}（默认）` : d.name
      }))
    ];
    return [{ label: '', items }];
  }

  async function handleUpdateToggle(shortcut: string) {
    await config.updateShortcut('toggle', shortcut);
  }

  async function handleUpdateRecording(shortcut: string) {
    await config.updateShortcut('recording', shortcut);
  }

  async function handleUpdateTranslate(shortcut: string) {
    await config.updateShortcut('translate', shortcut);
  }

  function toggleAutoMute() {
    const recording = $config.features?.recording ?? { auto_mute: false, input_device: '' };
    config.save(updateFeature($config, 'recording', () => ({
      ...recording,
      auto_mute: !recording.auto_mute
    })));
  }

  function toggleIntentRouting() {
    const routing = $config.features?.intent_routing ?? { enabled: false, disabled: [] as string[] };
    config.save(updateFeature($config, 'intent_routing', () => ({
      ...routing,
      enabled: !routing.enabled
    })));
  }

  function toggleReview() {
    const review = $config.features?.review ?? { enabled: false };
    config.save(updateFeature($config, 'review', () => ({
      ...review,
      enabled: !review.enabled
    })));
  }

  const INTENT_KINDS: { kind: string; label: string; desc: string }[] = [
    { kind: 'translate', label: '翻译指令', desc: '说“翻译成英文…”识别后直接给出译文' },
    { kind: 'ask', label: '问询指令', desc: '说“问一下…”打开对话窗口' },
    { kind: 'hook', label: '记录指令', desc: '说“记录…”发送到 Hook 后端' }
  ];

  function toggleIntentKind(kind: string) {
    const routing = $config.features?.intent_routing ?? { enabled: false, disabled: [] as string[] };
    const disabled = routing.disabled ?? [];
    config.save(updateFeature($config, 'intent_routing', () => ({
      ...routing,
      disabled: disabled.includes(kind) ? disabled.filter((k) => k !== kind) : [...disabled, kind]
    })));
  }

  const THEME_OPTIONS: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: '浅色', icon: Sun },
    { value: 'dark', label: '深色', icon: Moon },
    { value: 'system', label: '跟随系统', icon: Monitor },
  ];
</script>

<div class="max-w-[640px]">
  <h2 class="text-title font-semibold text-foreground m-0 mb-8">设置</h2>

  <section class="mb-10">
    <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">快捷键</div>
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
      <p class="text-body text-accent-foreground m-0">
        <Lightbulb size={15} class="inline -align-[2px] mr-1" />
        提示：点击"修改"按钮后，直接按下键盘上的组合键即可完成设置。按 Esc 取消录制。
      </p>
    </div>
  </section>

  <section class="mb-10">
    <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">语音指令</div>
    <div class="rounded-xl border border-border bg-background-alt p-5 mb-4">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-[15px] font-semibold text-foreground mb-1">语义路由</div>
          <div class="text-body text-foreground-alt">录音结束后自动识别开头的指令：说"翻译成英文…"则翻译后粘贴，无指令则正常转写。需要已配置润色模型</div>
        </div>
        <Toggle
          checked={$config.features?.intent_routing?.enabled ?? false}
          onCheckedChange={() => toggleIntentRouting()}
          ariaLabel="语义路由"
        />
      </div>
      {#if $config.features?.intent_routing?.enabled}
        <div class="mt-4 pt-4 border-t border-border flex flex-col gap-3">
          {#each INTENT_KINDS as item (item.kind)}
            <div class="flex items-center justify-between gap-4">
              <div>
                <div class="text-[14px] font-medium text-foreground">{item.label}</div>
                <div class="text-sm text-foreground-alt">{item.desc}</div>
              </div>
              <Toggle
                checked={!($config.features?.intent_routing?.disabled ?? []).includes(item.kind)}
                onCheckedChange={() => toggleIntentKind(item.kind)}
                ariaLabel={item.label}
              />
            </div>
          {/each}
        </div>
      {/if}
    </div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-[15px] font-semibold text-foreground mb-1">发送前人工确认</div>
          <div class="text-body text-foreground-alt">录音处理完成后弹出手动校对浮窗：可编辑文字、查看即将发送的内容，并选择粘贴 / 翻译 / 问询 / Hook 去向后再执行</div>
        </div>
        <Toggle
          checked={$config.features?.review?.enabled ?? false}
          onCheckedChange={() => toggleReview()}
          ariaLabel="发送前人工确认"
        />
      </div>
      {#if $config.features?.review?.enabled}
        <div class="mt-4 border-t border-border pt-4">
          <div class="text-body text-foreground-alt mb-2">浮窗发送键（校对浮窗内触发确认）</div>
          <div class="flex gap-2">
            {#each [{ v: 'enter', l: 'Enter' }, { v: 'cmd_enter', l: '⌘Enter' }] as opt (opt.v)}
              <button
                class="rounded-lg border px-4 py-2 text-sm transition-colors {($config.features?.review?.send_key ?? 'enter') === opt.v
                  ? 'border-btn-primary-to bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white font-medium'
                  : 'border-border bg-background text-foreground hover:bg-muted/50'}"
                onclick={() => config.save(updateFeature($config, 'review', (r) => ({ ...r, send_key: opt.v })))}
              >
                {opt.l}
              </button>
            {/each}
          </div>
          <div class="mt-2 text-caption text-muted-foreground">另一变体在文本框中作为换行；Shift+Enter 恒为换行；Esc 取消</div>
        </div>
      {/if}
    </div>
  </section>

  <section class="mb-10">
    <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">录音</div>
    <div class="rounded-xl border border-border bg-background-alt p-5 mb-4">
      <div class="flex items-center justify-between gap-4">
        <div>
          <div class="text-[15px] font-semibold text-foreground mb-1">录音时自动静音</div>
          <div class="text-body text-foreground-alt">开始录音后自动静音其他应用的声音，录音结束后自动恢复</div>
        </div>
        <Toggle
          checked={$config.features?.recording?.auto_mute ?? false}
          onCheckedChange={() => toggleAutoMute()}
          ariaLabel="录音时自动静音"
        />
      </div>
    </div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="text-[15px] font-semibold text-foreground mb-1">麦克风输入</div>
      <div class="text-body text-foreground-alt mb-4">选择录音使用的麦克风设备，切换后立即生效</div>
      {#if audioDevices.length > 0}
        <GroupedSelect
          value={$config.features?.recording?.input_device ?? ''}
          groups={deviceGroups()}
          onChange={handleSelectDevice}
          aria-label="麦克风输入设备"
        />
      {:else if deviceError}
        <div class="text-sm text-red-500">{deviceError}</div>
        <button class="mt-2 text-sm text-foreground-alt underline" onclick={() => refreshDevices()}>
          重试
        </button>
      {:else}
        <div class="text-body text-muted-foreground">未检测到可用麦克风设备</div>
      {/if}
    </div>
  </section>

  <section>
    <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">外观</div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="text-[15px] font-semibold text-foreground mb-1">主题模式</div>
      <div class="text-body text-foreground-alt mb-4">选择界面的色彩主题</div>
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
