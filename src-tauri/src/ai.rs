use base64::Engine;
use rig::OneOrMany;
use rig::client::CompletionClient;
use rig::client::transcription::TranscriptionClient;
use rig::completion::CompletionModel;
use rig::completion::message::Message;
use rig::completion::message::{AudioMediaType, MimeType, UserContent};
use rig::providers::openai;
use rig::transcription::TranscriptionModel;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::ProviderConfig;
use crate::logger::Logger;

#[derive(Debug)]
pub enum AiError {
    ProviderNotFound(String),
    MissingApiKey(String),
    FileReadError(String),
    RequestError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ThinkingMode {
    Default,
    Enabled,
    Disabled,
}

type VertexClientCache = Arc<Mutex<Option<rig_vertexai::Client>>>;

fn get_or_create_vertex_client(
    logger: &Logger,
    cache: &VertexClientCache,
) -> Result<rig_vertexai::Client, AiError> {
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref client) = *guard {
        return Ok(client.clone());
    }
    let client = rig_vertexai::Client::builder().build().map_err(|e| {
        logger.error(
            "ai",
            "Vertex AI client 初始化失败",
            Some(serde_json::json!({ "error": e.to_string() })),
        );
        AiError::RequestError(format!("Vertex AI client init failed: {}", e))
    })?;
    *guard = Some(client.clone());
    logger.info("ai", "Vertex AI client 已创建并缓存", None);
    Ok(client)
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
    vertex_cache: &VertexClientCache,
) -> Result<String, AiError> {
    let audio_data =
        std::fs::read(audio_path).map_err(|e| AiError::FileReadError(e.to_string()))?;
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
        "vertex" => {
            let client = get_or_create_vertex_client(logger, vertex_cache)?;
            send_via_vertex(
                logger,
                &client,
                &audio_b64,
                media_type,
                text_prompt,
                model_name,
            )
            .await
        }
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
    logger: &Logger,
    client: &rig_vertexai::Client,
    audio_b64: &str,
    media_type: &str,
    text_prompt: &str,
    model_name: &str,
) -> Result<String, AiError> {
    logger.info(
        "ai",
        "准备发送 Vertex AI 音频请求",
        Some(serde_json::json!({
            "model": model_name,
            "media_type": media_type,
            "audio_size_b64": audio_b64.len(),
            "prompt": text_prompt,
        })),
    );

    let audio_mt: Option<AudioMediaType> = AudioMediaType::from_mime_type(media_type);

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

    logger.info(
        "ai",
        "Vertex AI 请求发送中",
        Some(serde_json::json!({ "model": model_name })),
    );

    let request = model.completion_request(message).build();
    let response = model.completion(request).await.map_err(|e| {
        logger.error(
            "ai",
            "Vertex AI 请求失败",
            Some(serde_json::json!({ "error": e.to_string(), "model": model_name })),
        );
        AiError::RequestError(e.to_string())
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
        "Vertex AI 请求成功",
        Some(serde_json::json!({ "response_length": text.len() })),
    );

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
    let extension = media_type.split('/').next_back().unwrap_or("wav");

    let final_url = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));
    logger.info(
        "ai",
        "准备发送 Transcription 请求",
        Some(serde_json::json!({
            "model": model_name,
            "media_type": media_type,
            "audio_size_bytes": audio_bytes.len(),
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
    vertex_cache: &VertexClientCache,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
    match provider.provider_type.as_str() {
        "vertex" => {
            let client = get_or_create_vertex_client(logger, vertex_cache)?;
            send_text_via_vertex(logger, &client, text_prompt, model_name, thinking).await
        }
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
                thinking,
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
    logger: &Logger,
    client: &rig_vertexai::Client,
    text_prompt: &str,
    model_name: &str,
    _thinking: ThinkingMode,
) -> Result<String, AiError> {
    logger.info(
        "ai",
        "准备发送 Vertex AI 文本请求",
        Some(serde_json::json!({
            "model": model_name,
        })),
    );

    let model = client.completion_model(model_name);

    let prompt_content = OneOrMany::one(UserContent::text(text_prompt.to_string()));
    let message = Message::User {
        content: prompt_content,
    };

    logger.info(
        "ai",
        "Vertex AI 文本请求发送中",
        Some(serde_json::json!({ "model": model_name })),
    );

    let request = model.completion_request(message).build();
    let response = model.completion(request).await.map_err(|e| {
        logger.error(
            "ai",
            "Vertex AI 文本请求失败",
            Some(serde_json::json!({ "error": e.to_string(), "model": model_name })),
        );
        AiError::RequestError(e.to_string())
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
        "Vertex AI 文本请求成功",
        Some(serde_json::json!({ "response_length": text.len() })),
    );

    Ok(text)
}

async fn send_text_via_openai_compatible(
    logger: &Logger,
    text_prompt: &str,
    model_name: &str,
    api_key: &str,
    base_url: &str,
    thinking: ThinkingMode,
) -> Result<String, AiError> {
    use std::time::Instant;

    let t_start = Instant::now();

    let final_url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    logger.info(
        "ai",
        "准备发送 OpenAI 兼容文本请求",
        Some(serde_json::json!({
            "model": model_name,
        })),
    );

    let t_before_client = Instant::now();
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
    let client_build_ms = t_before_client.elapsed().as_millis();

    let t_before_model = Instant::now();
    let model = client.completion_model(model_name);

    let prompt_content = OneOrMany::one(UserContent::text(text_prompt.to_string()));
    let message = Message::User {
        content: prompt_content,
    };

    let request = match thinking {
        ThinkingMode::Default => model.completion_request(message).build(),
        ThinkingMode::Enabled => model
            .completion_request(message)
            .additional_params(serde_json::json!({"enable_thinking": true}))
            .build(),
        ThinkingMode::Disabled => model
            .completion_request(message)
            .additional_params(serde_json::json!({"enable_thinking": false}))
            .build(),
    };
    let build_req_ms = t_before_model.elapsed().as_millis();

    let t_before_completion = Instant::now();
    let response = model.completion(request).await.map_err(|e| {
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
    let completion_ms = t_before_completion.elapsed().as_millis();

    let t_before_parse = Instant::now();
    let text = response
        .choice
        .into_iter()
        .filter_map(|c| match c {
            rig::completion::message::AssistantContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    let parse_ms = t_before_parse.elapsed().as_millis();

    let total_ms = t_start.elapsed().as_millis();

    logger.info(
        "ai",
        "AI 文本请求成功 [分段计时]",
        Some(serde_json::json!({
            "response_length": text.len(),
            "total_ms": total_ms,
            "client_build_ms": client_build_ms,
            "build_request_ms": build_req_ms,
            "completion_ms": completion_ms,
            "parse_ms": parse_ms,
            "url": final_url,
            "model": model_name,
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
    vertex_cache: &VertexClientCache,
) -> Result<String, AiError> {
    match provider.provider_type.as_str() {
        "vertex" => {
            let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);
            let client = get_or_create_vertex_client(logger, vertex_cache)?;
            send_via_vertex(
                logger,
                &client,
                &audio_b64,
                media_type,
                text_prompt,
                model_name,
            )
            .await
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
