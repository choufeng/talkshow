export interface Skill {
  id: string;
  name: string;
  description: string;
  prompt: string;
  enabled: boolean;
}

export interface ProviderConfig {
  id: string;
  name: string;
  type: string;
  endpoint: string;
  api_key?: string;
  models?: string[];
  last_tested_at?: string;
  last_test_result?: string;
}

export interface TranscriptionConfig {
  provider_id: string;
  model: string;
  polish_enabled: boolean;
  polish_provider_id: string;
  polish_model: string;
}

export interface TranslationConfig {
  provider_id: string;
  model: string;
}

export interface RecordingConfig {
  auto_mute: boolean;
}

export interface SkillsConfig {
  enabled: boolean;
  skills: Skill[];
}

export interface FeaturesConfig {
  transcription: TranscriptionConfig;
  translation: TranslationConfig;
  recording: RecordingConfig;
  skills: SkillsConfig;
}

export interface AppConfig {
  features: FeaturesConfig;
  ai: {
    providers: ProviderConfig[];
  };
}
