import { invoke } from '@tauri-apps/api/core';

export interface InvokeOptions {
  onError?: (error: Error) => void;
}

/**
 * 带统一错误处理的 Tauri invoke 封装
 * 成功时返回值，失败时调用 onError 并返回 null
 */
export async function invokeWithError<T>(
  command: string,
  args?: Record<string, unknown>,
  options: InvokeOptions = {}
): Promise<T | null> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const error = e instanceof Error ? e : new Error(String(e));
    (options.onError ?? console.error)(error);
    return null;
  }
}
