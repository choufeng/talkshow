import { vi } from 'vitest';

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
