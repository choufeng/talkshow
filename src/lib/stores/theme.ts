import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'light' | 'dark' | 'system';

function getSystemTheme(): 'light' | 'dark' {
  if (!browser) return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyTheme(theme: Theme) {
  if (!browser) return;
  const resolved = theme === 'system' ? getSystemTheme() : theme;
  document.documentElement.classList.toggle('dark', resolved === 'dark');
}

function createThemeStore() {
  const initial: Theme = browser
    ? (localStorage.getItem('theme') as Theme) || 'system'
    : 'system';

  applyTheme(initial);

  const { subscribe, set } = writable<Theme>(initial);

  return {
    subscribe,
    set: (theme: Theme) => {
      if (browser) {
        localStorage.setItem('theme', theme);
      }
      applyTheme(theme);
      set(theme);
    },
    getResolved: (): 'light' | 'dark' => {
      const stored: Theme = browser
        ? (localStorage.getItem('theme') as Theme) || 'system'
        : 'system';
      return stored === 'system' ? getSystemTheme() : stored;
    }
  };
}

export const theme = createThemeStore();

if (browser) {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    const current: Theme = (localStorage.getItem('theme') as Theme) || 'system';
    if (current === 'system') {
      applyTheme('system');
    }
  });
}
