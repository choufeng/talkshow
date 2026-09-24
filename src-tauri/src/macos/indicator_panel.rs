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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::runtime::Sel;
use objc2::{define_class, msg_send, sel, DeclaredClass, MainThreadMarker};
use objc2_app_kit::{
    NSBackingStoreType, NSBezierPath, NSButton, NSColor, NSEvent, NSFont, NSGradient, NSLineCapStyle,
    NSPanel, NSScreen, NSTextField, NSView, NSWindowCollectionBehavior, NSWindowStyleMask,
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

/// 用户拖动过面板后,当前会话内不再自动重新居中(hide 时重置)
static USER_POSITIONED: AtomicBool = AtomicBool::new(false);

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

// ============ JellyfishView:处理中水母(原版 SVG 复刻,帧间插值形变) ============

/// 原版 SVG viewBox 28x28,y 轴已翻转为 AppKit 坐标(向上)
type Pt = (f64, f64);

/// 钟形两帧(A=舒张,B=收缩),各 9 点 = 起点 + 4x(Q控制点,终点)
const BELL_A: [Pt; 9] = [
    (5.0, 12.0),
    (5.0, 23.0),
    (14.0, 23.0),
    (23.0, 23.0),
    (23.0, 12.0),
    (19.0, 9.0),
    (14.0, 11.0),
    (9.0, 9.0),
    (5.0, 12.0),
];
const BELL_B: [Pt; 9] = [
    (5.0, 13.0),
    (6.0, 21.0),
    (14.0, 21.0),
    (22.0, 21.0),
    (23.0, 13.0),
    (19.0, 11.0),
    (14.0, 13.0),
    (9.0, 11.0),
    (5.0, 13.0),
];

struct TentacleSpec {
    /// A 帧三点(起,控制,终)
    a: [Pt; 3],
    /// B 帧三点
    b: [Pt; 3],
    period: f64,
    alpha: f64,
    width: f64,
}

const TENTACLES: [TentacleSpec; 5] = [
    {
        TentacleSpec {
            a: [(8.0, 11.5), (6.0, 6.0), (8.0, 1.0)],
            b: [(8.0, 12.5), (5.0, 7.0), (7.0, 2.0)],
            period: 2.5,
            alpha: 0.5,
            width: 0.8,
        }
    },
    TentacleSpec {
        a: [(11.0, 11.0), (9.0, 5.0), (10.0, 0.0)],
        b: [(11.0, 12.0), (8.0, 6.0), (9.0, 1.0)],
        period: 2.2,
        alpha: 0.7,
        width: 1.0,
    },
    TentacleSpec {
        a: [(14.0, 10.5), (13.0, 4.0), (14.0, 0.0)],
        b: [(14.0, 11.5), (12.0, 5.0), (13.0, 1.0)],
        period: 2.0,
        alpha: 0.8,
        width: 1.2,
    },
    TentacleSpec {
        a: [(17.0, 11.0), (19.0, 5.0), (18.0, 0.0)],
        b: [(17.0, 12.0), (20.0, 6.0), (19.0, 1.0)],
        period: 2.2,
        alpha: 0.7,
        width: 1.0,
    },
    TentacleSpec {
        a: [(20.0, 11.5), (22.0, 6.0), (20.0, 1.0)],
        b: [(20.0, 12.5), (23.0, 7.0), (21.0, 2.0)],
        period: 2.5,
        alpha: 0.5,
        width: 0.8,
    },
];

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp_pt(a: Pt, b: Pt, t: f64) -> Pt {
    (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
}

/// 二次曲线转三次控制点(NSBezierPath 只支持 cubic)
fn quad_ctrls(p0: Pt, c: Pt, p1: Pt) -> (Pt, Pt) {
    (
        (
            p0.0 + (c.0 - p0.0) * 2.0 / 3.0,
            p0.1 + (c.1 - p0.1) * 2.0 / 3.0,
        ),
        (
            p1.0 + (c.0 - p1.0) * 2.0 / 3.0,
            p1.1 + (c.1 - p1.1) * 2.0 / 3.0,
        ),
    )
}

/// 平滑往返相位 0→1→0
fn ease(period: f64, elapsed: f64) -> f64 {
    let t = (elapsed / period) % 1.0;
    (1.0 - (t * std::f64::consts::TAU).cos()) / 2.0
}

struct JellyfishState {
    anim_start: Cell<Option<Instant>>,
}

define_class! {
    #[unsafe(super(NSView))]
    #[ivars = JellyfishState]
    #[name = "TalkShowJellyfishView"]
    struct JellyfishView;

    impl JellyfishView {
        #[unsafe(method(drawRect:))]
        unsafe fn draw_rect(&self, _dirty: NSRect) {
            let elapsed = self
                .ivars()
                .anim_start
                .get()
                .map(|s| s.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            unsafe { self.draw_jellyfish(elapsed) };
        }
    }
}

impl JellyfishView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = mtm
            .alloc()
            .set_ivars(JellyfishState { anim_start: Cell::new(None) });
        let view: Retained<JellyfishView> =
            unsafe { msg_send![super(this), initWithFrame: frame] };
        view
    }

    fn start_anim(&self) {
        self.ivars().anim_start.set(Some(Instant::now()));
        self.setNeedsDisplay(true);
    }

    fn stop_anim(&self) {
        self.ivars().anim_start.set(None);
        self.setNeedsDisplay(true);
    }

    fn is_animating(&self) -> bool {
        self.ivars().anim_start.get().is_some()
    }

    unsafe fn draw_jellyfish(&self, elapsed: f64) {
        let mtm = unsafe { MainThreadMarker::from(self) };
        let cyan = NSColor::colorWithSRGBRed_green_blue_alpha(CYAN.0, CYAN.1, CYAN.2, 1.0);

        // --- 钟形 ---
        let t = ease(2.0, elapsed);
        let bell: Vec<Pt> = BELL_A
            .iter()
            .zip(BELL_B.iter())
            .map(|(a, b)| lerp_pt(*a, *b, t))
            .collect();
        let bell_path = unsafe { NSBezierPath::bezierPath() };
        unsafe {
            bell_path.moveToPoint(NSPoint::new(bell[0].0, bell[0].1));
            for seg in [(0usize, 1, 2), (2, 3, 4), (4, 5, 6), (6, 7, 8)] {
                let (c1, c2) = quad_ctrls(bell[seg.0], bell[seg.1], bell[seg.2]);
                bell_path.curveToPoint_controlPoint1_controlPoint2(
                    NSPoint::new(bell[seg.2].0, bell[seg.2].1),
                    NSPoint::new(c1.0, c1.1),
                    NSPoint::new(c2.0, c2.1),
                );
            }
            bell_path.closePath();
        }

        // 渐变填充(原版 radialGradient 青 0.3 → 0.05,线性近似)
        let g_from = NSColor::colorWithSRGBRed_green_blue_alpha(CYAN.0, CYAN.1, CYAN.2, 0.30);
        let g_to = NSColor::colorWithSRGBRed_green_blue_alpha(CYAN.0, CYAN.1, CYAN.2, 0.05);
        if let Some(gradient) = unsafe {
            NSGradient::initWithStartingColor_endingColor(mtm.alloc(), &g_from, &g_to)
        } {
            unsafe { gradient.drawInBezierPath_angle(&bell_path, 90.0) };
        }
        cyan.set();
        unsafe {
            bell_path.setLineWidth(1.2);
            bell_path.stroke();
        }

        // --- 内腔椭圆(ry 3→2 形变) ---
        let ry = lerp(3.0, 2.0, t);
        let inner = unsafe {
            NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                NSPoint::new(10.0, 16.0 - ry),
                NSSize::new(8.0, ry * 2.0),
            ))
        };
        let inner_c = NSColor::colorWithSRGBRed_green_blue_alpha(CYAN.0, CYAN.1, CYAN.2, 0.15);
        inner_c.set();
        inner.fill();

        // --- 触须(各周期不同 → 相位渐开自然摆动) ---
        for spec in TENTACLES.iter() {
            let tt = ease(spec.period, elapsed);
            let p0 = lerp_pt(spec.a[0], spec.b[0], tt);
            let c = lerp_pt(spec.a[1], spec.b[1], tt);
            let p1 = lerp_pt(spec.a[2], spec.b[2], tt);
            let path = unsafe { NSBezierPath::bezierPath() };
            unsafe {
                path.moveToPoint(NSPoint::new(p0.0, p0.1));
                let (c1, c2) = quad_ctrls(p0, c, p1);
                path.curveToPoint_controlPoint1_controlPoint2(
                    NSPoint::new(p1.0, p1.1),
                    NSPoint::new(c1.0, c1.1),
                    NSPoint::new(c2.0, c2.1),
                );
                let tc = NSColor::colorWithSRGBRed_green_blue_alpha(CYAN.0, CYAN.1, CYAN.2, spec.alpha);
                tc.set();
                path.setLineWidth(spec.width);
                path.setLineCapStyle(NSLineCapStyle::Round);
                path.stroke();
            }
        }
    }
}

