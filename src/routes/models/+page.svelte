<script lang="ts">
  import { onMount } from 'svelte';
  import { config, isBuiltinProvider, BUILTIN_PROVIDERS, MODEL_CAPABILITIES } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import PasswordInput from '$lib/components/ui/password-input/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import { Plus, RotateCcw } from 'lucide-svelte';
  import type { ProviderConfig, AppConfig, ModelConfig } from '$lib/stores/config';

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

  const PROVIDER_TYPES = [
    { value: 'openai-compatible', label: 'OpenAI Compatible' },
    { value: 'anthropic-compatible', label: 'Anthropic Compatible' },
    { value: 'stubbed', label: 'Stubbed' }
  ];

  onMount(() => {
    config.load();
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
    }));
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
        transcription: { provider_id: providerId, model }
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
</script>

<div class="max-w-[800px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">模型</h2>

  <section class="mb-7">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">Features</div>
    <div class="grid grid-cols-2 gap-3">
      <div class="rounded-lg border border-border bg-background-alt p-3.5">
        <div class="text-[13px] font-semibold text-foreground mb-0.5">Transcription</div>
        <div class="text-[11px] text-foreground-alt mb-2.5">转写服务</div>
        <GroupedSelect
          value={getTranscriptionValue()}
          groups={buildTranscriptionGroups()}
          placeholder="选择模型"
          onChange={handleTranscriptionChange}
        />
      </div>
    </div>
  </section>

  <section>
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">Providers</div>
    <div class="grid grid-cols-2 gap-3">
      {#each $config.ai.providers || [] as provider (provider.id)}
        <div class="rounded-lg border border-border bg-background-alt p-3.5">
          <div class="flex justify-between items-start mb-3">
            <div>
              <div class="text-sm font-semibold text-foreground">{provider.name}</div>
              <div class="text-[10px] text-muted-foreground mt-0.5">{provider.id}</div>
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

          {#if needsApiKey(provider)}
            <div class="mb-2.5">
              <label class="block text-[11px] text-foreground-alt mb-1">API Key</label>
              <PasswordInput
                value={provider.api_key || ''}
                placeholder="sk-..."
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          <div class="mb-2.5">
            <label class="block text-[11px] text-foreground-alt mb-1">Endpoint</label>
            <input
              class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
              type="text"
              value={provider.endpoint}
              onchange={(e) => handleProviderFieldChange(provider.id, 'endpoint', (e.target as HTMLInputElement).value)}
            />
          </div>

          <div>
            <label class="block text-[11px] text-foreground-alt mb-1">Models</label>
            <div class="mt-1">
              <div class="flex flex-wrap gap-1 mb-1">
                {#each provider.models || [] as model (model.name)}
                  <span class="inline-flex items-center gap-1 rounded bg-accent px-2 py-0.5 text-[10px] text-accent-foreground">
                    {model.name}
                    {#if model.capabilities?.includes('transcription')}
                      <span class="text-[8px] opacity-70">T</span>
                    {/if}
                    <button
                      class="opacity-60 hover:opacity-100 transition-opacity"
                      onclick={() => handleRemoveModel(provider.id, model.name)}
                    >
                      ✕
                    </button>
                  </span>
                {/each}
              </div>
              <button
                class="text-xs text-accent-foreground hover:underline"
                onclick={() => openAddModelDialog(provider.id)}
              >
                + 添加模型
              </button>
            </div>
          </div>
        </div>
      {/each}
      <button
        class="rounded-lg border-2 border-dashed border-border bg-background-alt/50 hover:bg-background-alt transition-colors flex flex-col items-center justify-center gap-1.5 cursor-pointer p-3.5 min-h-[140px]"
        onclick={() => (showAddDialog = true)}
      >
        <Plus class="h-5 w-5 text-muted-foreground" />
        <span class="text-[11px] text-muted-foreground">添加 Provider</span>
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
        <label for="provider-name" class="block text-[11px] text-foreground-alt mb-1">Name</label>
        <input
          id="provider-name"
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：阿里云"
          value={newProvider.name}
          oninput={(e) => handleNameInput((e.target as HTMLInputElement).value)}
        />
        {#if formErrors.name}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.name}</p>
        {/if}
      </div>

      <div>
        <span class="block text-[11px] text-foreground-alt mb-1">Type</span>
        <GroupedSelect
          value={newProvider.type}
          groups={[{ label: '', items: PROVIDER_TYPES }]}
          placeholder="选择类型"
          onChange={handleTypeChange}
        />
        {#if formErrors.type}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.type}</p>
        {/if}
      </div>

      <div>
        <label for="provider-id" class="block text-[11px] text-foreground-alt mb-1">ID</label>
        <input
          id="provider-id"
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：ali-yun"
          value={newProvider.id}
          oninput={(e) => { newProvider.id = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, id: '' }; }}
        />
        {#if formErrors.id}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.id}</p>
        {/if}
      </div>

      <div>
        <label for="provider-endpoint" class="block text-[11px] text-foreground-alt mb-1">Endpoint</label>
        <input
          id="provider-endpoint"
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="https://api.example.com/v1"
          value={newProvider.endpoint}
          oninput={(e) => { newProvider.endpoint = (e.target as HTMLInputElement).value; formErrors = { ...formErrors, endpoint: '' }; }}
        />
        {#if formErrors.endpoint}
          <p class="text-[10px] text-destructive mt-0.5">{formErrors.endpoint}</p>
        {/if}
      </div>
    {/snippet}

    {#snippet footer()}
      <button
        type="button"
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => handleDialogOpenChange(false)}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-3 py-1.5 text-xs text-background hover:bg-foreground/90 transition-colors"
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
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => { showDeleteConfirm = false; pendingActionProviderId = ''; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-destructive px-3 py-1.5 text-xs text-white hover:bg-destructive/90 transition-colors"
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
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => { showResetConfirm = false; pendingActionProviderId = ''; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-3 py-1.5 text-xs text-background hover:bg-foreground/90 transition-colors"
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
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => { showRemoveModelConfirm = false; pendingRemoveModel = { providerId: '', modelName: '' }; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-destructive px-3 py-1.5 text-xs text-white hover:bg-destructive/90 transition-colors"
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
        <label for="model-name" class="block text-[11px] text-foreground-alt mb-1">模型名称</label>
        <input
          id="model-name"
          class="flex h-8 w-full rounded-md border border-border-input bg-background px-3 py-1 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-foreground/20 focus-visible:ring-offset-1"
          type="text"
          placeholder="如：gpt-4o"
          bind:value={newModelName}
        />
      </div>
      <div>
        <span class="block text-[11px] text-foreground-alt mb-1">能力</span>
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
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => { showAddModelDialog = false; addModelProviderId = ''; newModelName = ''; newModelCapabilities = []; }}
      >
        取消
      </button>
      <button
        type="button"
        class="rounded-md bg-foreground px-3 py-1.5 text-xs text-background hover:bg-foreground/90 transition-colors"
        onclick={confirmAddModel}
        disabled={!newModelName.trim()}
      >
        添加
      </button>
    {/snippet}
  </Dialog>
</div>
