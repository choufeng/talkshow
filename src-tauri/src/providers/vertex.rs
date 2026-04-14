use crate::config::ModelConfig;
use crate::logger::Logger;
use crate::providers::{Provider, ProviderError, ThinkingMode};
use async_trait::async_trait;
use base64::Engine;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const VERTEX_BASE_URL: &str = "https://aiplatform.googleapis.com/v1";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub type VertexTokenCache = Arc<Mutex<Option<(String, std::time::Instant)>>>;

pub struct VertexAIProvider {
    token_cache: VertexTokenCache,
    adc_credentials: AdcCredentials,
}

#[derive(serde::Deserialize)]
struct AdcCredentials {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    #[allow(dead_code)]
    quota_project_id: Option<String>,
}

fn adc_credentials_path() -> Option<PathBuf> {
    let home = home::home_dir()?;
    Some(
        home.join(".config")
            .join("gcloud")
            .join("application_default_credentials.json"),
    )
}

fn gcloud_config_path() -> Option<PathBuf> {
    let home = home::home_dir()?;
    Some(
        home.join(".config")
            .join("gcloud")
            .join("configurations")
            .join("config_default"),
    )
}

fn read_adc_credentials() -> Result<AdcCredentials, ProviderError> {
    let path = adc_credentials_path().ok_or_else(|| {
        ProviderError::RequestError(
            "Cannot determine home directory for ADC credentials".to_string(),
        )
    })?;

    let content = std::fs::read_to_string(&path).map_err(|e| {
        ProviderError::RequestError(format!(
            "Failed to read ADC credentials at {}: {}",
            path.display(),
            e
        ))
    })?;

    serde_json::from_str::<AdcCredentials>(&content)
        .map_err(|e| ProviderError::RequestError(format!("Failed to parse ADC credentials: {}", e)))
}

pub fn get_project_from_gcloud_config() -> Option<String> {
    let path = gcloud_config_path()?;

    let content = std::fs::read_to_string(&path).ok()?;

    let mut in_core = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[core]" {
            in_core = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_core = false;
            continue;
        }
        if in_core && let Some(project) = trimmed.strip_prefix("project = ") {
            return Some(project.trim().to_string());
        }
    }
    None
}

pub fn get_vertex_project() -> Result<String, ProviderError> {
    if let Ok(project) = std::env::var("GOOGLE_CLOUD_PROJECT")
        && !project.is_empty()
    {
        return Ok(project);
    }
    get_project_from_gcloud_config().ok_or_else(|| {
        ProviderError::RequestError(
            "GOOGLE_CLOUD_PROJECT not set and could not read from gcloud config".to_string(),
        )
    })
}

pub fn get_vertex_location() -> String {
    std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".to_string())
}

impl VertexAIProvider {
    pub fn new(token_cache: VertexTokenCache) -> Self {
        let adc_credentials =
            read_adc_credentials().expect("Failed to load ADC credentials on startup");
        Self {
            token_cache,
            adc_credentials,
        }
    }

    async fn get_access_token(&self) -> Result<String, ProviderError> {
        {
            let guard = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((token, expires_at)) = guard.as_ref()
                && expires_at > &std::time::Instant::now()
            {
                return Ok(token.clone());
            }
        }

        let client = reqwest::Client::new();
        let params = [
            ("client_id", self.adc_credentials.client_id.as_str()),
            ("client_secret", self.adc_credentials.client_secret.as_str()),
            ("refresh_token", self.adc_credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                ProviderError::RequestError(format!("Failed to refresh ADC token: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestError(format!(
                "Token refresh failed (HTTP {}): {}",
                status, body
            )));
        }

        let token_resp: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::RequestError(format!("Failed to parse token response: {}", e))
        })?;

        let token = token_resp
            .get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                ProviderError::RequestError("Token response missing access_token field".to_string())
            })?
            .to_string();

        let expires_in = token_resp
            .get("expires_in")
            .and_then(|t| t.as_u64())
            .unwrap_or(3600);

        let expires_at =
            std::time::Instant::now() + std::time::Duration::from_secs(expires_in - 100);

        {
            let mut guard = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some((token.clone(), expires_at));
        }

        Ok(token)
    }

    fn build_url(project: &str, location: &str, model: &str) -> String {
        format!(
            "{}/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
            VERTEX_BASE_URL, project, location, model
        )
    }
}

