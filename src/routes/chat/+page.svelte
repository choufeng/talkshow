<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Mic, Send, Square } from 'lucide-svelte';
  import { chatOpen, type ChatOpenPayload } from '$lib/stores/chat';

  type Message = { role: 'user' | 'assistant'; content: string };

  let messages = $state<Message[]>([]);
  let input = $state('');
  let sending = $state(false);
  let recording = $state(false);
  let messagesEl = $state<HTMLDivElement | null>(null);

  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
    });
  }

  async function send(text: string) {
    const content = text.trim();
    if (!content || sending) return;
    messages.push({ role: 'user', content });
    input = '';
    sending = true;
    scrollToBottom();
    try {
      const reply = await invoke<string>('chat_send', { messages });
      messages.push({ role: 'assistant', content: reply });
    } catch (e) {
      messages.push({ role: 'assistant', content: `出错了：${e}` });
    } finally {
      sending = false;
      scrollToBottom();
    }
  }

  async function toggleVoice() {
    if (recording) {
      recording = false;
      try {
        const text = await invoke<string>('chat_record_stop');
        if (text.trim()) input = text.trim();
      } catch (e) {
        console.error('chat_record_stop failed:', e);
      }
    } else {
      try {
        await invoke('chat_record_start');
        recording = true;
      } catch (e) {
        console.error('chat_record_start failed:', e);
      }
    }
  }

  // 语义路由 ask 命中:layout 写入 store 并导航到此,消费后清空
  async function applyOpen(p: ChatOpenPayload) {
    messages = [{ role: 'user', content: p.question }];
    scrollToBottom();
    if (p.answer) {
      messages.push({ role: 'assistant', content: p.answer });
      scrollToBottom();
    } else {
      await send(p.question);
    }
  }

  $effect(() => {
    const p = $chatOpen;
    if (p) {
      chatOpen.set(null);
      applyOpen(p);
    }
  });
</script>

<div class="h-full flex flex-col">
  <header class="flex items-baseline justify-between mb-4 shrink-0">
    <h2 class="text-title font-semibold text-foreground m-0">对话</h2>
    {#if messages.length > 0}
      <button
        class="text-body text-muted-foreground hover:text-foreground transition-colors"
        onclick={() => (messages = [])}
      >
        新对话
      </button>
    {/if}
  </header>

  <div class="flex-1 min-h-0 overflow-y-auto rounded-xl border border-border bg-background-alt p-4 flex flex-col gap-2.5" bind:this={messagesEl}>
    {#if messages.length === 0}
      <div class="flex-1 flex flex-col items-center justify-center gap-2 text-muted-foreground">
        <p class="m-0 text-body">对麦克风说「问一下 …」开始对话</p>
        <p class="m-0 text-caption">也可以直接打字或用下方麦克风语音输入</p>
      </div>
    {:else}
      {#each messages as m}
        <div class="flex max-w-[80%] {m.role === 'user' ? 'self-end' : 'self-start'}">
          <div
            class="px-3 py-2 rounded-xl text-body whitespace-pre-wrap break-words {m.role === 'user'
              ? 'bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white rounded-br-sm'
              : 'bg-muted text-foreground rounded-bl-sm'}"
          >{m.content}</div>
        </div>
      {/each}
      {#if sending}
        <div class="flex self-start">
          <div class="px-3 py-2 rounded-xl rounded-bl-sm bg-muted text-muted-foreground text-body italic">思考中…</div>
        </div>
      {/if}
    {/if}
  </div>

  <footer class="flex gap-2 items-end mt-3 shrink-0">
    <textarea
      bind:value={input}
      placeholder={recording ? '录音中，点击麦克风结束…' : '输入或按麦克风说话，Enter 发送'}
      rows="2"
      class="flex-1 resize-none rounded-lg border border-border bg-background-alt px-3 py-2 text-body text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-accent-foreground/40"
      onkeydown={(e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          send(input);
        }
      }}
    ></textarea>
    <button
      class="flex items-center justify-center w-9 h-9 rounded-lg border border-border bg-background-alt text-foreground transition-colors {recording
        ? 'text-red-500 border-red-500/40 animate-pulse'
        : 'hover:bg-muted/50'}"
      onclick={toggleVoice}
      aria-label={recording ? '停止录音' : '语音输入'}
      title={recording ? '停止录音' : '语音输入'}
    >
      {#if recording}
        <Square size={15} />
      {:else}
        <Mic size={15} />
      {/if}
    </button>
    <button
      class="flex items-center justify-center w-9 h-9 rounded-lg bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white transition-opacity disabled:opacity-40"
      onclick={() => send(input)}
      disabled={sending || !input.trim()}
      aria-label="发送"
    >
      <Send size={15} />
    </button>
  </footer>
</div>
