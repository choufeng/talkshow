<script lang="ts">
  import { onboarding } from '$lib/stores/onboarding';
  import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES } from '$lib/stores/config';
  import type { ProviderConfig, ModelConfig } from '$lib/stores/config';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { updateNestedPath, invokeWithError } from '$lib/ai/shared';
  import { generateSlug } from '$lib/utils';
  import EditableField from '$lib/components/ui/editable-field/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import DialogFooter from '$lib/components/ui/dialog-footer/index.svelte';
  import { Server, CheckCircle2, AlertCircle, Loader2, Plus, Cloud, Zap, Info } from 'lucide-svelte';

  type Scene = 'loading' | 'vertex-detected' | 'manual-config';
  let scene = $state<Scene>('loading');
  let vertexEnvInfo = $state<{ project: string; location: string } | null>(null);
  let vertexTesting = $state(false);
  let vertexTestResult = $state<{ status: string; latency_ms?: number; message: string } | null>(null);

  let testingModels = $state<Set<string>>(new Set());
  let testResults = $state<Map<string, { status: string; latency_ms?: number; message: string }>>(new Map());

  let newProvider = $state({ name: '', type: '', id: '', endpoint: '' });
  let formErrors = $state<Record<string, string>>({});
  let addProviderDialogOpen = $state(false);

  let newModelName = $state('');
  let newModelCapabilities = $state<string[]>([]);
  let addModelProviderId = $state('');
  let addModelDialogOpen = $state(false);

  const PROVIDER_TYPES = [
    { value: 'openai-compatible', label: 'OpenAI Compatible' },
    { value: 'anthropic-compatible', label: 'Anthropic Compatible' }
  ];

  function classifyProviderError(msg: string): string {
    const lower = msg.toLowerCase();
    if (lower.includes('unauthorized') || lower.includes('401') || lower.includes('invalid api') || lower.includes('api key') || lower.includes('authentication')) {
      return 'API Key 无效或已过期，请检查后重新输入';
    }
    if (lower.includes('forbidden') || lower.includes('403') || lower.includes('permission')) {
      return '权限不足，请确认该 API Key 有访问权限';
    }
    if (lower.includes('timeout') || lower.includes('timed out')) {
      return '连接超时，请检查网络后重试';
    }
    if (lower.includes('network') || lower.includes('connection') || lower.includes('dns') || lower.includes('enotfound')) {
      return '网络连接失败，请检查网络设置';
    }
    if (lower.includes('404') || lower.includes('not found')) {
      return '模型不存在或 Endpoint 配置错误';
    }
    if (lower.includes('429') || lower.includes('rate limit') || lower.includes('quota')) {
      return '请求频率超限，请稍后重试';
    }
    return msg || '连接测试失败';
  }

  let hasAnyVerifiedProvider = $derived.by(() => {
    const providers = $config.ai.providers || [];
    for (const p of providers) {
      for (const m of p.models || []) {
        if (m.verified?.status === 'ok') return true;
      }
    }
    return false;
  });

  function checkStepValid() {
    onboarding.setStepValid(3, hasAnyVerifiedProvider);
  }

  function hasProviderWithApiKey(p: ProviderConfig): boolean {
    return p.type === 'openai-compatible' && !!p.api_key?.trim();
  }

  async function detectEnvironment() {
    try {
      const envInfo = await invoke<{ project: string; location: string }>('get_vertex_env_info');
      vertexEnvInfo = envInfo;

      if (envInfo.project) {
        scene = 'vertex-detected';
        await autoTestVertex();
      } else {
        scene = 'manual-config';
        if (hasAnyVerifiedProvider) {
          onboarding.setStepValid(3, true);
        }
      }
    } catch {
      scene = 'manual-config';
    }
  }

  async function autoTestVertex() {
    vertexTesting = true;
    vertexTestResult = null;
    try {
      const result = await invokeWithError<{ status: string; latency_ms?: number; message: string }>(
        'test_model_connectivity',
        { providerId: 'vertex', modelName: 'gemini-2.5-flash' }
      );
      if (result) {
        vertexTestResult = result;
        await config.load();
        if (result.status === 'ok') {
          onboarding.setStepValid(3, true);
        }
      }
    } catch (e) {
      console.error('Vertex test failed:', e);
    } finally {
      vertexTesting = false;
    }
  }

  async function testModel(providerId: string, modelName: string) {
    const key = `${providerId}::${modelName}`;
    testingModels = new Set([...testingModels, key]);
    try {
      const result = await invokeWithError<{ status: string; latency_ms?: number; message: string }>(
        'test_model_connectivity',
        { providerId, modelName }
      );
      if (result) {
        testResults = new Map([...testResults, [key, result]]);
        await config.load();
        checkStepValid();
      }
    } catch (e) {
      console.error('Test failed:', e);
    } finally {
      const next = new Set(testingModels);
      next.delete(key);
      testingModels = next;
    }
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

  function handleProviderFieldChange(providerId: string, field: string, value: string) {
    config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
      (providers as ProviderConfig[]).map((p) =>
        p.id === providerId ? { ...p, [field]: value } : p
      )
    ));
  }

  function handleNameInput(value: string) {
    newProvider.name = value;
    if (!formErrors.id) {
      newProvider.id = generateSlug(value);
    }
  }

  function validateForm(): boolean {
    const errors: Record<string, string> = {};
    if (!newProvider.name.trim()) errors.name = '请输入名称';
    if (!newProvider.type) errors.type = '请选择类型';
    if (!newProvider.id.trim()) errors.id = '请输入 ID';

    const needsEndpoint = newProvider.type === 'openai-compatible' || newProvider.type === 'anthropic-compatible';
    if (needsEndpoint && !newProvider.endpoint.trim()) errors.endpoint = '请输入端点';

    if (newProvider.endpoint.trim()) {
      try {
        const parsed = new URL(newProvider.endpoint);
        if (!['http:', 'https:'].includes(parsed.protocol)) {
          errors.endpoint = '必须以 http:// 或 https:// 开头';
        }
      } catch {
        errors.endpoint = '格式无效';
      }
    }

    const idRegex = /^[a-z0-9-]+$/;
    if (newProvider.id && !idRegex.test(newProvider.id)) {
      errors.id = '仅允许小写字母、数字和连字符';
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

  function handleAddProvider() {
    if (!validateForm()) return;

    const provider: ProviderConfig = {
      id: newProvider.id.trim(),
      type: newProvider.type,
      name: newProvider.name.trim(),
      endpoint: newProvider.endpoint.trim(),
      models: []
    };

    const newProviders = [...($config.ai.providers || []), provider];
    config.save(updateNestedPath($config, ['ai', 'providers'], () => newProviders));
    closeAddProviderDialog();
  }

  function openAddProviderDialog() {
    newProvider = { name: '', type: '', id: '', endpoint: '' };
    formErrors = {};
    addProviderDialogOpen = true;
  }

  function closeAddProviderDialog() {
    addProviderDialogOpen = false;
    newProvider = { name: '', type: '', id: '', endpoint: '' };
    formErrors = {};
  }

  function openAddModelDialog(providerId: string) {
    addModelProviderId = providerId;
    newModelName = '';
    newModelCapabilities = [];
    addModelDialogOpen = true;
  }

  function closeAddModelDialog() {
    addModelDialogOpen = false;
    addModelProviderId = '';
    newModelName = '';
    newModelCapabilities = [];
  }

  function confirmAddModel() {
    if (!newModelName.trim()) return;
    const model: ModelConfig = {
      name: newModelName.trim(),
      capabilities: [...newModelCapabilities]
    };
    config.save(updateNestedPath($config, ['ai', 'providers'], (providers) =>
      (providers as ProviderConfig[]).map((p) =>
        p.id === addModelProviderId
          ? { ...p, models: [...p.models, model] }
          : p
      )
    ));
    closeAddModelDialog();
  }

  function toggleCapability(cap: string) {
    if (newModelCapabilities.includes(cap)) {
      newModelCapabilities = newModelCapabilities.filter((c) => c !== cap);
    } else {
      newModelCapabilities = [...newModelCapabilities, cap];
    }
  }

  onMount(() => {
    detectEnvironment();
  });
</script>

<div>
  <div class="text-center mb-6">
    <div class="w-14 h-14 rounded-full bg-accent/50 flex items-center justify-center mx-auto mb-4">
      <Server size={28} class="text-accent-foreground" />
    </div>
    <h2 class="text-subheading font-semibold text-foreground mb-2">AI 服务配置</h2>
    <p class="text-body text-foreground-alt">
      配置至少一个可用的 AI 服务提供商，用于语音转写和翻译。
    </p>
  </div>

  {#if scene === 'loading'}
    <div class="flex items-center justify-center py-8">
      <Loader2 size={24} class="animate-spin text-muted-foreground" />
      <span class="ml-2 text-body text-muted-foreground">检测环境...</span>
    </div>

  {:else if scene === 'vertex-detected'}
    <div class="rounded-xl border border-border bg-background-alt p-5">
      <div class="flex items-center gap-3 mb-4">
        <div class="w-10 h-10 bg-blue-500/10 rounded-lg flex items-center justify-center">
          <Cloud size={20} class="text-blue-500" />
        </div>
        <div>
          <div class="text-[15px] font-semibold text-foreground">Vertex AI 已自动检测</div>
          <div class="text-caption text-muted-foreground">Google Cloud Vertex AI API</div>
        </div>
      </div>

      <div class="space-y-3 mb-4">
        <div class="flex items-start gap-2 text-body text-foreground-alt">
          <Zap size={14} class="mt-0.5 shrink-0 text-amber-500" />
          <span>支持语音转写、翻译润色等 AI 处理功能</span>
        </div>
        <div class="flex items-start gap-2 text-body text-foreground-alt">
          <Info size={14} class="mt-0.5 shrink-0 text-blue-400" />
          <span>目前不支持直接音频输入（使用文本测试）</span>
        </div>
      </div>

      <div class="text-[11px] bg-background rounded-md border border-border p-3 mb-4 space-y-1">
        <div>项目 ID: <span class="text-foreground">{vertexEnvInfo?.project || '未知'}</span></div>
        <div>Location: <span class="text-foreground">{vertexEnvInfo?.location || 'global'}</span></div>
        <div class="text-muted-foreground/70 mt-1">认证: <code class="text-[10px] bg-background px-1 py-0.5 rounded border border-border">gcloud auth application-default login</code></div>
      </div>

      {#if vertexTesting}
        <div class="flex items-center justify-center py-3">
          <Loader2 size={18} class="animate-spin text-accent-foreground" />
          <span class="ml-2 text-body text-muted-foreground">测试连通性...</span>
        </div>
      {:else if vertexTestResult}
        <div class="flex items-center gap-2 py-2 px-3 rounded-lg {vertexTestResult.status === 'ok' ? 'bg-green-500/10 border border-green-500/20' : 'bg-red-500/10 border border-red-500/20'}">
          {#if vertexTestResult.status === 'ok'}
            <CheckCircle2 size={18} class="text-green-500" />
            <div>
              <span class="text-body text-green-500 font-medium">连接成功</span>
              {#if vertexTestResult.latency_ms}
                <span class="text-caption text-muted-foreground ml-2">{vertexTestResult.latency_ms}ms</span>
              {/if}
            </div>
          {:else}
            <AlertCircle size={18} class="text-red-400" />
            <div>
              <span class="text-body text-red-400 font-medium">连接失败</span>
              <span class="text-caption text-muted-foreground ml-2">{classifyProviderError(vertexTestResult.message || '')}</span>
            </div>
          {/if}
        </div>
        {#if vertexTestResult.status !== 'ok'}
          <button
            class="mt-3 w-full py-2 rounded-lg text-body font-medium transition-colors border border-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-foreground shadow-btn-secondary hover:bg-muted/50"
            onclick={autoTestVertex}
          >
            重试
          </button>
        {/if}
      {/if}
    </div>

    <div class="mt-4 text-center">
      <button
        class="text-caption text-muted-foreground hover:text-foreground transition-colors underline"
        onclick={() => { scene = 'manual-config'; checkStepValid(); }}
      >
        手动配置其他 Provider
      </button>
    </div>

  {:else if scene === 'manual-config'}
    <div class="space-y-3">
      {#each $config.ai.providers || [] as provider (provider.id)}
        {@const isTested = provider.models?.some((m: ModelConfig) => m.verified?.status === 'ok')}
        <div class="rounded-xl border border-border bg-background-alt p-4 {isTested ? 'ring-1 ring-green-500/30' : ''}">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-2">
              <div class="w-8 h-8 bg-accent/10 rounded-lg flex items-center justify-center">
                <Server size={16} class="text-accent-foreground" />
              </div>
              <div>
                <div class="text-body font-medium text-foreground">{provider.name}</div>
                <div class="text-[11px] text-muted-foreground">{provider.id}</div>
              </div>
            </div>
            {#if isTested}
              <div class="flex items-center gap-1 text-green-500">
                <CheckCircle2 size={14} />
                <span class="text-caption">已验证</span>
              </div>
            {/if}
          </div>

          {#if provider.type === 'vertex' && vertexEnvInfo}
            <div class="text-[11px] bg-background rounded-md border border-border p-2 mb-3 space-y-0.5">
              <div>GOOGLE_CLOUD_PROJECT: <span class="text-foreground">{vertexEnvInfo.project || '未设置'}</span></div>
              <div>GOOGLE_CLOUD_LOCATION: <span class="text-foreground">{vertexEnvInfo.location || 'global'}</span></div>
            </div>
          {/if}

          {#if provider.type === 'openai-compatible'}
            <div class="mb-3">
              <span class="block text-caption text-foreground-alt mb-1">API Key</span>
              <EditableField
                value={provider.api_key || ''}
                aria-label="API Key"
                placeholder="sk-..."
                mode="password"
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          <div class="mb-2">
            <div class="flex flex-wrap gap-1.5">
              {#each provider.models || [] as model (model.name)}
                {@const key = `${provider.id}::${model.name}`}
                {@const testing = testingModels.has(key)}
                {@const result = testResults.get(key) || model.verified}
                {@const isSensevoice = provider.type === 'sensevoice'}
                <div
                  role="button"
                  tabindex="0"
                  class="inline-flex items-center gap-1 rounded px-2.5 py-1 text-[11px] text-accent-foreground {!isSensevoice ? 'cursor-pointer' : 'cursor-default'} select-none
                    {result?.status === 'ok' ? 'bg-green-500/15 border border-green-500/30' : ''}
                    {result?.status === 'error' ? 'bg-red-500/15 border border-red-500/30' : ''}
                    {!result && !testing ? 'bg-accent' : ''}
                    {testing ? 'bg-accent animate-pulse' : ''}"
                  title={!isSensevoice ? (result ? `${result.status === 'ok' ? '验证通过' : classifyProviderError(result.message || '验证失败')}${result.latency_ms ? ' · ' + result.latency_ms + 'ms' : ''}` : '点击测试') : ''}
                  onclick={() => { if (!isSensevoice) testModel(provider.id, model.name); }}
                  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (!isSensevoice) testModel(provider.id, model.name); } }}
                >
                  {model.name}
                  {#if testing}
                    <span class="animate-spin text-[10px]">⟳</span>
                  {:else if result?.status === 'ok'}
                    <span class="text-green-500 text-[10px]">✓</span>
                  {:else if result?.status === 'error'}
                    <span class="text-red-500 text-[10px]">✕</span>
                  {/if}
                </div>
              {/each}
            </div>
          </div>

          {#if provider.type !== 'sensevoice'}
            <div class="flex items-center gap-1.5">
              <button
                class="text-caption text-accent-foreground hover:underline"
                onclick={() => openAddModelDialog(provider.id)}
              >
                + 添加模型
      </button>

      {#if !hasAnyVerifiedProvider && ($config.ai.providers || []).length > 0}
        <div class="mt-3 text-center text-caption text-muted-foreground">
          点击模型名称测试连通性，至少需要一个模型验证通过
        </div>
      {/if}
    </div>
          {/if}
        </div>
      {/each}

      <button
        class="w-full rounded-xl border-2 border-dashed border-border bg-background-alt/50 hover:bg-background-alt transition-colors flex items-center justify-center gap-2 cursor-pointer py-4"
        onclick={openAddProviderDialog}
      >
        <Plus size={16} class="text-muted-foreground" />
        <span class="text-body text-muted-foreground">添加自定义 Provider</span>
      </button>
    </div>

    {#if vertexEnvInfo?.project}
      <div class="mt-4 text-center">
        <button
          class="text-caption text-muted-foreground hover:text-foreground transition-colors underline"
          onclick={() => { scene = 'vertex-detected'; }}
        >
          使用 Vertex AI
        </button>
      </div>
    {/if}
  {/if}
</div>

<Dialog
  open={addProviderDialogOpen}
  onOpenChange={(open: boolean) => { if (!open) closeAddProviderDialog(); else addProviderDialogOpen = true; }}
  title="添加 Provider"
  description="配置新的 AI 服务提供商"
>
  {#snippet children()}
    <div>
      <label for="onb-provider-name" class="block text-body text-foreground-alt mb-1">名称</label>
      <input
        id="onb-provider-name"
        class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
        type="text"
        placeholder="如：阿里云"
        value={newProvider.name}
        oninput={(e) => handleNameInput((e.target as HTMLInputElement).value)}
      />
      {#if formErrors.name}
        <p class="text-caption text-destructive mt-0.5">{formErrors.name}</p>
      {/if}
    </div>

    <div>
      <span class="block text-body text-foreground-alt mb-1">类型</span>
      <select
        class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20"
        value={newProvider.type}
        onchange={(e) => {
          const val = (e.target as HTMLSelectElement).value;
          newProvider.type = val;
          if (formErrors.type) formErrors = { ...formErrors, type: '' };
        }}
      >
        <option value="" disabled>选择类型</option>
        {#each PROVIDER_TYPES as pt}
          <option value={pt.value}>{pt.label}</option>
        {/each}
      </select>
      {#if formErrors.type}
        <p class="text-caption text-destructive mt-0.5">{formErrors.type}</p>
      {/if}
    </div>

    <div>
      <label for="onb-provider-id" class="block text-body text-foreground-alt mb-1">ID</label>
      <input
        id="onb-provider-id"
        class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
        type="text"
        placeholder="如：ali-yun"
        value={newProvider.id}
        oninput={(e) => { newProvider.id = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, id: '' }; }}
      />
      {#if formErrors.id}
        <p class="text-caption text-destructive mt-0.5">{formErrors.id}</p>
      {/if}
    </div>

    <div>
      <label for="onb-provider-endpoint" class="block text-body text-foreground-alt mb-1">Endpoint</label>
      <input
        id="onb-provider-endpoint"
        class="flex h-10 w-full rounded-md border border-border-input bg-background px-3 py-2 text-body ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
        type="text"
        placeholder="https://api.example.com/v1"
        value={newProvider.endpoint}
        oninput={(e) => { newProvider.endpoint = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, endpoint: '' }; }}
      />
      {#if formErrors.endpoint}
        <p class="text-caption text-destructive mt-0.5">{formErrors.endpoint}</p>
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <DialogFooter
      onCancel={closeAddProviderDialog}
      onConfirm={handleAddProvider}
      confirmText="添加"
    />
  {/snippet}
</Dialog>

<Dialog
  open={addModelDialogOpen}
  onOpenChange={(open: boolean) => { if (!open) closeAddModelDialog(); else addModelDialogOpen = true; }}
  title="添加模型"
  description="为 Provider 添加新模型"
>
  {#snippet children()}
    <div>
      <label for="onb-model-name" class="block text-body text-foreground-alt mb-1">模型名称</label>
      <input
        id="onb-model-name"
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
      onCancel={closeAddModelDialog}
      onConfirm={confirmAddModel}
      confirmText="添加"
      confirmDisabled={!newModelName.trim()}
    />
  {/snippet}
</Dialog>
