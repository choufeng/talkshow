<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { Mic, Send, Square, X } from 'lucide-svelte';

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
        // 静默失败:录音 UI 复位即可,不打断对话
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

  async function hide() {
    if (recording) {
      recording = false;
      try {
        await invoke('chat_record_stop');
      } catch {
        // 忽略:窗体即将隐藏,录音残留由下次 start 重置
      }
    }
    invoke('chat_hide');
  }

  async function applyOpen(question: string, answer: string) {
    messages = [{ role: 'user', content: question }];
    scrollToBottom();
    if (answer) {
      messages.push({ role: 'assistant', content: answer });
      scrollToBottom();
    } else {
      // 合并调用未产出回答 → 前端兜底补一轮
      await send(question);
    }
  }

  $effect(() => {
    const unsubs: Array<() => void> = [];
    (async () => {
      // 双通道之一:窗口新建后页面未就绪错过事件,mount 主动拉取
      const pending = await invoke<{ question: string; answer: string } | null>('chat_take_latest');
      if (pending) await applyOpen(pending.question, pending.answer);
      unsubs.push(
        await listen<{ question: string; answer: string }>('chat:open', (e) =>
          applyOpen(e.payload.question, e.payload.answer)
        )
      );
    })();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') hide();
    };
    window.addEventListener('keydown', onKey);
    unsubs.push(() => window.removeEventListener('keydown', onKey));
    return () => unsubs.forEach((fn) => fn());
  });
</script>

<svelte:head>
  <title>对话</title>
</svelte:head>

<div class="chat-window">
  <header class="titlebar" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>TalkShow 对话</span>
    <button class="close-btn" onclick={hide} aria-label="关闭">
      <X size={14} />
    </button>
  </header>

  <div class="messages" bind:this={messagesEl}>
    {#each messages as m, i}
      <div class="msg {m.role}">
        <div class="bubble">{m.content}</div>
      </div>
    {/each}
    {#if sending}
      <div class="msg assistant">
        <div class="bubble typing">思考中…</div>
      </div>
    {/if}
  </div>

  <footer class="input-area">
    <textarea
      bind:value={input}
      placeholder={recording ? '录音中，点击麦克风结束…' : '输入或按麦克风说话'}
      rows="2"
      onkeydown={(e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          send(input);
        }
      }}
    ></textarea>
    <div class="actions">
      <button
        class="icon-btn {recording ? 'recording' : ''}"
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
        class="icon-btn send"
        onclick={() => send(input)}
        disabled={sending || !input.trim()}
        aria-label="发送"
      >
        <Send size={15} />
      </button>
    </div>
  </footer>
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    overflow: hidden;
    height: 100%;
  }

  .chat-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    border-radius: 12px;
    background: rgba(28, 28, 30, 0.97);
    backdrop-filter: blur(20px);
    color: #e8e8ea;
    font-family: -apple-system, BlinkMacSystemFont, 'PingFang SC', sans-serif;
    overflow: hidden;
  }

  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 8px;
    flex-shrink: 0;
    -webkit-user-select: none;
    user-select: none;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    color: #9a9aa2;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    color: #8b8b93;
    cursor: pointer;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: rgba(255, 95, 87, 0.22);
    color: #ff5f57;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 4px 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .msg {
    display: flex;
    max-width: 85%;
  }

  .msg.user {
    align-self: flex-end;
  }

  .msg.assistant {
    align-self: flex-start;
  }

  .bubble {
    padding: 8px 12px;
    border-radius: 14px;
    font-size: 14px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .msg.user .bubble {
    background: linear-gradient(to bottom, #3b82f6, #2563eb);
    color: #fff;
    border-bottom-right-radius: 4px;
  }

  .msg.assistant .bubble {
    background: rgba(255, 255, 255, 0.08);
    color: #e8e8ea;
    border-bottom-left-radius: 4px;
  }

  .bubble.typing {
    color: #8b8b93;
    font-style: italic;
  }

  .input-area {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    padding: 10px 12px 12px;
    border-top: 0.5px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }

  textarea {
    flex: 1;
    resize: none;
    border: none;
    outline: none;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.07);
    color: #e8e8ea;
    font-size: 14px;
    font-family: inherit;
    line-height: 1.5;
    padding: 8px 12px;
  }

  textarea::placeholder {
    color: #6b6b73;
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border: none;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.07);
    color: #b0b0b8;
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
  }

  .icon-btn.recording {
    background: rgba(255, 0, 85, 0.2);
    color: #ff0055;
    animation: pulse 1.2s ease-in-out infinite;
  }

  .icon-btn.send {
    background: linear-gradient(to bottom, #3b82f6, #2563eb);
    color: #fff;
  }

  .icon-btn.send:disabled {
    opacity: 0.4;
    cursor: default;
  }

  @keyframes pulse {
    0%,
    100% {
      box-shadow: 0 0 0 0 rgba(255, 0, 85, 0.4);
    }
    50% {
      box-shadow: 0 0 0 6px rgba(255, 0, 85, 0);
    }
  }
</style>
