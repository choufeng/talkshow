#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

static TARGET_APP: Mutex<Option<String>> = Mutex::new(None);

pub fn save_target_app(app_name: &str) {
    if let Ok(mut guard) = TARGET_APP.lock() {
        *guard = Some(app_name.to_string());
    }
}

pub fn get_target_app() -> Option<String> {
    TARGET_APP.lock().ok().and_then(|g| g.clone())
}

pub fn write_and_paste(text: &str, target_app: Option<String>) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;
    simulate_paste(&target_app);
    Ok(())
}

#[cfg(target_os = "macos")]
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn simulate_paste(target_app: &Option<String>) {
    // Use System Events to activate by process name rather than
    // `tell application "<name>" to activate`, which requires the exact
    // application bundle name and fails for process names like "stable"
    // (e.g. Google Chrome Stable whose process name differs from its bundle name).
    let script = if let Some(app) = target_app {
        format!(
            "tell application \"System Events\"\n\
             set frontmost of process \"{}\" to true\n\
             end tell\n\
             delay 0.3\n\
             tell application \"System Events\" to keystroke \"v\" using command down",
            escape_applescript_string(app)
        )
    } else {
        String::from("tell application \"System Events\" to keystroke \"v\" using command down")
    };

    let output = Command::new("osascript").arg("-e").arg(&script).output();
    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("1002") || stderr.contains("not allowed to send keystrokes") {
                    // Surface accessibility permission error clearly
                    eprintln!(
                        "[TalkShow] Paste blocked — grant Accessibility permission to this \
                         app in System Settings → Privacy & Security → Accessibility. \
                         Error: {stderr}"
                    );
                } else {
                    eprintln!("[TalkShow] osascript failed: {stderr}");
                }
            }
        }
        Err(e) => {
            eprintln!("[TalkShow] Failed to spawn osascript: {}", e);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste(_target_app: &Option<String>) {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}
