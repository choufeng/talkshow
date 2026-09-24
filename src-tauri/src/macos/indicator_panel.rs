//! 录音指示条:原生 NSPanel 实现。
//!
//! 与旧 Tauri WebviewWindow 方案的核心差异:自定义 contentView 重写
//! `hitTest:`,胶囊圆角矩形之外的区域返回 nil → 鼠标事件穿透到下层窗口,
//! 不再出现"透明区域挡住下层内容点击"的问题。
//!
//! 线程模型:PANEL 全局仅存指针(始终泄漏,窗口常驻与进程同生命周期),
//! 所有 UI 操作经 `AppHandle::run_on_main_thread` 派发到主线程执行。

// objc2-app-kit 0.3.x 各小版本对方法 safe/unsafe 标注不一致,不逐个追
#![allow(unused_unsafe)]

use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::runtime::Sel;
use objc2::{define_class, msg_send, sel, DeclaredClass, MainThreadMarker};
use objc2_app_kit::{
    NSBackingStoreType, NSBezierPath, NSButton, NSColor, NSFont, NSPanel, NSScreen, NSTextField,
    NSView, NSWindowCollectionBehavior, NSWindowStyleMask, NSFloatingWindowLevel,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSTimer};
use tauri::{AppHandle, Emitter};

/// 面板高(逻辑点)
const PANEL_H: f64 = 48.0;
/// 顶部边距,避开菜单栏/刘海
const TOP_MARGIN: f64 = 32.0;
/// 胶囊圆角(半高 = 全圆角)
const CORNER: f64 = 24.0;
/// paste-failed 提示自动隐藏时长
const PASTE_FAILED_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Recording,
    Processing,
    PasteFailed,
}

/// 每次相位切换自增;延迟隐藏闭包执行时比对,防止误藏新会话面板
static SEQ: AtomicU64 = AtomicU64::new(0);

/// 取消按钮点击时取 AppHandle(面板单实例,创建时写入)
static CANCEL_TARGET: Mutex<Option<AppHandle>> = Mutex::new(None);

struct IVars {
    dot: Retained<NSTextField>,
    timer_label: Retained<NSTextField>,
    status_label: Retained<NSTextField>,
    cancel_btn: Retained<NSButton>,
    timer: RefCell<Option<Retained<NSTimer>>>,
    start: RefCell<Option<Instant>>,
}

define_class! {
    #[unsafe(super(NSView))]
    #[ivars = IVars]
    #[name = "TalkShowIndicatorView"]
    struct IndicatorView;

    impl IndicatorView {
        /// 透明区域鼠标穿透:圆角胶囊内命中(按钮优先),之外返回 nil
        #[unsafe(method(hitTest:))]
        unsafe fn hit_test(&self, point: NSPoint) -> *mut NSView {
            let frame = unsafe { self.frame() };
            let local = NSPoint::new(point.x - frame.origin.x, point.y - frame.origin.y);
            let bounds = unsafe { self.bounds() };
            let in_capsule = local.x >= 2.0
                && local.x <= bounds.size.width - 2.0
                && local.y >= 2.0
                && local.y <= bounds.size.height - 2.0;
            if !in_capsule {
                return std::ptr::null_mut();
            }
            // 取消按钮优先
            let btn = &self.ivars().cancel_btn;
            let bf = unsafe { btn.frame() };
            if bf.origin.x <= local.x
                && local.x <= bf.origin.x + bf.size.width
                && bf.origin.y <= local.y
                && local.y <= bf.origin.y + bf.size.height
            {
                return Retained::as_ptr(btn) as *mut NSView;
            }
            self as *const Self as *mut NSView
        }

        #[unsafe(method(drawRect:))]
        unsafe fn draw_rect(&self, _dirty: NSRect) {
            let bounds = unsafe { self.bounds() };
            let path = unsafe {
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, CORNER, CORNER)
            };
            let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.07, 0.07, 0.10, 0.92);
            bg.set();
            path.fill();
            let border = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.10);
            border.set();
            path.setLineWidth(1.0);
            path.stroke();
        }

        #[unsafe(method(cancelClicked:))]
        fn cancel_clicked(&self, _sender: Option<&AnyObject>) {
            // 走 Tauri 事件,由 lib.rs 的 listener 执行与 ESC 等价的取消逻辑
            let handle = CANCEL_TARGET.lock().ok().and_then(|g| g.clone());
            if let Some(h) = handle {
                let _ = h.emit("indicator:cancel", ());
            }
        }

        #[unsafe(method(tick:))]
        fn tick(&self, _timer: *mut NSTimer) {
            let Some(start) = *self.ivars().start.borrow() else {
                return;
            };
            let secs = start.elapsed().as_secs();
            let text = format!("{}:{:02}", secs / 60, secs % 60);
            unsafe {
                self.ivars()
                    .timer_label
                    .setStringValue(&NSString::from_str(&text));
            }
        }
    }
}

