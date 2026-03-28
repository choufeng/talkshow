<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import TagInput from '$lib/components/ui/tag-input/index.svelte';
  import PasswordInput from '$lib/components/ui/password-input/index.svelte';
  import Dialog from '$lib/components/ui/dialog/index.svelte';
  import { Plus } from 'lucide-svelte';
  import type { ProviderConfig, AppConfig } from '$lib/stores/config';

  let showAddDialog = $state(false);
  let newProvider = $state({
    name: '',
    type: '',
    id: '',
    endpoint: ''
  });
  let formErrors = $state<Record<string, string>>({});

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
      items: (p.models || []).map((m: string) => ({
        value: `${p.id}::${m}`,
        label: m
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

  function handleAddModel(providerId: string, model: string) {
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

  function handleRemoveModel(providerId: string, model: string) {
    const newProviders = $config.ai.providers.map((p: ProviderConfig) =>
      p.id === providerId
        ? { ...p, models: p.models.filter((m: string) => m !== model) }
        : p
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
  }

  function handleRemoveProvider(providerId: string) {
    const newProviders = $config.ai.providers.filter(
      (p: ProviderConfig) => p.id !== providerId
    );
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);
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
    const newConfig: AppConfig = {
      ...$config,
      ai: { providers: newProviders }
    };
    config.save(newConfig);

    newProvider = { name: '', type: '', id: '', endpoint: '' };
    formErrors = {};
    showAddDialog = false;
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
            <button
              class="text-xs text-muted-foreground hover:text-destructive transition-colors p-0.5"
              onclick={() => handleRemoveProvider(provider.id)}
            >
              ✕
            </button>
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
            <TagInput
              tags={provider.models}
              onAdd={(tag: string) => handleAddModel(provider.id, tag)}
              onRemove={(tag: string) => handleRemoveModel(provider.id, tag)}
            />
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
        <label class="block text-[11px] text-foreground-alt mb-1">Name</label>
        <input
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
        <label class="block text-[11px] text-foreground-alt mb-1">Type</label>
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
        <label class="block text-[11px] text-foreground-alt mb-1">ID</label>
        <input
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
        <label class="block text-[11px] text-foreground-alt mb-1">Endpoint</label>
        <input
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
        class="rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-muted transition-colors"
        onclick={() => handleDialogOpenChange(false)}
      >
        取消
      </button>
      <button
        class="rounded-md bg-foreground px-3 py-1.5 text-xs text-background hover:bg-foreground/90 transition-colors"
        onclick={handleAddProvider}
      >
        添加
      </button>
    {/snippet}
  </Dialog>
</div>
