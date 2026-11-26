//! Calm stacked notification toasts with hover fan-out, promise morphing, and swipe dismiss.
//!
//! Renders an interactive toast trigger deck alongside a stacked notification
//! viewport supporting `Default`, `Success`, `Action / Undo`, `Error`, and
//! `Promise (Saving -> Saved)` notifications, vertical `Slide` fan-out on hover,
//! hover-paused countdown timers, and pointer drag-to-swipe dismissal.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_eid, pro_empty_state, pro_skeleton_bar, pro_spring_f32,
        pro_tactile, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::f32::consts::{FRAC_PI_2, TAU};
use web_time::Instant;

const ICON_DOC: &str = r#"<path d="M7 3.5h7l4.5 4.5v11a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 19V5A1.5 1.5 0 0 1 7 3.5zM14 3.5V8h4.5M8.5 12.5h7M8.5 16h5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#;
const ICON_FLOPPY: &str = r#"<path d="M6 4h9.5L20 8.5V18a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zM8 4v4.5h7V4M8 20v-6h8v6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#;
const ICON_SYNC_ARROWS: &str = r#"<path d="M19.5 12a7.5 7.5 0 0 1-13.3 4.8M4.5 12a7.5 7.5 0 0 1 13.3-4.8M18 3.5v4h-4M6 20.5v-4h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#;

const CARD_WIDTH: f32 = 356.0;
const CARD_HEIGHT: f32 = 68.0;
const FAN_GAP: f32 = 10.0;
const SWIPE_THRESHOLD: f32 = 72.0;

/// Corner or edge anchor for [`Toaster`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToasterPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

/// Visual and lifecycle variant of a [`ToastItem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    /// Calm neutral info notification.
    Default,
    /// Emerald check confirmation.
    Success,
    /// Reversible notification carrying an inline action button (e.g. `Undo`).
    Action,
    /// Non-blocking red cross alert.
    Error,
    /// Async operation that starts spinning (`Saving…`) and morphs into a check (`Saved`).
    Promise { resolved: bool },
}

/// Public toast specification pushed onto [`Toaster`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToastItem {
    pub id: u64,
    pub kind: ToastKind,
    pub title: SharedString,
    pub description: Option<SharedString>,
    pub resolved_title: Option<SharedString>,
    pub resolved_description: Option<SharedString>,
    pub action_label: Option<SharedString>,
    /// Auto-dismiss duration in milliseconds (`None` keeps the toast pinned until dismissed).
    pub duration_ms: Option<u64>,
    /// Milliseconds before a `Promise` toast resolves from spinning arc to check mark.
    pub promise_resolve_ms: u64,
}

impl ToastItem {
    /// Construct a plain [`ToastKind::Default`] notification.
    pub fn new(id: u64, title: impl Into<SharedString>) -> Self {
        Self {
            id,
            kind: ToastKind::Default,
            title: title.into(),
            description: None,
            resolved_title: None,
            resolved_description: None,
            action_label: None,
            duration_ms: Some(4600),
            promise_resolve_ms: 1500,
        }
    }

    /// Set secondary description copy beneath the toast title.
    pub fn with_description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Override the [`ToastKind`] variant.
    pub fn with_kind(mut self, kind: ToastKind) -> Self {
        self.kind = kind;
        self
    }

    /// Attach an inline action button (such as `"Undo"`).
    pub fn with_action(mut self, label: impl Into<SharedString>) -> Self {
        self.action_label = Some(label.into());
        self
    }

    /// Configure auto-dismiss lifetime (`None` disables auto-dismiss).
    pub fn with_duration_ms(mut self, duration_ms: Option<u64>) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Configure the resolved title and description for [`ToastKind::Promise`].
    pub fn with_promise_resolution(
        mut self,
        resolve_ms: u64,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.kind = ToastKind::Promise { resolved: false };
        self.promise_resolve_ms = resolve_ms;
        self.resolved_title = Some(title.into());
        self.resolved_description = Some(description.into());
        self
    }
}

/// Typed events emitted by [`Toaster`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToasterEvent {
    /// New toast pushed onto the stack.
    Pushed { id: u64, kind: ToastKind },
    /// Promise toast transitioned from spinning loading state to resolved check.
    PromiseResolved(u64),
    /// User clicked the inline action button (e.g. `Undo`) on toast `id`.
    ActionClicked { id: u64, label: SharedString },
    /// Toast dismissed via timer, close button, `Esc`, or pointer swipe.
    Dismissed(u64),
    /// Stack fan-out state changed (`true` when hovered, focused via `F8`, or pinned).
    ExpandedChanged(bool),
}

struct LiveToast {
    spec: ToastItem,
    last_tick: Instant,
    active_elapsed_ms: f32,
    promise_elapsed_ms: f32,
    swipe_release: Slide,
    top_slide: Slide,
    scale_slide: Slide,
    alpha_slide: Slide,
    last_targets: (f32, f32, f32),
}

impl LiveToast {
    fn from_spec_at_slot(spec: ToastItem, slot: usize, entering: bool) -> Self {
        let init_top = if entering { -16.0 } else { slot as f32 * 13.0 };
        let init_scale = if entering {
            0.96
        } else {
            (1.0 - slot as f32 * 0.06).max(0.84)
        };
        let init_alpha = if slot == 0 { 1.0 } else { 0.0 };
        Self {
            spec,
            last_tick: Instant::now(),
            active_elapsed_ms: 0.,
            promise_elapsed_ms: 0.,
            swipe_release: Slide::snappy(0.),
            top_slide: Slide::snappy(init_top),
            scale_slide: Slide::snappy(init_scale),
            alpha_slide: Slide::snappy(init_alpha),
            last_targets: (init_top, init_scale, init_alpha),
        }
    }
}

