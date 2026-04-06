<script lang="ts">
  import { onMount } from 'svelte';
  import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES, TRANSLATE_LANGUAGES, PROVIDERS_REQUIRING_KEY } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import EditableField from '$lib/components/ui/editable-field/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import Toggle from '$lib/components/ui/toggle/index.svelte';
  import DialogFooter from '$lib/components/ui/dialog-footer/index.svelte';
  import { listen } from '@tauri-apps/api/event';
  import { RotateCcw, Mic, Languages } from 'lucide-svelte';
  import type { ProviderConfig, AppConfig, ModelConfig, ModelVerified, TranscriptionConfig, TranslationConfig } from '$lib/stores/config';
  import { SENSEVOICE_LANGUAGES } from '$lib/stores/config';
  import { updateFeature, updateNestedPath, invokeWithError } from '$lib/ai/shared';
  import { formatDate } from '$lib/utils';
  import { createDialogState } from '$lib/hooks';

  const deleteConfirmDialog = createDialogState({
    onReset: () => { pendingActionProviderId = ''; }
  });
  let pendingActionProviderId = $state('');
  const resetConfirmDialog = createDialogState({
    onReset: () => { pendingActionProviderId = ''; }
  });
  const addModelDialog = createDialogState({
    onReset: () => { addModelProviderId = ''; newModelName = ''; newModelCapabilities = []; }
  });
  let addModelProviderId = $state('');
  let newModelName = $state('');
  let newModelCapabilities = $state<string[]>([]);
  const removeModelDialog = createDialogState({
    onReset: () => { pendingRemoveModel = { providerId: '', modelName: '' }; }
  });
  let pendingRemoveModel = $state<{ providerId: string; modelName: string }>({ providerId: '', modelName: '' });
  let testingModels = $state<Set<string>>(new Set());
  let vertexEnvInfo = $state<{ project: string; location: string } | null>(null);
  let sensevoiceStatus = $state<{ status: string; size_bytes?: number } | null>(null);
  let sensevoiceDownloading = $state(false);
  let sensevoiceDownloadProgress = $state({ file: '', percent: 0, downloaded: 0, total: 0 });
  let sensevoiceLanguage = $state(0);

  async function loadSenseVoiceStatus() {
    sensevoiceStatus = await invokeWithError<{ status: string; size_bytes?: number }>('get_sensevoice_status');
  }

  async function downloadSenseVoice() {
    sensevoiceDownloading = true;
    try {
      await invokeWithError('download_sensevoice_model');
      await loadSenseVoiceStatus();
    } catch (e) {
      console.error('Download failed:', e);
    } finally {
      sensevoiceDownloading = false;
    }
  }

  async function deleteSenseVoiceModel() {
    await invokeWithError('delete_sensevoice_model');
    await loadSenseVoiceStatus();
  }

  onMount(async () => {
    config.load();
    vertexEnvInfo = await invokeWithError<{ project: string; location: string }>('get_vertex_env_info');
    await loadSenseVoiceStatus();
    listen('sensevoice:download-progress', (event) => {
      sensevoiceDownloadProgress = event.payload as any;
    });
  });

  function buildTranscriptionGroups() {
    return ($config.ai.providers || []).map((p: ProviderConfig) => ({
      label: p.name,
      items: (p.models || [])
        .filter((m: ModelConfig) => m.capabilities.includes('transcription'))
        .map((m: ModelConfig) => ({
          value: `${p.id}::${m.name}`,
          label: m.name
        }))
    })).filter((g) => g.items.length > 0);
  }

  function buildPolishGroups() {
    return ($config.ai.providers || []).map((p: ProviderConfig) => ({
      label: p.name,
      items: (p.models || [])
        .filter((m: ModelConfig) => 
          m.capabilities.includes('chat') || m.capabilities.includes('text_generation')
        )
        .map((m: ModelConfig) => ({
          value: `${p.id}::${m.name}`,
          label: m.name
        }))
    })).filter((g) => g.items.length > 0);
  }

  function getTranscriptionValue(): string {
    const t = $config.features.transcription;
    if (t.provider_id && t.model) {
      return `${t.provider_id}::${t.model}`;
    }
    return '';
  }

  function handleTranscriptionChange(val: string) {
    const [providerId, model] = val.split('::');
    config.save(updateFeature($config, 'transcription', (t) => ({
      ...(t as TranscriptionConfig),
      provider_id: providerId,
      model
    })));
  }

  function getPolishValue(): string {
    const t = $config.features.transcription;
    if (t.polish_provider_id && t.polish_model) {
      return `${t.polish_provider_id}::${t.polish_model}`;
    }
    return '';
  }

  function handlePolishChange(val: string) {
    const [providerId, model] = val.split('::');
    config.save(updateFeature($config, 'transcription', (t) => ({
      ...(t as TranscriptionConfig),
      polish_provider_id: providerId,
      polish_model: model
    })));
  }

  function handlePolishEnabled(enabled: boolean) {
    config.save(updateFeature($config, 'transcription', (t) => ({
      ...(t as TranscriptionConfig),
      polish_enabled: enabled
    })));
  }

  function handleTargetLangChange(lang: string) {
    config.save(updateFeature($config, 'translation', (t) => ({
      ...(t as TranslationConfig),
      target_lang: lang
    })));
  }

  async function handleApiKeyChange(providerId: string, value: string) {
    try {
      await config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
        (providers as ProviderConfig[]).map((p) =>
          p.id === providerId ? { ...p, api_key: value } : p
        )
      ));
    } catch (e) {
      console.error('Failed to save API key:', e);
      await config.load();
    }
  }

  function handleAddModel(providerId: string, model: ModelConfig) {
    config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
      (providers as ProviderConfig[]).map((p) =>
        p.id === providerId
          ? { ...p, models: [...p.models, model] }
          : p
      )
    ));
  }

  function openAddModelDialog(providerId: string) {
    addModelProviderId = providerId;
    newModelName = '';
    newModelCapabilities = [];
    addModelDialog.open();
  }

  function confirmAddModel() {
    if (!newModelName.trim()) return;
    const model: ModelConfig = {
      name: newModelName.trim(),
      capabilities: [...newModelCapabilities]
    };
    handleAddModel(addModelProviderId, model);
    addModelDialog.close();
  }

  function handleRemoveModel(providerId: string, modelName: string) {
    pendingRemoveModel = { providerId, modelName };
    removeModelDialog.open();
  }

  function confirmRemoveModel() {
    const { providerId, modelName } = pendingRemoveModel;
    config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
      (providers as ProviderConfig[]).map((p) =>
        p.id === providerId
          ? { ...p, models: p.models.filter((m: ModelConfig) => m.name !== modelName) }
          : p
      )
    ));
    removeModelDialog.close();
  }

  function toggleCapability(cap: string) {
    if (newModelCapabilities.includes(cap)) {
      newModelCapabilities = newModelCapabilities.filter((c) => c !== cap);
    } else {
      newModelCapabilities = [...newModelCapabilities, cap];
    }
  }

  function handleRemoveProvider(providerId: string) {
    pendingActionProviderId = providerId;
    deleteConfirmDialog.open();
  }

  function confirmRemoveProvider() {
    config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
      (providers as ProviderConfig[]).filter((p) => p.id !== pendingActionProviderId)
    ));
    deleteConfirmDialog.close();
  }

  function handleResetProvider(providerId: string) {
    pendingActionProviderId = providerId;
    resetConfirmDialog.open();
  }

  function confirmResetProvider() {
    const builtin = BUILTIN_PROVIDERS.find((p) => p.id === pendingActionProviderId);
    if (!builtin) return;
    config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
      (providers as ProviderConfig[]).map((p) =>
        p.id === pendingActionProviderId ? { ...builtin } : p
      )
    ));
    resetConfirmDialog.close();
  }

  function needsApiKey(provider: ProviderConfig): boolean {
    return PROVIDERS_REQUIRING_KEY.includes(provider.id);
  }

  function getTestKey(providerId: string, modelName: string): string {
    return `${providerId}::${modelName}`;
  }

  function isTesting(providerId: string, modelName: string): boolean {
    return testingModels.has(getTestKey(providerId, modelName));
  }

  async function testModel(providerId: string, modelName: string) {
    const key = getTestKey(providerId, modelName);
    testingModels = new Set([...testingModels, key]);
    try {
      const result = await invokeWithError<{ status: string; latency_ms?: number; message: string }>('test_model_connectivity', {
        providerId,
        modelName,
      });
      await config.load();
      return result;
    } catch (e) {
      await config.load();
      throw e;
    } finally {
      const next = new Set(testingModels);
      next.delete(key);
      testingModels = next;
    }
  }

  async function testAllModels(provider: ProviderConfig) {
    for (const model of provider.models) {
      await testModel(provider.id, model.name);
    }
  }
