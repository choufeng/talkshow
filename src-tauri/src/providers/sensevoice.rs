use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct SenseVoiceProvider {
    pub engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
    pub language: Arc<Mutex<i32>>,
}

impl SenseVoiceProvider {
    pub fn new(engine: Arc<Mutex<Option<SenseVoiceEngine>>>) -> Self {
        Self {
            engine,
            language: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl Provider for SenseVoiceProvider {
    async fn transcribe(
        &self,
        _logger: &Logger,
        audio_bytes: &[u8],
        _media_type: &str,
        _prompt: &str,
        _model: &str,
    ) -> Result<String, ProviderError> {
        let tmp_dir = std::env::temp_dir().join("talkshow_provider");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join(format!("audio_{}.wav", uuid()));

        std::fs::write(&tmp_path, audio_bytes)
            .map_err(|e| ProviderError::FileReadError(e.to_string()))?;

        let lang = self
            .language
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .to_owned();

        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(engine) => engine
                .transcribe(&tmp_path, lang)
                .map_err(|e| ProviderError::RequestError(e.to_string())),
            None => Err(ProviderError::RequestError(
                "SenseVoice engine not initialized".to_string(),
            )),
        }
    }

    async fn complete_text(
        &self,
        _logger: &Logger,
        _prompt: &str,
        _model: &str,
        _thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        Err(ProviderError::UnsupportedOperation(
            "SenseVoice does not support text completion".to_string(),
        ))
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "SenseVoice-Small".to_string(),
            capabilities: vec!["transcription".to_string()],
            verified: None,
        }]
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
