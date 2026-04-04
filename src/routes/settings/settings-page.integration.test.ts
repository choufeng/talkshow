import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '../../test/mocks/tauri';
import SettingsPage from './+page.svelte';
import { config } from '$lib/stores/config';
import type { AppConfig } from '$lib/stores/config';

function defaultConfig(): AppConfig {
  return {
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: { providers: [] },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.0-flash',
        polish_enabled: true,
        polish_provider_id: '',
        polish_model: '',
      },
      translation: { target_lang: 'English' },
      skills: { enabled: true, skills: [] },
      recording: { auto_mute: false },
    },
  };
}

beforeEach(() => {
  resetTauriMocks();
});

describe('Settings Page Integration', () => {
  it('should display settings title and sections', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      return null;
    });

    render(SettingsPage);

    expect(screen.getByText('设置')).toBeInTheDocument();
    expect(screen.getByText('快捷键')).toBeInTheDocument();
    expect(screen.getByText('录音')).toBeInTheDocument();
    expect(screen.getByText('外观')).toBeInTheDocument();
  });

  it('should display shortcut labels', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      return null;
    });

    render(SettingsPage);

    expect(screen.getByText('窗口切换')).toBeInTheDocument();
    expect(screen.getByText('录音控制')).toBeInTheDocument();
    expect(screen.getByText('AI 翻译')).toBeInTheDocument();
  });

  it('should display auto-mute toggle', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      return null;
    });

    render(SettingsPage);

    expect(screen.getByText('录音时自动静音')).toBeInTheDocument();
    expect(screen.getByText('开始录音后自动静音其他应用的声音，录音结束后自动恢复')).toBeInTheDocument();
  });

  it('should display theme options', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      return null;
    });

    render(SettingsPage);

    expect(screen.getByText('浅色')).toBeInTheDocument();
    expect(screen.getByText('深色')).toBeInTheDocument();
    expect(screen.getByText('跟随系统')).toBeInTheDocument();
  });

  it('should call invoke with get_config on mount', async () => {
    const invokeSpy = vi.fn(async (command: string) => {
      if (command === 'get_config') return defaultConfig();
      return null;
    });
    mockTauriInvoke(invokeSpy);

    render(SettingsPage);

    expect(invokeSpy).toHaveBeenCalledWith('get_config', undefined);
  });

  it('should handle config load error gracefully', async () => {
    mockTauriInvoke(async () => {
      throw new Error('Failed to load config');
    });

    expect(() => render(SettingsPage)).not.toThrow();
  });

  it('should call update_shortcut when shortcut is updated', async () => {
    const invokeSpy = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'update_shortcut') return;
      return null;
    });
    mockTauriInvoke(invokeSpy);

    render(SettingsPage);

    await config.updateShortcut('toggle', 'Control+Shift+KeyA');

    expect(invokeSpy).toHaveBeenCalledWith(
      'update_shortcut',
      { shortcutType: 'toggle', shortcut: 'Control+Shift+KeyA' }
    );
  });

  it('should call save_config_cmd when auto-mute is toggled', async () => {
    const invokeSpy = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'save_config_cmd') return;
      return null;
    });
    mockTauriInvoke(invokeSpy);

    render(SettingsPage);

    const toggle = screen.getByRole('switch', { name: '录音时自动静音' });
    await fireEvent.click(toggle);

    expect(invokeSpy).toHaveBeenCalledWith('save_config_cmd', expect.objectContaining({
      config: expect.objectContaining({
        features: expect.objectContaining({
          recording: expect.objectContaining({ auto_mute: true })
        })
      })
    }));
  });
});
