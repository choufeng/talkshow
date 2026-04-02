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

#[tauri::command]
pub fn get_replace_mode_state() -> serde_json::Value {
    let text = SELECTED_TEXT.lock().ok().and_then(|g| g.clone());
    serde_json::json!({
        "replaceMode": text.is_some(),
        "selectedPreview": text.map(|t| t.chars().take(50).collect::<String>()).unwrap_or_default()
    })
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
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn simulate_paste() {
    let target_app = TARGET_APP.lock().ok().and_then(|g| g.clone());
    if let Some(app) = target_app {
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"{}\" to activate",
                escape_applescript_string(&app)
            ))
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
pub fn detect_selected_text(_app_name: &str) -> Option<String> {
    let original_clipboard = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok());

    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"c\" using command down")
        .output();

    std::thread::sleep(std::time::Duration::from_millis(50));

    let copied = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok());

    if let Some(ref original) = original_clipboard
        && let Ok(mut cb) = arboard::Clipboard::new()
    {
        let _ = cb.set_text(original);
    }

    match (original_clipboard, copied) {
        (Some(ref orig), Some(ref new)) if orig != new => Some(new.clone()),
        (None, Some(new)) if !new.is_empty() => Some(new),
        _ => None,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn detect_selected_text(_app_name: &str) -> Option<String> {
    None
}
