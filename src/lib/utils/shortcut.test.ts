import { describe, expect, it } from 'vitest';
import { joinShortcuts, parseShortcutList } from './shortcut';

describe('parseShortcutList', () => {
  it('单组无逗号', () => {
    expect(parseShortcutList('Control+Shift+Quote')).toEqual(['Control+Shift+Quote']);
  });

  it('多组逗号分隔', () => {
    expect(parseShortcutList('Control+Shift+Quote,Alt+KeyQ')).toEqual([
      'Control+Shift+Quote',
      'Alt+KeyQ'
    ]);
  });

  it('容忍空格与空段', () => {
    expect(parseShortcutList(' Control+KeyA , , Alt+KeyB ')).toEqual(['Control+KeyA', 'Alt+KeyB']);
  });

  it('空字符串', () => {
    expect(parseShortcutList('')).toEqual([]);
  });
});

describe('joinShortcuts', () => {
  it('往返一致', () => {
    const list = ['Control+KeyA', 'Alt+KeyB'];
    expect(parseShortcutList(joinShortcuts(list))).toEqual(list);
  });
});
