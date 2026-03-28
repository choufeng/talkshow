import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface ProviderConfig {
  id: string;
  type: string;
  name: string;
  endpoint: string;
  api_key?: string;
  models: string[];
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

function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    ai: {
      providers: [
        {
          id: 'vertex',
          type: 'vertex',
          name: 'VTX',
          endpoint: 'https://aiplatform.googleapis.com/v1',
          models: ['gemini-2.0-flash']
        },
        {
          id: 'dashscope',
          type: 'openai-compatible',
          name: '阿里云',
          endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
          api_key: '',
          models: ['qwen2-audio-instruct']
        }
      ]
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