</script>

<div class="max-w-[800px]">
  <h2 class="text-title font-semibold text-foreground m-0 mb-8">模型</h2>

  <section class="mb-10">
    <!-- 共享标题栏 -->
    <div class="rounded-t-xl border border-border border-b-0 bg-muted p-4 flex justify-between items-center">
      <span class="text-subheading font-semibold text-foreground">AI 服务</span>
      <span class="text-caption text-muted-foreground">配置转写和翻译功能</span>
    </div>
    
    <!-- 横向卡片容器 -->
    <div class="flex flex-col sm:flex-row border border-border rounded-b-xl bg-background-alt overflow-hidden ai-service-container">
      <!-- 左侧卡片：AI 转写 -->
      <div class="flex-1 p-5 border-r border-border">
        <div class="flex items-center gap-3 mb-5">
          <div class="w-9 h-9 bg-accent/10 rounded-lg flex items-center justify-center">
            <Mic class="w-4 h-4 text-accent-foreground" />
          </div>
          <div>
            <div class="text-subheading font-semibold text-foreground">AI 转写</div>
            <div class="text-caption text-muted-foreground">语音转文字 + 润色</div>
          </div>
        </div>
        
        <div class="mb-5">
          <span class="block text-body text-foreground-alt mb-1.5">转写模型</span>
          <GroupedSelect
            value={getTranscriptionValue()}
            aria-label="转写模型"
            groups={buildTranscriptionGroups()}
            placeholder="选择模型"
            onChange={handleTranscriptionChange}
          />
        </div>

        <div class="flex items-center justify-between mb-5">
          <div>
            <div class="text-[15px] font-semibold text-foreground">启用润色</div>
            <div class="text-body text-foreground-alt">转写后自动使用 LLM 润色文字</div>
          </div>
          <Toggle
            checked={$config.features.transcription.polish_enabled}
            onCheckedChange={handlePolishEnabled}
            ariaLabel="启用润色"
          />
        </div>

        {#if $config.features.transcription.polish_enabled}
        <div>
          <span class="block text-body text-foreground-alt mb-1.5">润色模型</span>
          <GroupedSelect
            value={getPolishValue()}
            aria-label="润色模型"
            groups={buildPolishGroups()}
            placeholder="选择模型"
            onChange={handlePolishChange}
          />
        </div>
        {/if}
      </div>
      
      <!-- 右侧卡片：AI 翻译 -->
      <div class="flex-1 p-5">
        <div class="flex items-center gap-3 mb-5">
          <div class="w-9 h-9 bg-accent/10 rounded-lg flex items-center justify-center">
            <Languages class="w-4 h-4 text-accent-foreground" />
          </div>
          <div>
            <div class="text-subheading font-semibold text-foreground">AI 翻译</div>
            <div class="text-caption text-muted-foreground">实时翻译转写内容</div>
          </div>
        </div>
        
        <div class="mb-5">
          <label for="target-lang-select" class="block text-body text-foreground-alt mb-1.5">目标语言</label>
          <select
            id="target-lang-select"
            class="flex h-9 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
            value={$config.features.translation?.target_lang || 'English'}
            onchange={(e) => handleTargetLangChange((e.target as HTMLSelectElement).value)}
          >
            {#each TRANSLATE_LANGUAGES as lang}
              <option value={lang}>{lang}</option>
            {/each}
          </select>
        </div>
        
        <div class="p-3 bg-muted rounded-lg border border-dashed border-border">
          <div class="text-caption text-muted-foreground mb-1">翻译模型</div>
          <div class="text-body text-muted-foreground">复用润色模型：{$config.features.transcription.polish_model || '未配置'}</div>
          <div class="text-caption text-muted-foreground/70 mt-1">启用润色后，翻译将使用相同的模型</div>
        </div>
      </div>
    </div>
  </section>

  <section>
    <div class="text-caption text-muted-foreground uppercase tracking-wider mb-3">Providers</div>
    <div class="grid grid-cols-2 gap-4">
      {#each $config.ai.providers || [] as provider (provider.id)}
        <div class="rounded-xl border border-border bg-background-alt p-5 overflow-hidden">
          <div class="flex justify-between items-start mb-4">
            <div>
              <div class="text-[15px] font-semibold text-foreground">{provider.name}</div>
              <div class="text-[11px] text-muted-foreground mt-0.5">{provider.id}</div>
            </div>
            {#if isBuiltinProvider(provider.id)}
              <button
                class="text-caption text-muted-foreground hover:text-foreground transition-colors p-0.5"
                onclick={() => handleResetProvider(provider.id)}
                title="重置为默认"
              >
                <RotateCcw class="h-3.5 w-3.5" />
              </button>
            {/if}
          </div>

          {#if provider.id === 'sensevoice'}
            <div class="mb-3">
              <div aria-label="模型状态" class="block text-body text-foreground-alt mb-1">模型状态</div>
              <div class="text-[11px] bg-background rounded-md border border-border p-2 space-y-1">
                {#if sensevoiceStatus?.status === 'ready'}
                  <div class="flex items-center justify-between">
                    <span class="text-green-500">已就绪</span>
                    <span class="text-muted-foreground">{(sensevoiceStatus!.size_bytes! / 1024 / 1024).toFixed(0)} MB</span>
                  </div>
                  <button
                    class="text-caption text-red-400 hover:text-red-300 transition-colors"
                    onclick={deleteSenseVoiceModel}
                  >
                    删除模型
                  </button>
                {:else if sensevoiceDownloading}
                  <div>
                    <div class="text-muted-foreground mb-1">下载中: {sensevoiceDownloadProgress.file}</div>
                    <div class="w-full bg-border rounded-full h-1.5">
                      <div class="bg-accent-foreground h-1.5 rounded-full transition-all" style="width: {sensevoiceDownloadProgress.percent}%"></div>
                    </div>
                    <div class="text-muted-foreground mt-0.5">{sensevoiceDownloadProgress.percent.toFixed(1)}%</div>
                  </div>
                {:else}
                  <button
class="text-caption text-accent-foreground hover:underline"
                    onclick={downloadSenseVoice}
                  >
                    下载模型 (约 242 MB)
                  </button>
                {/if}
              </div>
            </div>
            <div class="mb-3">
              <label for="sensevoice-lang-select" class="block text-body text-foreground-alt mb-1">转写语言</label>
              <select
                id="sensevoice-lang-select"
                class="flex h-9 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
                bind:value={sensevoiceLanguage}
              >
                {#each SENSEVOICE_LANGUAGES as lang}
                  <option value={lang.value}>{lang.label}</option>
                {/each}
              </select>
            </div>
          {/if}

          {#if needsApiKey(provider)}
            <div class="mb-3">
              <span class="block text-body text-foreground-alt mb-1">API Key</span>
              <EditableField
                value={provider.api_key || ''}
                aria-label="API Key"
                placeholder="sk-..."
                mode="password"
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          {#if provider.id === 'vertex'}
            <div class="mb-3">
              <div aria-label="Vertex AI 配置" class="block text-body text-foreground-alt mb-1">Vertex AI 配置</div>
              <div class="text-[11px] text-muted-foreground space-y-0.5 bg-background rounded-md border border-border p-2">
                <div>GOOGLE_CLOUD_PROJECT: <span class="text-foreground">{vertexEnvInfo?.project || '未设置'}</span></div>
                <div>GOOGLE_CLOUD_LOCATION: <span class="text-foreground">{vertexEnvInfo?.location || 'global'}</span></div>
                <div class="text-muted-foreground/70 mt-1">认证: <code class="text-[10px] bg-background px-1 py-0.5 rounded border border-border">gcloud auth application-default login</code></div>
              </div>
            </div>
          {/if}

          <div>
            <div class="block text-body text-foreground-alt mb-1">Models</div>
            <div class="mt-1">
              <div class="flex flex-wrap gap-1 mb-1">
                {#each provider.models || [] as model (model.name)}
                  {@const verified = model.verified}
                  {@const testing = isTesting(provider.id, model.name)}
                  <div
                    role="button"
                    tabindex="0"
                    class="inline-flex items-center gap-1 rounded px-2.5 py-1 text-[11px] text-accent-foreground {provider.id !== 'sensevoice' ? 'cursor-pointer' : 'cursor-default'} select-none
                      {verified?.status === 'ok' ? 'bg-green-500/15 border border-green-500/30' : ''}
                      {verified?.status === 'error' ? 'bg-red-500/15 border border-red-500/30' : ''}
                      {!verified && !testing ? 'bg-accent' : ''}
                      {testing ? 'bg-accent animate-pulse' : ''}"
                    title={provider.id === 'sensevoice' ? '' : verified ? `${verified.status === 'ok' ? '验证通过' : '验证失败'}${verified.latency_ms ? ' · ' + verified.latency_ms + 'ms' : ''}${verified.message ? ' · ' + verified.message : ''}` : '点击测试'}
                    onclick={() => { if (provider.id !== 'sensevoice') testModel(provider.id, model.name); }}
                    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (provider.id !== 'sensevoice') testModel(provider.id, model.name); } }}
                  >
                    {model.name}
                    {#if model.capabilities?.includes('transcription')}
                      <span class="text-[9px] opacity-70">T</span>
                    {/if}
                    {#if testing}
                      <span class="animate-spin text-[10px]">⟳</span>
                    {:else if verified?.status === 'ok'}
                      <span class="text-green-500 text-[10px]">✓</span>
                      <span class="text-[9px] text-green-500/70">{formatDate(verified.tested_at)}</span>
                    {:else if verified?.status === 'error'}
                      <span class="text-red-500 text-[10px]">✕</span>
                      <span class="text-[9px] text-red-500/70">{formatDate(verified.tested_at)}</span>
                    {/if}
                    {#if provider.id !== 'sensevoice'}
                    <button
                      type="button"
                      class="opacity-60 hover:opacity-100 transition-opacity"
                      onclick={(e) => { e.stopPropagation(); handleRemoveModel(provider.id, model.name); }}
                    >
                      ✕
                    </button>
                    {/if}
                  </div>
                {/each}
              </div>
              {#if provider.id !== 'sensevoice'}
              <div class="flex items-center gap-1.5">
                <button
                  class="text-caption text-accent-foreground hover:underline"
                  onclick={() => openAddModelDialog(provider.id)}
                >
                  + 添加模型
                </button>
                <span class="text-border">|</span>
                <button
                  class="text-caption text-accent-foreground hover:underline inline-flex items-center gap-0.5"
                  onclick={() => testAllModels(provider)}
                  disabled={provider.models.length === 0 || [...testingModels].some(k => k.startsWith(provider.id + '::'))}
                >
                  ⟳ 测试全部
                </button>
              </div>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  </section>
  <Dialog
    open={deleteConfirmDialog.isOpen}
    onOpenChange={deleteConfirmDialog.onOpenChange}
    title="删除 Provider"
    description="确定要删除该 Provider 吗？此操作无法撤销。"
  >
    {#snippet footer()}
      <DialogFooter
        onCancel={() => deleteConfirmDialog.close()}
        onConfirm={confirmRemoveProvider}
        confirmText="删除"
        confirmVariant="danger"
      />
    {/snippet}
  </Dialog>

  <Dialog
    open={resetConfirmDialog.isOpen}
    onOpenChange={resetConfirmDialog.onOpenChange}
    title="重置 Provider"
    description="确定要重置为默认设置吗？API Key 和自定义模型将被覆盖。"
  >
    {#snippet footer()}
      <DialogFooter
        onCancel={() => resetConfirmDialog.close()}
        onConfirm={confirmResetProvider}
        confirmText="重置"
      />
    {/snippet}
  </Dialog>

  <Dialog
    open={removeModelDialog.isOpen}
    onOpenChange={removeModelDialog.onOpenChange}
    title="删除模型"
    description="确定要删除该模型吗？此操作无法撤销。"
  >
    {#snippet footer()}
      <DialogFooter
        onCancel={() => removeModelDialog.close()}
        onConfirm={confirmRemoveModel}
        confirmText="删除"
        confirmVariant="danger"
      />
    {/snippet}
  </Dialog>

  <Dialog
    open={addModelDialog.isOpen}
    onOpenChange={addModelDialog.onOpenChange}
    title="添加模型"
    description="为 Provider 添加新模型"
  >
    {#snippet children()}
      <div>
        <label for="model-name" class="block text-body text-foreground-alt mb-1">模型名称</label>
        <input
          id="model-name"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：gpt-4o"
          bind:value={newModelName}
        />
      </div>
      <div>
        <span class="block text-body text-foreground-alt mb-1">能力</span>
        <div class="flex flex-wrap gap-2">
          {#each MODEL_CAPABILITIES as cap}
            <label class="inline-flex items-center gap-1.5 text-caption text-foreground cursor-pointer">
              <input
                type="checkbox"
                class="rounded border-border-input"
                checked={newModelCapabilities.includes(cap.value)}
                onchange={() => toggleCapability(cap.value)}
              />
              {cap.label}
            </label>
          {/each}
        </div>
      </div>
    {/snippet}

    {#snippet footer()}
      <DialogFooter
        onCancel={() => addModelDialog.close()}
        onConfirm={confirmAddModel}
        confirmText="添加"
        confirmDisabled={!newModelName.trim()}
      />
    {/snippet}
  </Dialog>
</div>

<style>
  @media (max-width: 640px) {
    .ai-service-container {
      flex-direction: column;
    }
    
    .ai-service-container > div {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    
    .ai-service-container > div:last-child {
      border-bottom: none;
    }
  }
</style>
