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
/// In dev mode, looks in resource_dir()/binaries/ffmpeg-<triple>.
/// In production (.app bundle), Tauri places sidecars in Contents/MacOS/,
/// but resource_dir() points to Contents/Resources/ — so we also check the parent.
pub fn ffmpeg_bin_path(app: &AppHandle) -> Option<PathBuf> {
    let triple = tauri::utils::platform::target_triple().ok()?;
    let filename = format!("ffmpeg-{}", triple);

    // Try resource_dir/binaries/ first (works in dev mode and some bundle layouts)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("binaries").join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
        // In .app bundle, sidecars go to Contents/MacOS/ (sibling of Contents/Resources/)
        if let Some(contents) = resource_dir.parent() {
            let candidate = contents.join("MacOS").join(&filename);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
