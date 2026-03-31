<script lang="ts">
  import { onMount } from 'svelte';
  import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import PasswordInput from '$lib/components/ui/password-input/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { Plus, RotateCcw } from 'lucide-svelte';
  import type { ProviderConfig, AppConfig, ModelConfig, ModelVerified } from '$lib/stores/config';
  import { SENSEVOICE_LANGUAGES } from '$lib/stores/config';

  let showAddDialog = $state(false);
  let newProvider = $state({
    name: '',
    type: '',
    id: '',
    endpoint: ''
  });
  let formErrors = $state<Record<string, string>>({});
  let showDeleteConfirm = $state(false);
  let showResetConfirm = $state(false);
  let pendingActionProviderId = $state('');
  let showAddModelDialog = $state(false);
  let addModelProviderId = $state('');
  let newModelName = $state('');
  let newModelCapabilities = $state<string[]>([]);
  let showRemoveModelConfirm = $state(false);
  let pendingRemoveModel = $state<{ providerId: string; modelName: string }>({ providerId: '', modelName: '' });
  let testingModels = $state<Set<string>>(new Set());
  let vertexEnvInfo = $state<{ project: string; location: string } | null>(null);
  let sensevoiceStatus = $state<{ status: string; size_bytes?: number } | null>(null);
  let sensevoiceDownloading = $state(false);
  let sensevoiceDownloadProgress = $state({ file: '', percent: 0, downloaded: 0, total: 0 });
  let sensevoiceLanguage = $state(0);

  async function loadSenseVoiceStatus() {
    try {
      sensevoiceStatus = await invoke<{ status: string; size_bytes?: number }>('get_sensevoice_status');
    } catch {
      sensevoiceStatus = null;
    }
  }

  async function downloadSenseVoice() {
    sensevoiceDownloading = true;
    try {
      await invoke('download_sensevoice_model');
      await loadSenseVoiceStatus();
    } catch (e) {
      console.error('Download failed:', e);
    } finally {
      sensevoiceDownloading = false;
    }
  }

  async function deleteSenseVoiceModel() {
    try {
      await invoke('delete_sensevoice_model');
      await loadSenseVoiceStatus();
    } catch (e) {
      console.error('Delete failed:', e);
    }
  }

  const PROVIDER_TYPES = [
    { value: 'openai-compatible', label: 'OpenAI Compatible' },
    { value: 'anthropic-compatible', label: 'Anthropic Compatible' }
  ];

  onMount(async () => {
    config.load();
    try {
      vertexEnvInfo = await invoke<{ project: string; location: string }>('get_vertex_env_info');
    } catch {
      vertexEnvInfo = null;
    }
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
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        transcription: { provider_id: providerId, model, polish_enabled: $config.features.transcription.polish_enabled, polish_provider_id: $config.features.transcription.polish_provider_id, polish_model: $config.features.transcription.polish_model }
      }
    };
    config.save(newConfig);
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
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        transcription: {
          ...$config.features.transcription,
          polish_provider_id: providerId,
          polish_model: model
        }
      }
    };
    config.save(newConfig);
  }

  function handlePolishEnabled(enabled: boolean) {
    const newConfig: AppConfig = {
      ...$config,
      features: {
        ...$config.features,
        transcription: {
          ...$config.features.transcription,
          polish_enabled: enabled
        }
      }
    };
    config.save(newConfig);
  }

  function handleProviderFieldChange(
    providerId: string,
    field: string,
    value: string
  ) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId ? { ...p, [field]: value } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleApiKeyChange(providerId: string, value: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId ? { ...p, api_key: value } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleAddModel(providerId: string, model: ModelConfig) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: [...p.models, model] }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function openAddModelDialog(providerId: string) {
    addModelProviderId = providerId;
    newModelName = '';
    newModelCapabilities = [];
    showAddModelDialog = true;
  }

  function confirmAddModel() {
    if (!newModelName.trim()) return;
    const model: ModelConfig = {
      name: newModelName.trim(),
      capabilities: [...newModelCapabilities]
    };
    handleAddModel(addModelProviderId, model);
    showAddModelDialog = false;
    addModelProviderId = '';
    newModelName = '';
    newModelCapabilities = [];
  }

  function handleRemoveModel(providerId: string, modelName: string) {
    pendingRemoveModel = { providerId, modelName };
    showRemoveModelConfirm = true;
  }

  function confirmRemoveModel() {
    const { providerId, modelName } = pendingRemoveModel;
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: p.models.filter((m: ModelConfig) => m.name !== modelName) }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
    showRemoveModelConfirm = false;
    pendingRemoveModel = { providerId: '', modelName: '' };
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
    showDeleteConfirm = true;
  }

  function confirmRemoveProvider() {
    const newProviders = $config.ai.providers.filter(
      (p: ProviderConfig) => p.id !== pendingActionProviderId
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
    showDeleteConfirm = false;
    pendingActionProviderId = '';
  }

  function handleResetProvider(providerId: string) {
    pendingActionProviderId = providerId;
    showResetConfirm = true;
  }

  function confirmResetProvider() {
    const builtin = BUILTIN_PROVIDERS.find((p) => p.id === pendingActionProviderId);
    if (!builtin) return;
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === pendingActionProviderId ? { ...builtin } : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
    showResetConfirm = false;
    pendingActionProviderId = '';
  }

  function needsApiKey(provider: ProviderConfig): boolean {
    return provider.type === 'openai-compatible';
  }

  function generateSlug(name: string): string {
    return name
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

  function handleNameInput(value: string) {
    newProvider.name = value;
    if (!formErrors.id) {
      newProvider.id = generateSlug(value);
    }
  }

  function handleTypeChange(value: string) {
    newProvider.type = value;
    if (formErrors.type) {
      formErrors = { ...formErrors, type: '' };
    }
  }

  function validateForm(): boolean {
    const errors: Record<string, string> = {};
    if (!newProvider.name.trim()) errors.name = '请输入名称';
    if (!newProvider.type) errors.type = '请选择类型';
    if (!newProvider.id.trim()) errors.id = '请输入 ID';
    if (!newProvider.endpoint.trim()) errors.endpoint = '请输入端点';

    const idRegex = /^[a-z0-9-]+$/;
    if (newProvider.id && !idRegex.test(newProvider.id)) {
      errors.id = 'ID 仅允许小写字母、数字和连字符';
    }
    if (
      newProvider.id &&
      ($config.ai.providers || []).some((p: ProviderConfig) => p.id === newProvider.id)
    ) {
      errors.id = '该 ID 已存在';
    }

    formErrors = errors;
    return Object.keys(errors).length === 0;
  }

  async function handleAddProvider() {
    if (!validateForm()) return;

    const provider: ProviderConfig = {
      id: newProvider.id.trim(),
      type: newProvider.type,
      name: newProvider.name.trim(),
      endpoint: newProvider.endpoint.trim(),
      models: []
    };

    const newProviders = [...($config.ai.providers || []), provider];
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };

    try {
      await config.save(newConfig);
      newProvider = { name: '', type: '', id: '', endpoint: '' };
      formErrors = {};
      showAddDialog = false;
    } catch (error) {
      console.error('Failed to add provider:', error);
    }
  }

  function handleDialogOpenChange(open: boolean) {
    if (!open) {
      newProvider = { name: '', type: '', id: '', endpoint: '' };
      formErrors = {};
    }
    showAddDialog = open;
  }

  function formatTestDate(isoStr: string): string {
    try {
      return new Date(isoStr).toLocaleDateString(undefined, { month: '2-digit', day: '2-digit' });
    } catch {
      return '';
    }
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
      const result = await invoke<{ status: string; latency_ms?: number; message: string }>('test_model_connectivity', {
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
  <h2 class="text-2xl font-semibold text-foreground m-0 mb-8">模型</h2>

  <section class="mb-10">
    <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">转写服务</div>
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="flex flex-col gap-5">
        <div>
          <label class="block text-sm text-foreground-alt mb-1.5">转写模型</label>
          <GroupedSelect
            value={getTranscriptionValue()}
            groups={buildTranscriptionGroups()}
            placeholder="选择模型"
            onChange={handleTranscriptionChange}
          />
        </div>

        <div class="flex items-center justify-between">
          <div>
            <div class="text-[15px] font-semibold text-foreground">启用润色</div>
            <div class="text-sm text-foreground-alt">转写后自动使用 LLM 润色文字</div>
          </div>
          <button
            class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-2 {$config.features.transcription.polish_enabled ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to shadow-btn-primary' : 'bg-gradient-to-b from-toggle-off-from to-toggle-off-to shadow-btn-secondary'}"
            role="switch"
            aria-checked={$config.features.transcription.polish_enabled}
            onclick={() => handlePolishEnabled(!$config.features.transcription.polish_enabled)}
          >
            <span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-gradient-to-b from-toggle-thumb-from to-toggle-thumb-to shadow ring-0 transition duration-200 ease-in-out {$config.features.transcription.polish_enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
          </button>
        </div>

        {#if $config.features.transcription.polish_enabled}
        <div>
          <label class="block text-sm text-foreground-alt mb-1.5">润色模型</label>
          <GroupedSelect
            value={getPolishValue()}
            groups={buildPolishGroups()}
            placeholder="选择模型"
            onChange={handlePolishChange}
          />
        </div>
        {/if}
      </div>
    </div>
  </section>

  <section>
    <div class="text-xs text-muted-foreground uppercase tracking-wider mb-3">Providers</div>
    <div class="grid grid-cols-2 gap-4">
      {#each $config.ai.providers || [] as provider (provider.id)}
        <div class="rounded-xl border border-border bg-background-alt p-5">
          <div class="flex justify-between items-start mb-4">
            <div>
              <div class="text-[15px] font-semibold text-foreground">{provider.name}</div>
              <div class="text-[11px] text-muted-foreground mt-0.5">{provider.id}</div>
            </div>
            {#if isBuiltinProvider(provider.id)}
              <button
                class="text-xs text-muted-foreground hover:text-foreground transition-colors p-0.5"
                onclick={() => handleResetProvider(provider.id)}
                title="重置为默认"
              >
                <RotateCcw class="h-3.5 w-3.5" />
              </button>
            {:else}
              <button
                class="text-xs text-muted-foreground hover:text-destructive transition-colors p-0.5"
                onclick={() => handleRemoveProvider(provider.id)}
              >
                ✕
              </button>
            {/if}
          </div>

          {#if provider.type === 'sensevoice'}
            <div class="mb-3">
              <label class="block text-sm text-foreground-alt mb-1">模型状态</label>
              <div class="text-[11px] bg-background rounded-md border border-border p-2 space-y-1">
                {#if sensevoiceStatus?.status === 'ready'}
                  <div class="flex items-center justify-between">
                    <span class="text-green-500">已就绪</span>
                    <span class="text-muted-foreground">{(sensevoiceStatus!.size_bytes! / 1024 / 1024).toFixed(0)} MB</span>
                  </div>
                  <button
                    class="text-xs text-red-400 hover:text-red-300 transition-colors"
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
                    class="text-xs text-accent-foreground hover:underline"
                    onclick={downloadSenseVoice}
                  >
                    下载模型 (约 242 MB)
                  </button>
                {/if}
              </div>
            </div>
            <div class="mb-3">
              <label class="block text-sm text-foreground-alt mb-1">转写语言</label>
              <select
                class="flex h-9 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
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
              <label class="block text-sm text-foreground-alt mb-1">API Key</label>
              <PasswordInput
                value={provider.api_key || ''}
                placeholder="sk-..."
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          {#if provider.type === 'vertex'}
            <div class="mb-3">
              <label class="block text-sm text-foreground-alt mb-1">Vertex AI 配置</label>
              <div class="text-[11px] text-muted-foreground space-y-0.5 bg-background rounded-md border border-border p-2">
                <div>GOOGLE_CLOUD_PROJECT: <span class="text-foreground">{vertexEnvInfo?.project || '未设置'}</span></div>
                <div>GOOGLE_CLOUD_LOCATION: <span class="text-foreground">{vertexEnvInfo?.location || 'global'}</span></div>
                <div class="text-muted-foreground/70 mt-1">认证: <code class="text-[10px] bg-background px-1 py-0.5 rounded border border-border">gcloud auth application-default login</code></div>
              </div>
            </div>
          {:else if provider.type !== 'sensevoice'}
            <div class="mb-3">
              <label class="block text-sm text-foreground-alt mb-1">Endpoint</label>
              <input
                class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
                type="text"
                value={provider.endpoint}
                onchange={(e) => handleProviderFieldChange(provider.id, 'endpoint', (e.target as HTMLInputElement).value)}
              />
            </div>
          {/if}

          <div>
            <label class="block text-sm text-foreground-alt mb-1">Models</label>
            <div class="mt-1">
              <div class="flex flex-wrap gap-1 mb-1">
                {#each provider.models || [] as model (model.name)}
                  {@const verified = model.verified}
                  {@const testing = isTesting(provider.id, model.name)}
                  <span
                    class="inline-flex items-center gap-1 rounded px-2.5 py-1 text-[11px] text-accent-foreground cursor-pointer select-none
                      {verified?.status === 'ok' ? 'bg-green-500/15 border border-green-500/30' : ''}
                      {verified?.status === 'error' ? 'bg-red-500/15 border border-red-500/30' : ''}
                      {!verified && !testing ? 'bg-accent' : ''}
                      {testing ? 'bg-accent animate-pulse' : ''}"
                    title={verified ? `${verified.status === 'ok' ? '验证通过' : '验证失败'}${verified.latency_ms ? ' · ' + verified.latency_ms + 'ms' : ''}${verified.message ? ' · ' + verified.message : ''}` : '点击测试'}
                    onclick={() => testModel(provider.id, model.name)}
                  >
                    {model.name}
                    {#if model.capabilities?.includes('transcription')}
                      <span class="text-[9px] opacity-70">T</span>
                    {/if}
                    {#if testing}
                      <span class="animate-spin text-[10px]">⟳</span>
                    {:else if verified?.status === 'ok'}
                      <span class="text-green-500 text-[10px]">✓</span>
                      <span class="text-[9px] text-green-500/70">{formatTestDate(verified.tested_at)}</span>
                    {:else if verified?.status === 'error'}
                      <span class="text-red-500 text-[10px]">✕</span>
                      <span class="text-[9px] text-red-500/70">{formatTestDate(verified.tested_at)}</span>
                    {/if}
                    <button
                      class="opacity-60 hover:opacity-100 transition-opacity"
                      onclick={(e) => { e.stopPropagation(); handleRemoveModel(provider.id, model.name); }}
                    >
                      ✕
                    </button>
                  </span>
                {/each}
              </div>
              <div class="flex items-center gap-1.5">
                <button
                  class="text-xs text-accent-foreground hover:underline"
                  onclick={() => openAddModelDialog(provider.id)}
                >
                  + 添加模型
                </button>
                <span class="text-border">|</span>
                <button
                  class="text-xs text-accent-foreground hover:underline inline-flex items-center gap-0.5"
                  onclick={() => testAllModels(provider)}
                  disabled={provider.models.length === 0 || [...testingModels].some(k => k.startsWith(provider.id + '::'))}
                >
                  ⟳ 测试全部
                </button>
              </div>
            </div>
          </div>
        </div>
      {/each}
      <button
        class="rounded-xl border-2 border-dashed border-border bg-background-alt/50 hover:bg-background-alt transition-colors flex flex-col items-center justify-center gap-1.5 cursor-pointer p-5 min-h-[160px]"
        onclick={() => (showAddDialog = true)}
      >
        <Plus class="h-6 w-6 text-muted-foreground" />
        <span class="text-sm text-muted-foreground">添加 Provider</span>
      </button>
    </div>
  </section>
  <Dialog
    open={showAddDialog}
    onOpenChange={handleDialogOpenChange}
    title="添加 Provider"
    description="配置新的 AI 服务提供商"
  >
    {#snippet children()}
      <div>
        <label for="provider-name" class="block text-sm text-foreground-alt mb-1">Name</label>
        <input
          id="provider-name"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：阿里云"
          value={newProvider.name}
          oninput={(e) => handleNameInput((e.target as HTMLInputElement).value)}
        />
        {#if formErrors.name}
          <p class="text-xs text-destructive mt-0.5">{formErrors.name}</p>
        {/if}
      </div>

      <div>
        <span class="block text-sm text-foreground-alt mb-1">Type</span>
        <GroupedSelect
          value={newProvider.type}
          groups={[{ label: '', items: PROVIDER_TYPES }]}
          placeholder="选择类型"
          onChange={handleTypeChange}
        />
        {#if formErrors.type}
          <p class="text-xs text-destructive mt-0.5">{formErrors.type}</p>
        {/if}
      </div>

      <div>
        <label for="provider-id" class="block text-sm text-foreground-alt mb-1">ID</label>
        <input
          id="provider-id"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：ali-yun"
          value={newProvider.id}
          oninput={(e) => { newProvider.id = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, id: '' }; }}
        />
        {#if formErrors.id}
          <p class="text-xs text-destructive mt-0.5">{formErrors.id}</p>
        {/if}
      </div>

      <div>
        <label for="provider-endpoint" class="block text-sm text-foreground-alt mb-1">Endpoint</label>
        <input
          id="provider-endpoint"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="https://api.example.com/v1"
          value={newProvider.endpoint}
          oninput={(e) => { newProvider.endpoint = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, endpoint: '' }; }}
        />
        {#if formErrors.endpoint}
          <p class="text-xs text-destructive mt-0.5">{formErrors.endpoint}</p>
        {/if}
      </div>
    {/snippet}

    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
        onclick={() => handleDialogOpenChange(false)}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-4 py-2 text-sm text-background hover:bg-foreground/90 transition-colors"
        onclick={handleAddProvider}
      >
        添加
      </button>
    {/snippet}
  </Dialog>

  <Dialog
    open={showDeleteConfirm}
    onOpenChange={(open) => { showDeleteConfirm = open; if (!open) pendingActionProviderId = ''; }}
    title="删除 Provider"
    description="确定要删除该 Provider 吗？此操作无法撤销。"
  >
    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
        onclick={() => { showDeleteConfirm = false; pendingActionProviderId = ''; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-destructive px-4 py-2 text-sm text-white hover:bg-destructive/90 transition-colors"
        onclick={confirmRemoveProvider}
      >
        删除
      </button>
    {/snippet}
  </Dialog>

  <Dialog
    open={showResetConfirm}
    onOpenChange={(open) => { showResetConfirm = open; if (!open) pendingActionProviderId = ''; }}
    title="重置 Provider"
    description="确定要重置为默认设置吗？自定义的 Endpoint、API Key 和 Models 将被覆盖。"
  >
    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
        onclick={() => { showResetConfirm = false; pendingActionProviderId = ''; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-4 py-2 text-sm text-background hover:bg-foreground/90 transition-colors"
        onclick={confirmResetProvider}
      >
        重置
      </button>
    {/snippet}
  </Dialog>

  <Dialog
    open={showRemoveModelConfirm}
    onOpenChange={(open) => { showRemoveModelConfirm = open; if (!open) pendingRemoveModel = { providerId: '', modelName: '' }; }}
    title="删除模型"
    description="确定要删除该模型吗？此操作无法撤销。"
  >
    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
        onclick={() => { showRemoveModelConfirm = false; pendingRemoveModel = { providerId: '', modelName: '' }; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-destructive px-4 py-2 text-sm text-white hover:bg-destructive/90 transition-colors"
        onclick={confirmRemoveModel}
      >
        删除
      </button>
    {/snippet}
  </Dialog>

  <Dialog
    open={showAddModelDialog}
    onOpenChange={(open) => { showAddModelDialog = open; if (!open) { addModelProviderId = ''; newModelName = ''; newModelCapabilities = []; } }}
    title="添加模型"
    description="为 Provider 添加新模型"
  >
    {#snippet children()}
      <div>
        <label for="model-name" class="block text-sm text-foreground-alt mb-1">模型名称</label>
        <input
          id="model-name"
          class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：gpt-4o"
          bind:value={newModelName}
        />
      </div>
      <div>
        <span class="block text-sm text-foreground-alt mb-1">能力</span>
        <div class="flex flex-wrap gap-2">
          {#each MODEL_CAPABILITIES as cap}
            <label class="inline-flex items-center gap-1.5 text-xs text-foreground cursor-pointer">
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
      <button
        type="button"
        class="rounded-md border border-border px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors"
        onclick={() => { showAddModelDialog = false; addModelProviderId = ''; newModelName = ''; newModelCapabilities = []; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-4 py-2 text-sm text-background hover:bg-foreground/90 transition-colors"
        onclick={confirmAddModel}
        disabled={!newModelName.trim()}
      >
        添加
      </button>
    {/snippet}
  </Dialog>
</div>
