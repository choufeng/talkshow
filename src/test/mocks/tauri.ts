import { vi } from 'vitest';

// Mock window.matchMedia for jsdom environment
if (typeof window !== 'undefined' && !window.matchMedia) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: vi.fn((key: string) => store[key] || null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value.toString();
    }),
    removeItem: vi.fn((key: string) => {
      delete store[key];
    }),
    clear: vi.fn(() => {
      store = {};
    }),
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

export type MockInvokeHandler = (
  command: string,
  args?: Record<string, unknown>
) => unknown | Promise<unknown>;

let invokeHandler: MockInvokeHandler | null = null;

export function mockTauriInvoke(handler: MockInvokeHandler) {
  invokeHandler = handler;
}

export function resetTauriMocks() {
  invokeHandler = null;
  vi.clearAllMocks();
  localStorageMock.clear();
}

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string, args?: Record<string, unknown>) => {
    if (invokeHandler) {
      return invokeHandler(command, args);
    }
    throw new Error(`No mock handler for command: ${command}`);
  }),
}));

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve({ unlisten: vi.fn() })),
  emit: vi.fn(),
}));

// Mock @tauri-apps/plugin-global-shortcut
vi.mock('@tauri-apps/plugin-global-shortcut', () => ({}));

// Mock @tauri-apps/plugin-notification
vi.mock('@tauri-apps/plugin-notification', () => ({}));

// Mock @tauri-apps/plugin-opener
vi.mock('@tauri-apps/plugin-opener', () => ({}));
