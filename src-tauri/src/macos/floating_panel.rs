#[cfg(target_os = "macos")]
pub fn make_window_nonactivating(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}
