#[cfg(target_os = "macos")]
use objc2_app_kit::{NSWindow, NSWindowStyleMask};
use tauri::WebviewWindow;

pub fn make_window_nonactivating(window: &WebviewWindow) -> Result<(), String> {
    let ns_window_ptr = window
        .ns_window()
        .map_err(|e| format!("Failed to get NSWindow handle: {e}"))?;

    // SAFETY: ns_window_ptr is obtained from tauri::WebviewWindow's NSWindow handle,
    // which guarantees the window pointer is valid for the lifetime of the WebviewWindow.
    // We borrow the window immutably and only access its styleMask for reading and writing,
    // which is safe because we're not modifying the window's core state beyond style attributes.
    let ns_window: &NSWindow = unsafe { &*ns_window_ptr.cast() };

    let style = ns_window.styleMask();
    let new_style = style | NSWindowStyleMask::NonactivatingPanel;
    ns_window.setStyleMask(new_style);

    Ok(())
}
