import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface VertexConfig {
  endpoint: string;
  models: string[];
}

export interface DashScopeConfig {
  api_key: string;
  endpoint: string;
  models: string[];
}

export interface AiConfig {
  vertex: VertexConfig;
  dashscope: DashScopeConfig;
}

export interface TranscriptionConfig {
  provider: string;
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
      vertex: {
        endpoint: 'https://aiplatform.googleapis.com/v1',
        models: ['gemini-2.0-flash']
      },
      dashscope: {
        api_key: '',
        endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
        models: ['qwen2-audio-instruct']
      }
    },
    features: {
      transcription: {
        provider: 'vertex',
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
    }
  };
}

export const config = createConfigStore();