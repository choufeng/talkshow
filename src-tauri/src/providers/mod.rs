pub mod dashscope;
pub mod sensevoice;
pub mod vertex;

use crate::config::{ModelConfig, ProviderConfig};
use crate::logger::Logger;
use crate::sensevoice::SenseVoiceEngine;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ThinkingMode {
    Default,
    Enabled,
    Disabled,
}

#[derive(Debug)]
pub enum ProviderError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
    UnsupportedOperation(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            ProviderError::MissingApiKey(id) => {
                write!(f, "Missing API key for provider: {}", id)
            }
            ProviderError::FileReadError(e) => write!(f, "Failed to read audio file: {}", e),
            ProviderError::RequestError(e) => write!(f, "AI request failed: {}", e),
            ProviderError::UnsupportedOperation(op) => {
                write!(f, "Unsupported operation: {}", op)
            }
        }
    }
}

impl std::error::Error for ProviderError {}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError>;

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        thinking: ThinkingMode,
    ) -> Result<String, ProviderError>;

    #[allow(dead_code)]
    fn needs_api_key(&self) -> bool;
    #[allow(dead_code)]
    fn default_models() -> Vec<ModelConfig>
    where
        Self: Sized;
}

pub struct ProviderContext {
    pub sensevoice_engine: Arc<Mutex<Option<SenseVoiceEngine>>>,
}

impl ProviderContext {
    pub fn new() -> Self {
        ProviderContext {
            sensevoice_engine: Arc::new(Mutex::new(None)),
        }
    }
}

pub fn create_provider(
    config: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<Box<dyn Provider>, ProviderError> {
    match config.id.as_str() {
        "dashscope" => Ok(Box::new(dashscope::DashScopeProvider::new(
            config.api_key.clone(),
        ))),
        "vertex" => Ok(Box::new(vertex::VertexAIProvider::new())),
        "sensevoice" => Ok(Box::new(sensevoice::SenseVoiceProvider::new(
            ctx.sensevoice_engine.clone(),
        ))),
        _ => Err(ProviderError::ProviderNotFound(config.id.clone())),
    }
}
