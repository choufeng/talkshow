pub(crate) mod bundled_paths;
mod download;
pub(crate) mod engine;

pub use download::model_status;
pub use engine::{SenseVoiceEngine, ensure_ort_initialized_pub};

use tauri::Manager;

#[derive(serde::Serialize, Clone)]
#[serde(tag = "status")]
#[allow(dead_code)]
pub enum SenseVoiceModelStatus {
    #[serde(rename = "not_downloaded")]
    NotDownloaded,
    #[serde(rename = "downloading")]
    Downloading {
        file: String,
        percent: f64,
        downloaded: u64,
        total: u64,
    },
    #[serde(rename = "ready")]
    Ready { size_bytes: u64 },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SenseVoiceError {
    ModelNotDownloaded,
    DownloadFailed(String),
    LoadFailed(String),
    InferenceFailed(String),
    InvalidAudio(String),
}

impl std::fmt::Display for SenseVoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SenseVoiceError::ModelNotDownloaded => write!(f, "SenseVoice 模型未下载"),
            SenseVoiceError::DownloadFailed(e) => write!(f, "模型下载失败: {}", e),
            SenseVoiceError::LoadFailed(e) => write!(f, "模型加载失败: {}", e),
            SenseVoiceError::InferenceFailed(e) => write!(f, "推理失败: {}", e),
            SenseVoiceError::InvalidAudio(e) => write!(f, "音频无效: {}", e),
        }
    }
}

impl std::error::Error for SenseVoiceError {}

#[tauri::command]
pub async fn get_sensevoice_status(
    app_handle: tauri::AppHandle,
) -> Result<SenseVoiceModelStatus, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(model_status(&app_data_dir))
}

#[tauri::command]
pub async fn download_sensevoice_model(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    download::download_model_files(&app_handle, &app_data_dir).await
}

#[tauri::command]
pub async fn delete_sensevoice_model(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    download::delete_model(&app_data_dir).await
}
