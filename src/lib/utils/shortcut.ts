const isMac =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent);

export const MODIFIER_DISPLAY: Record<string, string> = isMac
  ? { Control: '⌃', Shift: '⇧', Alt: '⌥', Command: '⌘', Super: '⌘' }
  : { Control: 'Ctrl', Shift: 'Shift', Alt: 'Alt', Command: 'Cmd', Super: 'Cmd' };

export const KEY_DISPLAY: Record<string, string> = {
  Quote: "'",
  Backslash: '\\',
  Space: 'Space',
};

export function parseKeys(shortcut: string): string[] {
  return shortcut.split('+').map((key) => {
    if (MODIFIER_DISPLAY[key]) return MODIFIER_DISPLAY[key];
    if (KEY_DISPLAY[key]) return KEY_DISPLAY[key];
    return key.replace('Key', '').replace('Digit', '');
  });
}