struct ExitingToast {
    spec: ToastItem,
    base_top: f32,
    base_scale: f32,
    start_dx: f32,
    travel_dx: f32,
    exit_slide: Slide,
    started_at: Instant,
}

#[derive(Clone, Copy)]
struct ActiveDrag {
    toast_id: u64,
    start_x: f32,
    current_dx: f32,
}

fn render_toast_status_icon(
    id: ElementId,
    kind: ToastKind,
    spin_elapsed_secs: f32,
    p: &ProPalette,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let is_loading = matches!(kind, ToastKind::Promise { resolved: false });
    let is_success = matches!(
        kind,
        ToastKind::Success | ToastKind::Promise { resolved: true }
    );
    let is_error = matches!(kind, ToastKind::Error);

    let ring_sweep = pro_spring_f32(
        pro_eid(&id, "ring-sweep"),
        if is_loading { 0.28 } else { 1.0 },
        0.38,
        w,
        cx,
    )
    .clamp(0.08, 1.0);
    let mark_draw = pro_spring_f32(
        pro_eid(&id, "mark-draw"),
        if is_loading { 0.0 } else { 1.0 },
        0.32,
        w,
        cx,
    )
    .clamp(0., 1.);

    let stroke_col: Hsla = if is_loading {
        p.muted
    } else if is_success {
        if p.dark {
            rgb(0x5fc88a).into()
        } else {
            rgb(0x1e8a4f).into()
        }
    } else if is_error {
        if p.dark {
            rgb(0xff6b5f).into()
        } else {
            rgb(0xf04438).into()
        }
    } else if p.dark {
        rgb(0x4fb0fe).into()
    } else {
        rgba(0x0060e2c4).into()
    };

    if is_loading {
        w.request_animation_frame();
    }

    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let center = bounds.center();
            let side = f32::from(bounds.size.width).min(f32::from(bounds.size.height));
            if side <= 2. {
                return;
            }
            let r = side * 0.40;
            let base_rot = if is_loading {
                (spin_elapsed_secs / 0.76).fract() * TAU - FRAC_PI_2
            } else {
                -FRAC_PI_2
            };
            let ring_alpha = if is_loading {
                0.92
            } else {
                0.24 + 0.68 * (1. - mark_draw)
            };

            let steps = 28usize;
            let mut ring = PathBuilder::stroke(px(1.65));
            for i in 0..=steps {
                let a = base_rot + (i as f32 / steps as f32) * ring_sweep * TAU;
                let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    ring.move_to(pt);
                } else {
                    ring.line_to(pt);
                }
            }
            if let Ok(path) = ring.build() {
                window.paint_path(path, stroke_col.opacity(ring_alpha));
            }

            if mark_draw > 0.02 {
                if is_success {
                    let p0 = point(
                        bounds.left() + px(side * 0.31),
                        bounds.top() + px(side * 0.52),
                    );
                    let p1 = point(
                        bounds.left() + px(side * 0.45),
                        bounds.top() + px(side * 0.65),
                    );
                    let p2 = point(
                        bounds.left() + px(side * 0.70),
                        bounds.top() + px(side * 0.37),
                    );
                    let mut chk = PathBuilder::stroke(px(1.65));
                    chk.move_to(p0);
                    let knee = 0.38;
                    if mark_draw <= knee {
                        let u = mark_draw / knee;
                        chk.line_to(point(p0.x + (p1.x - p0.x) * u, p0.y + (p1.y - p0.y) * u));
                    } else {
                        chk.line_to(p1);
                        let u = (mark_draw - knee) / (1. - knee);
                        chk.line_to(point(p1.x + (p2.x - p1.x) * u, p1.y + (p2.y - p1.y) * u));
                    }
                    if let Ok(path) = chk.build() {
                        window.paint_path(path, stroke_col.opacity(mark_draw));
                    }
                } else if is_error {
                    let a0 = point(
                        bounds.left() + px(side * 0.35),
                        bounds.top() + px(side * 0.35),
                    );
                    let a1 = point(
                        bounds.left() + px(side * 0.65),
                        bounds.top() + px(side * 0.65),
                    );
                    let b0 = point(
                        bounds.left() + px(side * 0.65),
                        bounds.top() + px(side * 0.35),
                    );
                    let b1 = point(
                        bounds.left() + px(side * 0.35),
                        bounds.top() + px(side * 0.65),
                    );
                    let u1 = (mark_draw * 1.6).clamp(0., 1.);
                    let mut s1 = PathBuilder::stroke(px(1.65));
                    s1.move_to(a0);
                    s1.line_to(point(a0.x + (a1.x - a0.x) * u1, a0.y + (a1.y - a0.y) * u1));
                    if let Ok(path) = s1.build() {
                        window.paint_path(path, stroke_col);
                    }
                    if mark_draw > 0.35 {
                        let u2 = ((mark_draw - 0.35) / 0.65).clamp(0., 1.);
                        let mut s2 = PathBuilder::stroke(px(1.65));
                        s2.move_to(b0);
                        s2.line_to(point(b0.x + (b1.x - b0.x) * u2, b0.y + (b1.y - b0.y) * u2));
                        if let Ok(path) = s2.build() {
                            window.paint_path(path, stroke_col);
                        }
                    }
                } else {
                    // Info 'i' mark for Default / Action
                    let stem_top = point(center.x, bounds.top() + px(side * 0.45));
                    let stem_bot = point(center.x, bounds.top() + px(side * 0.68));
                    let mut stem = PathBuilder::stroke(px(1.65));
                    stem.move_to(stem_top);
                    stem.line_to(stem_bot);
                    if let Ok(path) = stem.build() {
                        window.paint_path(path, stroke_col.opacity(mark_draw));
                    }
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(center.x - px(1.), bounds.top() + px(side * 0.29)),
                                size(px(2.), px(2.)),
                            ),
                            stroke_col.opacity(mark_draw),
                        )
                        .corner_radii(px(1.)),
                    );
                }
            }
        },
    )
    .size(px(18.))
    .flex_none()
    .into_any_element()
}