impl IndicatorView {
    fn stop_timer(&self) {
        if let Some(t) = self.ivars().timer.borrow_mut().take() {
            t.invalidate();
        }
        *self.ivars().start.borrow_mut() = None;
    }

    fn start_timer(&self, start: Instant) {
        self.stop_timer();
        *self.ivars().start.borrow_mut() = Some(start);
        let target: &AnyObject = unsafe { &*(self as *const Self as *const AnyObject) };
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                1.0,
                target,
                sel!(tick:),
                None,
                true,
            )
        };
        *self.ivars().timer.borrow_mut() = Some(timer);
        // 立即刷新一次,不等首个 1s(宏内方法不可直接调用,走 selector)
        let _: () = unsafe { msg_send![self, tick: std::ptr::null_mut::<NSTimer>()] };
    }
}

struct PanelPtrs {
    panel: *mut NSPanel,
    view: *mut IndicatorView,
}
// SAFETY: 指针只在 run_on_main_thread 闭包内解引用;窗口常驻永不析构
unsafe impl Send for PanelPtrs {}

static PANEL: Mutex<Option<PanelPtrs>> = Mutex::new(None);

fn make_label(
    mtm: MainThreadMarker,
    text: &str,
    size: f64,
    mono: bool,
    color: (f64, f64, f64, f64),
    frame: NSRect,
) -> Retained<NSTextField> {
    let label = unsafe { NSTextField::labelWithString(&NSString::from_str(text), mtm) };
    let font = if mono {
        unsafe { NSFont::monospacedDigitSystemFontOfSize_weight(size, 0.5) }
    } else {
        unsafe { NSFont::systemFontOfSize(size) }
    };
    unsafe {
        label.setFont(Some(&font));
        let c = NSColor::colorWithSRGBRed_green_blue_alpha(color.0, color.1, color.2, color.3);
        label.setTextColor(Some(&c));
        label.setFrame(frame);
    }
    label
}

