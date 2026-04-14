use objc2::runtime::AnyObject;
use objc2_app_kit::{NSWindow, NSWindowStyleMask};
use tauri::WebviewWindow;

pub fn make_window_nonactivating(window: &WebviewWindow) -> Result<(), String> {
    let ns_window_ptr = window
        .ns_window()
        .map_err(|e| format!("Failed to get NSWindow handle: {e}"))?;

    let ns_window: &NSWindow = unsafe { &*ns_window_ptr.cast() };

    let style = ns_window.styleMask();
    let new_style = style | NSWindowStyleMask::NonactivatingPanel;
    ns_window.setStyleMask(new_style);

    Ok(())
}

pub fn show_without_activating(window: &WebviewWindow) -> Result<(), String> {
    let ns_window_ptr = window
        .ns_window()
        .map_err(|e| format!("Failed to get NSWindow handle: {e}"))?;

    let ns_window: &NSWindow = unsafe { &*ns_window_ptr.cast() };
    ns_window.orderFront(None::<&AnyObject>);

    Ok(())
}
