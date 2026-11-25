//! Agent tool-call approval card with risk badge, pausing countdown ring, and resolved morph.
//!
//! Presents a human-in-the-loop consent card before an agent executes an external, paid, or
//! hard-to-undo action, complete with a leading chat prompt bubble, recipient avatars, structured
//! key-value preview rows, an "Always allow" checkbox, keyboard shortcuts (`⌘↵` / `Esc`), a live
//! countdown ring that pauses on hover or focus, and a spring fold into a compact resolved row.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_check_draw, pro_ease, pro_eid, pro_empty_state, pro_fold,
        pro_mix_color, pro_person_avatar, pro_skeleton_bar, pro_swap_label, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SPARK_BOT_SVG: &str = r#"<path d="M12 3.5 13.9 9l5.6 1.9-5.6 1.9L12 18.5l-1.9-5.7-5.6-1.9L10.1 9 12 3.5Z" fill="currentColor"></path>"#;
const MAIL_STROKE_SVG: &str = r#"<path d="M4 6.5h16v11H4z" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="m4.5 7 7.5 6 7.5-6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const RESOLVED_CROSS_SVG: &str = r#"<path d="M4.8 4.8 11.2 11.2M11.2 4.8 4.8 11.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"></path>"#;

/// Risk tier displayed in the top-right badge of [`ApprovalCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalRisk {
    /// Reversible or internal action (`Low risk`).
    Low,
    /// External communication or state mutation (`Medium risk`).
    #[default]
    Medium,
    /// Destructive, financial, or production-critical action (`High risk`).
    High,
}

impl ApprovalRisk {
    fn full_label(self, cx: &App) -> &'static str {
        match self {
            Self::Low => tr(cx, "Low risk", "低风险"),
            Self::Medium => tr(cx, "Medium risk", "中风险"),
            Self::High => tr(cx, "High risk", "高风险"),
        }
    }

    fn short_label(self, cx: &App) -> &'static str {
        match self {
            Self::Low => tr(cx, "Low", "低风险"),
            Self::Medium => tr(cx, "Medium", "中风险"),
            Self::High => tr(cx, "High", "高风险"),
        }
    }
}

/// Resolution state of [`ApprovalCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// Waiting for user approval or denial.
    #[default]
    Pending,
    /// Approved by the user.
    Approved,
    /// Denied by the user or expired when the countdown reached zero.
    Denied,
}

/// Reason an [`ApprovalCard`] transitioned to [`ApprovalStatus::Denied`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalDenyReason {
    /// Explicitly denied via the `Deny` button or `Escape`.
    User,
    /// Countdown reached zero before a decision was made.
    Timeout,
}

/// Where the action footer sits inside [`ApprovalCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalActionsPlacement {
    /// Recessed outer tray surrounding the white/elevated surface card.
    #[default]
    Tray,
    /// Footer embedded inside the card surface itself.
    Card,
}

/// Structured key-value row inside the preview section of [`ApprovalCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalDetail {
    /// Field label (for example `"Subject"` or `"Send"`).
    pub label: SharedString,
    /// Field value (for example `"What's new in October"`).
    pub value: SharedString,
}

