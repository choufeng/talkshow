use std::sync::{Mutex, RwLock};
use std::time::Instant;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

pub static LAST_REC_PRESS: Mutex<Option<Instant>> = Mutex::new(None);

pub struct ShortcutIds {
    pub toggle: Vec<u32>,
    pub recording: Vec<u32>,
    pub translate: Vec<u32>,
}

pub static SHORTCUT_IDS: RwLock<ShortcutIds> = RwLock::new(ShortcutIds {
    toggle: Vec::new(),
    recording: Vec::new(),
    translate: Vec::new(),
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

/// 解析逗号分隔的多组快捷键: "Control+Shift+Quote,Alt+Q" → 两组 Shortcut
pub fn parse_shortcuts(shortcut_str: &str) -> Vec<Shortcut> {
    shortcut_str
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(parse_shortcut)
        .collect()
}

pub fn shortcut_ids(shortcuts: &[Shortcut]) -> Vec<u32> {
    shortcuts.iter().map(|s| s.id()).collect()
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
    fn test_parse_shortcuts_multiple() {
        let result = parse_shortcuts("Control+Shift+Quote, Alt+KeyQ");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_shortcuts_single_no_comma() {
        let result = parse_shortcuts("Control+Backslash");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_shortcuts_skips_invalid_and_empty() {
        let result = parse_shortcuts("Control+Shift, , Control+KeyA");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_shortcuts_empty() {
        assert!(parse_shortcuts("").is_empty());
    }
}