fn seed_initial_toasts(zh: bool) -> Vec<LiveToast> {
    let items = if zh {
        vec![
            ToastItem::new(3, "正在保存报告…")
                .with_description("Q3-Board-Report.pdf · 4.2 MB")
                .with_promise_resolution(1700, "报告已保存", "已自动同步给 8 位协作成员")
                .with_duration_ms(None),
            ToastItem::new(2, "已移至废纸篓")
                .with_description("Q2-Forecast-Archive.xlsx")
                .with_kind(ToastKind::Action)
                .with_action("撤销")
                .with_duration_ms(None),
            ToastItem::new(1, "草稿已自动保存")
                .with_description("财务季报 · 保留全部修订记录")
                .with_kind(ToastKind::Success)
                .with_duration_ms(None),
        ]
    } else {
        vec![
            ToastItem::new(3, "Saving report…")
                .with_description("Q3-Board-Report.pdf · 4.2 MB")
                .with_promise_resolution(
                    1700,
                    "Saved & synced",
                    "Shared with 8 workspace reviewers",
                )
                .with_duration_ms(None),
            ToastItem::new(2, "Moved to trash")
                .with_description("Q2-Forecast-Archive.xlsx")
                .with_kind(ToastKind::Action)
                .with_action("Undo")
                .with_duration_ms(None),
            ToastItem::new(1, "Draft saved")
                .with_description("Q3 Board Report · all changes kept")
                .with_kind(ToastKind::Success)
                .with_duration_ms(None),
        ]
    };
    items
        .into_iter()
        .enumerate()
        .map(|(slot, spec)| LiveToast::from_spec_at_slot(spec, slot, false))
        .collect()
}

/// Calm stacked notification toaster with trigger buttons, hover fan-out, and swipe dismiss.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::toaster::{Toaster, ToasterEvent, ToasterPosition};
///
/// # fn doc(cx: &mut gpui::App) {
/// let toaster = cx.new(|cx| {
///     Toaster::new("app-toaster", cx)
///         .with_position(ToasterPosition::BottomRight)
///         .with_visible_toasts(3)
/// });
/// cx.subscribe(&toaster, |_, ev: &ToasterEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct Toaster {
    id: ElementId,
    position: ToasterPosition,
    visible_toasts: usize,
    default_duration_ms: u64,
    always_expand: bool,
    show_hairline: bool,
    show_triggers: bool,
    hovered_stack: bool,
    keyboard_expanded: bool,
    hovered_card_id: Option<u64>,
    active_drag: Option<ActiveDrag>,
    next_id: u64,
    toasts: Vec<LiveToast>,
    exiting_toasts: Vec<ExitingToast>,
    stack_h_slide: Slide,
    last_stack_h: f32,
    seeded_lang_zh: bool,
    custom_seeded: bool,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl EventEmitter<ToasterEvent> for Toaster {}

impl Toaster {
    /// Construct a [`Toaster`] with interactive trigger buttons and 3 pre-seeded stacked toasts.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        Self {
            id: id.into(),
            position: ToasterPosition::BottomRight,
            visible_toasts: 3,
            default_duration_ms: 4600,
            always_expand: false,
            show_hairline: true,
            show_triggers: true,
            hovered_stack: false,
            keyboard_expanded: false,
            hovered_card_id: None,
            active_drag: None,
            next_id: 4,
            toasts: seed_initial_toasts(zh),
            exiting_toasts: Vec::new(),
            stack_h_slide: Slide::snappy(108.),
            last_stack_h: 108.,
            seeded_lang_zh: zh,
            custom_seeded: false,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Configure the anchor position metadata.
    pub fn with_position(mut self, position: ToasterPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the maximum number of toasts visible simultaneously in the stack (`default = 3`).
    pub fn with_visible_toasts(mut self, count: usize) -> Self {
        self.visible_toasts = count.max(1);
        self
    }

    /// Set the default auto-dismiss duration in milliseconds (`default = 4600`).
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.default_duration_ms = duration_ms.max(800);
        self
    }

    /// Keep the stack permanently fanned out instead of only on hover / `F8`.
    pub fn with_expand(mut self, expand: bool) -> Self {
        self.always_expand = expand;
        self
    }

    /// Show or hide the `1px` bottom countdown hairline on auto-dismissing toasts.
    pub fn with_hairline(mut self, hairline: bool) -> Self {
        self.show_hairline = hairline;
        self
    }

    /// Show or hide the built-in interactive trigger card above the stack.
    pub fn with_triggers(mut self, show: bool) -> Self {
        self.show_triggers = show;
        self
    }

    /// Seed an explicit initial list of [`ToastItem`]s (newest first).
    pub fn with_toasts(mut self, items: Vec<ToastItem>) -> Self {
        let max_id = items.iter().map(|t| t.id).max().unwrap_or(0);
        self.next_id = max_id + 1;
        self.toasts = items
            .into_iter()
            .enumerate()
            .map(|(slot, spec)| LiveToast::from_spec_at_slot(spec, slot, false))
            .collect();
        self.custom_seeded = true;
        self
    }

