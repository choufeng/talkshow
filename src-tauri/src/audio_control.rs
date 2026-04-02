use crate::logger;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MUTE_STATE_FILE: &str = "mute_state.json";
const MAX_STALE_SECONDS: u64 = 600;

#[derive(Serialize, Deserialize)]
struct MuteState {
    volume: f64,
    timestamp: u64,
}

fn state_file_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(MUTE_STATE_FILE)
}

fn get_current_volume() -> Result<f64, String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("output volume of (get volume settings)")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get volume".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    stdout
        .parse::<f64>()
        .map_err(|_| format!("Failed to parse volume: {}", stdout))
}

fn set_volume(volume: f64) -> Result<(), String> {
    let vol = volume.round() as i64;
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!("set volume output volume {}", vol))
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn save_and_mute(
    app_data_dir: &std::path::Path,
    logger: Option<&logger::Logger>,
) -> Result<(), String> {
    let volume = get_current_volume()?;

    if volume == 0.0 {
        return Ok(());
    }

    let state = MuteState {
        volume,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    if let Some(parent) = state_file_path(app_data_dir).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let content = serde_json::to_string(&state).map_err(|e| e.to_string())?;
    fs::write(state_file_path(app_data_dir), content).map_err(|e| e.to_string())?;

    set_volume(0.0)?;

    if let Some(lg) = logger {
        lg.info(
            "audio_control",
            &format!("系统已静音 (原音量: {})", volume),
            None,
        );
    }

    Ok(())
}

pub fn restore(
    app_data_dir: &std::path::Path,
    logger: Option<&logger::Logger>,
) -> Result<(), String> {
    let path = state_file_path(app_data_dir);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let state: MuteState = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let _ = fs::remove_file(&path);

    set_volume(state.volume)?;

    if let Some(lg) = logger {
        lg.info(
            "audio_control",
            &format!("系统音量已恢复 ({})", state.volume),
            None,
        );
    }

    Ok(())
}

pub fn cleanup_stale_state(app_data_dir: &std::path::Path) -> Result<(), String> {
    let path = state_file_path(app_data_dir);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let state: MuteState = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now - state.timestamp > MAX_STALE_SECONDS {
        let _ = fs::remove_file(&path);
        set_volume(state.volume)?;
    }

    Ok(())
}
