use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn send_text(
        &self,
        prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String>;

    async fn send_audio(
        &self,
        audio_bytes: &[u8],
        media_type: &str,
        text_prompt: &str,
        model_name: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> Result<String, String>;
}
