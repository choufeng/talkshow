//! 录音指示条:原生 NSPanel 实现。
//!
//! 与旧 Tauri WebviewWindow 方案的核心差异:自定义 contentView 重写
//! `hitTest:`,胶囊圆角矩形之外的区域返回 nil → 鼠标事件穿透到下层窗口,
//! 不再出现"透明区域挡住下层内容点击"的问题。
//!
//! 动态效果:layer opacity 呼吸(NSTimer 60fps 插值,GPU 合成,不重绘)。
//! 原版水母 SVG path 形变不复刻,以青色脉冲光点等效。
//!
//! 线程模型:PANEL 全局仅存指针(始终泄漏,窗口常驻与进程同生命周期),
//! 所有 UI 操作经 `AppHandle::run_on_main_thread` 派发到主线程执行。

// objc2-app-kit 各小版本对方法 safe/unsafe 标注不一致,不逐个追
#![allow(unused_unsafe)]

use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::runtime::Sel;
use objc2::{define_class, msg_send, sel, DeclaredClass, MainThreadMarker};
use objc2_app_kit::{
    NSBackingStoreType, NSBezierPath, NSButton, NSColor, NSFont, NSLineCapStyle, NSPanel,
    NSScreen, NSTextField, NSView, NSWindowCollectionBehavior, NSWindowStyleMask,
    NSFloatingWindowLevel,
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

/// 呼吸动画帧间隔(60fps)
const BREATH_INTERVAL: f64 = 1.0 / 60.0;

const RED: (f64, f64, f64, f64) = (1.0, 0.0, 0.33, 1.0); // #ff0055
const CYAN: (f64, f64, f64, f64) = (0.0, 0.78, 1.0, 1.0); // #00c8ff
const AMBER: (f64, f64, f64, f64) = (1.0, 0.69, 0.13, 1.0); // #ffb020

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

// ============ RingView:状态图标(环/点/警告) ============

#[derive(Clone, Copy)]
enum RingStyle {
    /// 圆环 + 中心点(录音)
    Ring,
    /// 实心圆(处理中光点)
    Dot,
    /// 圆圈 + 感叹号(警告)
    Warn,
}

impl PartialEq for RingStyle {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

struct RingStyleState {
    style: Cell<RingStyle>,
    color: Cell<(f64, f64, f64, f64)>,
}

define_class! {
    #[unsafe(super(NSView))]
    #[ivars = RingStyleState]
    #[name = "TalkShowRingView"]
    struct RingView;

    impl RingView {
        #[unsafe(method(drawRect:))]
        unsafe fn draw_rect(&self, _dirty: NSRect) {
            let bounds = unsafe { self.bounds() };
            let iv = self.ivars();
            let (r, g, b, a) = iv.color.get();
            let c = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a);
            c.set();
            let style = iv.style.get();
            if style == RingStyle::Dot {
                let path = unsafe {
                    NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                        NSPoint::new(2.0, 2.0),
                        NSSize::new(bounds.size.width - 4.0, bounds.size.height - 4.0),
                    ))
                };
                path.fill();
            } else if style == RingStyle::Ring {
                // 外环(原版 r 7→9 呼吸由外层 opacity 动画近似)
                let ring = unsafe {
                    NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                        NSPoint::new(2.0, 2.0),
                        NSSize::new(bounds.size.width - 4.0, bounds.size.height - 4.0),
                    ))
                };
                ring.setLineWidth(1.5);
                ring.stroke();
                // 中心实心点
                let dot = unsafe {
                    NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                        NSPoint::new(
                            bounds.size.width / 2.0 - 3.0,
                            bounds.size.height / 2.0 - 3.0,
                        ),
                        NSSize::new(6.0, 6.0),
                    ))
                };
                dot.fill();
            } else {
                // Warn:圆圈 + 感叹号
                let circle = unsafe {
                    NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                        NSPoint::new(2.0, 2.0),
                        NSSize::new(bounds.size.width - 4.0, bounds.size.height - 4.0),
                    ))
                };
                circle.setLineWidth(1.5);
                circle.stroke();
                // 感叹号竖线(y 轴向上)+ 点
                let mark = unsafe { NSBezierPath::bezierPath() };
                unsafe {
                    mark.setLineWidth(1.5);
                    mark.setLineCapStyle(NSLineCapStyle::Round);
                    // 感叹号竖线在上段,点在下段(y 轴向上)
                    mark.moveToPoint(NSPoint::new(bounds.size.width / 2.0, 10.0));
                    mark.lineToPoint(NSPoint::new(bounds.size.width / 2.0, 14.5));
                    mark.stroke();
                    let dot = NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                        NSPoint::new(bounds.size.width / 2.0 - 0.8, 5.0),
                        NSSize::new(1.6, 1.6),
                    ));
                    dot.fill();
                }
            }
        }
    }
}

impl RingView {
    fn new(
        mtm: MainThreadMarker,
        style: RingStyle,
        color: (f64, f64, f64, f64),
        frame: NSRect,
    ) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(RingStyleState {
            style: Cell::new(style),
            color: Cell::new(color),
        });
        let view: Retained<RingView> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }

    fn set_style(&self, style: RingStyle, color: (f64, f64, f64, f64)) {
        self.ivars().style.set(style);
        self.ivars().color.set(color);
        self.setNeedsDisplay(true);
    }
}

// ============ IndicatorView:contentView ============

struct IVars {
    ring: Retained<RingView>,
    timer_label: Retained<NSTextField>,
    rec_label: Retained<NSTextField>,
    status_label: Retained<NSTextField>,
    cancel_btn: Retained<NSButton>,
    timer: RefCell<Option<Retained<NSTimer>>>,
    start: RefCell<Option<Instant>>,
    breath: RefCell<Option<Retained<NSTimer>>>,
    breath_start: RefCell<Option<Instant>>,
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