impl ApprovalDetail {
    /// Construct a new [`ApprovalDetail`] row.
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Events emitted by [`ApprovalCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalCardEvent {
    /// User approved the pending action (`always` reflects the "Always allow" checkbox).
    Approved {
        /// Whether `"Always allow this action"` was checked.
        always: bool,
        /// Updated first detail value when edited inline prior to approval.
        edited_subject: Option<SharedString>,
    },
    /// Action was denied by the user or timed out.
    Denied {
        /// Whether the denial came from user input or timer expiration.
        reason: ApprovalDenyReason,
    },
    /// Countdown timer reached zero (emitted immediately before `Denied { reason: Timeout }`).
    Expired,
    /// User toggled the `"Always allow this action"` checkbox.
    AlwaysAllowChanged(bool),
    /// User opened or closed inline payload editing.
    EditToggled(bool),
    /// User reset the resolved row back to [`ApprovalStatus::Pending`].
    Reset,
}

impl EventEmitter<ApprovalCardEvent> for ApprovalCard {}

fn default_recipients() -> Vec<SharedString> {
    vec!["Mia".into(), "Jonas".into(), "Sara".into(), "Alex".into()]
}

fn default_details() -> Vec<ApprovalDetail> {
    vec![
        ApprovalDetail::new("Subject", "What's new in October"),
        ApprovalDetail::new("Send", "Today, 9:00 AM"),
    ]
}

fn format_mm_ss(total_secs: u32) -> String {
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}

#[derive(IntoElement)]
struct CountdownRing {
    remaining_ratio: f32,
    side: f32,
    stroke: f32,
    fg: Hsla,
    track: Hsla,
}

impl RenderOnce for CountdownRing {
    fn render(self, _w: &mut Window, _cx: &mut App) -> impl IntoElement {
        let ratio = self.remaining_ratio.clamp(0.0, 1.0);
        let stroke = self.stroke;
        let fg = self.fg;
        let track = self.track;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let center = bounds.center();
                let r = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) - stroke)
                    * 0.5;
                if r <= 1.0 {
                    return;
                }
                let steps = 48usize;
                let mut bg_ring = PathBuilder::stroke(px(stroke));
                for i in 0..=steps {
                    let angle = (i as f32 / steps as f32) * std::f32::consts::TAU
                        - std::f32::consts::FRAC_PI_2;
                    let pt = center + point(px(r * angle.cos()), px(r * angle.sin()));
                    if i == 0 {
                        bg_ring.move_to(pt);
                    } else {
                        bg_ring.line_to(pt);
                    }
                }
                if let Ok(path) = bg_ring.build() {
                    window.paint_path(path, track);
                }
                if ratio > 0.005 {
                    let arc_steps = ((steps as f32 * ratio).ceil() as usize).max(3);
                    let mut fg_arc = PathBuilder::stroke(px(stroke));
                    for i in 0..=arc_steps {
                        let frac = (i as f32 / arc_steps as f32) * ratio;
                        let angle = frac * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                        let pt = center + point(px(r * angle.cos()), px(r * angle.sin()));
                        if i == 0 {
                            fg_arc.move_to(pt);
                        } else {
                            fg_arc.line_to(pt);
                        }
                    }
                    if let Ok(path) = fg_arc.build() {
                        window.paint_path(path, fg);
                    }
                }
            },
        )
        .size(px(self.side))
        .flex_none()
    }
}

#[derive(IntoElement)]
struct ApcTactileBtn {
    id: ElementId,
    aria: SharedString,
    height: f32,
    pad_l: f32,
    pad_r: f32,
    radius: f32,
    rest_bg: Hsla,
    hover_bg: Hsla,
    rest_fg: Hsla,
    hover_fg: Hsla,
    border: Hsla,
    elevated: bool,
    flex_fill: bool,
    gap: f32,
    font_size: f32,
    content: AnyElement,
    #[allow(clippy::type_complexity)]
    on_click: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
}

impl RenderOnce for ApcTactileBtn {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let state_id = pro_eid(&self.id, "apc-tact");
        let st = w.use_keyed_state(state_id, cx, |_, _| {
            (Slide::snappy(0.0), Slide::snappy(0.0))
        });
        let (hov, down) = st.update(cx, |s, cx| {
            (
                s.0.get(w, cx).clamp(0.0, 1.0),
                s.1.get(w, cx).clamp(0.0, 1.0),
            )
        });
        let face_bg = pro_mix_color(self.rest_bg, self.hover_bg, hov);
        let fg = pro_mix_color(self.rest_fg, self.hover_fg, hov);
        let border = self.border;
        let radius = self.radius;
        let dark = p.dark;
        let elevated = self.elevated;

        let paint_face = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 1.0 || bh <= 1.0 {
                    return;
                }
                let ix = (bw * 0.022 * down).clamp(0.0, 2.0);
                let iy = (bh * 0.042 * down).clamp(0.0, 1.4);
                let rect = Bounds::new(
                    point(bounds.origin.x + px(ix), bounds.origin.y + px(iy)),
                    size(px((bw - 2.0 * ix).max(1.0)), px((bh - 2.0 * iy).max(1.0))),
                );
                let r = px(radius)
                    .min(rect.size.width / 2.0)
                    .min(rect.size.height / 2.0);
                if !dark && elevated && hov > 0.02 && down < 0.7 {
                    let sh = 0.045 * hov * (1.0 - down);
                    window.paint_quad(
                        fill(
                            Bounds::new(point(rect.origin.x, rect.origin.y + px(1.0)), rect.size),
                            black().opacity(sh),
                        )
                        .corner_radii(r),
                    );
                }
                if face_bg.a > 0.001 || border.a > 0.001 {
                    window.paint_quad(quad(
                        rect,
                        Corners::all(r),
                        face_bg,
                        Edges::all(px(if border.a > 0.001 { 1.0 } else { 0.0 })),
                        border,
                        BorderStyle::Solid,
                    ));
                }
            },
        )
        .absolute()
        .inset_0();

        let on_click = self.on_click;
        div()
            .id(self.id)
            .role(Role::Button)
            .aria_label(self.aria)
            .tab_index(0)
            .when(self.flex_fill, |d| d.flex_1())
            .h(px(self.height))
            .pl(px(self.pad_l))
            .pr(px(self.pad_r))
            .relative()
            .rounded(px(radius))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(self.font_size))
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .whitespace_nowrap()
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.0.set(if *inside { 1.0 } else { 0.0 }, cx);
                        if !*inside {
                            s.1.set(0.0, cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.1.set(1.0, cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.1.set(0.0, cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.1.set(0.0, cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_key_down({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | " " | "space") {
                        st.update(cx, |s, cx| {
                            s.1.set(1.0, cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_key_up({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | " " | "space") {
                        st.update(cx, |s, cx| {
                            s.1.set(0.0, cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
            .child(paint_face)
            .child(
                div()
                    .relative()
                    .top(px(0.5 * down))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(self.gap))
                    .child(self.content),
            )
    }
}

/// Human-in-the-loop agent action approval card with pausing countdown and resolved row morph.
pub struct ApprovalCard {
    id: ElementId,
    focus: FocusHandle,
    prompt: Option<SharedString>,
    tray_label: Option<SharedString>,
    title: SharedString,
    description: Option<SharedString>,
    risk: Option<ApprovalRisk>,
    icon: Option<ProIcon>,
    recipients: Vec<SharedString>,
    extra_recipients: u32,
    details: Vec<ApprovalDetail>,
    allow_always: bool,
    always_checked: bool,
    always_hover: Slide,
    shortcuts: bool,
    editable: bool,
    editing: bool,
    edited_subject: Option<SharedString>,
    subject_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    placement: ApprovalActionsPlacement,
    status: ApprovalStatus,
    deny_reason: Option<ApprovalDenyReason>,
    expires_in: Option<u32>,
    accumulated_secs: f32,
    last_tick: Option<Instant>,
    card_hovered: bool,
    resolved_clock: PhaseClock,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl ApprovalCard {
    /// Create an [`ApprovalCard`] preloaded with the October campaign newsletter approval scene.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            prompt: Some("I drafted the October newsletter. Ready to send?".into()),
            tray_label: None,
            title: "Send campaign email".into(),
            description: Some("October product update · Newsletter".into()),
            risk: Some(ApprovalRisk::Medium),
            icon: None,
            recipients: default_recipients(),
            extra_recipients: 38,
            details: default_details(),
            allow_always: true,
            always_checked: false,
            always_hover: Slide::snappy(0.0),
            shortcuts: true,
            editable: false,
            editing: false,
            edited_subject: None,
            subject_editor: None,
            editor_focused: false,
            placement: ApprovalActionsPlacement::Tray,
            status: ApprovalStatus::Pending,
            deny_reason: None,
            expires_in: Some(45),
            accumulated_secs: 0.0,
            last_tick: None,
            card_hovered: false,
            resolved_clock: PhaseClock::default(),
            measured_width: Rc::new(Cell::new(420.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Configure or omit (`None`) the leading chat bubble above the card.
    pub fn with_prompt(mut self, prompt: Option<SharedString>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Set a small step/tool caption above the card inside the tray (for example `"Step 4 · gmail.send"`).
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.tray_label = Some(label.into());
        self
    }

    /// Set the primary action title (also used as the accessible group label).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set or clear the single-line subtitle below the title.
    pub fn with_description(mut self, description: Option<SharedString>) -> Self {
        self.description = description;
        self
    }

    /// Set or hide (`None`) the risk badge in the top-right corner.
    pub fn with_risk(mut self, risk: Option<ApprovalRisk>) -> Self {
        self.risk = risk;
        self
    }

    /// Override the 32x32 header tile icon (`None` renders the default crisp mail stroke glyph).
    pub fn with_icon(mut self, icon: ProIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set overlapping recipient names and extra recipient count in the preview header.
    pub fn with_recipients<I, S>(mut self, names: I, extra_count: u32) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.recipients = names.into_iter().map(Into::into).collect();
        self.extra_recipients = extra_count;
        self
    }

    /// Replace the structured key-value rows in the preview panel.
    pub fn with_details(mut self, details: Vec<ApprovalDetail>) -> Self {
        self.details = details;
        self
    }

    /// Show or hide the `"Always allow this action"` checkbox.
    pub fn with_allow_always(mut self, allow_always: bool) -> Self {
        self.allow_always = allow_always;
        self
    }

    /// Set the countdown duration in seconds (`None` disables auto-expiration).
    pub fn with_expires_in(mut self, seconds: Option<u32>) -> Self {
        self.expires_in = seconds.map(|s| s.max(1));
        self.accumulated_secs = 0.0;
        self.last_tick = None;
        self
    }

    /// Show or hide keyboard shortcut hints (`⌘↵` and `Esc`) on the action buttons.
    pub fn with_shortcuts(mut self, shortcuts: bool) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Enable an inline `Edit` button before `Deny`.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Choose whether the action buttons sit on the outer tray or inside the card surface.
    pub fn with_actions_placement(mut self, placement: ApprovalActionsPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set the starting [`ApprovalStatus`].
    pub fn with_status(mut self, status: ApprovalStatus) -> Self {
        self.status = status;
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

    /// Current [`ApprovalStatus`] of the card.
    pub fn status(&self) -> ApprovalStatus {
        self.status
    }

    /// Whether `"Always allow this action"` is currently checked.
    pub fn is_always_allowed(&self) -> bool {
        self.always_checked
    }

    /// Remaining seconds before expiration when [`ApprovalStatus::Pending`].
    pub fn remaining_seconds(&self) -> Option<u32> {
        let total = self.expires_in? as f32;
        let rem = (total - self.accumulated_secs).max(0.0).ceil() as u32;
        Some(rem)
    }

    /// Approve the pending request programmatically or via user interaction.
    pub fn approve(&mut self, cx: &mut Context<Self>) {
        if self.status != ApprovalStatus::Pending {
            return;
        }
        if self.editing
            && let Some(ed) = &self.subject_editor
        {
            let v = ed.read(cx).value().trim().to_owned();
            if !v.is_empty() {
                self.edited_subject = Some(v.clone().into());
                if let Some(first) = self.details.first_mut() {
                    first.value = v.into();
                }
            }
        }
        self.editing = false;
        self.status = ApprovalStatus::Approved;
        self.deny_reason = None;
        self.last_tick = None;
        self.resolved_clock.restart(cx);
        let msg: SharedString = if self.always_checked {
            tr(cx, "Approved & always allowed", "已批准并设为始终允许").into()
        } else {
            tr(cx, "Approved campaign email", "已批准发送邮件").into()
        };
        self.toast = Some((msg, Instant::now()));
        cx.emit(ApprovalCardEvent::Approved {
            always: self.always_checked,
            edited_subject: self.edited_subject.clone(),
        });
        cx.notify();
    }

    /// Deny the pending request with the given [`ApprovalDenyReason`].
    pub fn deny(&mut self, reason: ApprovalDenyReason, cx: &mut Context<Self>) {
        if self.status != ApprovalStatus::Pending {
            return;
        }
        self.editing = false;
        self.status = ApprovalStatus::Denied;
        self.deny_reason = Some(reason);
        self.last_tick = None;
        self.resolved_clock.restart(cx);
        if reason == ApprovalDenyReason::Timeout {
            cx.emit(ApprovalCardEvent::Expired);
        }
        cx.emit(ApprovalCardEvent::Denied { reason });
        cx.notify();
    }

    /// Reset a resolved card back to a fresh [`ApprovalStatus::Pending`] state.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.status = ApprovalStatus::Pending;
        self.deny_reason = None;
        self.accumulated_secs = 0.0;
        self.last_tick = None;
        self.editing = false;
        self.toast = None;
        cx.emit(ApprovalCardEvent::Reset);
        cx.notify();
    }

    fn toggle_always(&mut self, cx: &mut Context<Self>) {
        if self.status != ApprovalStatus::Pending {
            return;
        }
        self.always_checked = !self.always_checked;
        cx.emit(ApprovalCardEvent::AlwaysAllowChanged(self.always_checked));
        cx.notify();
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.subject_editor.is_none() {
            let initial = self
                .details
                .first()
                .map(|d| d.value.to_string())
                .unwrap_or_default();
            let ph: SharedString = tr(cx, "Edit subject line…", "编辑邮件主题…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e.set_value(initial, w, cx);
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let v = ed_ent.read(cx).value().trim().to_owned();
                    if !v.is_empty() {
                        this.edited_subject = Some(v.clone().into());
                        if let Some(first) = this.details.first_mut() {
                            first.value = v.into();
                        }
                    }
                    this.editing = false;
                    this.focus.focus(w, cx);
                    cx.emit(ApprovalCardEvent::EditToggled(false));
                    cx.notify();
                }
                EditEvent::Escape => {
                    this.editing = false;
                    this.focus.focus(w, cx);
                    cx.emit(ApprovalCardEvent::EditToggled(false));
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.subject_editor = Some(ed);
        }
    }

    fn tick_countdown(
        &mut self,
        paused: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(u32, f32)> {
        let total = self.expires_in? as f32;
        if self.status != ApprovalStatus::Pending {
            self.last_tick = None;
            return None;
        }
        let now = Instant::now();
        if paused {
            self.last_tick = Some(now);
        } else {
            if let Some(prev) = self.last_tick {
                let dt = now.duration_since(prev).as_secs_f32().clamp(0.0, 0.25);
                self.accumulated_secs = (self.accumulated_secs + dt).min(total);
            }
            self.last_tick = Some(now);
            if self.accumulated_secs >= total {
                self.deny(ApprovalDenyReason::Timeout, cx);
                return Some((0, 0.0));
            }
            w.request_animation_frame();
        }
        let rem_secs = (total - self.accumulated_secs).max(0.0);
        let ratio = (rem_secs / total).clamp(0.0, 1.0);
        Some((rem_secs.ceil() as u32, ratio))
    }
}

impl Render for ApprovalCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        if self.editing {
            self.ensure_editor(w, cx);
        }

        if self.loading {
            let phase = (Instant::now().elapsed().as_secs_f32() * 0.5).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .py(px(28.0))
                .px(px(16.0))
                .child(
                    div()
                        .w_full()
                        .max_w(px(420.0))
                        .p(px(16.0))
                        .rounded(px(16.0))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(pro_skeleton_bar(200.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(280.0, 13.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(340.0, 48.0, 8.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Shield,
                    tr(cx, "No pending approvals", "暂无待审批动作"),
                    tr(
                        cx,
                        "Tool calls requiring confirmation will appear here before execution.",
                        "需要人工确认的工具调用将在执行前显示于此。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let card_focused = self.focus.is_focused(w) || self.editor_focused;
        let paused = self.card_hovered || card_focused;
        let countdown = self.tick_countdown(paused, w, cx);
        let always_hov = self.always_hover.get(w, cx).clamp(0.0, 1.0);
        let cw = self.measured_width.get();
        let compact_buttons = cw < 400.0;
        let ultra_narrow = cw < 340.0;

        // Tray & surface tokens aligned with reference sunken-tray + elevated-card hierarchy
        let tray_bg: Hsla = if p.dark {
            rgb(0x121215).into()
        } else {
            rgb(0xf0f0f2).into()
        };
        let card_bg: Hsla = if p.dark {
            rgb(0x19191c).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border = p.line;
        let icon_tile_bg: Hsla = if p.dark {
            rgb(0x202024).into()
        } else {
            rgb(0xfafafa).into()
        };
        let quiet_hover: Hsla = if p.dark {
            rgb(0x232327).into()
        } else {
            rgb(0xe4e4e7).into()
        };

        let is_pending = self.status == ApprovalStatus::Pending;

        // Build sleek compact leading chat prompt bubble if configured
        let prompt_el = self.prompt.as_ref().map(|prompt_txt| {
            let prompt_localized: SharedString =
                if prompt_txt.as_ref() == "I drafted the October newsletter. Ready to send?" {
                    tr(
                        cx,
                        "I drafted the October newsletter. Ready to send?",
                        "我已经起草好十月产品通讯，现在可以发送吗？",
                    )
                    .into()
                } else {
                    prompt_txt.clone()
                };
            let bot_bg: Hsla = if p.dark {
                rgb(0xededef).into()
            } else {
                rgb(0x171717).into()
            };
            let bot_fg: Hsla = if p.dark {
                rgb(0x18181b).into()
            } else {
                rgb(0xffffff).into()
            };
            let bubble_bg: Hsla = if p.dark {
                rgb(0x232327).into()
            } else {
                rgb(0xf1f1f3).into()
            };
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(
                    div()
                        .size(px(24.0))
                        .flex_none()
                        .rounded_full()
                        .bg(bot_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 24 24", SPARK_BOT_SVG, 12.0, bot_fg)),
                )
                .child(
                    div()
                        .px(px(13.0))
                        .py(px(6.5))
                        .rounded(px(16.0))
                        .bg(bubble_bg)
                        .border_1()
                        .border_color(p.line_soft)
                        .text_size(px(13.0))
                        .line_height(px(18.5))
                        .text_color(p.ink)
                        .child(prompt_localized),
                )
        });

        // Build Pending content for `pro_fold`
        let title_str: SharedString = if self.title.as_ref() == "Send campaign email" {
            tr(cx, "Send campaign email", "发送营销活动邮件").into()
        } else {
            self.title.clone()
        };
        let desc_str: Option<SharedString> = self.description.as_ref().map(|d| {
            if d.as_ref() == "October product update · Newsletter" {
                tr(
                    cx,
                    "October product update · Newsletter",
                    "十月产品更新 · 订阅通讯",
                )
                .into()
            } else {
                d.clone()
            }
        });

        let risk_pill = self.risk.map(|r| {
            let (r_bg, r_fg): (Hsla, Hsla) = match r {
                ApprovalRisk::Low => {
                    if p.dark {
                        (hsla(0.0, 0.0, 1.0, 0.07), rgb(0xa1a1a6).into())
                    } else {
                        (rgb(0xf2f2f3).into(), rgb(0x6b6b70).into())
                    }
                }
                ApprovalRisk::Medium => {
                    if p.dark {
                        (hsla(46.0 / 360.0, 0.75, 0.60, 0.14), rgb(0xe6c24d).into())
                    } else {
                        (rgb(0xfbf5e2).into(), rgb(0x8f6c00).into())
                    }
                }
                ApprovalRisk::High => {
                    if p.dark {
                        (hsla(4.0 / 360.0, 1.0, 0.69, 0.14), rgb(0xff8a7a).into())
                    } else {
                        (rgb(0xfdedeb).into(), rgb(0xc4320a).into())
                    }
                }
            };
            let lbl = if ultra_narrow {
                r.short_label(cx)
            } else {
                r.full_label(cx)
            };
            div()
                .h(px(22.0))
                .px(px(7.5))
                .mt(px(1.0))
                .flex_none()
                .rounded(px(6.0))
                .bg(r_bg)
                .text_color(r_fg)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .whitespace_nowrap()
                .child(lbl)
        });

        let header_icon = if let Some(ic) = self.icon {
            pro_icon(ic, 15.5, p.ink)
        } else {
            pro_svg("0 0 24 24", MAIL_STROKE_SVG, 15.5, p.ink)
        };

        // Recipients overlapping avatar strip
        let recipients_el = (!self.recipients.is_empty()).then(|| {
            let mut avatars = div().flex().items_center();
            for (idx, person) in self.recipients.iter().take(4).enumerate() {
                avatars = avatars.child(
                    div()
                        .when(idx > 0, |d| d.ml(px(-6.0)))
                        .rounded_full()
                        .p(px(1.5))
                        .bg(card_bg)
                        .child(pro_person_avatar(person.as_ref(), 22.0, false, None, cx)),
                );
            }
            let names_joined = self
                .recipients
                .iter()
                .take(4)
                .map(|s| s.as_ref())
                .collect::<Vec<_>>()
                .join(", ");
            let extra = self.extra_recipients;
            let more_txt = if extra > 0 {
                Some(SharedString::from(
                    if UiLanguage::current(cx) == UiLanguage::Chinese {
                        format!("+{extra} 人")
                    } else {
                        format!("+{extra} more")
                    },
                ))
            } else {
                None
            };
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .min_w_0()
                .child(avatars)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_baseline()
                        .gap(px(6.0))
                        .overflow_hidden()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(p.ink)
                                .truncate()
                                .child(names_joined),
                        )
                        .when_some(more_txt, |d, m| {
                            d.child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .child(m),
                            )
                        }),
                )
        });

        // Structured preview details
        let mut details_col = div().w_full().flex().flex_col().gap(px(5.0));
        for (idx, row) in self.details.iter().enumerate() {
            let lbl_text: SharedString = match row.label.as_ref() {
                "Subject" => tr(cx, "Subject", "主题").into(),
                "Send" => tr(cx, "Send", "发送时间").into(),
                "To" => tr(cx, "To", "收件人").into(),
                _ => row.label.clone(),
            };
            let val_text: SharedString = match row.value.as_ref() {
                "What's new in October" => {
                    tr(cx, "What's new in October", "十月功能更新速览").into()
                }
                "Today, 9:00 AM" => tr(cx, "Today, 9:00 AM", "今天 09:00").into(),
                _ => row.value.clone(),
            };
            let value_cell = if idx == 0 && self.editing {
                if let Some(ed) = &self.subject_editor {
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(26.0))
                        .px(px(8.0))
                        .py(px(3.0))
                        .rounded(px(6.0))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.focus)
                        .child(ed.clone())
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .text_color(p.ink)
                        .child(val_text)
                        .into_any_element()
                }
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(13.0))
                    .line_height(px(18.5))
                    .text_color(p.ink)
                    .truncate()
                    .child(val_text)
                    .into_any_element()
            };
            details_col = details_col.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(if ultra_narrow { 12.0 } else { 16.0 }))
                    .child(
                        div()
                            .w(px(if ultra_narrow { 52.0 } else { 60.0 }))
                            .flex_none()
                            .text_size(px(12.5))
                            .line_height(px(18.5))
                            .text_color(p.muted)
                            .truncate()
                            .child(lbl_text),
                    )
                    .child(value_cell),
            );
        }

        // Always allow checkbox row (`pro_check_draw` + `Slide::snappy` hover tint)
        let checkbox_el = self.allow_always.then(|| {
            let checked = self.always_checked;
            let always_lbl = tr(cx, "Always allow this action", "始终允许执行此操作");
            let label_ink = pro_mix_color(p.muted, p.ink, always_hov);
            div()
                .id(pro_eid(&self.id, "always-chk"))
                .role(Role::CheckBox)
                .aria_label(always_lbl)
                .aria_toggled(checked.into())
                .tab_index(0)
                .flex()
                .items_center()
                .gap(px(7.0))
                .cursor_pointer()
                .focus_visible(move |s| s.rounded(px(5.0)).border_1().border_color(p.focus))
                .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                    this.always_hover.set(if *inside { 1.0 } else { 0.0 }, cx);
                    cx.notify();
                }))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.toggle_always(cx);
                }))
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                    if matches!(ev.keystroke.key.as_str(), " " | "space" | "enter") {
                        this.toggle_always(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(pro_check_draw(
                    pro_eid(&self.id, "always-chk-draw"),
                    checked,
                    15.0,
                    true,
                    p.primary,
                ))
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(17.5))
                        .text_color(label_ink)
                        .child(always_lbl),
                )
        });

        // Countdown label (shown when `allow_always` is off, or as compact pause cue)
        let timer_label = countdown.map(|(rem, _)| {
            let is_low = rem <= 10;
            let timer_fg = if is_low { p.amber_ink } else { p.muted };
            let text_str = if paused {
                if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("已暂停 · {}", format_mm_ss(rem))
                } else {
                    format!("Paused · {}", format_mm_ss(rem))
                }
            } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("{} 后过期", format_mm_ss(rem))
            } else {
                format!("Expires in {}", format_mm_ss(rem))
            };
            div()
                .flex()
                .items_center()
                .gap(px(4.5))
                .text_size(px(12.0))
                .text_color(timer_fg)
                .whitespace_nowrap()
                .child(pro_icon(
                    if paused {
                        ProIcon::Pause
                    } else {
                        ProIcon::Clock
                    },
                    12.5,
                    timer_fg,
                ))
                .child(text_str)
        });

        let show_kbd = self.shortcuts && !compact_buttons;
        let kbd_quiet_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.08)
        } else {
            hsla(0.0, 0.0, 0.0, 0.055)
        };
        let kbd_primary_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 0.0, 0.12)
        } else {
            hsla(0.0, 0.0, 1.0, 0.16)
        };
        let kbd_primary_fg: Hsla = p.on_primary.opacity(0.76);

        let edit_btn = self.editable.then(|| {
            let edit_lbl = if self.editing {
                tr(cx, "Done", "完成")
            } else {
                tr(cx, "Edit", "编辑")
            };
            ApcTactileBtn {
                id: pro_eid(&self.id, "edit-btn"),
                aria: edit_lbl.into(),
                height: 32.0,
                pad_l: 10.0,
                pad_r: 10.0,
                radius: 9.0,
                rest_bg: transparent_black(),
                hover_bg: quiet_hover,
                rest_fg: p.text,
                hover_fg: p.ink,
                border: transparent_black(),
                elevated: false,
                flex_fill: false,
                gap: 5.0,
                font_size: 12.5,
                content: pro_swap_label(
                    pro_eid(&self.id, "edit-swap-lbl"),
                    edit_lbl,
                    12.5,
                    FontWeight::MEDIUM,
                    p.ink,
                ),
                on_click: Rc::new(cx.listener(|this, _: &ClickEvent, w, cx| {
                    this.editing = !this.editing;
                    if this.editing {
                        this.ensure_editor(w, cx);
                        if let Some(ed) = &this.subject_editor {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                    }
                    cx.emit(ApprovalCardEvent::EditToggled(this.editing));
                    cx.notify();
                })),
            }
        });

        let deny_lbl = tr(cx, "Deny", "拒绝");
        let deny_inner = div()
            .flex()
            .items_center()
            .gap(px(6.5))
            .child(deny_lbl)
            .when(show_kbd, |d| {
                d.child(
                    div()
                        .h(px(19.0))
                        .px(px(5.0))
                        .rounded(px(5.0))
                        .bg(kbd_quiet_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child("Esc"),
                )
            });
        let deny_btn = ApcTactileBtn {
            id: pro_eid(&self.id, "deny-btn"),
            aria: deny_lbl.into(),
            height: 32.0,
            pad_l: 11.0,
            pad_r: if show_kbd { 7.5 } else { 11.0 },
            radius: 9.0,
            rest_bg: transparent_black(),
            hover_bg: quiet_hover,
            rest_fg: p.text,
            hover_fg: p.strong,
            border: transparent_black(),
            elevated: false,
            flex_fill: compact_buttons,
            gap: 6.5,
            font_size: 12.5,
            content: deny_inner.into_any_element(),
            on_click: Rc::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.deny(ApprovalDenyReason::User, cx);
            })),
        };

        let approve_lbl = tr(cx, "Approve", "批准");
        let approve_inner = div()
            .flex()
            .items_center()
            .gap(px(7.0))
            .child(approve_lbl)
            .map(|d| {
                if let Some((rem, ratio)) = countdown {
                    let ring_fg = if rem <= 10 {
                        p.amber
                    } else {
                        p.on_primary.opacity(0.90)
                    };
                    let ring_track = p.on_primary.opacity(0.22);
                    d.child(CountdownRing {
                        remaining_ratio: ratio,
                        side: 13.5,
                        stroke: 1.75,
                        fg: ring_fg,
                        track: ring_track,
                    })
                } else if show_kbd {
                    d.child(
                        div()
                            .h(px(19.0))
                            .px(px(5.0))
                            .rounded(px(5.0))
                            .bg(kbd_primary_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(kbd_primary_fg)
                            .child("⌘↵"),
                    )
                } else {
                    d
                }
            });
        let approve_btn = ApcTactileBtn {
            id: pro_eid(&self.id, "approve-btn"),
            aria: approve_lbl.into(),
            height: 32.0,
            pad_l: 13.0,
            pad_r: if countdown.is_some() || show_kbd {
                10.5
            } else {
                13.0
            },
            radius: 9.0,
            rest_bg: p.primary,
            hover_bg: pro_mix_color(p.primary, p.on_primary, 0.12),
            rest_fg: p.on_primary,
            hover_fg: p.on_primary,
            border: transparent_black(),
            elevated: true,
            flex_fill: compact_buttons,
            gap: 7.0,
            font_size: 12.5,
            content: approve_inner.into_any_element(),
            on_click: Rc::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.approve(cx);
            })),
        };

        let actions_row = div()
            .when(compact_buttons, |d| d.w_full())
            .flex()
            .items_center()
            .justify_end()
            .gap(px(4.0))
            .when_some(edit_btn, |d, b| d.child(b))
            .child(deny_btn)
            .child(approve_btn);

        let has_preview = !self.recipients.is_empty() || !self.details.is_empty();
        let mut checkbox_slot = checkbox_el;
        let mut timer_slot = timer_label;
        let mut actions_slot = Some(actions_row);

        let inner_card = {
            let mut c = div()
                .w_full()
                .p(px(if ultra_narrow { 14.0 } else { 15.0 }))
                .rounded(px(14.0))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .shadow(pro_card_shadow(p.dark))
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .gap(px(if ultra_narrow { 10.0 } else { 11.0 }))
                        .child(
                            div()
                                .size(px(32.0))
                                .flex_none()
                                .rounded(px(8.0))
                                .bg(icon_tile_bg)
                                .border_1()
                                .border_color(card_border)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(header_icon),
                        )
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
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .line_height(px(19.0))
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(title_str.clone()),
                                )
                                .when_some(desc_str, |d, desc| {
                                    d.child(
                                        div()
                                            .text_size(px(12.5))
                                            .line_height(px(17.5))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(desc),
                                    )
                                }),
                        )
                        .when_some(risk_pill, |d, pill| d.child(pill)),
                );

            if has_preview {
                c = c.child(
                    div()
                        .w_full()
                        .mt(px(13.0))
                        .pt(px(13.0))
                        .border_t_1()
                        .border_color(card_border)
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .when_some(recipients_el, |d, r| d.child(r))
                        .when(!self.details.is_empty(), |d| d.child(details_col)),
                );
            }

            if self.placement == ApprovalActionsPlacement::Card {
                c = c.child(
                    div()
                        .w_full()
                        .mt(px(14.0))
                        .pt(px(12.0))
                        .border_t_1()
                        .border_color(card_border)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(div().flex().items_center().gap(px(8.0)).map(|left| {
                            if let Some(chk) = checkbox_slot.take() {
                                left.child(chk)
                            } else if let Some(t) = timer_slot.take() {
                                left.child(t)
                            } else {
                                left
                            }
                        }))
                        .when_some(actions_slot.take(), |d, a| d.child(a)),
                );
            }
            c
        };

        let tray_footer = (self.placement == ApprovalActionsPlacement::Tray).then(|| {
            div()
                .w_full()
                .min_h(px(42.0))
                .pl(px(if compact_buttons { 6.0 } else { 10.0 }))
                .pr(px(2.0))
                .pt(px(6.0))
                .pb(px(2.0))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(div().flex().items_center().gap(px(8.0)).map(|left| {
                    if let Some(chk) = checkbox_slot.take() {
                        left.child(chk)
                    } else if let Some(t) = timer_slot.take() {
                        left.child(t)
                    } else {
                        left
                    }
                }))
                .when_some(actions_slot.take(), |d, a| d.child(a))
        });

        let pending_tree = div()
            .w_full()
            .flex()
            .flex_col()
            .when_some(self.tray_label.clone(), |d, lbl| {
                d.child(
                    div()
                        .px(px(8.0))
                        .pt(px(5.0))
                        .pb(px(4.0))
                        .text_size(px(12.0))
                        .text_color(p.muted)
                        .truncate()
                        .child(lbl),
                )
            })
            .child(inner_card)
            .when_some(tray_footer, |d, foot| d.child(foot));

        // Resolved compact row morph
        let pop_age = self.resolved_clock.time(0.32, w, cx);
        let pop_t = if disabled_motion(cx) {
            1.0
        } else {
            pro_ease((pop_age / 0.32).clamp(0.0, 1.0))
        };
        let is_approved = self.status == ApprovalStatus::Approved;
        let is_timeout = self.deny_reason == Some(ApprovalDenyReason::Timeout);
        let resolved_chk_on =
            is_approved && (disabled_motion(cx) || self.resolved_clock.age(cx) > 0.012);
        let (mark_bg, mark_fg): (Hsla, Hsla) = if is_approved {
            (p.green_soft, p.green_ink)
        } else if is_timeout {
            (p.amber_soft, p.amber_ink)
        } else {
            (p.chip, p.muted)
        };
        let status_word: SharedString = if is_approved {
            if self.always_checked {
                tr(cx, "Approved · Always", "已批准 · 始终允许").into()
            } else {
                tr(cx, "Approved", "已批准").into()
            }
        } else if is_timeout {
            tr(cx, "Expired", "已超时").into()
        } else {
            tr(cx, "Denied", "已拒绝").into()
        };
        let undo_lbl = tr(cx, "Undo", "撤销");
        let undo_inner = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(pro_icon(ProIcon::Undo, 12.0, p.muted))
            .child(undo_lbl);
        let resolved_row = div()
            .w_full()
            .min_h(px(48.0))
            .px(px(14.0))
            .py(px(10.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(if is_approved {
                pro_check_draw(
                    pro_eid(&self.id, "resolved-chk"),
                    resolved_chk_on,
                    22.0,
                    false,
                    p.green,
                )
            } else {
                div()
                    .size(px(22.0))
                    .flex_none()
                    .rounded_full()
                    .bg(mark_bg)
                    .opacity(0.35 + 0.65 * pop_t)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if is_timeout {
                        pro_icon(ProIcon::Clock, 13.0, mark_fg)
                    } else {
                        pro_svg("0 0 16 16", RESOLVED_CROSS_SVG, 12.0, mark_fg)
                    })
                    .into_any_element()
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .truncate()
                    .child(title_str),
            )
            .child(div().flex_none().child(pro_swap_label(
                pro_eid(&self.id, "res-status-lbl"),
                status_word,
                12.5,
                FontWeight::NORMAL,
                p.muted,
            )))
            .child(ApcTactileBtn {
                id: pro_eid(&self.id, "reset-btn"),
                aria: undo_lbl.into(),
                height: 26.0,
                pad_l: 8.0,
                pad_r: 8.0,
                radius: 6.0,
                rest_bg: transparent_black(),
                hover_bg: quiet_hover,
                rest_fg: p.text,
                hover_fg: p.ink,
                border: transparent_black(),
                elevated: false,
                flex_fill: false,
                gap: 4.0,
                font_size: 12.0,
                content: undo_inner.into_any_element(),
                on_click: Rc::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.reset(cx);
                })),
            });

        let outer_shell = if self.placement == ApprovalActionsPlacement::Tray {
            div()
                .w_full()
                .p(px(4.0))
                .rounded(px(18.0))
                .bg(tray_bg)
                .border_1()
                .border_color(p.line_soft)
                .flex()
                .flex_col()
                .child(pro_fold(
                    pro_eid(&self.id, "pending-fold"),
                    is_pending,
                    228.0,
                    0.0,
                    move || pending_tree.into_any_element(),
                ))
                .child(pro_fold(
                    pro_eid(&self.id, "resolved-fold"),
                    !is_pending,
                    48.0,
                    0.0,
                    move || resolved_row.into_any_element(),
                ))
        } else {
            div()
                .w_full()
                .rounded(px(14.0))
                .flex()
                .flex_col()
                .child(pro_fold(
                    pro_eid(&self.id, "pending-fold-c"),
                    is_pending,
                    228.0,
                    0.0,
                    move || pending_tree.into_any_element(),
                ))
                .child(pro_fold(
                    pro_eid(&self.id, "resolved-fold-c"),
                    !is_pending,
                    48.0,
                    0.0,
                    move || {
                        div()
                            .w_full()
                            .rounded(px(14.0))
                            .bg(card_bg)
                            .border_1()
                            .border_color(card_border)
                            .shadow(pro_card_shadow(p.dark))
                            .child(resolved_row)
                            .into_any_element()
                    },
                ))
        };

        let width_sensor = self.measured_width.clone();
        let a11y_label: SharedString = self.title.clone();
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
            .focus_visible(move |s| s.rounded(px(16.0)).border_color(focus_ring))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                this.card_hovered = *inside;
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if this.editing || this.status != ApprovalStatus::Pending {
                    return;
                }
                let key = ev.keystroke.key.as_str();
                let m = &ev.keystroke.modifiers;
                if key == "enter" && (m.platform || m.control || !this.editor_focused) {
                    this.approve(cx);
                    cx.stop_propagation();
                } else if key == "escape" {
                    this.deny(ApprovalDenyReason::User, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(420.0))
                    .flex()
                    .flex_col()
                    .gap(px(11.0))
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.last() {
                            let w_px = f32::from(b.size.width);
                            if w_px > 20.0 {
                                width_sensor.set(w_px);
                            }
                        }
                    })
                    .when_some(prompt_el, |d, p_el| d.child(p_el))
                    .child(outer_shell),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalActionsPlacement, ApprovalCard, ApprovalDenyReason, ApprovalDetail, ApprovalRisk,
        ApprovalStatus, format_mm_ss,
    };
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn format_mm_ss_renders_tabular_clock() {
        assert_eq!(format_mm_ss(0), "0:00");
        assert_eq!(format_mm_ss(9), "0:09");
        assert_eq!(format_mm_ss(45), "0:45");
        assert_eq!(format_mm_ss(125), "2:05");
    }

    #[gpui::test]
    fn approval_card_approve_deny_and_reset_transitions(cx: &mut TestAppContext) {
        let card = cx.new(|cx| {
            ApprovalCard::new("test-approval-card", cx)
                .with_risk(Some(ApprovalRisk::High))
                .with_details(vec![ApprovalDetail::new("Target", "Customers (42)")])
                .with_actions_placement(ApprovalActionsPlacement::Tray)
                .with_expires_in(Some(30))
        });

        card.update(cx, |c, cx| {
            assert_eq!(c.status(), ApprovalStatus::Pending);
            assert_eq!(c.remaining_seconds(), Some(30));
            c.toggle_always(cx);
            assert!(c.is_always_allowed());
            c.approve(cx);
            assert_eq!(c.status(), ApprovalStatus::Approved);
        });

        card.update(cx, |c, cx| {
            c.reset(cx);
            assert_eq!(c.status(), ApprovalStatus::Pending);
            c.deny(ApprovalDenyReason::User, cx);
            assert_eq!(c.status(), ApprovalStatus::Denied);
        });
    }
}
