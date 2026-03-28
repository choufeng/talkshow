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
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() {
    eprintln!("[TalkShow] Paste simulation not supported on this platform");
}
