import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { mockTauriInvoke, resetTauriMocks } from '../../test/mocks/tauri';
import ReviewPage from './+page.svelte';

function sampleRequest(overrides: Record<string, unknown> = {}) {
  return {
    id: 7,
    raw_text: '记录 明天下午三点开会',
    text: '明天下午三点开会',
    original_text: null,
    target_lang: null,
    action: 'hook',
    target_app: 'Safari',
    hook: {
      url: 'https://api.dida365.com/open/v1/task',
      body: '{"title":"开会","content":"记录 明天下午三点开会"}',
      vars: { title: '开会' }
    },
    can_hook: true,
    audio_path: '/tmp/a.wav',
    ...overrides
  };
}

describe('review 浮窗页', () => {
  beforeEach(() => {
    resetTauriMocks();
  });

  it('mount 拉取当前请求并展示', async () => {
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get') return sampleRequest();
      return null;
    });
    render(ReviewPage);
    await waitFor(() => {
      expect(screen.getByDisplayValue('明天下午三点开会')).toBeTruthy();
    });
    expect(screen.getByText(/dida365/)).toBeTruthy();
    expect(screen.getByText('title: 开会')).toBeTruthy();
  });

  it('确认时提交编辑后的文本与所选去向', async () => {
    const confirmSpy = vi.fn(async (_args?: Record<string, unknown>) => {});
    mockTauriInvoke(async (cmd, args) => {
      if (cmd === 'review_get') return sampleRequest();
      if (cmd === 'review_confirm') {
        confirmSpy(args);
        return null;
      }
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));

    // 切到翻译去向
    await fireEvent.click(screen.getByTitle('翻译为目标语言后粘贴'));
    const ta = screen.getByRole('textbox') as HTMLTextAreaElement;
    await fireEvent.input(ta, { target: { value: '改后的内容' } });
    await fireEvent.click(screen.getByText('翻译为English并粘贴'));

    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));
    expect(confirmSpy.mock.calls[0]?.[0]?.confirm).toEqual({
      id: 7,
      action: 'translate',
      text: '改后的内容',
      target_lang: 'English'
    });
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

  it('hook 未配置时按钮禁用', async () => {
    mockTauriInvoke(async (cmd) => {
      if (cmd === 'review_get')
        return sampleRequest({ action: 'paste', hook: null, can_hook: false });
      return null;
    });
    render(ReviewPage);
    await waitFor(() => screen.getByDisplayValue('明天下午三点开会'));
    expect((screen.getByTitle('整理后发送到后端') as HTMLButtonElement).disabled).toBe(true);
  });
});
