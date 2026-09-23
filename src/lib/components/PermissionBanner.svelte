<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ShieldAlert, LoaderCircle, ShieldX } from 'lucide-svelte';

  // 状态机:
  // checking → ok(隐藏)| denied(权限失效,引导重置)
  // denied → authorizing(已 reset + 打开设置,轮询实测)
  // authorizing → ok | automation-denied(AXI ✓ 但自动化被拒)
  type State = 'checking' | 'ok' | 'denied' | 'authorizing' | 'automation-denied';
  let state: State = $state('checking');

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let recheckTimer: ReturnType<typeof setInterval> | null = null;
  let pollCount = 0;

  async function checkTrusted(): Promise<boolean> {
    return invoke<boolean>('check_accessibility_permission');
  }

  onMount(async () => {
    state = (await checkTrusted()) ? 'ok' : 'denied';
    // 运行期间定期复查:用户中途关闭权限时恢复提示
    recheckTimer = setInterval(async () => {
      if (state === 'ok' && !(await checkTrusted())) state = 'denied';
    }, 30_000);
  });

  onDestroy(() => {
    stopPolling();
    if (recheckTimer) clearInterval(recheckTimer);
  });

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function startAuthorization() {
    // 先清掉伪授权条目(旧签名残留),否则设置里显示已勾选,用户无从下手
    try {
      await invoke('reset_tcc_permissions');
    } catch (e) {
      console.warn('tccutil reset failed:', e);
    }
    await invoke('open_accessibility_settings', { automation: false });
    state = 'authorizing';
    pollCount = 0;
    pollTimer = setInterval(async () => {
      pollCount++;
      if (await checkTrusted()) {
        stopPolling();
        await verifyAutomation();
      } else if (pollCount > 300) {
        // 5 分钟超时,回到初始提示
        stopPolling();
        state = 'denied';
      }
    }, 1000);
  }

  async function verifyAutomation() {
    try {
      await invoke('probe_automation_permission');
      state = 'ok';
    } catch (e) {
      if (e === 'automation-denied') {
        state = 'automation-denied';
      } else {
        // 其他错误(超时等)不阻塞:粘贴时系统会再弹授权
        state = 'ok';
      }
    }
  }

  async function openAutomationSettings() {
    await invoke('open_accessibility_settings', { automation: true });
  }

  async function retryAutomation() {
    await openAutomationSettings();
    state = 'authorizing';
    pollTimer = setInterval(async () => {
      pollCount++;
      try {
        await invoke('probe_automation_permission');
        stopPolling();
        state = 'ok';
      } catch {
        if (pollCount > 120) {
          stopPolling();
          state = 'automation-denied';
        }
      }
    }, 2000);
  }
</script>

{#if state !== 'ok' && state !== 'checking'}
  <div
    class="fixed inset-x-0 top-0 z-50 flex items-center justify-center gap-3 px-4 py-2.5 text-body shadow-lg {state === 'automation-denied'
      ? 'bg-red-500/90 text-white'
      : state === 'authorizing'
        ? 'bg-blue-500/90 text-white'
        : 'bg-amber-500/95 text-amber-950'}"
  >
    {#if state === 'denied'}
      <ShieldAlert size={18} class="shrink-0" />
      <span>粘贴权限已失效(常见于安装新版本后),需重新授权。</span>
      <button
        class="ml-2 rounded-md bg-amber-950 px-3 py-1 font-medium text-amber-50 hover:bg-amber-900"
        onclick={startAuthorization}>重新授权</button
      >
    {:else if state === 'authorizing'}
      <LoaderCircle size={18} class="shrink-0 animate-spin" />
      <span>请在「系统设置 → 隐私与安全性 → 辅助功能」中开启 TalkShow。若列表中没有,点「+」添加本应用。</span>
    {:else}
      <ShieldX size={18} class="shrink-0" />
      <span>还需在「隐私与安全性 → 自动化」中允许 TalkShow 控制 System Events。</span>
      <button
        class="ml-2 rounded-md bg-white/15 px-3 py-1 font-medium text-white hover:bg-white/25"
        onclick={openAutomationSettings}>打开设置</button
      >
      <button
        class="rounded-md bg-white/15 px-3 py-1 font-medium text-white hover:bg-white/25"
        onclick={retryAutomation}>我已允许</button
      >
    {/if}
  </div>
{/if}
