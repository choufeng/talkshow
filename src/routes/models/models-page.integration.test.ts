import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '../../test/mocks/tauri';
import ModelsPage from './+page.svelte';

function defaultConfig() {
  return {
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: {
      providers: [
        {
          id: 'vertex',
          type: 'vertex',
          name: 'Vertex AI',
          endpoint: '',
          api_key: null,
          models: [{ name: 'gemini-2.0-flash', capabilities: ['transcription', 'chat'] }],
        },
        {
          id: 'dashscope',
          type: 'openai-compatible',
          name: '阿里云',
          endpoint: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
          api_key: 'sk-xxx',
          models: [{ name: 'qwen2-audio-instruct', capabilities: ['transcription'] }],
        },
      ],
    },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.0-flash',
        polish_enabled: true,
        polish_provider_id: 'dashscope',
        polish_model: 'qwen-plus',
      },
      translation: { target_lang: 'English' },
      skills: { enabled: true, skills: [] },
      recording: { auto_mute: false },
    },
  };
}

function emptyConfig() {
  return {
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: { providers: [] },
    features: {
      transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
      translation: { target_lang: 'English' },
      skills: { enabled: false, skills: [] },
      recording: { auto_mute: false },
    },
  };
}

beforeEach(() => {
  resetTauriMocks();
});

describe('Models Page Integration', () => {
  it('should display page title', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    expect(screen.getByText('模型')).toBeInTheDocument();
  });

  it('should load and display provider names', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('Vertex AI')).toBeInTheDocument();
      expect(screen.getByText('阿里云')).toBeInTheDocument();
    });
  });

  it('should display AI service section with transcription and translation', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('AI 转写')).toBeInTheDocument();
      expect(screen.getByText('AI 翻译')).toBeInTheDocument();
      expect(screen.getByText('启用润色')).toBeInTheDocument();
      expect(screen.getByText('目标语言')).toBeInTheDocument();
    });
  });

  it('should display add provider button', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('添加 Provider')).toBeInTheDocument();
    });
  });

  it('should display model badges with capabilities', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('gemini-2.0-flash')).toBeInTheDocument();
      expect(screen.getByText('qwen2-audio-instruct')).toBeInTheDocument();
    });
  });

  it('should show polish model when polish is enabled', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return defaultConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('润色模型')).toBeInTheDocument();
    });
  });

  it('should handle config load error gracefully', async () => {
    mockTauriInvoke(async () => {
      throw new Error('Failed to load config');
    });

    // Should not crash
    expect(() => render(ModelsPage)).not.toThrow();
  });

  it('should display empty state when no providers', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return emptyConfig();
      if (command === 'get_vertex_env_info') return { project: '', location: '' };
      if (command === 'get_sensevoice_status') return { status: 'not_downloaded' };
      return null;
    });

    render(ModelsPage);

    await waitFor(() => {
      expect(screen.getByText('添加 Provider')).toBeInTheDocument();
    });
  });
});