// ============ IndicatorView:contentView ============

struct IVars {
    ring: Retained<RingView>,
    jellyfish: Retained<JellyfishView>,
    timer_label: Retained<NSTextField>,
    rec_label: Retained<NSTextField>,
    status_label: Retained<NSTextField>,
    cancel_btn: Retained<NSButton>,
    timer: RefCell<Option<Retained<NSTimer>>>,
    start: RefCell<Option<Instant>>,
    breath: RefCell<Option<Retained<NSTimer>>>,
    breath_start: RefCell<Option<Instant>>,
    /// 左键拖动:按下时鼠标 window 坐标 + 窗口 origin
    drag_loc: Cell<NSPoint>,
    drag_origin: Cell<NSPoint>,
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

        /// 左键按住拖动移动面板(✕ 按钮区域除外,按钮自处理点击)
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.ivars().drag_loc.set(unsafe { event.locationInWindow() });
            if let Some(w) = unsafe { self.window() } {
                self.ivars().drag_origin.set(unsafe { w.frame().origin });
            }
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let loc = unsafe { event.locationInWindow() };
            let start = self.ivars().drag_loc.get();
            let origin = self.ivars().drag_origin.get();
            if let Some(w) = unsafe { self.window() } {
                unsafe {
                    w.setFrameOrigin(NSPoint::new(
                        origin.x + (loc.x - start.x),
                        origin.y + (loc.y - start.y),
                    ));
                }
                USER_POSITIONED.store(true, Ordering::Relaxed);
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

        /// 呼吸动画:layer.opacity 正弦插值(不重绘,GPU 合成) + 水母形变推进
        #[unsafe(method(breatheTick:))]
        fn breathe_tick(&self, _timer: *mut NSTimer) {
            // 水母形变动画(processing 相位)
            let jelly = &self.ivars().jellyfish;
            if jelly.is_animating() {
                jelly.setNeedsDisplay(true);
            }
            // 红环呼吸仅在录音相位(timer 存在)
            if self.ivars().timer.borrow().is_none() {
                return;
            }
            let Some(start) = *self.ivars().breath_start.borrow() else {
                return;
            };
            let elapsed = start.elapsed().as_secs_f64();
            let period = 1.2; // 录音红环(原版 1;0.5;1 @1.2s)
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

    let (ring_f, jelly_f, timer_f, rec_f, status_f, btn_f) = match phase {
        Phase::Recording => (
            NSRect::new(NSPoint::new(14.0, 15.0), NSSize::new(18.0, 18.0)),
            NSRect::ZERO,
            NSRect::new(NSPoint::new(36.0, 15.0), NSSize::new(44.0, 18.0)),
            NSRect::new(NSPoint::new(82.0, 15.0), NSSize::new(26.0, 12.0)),
            NSRect::ZERO,
            NSRect::new(NSPoint::new(146.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
        Phase::Processing => (
            NSRect::ZERO,
            NSRect::new(NSPoint::new(12.0, 10.0), NSSize::new(28.0, 28.0)),
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::new(NSPoint::new(44.0, 15.0), NSSize::new(72.0, 18.0)),
            NSRect::new(NSPoint::new(118.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
        Phase::PasteFailed => (
            NSRect::new(NSPoint::new(14.0, 14.0), NSSize::new(20.0, 20.0)),
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::ZERO,
            NSRect::new(NSPoint::new(40.0, 15.0), NSSize::new(190.0, 18.0)),
            NSRect::new(NSPoint::new(234.0, 12.0), NSSize::new(22.0, 24.0)),
        ),
    };

    unsafe {
        iv.ring.setFrame(ring_f);
        iv.ring.setHidden(phase == Phase::Processing);
        iv.jellyfish.setFrame(jelly_f);
        iv.jellyfish.setHidden(phase != Phase::Processing);
        iv.timer_label.setFrame(timer_f);
        iv.timer_label.setHidden(phase != Phase::Recording);
        iv.rec_label.setFrame(rec_f);
        iv.rec_label.setHidden(phase != Phase::Recording);
        iv.status_label.setFrame(status_f);
        iv.status_label.setHidden(phase == Phase::Recording);
        iv.cancel_btn.setFrame(btn_f);
    }

    // 图标样式随相位切换(processing 用水母,ring 隐藏)
    match phase {
        Phase::Recording => iv.ring.set_style(RingStyle::Ring, RED),
        Phase::Processing => {}
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
    let jellyfish = JellyfishView::new(mtm, NSRect::ZERO);
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
        jellyfish: jellyfish.clone(),
        timer_label: timer_label.clone(),
        rec_label: rec_label.clone(),
        status_label: status_label.clone(),
        cancel_btn: cancel_btn.clone(),
        timer: RefCell::new(None),
        start: RefCell::new(None),
        breath: RefCell::new(None),
        breath_start: RefCell::new(None),
        drag_loc: Cell::new(NSPoint::ZERO),
        drag_origin: Cell::new(NSPoint::ZERO),
    });
    let view: Retained<IndicatorView> =
        unsafe { msg_send![super(this), initWithFrame: content_rect] };

    // --- 挂载子控件(此前漏挂导致浮窗只有底壳) ---
    unsafe {
        view.addSubview(&jellyfish);
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
                view.ivars().jellyfish.stop_anim();
            }
            Phase::Processing => {
                view.stop_timer();
                view.stop_breath();
                view.ivars().jellyfish.start_anim();
            }
            Phase::PasteFailed => {
                view.stop_timer();
                view.stop_breath();
                view.ivars().jellyfish.stop_anim();
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
        (*p.view).ivars().jellyfish.stop_anim();
        (*p.panel).orderOut(None);
    }
    USER_POSITIONED.store(false, Ordering::Relaxed);
}
