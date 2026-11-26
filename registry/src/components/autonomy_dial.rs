//! Three-stop agent autonomy selector with gliding action groups and padlock overrides.
//!
//! Lets people configure how much an agent may do without asking (`Always ask`, `If risky`,
//! `Full auto`) while real workspace actions smoothly glide between `"Runs on its own"` and
//! `"Asks you first"`, with per-action padlocks and non-negotiable red-line locks.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_eid, pro_empty_state, pro_fold, pro_mark_selected,
        pro_mix_color, pro_pill_indicator, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        direct_touch::Contact,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    f32::consts::{FRAC_PI_2, PI},
    rc::Rc,
};
use web_time::Instant;

fn sparkle_header_icon(side: f32, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, win, _| {
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            if w <= 2.0 || h <= 2.0 {
                return;
            }
            let ox = f32::from(bounds.left());
            let oy = f32::from(bounds.top());
            let map = |u: f32, v: f32| point(px(ox + u * w), px(oy + v * h));
            let cx_u = 0.48_f32;
            let cy_v = 0.44_f32;
            let r_outer = 0.37_f32;
            let r_inner = 0.105_f32;
            let d = r_inner * 0.72;
            let mut pb = PathBuilder::fill();
            let pts = [
                (cx_u, cy_v - r_outer),
                (cx_u + d, cy_v - d),
                (cx_u + r_outer, cy_v),
                (cx_u + d, cy_v + d),
                (cx_u, cy_v + r_outer),
                (cx_u - d, cy_v + d),
                (cx_u - r_outer, cy_v),
                (cx_u - d, cy_v - d),
            ];
            pb.move_to(map(pts[0].0, pts[0].1));
            for i in 0..4 {
                let ctrl = pts[i * 2 + 1];
                let next = pts[(i * 2 + 2) % 8];
                pb.curve_to(map(next.0, next.1), map(ctrl.0, ctrl.1));
            }
            pb.close();
            if let Ok(path) = pb.build() {
                win.paint_path(path, color);
            }
            let dot_r = (w * 0.085).max(1.1);
            let dot_c = map(0.82, 0.80);
            win.paint_quad(
                fill(
                    Bounds::new(
                        point(dot_c.x - px(dot_r), dot_c.y - px(dot_r)),
                        size(px(dot_r * 2.0), px(dot_r * 2.0)),
                    ),
                    color.opacity(0.62),
                )
                .corner_radii(px(dot_r)),
            );
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn warn_triangle_icon(side: f32, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, win, _| {
            let s = f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 24.0;
            if s <= 0.05 {
                return;
            }
            let ox = f32::from(bounds.left());
            let oy = f32::from(bounds.top());
            let pt = |x: f32, y: f32| point(px(ox + x * s), px(oy + y * s));
            let sw = (1.75 * s).clamp(1.05, 1.8);

            let mut tri = PathBuilder::stroke(px(sw));
            tri.move_to(pt(10.6, 4.4));
            tri.curve_to(pt(13.4, 4.4), pt(12.0, 2.3));
            tri.line_to(pt(20.6, 17.2));
            tri.curve_to(pt(19.1, 20.2), pt(21.6, 19.8));
            tri.line_to(pt(4.9, 20.2));
            tri.curve_to(pt(3.4, 17.2), pt(2.4, 19.8));
            tri.close();
            if let Ok(path) = tri.build() {
                win.paint_path(path, color);
            }

            let mut stem = PathBuilder::stroke(px(sw));
            stem.move_to(pt(12.0, 9.2));
            stem.line_to(pt(12.0, 13.5));
            if let Ok(path) = stem.build() {
                win.paint_path(path, color);
            }

            let r = (1.15 * s).max(0.85);
            let dc = pt(12.0, 16.7);
            win.paint_quad(
                fill(
                    Bounds::new(
                        point(dc.x - px(r), dc.y - px(r)),
                        size(px(r * 2.0), px(r * 2.0)),
                    ),
                    color,
                )
                .corner_radii(px(r)),
            );
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn danger_diamond_icon(side: f32, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, win, _| {
            let s = f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 24.0;
            if s <= 0.05 {
                return;
            }
            let ox = f32::from(bounds.left());
            let oy = f32::from(bounds.top());
            let pt = |x: f32, y: f32| point(px(ox + x * s), px(oy + y * s));
            let sw = (1.75 * s).clamp(1.05, 1.8);

            let mut dia = PathBuilder::stroke(px(sw));
            dia.move_to(pt(10.3, 3.5));
            dia.curve_to(pt(13.7, 3.5), pt(12.0, 1.8));
            dia.line_to(pt(20.5, 10.3));
            dia.curve_to(pt(20.5, 13.7), pt(22.2, 12.0));
            dia.line_to(pt(13.7, 20.5));
            dia.curve_to(pt(10.3, 20.5), pt(12.0, 22.2));
            dia.line_to(pt(3.5, 13.7));
            dia.curve_to(pt(3.5, 10.3), pt(1.8, 12.0));
            dia.close();
            if let Ok(path) = dia.build() {
                win.paint_path(path, color);
            }

            let mut stem = PathBuilder::stroke(px(sw));
            stem.move_to(pt(12.0, 8.1));
            stem.line_to(pt(12.0, 12.6));
            if let Ok(path) = stem.build() {
                win.paint_path(path, color);
            }

            let r = (1.15 * s).max(0.85);
            let dc = pt(12.0, 15.9);
            win.paint_quad(
                fill(
                    Bounds::new(
                        point(dc.x - px(r), dc.y - px(r)),
                        size(px(r * 2.0), px(r * 2.0)),
                    ),
                    color,
                )
                .corner_radii(px(r)),
            );
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn padlock_icon(side: f32, locked: bool, color: Hsla) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, win, _| {
            let s = f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 24.0;
            if s <= 0.05 {
                return;
            }
            let ox = f32::from(bounds.left());
            let oy = f32::from(bounds.top());
            let pt = |x: f32, y: f32| point(px(ox + x * s), px(oy + y * s));
            let sw = (1.65 * s).clamp(1.1, 1.7);

            win.paint_quad(quad(
                Bounds::new(pt(4.5, 9.5), size(px(15.0 * s), px(11.8 * s))),
                Corners::all(px(3.6 * s)),
                transparent_black(),
                Edges::all(px(sw)),
                color,
                BorderStyle::Solid,
            ));

            let mut shackle = PathBuilder::stroke(px(sw));
            shackle.move_to(pt(7.7, 9.5));
            shackle.line_to(pt(7.7, 6.6));
            let end_a = if locked { 0.0 } else { -0.45 };
            let steps = 12usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let a = -PI + t * (PI + end_a);
                shackle.line_to(pt(12.0 + 4.3 * a.cos(), 6.6 + 4.3 * a.sin()));
            }
            if locked {
                let _ = FRAC_PI_2;
                shackle.line_to(pt(16.3, 9.5));
            }
            if let Ok(path) = shackle.build() {
                win.paint_path(path, color);
            }

            let kr = (1.05 * s).max(0.8);
            let kc = pt(12.0, 14.3);
            win.paint_quad(
                fill(
                    Bounds::new(
                        point(kc.x - px(kr), kc.y - px(kr)),
                        size(px(kr * 2.0), px(kr * 2.0)),
                    ),
                    color,
                )
                .corner_radii(px(kr)),
            );
            let mut key_stem = PathBuilder::stroke(px(sw));
            key_stem.move_to(pt(12.0, 14.6));
            key_stem.line_to(pt(12.0, 17.4));
            if let Ok(path) = key_stem.build() {
                win.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

/// Three-stop autonomy tier for [`AutonomyDial`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutonomyLevel {
    /// Every action asks the user before running (`0`).
    AlwaysAsk,
    /// Safe actions run automatically; risky and destructive actions ask first (`1`).
    #[default]
    IfRisky,
    /// Every action runs automatically unless locked (`2`).
    FullAuto,
}

impl AutonomyLevel {
    const ALL: [Self; 3] = [Self::AlwaysAsk, Self::IfRisky, Self::FullAuto];

    fn key(self) -> &'static str {
        match self {
            Self::AlwaysAsk => "always-ask",
            Self::IfRisky => "if-risky",
            Self::FullAuto => "full-auto",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::AlwaysAsk => 0,
            Self::IfRisky => 1,
            Self::FullAuto => 2,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::AlwaysAsk => tr(cx, "Always ask", "每次询问"),
            Self::IfRisky => tr(cx, "If risky", "有风险时询问"),
            Self::FullAuto => tr(cx, "Full auto", "全自动"),
        }
    }
}

/// Risk classification of an [`AutonomyAction`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutonomyActionRisk {
    /// Read-only or draft action that stays quiet without a warning pill.
    #[default]
    Safe,
    /// External write or outbound communication (`Risky` amber pill).
    Risky,
    /// Irreversible deletion, production mutation, or spend (`Destructive` red pill).
    Destructive,
}

/// Workspace action governed by [`AutonomyDial`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutonomyAction {
    /// Stable action identifier (for example `"spend"`).
    pub id: SharedString,
    /// Human-readable action title (for example `"Spend money"`).
    pub label: SharedString,
    /// Optional detail subtitle (for example `"Up to $500"`).
    pub detail: Option<SharedString>,
    /// Inherent risk tier (`Safe`, `Risky`, `Destructive`).
    pub risk: AutonomyActionRisk,
    /// App or provider mark key (such as `"notion"`, `"gmail"`, `"hubspot"`, `"drive"`, `"stripe"`).
    pub app: Option<SharedString>,
    /// Fallback icon when `app` is `None`.
    pub icon: ProIcon,
    /// Non-negotiable policy red-line that always asks first and cannot be unlocked.
    pub red_line: bool,
}

impl AutonomyAction {
    /// Construct an [`AutonomyAction`] with `id`, `label`, and `risk`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        risk: AutonomyActionRisk,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: None,
            risk,
            app: None,
            icon: ProIcon::Bolt,
            red_line: false,
        }
    }

    /// Attach a secondary detail string.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Attach a multicolor app brand mark (`"notion"`, `"gmail"`, `"hubspot"`, `"drive"`, `"stripe"`, `"slack"`).
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }

    /// Set a fallback [`ProIcon`] when no app brand mark is configured.
    pub fn with_icon(mut self, icon: ProIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Mark this action as a non-negotiable red-line policy that always asks first.
    pub fn with_red_line(mut self, red_line: bool) -> Self {
        self.red_line = red_line;
        self
    }
}

/// Evaluate whether `action` must ask the user first under `level` and `pinned` locks.
///
/// Mirrors the runtime rule so UI settings and agent execution stay in lockstep:
/// - `action.red_line` or `pinned.contains(&action.id)` -> always asks first.
/// - [`AutonomyLevel::AlwaysAsk`] -> asks for every action.
/// - [`AutonomyLevel::IfRisky`] -> runs [`AutonomyActionRisk::Safe`] actions on its own; asks for risky/destructive.
/// - [`AutonomyLevel::FullAuto`] -> runs everything that is not pinned or red-lined.
pub fn asks_first(
    action: &AutonomyAction,
    level: AutonomyLevel,
    pinned: &HashSet<SharedString>,
) -> bool {
    if action.red_line || pinned.contains(&action.id) {
        return true;
    }
    match level {
        AutonomyLevel::AlwaysAsk => true,
        AutonomyLevel::IfRisky => action.risk != AutonomyActionRisk::Safe,
        AutonomyLevel::FullAuto => false,
    }
}

/// Events emitted by [`AutonomyDial`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutonomyDialEvent {
    /// User switched the three-stop autonomy level.
    LevelChanged(AutonomyLevel),
    /// User locked or unlocked an individual action (`pinned` is the updated lock set).
    PinnedChanged {
        /// Action identifier that was toggled.
        action_id: SharedString,
        /// Whether the action is now locked to `"Asks you first"`.
        locked: bool,
        /// Sorted list of all currently pinned action ids.
        pinned: Vec<SharedString>,
    },
}

