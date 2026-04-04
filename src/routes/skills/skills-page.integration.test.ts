import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '../../test/mocks/tauri';
import SkillsPage from './+page.svelte';

function configWithSkills() {
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
          { id: 'custom-1', name: '自定义翻译', description: '翻译优化', prompt: '翻译并优化', builtin: false, editable: true, enabled: true },
        ],
      },
      recording: { auto_mute: false },
    },
  };
}

function configWithoutSkills() {
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

describe('Skills Page Integration', () => {
  it('should display page title', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return configWithSkills();
      return null;
    });

    render(SkillsPage);

    expect(screen.getByText('技能设置')).toBeInTheDocument();
  });

  it('should load and display skills list', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return configWithSkills();
      return null;
    });

    render(SkillsPage);

    await waitFor(() => {
      expect(screen.getByText('语气词剔除')).toBeInTheDocument();
      expect(screen.getByText('自定义翻译')).toBeInTheDocument();
    });
  });

  it('should display builtin and custom badges', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return configWithSkills();
      return null;
    });

    render(SkillsPage);

    await waitFor(() => {
      expect(screen.getByText('预置')).toBeInTheDocument();
      expect(screen.getByText('自定义')).toBeInTheDocument();
    });
  });

  it('should show empty state when no skills', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return configWithoutSkills();
      return null;
    });

    render(SkillsPage);

    await waitFor(() => {
      expect(screen.getByText('暂无技能，点击上方按钮添加')).toBeInTheDocument();
    });
  });

  it('should display add skill button', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return configWithSkills();
      return null;
    });

    render(SkillsPage);

    expect(screen.getByText('添加自定义 Skill')).toBeInTheDocument();
  });

  it('should show edit button for all skills and delete only for custom', async () => {
    mockTauriInvoke(async (command) => {
      if (command === 'get_config') return configWithSkills();
      return null;
    });

    render(SkillsPage);

    await waitFor(() => {
      const editButtons = screen.getAllByTitle('编辑');
      expect(editButtons.length).toBe(2);

      const deleteButtons = screen.getAllByTitle('删除');
      expect(deleteButtons.length).toBe(1);
    });
  });

  it('should handle config load error gracefully', async () => {
    mockTauriInvoke(async () => {
      throw new Error('Failed to load config');
    });

    expect(() => render(SkillsPage)).not.toThrow();
  });

  it('should toggle skill enabled state', async () => {
    mockTauriInvoke(async (command: string) => {
      if (command === 'get_config') return configWithSkills();
      return null;
    });

    render(SkillsPage);

    await waitFor(() => {
      const toggle = screen.getByRole('switch', { name: '启用 语气词剔除' });
      expect(toggle).toBeChecked();
    });
  });
});
