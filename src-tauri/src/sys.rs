//! macOS 系统交互收口层。
//!
//! macOS 的 AppleEvents 全局串行:多个 osascript 子进程并发执行会互相
//! 阻塞(历史上造成多秒死锁)。所有 osascript 调用必须走本模块的专用
//! 串行线程,保证任一时刻至多一个 osascript 在执行。
//!
//! 所有函数都会阻塞调用线程直至结果返回 —— 只在后台线程调用。

// 模块尚未接线(后续任务迁移 audio_control/clipboard/skills/lib.rs 调用点),
// 挂载期先豁免 dead_code,接线后移除。
#![allow(dead_code)]

use std::sync::OnceLock;
use std::sync::mpsc;

struct Request {
    script: String,
    reply: mpsc::Sender<Result<String, String>>,
}

static QUEUE: OnceLock<mpsc::Sender<Request>> = OnceLock::new();

fn queue() -> mpsc::Sender<Request> {
    QUEUE
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel::<Request>();
            std::thread::Builder::new()
                .name("talkshow-osascript".into())
                .spawn(move || {
                    for req in rx {
                        let result = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(&req.script)
                            .output()
                            .map_err(|e| e.to_string())
                            .and_then(|o| {
                                if o.status.success() {
                                    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
                                } else {
                                    Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                                }
                            });
                        let _ = req.reply.send(result);
                    }
                })
                .expect("failed to spawn osascript worker thread");
            tx
        })
        .clone()
}

/// 串行执行一段 AppleScript,返回 stdout(trim 后)。
pub fn osascript(script: &str) -> Result<String, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    queue()
        .send(Request {
            script: script.to_string(),
            reply: reply_tx,
        })
        .map_err(|e| format!("osascript queue unavailable: {e}"))?;
    reply_rx
        .recv()
        .map_err(|e| format!("osascript worker died: {e}"))?
}

/// 当前最前台应用进程名(如 "Google Chrome")。
pub fn frontmost_app_name() -> Option<String> {
    osascript(
        "tell application \"System Events\" to get name of first process whose frontmost is true",
    )
    .ok()
    .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn osascript_evaluates_expression() {
        assert_eq!(osascript("return 1 + 1").unwrap(), "2");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn osascript_reports_script_error() {
        assert!(osascript("syntax error here").is_err());
    }

    #[test]
    fn frontmost_app_name_is_plausible() {
        if cfg!(target_os = "macos") {
            let name = frontmost_app_name();
            // 非前台无权限时可能为 None,但不能 panic
            if let Some(n) = name {
                assert!(!n.is_empty());
            }
        }
    }
}