#[async_trait]
impl Provider for VertexAIProvider {
    async fn transcribe(
        &self,
        logger: &Logger,
        audio_bytes: &[u8],
        media_type: &str,
        prompt: &str,
        model: &str,
    ) -> Result<String, ProviderError> {
        let token = self.get_access_token().await?;
        let project = get_vertex_project()?;
        let location = get_vertex_location();
        let url = Self::build_url(&project, &location, model);

        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(audio_bytes);

        logger.info(
            "vertex",
            "准备发送音频请求",
            Some(serde_json::json!({
                "model": model,
                "media_type": media_type,
                "audio_size_b64": audio_b64.len(),
            })),
        );

        let body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [
                    {"inline_data": {"mime_type": media_type, "data": audio_b64}},
                    {"text": prompt}
                ]
            }],
            "generationConfig": {
                "thinkingConfig": {
                    "thinkingBudget": 0
                }
            }
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "vertex",
                    "音频请求失败",
                    Some(serde_json::json!({ "error": e.to_string() })),
                );
                ProviderError::RequestError(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let err = format!("HTTP {} - {}", status, body);
            logger.error(
                "vertex",
                "音频请求失败",
                Some(serde_json::json!({ "error": &err })),
            );
            return Err(ProviderError::RequestError(err));
        }

        let resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestError(format!("Failed to parse response: {}", e)))?;

        let text = extract_text_from_vertex_response(&resp);

        logger.info(
            "vertex",
            "音频请求成功",
            Some(serde_json::json!({ "response_length": text.len() })),
        );

        Ok(text)
    }

    async fn complete_text(
        &self,
        logger: &Logger,
        prompt: &str,
        model: &str,
        thinking: ThinkingMode,
    ) -> Result<String, ProviderError> {
        let t = std::time::Instant::now();
        let token = self.get_access_token().await?;
        logger.info(
            "vertex",
            "get_access_token 完成",
            Some(serde_json::json!({
                "elapsed_ms": t.elapsed().as_millis(),
            })),
        );
        let project = get_vertex_project()?;
        let location = get_vertex_location();
        let url = Self::build_url(&project, &location, model);

        logger.info(
            "vertex",
            "准备发送文本请求",
            Some(serde_json::json!({ "model": model })),
        );

        let mut generation_config = serde_json::json!({});
        match thinking {
            ThinkingMode::Disabled => {
                generation_config["thinkingConfig"] = serde_json::json!({"thinkingBudget": 0});
            }
            ThinkingMode::Enabled => {
                generation_config["thinkingConfig"] = serde_json::json!({"thinkingBudget": 8192});
            }
            ThinkingMode::Default => {}
        }

        let body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": prompt}]
            }],
            "generationConfig": generation_config
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                logger.error(
                    "vertex",
                    "文本请求失败",
                    Some(serde_json::json!({ "error": e.to_string() })),
                );
                ProviderError::RequestError(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let err = format!("HTTP {} - {}", status, body);
            logger.error(
                "vertex",
                "文本请求失败",
                Some(serde_json::json!({ "error": &err })),
            );
            return Err(ProviderError::RequestError(err));
        }

        let resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestError(format!("Failed to parse response: {}", e)))?;

        let text = extract_text_from_vertex_response(&resp);

        logger.info(
            "vertex",
            "文本请求成功",
            Some(serde_json::json!({ "response_length": text.len() })),
        );

        Ok(text)
    }

    fn needs_api_key(&self) -> bool {
        false
    }

    fn default_models() -> Vec<ModelConfig> {
        vec![ModelConfig {
            name: "gemini-2.5-flash".to_string(),
            capabilities: vec!["chat".to_string()],
            verified: None,
        }]
    }
}

fn extract_text_from_vertex_response(resp: &serde_json::Value) -> String {
    resp.get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}
