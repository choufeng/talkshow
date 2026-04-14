use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{Mutex, RwLock};
use std::time::Instant;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub const RECORDING_MODE_NONE: u8 = 0;
pub const RECORDING_MODE_TRANSCRIPTION: u8 = 1;
pub const RECORDING_MODE_TRANSLATION: u8 = 2;

pub static RECORDING: AtomicU8 = AtomicU8::new(RECORDING_MODE_NONE);
pub static CANCELLED: AtomicBool = AtomicBool::new(false);
pub static LAST_REC_PRESS: Mutex<Option<Instant>> = Mutex::new(None);

pub struct ShortcutIds {
    pub toggle: u32,
    pub recording: u32,
    pub translate: u32,
}

pub static SHORTCUT_IDS: RwLock<ShortcutIds> = RwLock::new(ShortcutIds {
    toggle: 0,
    recording: 0,
    translate: 0,
});

pub fn parse_shortcut(shortcut_str: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in &parts {
        match *part {
            "Control" => modifiers |= Modifiers::CONTROL,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Alt" => modifiers |= Modifiers::ALT,
            "Command" | "Super" => modifiers |= Modifiers::SUPER,
            s => {
                if let Ok(code) = s.parse::<Code>() {
                    key_code = Some(code);
                }
            }
        }
    }

    key_code.map(|code| Shortcut::new(Some(modifiers), code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_control_shift_quote() {
        let result = parse_shortcut("Control+Shift+Quote");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_control_backslash() {
        let result = parse_shortcut("Control+Backslash");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_single_key() {
        let result = parse_shortcut("KeyA");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_shortcut_empty_string() {
        let result = parse_shortcut("");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_shortcut_only_modifiers() {
        let result = parse_shortcut("Control+Shift");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_shortcut_command_super_alias() {
        let cmd_result = parse_shortcut("Command+KeyA");
        let super_result = parse_shortcut("Super+KeyA");
        assert_eq!(cmd_result.is_some(), super_result.is_some());
    }

    #[test]
    fn test_recording_mode_constants() {
        assert_eq!(RECORDING_MODE_NONE, 0);
        assert_eq!(RECORDING_MODE_TRANSCRIPTION, 1);
        assert_eq!(RECORDING_MODE_TRANSLATION, 2);
    }
}
