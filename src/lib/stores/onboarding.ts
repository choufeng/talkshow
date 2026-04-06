import { invoke } from '@tauri-apps/api/core';
import { writable, get } from 'svelte/store';

export const ONBOARDING_STEPS = [
  'welcome',
  'download-model',
  'provider-config',
  'shortcuts-intro',
  'try-transcription',
  'try-translation',
  'completion',
] as const;

export type OnboardingStepId = (typeof ONBOARDING_STEPS)[number];

export const TOTAL_STEPS = ONBOARDING_STEPS.length;

function createOnboardingStore() {
  const { subscribe, set, update } = writable({
    currentStep: 1,
    completed: false,
    stepValid: (() => { const a = new Array<boolean>(TOTAL_STEPS + 1).fill(false); a[1] = true; return a; })(),
    lastTranscriptionText: '',
  });

  return {
    subscribe,
    load: async () => {
      try {
        const completed = await invoke<boolean>('get_onboarding_status');
        update((s) => ({ ...s, completed }));
      } catch (error) {
        console.error('Failed to load onboarding status:', error);
      }
    },
    nextStep: () => {
      update((s) => {
        if (s.currentStep < TOTAL_STEPS) {
          return { ...s, currentStep: s.currentStep + 1 };
        }
        return s;
      });
    },
    prevStep: () => {
      update((s) => {
        if (s.currentStep > 1) {
          return { ...s, currentStep: s.currentStep - 1 };
        }
        return s;
      });
    },
    goToStep: (n: number) => {
      update((s) => {
        if (n >= 1 && n <= TOTAL_STEPS) {
          return { ...s, currentStep: n };
        }
        return s;
      });
    },
    setStepValid: (step: number, valid: boolean) => {
      update((s) => {
        const stepValid = [...s.stepValid];
        stepValid[step] = valid;
        return { ...s, stepValid };
      });
    },
    canGoNext: (): boolean => {
      const s = get({ subscribe });
      return s.currentStep < TOTAL_STEPS && s.stepValid[s.currentStep];
    },
    completeOnboarding: async () => {
      try {
        await invoke('set_onboarding_completed', { completed: true });
        update((s) => ({ ...s, completed: true }));
      } catch (error) {
        console.error('Failed to complete onboarding:', error);
      }
    },
    setTranscriptionText: (text: string) => {
      update((s) => ({ ...s, lastTranscriptionText: text }));
    },
    reset: () => {
      set({
        currentStep: 1,
        completed: false,
        stepValid: (() => { const a = new Array<boolean>(TOTAL_STEPS + 1).fill(false); a[1] = true; return a; })(),
        lastTranscriptionText: '',
      });
    },
  };
}

export const onboarding = createOnboardingStore();
