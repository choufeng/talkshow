use sha2::Digest as _;
use std::io::Read as _;
use std::path::PathBuf;
use tauri::Emitter;

use super::SenseVoiceModelStatus;

const MODEL_DIR_NAME: &str = "sensevoice";
const HF_REPO: &str = "haixuantao/SenseVoiceSmall-onnx";

const MODEL_FILES: &[(&str, u64, &str)] = &[
    (
        "model_quant.onnx",
        241_216_270,
        "21dc965f689a78d1604717bf561e40d5a236087c85a95584567835750549e822",
    ),
    (
        "config.yaml",
        1_855,
        "f71e239ba36705564b5bf2d2ffd07eece07b8e3f2bbf6d2c99d8df856339ac19",
    ),
    (
        "am.mvn",
        11_203,
        "29b3c740a2c0cfc6b308126d31d7f265fa2be74f3bb095cd2f143ea970896ae5",
    ),
    (
        "chn_jpn_yue_eng_ko_spectok.bpe.model",
        377_341,
        "a2594fc1474e78973149cba8cd1f603ebed8c39c7decb470631f66e70ce58e97",
    ),
    (
        "tokens.json",
        352_064,
        "aa87f86064c3730d799ddf7af3c04659151102cba548bce325cf06ba4da4e6a8",
    ),
];

fn verify_file_hash(file_path: &std::path::Path, expected_hash: &str) -> Result<bool, String> {
    let mut file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected_hash)
}

fn model_dir(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("models").join(MODEL_DIR_NAME)
}

/// Files required at runtime (subset of MODEL_FILES that the engine actually reads).
const REQUIRED_FILES: &[&str] = &[
    "model_quant.onnx",
    "am.mvn",
    "chn_jpn_yue_eng_ko_spectok.bpe.model",
];

pub fn model_status(app_data_dir: &std::path::Path) -> SenseVoiceModelStatus {
    let dir = model_dir(app_data_dir);

    for &file in REQUIRED_FILES {
        if !dir.join(file).exists() {
            return SenseVoiceModelStatus::NotDownloaded;
        }
    }

    let mut total_size: u64 = 0;
    for &(filename, expected_size, _expected_hash) in MODEL_FILES {
        let path = dir.join(filename);
        if !path.exists() {
            return SenseVoiceModelStatus::NotDownloaded;
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size != expected_size {
            return SenseVoiceModelStatus::NotDownloaded;
        }
        total_size += size;
    }
    SenseVoiceModelStatus::Ready {
        size_bytes: total_size,
    }
}

pub async fn download_model_files(
    app_handle: &tauri::AppHandle,
    app_data_dir: &std::path::Path,
) -> Result<(), String> {
    let dir = model_dir(app_data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    for &(filename, expected_size, expected_hash) in MODEL_FILES {
        let file_path = dir.join(filename);
        let is_valid = match std::fs::metadata(&file_path) {
            Ok(meta) => {
                if meta.len() != expected_size {
                    false
                } else {
                    verify_file_hash(&file_path, expected_hash).unwrap_or(true)
                }
            }
            Err(_) => false,
        };
        if is_valid {
            continue;
        }
        let _ = std::fs::remove_file(&file_path);
        let tmp_path = dir.join(format!("{}.tmp", filename));
        if tmp_path.exists() {
            let _ = std::fs::remove_file(&tmp_path);
        }
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            HF_REPO, filename
        );
        let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
        let total = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        let file = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        use tokio::io::AsyncWriteExt;
        let mut async_file = tokio::fs::File::from_std(file);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            async_file
                .write_all(&chunk)
                .await
                .map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            let percent = if total > 0 {
                (downloaded as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            let _ = app_handle.emit(
                "sensevoice:download-progress",
                serde_json::json!({
                    "file": filename,
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent,
                }),
            );
        }
        async_file.flush().await.map_err(|e| e.to_string())?;
        drop(async_file);
        if let Ok(valid) = verify_file_hash(&tmp_path, expected_hash)
            && !valid
        {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(format!("SHA-256 hash mismatch for {}", filename));
        }
        std::fs::rename(&tmp_path, &file_path).map_err(|e| e.to_string())?;
    }
    let _ = app_handle.emit("sensevoice:download-complete", serde_json::json!({}));
    Ok(())
}

pub async fn delete_model(app_data_dir: &std::path::Path) -> Result<(), String> {
    let dir = model_dir(app_data_dir);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn cleanup_tmp_files(app_data_dir: &std::path::Path) {
    let dir = model_dir(app_data_dir);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && name.ends_with(".tmp")
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