    /// Enable or disable the loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state override.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the number of active toasts currently in the queue.
    pub fn toast_count(&self) -> usize {
        self.toasts.len()
    }

    /// Return whether the stack is currently fanned out.
    pub fn is_fanned_out(&self) -> bool {
        self.always_expand
            || self.hovered_stack
            || self.keyboard_expanded
            || self.active_drag.is_some()
    }

    /// Allocate the next unique toast id.
    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Push a [`ToastItem`] to the front of the stack.
    pub fn push_toast(&mut self, item: ToastItem, cx: &mut Context<Self>) -> u64 {
        self.custom_seeded = true;
        let id = item.id;
        let kind = item.kind;
        self.next_id = self.next_id.max(id + 1);
        self.toasts
            .insert(0, LiveToast::from_spec_at_slot(item, 0, true));
        if self.toasts.len() > 8 {
            self.toasts.truncate(8);
        }
        cx.emit(ToasterEvent::Pushed { id, kind });
        cx.notify();
        id
    }

    fn dismiss_with_motion(
        &mut self,
        id: u64,
        start_dx: f32,
        travel_dx: f32,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(pos) = self.toasts.iter().position(|t| t.spec.id == id) {
            let removed = self.toasts.remove(pos);
            if self.hovered_card_id == Some(id) {
                self.hovered_card_id = None;
            }
            if !disabled_motion(cx) && pos < self.visible_toasts {
                let mut exit_slide = Slide::snappy(0.);
                exit_slide.duration(1.0, 0.22, cx);
                self.exiting_toasts.push(ExitingToast {
                    spec: removed.spec,
                    base_top: removed.last_targets.0.max(0.),
                    base_scale: removed.last_targets.1,
                    start_dx,
                    travel_dx,
                    exit_slide,
                    started_at: Instant::now(),
                });
            }
            cx.emit(ToasterEvent::Dismissed(id));
            cx.notify();
            true
        } else {
            false
        }
    }

    /// Dismiss a single toast by `id`, emitting [`ToasterEvent::Dismissed`].
    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) -> bool {
        self.dismiss_with_motion(id, 0.0, 56.0, cx)
    }

    /// Dismiss the newest toast at the front of the stack.
    pub fn dismiss_newest(&mut self, cx: &mut Context<Self>) -> bool {
        if let Some(front_id) = self.toasts.first().map(|t| t.spec.id) {
            self.dismiss(front_id, cx)
        } else {
            false
        }
    }

    fn trigger_preset(&mut self, preset: &'static str, cx: &mut Context<Self>) {
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let id = self.allocate_id();
        let dur = Some(self.default_duration_ms);
        let item = match preset {
            "default" => {
                if zh {
                    ToastItem::new(id, "分享链接已复制")
                        .with_description("Q3 财务季报 · 团队成员只读权限")
                        .with_kind(ToastKind::Default)
                        .with_duration_ms(dur)
                } else {
                    ToastItem::new(id, "Link copied to clipboard")
                        .with_description("Q3 Board Report · workspace view access")
                        .with_kind(ToastKind::Default)
                        .with_duration_ms(dur)
                }
            }
            "success" => {
                if zh {
                    ToastItem::new(id, "草稿已保存")
                        .with_description("Q3 财务季报 · 所有更改均已保留")
                        .with_kind(ToastKind::Success)
                        .with_duration_ms(dur)
                } else {
                    ToastItem::new(id, "Draft saved")
                        .with_description("Q3 Board Report · all changes kept")
                        .with_kind(ToastKind::Success)
                        .with_duration_ms(dur)
                }
            }
            "action" => {
                if zh {
                    ToastItem::new(id, "文件已移至废纸篓")
                        .with_description("Q3-Board-Report.pdf")
                        .with_kind(ToastKind::Action)
                        .with_action("撤销")
                        .with_duration_ms(Some(6000))
                } else {
                    ToastItem::new(id, "File moved to trash")
                        .with_description("Q3-Board-Report.pdf")
                        .with_kind(ToastKind::Action)
                        .with_action("Undo")
                        .with_duration_ms(Some(6000))
                }
            }
            "error" => {
                if zh {
                    ToastItem::new(id, "云端同步失败")
                        .with_description("连接上游存储超时，请稍后重试")
                        .with_kind(ToastKind::Error)
                        .with_duration_ms(Some(6000))
                } else {
                    ToastItem::new(id, "Cloud sync failed")
                        .with_description("Upstream connection timed out — retry shortly")
                        .with_kind(ToastKind::Error)
                        .with_duration_ms(Some(6000))
                }
            }
            _ => {
                if zh {
                    ToastItem::new(id, "正在保存并发布…")
                        .with_description("Q3-Board-Report.pdf · 4.2 MB")
                        .with_promise_resolution(1450, "保存成功", "Q3 财务季报已同步给 8 位审阅人")
                        .with_duration_ms(Some(5400))
                } else {
                    ToastItem::new(id, "Saving changes…")
                        .with_description("Q3-Board-Report.pdf · 4.2 MB")
                        .with_promise_resolution(1450, "Saved", "Shared with 8 workspace reviewers")
                        .with_duration_ms(Some(5400))
                }
            }
        };
        self.push_toast(item, cx);
    }

    fn handle_toast_action(&mut self, toast_id: u64, label: SharedString, cx: &mut Context<Self>) {
        cx.emit(ToasterEvent::ActionClicked {
            id: toast_id,
            label: label.clone(),
        });
        self.dismiss(toast_id, cx);
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let confirm_id = self.allocate_id();
        let restored = if zh {
            ToastItem::new(confirm_id, "已撤销删除并恢复文件")
                .with_description("Q3-Board-Report.pdf 已回到原目录")
                .with_kind(ToastKind::Success)
                .with_duration_ms(Some(3600))
        } else {
            ToastItem::new(confirm_id, "File restored")
                .with_description("Q3-Board-Report.pdf returned to Finance")
                .with_kind(ToastKind::Success)
                .with_duration_ms(Some(3600))
        };
        self.push_toast(restored, cx);
    }

    fn finish_drag(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.active_drag.take() else {
            return;
        };
        if drag.current_dx.abs() >= SWIPE_THRESHOLD {
            let dir = if drag.current_dx >= 0.0 { 1.0 } else { -1.0 };
            self.dismiss_with_motion(drag.toast_id, drag.current_dx, dir * 130.0, cx);
        } else {
            if let Some(t) = self.toasts.iter_mut().find(|t| t.spec.id == drag.toast_id) {
                t.swipe_release.reset(drag.current_dx);
                t.swipe_release.duration(0., 0.24, cx);
            }
            cx.notify();
        }
    }

    fn tick_toasts(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.custom_seeded {
            let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
            if self.seeded_lang_zh != zh {
                self.toasts = seed_initial_toasts(zh);
                self.seeded_lang_zh = zh;
            }
        }

        let now = Instant::now();
        let paused = self.hovered_stack || self.keyboard_expanded || self.active_drag.is_some();
        let vis_limit = self.visible_toasts;
        let mut expired_ids = Vec::new();
        let mut resolved_ids = Vec::new();
        let mut needs_frame = false;

        for (idx, t) in self.toasts.iter_mut().enumerate() {
            let dt_ms = now
                .saturating_duration_since(t.last_tick)
                .as_secs_f32()
                .min(0.1)
                * 1000.0;
            t.last_tick = now;

            if matches!(t.spec.kind, ToastKind::Promise { resolved: false }) {
                t.promise_elapsed_ms += dt_ms;
                needs_frame = true;
                if t.promise_elapsed_ms >= t.spec.promise_resolve_ms as f32 {
                    t.spec.kind = ToastKind::Promise { resolved: true };
                    if let Some(rt) = t.spec.resolved_title.clone() {
                        t.spec.title = rt;
                    }
                    if let Some(rd) = t.spec.resolved_description.clone() {
                        t.spec.description = Some(rd);
                    }
                    t.active_elapsed_ms = 0.;
                    resolved_ids.push(t.spec.id);
                }
            } else if idx < vis_limit
                && let Some(limit_ms) = t.spec.duration_ms
                && !paused
            {
                t.active_elapsed_ms += dt_ms;
                needs_frame = true;
                if t.active_elapsed_ms >= limit_ms as f32 {
                    expired_ids.push(t.spec.id);
                }
            }
        }

        for rid in resolved_ids {
            cx.emit(ToasterEvent::PromiseResolved(rid));
        }
        for eid in expired_ids {
            self.dismiss(eid, cx);
        }
        if needs_frame && !disabled_motion(cx) {
            w.request_animation_frame();
        }
    }
}