        /// 呼吸动画:layer.opacity 正弦插值(不重绘,GPU 合成)
        #[unsafe(method(breatheTick:))]
        fn breathe_tick(&self, _timer: *mut NSTimer) {
            let Some(start) = *self.ivars().breath_start.borrow() else {
                return;
            };
            let elapsed = start.elapsed().as_secs_f64();
            let period = if self.ivars().timer.borrow().is_some() {
                1.2 // 录音红环(原版 1;0.5;1 @1.2s)
            } else {
                2.0 // 处理中光点(原版水母 2s 节奏)
            };
            // 0.5 → 1.0 → 0.5 正弦
            let opacity = (0.75 - 0.25 * ((elapsed / period) * std::f64::consts::TAU).cos()) as f32;
            unsafe {
                if let Some(layer) = self.ivars().ring.layer() {
                    layer.setOpacity(opacity);
                }
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

    fn stop_breath(&self) {
        if let Some(t) = self.ivars().breath.borrow_mut().take() {
            t.invalidate();
        }
        *self.ivars().breath_start.borrow_mut() = None;
        if let Some(layer) = unsafe { self.ivars().ring.layer() } {
            layer.setOpacity(1.0);
        }
    }

    fn start_breath(&self) {
        self.stop_breath();
        *self.ivars().breath_start.borrow_mut() = Some(Instant::now());
        let target: &AnyObject = unsafe { &*(self as *const Self as *const AnyObject) };
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                BREATH_INTERVAL,
                target,
                sel!(breatheTick:),
                None,
                true,
            )
        };
        *self.ivars().breath.borrow_mut() = Some(timer);
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
        Phase::PasteFailed => 260.0,
    };

    let (ring_f, timer_f, rec_f, status_f, btn_f) = match phase {
        Phase::Recording => (
            NSRect::new(NSPoint::new(14.0, 15.0), NSSize::new(18.0, 18.0)),
            NSRect::new(NSPoint::new(36.0, 15.0), NSSize::new(44.0, 18.0)),
            NSRect::new(NSPoint::new(82.0, 15.0), NSSize::new(26.0, 12.0)),
            NSRect::ZERO,
            NSRect::new(NSPoint::new(146.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
        Phase::Processing => (
            NSRect::new(NSPoint::new(16.0, 14.0), NSSize::new(20.0, 20.0)),
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::new(NSPoint::new(40.0, 15.0), NSSize::new(76.0, 18.0)),
            NSRect::new(NSPoint::new(118.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
        Phase::PasteFailed => (
            NSRect::new(NSPoint::new(14.0, 14.0), NSSize::new(20.0, 20.0)),
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::new(NSPoint::new(40.0, 15.0), NSSize::new(190.0, 18.0)),
            NSRect::new(NSPoint::new(234.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
    };

    unsafe {
        iv.ring.setFrame(ring_f);
        iv.ring.setHidden(false);
        iv.timer_label.setFrame(timer_f);
        iv.timer_label.setHidden(phase != Phase::Recording);
        iv.rec_label.setFrame(rec_f);
        iv.rec_label.setHidden(phase != Phase::Recording);
        iv.status_label.setFrame(status_f);
        iv.status_label.setHidden(phase == Phase::Recording);
        iv.cancel_btn.setFrame(btn_f);
    }

    // 图标样式随相位切换
    match phase {
        Phase::Recording => iv.ring.set_style(RingStyle::Ring, RED),
        Phase::Processing => iv.ring.set_style(RingStyle::Dot, CYAN),
        Phase::PasteFailed => iv.ring.set_style(RingStyle::Warn, AMBER),
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
    let ring = RingView::new(mtm, RingStyle::Ring, RED, NSRect::ZERO);
    ring.setWantsLayer(true);
    let timer_label = make_label(mtm, "0:00", 13.0, true, (1.0, 1.0, 1.0, 0.9), NSRect::ZERO);
    let rec_label = make_label(mtm, "REC", 8.0, true, (1.0, 1.0, 1.0, 0.55), NSRect::ZERO);
    let status_label = make_label(mtm, "处理中…", 12.0, true, CYAN, NSRect::ZERO);
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
        ring: ring.clone(),
        timer_label: timer_label.clone(),
        rec_label: rec_label.clone(),
        status_label: status_label.clone(),
        cancel_btn: cancel_btn.clone(),
        timer: RefCell::new(None),
        start: RefCell::new(None),
        breath: RefCell::new(None),
        breath_start: RefCell::new(None),
    });
    let view: Retained<IndicatorView> =
        unsafe { msg_send![super(this), initWithFrame: content_rect] };

    // --- 挂载子控件(此前漏挂导致浮窗只有底壳) ---
    unsafe {
        view.addSubview(&ring);
        view.addSubview(&timer_label);
        view.addSubview(&rec_label);
        view.addSubview(&status_label);
        view.addSubview(&cancel_btn);
    }

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
        match phase {
            Phase::Recording => {
                view.start_timer(Instant::now());
                view.start_breath();
            }
            Phase::Processing => {
                view.stop_timer();
                view.start_breath();
            }
            Phase::PasteFailed => {
                view.stop_timer();
                view.stop_breath();
            }
        }
        if phase == Phase::Processing {
            view.ivars()
                .status_label
                .setStringValue(&NSString::from_str("处理中…"));
        } else if phase == Phase::PasteFailed {
            view.ivars()
                .status_label
                .setStringValue(&NSString::from_str("自动粘贴失败，请手动粘贴"));
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
        (*p.view).stop_breath();
        (*p.panel).orderOut(None);
    }
}
