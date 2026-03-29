use base64::Engine;
use rig::completion::message::{AudioMediaType, MimeType, UserContent};
use rig::client::CompletionClient;
use rig::client::transcription::TranscriptionClient;
use rig::completion::CompletionModel;
use rig::providers::openai;
use rig::transcription::TranscriptionModel;
use rig::OneOrMany;
use rig::completion::message::Message;
use std::path::Path;

use crate::config::ProviderConfig;
use crate::logger::Logger;

#[derive(Debug)]
pub enum AiError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            AiError::MissingApiKey(id) => write!(f, "Missing API key for provider: {}", id),
            AiError::FileReadError(e) => write!(f, "Failed to read audio file: {}", e),
            AiError::RequestError(e) => write!(f, "AI request failed: {}", e),
        }
    }
}

pub async fn send_audio_prompt(
    logger: &Logger,
    audio_path: &Path,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    let audio_data = std::fs::read(audio_path)
        .map_err(|e| AiError::FileReadError(e.to_string()))?;
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&audio_data);

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

    match provider.provider_type.as_str() {
        "vertex" => send_via_vertex(logger, &audio_b64, media_type, text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_via_openai_compatible(
                logger,
                &audio_data,
                media_type,
                text_prompt,
                model_name,
                api_key,
                &provider.endpoint,
            )
            .await
        }
        _ => Err(AiError::ProviderNotFound(format!(
            "Unknown provider type: {}",
            provider.provider_type
        ))),
    }
}

async fn send_via_vertex(
    _logger: &Logger,
    audio_b64: &str,
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
    let audio_mt: Option<AudioMediaType> = AudioMediaType::from_mime_type(media_type);

    let client = rig_vertexai::Client::builder()
        .build()
        .map_err(|e| AiError::RequestError(format!("Vertex AI client init failed: {}", e)))?;

    let model = client.completion_model(model_name);
    let audio_content = UserContent::audio(audio_b64.to_string(), audio_mt);

    let prompt_content = OneOrMany::many(vec![
        audio_content,
        UserContent::text(text_prompt.to_string()),
    ])
    .map_err(|e| AiError::RequestError(format!("Failed to build prompt: {}", e)))?;

    let message = Message::User {
        content: prompt_content,
    };

    let request = model.completion_request(message).build();
    let response = model
        .completion(request)
        .await
        .map_err(|e| AiError::RequestError(e.to_string()))?;

    let text = response
        .choice
        .into_iter()
        .filter_map(|c| match c {
            rig::completion::message::AssistantContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}

async fn send_via_openai_compatible(
    logger: &Logger,
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
    let extension = media_type.split('/').last().unwrap_or("wav");

    let final_url = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));
    logger.info(
        "ai",
        "准备发送 Transcription 请求",
        Some(serde_json::json!({
            "url": final_url,
            "model": model_name,
            "media_type": media_type,
            "audio_size_bytes": audio_bytes.len(),
            "prompt": text_prompt,
        })),
    );

    let client = openai::Client::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| {
            logger.error(
                "ai",
                "Client 初始化失败",
                Some(serde_json::json!({ "error": e.to_string() })),
            );
            AiError::RequestError(format!("Client init failed: {}", e))
        })?;

    let model = client.transcription_model(model_name);

    logger.info(
        "ai",
        "请求构建完成，正在发送",
        Some(serde_json::json!({
            "client_base_url": client.base_url(),
            "model": model_name,
        })),
    );

    let response = model
        .transcription_request()
        .data(audio_bytes.to_vec())
        .filename(Some(format!("audio.{}", extension)))
        .prompt(text_prompt.to_string())
        .send()
        .await
        .map_err(|e| {
            let err_str = e.to_string();
            logger.error(
                "ai",
                "AI 请求失败",
                Some(serde_json::json!({
                    "error": err_str,
                    "url": final_url,
                    "model": model_name,
                })),
            );
            AiError::RequestError(err_str)
        })?;

    logger.info(
        "ai",
        "AI 请求成功",
        Some(serde_json::json!({
            "response_length": response.text.len(),
        })),
    );

    Ok(response.text)
}

pub async fn send_text_prompt(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    match provider.provider_type.as_str() {
        "vertex" => send_text_via_vertex(logger, text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_text_via_openai_compatible(
                logger,
                text_prompt,
                model_name,
                api_key,
                &provider.endpoint,
            )
            .await
        }
        _ => Err(AiError::ProviderNotFound(format!(
            "Unknown provider type: {}",
            provider.provider_type
        ))),
    }
}

async fn send_text_via_vertex(
    _logger: &Logger,
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
    let client = rig_vertexai::Client::builder()
        .build()
        .map_err(|e| AiError::RequestError(format!("Vertex AI client init failed: {}", e)))?;

    let model = client.completion_model(model_name);

    let prompt_content = OneOrMany::one(UserContent::text(text_prompt.to_string()));
    let message = Message::User {
        content: prompt_content,
    };

    let request = model.completion_request(message).build();
    let response = model
        .completion(request)
        .await
        .map_err(|e| AiError::RequestError(e.to_string()))?;

    let text = response
        .choice
        .into_iter()
        .filter_map(|c| match c {
            rig::completion::message::AssistantContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}

async fn send_text_via_openai_compatible(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
    let final_url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    logger.info(
        "ai",
        "准备发送 OpenAI 兼容文本请求",
        Some(serde_json::json!({
            "url": final_url,
            "model": model_name,
            "prompt": text_prompt,
        })),
    );

    let client = openai::CompletionsClient::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| {
            logger.error(
                "ai",
                "Client 初始化失败",
                Some(serde_json::json!({ "error": e.to_string() })),
            );
            AiError::RequestError(format!("Client init failed: {}", e))
        })?;

    let model = client.completion_model(model_name);

    let prompt_content = OneOrMany::one(UserContent::text(text_prompt.to_string()));
    let message = Message::User {
        content: prompt_content,
    };

    let request = model.completion_request(message).build();
    let response = model
        .completion(request)
        .await
        .map_err(|e| {
            let err_str = e.to_string();
            logger.error(
                "ai",
                "AI 文本请求失败",
                Some(serde_json::json!({
                    "error": err_str,
                    "url": final_url,
                    "model": model_name,
                })),
            );
            AiError::RequestError(err_str)
        })?;

    let text = response
        .choice
        .into_iter()
        .filter_map(|c| match c {
            rig::completion::message::AssistantContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    logger.info(
        "ai",
        "AI 文本请求成功",
        Some(serde_json::json!({
            "response_length": text.len(),
        })),
    );

    Ok(text)
}

pub async fn send_audio_prompt_from_bytes(
    logger: &Logger,
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    match provider.provider_type.as_str() {
        "vertex" => {
            let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);
            send_via_vertex(logger, &audio_b64, media_type, text_prompt, model_name).await
        }
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_via_openai_compatible(
                logger,
                audio_bytes,
                media_type,
                text_prompt,
                model_name,
                api_key,
                &provider.endpoint,
            )
            .await
        }
        _ => Err(AiError::ProviderNotFound(format!(
            "Unknown provider type: {}",
            provider.provider_type
        ))),
    }
}
