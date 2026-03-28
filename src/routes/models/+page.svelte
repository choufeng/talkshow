<script lang="ts">
  import { onMount } from 'svelte';
  import { config } from '$lib/stores/config';
  import GroupedSelect from '$lib/components/GroupedSelect.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import PasswordInput from '$lib/components/PasswordInput.svelte';
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

<div class="models-page">
  <h2>模型</h2>

  <section class="section">
    <div class="section-label">Features</div>
    <div class="features-grid">
      <div class="feature-card">
        <div class="feature-name">Transcription</div>
        <div class="feature-desc">转写服务</div>
        <GroupedSelect
          value={getTranscriptionValue()}
          groups={buildTranscriptionGroups()}
          placeholder="选择模型"
          onChange={handleTranscriptionChange}
        />
      </div>
    </div>
  </section>

  <section class="section">
    <div class="section-label">Providers</div>
    <div class="providers-grid">
      {#each $config.ai.providers || [] as provider (provider.id)}
        <div class="provider-card">
          <div class="provider-header">
            <div>
              <div class="provider-name">{provider.name}</div>
              <div class="provider-subtitle">{provider.id}</div>
            </div>
            <button class="provider-remove" onclick={() => handleRemoveProvider(provider.id)}>✕</button>
          </div>

          {#if needsApiKey(provider)}
            <div class="field-group">
              <label class="field-label">API Key</label>
              <PasswordInput
                value={provider.api_key || ''}
                placeholder="sk-..."
                onChange={(val: string) => handleApiKeyChange(provider.id, val)}
              />
            </div>
          {/if}

          <div class="field-group">
            <label class="field-label">Endpoint</label>
            <input
              class="field"
              type="text"
              value={provider.endpoint}
              onchange={(e) => handleProviderFieldChange(provider.id, 'endpoint', (e.target as HTMLInputElement).value)}
            />
          </div>

          <div class="field-group">
            <label class="field-label">Models</label>
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

<style>
  .models-page {
    max-width: 800px;
  }

  h2 {
    margin: 0 0 24px 0;
    font-size: 20px;
    font-weight: 600;
    color: #333;
  }

  .section {
    margin-bottom: 28px;
  }

  .section-label {
    font-size: 11px;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 10px;
  }

  .features-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .feature-card {
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 14px;
  }

  .feature-name {
    font-size: 13px;
    font-weight: 600;
    color: #333;
    margin-bottom: 2px;
  }

  .feature-desc {
    font-size: 11px;
    color: #999;
    margin-bottom: 10px;
  }

  .providers-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .provider-card {
    background: #fff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 14px;
  }

  .provider-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .provider-name {
    font-size: 14px;
    font-weight: 600;
    color: #333;
  }

  .provider-subtitle {
    font-size: 10px;
    color: #aaa;
    margin-top: 1px;
  }

  .provider-remove {
    background: none;
    border: none;
    font-size: 10px;
    color: #ccc;
    cursor: pointer;
    padding: 2px;
  }

  .provider-remove:hover {
    color: #d32f2f;
  }

  .field-group {
    margin-bottom: 10px;
  }

  .field-label {
    display: block;
    font-size: 11px;
    color: #888;
    margin-bottom: 4px;
  }

  .field {
    width: 100%;
    background: #f8f8f8;
    border: 1px solid #ddd;
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 11px;
    outline: none;
    word-break: break-all;
    box-sizing: border-box;
  }

  .field:focus {
    border-color: #396cd8;
    box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.15);
  }
</style>