/// 按相位重排子控件 + 调整面板宽度并重新定位(主屏顶部居中)
fn layout(view: &IndicatorView, panel: &NSPanel, phase: Phase, mtm: MainThreadMarker) {
    let iv = view.ivars();
    let width = match phase {
        Phase::Recording => 180.0,
        Phase::Processing => 150.0,
        Phase::PasteFailed => 250.0,
    };

    let (dot_f, timer_f, status_f, btn_f) = match phase {
        Phase::Recording => (
            NSRect::new(NSPoint::new(14.0, 15.0), NSSize::new(16.0, 18.0)),
            NSRect::new(NSPoint::new(32.0, 15.0), NSSize::new(50.0, 18.0)),
            NSRect::ZERO,
            NSRect::new(NSPoint::new(146.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
        Phase::Processing => (
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::new(NSPoint::new(14.0, 15.0), NSSize::new(96.0, 18.0)),
            NSRect::new(NSPoint::new(118.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
        Phase::PasteFailed => (
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::new(NSPoint::new(14.0, 15.0), NSSize::new(200.0, 18.0)),
            NSRect::new(NSPoint::new(218.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
    };

    unsafe {
        iv.dot.setFrame(dot_f);
        iv.dot.setHidden(phase != Phase::Recording);
        iv.timer_label.setFrame(timer_f);
        iv.timer_label.setHidden(phase != Phase::Recording);
        iv.status_label.setFrame(status_f);
        iv.status_label.setHidden(phase == Phase::Recording);
        iv.cancel_btn.setFrame(btn_f);
    }

    unsafe {
        panel.setContentSize(NSSize::new(width, PANEL_H));
    }

    // 主屏顶部居中(AppKit y 轴向上)
    if let Some(screen) = unsafe { NSScreen::mainScreen(mtm) } {
        let f = unsafe { screen.frame() };
        let x = f.origin.x + (f.size.width - width) / 2.0;
        let y = f.origin.y + f.size.height - TOP_MARGIN - PANEL_H;
        unsafe {
            panel.setFrameTopLeftPoint(NSPoint::new(x, y));
        }
    }
}

/// 惰性创建面板(必须主线程调用)。返回 (panel, view)。
fn ensure_panel(
    app: &AppHandle,
    mtm: MainThreadMarker,
) -> Option<(Retained<NSPanel>, Retained<IndicatorView>)> {
    if let Some(p) = PANEL.lock().ok()?.as_ref() {
        // SAFETY: 仅主线程访问;指针由创建路径构造且常驻
        unsafe {
            let panel = Retained::retain(p.panel)?;
            let view = Retained::retain(p.view)?;
            return Some((panel, view));
        }
    }

    // --- 子控件 ---
    let dot = make_label(
        mtm,
        "●",
        12.0,
        false,
        (1.0, 0.37, 0.34, 1.0), // #ff5f57
        NSRect::ZERO,
    );
    let timer_label = make_label(mtm, "0:00", 13.0, true, (1.0, 1.0, 1.0, 0.9), NSRect::ZERO);
    let status_label = make_label(
        mtm,
        "处理中…",
        12.0,
        true,
        (0.0, 0.78, 1.0, 0.95), // 青
        NSRect::ZERO,
    );

    let cancel_btn = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("✕"),
            None,
            Some(sel!(cancelClicked:) as Sel),
            mtm,
        )
    };
    unsafe {
        cancel_btn.setBordered(false);
        let c = NSColor::colorWithSRGBRed_green_blue_alpha(0.55, 0.6, 0.66, 1.0);
        cancel_btn.setContentTintColor(Some(&c));
        let font = NSFont::systemFontOfSize(11.0);
        cancel_btn.setFont(Some(&font));
    }

    // --- contentView(先 set_ivars 再经 super init) ---
    let content_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, PANEL_H));
    let this = mtm.alloc().set_ivars(IVars {
        dot,
        timer_label,
        status_label,
        cancel_btn,
        timer: RefCell::new(None),
        start: RefCell::new(None),
    });
    let view: Retained<IndicatorView> =
        unsafe { msg_send![super(this), initWithFrame: content_rect] };

    // --- 面板 ---
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let panel = unsafe {
        NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            content_rect,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        panel.setLevel(NSFloatingWindowLevel);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setReleasedWhenClosed(false);
        panel.setContentView(Some(&view));
    }
    // 取消按钮 target 指向 contentView(NSControl 对 target 是 weak 引用,无环)
    {
        let btn = &view.ivars().cancel_btn;
        let target: &AnyObject = unsafe { &*(Retained::as_ptr(btn) as *const AnyObject) };
        unsafe { btn.setTarget(Some(target)) };
    }

    if let Ok(mut g) = PANEL.lock() {
        *g = Some(PanelPtrs {
            panel: Retained::into_raw(panel.clone()),
            view: Retained::into_raw(view.clone()),
        });
    }
    if let Ok(mut g) = CANCEL_TARGET.lock() {
        *g = Some(app.clone());
    }

    Some((panel, view))
}

fn set_phase(app: &AppHandle, phase: Phase) -> u64 {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some((panel, view)) = ensure_panel(&h, mtm) else {
            return;
        };
        if phase == Phase::Recording {
            view.start_timer(Instant::now());
        } else {
            view.stop_timer();
        }
        if phase != Phase::Recording {
            let text = match phase {
                Phase::Processing => "处理中…",
                _ => "粘贴失败，请手动粘贴",
            };
            let color = match phase {
                Phase::Processing => (0.0, 0.78, 1.0, 0.95),
                _ => (1.0, 0.69, 0.13, 0.95), // 橙
            };
            unsafe {
                view.ivars()
                    .status_label
                    .setStringValue(&NSString::from_str(text));
                let c =
                    NSColor::colorWithSRGBRed_green_blue_alpha(color.0, color.1, color.2, color.3);
                view.ivars().status_label.setTextColor(Some(&c));
            }
        }
        layout(&view, &panel, phase, mtm);
        unsafe {
            panel.orderFrontRegardless();
        }
    });
    seq
}

pub fn show_recording(app: &AppHandle) {
    set_phase(app, Phase::Recording);
}

pub fn show_processing(app: &AppHandle) {
    set_phase(app, Phase::Processing);
}

pub fn show_paste_failed(app: &AppHandle) {
    let seq = set_phase(app, Phase::PasteFailed);
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(PASTE_FAILED_TIMEOUT);
        let _ = h.run_on_main_thread(move || {
            if SEQ.load(Ordering::SeqCst) != seq {
                return; // 期间已有新相位/新会话,不隐藏
            }
            hide_sync();
        });
    });
}

pub fn hide(app: &AppHandle) {
    SEQ.fetch_add(1, Ordering::SeqCst);
    let _ = app.run_on_main_thread(hide_sync);
}

fn hide_sync() {
    let Ok(guard) = PANEL.lock() else { return };
    let Some(p) = guard.as_ref() else { return };
    // SAFETY: 仅主线程访问,常驻指针
    unsafe {
        (*p.view).stop_timer();
        (*p.panel).orderOut(None);
    }
}
