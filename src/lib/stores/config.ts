import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';

export interface ModelVerified {
  status: 'ok' | 'error';
  tested_at: string;
  latency_ms?: number;
  message?: string;
}

export interface ModelConfig {
  name: string;
  capabilities: string[];
  verified?: ModelVerified;
}

export interface ProviderConfig {
  id: string;
  name: string;
  api_key?: string;
  endpoint?: string;
  models: ModelConfig[];
}

export interface AiConfig {
  providers: ProviderConfig[];
}

export interface TranscriptionConfig {
  provider_id: string;
  model: string;
  polish_enabled: boolean;
  polish_provider_id: string;
  polish_model: string;
}

export interface TranslationConfig {
  target_lang: string;
}

export const TRANSLATE_LANGUAGES = [
  'English',
  '中文',
  '日本語',
  '한국어',
  'Français',
  'Deutsch',
  'Español',
  'Português',
  'Italiano',
  'Русский',
  'العربية',
  'हिन्दी',
];

export interface Skill {
  id: string;
  name: string;
  description: string;
  prompt: string;
  builtin: boolean;
  editable: boolean;
  enabled: boolean;
}

export interface SkillsConfig {
  enabled: boolean;
  skills: Skill[];
}

export interface RecordingFeaturesConfig {
  auto_mute: boolean;
  /** 首选麦克风设备名,空 = 系统默认 */
  input_device: string;
}

export interface IntentRoutingConfig {
  enabled: boolean;
  /** 禁用的意图 ("translate"/"ask"/"hook"),空 = 全部启用 */
  disabled: string[];
}

export interface HooksConfig {
  enabled: boolean;
  url: string;
  headers: string;
  body_template: string;
  prompt: string;
  skill_ids: string[];
}

export interface ReviewConfig {
  enabled: boolean;
  send_key: string;
}

/** 滴答清单预设:开放平台 developer.dida365.com 创建应用后生成 PAT */
export const DIDA365_PRESET = {
  url: 'https://api.dida365.com/open/v1/task',
  headers: 'Authorization: Bearer 你的个人访问令牌',
  body_template:
    '{"title":"{{title}}","content":"{{raw}}","dueDate":"{{dueDate}}","isAllDay":"{{isAllDay}}","priority":"{{priority}}"}',
  prompt: [
    '你是任务整理助手。从输入中提取信息，严格输出 JSON 对象（不要代码块，不要解释）：',
    '{"title":"一句话任务标题","dueDate":"yyyy-MM-ddTHH:mm:ss+0800|null","isAllDay":false,"priority":0}',
    '- title 必填：提炼成简洁任务标题，去除语气词',
    '- dueDate：仅当输入明确提到时间/日期时，结合当前时间换算输出（如 2026-01-15T15:00:00+0800）；只有日期没时间则输出如 2026-01-15T09:00:00+0800 且 isAllDay 为 true；无时间信息输出 null',
    '- priority：0 无，1 低，3 中，5 高；未提及输出 0',
    '- 不得输出 JSON 以外的任何内容'
  ].join('\n')
};

export interface GlossaryEntry {
  wrong: string;
  correct: string;
}

export interface FeaturesConfig {
  transcription: TranscriptionConfig;
  translation: TranslationConfig;
  skills: SkillsConfig;
  recording: RecordingFeaturesConfig;
  glossary: GlossaryEntry[];
  intent_routing: IntentRoutingConfig;
  hooks: HooksConfig;
  review: ReviewConfig;
}

export interface AppConfig {
  shortcut: string;
  recording_shortcut: string;
  translate_shortcut: string;
  ai: AiConfig;
  features: FeaturesConfig;
}

export const MODEL_CAPABILITIES = [
  { value: 'transcription', label: '语音转文字' },
  { value: 'chat', label: '文本对话' }
];

export const BUILTIN_PROVIDERS: ProviderConfig[] = [
  {
    id: 'openai',
    name: 'OpenAI',
    api_key: '',
    endpoint: 'https://api.openai.com/v1',
    models: [{ name: 'gpt-4o', capabilities: ['chat'] }]
  },
  {
    id: 'vertex',
    name: 'Vertex AI',
    models: [{ name: 'gemini-2.5-flash', capabilities: ['chat'] }]
  },
  {
    id: 'dashscope',
    name: '阿里云',
    api_key: '',
    models: [{ name: 'qwen3.5-flash', capabilities: ['chat'] }]
  },
  {
    id: 'sensevoice',
    name: 'SenseVoice (本地)',
    models: [{ name: 'SenseVoice-Small', capabilities: ['transcription'] }]
  },
  {
    id: 'litellm',
    name: 'LiteLLM',
    api_key: '',
    endpoint: 'http://localhost:4000/v1',
    models: [{ name: 'whisper-1', capabilities: ['transcription', 'chat'] }]
  },
  {
    id: 'zhipu',
    name: '智谱 AI',
    api_key: '',
    endpoint: 'https://open.bigmodel.cn/api/paas/v4',
    models: [
      { name: 'glm-asr', capabilities: ['transcription'] },
      { name: 'glm-4.7-flash', capabilities: ['chat'] },
      { name: 'glm-4.7', capabilities: ['chat'] },
      { name: 'glm-5', capabilities: ['chat'] }
    ]
  }
];

