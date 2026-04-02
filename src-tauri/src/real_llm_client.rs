use crate::ai::{ThinkingMode, send_audio_prompt_from_bytes, send_text_prompt};
use crate::config::ProviderConfig;
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;

#[allow(dead_code)]
pub struct RealLlmClient<'a> {
    logger: &'a Logger,
    vertex_cache: &'a VertexClientCache,
}

impl<'a> RealLlmClient<'a> {
    #[allow(dead_code)]
    pub fn new(logger: &'a Logger, vertex_cache: &'a VertexClientCache) -> Self {
        Self {
            logger,
            vertex_cache,
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for RealLlmClient<'_> {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            provider_type: provider_id.to_string(),
            name: provider_id.to_string(),
            endpoint: endpoint.to_string(),
            api_key: None,
            models: vec![],
        };
        send_text_prompt(
            self.logger,
            prompt,
            model_name,
            &provider,
            self.vertex_cache,
            ThinkingMode::Disabled,
        )
        .await
        .map_err(|e| e.to_string())
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
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            provider_type: provider_id.to_string(),
            name: provider_id.to_string(),
            endpoint: endpoint.to_string(),
            api_key: None,
            models: vec![],
        };
        send_audio_prompt_from_bytes(
            self.logger,
            audio_bytes,
            media_type,
            text_prompt,
            model_name,
            &provider,
            self.vertex_cache,
        )
        .await
        .map_err(|e| e.to_string())
    }
}
