use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Returns the path to a compatible libonnxruntime dylib.
///
/// Priority:
/// 1. Bundled dylib inside the app package — used in production builds.
/// 2. Homebrew (`/opt/homebrew/lib/libonnxruntime.dylib`) — used in dev when
///    the bundled dylib's API version is too old.  ort-sys 2.0.0-rc.12 requires
///    ORT C API v24 (≥ ORT 1.22.x); the currently-bundled ORT 1.20.1 only
///    supports API v22 and will cause a version-mismatch error at runtime.
///    Homebrew typically ships a recent enough version.
///
/// When the bundled dylib is updated to ORT ≥ 1.22.x the homebrew fallback
/// can be removed.
pub fn onnxruntime_dylib_path(app: &AppHandle) -> Option<PathBuf> {
    // 1. Bundled dylib (production)
    let bundled = app
        .path()
        .resource_dir()
        .ok()?
        .join("resources")
        .join("libonnxruntime.dylib");
    if bundled.exists() {
        // Prefer bundled only if it is new enough (≥ ORT 1.22 = API v24).
        // We detect this by checking the symlink/file version embedded in the
        // filename of the resolved real path or by a naming convention.
        // For now we fall through to homebrew when the version is too old;
        // replace the bundled dylib with ORT ≥1.22.x to silence the fallback.
        if bundled_dylib_is_compatible(&bundled) {
            return Some(bundled);
        }
    }

    // 2. Homebrew fallback (dev / machines with recent homebrew ort)
    #[cfg(target_os = "macos")]
    {
        let homebrew = PathBuf::from("/opt/homebrew/lib/libonnxruntime.dylib");
        if homebrew.exists() {
            return Some(homebrew);
        }
    }

    None
}

/// Returns `true` if the bundled dylib is ORT ≥ 1.22.x (supports C API v24).
///
/// We check by resolving the symlink: the real file is typically named
/// `libonnxruntime.<major>.<minor>.<patch>.dylib`.  A minor version ≥ 22
/// indicates API v24 compatibility.
fn bundled_dylib_is_compatible(path: &PathBuf) -> bool {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
    let name = real
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    // e.g. "libonnxruntime.1.22.0.dylib" → minor = 22
    let minor = name
        .strip_prefix("libonnxruntime.")
        .and_then(|s| s.split('.').nth(1))
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    // API v24 requires ORT minor ≥ 22 (ORT 1.22.x → C API v24)
    minor >= 22
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
