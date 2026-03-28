<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/ui/select/index.svelte';
  import TagInput from '$lib/components/ui/tag-input/index.svelte';
  import PasswordInput from '$lib/components/ui/password-input/index.svelte';
  import type { ProviderConfig, AppConfig } from '$lib/stores/config';

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
</script>

<div class="max-w-[800px]">
  <h2 class="text-xl font-semibold text-foreground m-0 mb-6">模型</h2>

  <section class="mb-7">
    <div class="text-[11px] text-muted-foreground uppercase tracking-wider mb-2.5">Features</div>
    <div class="grid grid-cols-3 gap-3">
      <div class="rounded-lg border border-border bg-background p-3.5">
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
        <div class="rounded-lg border border-border bg-background p-3.5">
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
    </div>
  </section>
</div>
