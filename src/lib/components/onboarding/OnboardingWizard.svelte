<script lang="ts">
  import { onboarding, TOTAL_STEPS } from '$lib/stores/onboarding';
  import { Check } from 'lucide-svelte';
  import WelcomeStep from './steps/WelcomeStep.svelte';
  import DownloadModelStep from './steps/DownloadModelStep.svelte';
  import ProviderConfigStep from './steps/ProviderConfigStep.svelte';
  import ShortcutsIntroStep from './steps/ShortcutsIntroStep.svelte';
  import TryTranscriptionStep from './steps/TryTranscriptionStep.svelte';
  import TryTranslationStep from './steps/TryTranslationStep.svelte';
  import CompletionStep from './steps/CompletionStep.svelte';

  const STEP_COMPONENTS = [
    WelcomeStep,
    DownloadModelStep,
    ProviderConfigStep,
    ShortcutsIntroStep,
    TryTranscriptionStep,
    TryTranslationStep,
    CompletionStep,
  ];

  const STEP_LABELS = [
    '欢迎',
    '模型下载',
    'Provider 配置',
    '快捷键',
    '试用转写',
    '试用翻译',
    '完成',
  ];

  let canNext = $derived(onboarding.canGoNext());
  let isLastStep = $derived($onboarding.currentStep === TOTAL_STEPS);
  let isFirstStep = $derived($onboarding.currentStep === 1);

  function handleNext() {
    if (isLastStep) {
      onboarding.completeOnboarding();
    } else {
      onboarding.nextStep();
    }
  }

  function handlePrev() {
    onboarding.prevStep();
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-background">
  <div class="w-full max-w-xl mx-4">
    <div class="flex items-center justify-center gap-1 mb-10">
      {#each STEP_LABELS as label, i}
        {@const stepNum = i + 1}
        {@const isCurrent = $onboarding.currentStep === stepNum}
        {@const isCompleted = $onboarding.currentStep > stepNum}
        <div class="flex items-center gap-1">
          <div
            class="flex items-center justify-center w-8 h-8 rounded-full text-caption font-medium transition-all {isCurrent ? 'bg-btn-primary-from text-white shadow-btn-primary' : isCompleted ? 'bg-btn-primary-from/20 text-btn-primary-from' : 'bg-muted text-muted-foreground'}"
          >
            {#if isCompleted}
              <Check size={14} />
            {:else}
              {stepNum}
            {/if}
          </div>
          {#if stepNum < TOTAL_STEPS}
            <div class="w-4 h-0.5 rounded-full {isCompleted ? 'bg-btn-primary-from/30' : 'bg-muted'}"></div>
          {/if}
        </div>
      {/each}
    </div>

    <div class="rounded-xl border border-border bg-background-alt p-8 shadow-card min-h-[360px] flex flex-col">
      <div class="flex-1">
        {#each STEP_COMPONENTS as StepComponent, i}
          {#if $onboarding.currentStep === i + 1}
            {@const Step = StepComponent}
            <Step />
          {/if}
        {/each}
      </div>

      <div class="flex justify-between items-center mt-8 pt-4 border-t border-border">
        <button
          class="px-5 py-2 rounded-lg text-body font-medium transition-colors border border-border bg-gradient-to-b from-btn-secondary-from to-btn-secondary-to text-foreground shadow-btn-secondary {isFirstStep ? 'invisible' : 'hover:bg-muted/50'}"
          onclick={handlePrev}
          disabled={isFirstStep}
        >
          上一步
        </button>
        <button
          class="px-5 py-2 rounded-lg text-body font-medium transition-colors border-0 bg-gradient-to-b from-btn-primary-from to-btn-primary-to text-white shadow-btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
          onclick={handleNext}
          disabled={!canNext && !isLastStep}
        >
          {isLastStep ? '开始使用' : '下一步'}
        </button>
      </div>
    </div>
  </div>
</div>
