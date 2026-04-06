use crate::ai::{ThinkingMode, send_audio_prompt_from_bytes, send_text_prompt};
use crate::config::ProviderConfig;
use crate::llm_client::LlmClient;
use crate::logger::Logger;
use crate::providers::ProviderContext;

#[allow(dead_code)]
pub struct RealLlmClient<'a> {
    logger: &'a Logger,
    ctx: &'a ProviderContext,
}

impl<'a> RealLlmClient<'a> {
    #[allow(dead_code)]
    pub fn new(logger: &'a Logger, ctx: &'a ProviderContext) -> Self {
        Self { logger, ctx }
    }
}

#[async_trait::async_trait]
impl LlmClient for RealLlmClient<'_> {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            name: provider_id.to_string(),
            api_key: None,
            endpoint: None,
            models: vec![],
        };
        send_text_prompt(
            self.logger,
            prompt,
            model_name,
            &provider,
            self.ctx,
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
    ) -> Result<String, String> {
        let provider = ProviderConfig {
            id: provider_id.to_string(),
            name: provider_id.to_string(),
            api_key: None,
            endpoint: None,
            models: vec![],
        };
        send_audio_prompt_from_bytes(
            self.logger,
            audio_bytes,
            media_type,
            text_prompt,
            model_name,
            &provider,
            self.ctx,
        )
        .await
        .map_err(|e| e.to_string())
    }
}
