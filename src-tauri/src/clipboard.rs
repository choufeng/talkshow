use std::process::Command;
use std::sync::Mutex;
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
    let script = if let Some(app) = target_app {
        format!(
            "tell application \"{}\" to activate\ndelay 0.3\ntell application \"System Events\" to keystroke \"v\" using command down",
            escape_applescript_string(app)
        )
    } else {
        String::from("tell application \"System Events\" to keystroke \"v\" using command down")
    };

    match Command::new("osascript").arg("-e").arg(&script).spawn() {
        Ok(mut child) => {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            eprintln!(
                                "[TalkShow] osascript timed out, killing process (pid: {:?})",
                                child.id()
                            );
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        eprintln!("[TalkShow] osascript poll error: {}", e);
                        break;
                    }
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