impl EventEmitter<AutonomyDialEvent> for AutonomyDial {}

fn default_actions() -> Vec<AutonomyAction> {
    vec![
        AutonomyAction::new("read", "Read files and docs", AutonomyActionRisk::Safe)
            .with_app("notion"),
        AutonomyAction::new("draft", "Draft emails", AutonomyActionRisk::Safe).with_app("gmail"),
        AutonomyAction::new("crm", "Update CRM records", AutonomyActionRisk::Risky)
            .with_app("hubspot"),
        AutonomyAction::new("send", "Send emails", AutonomyActionRisk::Risky).with_app("gmail"),
        AutonomyAction::new("delete", "Delete files", AutonomyActionRisk::Destructive)
            .with_app("google drive"),
        AutonomyAction::new("spend", "Spend money", AutonomyActionRisk::Destructive)
            .with_app("stripe")
            .with_red_line(false),
    ]
}

/// Three-stop agent autonomy dial with gliding action lists and per-action padlock overrides.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::autonomy_dial::{AutonomyDial, AutonomyDialEvent, AutonomyLevel};
///
/// let dial = cx.new(|cx| {
///     AutonomyDial::new("sales-agent-autonomy", cx)
///         .with_title("Sales agent")
///         .with_level(AutonomyLevel::IfRisky)
/// });
/// cx.subscribe(&dial, |_, event: &AutonomyDialEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct AutonomyDial {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    subtitle: SharedString,
    level: AutonomyLevel,
    actions: Vec<AutonomyAction>,
    pinned: HashSet<SharedString>,
    hovered_row: Option<SharedString>,
    row_moves: HashMap<SharedString, Slide>,
    seg_bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    drag_start: Option<(f32, AutonomyLevel)>,
    suppress_seg_click: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl AutonomyDial {
    /// Create an [`AutonomyDial`] preloaded with the Sales agent permission suite (`Spend money` locked).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let mut pinned = HashSet::new();
        pinned.insert("spend".into());
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Sales agent".into(),
            subtitle: "What it may do without asking you".into(),
            level: AutonomyLevel::FullAuto,
            actions: default_actions(),
            pinned,
            hovered_row: None,
            row_moves: HashMap::new(),
            seg_bounds: Rc::new(Cell::new(Bounds::default())),
            contact: Contact::default(),
            drag_start: None,
            suppress_seg_click: false,
            measured_width: Rc::new(Cell::new(400.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Set the card heading (for example `"Sales agent"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the descriptive subtitle under the heading.
    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Set the starting [`AutonomyLevel`].
    pub fn with_level(mut self, level: AutonomyLevel) -> Self {
        self.level = level;
        self
    }

    /// Replace the governed [`AutonomyAction`] list.
    pub fn with_actions(mut self, actions: Vec<AutonomyAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Set the starting set of pinned action ids that always ask first.
    pub fn with_pinned<I, S>(mut self, pinned: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.pinned = pinned.into_iter().map(Into::into).collect();
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Current [`AutonomyLevel`].
    pub fn level(&self) -> AutonomyLevel {
        self.level
    }

    /// Whether `action_id` is currently pinned with a padlock.
    pub fn is_pinned(&self, action_id: &str) -> bool {
        self.pinned.contains(action_id)
    }

    /// Count of `(runs_on_its_own, asks_you_first)` under the current level and locks.
    pub fn partition_counts(&self) -> (usize, usize) {
        let mut auto_n = 0usize;
        let mut ask_n = 0usize;
        for act in &self.actions {
            if asks_first(act, self.level, &self.pinned) {
                ask_n += 1;
            } else {
                auto_n += 1;
            }
        }
        (auto_n, ask_n)
    }

    /// Set the active [`AutonomyLevel`] and animate any actions crossing between groups.
    pub fn set_level(&mut self, next: AutonomyLevel, cx: &mut Context<Self>) {
        if self.level == next {
            return;
        }
        let prev = self.level;
        self.level = next;
        for act in &self.actions {
            let was_ask = asks_first(act, prev, &self.pinned);
            let now_ask = asks_first(act, next, &self.pinned);
            if was_ask != now_ask {
                let start_dy = if now_ask { -14.0 } else { 14.0 };
                let slide = self
                    .row_moves
                    .entry(act.id.clone())
                    .or_insert_with(|| Slide::new(0.0));
                slide.reset(start_dy);
                slide.duration(0.0, 0.28, cx);
            }
        }
        cx.emit(AutonomyDialEvent::LevelChanged(next));
        cx.notify();
    }

    fn level_at_x(&self, x_px: f32) -> Option<AutonomyLevel> {
        let b = self.seg_bounds.get();
        let w_px = f32::from(b.size.width);
        if w_px <= 4.0 {
            return None;
        }
        let left_px = f32::from(b.left());
        let frac = ((x_px - left_px) / w_px).clamp(0.0, 0.9999);
        let idx = (frac * 3.0).floor() as usize;
        Some(AutonomyLevel::ALL[idx.min(2)])
    }

    fn begin_scrub(&mut self, x: Pixels, cx: &mut Context<Self>) {
        self.suppress_seg_click = false;
        let x_px = f32::from(x);
        self.drag_start = Some((x_px, self.level));
        if let Some(lv) = self.level_at_x(x_px) {
            self.set_level(lv, cx);
        }
        cx.notify();
    }

    fn move_scrub(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some((start_x, _)) = self.drag_start else {
            return;
        };
        let x_px = f32::from(x);
        if (x_px - start_x).abs() > 3.5 {
            self.suppress_seg_click = true;
        }
        if let Some(lv) = self.level_at_x(x_px) {
            self.set_level(lv, cx);
        }
    }

    fn end_scrub(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        let Some((_, initial)) = self.drag_start.take() else {
            return;
        };
        if cancelled {
            self.set_level(initial, cx);
        } else {
            cx.notify();
        }
    }

    /// Toggle the padlock override on `action_id` (unless marked `red_line`).
    pub fn toggle_pin(&mut self, action_id: &str, cx: &mut Context<Self>) {
        let Some(act) = self.actions.iter().find(|a| a.id.as_ref() == action_id) else {
            return;
        };
        if act.red_line {
            let msg: SharedString = tr(
                cx,
                "Red-line policy: this action always asks first",
                "红线安全策略：此操作必须始终询问确认",
            )
            .into();
            self.toast = Some((msg, Instant::now()));
            cx.notify();
            return;
        }
        let was_ask = asks_first(act, self.level, &self.pinned);
        let id_ss = act.id.clone();
        let locked = if self.pinned.contains(&id_ss) {
            self.pinned.remove(&id_ss);
            false
        } else {
            self.pinned.insert(id_ss.clone());
            true
        };
        let act_ref = self.actions.iter().find(|a| a.id == id_ss);
        if let Some(a) = act_ref {
            let now_ask = asks_first(a, self.level, &self.pinned);
            if was_ask != now_ask {
                let start_dy = if now_ask { -14.0 } else { 14.0 };
                let slide = self
                    .row_moves
                    .entry(id_ss.clone())
                    .or_insert_with(|| Slide::new(0.0));
                slide.reset(start_dy);
                slide.duration(0.0, 0.28, cx);
            }
        }
        let mut sorted: Vec<SharedString> = self.pinned.iter().cloned().collect();
        sorted.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
        cx.emit(AutonomyDialEvent::PinnedChanged {
            action_id: id_ss,
            locked,
            pinned: sorted,
        });
        cx.notify();
    }

    fn render_action_row(
        &self,
        act: &AutonomyAction,
        in_ask_group: bool,
        narrow: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let is_locked = act.red_line || self.pinned.contains(&act.id);
        let is_hovered = self.hovered_row.as_ref() == Some(&act.id);

        let dy_offset = if disabled_motion(cx) {
            0.0
        } else if let Some(slide) = self.row_moves.get(&act.id) {
            slide.get(w, cx)
        } else {
            0.0
        };

        let icon_el = if let Some(app_name) = &act.app
            && let Some(mark) = pro_app_artwork(app_name.as_ref(), 16.5, cx)
        {
            div()
                .size(px(17.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(mark)
                .into_any_element()
        } else {
            div()
                .size(px(17.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(act.icon, 15.0, p.muted))
                .into_any_element()
        };

        let label_localized: SharedString = match act.label.as_ref() {
            "Read files and docs" => tr(cx, "Read files and docs", "读取文档与知识库").into(),
            "Draft emails" => tr(cx, "Draft emails", "起草邮件草稿").into(),
            "Update CRM records" => tr(cx, "Update CRM records", "更新 CRM 客户记录").into(),
            "Send emails" => tr(cx, "Send emails", "对外发送邮件").into(),
            "Delete files" => tr(cx, "Delete files", "删除云端文件").into(),
            "Spend money" => tr(cx, "Spend money", "执行资金扣款").into(),
            _ => act.label.clone(),
        };

        let risk_pill = match act.risk {
            AutonomyActionRisk::Safe => None,
            AutonomyActionRisk::Risky => {
                let (bg, fg): (Hsla, Hsla) = if p.dark {
                    (hsla(26.0 / 360.0, 0.96, 0.49, 0.16), rgb(0xffb35c).into())
                } else {
                    (hsla(37.0 / 360.0, 1.0, 0.50, 0.16), rgb(0xb25e00).into())
                };
                Some(
                    div()
                        .h(px(18.0))
                        .pl(px(4.5))
                        .pr(px(6.0))
                        .flex_none()
                        .rounded(px(4.0))
                        .bg(bg)
                        .text_color(fg)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap(px(3.0))
                        .child(warn_triangle_icon(12.0, fg))
                        .child(tr(cx, "Risky", "有风险")),
                )
            }
            AutonomyActionRisk::Destructive => {
                let (bg, fg): (Hsla, Hsla) = if p.dark {
                    (hsla(5.0 / 360.0, 1.0, 0.57, 0.17), rgb(0xff8a7a).into())
                } else {
                    (hsla(10.0 / 360.0, 0.96, 0.48, 0.10), rgb(0xc4320a).into())
                };
                Some(
                    div()
                        .h(px(18.0))
                        .pl(px(4.5))
                        .pr(px(6.0))
                        .flex_none()
                        .rounded(px(4.0))
                        .bg(bg)
                        .text_color(fg)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap(px(3.0))
                        .child(danger_diamond_icon(12.0, fg))
                        .child(tr(cx, "Destructive", "破坏性")),
                )
            }
        };

        let show_pin = is_locked || is_hovered;
        let pin_color = if act.red_line {
            p.red_ink
        } else if is_locked {
            p.muted
        } else {
            p.faint
        };
        let pin_aria: SharedString = if is_locked {
            format!(
                "{}: {}",
                act.label,
                tr(cx, "always asks — unlock", "始终询问 — 点击解锁")
            )
            .into()
        } else {
            format!(
                "{}: {}",
                tr(cx, "Always ask before", "设为始终询问"),
                act.label
            )
            .into()
        };

        let act_id_hover = act.id.clone();
        let act_id_pin = act.id.clone();
        let group_tag = if in_ask_group { "ask" } else { "auto" };
        let wash_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.04)
        } else {
            hsla(0.0, 0.0, 0.0, 0.03)
        };

        let row_eid = pro_eid(&self.id, format!("row-{group_tag}-{}", act.id));
        let row_hov_st = w.use_keyed_state(pro_eid(&row_eid, "rhov"), cx, |_, _| {
            (
                is_hovered,
                Slide::snappy(f32::from(is_hovered)),
                Slide::snappy(0.0),
            )
        });
        let (hov_t, pin_down) = row_hov_st.update(cx, |s, cx| {
            if s.0 != is_hovered {
                s.0 = is_hovered;
                s.1.set(f32::from(is_hovered), cx);
            }
            (
                s.1.get(w, cx).clamp(0.0, 1.0),
                s.2.get(w, cx).clamp(0.0, 1.0),
            )
        });
        let row_bg = pro_mix_color(transparent_black(), wash_bg, hov_t);
        let pin_alpha = if is_locked { 1.0 } else { hov_t };

        div()
            .id(row_eid)
            .relative()
            .top(px(dy_offset))
            .w_full()
            .min_h(px(32.0))
            .pl(px(6.0))
            .pr(px(4.0))
            .rounded(px(6.0))
            .bg(row_bg)
            .flex()
            .items_center()
            .gap(px(11.0))
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_row = Some(act_id_hover.clone());
                } else if this.hovered_row.as_ref() == Some(&act_id_hover) {
                    this.hovered_row = None;
                }
                cx.notify();
            }))
            .child(icon_el)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(label_localized),
                            )
                            .when_some(risk_pill, |d, pill| d.child(pill)),
                    )
                    .when(!narrow, |d| {
                        d.when_some(act.detail.clone(), |d, det| {
                            d.child(
                                div()
                                    .text_size(px(11.5))
                                    .line_height(px(15.0))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(det),
                            )
                        })
                    }),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, format!("pin-{group_tag}-{}", act.id)))
                    .role(Role::Button)
                    .aria_label(pin_aria)
                    .tab_index(0)
                    .size(px(28.0))
                    .flex_none()
                    .rounded(px(6.0))
                    .opacity(if show_pin {
                        pin_alpha.max(0.18)
                    } else {
                        pin_alpha
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.opacity(1.0).bg(p.chip))
                    .focus_visible(move |s| s.opacity(1.0).border_1().border_color(p.focus))
                    .on_mouse_down(MouseButton::Left, {
                        let row_hov_st = row_hov_st.clone();
                        move |_, w, cx| {
                            row_hov_st.update(cx, |s, cx| {
                                s.2.set(1.0, cx);
                                cx.notify();
                            });
                            w.refresh();
                            cx.stop_propagation();
                        }
                    })
                    .on_mouse_up(MouseButton::Left, {
                        let row_hov_st = row_hov_st.clone();
                        move |_, w, cx| {
                            row_hov_st.update(cx, |s, cx| {
                                s.2.set(0.0, cx);
                                cx.notify();
                            });
                            w.refresh();
                        }
                    })
                    .on_mouse_up_out(MouseButton::Left, {
                        let row_hov_st = row_hov_st.clone();
                        move |_, w, cx| {
                            row_hov_st.update(cx, |s, cx| {
                                s.2.set(0.0, cx);
                                cx.notify();
                            });
                            w.refresh();
                        }
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.toggle_pin(act_id_pin.as_ref(), cx);
                    }))
                    .child(div().relative().top(px(0.5 * pin_down)).child(padlock_icon(
                        16.0 * (1.0 - 0.08 * pin_down),
                        is_locked,
                        pin_color,
                    ))),
            )
            .into_any_element()
    }
}

impl Render for AutonomyDial {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);

        if self.loading {
            let phase = (Instant::now().elapsed().as_secs_f32() * 0.5).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(24.0))
                .px(px(16.0))
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(400.0))
                        .p(px(16.0))
                        .rounded(px(12.0))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(pro_skeleton_bar(160.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(360.0, 28.0, 6.0, phase, cx))
                        .child(pro_skeleton_bar(360.0, 160.0, 8.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.actions.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Sliders,
                    tr(cx, "No agent actions configured", "暂未配置智能体动作"),
                    tr(
                        cx,
                        "Provide actions to preview which operations run automatically at each autonomy level.",
                        "传入智能体可执行动作列表，以实时预览不同自主等级下的审批边界。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let cw = self.measured_width.get();
        let narrow = cw < 340.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let seg_unselected_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.08)
        } else {
            hsla(0.0, 0.0, 0.0, 0.055)
        };

        let title_str: SharedString = if self.title.as_ref() == "Sales agent" {
            tr(cx, "Sales agent", "销售智能体").into()
        } else {
            self.title.clone()
        };
        let sub_str: SharedString = if self.subtitle.as_ref() == "What it may do without asking you"
        {
            tr(
                cx,
                "What it may do without asking you",
                "无需提前询问即可自主执行的操作范围",
            )
            .into()
        } else {
            self.subtitle.clone()
        };

        // Three-stop segmented selector with direct pointer/touch drag-scrubbing + `pro_pill_indicator` liquid stretch
        let sensor = pro_selection_sensor();
        let cur_level = self.level;
        let bounds_cell = self.seg_bounds.clone();
        let mut seg_strip = div()
            .id(pro_eid(&self.id, "dial-scrub-bar"))
            .role(Role::RadioGroup)
            .aria_label(tr(cx, "Autonomy level", "自主等级"))
            .relative()
            .w_full()
            .p(px(2.5))
            .rounded(px(8.0))
            .bg(seg_unselected_bg)
            .flex()
            .items_center()
            .gap(px(2.0))
            .cursor_ew_resize()
            .child(
                canvas(move |b, _, _| bounds_cell.set(b), |_, _, _, _| ())
                    .absolute()
                    .inset(px(2.5)),
            )
            .child(pro_pill_indicator(
                pro_eid(&self.id, "dial-pill"),
                cur_level.key(),
                sensor.clone(),
                6.0,
                card_bg,
                Some(p.line_strong),
                true,
            ));

        for lv in AutonomyLevel::ALL {
            let selected = lv == cur_level;
            let lbl = lv.label(cx);
            seg_strip = seg_strip.child(
                div()
                    .id(pro_eid(&self.id, format!("lvl-{}", lv.key())))
                    .role(Role::RadioButton)
                    .aria_label(lbl)
                    .aria_toggled(selected.into())
                    .tab_index(if selected { 0 } else { -1 })
                    .relative()
                    .flex_1()
                    .h(px(26.0))
                    .px(px(if narrow { 6.0 } else { 8.0 }))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.strong } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| if selected { s } else { s.text_color(p.ink) })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if this.suppress_seg_click {
                            this.suppress_seg_click = false;
                            return;
                        }
                        this.set_level(lv, cx);
                    }))
                    .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                    .child(div().relative().child(lbl)),
            );
        }

        seg_strip = seg_strip
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    this.begin_scrub(ev.position.x, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.end_scrub(false, cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.end_scrub(false, cx);
                }),
            )
            .when(self.drag_start.is_some(), |v| {
                let moved = cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                    if ev.pressed_button == Some(MouseButton::Left) {
                        this.move_scrub(ev.position.x, cx);
                    }
                });
                let released = cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.end_scrub(false, cx);
                });
                v.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, win, _| {
                            win.on_mouse_event(move |ev: &MouseMoveEvent, phase, win, cx| {
                                if phase.capture() {
                                    moved(ev, win, cx);
                                }
                            });
                            win.on_mouse_event(move |ev: &MouseUpEvent, phase, win, cx| {
                                if phase.capture() && ev.button == MouseButton::Left {
                                    released(ev, win, cx);
                                }
                            });
                        },
                    )
                    .absolute()
                    .size(px(0.0)),
                )
            })
            .child(self.contact.listen(cx, |this, ev, _, cx| match ev.phase {
                TouchPhase::Started => this.begin_scrub(ev.position.x, cx),
                TouchPhase::Moved => this.move_scrub(ev.position.x, cx),
                TouchPhase::Ended => this.end_scrub(false, cx),
                TouchPhase::Cancelled => this.end_scrub(true, cx),
            }));

        let (auto_count, ask_count) = self.partition_counts();
        let actions_snapshot = self.actions.clone();

        // Build `"Runs on its own"` list with smooth per-action `pro_fold` so rows glide across groups
        let mut auto_list = div().w_full().flex().flex_col();
        for act in &actions_snapshot {
            let in_auto = !asks_first(act, self.level, &self.pinned);
            let row_el = self.render_action_row(act, false, narrow, w, cx);
            auto_list = auto_list.child(pro_fold(
                pro_eid(&self.id, format!("auto-f-{}", act.id)),
                in_auto,
                32.0,
                0.0,
                move || row_el,
            ));
        }
        let empty_auto_txt: SharedString = tr(
            cx,
            "Nothing runs without asking",
            "所有操作均需先征得你同意",
        )
        .into();
        auto_list = auto_list.child(pro_fold(
            pro_eid(&self.id, "auto-empty"),
            auto_count == 0,
            32.0,
            0.0,
            move || {
                div()
                    .w_full()
                    .min_h(px(30.0))
                    .my(px(2.0))
                    .px(px(10.0))
                    .rounded(px(6.0))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .child(empty_auto_txt)
                    .into_any_element()
            },
        ));

        // Build `"Asks you first"` list with smooth per-action `pro_fold`
        let mut ask_list = div().w_full().flex().flex_col();
        for act in &actions_snapshot {
            let in_ask = asks_first(act, self.level, &self.pinned);
            let row_el = self.render_action_row(act, true, narrow, w, cx);
            ask_list = ask_list.child(pro_fold(
                pro_eid(&self.id, format!("ask-f-{}", act.id)),
                in_ask,
                32.0,
                0.0,
                move || row_el,
            ));
        }
        let empty_ask_txt: SharedString =
            tr(cx, "Everything runs automatically", "全部动作均自动执行").into();
        ask_list = ask_list.child(pro_fold(
            pro_eid(&self.id, "ask-empty"),
            ask_count == 0,
            32.0,
            0.0,
            move || {
                div()
                    .w_full()
                    .min_h(px(30.0))
                    .my(px(2.0))
                    .px(px(10.0))
                    .rounded(px(6.0))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .child(empty_ask_txt)
                    .into_any_element()
            },
        ));

        let count_pill = |id_suffix: &'static str, n: usize| {
            div()
                .min_w(px(17.0))
                .h(px(17.0))
                .px(px(4.5))
                .rounded(px(4.0))
                .bg(if p.dark { p.logo_tile } else { card_bg })
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_rolling_number(
                    pro_eid(&self.id, id_suffix),
                    n as f64,
                    0,
                    "",
                    "",
                    10.5,
                    FontWeight::MEDIUM,
                    p.text,
                ))
        };

        let card = div()
            .w_full()
            .max_w(px(408.0))
            .rounded(px(12.0))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .px(px(16.0))
                    .pt(px(16.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(sparkle_header_icon(15.0, p.blue))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .line_height(px(19.0))
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(title_str),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .line_height(px(16.0))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(sub_str),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .px(px(16.0))
                    .pt(px(15.0))
                    .pb(px(17.0))
                    .child(seg_strip),
            )
            .child(
                div()
                    .w_full()
                    .px(px(10.0))
                    .pt(px(14.0))
                    .pb(px(12.0))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .px(px(6.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(div().text_size(px(12.0)).text_color(p.muted).child(tr(
                                        cx,
                                        "Runs on its own",
                                        "自主运行",
                                    )))
                                    .child(count_pill("cnt-auto", auto_count)),
                            )
                            .child(auto_list),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .px(px(6.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(div().text_size(px(12.0)).text_color(p.muted).child(tr(
                                        cx,
                                        "Asks you first",
                                        "先询问你",
                                    )))
                                    .child(count_pill("cnt-ask", ask_count)),
                            )
                            .child(ask_list),
                    ),
            );

        let width_sensor = self.measured_width.clone();
        let a11y_label = tr(
            cx,
            "Agent autonomy level and action locks",
            "智能体自主等级与动作锁定设置",
        );
        let focus_ring = p.focus;

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(a11y_label)
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .w_full()
            .py(px(24.0))
            .px(px(16.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.0)).border_color(focus_ring))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let idx = this.level.index();
                match ev.keystroke.key.as_str() {
                    "left" | "up" => {
                        let next = AutonomyLevel::ALL[idx.saturating_sub(1)];
                        this.set_level(next, cx);
                        cx.stop_propagation();
                    }
                    "right" | "down" => {
                        let next = AutonomyLevel::ALL[(idx + 1).min(2)];
                        this.set_level(next, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(408.0))
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            let w_px = f32::from(b.size.width);
                            if w_px > 20.0 {
                                width_sensor.set(w_px);
                            }
                        }
                    })
                    .child(card),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{AutonomyAction, AutonomyActionRisk, AutonomyDial, AutonomyLevel, asks_first};
    use gpui::{AppContext, TestAppContext};
    use std::collections::HashSet;

    #[test]
    fn asks_first_obeys_levels_pins_and_red_lines() {
        let safe = AutonomyAction::new("read", "Read docs", AutonomyActionRisk::Safe);
        let risky = AutonomyAction::new("send", "Send email", AutonomyActionRisk::Risky);
        let red_line = AutonomyAction::new("drop", "Drop table", AutonomyActionRisk::Destructive)
            .with_red_line(true);
        let empty = HashSet::new();

        assert!(asks_first(&safe, AutonomyLevel::AlwaysAsk, &empty));
        assert!(!asks_first(&safe, AutonomyLevel::IfRisky, &empty));
        assert!(asks_first(&risky, AutonomyLevel::IfRisky, &empty));
        assert!(!asks_first(&risky, AutonomyLevel::FullAuto, &empty));
        assert!(asks_first(&red_line, AutonomyLevel::FullAuto, &empty));
    }

    #[gpui::test]
    fn autonomy_dial_partitions_and_toggles_locks(cx: &mut TestAppContext) {
        let dial = cx.new(|cx| AutonomyDial::new("test-autonomy-dial", cx));

        dial.update(cx, |d, cx| {
            // Default is FullAuto with "spend" pinned -> (5 auto, 1 ask)
            assert_eq!(d.level(), AutonomyLevel::FullAuto);
            assert_eq!(d.partition_counts(), (5, 1));

            d.set_level(AutonomyLevel::IfRisky, cx);
            assert_eq!(d.partition_counts(), (2, 4));

            d.set_level(AutonomyLevel::AlwaysAsk, cx);
            assert_eq!(d.partition_counts(), (0, 6));

            d.set_level(AutonomyLevel::FullAuto, cx);
            d.toggle_pin("spend", cx);
            assert!(!d.is_pinned("spend"));
            assert_eq!(d.partition_counts(), (6, 0));
        });
    }
}
