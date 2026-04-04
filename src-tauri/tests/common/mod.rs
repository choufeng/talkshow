//! Common test utilities for integration tests.
//!
//! This module is compiled separately for each test target, so items may appear
//! unused in some targets while being used in others.
#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use talkshow_lib::{
    AiConfig, AppConfig, FeaturesConfig, LlmClient, Logger, ProviderConfig,
    RecordingFeaturesConfig, Skill, SkillsConfig, TranscriptionConfig, TranslationConfig,
};

use std::sync::atomic::{AtomicUsize, Ordering};

/// 创建临时目录的测试 Logger
/// 返回 (Logger, TempDir) 元组，调用者需保持 TempDir 存活
pub fn create_test_logger() -> (Logger, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let logger = Logger::new(dir.path()).unwrap();
    (logger, dir)
}

/// 创建最小 AppConfig
pub fn test_app_config() -> AppConfig {
    AppConfig {
        shortcut: "Control+Shift+KeyA".to_string(),
        recording_shortcut: "Control+Shift+KeyB".to_string(),
        translate_shortcut: "Control+Shift+KeyC".to_string(),
        ai: AiConfig {
            providers: test_providers(),
        },
        features: FeaturesConfig {
            transcription: test_transcription_config(),
            translation: TranslationConfig {
                target_lang: "English".to_string(),
            },
            skills: enabled_skills_config(),
            recording: RecordingFeaturesConfig { auto_mute: false },
        },
    }
}

/// 创建测试 Provider 列表
pub fn test_providers() -> Vec<ProviderConfig> {
    vec![ProviderConfig {
        id: "test-provider".to_string(),
        provider_type: "openai-compatible".to_string(),
        name: "Test Provider".to_string(),
        endpoint: "https://api.example.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        models: vec![],
    }]
}

/// 创建启用的 Skills 配置
pub fn enabled_skills_config() -> SkillsConfig {
    SkillsConfig {
        enabled: true,
        skills: vec![Skill {
            id: "test-skill".to_string(),
            name: "测试技能".to_string(),
            description: "用于测试".to_string(),
            prompt: "测试 prompt".to_string(),
            builtin: false,
            editable: true,
            enabled: true,
        }],
    }
}

/// 创建测试 Transcription 配置
pub fn test_transcription_config() -> TranscriptionConfig {
    TranscriptionConfig {
        provider_id: "test-provider".to_string(),
        model: "test-model".to_string(),
        polish_enabled: true,
        polish_provider_id: "test-provider".to_string(),
        polish_model: "test-model".to_string(),
    }
}

/// 手动 Mock LlmClient，用于 tests/ 目录的集成测试
///
/// MockLlmClient 由 mockall 生成，但仅在 crate 内 #[cfg(test)] 可见。
/// tests/ 目录是独立 crate，无法访问 MockLlmClient，因此需要手动实现。
pub struct MockLlmClientIntegration {
    send_text_handler:
        Option<Box<dyn Fn(&str, &str, &str, &str) -> Result<String, String> + Send + Sync>>,
    send_audio_handler: Option<
        Box<dyn Fn(&[u8], &str, &str, &str, &str, &str) -> Result<String, String> + Send + Sync>,
    >,
    send_text_call_count: AtomicUsize,
    send_audio_call_count: AtomicUsize,
}

impl MockLlmClientIntegration {
    pub fn new() -> Self {
        Self {
            send_text_handler: None,
            send_audio_handler: None,
            send_text_call_count: AtomicUsize::new(0),
            send_audio_call_count: AtomicUsize::new(0),
        }
    }

    pub fn expect_send_text<F>(&mut self, handler: F)
    where
        F: Fn(&str, &str, &str, &str) -> Result<String, String> + Send + Sync + 'static,
    {
        self.send_text_handler = Some(Box::new(handler));
    }

    pub fn expect_send_audio<F>(&mut self, handler: F)
    where
        F: Fn(&[u8], &str, &str, &str, &str, &str) -> Result<String, String>
            + Send
            + Sync
            + 'static,
    {
        self.send_audio_handler = Some(Box::new(handler));
    }

    pub fn send_text_call_count(&self) -> usize {
        self.send_text_call_count.load(Ordering::SeqCst)
    }

    pub fn send_audio_call_count(&self) -> usize {
        self.send_audio_call_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClientIntegration {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String> {
        self.send_text_call_count.fetch_add(1, Ordering::SeqCst);
        match &self.send_text_handler {
            Some(handler) => handler(prompt, model_name, provider_id, endpoint),
            None => Ok("default mock response".to_string()),
        }
    }

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String> {
        self.send_audio_call_count.fetch_add(1, Ordering::SeqCst);
        match &self.send_audio_handler {
            Some(handler) => handler(
                audio_bytes,
                media_type,
                text_prompt,
                model_name,
                provider_id,
                endpoint,
            ),
            None => Ok("default mock audio response".to_string()),
        }
    }
}
