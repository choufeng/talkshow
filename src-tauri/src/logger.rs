use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_LOG_FILES: usize = 10;
const LOG_DIR_NAME: &str = "logs";

#[derive(Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub ts: String,
    pub module: String,
    pub level: String,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct LogSession {
    pub filename: String,
    pub size_bytes: u64,
    pub entry_count: usize,
    pub first_ts: Option<String>,
    pub is_current: bool,
}

pub struct Logger {
    log_dir: PathBuf,
    current_file: Mutex<fs::File>,
    current_filename: String,
}

impl Logger {
    pub fn new(app_data_dir: &std::path::Path) -> Result<Self, String> {
        let log_dir = app_data_dir.join(LOG_DIR_NAME);
        fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;

        let now = Local::now();
        let filename = format!("talkshow-{}.jsonl", now.format("%Y-%m-%d_%H-%M-%S"));
        let filepath = log_dir.join(&filename);
        let file = fs::File::options()
            .create_new(true)
            .append(true)
            .open(&filepath)
            .map_err(|e| format!("Failed to create log file: {}", e))?;

        cleanup_old_logs(&log_dir, MAX_LOG_FILES);

        Ok(Self {
            log_dir,
            current_file: Mutex::new(file),
            current_filename: filename,
        })
    }

    pub fn log(&self, module: &str, level: &str, msg: &str, meta: Option<serde_json::Value>) {
        let entry = LogEntry {
            ts: Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            module: module.to_string(),
            level: level.to_string(),
            msg: msg.to_string(),
            meta,
        };

        let mut line = match serde_json::to_string(&entry) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[TalkShow] Failed to serialize log entry: {}", e);
                return;
            }
        };
        line.push('\n');

        if let Ok(mut file) = self.current_file.lock() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }

    pub fn info(&self, module: &str, msg: &str, meta: Option<serde_json::Value>) {
        self.log(module, "info", msg, meta);
    }

    pub fn warn(&self, module: &str, msg: &str, meta: Option<serde_json::Value>) {
        self.log(module, "warn", msg, meta);
    }

    pub fn error(&self, module: &str, msg: &str, meta: Option<serde_json::Value>) {
        self.log(module, "error", msg, meta);
    }

    pub fn get_sessions(&self) -> Vec<LogSession> {
        let mut sessions: Vec<LogSession> = Vec::new();

        let entries = match fs::read_dir(&self.log_dir) {
            Ok(e) => e,
            Err(_) => return sessions,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let content = fs::read_to_string(&path).unwrap_or_default();
            let entry_count = content.lines().filter(|l| !l.is_empty()).count();

            let first_ts = content
                .lines()
                .next()
                .and_then(|line| serde_json::from_str::<LogEntry>(line).ok())
                .map(|e| e.ts);

            let is_current = filename == self.current_filename;

            sessions.push(LogSession {
                filename,
                size_bytes: metadata.len(),
                entry_count,
                first_ts,
                is_current,
            });
        }

        sessions.sort_by(|a, b| b.filename.cmp(&a.filename));
        sessions
    }

    pub fn get_content(&self, session_file: Option<&str>) -> Vec<LogEntry> {
        let filename = session_file.unwrap_or(&self.current_filename);
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Vec::new();
        }
        let filepath = self.log_dir.join(filename);

        let content = match fs::read_to_string(filepath) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| serde_json::from_str::<LogEntry>(line).ok())
            .collect()
    }
}

fn cleanup_old_logs(log_dir: &std::path::Path, max_files: usize) {
    let mut files: Vec<(String, std::time::SystemTime)> = Vec::new();

    let entries = match fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
        if let Some(modified) = modified {
            files.push((filename, modified));
        }
    }

    files.sort_by(|a, b| b.1.cmp(&a.1));

    if files.len() > max_files {
        for (_, filename) in files.iter().skip(max_files) {
            let _ = fs::remove_file(log_dir.join(filename));
        }
    }
}

#[tauri::command]
fn get_log_sessions(app_handle: tauri::AppHandle) -> Vec<LogSession> {
    let logger = app_handle.state::<Logger>();
    logger.get_sessions()
}

#[tauri::command]
fn get_log_content(app_handle: tauri::AppHandle, session_file: Option<String>) -> Vec<LogEntry> {
    let logger = app_handle.state::<Logger>();
    logger.get_content(session_file.as_deref())
}
