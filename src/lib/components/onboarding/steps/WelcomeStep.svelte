<script lang="ts">
  import { onboarding } from '$lib/stores/onboarding';
  import { onMount } from 'svelte';

  onMount(() => {
    onboarding.setStepValid(1, true);
  });
</script>

<div class="text-center">
  <img src="/logo.svg" alt="TalkShow" class="w-16 h-16 mx-auto mb-5" />
  <h1 class="text-title font-semibold text-foreground mb-2">欢迎使用 TalkShow</h1>
  <p class="text-body text-foreground-alt">
    按快捷键，语音即转文字。让我们一起完成初始配置。
  </p>
</div>
