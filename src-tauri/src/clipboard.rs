use std::sync::Mutex;

static TARGET_APP: Mutex<Option<String>> = Mutex::new(None);

pub fn save_target_app(app_name: &str) {
    if let Ok(mut guard) = TARGET_APP.lock() {
        *guard = Some(app_name.to_string());
    }
}

// pipeline 已改读 session.target_app;lib.rs 录音线程接线(Task 5)后删除。
#[allow(dead_code)]
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
    //
    // `delay 0.3`:等待前台切换完成。串行队列中延迟可接受,
    // 替代品是并发执行 AppleEvents 导致的死锁。
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

    match crate::sys::osascript(&script) {
        Ok(_) => {}
        Err(stderr) => {
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
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste(_target_app: &Option<String>) {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}