impl Render for Toaster {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tick_toasts(w, cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            let phase = (self.started_at.elapsed().as_secs_f32() / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(16.))
                .child(pro_skeleton_bar(356., 156., 10., phase, cx))
                .child(pro_skeleton_bar(356., 76., 10., phase + 0.2, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(356.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x222226) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line)
                .shadow(pro_card_shadow(p.dark))
                .child(pro_empty_state(
                    ProIcon::Bell,
                    tr(cx, "No notifications", "暂无通知提醒"),
                    tr(
                        cx,
                        "Action confirmations and background sync updates will stack quietly here.",
                        "操作回执与后台异步进度将在此安静堆叠展示。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x222226).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.08)
        };
        let hairline_col: Hsla = if p.dark {
            hsla(0., 0., 1., 0.16)
        } else {
            hsla(0., 0., 0., 0.12)
        };

        // Build interactive trigger panel matching reference light.png / dark.png + all 5 toast types
        let build_trigger_btn = |btn_id: &'static str,
                                 preset: &'static str,
                                 label: SharedString,
                                 icon_el: AnyElement| {
            pro_tactile(
                pro_eid(&self.id, btn_id),
                label.clone(),
                6.,
                card_bg,
                p.hover,
                p.line_strong,
                p.line_strong,
                false,
                div()
                    .w_full()
                    .h(px(32.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(icon_el)
                    .child(div().truncate().child(label)),
                cx,
                move |v, _, cx| {
                    v.trigger_preset(preset, cx);
                },
            )
            .flex_1()
            .min_w_0()
        };

        let trigger_section = if self.show_triggers {
            let btn_default = build_trigger_btn(
                "trig-default",
                "default",
                tr(cx, "Default", "默认通知").into(),
                pro_icon(ProIcon::Link, 15., p.muted),
            );
            let btn_success = build_trigger_btn(
                "trig-success",
                "success",
                tr(cx, "Success", "成功保存").into(),
                pro_svg("0 0 24 24", ICON_FLOPPY, 15., p.muted),
            );
            let btn_action = build_trigger_btn(
                "trig-action",
                "action",
                tr(cx, "Action / Undo", "带撤销动作").into(),
                pro_icon(ProIcon::Trash, 15., p.muted),
            );
            let btn_error = build_trigger_btn(
                "trig-error",
                "error",
                tr(cx, "Error", "异常告警").into(),
                pro_svg("0 0 24 24", ICON_SYNC_ARROWS, 15., p.muted),
            );
            let btn_promise = build_trigger_btn(
                "trig-promise",
                "promise",
                tr(
                    cx,
                    "Promise (Saving → Saved)",
                    "异步状态流转 (保存中 → 已保存)",
                )
                .into(),
                pro_icon(ProIcon::Upload, 15., p.blue),
            );

            Some(
                div()
                    .w(px(CARD_WIDTH))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .w_full()
                            .p(px(13.))
                            .rounded(px(10.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(card_border)
                            .shadow(pro_card_shadow(p.dark))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .size(px(35.))
                                            .flex_none()
                                            .rounded(px(7.))
                                            .bg(p.subtle)
                                            .border_1()
                                            .border_color(p.line)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(pro_svg("0 0 24 24", ICON_DOC, 17., p.muted)),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .line_height(px(19.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .truncate()
                                                    .child(tr(
                                                        cx,
                                                        "Q3 Board Report",
                                                        "Q3 财务董事会报告",
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .truncate()
                                                    .child(tr(
                                                        cx,
                                                        "Finance · edited 2 min ago",
                                                        "财务部 · 2 分钟前编辑",
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(7.))
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(7.))
                                            .child(btn_default)
                                            .child(btn_success),
                                    )
                                    .child(
                                        div().flex().gap(px(7.)).child(btn_action).child(btn_error),
                                    )
                                    .child(div().flex().child(btn_promise)),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Press", "按"))
                            .child(
                                div()
                                    .h(px(18.))
                                    .px(px(5.))
                                    .rounded(px(4.))
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.))
                                    .text_color(p.ink)
                                    .child("F8"),
                            )
                            .child(tr(
                                cx,
                                "or hover stack to fan out · drag to dismiss",
                                "或悬停展开通知栈 · 拖拽滑动关闭",
                            )),
                    ),
            )
        } else {
            None
        };

        // Build stacked toast region (render back-to-front so index 0 paints on top!)
        let fanned = self.is_fanned_out();
        let paused = self.hovered_stack || self.keyboard_expanded || self.active_drag.is_some();
        let visible_n = self.toasts.len().min(self.visible_toasts);
        let stack_target_h = if visible_n == 0 {
            72.0
        } else if fanned {
            visible_n as f32 * CARD_HEIGHT + (visible_n.saturating_sub(1)) as f32 * FAN_GAP + 8.0
        } else {
            CARD_HEIGHT + (visible_n.saturating_sub(1)) as f32 * 14.0 + 12.0
        };
        if (self.last_stack_h - stack_target_h).abs() > 0.25 {
            self.last_stack_h = stack_target_h;
            self.stack_h_slide.duration(stack_target_h, 0.28, cx);
        }
        let stack_h = self.stack_h_slide.get(w, cx).max(72.0);

        if reduced {
            self.exiting_toasts.clear();
        } else {
            self.exiting_toasts
                .retain(|et| et.started_at.elapsed().as_secs_f32() < 0.25);
            if !self.exiting_toasts.is_empty() {
                w.request_animation_frame();
            }
        }

        let spin_secs = self.started_at.elapsed().as_secs_f32();
        let mut stack_container = div()
            .id(pro_eid(&self.id, "toast-stack-region"))
            .role(Role::Region)
            .aria_label(tr(cx, "Notifications (F8)", "通知提醒 (F8)"))
            .w(px(CARD_WIDTH))
            .max_w_full()
            .h(px(stack_h.max(228.)))
            .relative()
            .on_hover(cx.listener(|v, inside: &bool, _, cx| {
                if v.hovered_stack != *inside {
                    v.hovered_stack = *inside;
                    if !*inside {
                        v.hovered_card_id = None;
                    }
                    cx.emit(ToasterEvent::ExpandedChanged(v.is_fanned_out()));
                    cx.notify();
                }
            }));

        // Render exiting dismissed toasts smoothly sliding out via `Slide`
        struct ExitRenderSpec {
            id: u64,
            kind: ToastKind,
            title: SharedString,
            desc: Option<SharedString>,
            top: f32,
            scale: f32,
            dx: f32,
            opacity: f32,
        }
        let mut exit_specs = Vec::with_capacity(self.exiting_toasts.len());
        for et in &mut self.exiting_toasts {
            let u = et.exit_slide.get(w, cx).clamp(0.0, 1.0);
            exit_specs.push(ExitRenderSpec {
                id: et.spec.id,
                kind: et.spec.kind,
                title: et.spec.title.clone(),
                desc: et.spec.description.clone(),
                top: et.base_top - 4.0 * u,
                scale: (et.base_scale * (1.0 - 0.04 * u)).clamp(0.82, 1.0),
                dx: et.start_dx + et.travel_dx * u,
                opacity: (1.0 - u).clamp(0.0, 1.0),
            });
        }

        for ex in exit_specs {
            let ex_eid = pro_eid(&self.id, format!("toast-exit-{}", ex.id));
            let inset_x = ((1.0 - ex.scale) * CARD_WIDTH * 0.5).max(0.);
            let card_h = CARD_HEIGHT * (0.90 + 0.10 * ex.scale);
            let status_icon =
                render_toast_status_icon(pro_eid(&ex_eid, "icon"), ex.kind, spin_secs, &p, w, cx);
            stack_container = stack_container.child(
                div()
                    .absolute()
                    .top(px(ex.top))
                    .left(px(inset_x + ex.dx))
                    .w(px((CARD_WIDTH - 2. * inset_x).max(160.)))
                    .h(px(card_h))
                    .opacity(ex.opacity)
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow_md()
                    .overflow_hidden()
                    .child(
                        div()
                            .size_full()
                            .px(px(12.))
                            .py(px(11.))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(status_icon)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(19.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(ex.title),
                                    )
                                    .when_some(ex.desc, |d, desc| {
                                        d.child(
                                            div()
                                                .text_size(px(12.5))
                                                .line_height(px(18.))
                                                .text_color(p.text)
                                                .truncate()
                                                .child(desc),
                                        )
                                    }),
                            ),
                    ),
            );
        }

        // Collect snapshot data and step per-card `Slide` springs (`0.94 -> 1.0` scale & vertical offset)
        struct CardRenderSpec {
            id: u64,
            slot: usize,
            kind: ToastKind,
            title: SharedString,
            desc: Option<SharedString>,
            action: Option<SharedString>,
            timer_frac: Option<f32>,
            drag_dx: f32,
            anim_top: f32,
            anim_scale: f32,
            anim_content: f32,
        }

        let mut specs_rev: Vec<CardRenderSpec> = Vec::with_capacity(visible_n);
        for slot in (0..visible_n).rev() {
            let target_top = if fanned {
                slot as f32 * (CARD_HEIGHT + FAN_GAP)
            } else {
                slot as f32 * 13.0
            };
            // Stacked scale: slot 0 = 1.00, slot 1 = 0.94, slot 2 = 0.88 -> fans out to 1.00!
            let target_scale = if fanned {
                1.0
            } else {
                (1.0 - slot as f32 * 0.06).max(0.84)
            };
            let target_alpha = if fanned || slot == 0 { 1.0 } else { 0.0 };

            let t = &mut self.toasts[slot];
            if (t.last_targets.0 - target_top).abs() > 0.1
                || (t.last_targets.1 - target_scale).abs() > 0.002
                || (t.last_targets.2 - target_alpha).abs() > 0.01
            {
                t.last_targets = (target_top, target_scale, target_alpha);
                t.top_slide.duration(target_top, 0.28, cx);
                t.scale_slide.duration(target_scale, 0.28, cx);
                t.alpha_slide.duration(target_alpha, 0.22, cx);
            }
            let anim_top = t.top_slide.get(w, cx);
            let anim_scale = t.scale_slide.get(w, cx).clamp(0.80, 1.04);
            let anim_content = t.alpha_slide.get(w, cx).clamp(0.0, 1.0);

            let release_dx = t.swipe_release.get(w, cx);
            let drag_dx = if let Some(d) = self.active_drag
                && d.toast_id == t.spec.id
            {
                d.current_dx
            } else {
                release_dx
            };
            let timer_frac = if self.show_hairline
                && !matches!(t.spec.kind, ToastKind::Promise { resolved: false })
                && let Some(dur_ms) = t.spec.duration_ms
            {
                Some((1.0 - t.active_elapsed_ms / dur_ms as f32).clamp(0., 1.))
            } else {
                None
            };
            specs_rev.push(CardRenderSpec {
                id: t.spec.id,
                slot,
                kind: t.spec.kind,
                title: t.spec.title.clone(),
                desc: t.spec.description.clone(),
                action: t.spec.action_label.clone(),
                timer_frac,
                drag_dx,
                anim_top,
                anim_scale,
                anim_content,
            });
        }

        for card in specs_rev {
            let anim_top = card.anim_top;
            let anim_scale = card.anim_scale;
            let anim_content = card.anim_content;
            let card_eid = pro_eid(&self.id, format!("toast-{}", card.id));

            // Tonal elevation steps across stacked background toasts (`#222226` -> `#1c1c20` -> `#17171b` in dark)
            let slot_bg: Hsla = if p.dark {
                let stacked_hex = match card.slot {
                    0 => 0x222226,
                    1 => 0x1c1c20,
                    _ => 0x17171b,
                };
                if fanned && card.slot > 0 {
                    rgb(0x222226).into()
                } else {
                    rgb(stacked_hex).into()
                }
            } else {
                let stacked_hex = match card.slot {
                    0 => 0xffffff,
                    1 => 0xfafafb,
                    _ => 0xf3f3f5,
                };
                if fanned && card.slot > 0 {
                    rgb(0xffffff).into()
                } else {
                    rgb(stacked_hex).into()
                }
            };
            let slot_border: Hsla = if p.dark {
                hsla(0., 0., 1., 0.09)
            } else {
                hsla(0., 0., 0., 0.08)
            };

            let inset_x = ((1.0 - anim_scale) * CARD_WIDTH * 0.5).max(0.);
            let card_h = CARD_HEIGHT * (0.90 + 0.10 * anim_scale);
            let swipe_fade = (1.0 - card.drag_dx.abs() / 190.0).clamp(0.18, 1.0);

            let status_icon = render_toast_status_icon(
                pro_eid(&card_eid, "icon"),
                card.kind,
                spin_secs,
                &p,
                w,
                cx,
            );

            let copy_key: SharedString = format!("{}-{:?}", card.title, card.desc).into();
            let text_block = pro_view_transition(
                pro_eid(&card_eid, "copy-vtx"),
                copy_key,
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(card.title.clone()),
                    )
                    .when_some(card.desc.clone(), |d, desc| {
                        d.child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.text)
                                .truncate()
                                .child(desc),
                        )
                    }),
            );

            let id_for_action = card.id;
            let action_el = card.action.clone().map(|act_lbl| {
                let lbl_for_cb = act_lbl.clone();
                let act_rest: Hsla = if p.dark {
                    hsla(0., 0., 1., 0.07)
                } else {
                    p.card
                };
                div()
                    .flex_none()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        pro_tactile(
                            pro_eid(&card_eid, "act-btn"),
                            act_lbl.clone(),
                            6.,
                            act_rest,
                            p.chip_hover,
                            p.line_strong,
                            p.line_strong,
                            false,
                            div()
                                .h(px(27.))
                                .px(px(10.))
                                .flex()
                                .items_center()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .whitespace_nowrap()
                                .child(act_lbl),
                            cx,
                            move |v, _, cx| {
                                v.handle_toast_action(id_for_action, lbl_for_cb.clone(), cx);
                            },
                        )
                        .flex_none(),
                    )
            });

            let is_card_hovered = self.hovered_card_id == Some(card.id);
            let close_lbl: SharedString = tr(cx, "Dismiss notification", "关闭通知").into();
            let id_for_close = card.id;
            let id_for_hover = card.id;
            let id_for_down = card.id;

            let close_btn = div()
                .id(pro_eid(&card_eid, "close-btn"))
                .role(Role::Button)
                .aria_label(close_lbl)
                .tab_index(0)
                .size(px(20.))
                .flex_none()
                .rounded_full()
                .opacity(if is_card_hovered && anim_content > 0.6 {
                    1.0
                } else {
                    0.0
                })
                .bg(if p.dark {
                    Hsla::from(rgb(0x2a2a2d))
                } else {
                    p.subtle
                })
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.chip_hover))
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .on_click(cx.listener(move |v, _, _, cx| {
                    cx.stop_propagation();
                    v.dismiss(id_for_close, cx);
                }))
                .child(pro_icon(ProIcon::Close, 11., p.muted));

            let card_el = div()
                .id(card_eid)
                .tab_index(0)
                .absolute()
                .top(px(anim_top))
                .left(px(inset_x + card.drag_dx))
                .w(px((CARD_WIDTH - 2. * inset_x).max(160.)))
                .h(px(card_h))
                .opacity(swipe_fade)
                .rounded(px(10.))
                .bg(slot_bg)
                .border_1()
                .border_color(slot_border)
                .shadow(pro_card_shadow(p.dark))
                .overflow_hidden()
                .cursor(if self.active_drag.is_some_and(|d| d.toast_id == card.id) {
                    CursorStyle::ClosedHand
                } else {
                    CursorStyle::OpenHand
                })
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |v, inside: &bool, _, cx| {
                    if *inside {
                        v.hovered_card_id = Some(id_for_hover);
                    } else if v.hovered_card_id == Some(id_for_hover) {
                        v.hovered_card_id = None;
                    }
                    cx.notify();
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |v, ev: &MouseDownEvent, _, cx| {
                        v.active_drag = Some(ActiveDrag {
                            toast_id: id_for_down,
                            start_x: f32::from(ev.position.x),
                            current_dx: 0.,
                        });
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .size_full()
                        .px(px(12.))
                        .py(px(11.))
                        .opacity(anim_content)
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .child(status_icon)
                        .child(div().flex_1().min_w_0().child(text_block))
                        .when_some(action_el, |d, act| d.child(act))
                        .child(close_btn),
                )
                .when_some(card.timer_frac, |d, frac| {
                    let bar_col = if paused {
                        p.blue.opacity(0.45)
                    } else {
                        hairline_col
                    };
                    d.child(
                        div()
                            .absolute()
                            .left(px(12.))
                            .right(px(12.))
                            .bottom_0()
                            .h(px(1.5))
                            .opacity(anim_content)
                            .child(div().h_full().w(relative(frac)).bg(bar_col)),
                    )
                });

            stack_container = stack_container.child(card_el);
        }

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(14.))
            .on_mouse_move(cx.listener(|v, ev: &MouseMoveEvent, _, cx| {
                if let Some(drag) = &mut v.active_drag {
                    if ev.pressed_button == Some(MouseButton::Left) {
                        drag.current_dx = f32::from(ev.position.x) - drag.start_x;
                        cx.notify();
                    } else {
                        v.finish_drag(cx);
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| {
                    v.finish_drag(cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, _, _, cx| {
                    v.finish_drag(cx);
                }),
            )
            .capture_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                match ev.keystroke.key.as_str() {
                    "f8" => {
                        v.keyboard_expanded = !v.keyboard_expanded;
                        cx.emit(ToasterEvent::ExpandedChanged(v.is_fanned_out()));
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "escape" => {
                        if v.keyboard_expanded {
                            v.keyboard_expanded = false;
                            cx.emit(ToasterEvent::ExpandedChanged(v.is_fanned_out()));
                            cx.notify();
                        } else {
                            v.dismiss_newest(cx);
                        }
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .when_some(trigger_section, |d, trig| d.child(trig))
            .child(stack_container)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{ToastItem, ToastKind, Toaster};
    use gpui::TestAppContext;

    #[gpui::test]
    fn push_undo_action_and_dismiss_stack_lifecycle(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let toaster = cx.new(|cx| Toaster::new("toaster-test", cx));
        toaster.update(cx, |t, cx| {
            assert_eq!(t.toast_count(), 3);
            let id = t.allocate_id();
            t.push_toast(
                ToastItem::new(id, "Saving…").with_promise_resolution(1200, "Saved", "Done"),
                cx,
            );
            assert_eq!(t.toast_count(), 4);
            assert!(matches!(
                t.toasts[0].spec.kind,
                ToastKind::Promise { resolved: false }
            ));

            assert!(t.dismiss(id, cx));
            assert_eq!(t.toast_count(), 3);

            // Triggering Undo action dismisses target toast and pushes restoration confirmation
            t.handle_toast_action(2, "Undo".into(), cx);
            assert_eq!(t.toast_count(), 3);
            assert!(matches!(t.toasts[0].spec.kind, ToastKind::Success));
        });
    }
}
