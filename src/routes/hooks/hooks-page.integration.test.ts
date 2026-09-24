import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '../../test/mocks/tauri';
import HooksPage from './+page.svelte';

function baseConfig() {
  return {
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: { providers: [] },
    features: {
      transcription: { provider_id: '', model: '', polish_enabled: false, polish_provider_id: '', polish_model: '' },
      translation: { target_lang: 'English' },
      skills: {
        enabled: true,
        skills: [
          { id: 'builtin-fillers', name: '语气词剔除', description: '去除语气词', prompt: '去除语气词', builtin: true, editable: false, enabled: true },
        ],
      },
      recording: { auto_mute: false },
      glossary: [],
      intent_routing: { enabled: false },
      hooks: { enabled: false, url: '', headers: '', body_template: '', prompt: '', skill_ids: [] },
    },
  };
}

beforeEach(() => {
  resetTauriMocks();
});

describe('Hooks Page Integration', () => {
  it('should display page title', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return baseConfig();
      return null;
    });

    render(HooksPage);

    expect(screen.getByText('Hook 扩展')).toBeInTheDocument();
  });

  it('should show backend config only when enabled', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return baseConfig();
      return null;
    });

    render(HooksPage);

    expect(screen.queryByText('后端配置')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('switch', { name: '启用语音记录' }));

    await waitFor(() => {
      expect(screen.getByText('后端配置')).toBeInTheDocument();
    });
  });

  it('should fill dida365 preset fields', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return baseConfig();
      return null;
    });

    render(HooksPage);

    fireEvent.click(screen.getByRole('switch', { name: '启用语音记录' }));
    fireEvent.click(screen.getByText('填入滴答清单预设'));

    const urlInput = screen.getByLabelText('请求 URL') as HTMLInputElement;
    expect(urlInput.value).toBe('https://api.dida365.com/open/v1/task');

    const headersInput = screen.getByLabelText(/请求头/) as HTMLTextAreaElement;
    expect(headersInput.value).toContain('Authorization: Bearer');

    const templateInput = screen.getByLabelText(/Body 模板/) as HTMLTextAreaElement;
    expect(templateInput.value).toContain('{{title}}');
    expect(templateInput.value).toContain('{{dueDate}}');

    const promptInput = screen.getByLabelText('整理指令（可选，发送前由 LLM 整理）') as HTMLTextAreaElement;
    expect(promptInput.value).toContain('JSON');
  });

  it('should toggle associated skill and save config', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return baseConfig();
      return null;
    });

    render(HooksPage);

    fireEvent.click(screen.getByRole('switch', { name: '启用语音记录' }));
    fireEvent.click(screen.getByText('填入滴答清单预设'));
    fireEvent.click(screen.getByText('语气词剔除'));

    fireEvent.click(screen.getByText('保存'));

    await waitFor(() => {
      expect(screen.getByText('已保存')).toBeInTheDocument();
    });

    const { invoke } = await import('@tauri-apps/api/core');
    expect(vi.mocked(invoke)).toHaveBeenCalledWith(
      'save_config_cmd',
      expect.objectContaining({
        config: expect.objectContaining({
          features: expect.objectContaining({
            hooks: expect.objectContaining({
              enabled: true,
              url: 'https://api.dida365.com/open/v1/task',
              skill_ids: ['builtin-fillers'],
            }),
          }),
        }),
      })
    );
  });
});
