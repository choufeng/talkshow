use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingCancelled {
    pub duration_secs: u64,
}

const RECORDINGS_DIR_NAME: &str = "talkshow";

pub fn recordings_dir() -> PathBuf {
    std::env::temp_dir().join(RECORDINGS_DIR_NAME)
}

pub fn generate_filename() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let total_secs = now.as_secs();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;

    let days_since_epoch = total_secs / 86400;
    let (year, month, day) = days_to_date(days_since_epoch);

    format!(
        "talkshow_{:04}{:02}{:02}_{:02}{:02}{:02}.flac",
        year, month, day, hours, mins, secs
    )
}

fn days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    let mut days = days_since_epoch;
    let mut year = 1970u64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0u64;
    for (i, &d) in month_days.iter().enumerate() {
        if days < d {
            month = i as u64 + 1;
            break;
        }
        days -= d;
    }

    (year, month, days + 1)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn ensure_recordings_dir() -> Result<PathBuf, String> {
    let dir = recordings_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings dir: {}", e))?;
    Ok(dir)
}

pub fn wav_to_flac(wav_path: &Path, flac_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("flac")
        .arg("--silent")
        .arg("--force")
        .arg("-o")
        .arg(flac_path)
        .arg(wav_path)
        .output()
        .map_err(|e| format!("Failed to execute flac: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("flac encoding failed: {}", stderr))
    } else {
        Ok(())
    }
}
