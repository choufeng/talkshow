import { describe, it, expect } from 'vitest';
import { updateFeature, updateNestedPath } from './config';
import type { AppConfig } from '$lib/stores/config';

function createTestConfig(): AppConfig {
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

describe('updateFeature', () => {
  it('updates a nested feature without mutating original', () => {
    const config = createTestConfig();
    const updated = updateFeature(config, 'recording', (r) => ({
      ...r,
      auto_mute: true,
    }));
    expect(updated.features.recording.auto_mute).toBe(true);
    expect(config.features.recording.auto_mute).toBe(false);
  });

  it('preserves other features', () => {
    const config = createTestConfig();
    const updated = updateFeature(config, 'translation', (t) => ({
      ...t,
      target_lang: '中文',
    }));
    expect(updated.features.translation.target_lang).toBe('中文');
    expect(updated.features.recording.auto_mute).toBe(false);
    expect(updated.features.skills.enabled).toBe(true);
  });
});

describe('updateNestedPath', () => {
  it('updates a shallow path', () => {
    const obj = { a: 1, b: 2 };
    const updated = updateNestedPath(obj, ['a'], () => 10);
    expect(updated.a).toBe(10);
    expect(updated.b).toBe(2);
    expect(obj.a).toBe(1);
  });

  it('updates a deep path', () => {
    const obj = { a: { b: { c: 'old' } } };
    const updated = updateNestedPath(obj, ['a', 'b', 'c'], () => 'new');
    expect(updated.a.b.c).toBe('new');
    expect(obj.a.b.c).toBe('old');
  });

  it('preserves sibling keys at each level', () => {
    const obj = { a: { b: 1, c: 2 }, x: 3 };
    const updated = updateNestedPath(obj, ['a', 'b'], () => 10);
    expect(updated.a.b).toBe(10);
    expect(updated.a.c).toBe(2);
    expect(updated.x).toBe(3);
  });
});

describe('updateNestedPath edge cases', () => {
  it('handles empty path gracefully', () => {
    const obj = { a: 1 };
    const updated = updateNestedPath(obj, [], () => 99);
    expect(updated).toBe(obj);
  });
});
