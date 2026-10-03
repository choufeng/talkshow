import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '../../test/mocks/tauri';
import ReviewPage from './+page.svelte';

function sampleRequest(overrides: Record<string, unknown> = {}) {
  return {
    id: 7,
    raw_text: '记录 明天下午三点开会',
    text: '明天下午三点开会',
    original_text: null,
    target_lang: null,
    primary: 'hook',
    alternatives: [],
    available: ['paste', 'translate', 'ask', 'hook'],
    target_app: 'Safari',
    hook: {
      url: 'https://api.dida365.com/open/v1/task',
      body: '{"title":"开会","content":"记录 明天下午三点开会"}',
      vars: { title: '开会' }
    },
    audio_path: '/tmp/a.wav',
    ...overrides
  };
}

describe('review 浮窗页', () => {
  beforeEach(() => {
    resetTauriMocks();
  });

  it('mount 拉取当前请求并展示 hook 预览', async () => {
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return sampleRequest();
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      return null;
    });
    render(ReviewPage);
    await waitFor(() => {
      expect(screen.getByDisplayValue('明天下午三点开会')).toBeTruthy();
    });
    expect(screen.getByText(/dida365/)).toBeTruthy();
    expect(screen.getByText('title: 开会')).toBeTruthy();
    // 高置信:主按钮为发送到 Hook,无歧义提示
    expect(screen.getByText('发送到 Hook', { selector: 'button' })).toBeTruthy();
    expect(screen.queryByText(/语义路由不确定/)).toBeNull();
  });

  it('歧义时渲染主次双按钮,次按钮切换后确认提交 hook', async () => {
    const confirmSpy = vi.fn(async (_args?: Record<string, unknown>) => {});
    mockTauriInvoke(async (cmd, args) => {
      if (cmd === 'review_get')
        return sampleRequest({ primary: 'paste', alternatives: ['hook'], hook: null });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      if (cmd === 'review_confirm') return confirmSpy(args);
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    expect(screen.getByText(/语义路由不确定/)).toBeTruthy();
    // 主按钮 paste
    expect(screen.getByRole('button', { name: /粘贴到 Safari/ })).toBeTruthy();
    // 次按钮切换到 hook(主按钮随之变为发送到 Hook,以发送键提示区分)
    await fireEvent.click(screen.getByText('发送到 Hook', { selector: 'button' }));
    await fireEvent.click(screen.getByRole('button', { name: /发送到 Hook ⏎/ }));
    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));
    expect(confirmSpy.mock.calls[0]?.[0]?.confirm).toEqual({
      id: 7,
      action: 'hook',
      text: '明天下午三点开会',
      target_lang: null
    });
  });

  it('Enter 触发发送(enter 配置)', async () => {
    const confirmSpy = vi.fn(async () => {});
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return sampleRequest({ primary: 'paste', hook: null });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      if (cmd === 'review_confirm') return confirmSpy();
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    await fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));
  });

  it('⌘4 切换到 hook(cmd 数字键映射)', async () => {
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return sampleRequest({ primary: 'paste', hook: null });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    await fireEvent.keyDown(window, { key: '4', metaKey: true });
    await waitFor(() => expect(screen.getByRole('button', { name: /发送到 Hook/ })).toBeTruthy());
  });

  it('其他去向面板按 available 禁用', async () => {
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get')
        return sampleRequest({ primary: 'paste', hook: null, available: ['paste', 'translate', 'ask'] });
      if (cmd === 'get_config') return { features: { review: { send_key: 'enter' } } };
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    await fireEvent.click(screen.getByText('其他去向…'));
    const hookBtn = screen.getByRole('button', { name: /发送到 Hook⌘4/ }) as HTMLButtonElement;
    expect(hookBtn.disabled).toBe(true);
  });

  it('无请求时主动 dismiss', async () => {
    const dismissSpy = vi.fn(async () => {});
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return null;
      if (cmd === 'review_dismiss') dismissSpy();
      return null;
    });
    render(ReviewPage);
    await waitFor(() => expect(dismissSpy).toHaveBeenCalledTimes(1));
    expect(screen.getByText('没有待确认的内容')).toBeTruthy();
  });
});
