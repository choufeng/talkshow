use std::sync::Mutex;

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
    if let Some(app) = target_app {
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"{}\" to activate",
                escape_applescript_string(app)
            ))
            .output();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .output();
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste(_target_app: &Option<String>) {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}
