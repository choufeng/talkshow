use crate::config::ProviderConfig;
use crate::logger::Logger;
use crate::providers::{self, ProviderContext};
use std::path::Path;

pub use providers::ProviderError as AiError;
pub use providers::ThinkingMode;

pub async fn send_audio_prompt(
    logger: &Logger,
    audio_path: &Path,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<String, AiError> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| AiError::FileReadError(e.to_string()))?;

    let extension = audio_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    let media_type = match extension {
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "mp3" => "audio/mp3",
        "ogg" => "audio/ogg",
        _ => "audio/wav",
    };

    send_audio_prompt_from_bytes(
        logger,
        &audio_data,
        media_type,
        text_prompt,
        model_name,
        provider,
        ctx,
    )
    .await
}

pub async fn send_audio_prompt_from_bytes(
    logger: &Logger,
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
) -> Result<String, AiError> {
    let p = providers::create_provider(provider, ctx)?;
    p.transcribe(logger, audio_bytes, media_type, text_prompt, model_name)
        .await
}

pub async fn send_text_prompt(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
    ctx: &ProviderContext,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
    let t = std::time::Instant::now();
    let p = providers::create_provider(provider, ctx)?;
    logger.info("ai", "create_provider 完成", Some(serde_json::json!({
        "elapsed_ms": t.elapsed().as_millis(),
    })));
    p.complete_text(logger, text_prompt, model_name, thinking)
        .await
}
