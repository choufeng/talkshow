import { writable } from 'svelte/store';

export interface ChatOpenPayload {
  question: string;
  answer: string;
}

/** 语义路由 ask 命中时由 layout 写入,对话栏目消费后清空 */
export const chatOpen = writable<ChatOpenPayload | null>(null);
