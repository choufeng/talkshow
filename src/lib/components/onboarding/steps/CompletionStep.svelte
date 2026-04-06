<script lang="ts">
  import { onboarding } from '$lib/stores/onboarding';
  import { onMount } from 'svelte';
  import { PartyPopper } from 'lucide-svelte';

  onMount(() => {
    onboarding.setStepValid(7, true);
  });
</script>

<div class="text-center">
  <div class="w-14 h-14 rounded-full bg-accent/50 flex items-center justify-center mx-auto mb-5">
    <PartyPopper size={28} class="text-accent-foreground" />
  </div>
  <h2 class="text-subheading font-semibold text-foreground mb-2">一切就绪！</h2>
  <p class="text-body text-foreground-alt">
    配置已完成，点击下方按钮开始使用 TalkShow。你可以随时在设置中修改配置。
  </p>
</div>
