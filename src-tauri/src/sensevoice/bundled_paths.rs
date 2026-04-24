use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Returns the path to the bundled libonnxruntime dylib inside the app package.
pub fn onnxruntime_dylib_path(app: &AppHandle) -> Option<PathBuf> {
    let resource_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("resources")
        .join("libonnxruntime.dylib");
    if resource_path.exists() {
        Some(resource_path)
    } else {
        None
    }
}

/// Returns the path to the bundled ffmpeg sidecar binary.
pub fn ffmpeg_bin_path(app: &AppHandle) -> Option<PathBuf> {
    // Try full Tauri target triple format first (e.g. ffmpeg-aarch64-apple-darwin)
    if let Ok(triple) = tauri::utils::platform::target_triple() {
        let full_path = app
            .path()
            .resource_dir()
            .ok()?
            .join("binaries")
            .join(format!("ffmpeg-{}", triple));
        if full_path.exists() {
            return Some(full_path);
        }
    }
    // Fallback: try simple arch name
    let bin_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("binaries")
        .join(format!("ffmpeg-{}", std::env::consts::ARCH));
    if bin_path.exists() {
        Some(bin_path)
    } else {
        None
    }
}
