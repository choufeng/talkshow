use std::sync::Mutex;

pub const MODE_TRANSCRIPTION: u8 = 1;
pub const MODE_TRANSLATION: u8 = 2;

#[derive(Clone, Debug)]
pub struct Session {
    pub id: u64,
    pub mode: u8,
    pub target_app: Option<String>,
}

struct Inner {
    active: Option<Session>,
    last_finished: Option<u64>,
    cancelled: Option<u64>,
    next_id: u64,
}

pub struct SessionManager {
    inner: Mutex<Inner>,
}

impl SessionManager {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                active: None,
                last_finished: None,
                cancelled: None,
                next_id: 1,
            }),
        }
    }

    /// 开始新会话。已有活动会话时返回 None(调用方决定是否忽略)。
    /// 开始新会话同时清除旧取消标志(新 pipeline 不继承旧取消)。
    pub fn start(&self, mode: u8) -> Option<Session> {
        let mut g = self.lock();
        if g.active.is_some() {
            return None;
        }
        let id = g.next_id;
        g.next_id += 1;
        let s = Session {
            id,
            mode,
            target_app: None,
        };
        g.active = Some(s.clone());
        g.cancelled = None;
        Some(s)
    }

    /// 结束当前会话并返回它(供后台清理 / pipeline 使用)。无活动会话返回 None。
    pub fn stop(&self) -> Option<Session> {
        let mut g = self.lock();
        let s = g.active.take();
        if let Some(s) = &s {
            g.last_finished = Some(s.id);
        }
        s
    }

    /// ESC / 指示器取消:
    /// - 录音中 → 结束会话并标记取消,返回 Some(session)(走 cancel pipeline)
    /// - 处理中(无活动会话)→ 标记最近结束的会话为取消,返回 None(pipeline 自行检查)
    pub fn signal_cancel(&self) -> Option<Session> {
        let mut g = self.lock();
        let Some(s) = g.active.take() else {
            // 处理中:标记最近结束的会话为取消(无历史则忽略)。
            if let Some(id) = g.last_finished {
                g.cancelled = Some(id);
            }
            return None;
        };
        g.cancelled = Some(s.id);
        g.last_finished = Some(s.id);
        Some(s)
    }

    /// 后台线程 checkpoint:该会话是否仍是当前活动会话。
    pub fn is_active(&self, id: u64) -> bool {
        self.lock().active.as_ref().is_some_and(|s| s.id == id)
    }

    pub fn has_active(&self) -> bool {
        self.lock().active.is_some()
    }

    /// pipeline 取消检查:该会话是否被标记取消。
    pub fn is_cancelled(&self, id: u64) -> bool {
        self.lock().cancelled == Some(id)
    }

    /// 后台线程写入粘贴目标应用(仅对当前活动会话生效,过期 id 静默丢弃)。
    pub fn set_target_app(&self, id: u64, app: String) {
        let mut g = self.lock();
        if let Some(ref mut s) = g.active
            && s.id == id
        {
            s.target_app = Some(app);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_returns_session_and_stop_returns_same() {
        let m = SessionManager::new();
        let a = m.start(MODE_TRANSCRIPTION).unwrap();
        let b = m.stop().unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(b.mode, MODE_TRANSCRIPTION);
    }

    #[test]
    fn ids_monotonically_increase() {
        let m = SessionManager::new();
        let a = m.start(MODE_TRANSLATION).unwrap();
        m.stop();
        let c = m.start(MODE_TRANSLATION).unwrap();
        assert!(c.id > a.id);
    }

    #[test]
    fn start_while_active_returns_none() {
        let m = SessionManager::new();
        assert!(m.start(MODE_TRANSCRIPTION).is_some());
        assert!(m.start(MODE_TRANSLATION).is_none());
    }

    #[test]
    fn stop_clears_active() {
        let m = SessionManager::new();
        m.start(MODE_TRANSLATION).unwrap();
        assert!(m.stop().is_some());
        assert!(!m.has_active());
        assert!(m.stop().is_none());
    }

    #[test]
    fn signal_cancel_during_recording_ends_session() {
        let m = SessionManager::new();
        let s = m.start(MODE_TRANSCRIPTION).unwrap();
        assert!(m.is_active(s.id));
        let cancelled = m.signal_cancel().unwrap();
        assert_eq!(cancelled.id, s.id);
        assert!(!m.has_active());
        assert!(m.is_cancelled(s.id));
    }

    #[test]
    fn signal_cancel_after_stop_marks_last_pipeline() {
        let m = SessionManager::new();
        let s = m.start(MODE_TRANSLATION).unwrap();
        m.stop();
        // 无活动会话:仅标记,返回 None
        assert!(m.signal_cancel().is_none());
        assert!(m.is_cancelled(s.id));
    }

    #[test]
    fn new_session_clears_cancel_flag() {
        let m = SessionManager::new();
        let s = m.start(MODE_TRANSLATION).unwrap();
        m.stop();
        m.signal_cancel();
        let n = m.start(MODE_TRANSLATION).unwrap();
        assert!(!m.is_cancelled(n.id));
        assert!(!m.is_cancelled(s.id));
    }

    #[test]
    fn signal_cancel_on_fresh_manager_returns_none() {
        let m = SessionManager::new();
        assert!(m.signal_cancel().is_none());
        assert!(!m.has_active());
    }

    #[test]
    fn target_app_scoped_to_active_session() {
        let m = SessionManager::new();
        let a = m.start(MODE_TRANSCRIPTION).unwrap();
        m.set_target_app(a.id, "Safari".into());
        let stopped = m.stop().unwrap();
        assert_eq!(stopped.target_app.as_deref(), Some("Safari"));

        // 过期 id 写入被丢弃;错 id 写入被丢弃
        m.set_target_app(a.id, "stale".into());
        let b = m.start(MODE_TRANSLATION).unwrap();
        m.set_target_app(a.id, "wrong-id".into());
        let stopped_b = m.stop().unwrap();
        assert_eq!(stopped_b.target_app, None);
        assert!(b.id != a.id);
    }
}
