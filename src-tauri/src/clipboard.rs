use std::sync::Mutex;

static TARGET_APP: Mutex<Option<String>> = Mutex::new(None);
static SELECTED_TEXT: Mutex<Option<String>> = Mutex::new(None);

pub fn save_target_app(app_name: &str) {
    if let Ok(mut guard) = TARGET_APP.lock() {
        *guard = Some(app_name.to_string());
    }
}

pub fn save_selected_text(text: &str) {
    if let Ok(mut guard) = SELECTED_TEXT.lock() {
        *guard = Some(text.to_string());
    }
}

pub fn get_saved_selected_text() -> Option<String> {
    SELECTED_TEXT.lock().ok().and_then(|g| g.clone())
}

pub fn clear_selected_text() {
    if let Ok(mut guard) = SELECTED_TEXT.lock() {
        *guard = None;
    }
}

pub fn write_and_paste(text: &str) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;
    simulate_paste();
    Ok(())
}

#[cfg(target_os = "macos")]
fn simulate_paste() {
    let target_app = TARGET_APP.lock().ok().and_then(|g| g.clone());
    if let Some(app) = target_app {
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("tell application \"{}\" to activate", app))
            .output();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .output();
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}

#[cfg(target_os = "macos")]
pub fn detect_selected_text(app_name: &str) -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "tell application \"{}\" to get selection",
            app_name
        ))
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }

    Some(text)
}

#[cfg(not(target_os = "macos"))]
pub fn detect_selected_text(_app_name: &str) -> Option<String> {
    None
}
