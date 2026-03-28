import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
}

function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash'
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