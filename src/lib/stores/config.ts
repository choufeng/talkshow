import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface ModelVerified {
  status: 'ok' | 'error';
  tested_at: string;
  latency_ms?: number;
  message?: string;
}

export interface ModelConfig {
  name: string;
  capabilities: string[];
  verified?: ModelVerified;
}

export interface ProviderConfig {
  id: string;
  type: string;
  name: string;
  endpoint: string;
  api_key?: string;
  models: ModelConfig[];
}

export interface AiConfig {
  providers: ProviderConfig[];
}

export interface TranscriptionConfig {
  provider_id: string;
  model: string;
}

export interface FeaturesConfig {
  transcription: TranscriptionConfig;
}

export interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
  ai: AiConfig;
  features: FeaturesConfig;
}

export const MODEL_CAPABILITIES = [
  { value: 'transcription', label: '语音转文字' }
];

export const BUILTIN_PROVIDERS: ProviderConfig[] = [
  {
    id: 'vertex',
    type: 'vertex',
    name: 'Vertex AI',
    endpoint: '',
    models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription'] }]
  },
  {
    id: 'dashscope',
    type: 'openai-compatible',
    name: '阿里云',
    endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    api_key: '',
    models: [{ name: 'qwen2-audio-instruct', capabilities: ['transcription'] }]
  }
];

export function isBuiltinProvider(id: string): boolean {
  return BUILTIN_PROVIDERS.some((p) => p.id === id);
}

function getBuiltinProvider(id: string): ProviderConfig | undefined {
  return BUILTIN_PROVIDERS.find((p) => p.id === id);
}

function migrateModels(providers: ProviderConfig[]): ProviderConfig[] {
  return providers.map((p) => ({
    ...p,
    models: (p.models || []).map((m) =>
      typeof m === 'string' ? { name: m, capabilities: [] as string[] } : m
    )
  }));
}

function mergeBuiltinProviders(providers: ProviderConfig[]): ProviderConfig[] {
  const userIds = new Set(providers.map((p) => p.id));
  const missing = BUILTIN_PROVIDERS.filter((p) => !userIds.has(p.id));
  const corrected = providers.map((p) => {
    const builtin = BUILTIN_PROVIDERS.find((b) => b.id === p.id);
    if (builtin) {
      return { ...p, type: builtin.type, endpoint: builtin.endpoint };
    }
    return p;
  });
  return [...missing, ...corrected];
}

function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    ai: {
      providers: BUILTIN_PROVIDERS.map((p) => ({ ...p }))
    },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.0-flash'
      }
    }
  });

  return {
    subscribe,
    load: async () => {
      try {
        const config = await invoke<AppConfig>('get_config');
        config.ai.providers = mergeBuiltinProviders(
          migrateModels(config.ai.providers || [])
        );
        set(config);
      } catch (error) {
        console.error('Failed to load config:', error);
      }
    },
    updateShortcut: async (type: 'toggle' | 'recording', shortcut: string) => {
      try {
        await invoke('update_shortcut', { shortcutType: type, shortcut });
        update(config => {
          if (type === 'toggle') {
            return { ...config, shortcut };
          } else {
            return { ...config, recording_shortcut: shortcut };
          }
        });
      } catch (error) {
        console.error('Failed to update shortcut:', error);
        throw error;
      }
    },
    save: async (newConfig: AppConfig) => {
      try {
        await invoke('save_config_cmd', { config: newConfig });
        set(newConfig);
      } catch (error) {
        console.error('Failed to save config:', error);
        throw error;
      }
    }
  };
}

export const config = createConfigStore();