export function isBuiltinProvider(id: string): boolean {
  return BUILTIN_PROVIDERS.some((p) => p.id === id);
}

export const PROVIDERS_REQUIRING_KEY = ['dashscope', 'openai', 'zhipu', 'litellm'];

export const PROVIDERS_WITH_ENDPOINT = ['openai', 'zhipu', 'litellm'];
export function needsApiKey(providerId: string): boolean {
  return PROVIDERS_REQUIRING_KEY.includes(providerId);
}

function getBuiltinProvider(id: string): ProviderConfig | undefined {
  return BUILTIN_PROVIDERS.find((p) => p.id === id);
}

function migrateModels(providers: ProviderConfig[]): ProviderConfig[] {
  return providers.map((p) => ({
    ...p,
    models: (p.models || []).map((m) =>
      typeof m === 'string' ? { name: m, capabilities: [] as string[] } : m
    )
  }));
}

function mergeBuiltinProviders(providers: ProviderConfig[]): ProviderConfig[] {
  const userIds = new Set(providers.map((p) => p.id));
  const missing = BUILTIN_PROVIDERS.filter((p) => !userIds.has(p.id));
  const corrected = providers.map((p) => {
    const builtin = BUILTIN_PROVIDERS.find((b) => b.id === p.id);
    if (builtin) {
      return { ...p, name: builtin.name };
    }
    return p;
  });
  return [...missing, ...corrected];
}

function createConfigStore() {
  const { subscribe, set, update } = writable<AppConfig>({
    shortcut: 'Control+Shift+Quote',
    recording_shortcut: 'Control+Backslash',
    translate_shortcut: 'Control+Shift+T',
    ai: {
      providers: BUILTIN_PROVIDERS.map((p) => ({ ...p }))
    },
    features: {
      transcription: {
        provider_id: 'vertex',
        model: 'gemini-2.5-flash',
        polish_enabled: true,
        polish_provider_id: '',
        polish_model: ''
      },
      translation: {
        target_lang: 'English'
      },
      skills: {
        enabled: true,
        skills: []
      },
      recording: {
        auto_mute: false,
        input_device: ''
      },
      glossary: [],
      intent_routing: {
        enabled: false,
        disabled: []
      },
      hooks: {
        enabled: false,
        url: '',
        headers: '',
        body_template: '{"raw":"{{raw}}","text":"{{text}}","ts":"{{ts}}","app":"{{app}}"}',
        prompt: '',
        skill_ids: []
      },
      review: {
        enabled: false,
        send_key: 'enter'
      }
    }
  });

  return {
    subscribe,
    load: async () => {
      try {
        const config = await invoke<AppConfig>('get_config');
        config.ai.providers = mergeBuiltinProviders(
          migrateModels(config.ai.providers || [])
        );
        config.features.glossary = config.features.glossary || [];
        config.features.recording = config.features.recording || { auto_mute: false, input_device: '' };
        config.features.recording.input_device = config.features.recording.input_device || '';
        config.features.intent_routing = config.features.intent_routing || { enabled: false, disabled: [] };
        config.features.intent_routing.disabled = config.features.intent_routing.disabled || [];
        config.features.hooks = config.features.hooks || {
          enabled: false,
          url: '',
          headers: '',
          body_template: '{"raw":"{{raw}}","text":"{{text}}","ts":"{{ts}}","app":"{{app}}"}',
          prompt: '',
          skill_ids: []
        };
        config.features.review = config.features.review || { enabled: false, send_key: 'enter' };
        config.features.review.send_key = config.features.review.send_key || 'enter';
        set(config);
      } catch (error) {
        console.error('Failed to load config:', error);
      }
    },
    updateShortcut: async (type: 'toggle' | 'recording' | 'translate', shortcut: string) => {
      try {
        await invoke('update_shortcut', { shortcutType: type, shortcut });
        update(config => {
          if (type === 'toggle') {
            return { ...config, shortcut };
          } else if (type === 'recording') {
            return { ...config, recording_shortcut: shortcut };
          } else {
            return { ...config, translate_shortcut: shortcut };
          }
        });
      } catch (error) {
        console.error('Failed to update shortcut:', error);
        throw error;
      }
    },
    save: async (newConfig: AppConfig) => {
      try {
        await invoke('save_config_cmd', { config: newConfig });
        set(newConfig);
      } catch (error) {
        console.error('Failed to save config:', error);
        throw error;
      }
    }
  };
}

export interface SenseVoiceModelStatus {
  status: 'not_downloaded' | 'downloading' | 'ready' | 'error';
  file?: string;
  percent?: number;
  downloaded?: number;
  total?: number;
  size_bytes?: number;
  message?: string;
}

export const SENSEVOICE_LANGUAGES = [
  { value: 0, label: '自动检测' },
  { value: 3, label: '中文' },
  { value: 4, label: '英文' },
  { value: 11, label: '日文' },
];

export const config = createConfigStore();
