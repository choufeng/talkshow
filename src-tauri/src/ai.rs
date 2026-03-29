use base64::Engine;
use rig::completion::message::{AudioMediaType, MimeType, UserContent};
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::providers::openai;
use rig::OneOrMany;
use rig::completion::message::Message;
use std::path::Path;

use crate::config::ProviderConfig;

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
        "vertex" => send_via_vertex(&audio_b64, media_type, text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_via_openai_compatible(
                &audio_b64,
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
    audio_b64: &str,
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
    let audio_mt: Option<AudioMediaType> = AudioMediaType::from_mime_type(media_type);

    let client = openai::CompletionsClient::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| AiError::RequestError(format!("Client init failed: {}", e)))?;

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

pub async fn send_text_prompt(
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    match provider.provider_type.as_str() {
        "vertex" => send_text_via_vertex(text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_text_via_openai_compatible(
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
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, AiError> {
    let client = openai::CompletionsClient::builder()
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .map_err(|e| AiError::RequestError(format!("Client init failed: {}", e)))?;

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

pub async fn send_audio_prompt_from_bytes(
    audio_bytes: &[u8],
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
    provider: &ProviderConfig,
) -> Result<String, AiError> {
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);

    match provider.provider_type.as_str() {
        "vertex" => send_via_vertex(&audio_b64, media_type, text_prompt, model_name).await,
        "openai-compatible" => {
            let api_key = provider
                .api_key
                .as_deref()
                .ok_or_else(|| AiError::MissingApiKey(provider.id.clone()))?;
            send_via_openai_compatible(
                &audio_b64,
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
