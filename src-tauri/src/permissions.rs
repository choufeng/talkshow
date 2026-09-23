//! TCC 权限检测与修复。
//!
//! 背景:app 为 ad-hoc 签名,每个新构建 cdhash 不同。TCC 数据库按代码签名
//! requirement 校验 → 新版本安装后旧授权失效,但系统设置 UI 仍显示"已授权"
//! (伪授权)。唯一可信信号是 `AXIsProcessTrusted()` 的实测值。
//!
//! 修复流程:实测失败 → `tccutil reset`(清掉伪授权条目,否则用户看到
//! 已勾选不会重新操作)→ 引导重新授权 → 探测 Automation 权限。

const BUNDLE_ID: &str = "com.jiaxia.talkshow";

/// 辅助功能权限实测状态(含代码签名校验,伪授权时返回 false)。
#[cfg(target_os = "macos")]
pub fn is_trusted() -> bool {
    // kAXTrustedCheckOptionPrompt 不传:不弹系统引导,由前端 UI 引导
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    true
}

/// 重置本 app 全部 TCC 条目(Accessibility + AppleEvents + Microphone 等)。
/// 清掉旧签名的伪授权条目,让用户重新授权新版本。
pub fn reset_tcc() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("tccutil")
            .args(["reset", "All", BUNDLE_ID])
            .output()
            .map_err(|e| format!("failed to run tccutil: {e}"))?;
        let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(format!("tccutil failed: {err}"));
        }
        Ok(msg)
    }
    #[cfg(not(target_os = "macos"))]
    Ok(String::new())
}

/// 打开系统设置的辅助功能/自动化面板。
pub fn open_accessibility_settings(automation: bool) {
    #[cfg(target_os = "macos")]
    {
        let pane = if automation {
            "Privacy_Automation"
        } else {
            "Privacy_Accessibility"
        };
        let _ = std::process::Command::new("open")
            .arg(format!(
                "x-apple.systempreferences:com.apple.preference.security?{pane}"
            ))
            .spawn();
    }
}

/// 探测向 System Events 发送 AppleEvent 的自动化权限。
/// 首次调用会触发系统授权弹窗;被拒绝(-1743)返回 Err。
#[cfg(target_os = "macos")]
pub fn probe_automation() -> Result<(), String> {
    crate::sys::osascript("tell application \"System Events\" to get name of first process")
        .map(|_| ())
        .map_err(|stderr| {
            if stderr.contains("-1743") {
                "automation-denied".to_string()
            } else {
                stderr
            }
        })
}

#[cfg(not(target_os = "macos"))]
pub fn probe_automation() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    // is_trusted 在 CI(无 GUI)环境下无意义,不测;FFI 声明由编译器校验。
    #[test]
    fn bundle_id_matches_tauri_config() {
        let conf = include_str!("../tauri.conf.json");
        assert!(conf.contains("\"identifier\": \"com.jiaxia.talkshow\""));
    }
}
