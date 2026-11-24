//! Granular agent scope request card with timed grants, draining status bar, and one-click revoke.
//!
//! Asks for one specific capability at the moment an agent needs it—showing who asks
//! (`pro_agent_avatar`), target application (`pro_app_tile`), why access is needed, and concrete
//! impact rows—letting the user grant `Allow once`, `Allow for 1 hour`, `Always allow`, or `Deny`,
//! then folding smoothly into a single status line with a live draining timer and `Revoke`.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_mark_selected, pro_mix_color, pro_pill_indicator,
        pro_selection_sensor, pro_skeleton_bar, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const CIRCLE_CROSS_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.5"></circle><path d="m8.8 8.8 6.4 6.4M15.2 8.8l-6.4 6.4" stroke="currentColor" stroke-linecap="round" stroke-width="1.5"></path>"#;
const WARN_MINI_SVG: &str = r#"<path d="M13.92 21H10.08C5.44 21 3.13 21 2.28 19.49C1.42 17.99 2.61 15.99 4.98 12L6.9 8.75C9.18 4.92 10.31 3 12 3C13.69 3 14.82 4.92 17.1 8.75L19.02 12C21.39 15.99 22.58 17.99 21.72 19.49C20.87 21 18.56 21 13.92 21Z" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round"></path><path d="M12 9V13" stroke="currentColor" stroke-width="1.65" stroke-linecap="round"></path><circle cx="12" cy="16.7" r="1" fill="currentColor"></circle>"#;
const DANGER_MINI_SVG: &str = r#"<path d="M5.92 5.92C8.16 3.68 9.28 2.56 10.59 2.2C11.51 1.93 12.49 1.93 13.41 2.2C14.72 2.56 15.84 3.68 18.08 5.92C20.32 8.16 21.44 9.28 21.8 10.59C22.07 11.51 22.07 12.49 21.8 13.41C21.44 14.72 20.32 15.84 18.08 18.08C15.84 20.32 14.72 21.44 13.41 21.8C12.49 22.07 11.51 22.07 10.59 21.8C9.28 21.44 8.16 20.32 5.92 18.08C3.68 15.84 2.56 14.72 2.2 13.41C1.93 12.49 1.93 11.51 2.2 10.59C2.56 9.28 3.68 8.16 5.92 5.92Z" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round"></path><path d="M12 8V12.2" stroke="currentColor" stroke-width="1.65" stroke-linecap="round"></path><circle cx="12" cy="15.8" r="1" fill="currentColor"></circle>"#;

/// Risk classification of a [`PermissionRequest`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionRisk {
    /// Low-impact scope (no warning tag shown).
    Safe,
    /// Sensitive external or branch mutation (`Risky` amber tag).
    #[default]
    Risky,
    /// Irreversible or high-blast-radius operation (`Destructive` red tag; omits `Always` scope and turns `Allow` red).
    Destructive,
}

/// Duration scope offered when granting a [`PermissionRequest`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionScope {
    /// Authorize only this single pending invocation.
    Once,
    /// Authorize for a bounded duration (defaults to `60` minutes).
    #[default]
    Timed,
    /// Authorize permanently until explicitly revoked (disallowed for [`PermissionRisk::Destructive`]).
    Always,
}

impl PermissionScope {
    fn key(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Timed => "timed",
            Self::Always => "always",
        }
    }

    fn pill_label(self, duration_mins: u32, cx: &App) -> SharedString {
        match self {
            Self::Once => tr(cx, "Once", "仅此次").into(),
            Self::Timed => {
                if duration_mins == 60 {
                    tr(cx, "1 hour", "1 小时").into()
                } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("{duration_mins} 分钟").into()
                } else {
                    format!("{duration_mins} min").into()
                }
            }
            Self::Always => tr(cx, "Always", "始终允许").into(),
        }
    }

    fn button_label(self, duration_mins: u32, cx: &App) -> SharedString {
        match self {
            Self::Once => tr(cx, "Allow once", "允许一次").into(),
            Self::Timed => {
                if duration_mins == 60 {
                    tr(cx, "Allow for 1 hour", "允许 1 小时").into()
                } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("允许 {duration_mins} 分钟").into()
                } else {
                    format!("Allow for {duration_mins} min").into()
                }
            }
            Self::Always => tr(cx, "Always allow", "始终允许").into(),
        }
    }
}

/// Concrete impact preview row inside [`PermissionRequest`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionPreviewItem {
    /// Stable row identifier.
    pub id: SharedString,
    /// Affected file, recipient, or system description.
    pub label: SharedString,
    /// Optional right-aligned tabular metadata (for example `"+12 −4"`).
    pub meta: Option<SharedString>,
    /// Leading row icon.
    pub icon: ProIcon,
}

impl PermissionPreviewItem {
    /// Construct a [`PermissionPreviewItem`].
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            meta: None,
            icon: ProIcon::Code,
        }
    }

    /// Set the right-aligned metadata badge (such as `"+12 −4"` or `"$140.00"`).
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Set the leading [`ProIcon`].
    pub fn with_icon(mut self, icon: ProIcon) -> Self {
        self.icon = icon;
        self
    }
}

/// Recorded grant or denial state of [`PermissionRequest`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PermissionDecision {
    /// `true` when granted, `false` when denied or expired.
    pub allowed: bool,
    /// Selected [`PermissionScope`] at decision time.
    pub scope: PermissionScope,
    /// Whether a timed grant expired naturally.
    pub expired: bool,
}

/// Events emitted by [`PermissionRequest`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionRequestEvent {
    /// User switched the selected [`PermissionScope`] (`Once`, `Timed`, `Always`).
    ScopeChanged(PermissionScope),
    /// User granted, denied, or revoked (`None`) the permission.
    DecisionChanged(Option<PermissionDecision>),
    /// User expanded or collapsed overflow preview rows.
    PreviewExpanded(bool),
}

impl EventEmitter<PermissionRequestEvent> for PermissionRequest {}

fn default_preview_items() -> Vec<PermissionPreviewItem> {
    vec![
        PermissionPreviewItem::new("cart", "src/checkout/cart.ts")
            .with_meta("+12 −4")
            .with_icon(ProIcon::Code),
        PermissionPreviewItem::new("spec", "tests/checkout.spec.ts")
            .with_meta("+28 −2")
            .with_icon(ProIcon::Code),
        PermissionPreviewItem::new("deploy", "Deploys to production after the merge")
            .with_meta("us-east-1")
            .with_icon(ProIcon::Bolt),
    ]
}

fn format_remaining_grant(rem_secs: u32, zh: bool) -> String {
    let mins = rem_secs / 60;
    let secs = rem_secs % 60;
    if mins > 0 {
        if zh {
            format!("剩余 {mins} 分 {secs:02} 秒")
        } else {
            format!("{mins} min {secs:02}s left")
        }
    } else if zh {
        format!("剩余 {secs} 秒")
    } else {
        format!("{secs}s left")
    }
}

#[derive(IntoElement)]
struct PrTactileBtn {
    id: ElementId,
    aria: SharedString,
    flex_fill: bool,
    pad_l: f32,
    pad_r: f32,
    rest_bg: Hsla,
    hover_bg: Hsla,
    fg: Hsla,
    border: Hsla,
    elevated: bool,
    content: AnyElement,
    #[allow(clippy::type_complexity)]
    on_click: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
}

impl RenderOnce for PrTactileBtn {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let st = w.use_keyed_state(pro_eid(&self.id, "pr-tbtn"), cx, |_, _| {
            (Slide::snappy(0.0), Slide::snappy(0.0))
        });
        let (hov, down) = st.update(cx, |s, cx| {
            (
                s.0.get(w, cx).clamp(0.0, 1.0),
                s.1.get(w, cx).clamp(0.0, 1.0),
            )
        });
        let face_bg = pro_mix_color(self.rest_bg, self.hover_bg, hov);
        let border = self.border;
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
                let iy = (bh * 0.042 * down).clamp(0.0, 1.3);
                let rect = Bounds::new(
                    point(bounds.origin.x + px(ix), bounds.origin.y + px(iy)),
                    size(px((bw - 2.0 * ix).max(1.0)), px((bh - 2.0 * iy).max(1.0))),
                );
                let r = px(7.0)
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
                window.paint_quad(quad(
                    rect,
                    Corners::all(r),
                    face_bg,
                    Edges::all(px(if border.a > 0.001 { 1.0 } else { 0.0 })),
                    border,
                    BorderStyle::Solid,
                ));
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
            .h(px(30.0))
            .pl(px(self.pad_l))
            .pr(px(self.pad_r))
            .relative()
            .rounded(px(7.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(self.fg)
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
                    .gap(px(7.0))
                    .child(self.content),
            )
    }
}

/// Granular scope consent card that folds into a live draining status line with `Revoke`.
pub struct PermissionRequest {
    id: ElementId,
    focus: FocusHandle,
    agent_name: SharedString,
    agent_model: Option<SharedString>,
    action: SharedString,
    target: SharedString,
    tool_app: SharedString,
    risk: PermissionRisk,
    reason: SharedString,
    preview_title: SharedString,
    preview: Vec<PermissionPreviewItem>,
    preview_limit: usize,
    preview_expanded: bool,
    expanded_items: HashSet<SharedString>,
    scopes: Vec<PermissionScope>,
    scope: PermissionScope,
    duration_mins: u32,
    decision: Option<PermissionDecision>,
    granted_at: Option<Instant>,
    settled_clock: PhaseClock,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl PermissionRequest {
    /// Create a [`PermissionRequest`] preloaded with `Coder wants to push to main` on `acme/web` (`GitHub`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            agent_name: "Coder".into(),
            agent_model: Some("GPT-6 Sol".into()),
            action: "push to main".into(),
            target: "acme/web".into(),
            tool_app: "github".into(),
            risk: PermissionRisk::Risky,
            reason: "The fix for the flaky checkout test passed review and all checks.".into(),
            preview_title: "What will happen".into(),
            preview: default_preview_items(),
            preview_limit: 3,
            preview_expanded: false,
            expanded_items: HashSet::new(),
            scopes: vec![
                PermissionScope::Once,
                PermissionScope::Timed,
                PermissionScope::Always,
            ],
            scope: PermissionScope::Timed,
            duration_mins: 60,
            decision: None,
            granted_at: None,
            settled_clock: PhaseClock::default(),
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(400.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Configure the asking agent name and optional frontier model label.
    pub fn with_agent(
        mut self,
        name: impl Into<SharedString>,
        model: Option<SharedString>,
    ) -> Self {
        self.agent_name = name.into();
        self.agent_model = model;
        self
    }

    /// Set the requested capability verb phrase (read after `"wants to"`, e.g. `"push to main"`).
    pub fn with_action(mut self, action: impl Into<SharedString>) -> Self {
        self.action = action.into();
        self
    }

    /// Set the target resource and app tile key (for example `"acme/web"` and `"github"`).
    pub fn with_target(
        mut self,
        target: impl Into<SharedString>,
        tool_app: impl Into<SharedString>,
    ) -> Self {
        self.target = target.into();
        self.tool_app = tool_app.into();
        self
    }

    /// Set the [`PermissionRisk`] tier (`Destructive` automatically removes `Always` and styles `Allow` red).
    pub fn with_risk(mut self, risk: PermissionRisk) -> Self {
        self.risk = risk;
        if risk == PermissionRisk::Destructive && self.scope == PermissionScope::Always {
            self.scope = PermissionScope::Once;
        }
        self
    }

    /// Set the agent's explanation of why access is required.
    pub fn with_reason(mut self, reason: impl Into<SharedString>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Replace the concrete [`PermissionPreviewItem`] impact list.
    pub fn with_preview(mut self, preview: Vec<PermissionPreviewItem>) -> Self {
        self.preview = preview;
        self
    }

    /// Set the starting [`PermissionScope`] (`Once`, `Timed`, `Always`).
    pub fn with_scope(mut self, scope: PermissionScope) -> Self {
        self.scope = if self.risk == PermissionRisk::Destructive && scope == PermissionScope::Always
        {
            PermissionScope::Once
        } else {
            scope
        };
        self
    }

    /// Set the timed grant duration in minutes (defaults to `60`).
    pub fn with_duration_mins(mut self, minutes: u32) -> Self {
        self.duration_mins = minutes.max(1);
        self
    }

    /// Set an initial [`PermissionDecision`] (`Some(...)` starts folded in the status line).
    pub fn with_decision(mut self, decision: Option<PermissionDecision>) -> Self {
        self.decision = decision;
        self.granted_at = decision.and_then(|d| d.allowed.then(Instant::now));
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

    /// Active [`PermissionScope`] selection.
    pub fn scope(&self) -> PermissionScope {
        self.scope
    }

    /// Current [`PermissionDecision`], or `None` while awaiting an answer.
    pub fn decision(&self) -> Option<PermissionDecision> {
        self.decision
    }

    fn available_scopes(&self) -> Vec<PermissionScope> {
        self.scopes
            .iter()
            .copied()
            .filter(|s| {
                !(self.risk == PermissionRisk::Destructive && *s == PermissionScope::Always)
            })
            .collect()
    }

    /// Select a [`PermissionScope`] (`Always` is ignored when [`PermissionRisk::Destructive`]).
    pub fn set_scope(&mut self, scope: PermissionScope, cx: &mut Context<Self>) {
        if self.risk == PermissionRisk::Destructive && scope == PermissionScope::Always {
            return;
        }
        if self.scope != scope {
            self.scope = scope;
            cx.emit(PermissionRequestEvent::ScopeChanged(scope));
            cx.notify();
        }
    }

    /// Grant the requested scope and fold into the status line.
    pub fn allow(&mut self, cx: &mut Context<Self>) {
        let effective_scope =
            if self.risk == PermissionRisk::Destructive && self.scope == PermissionScope::Always {
                PermissionScope::Once
            } else {
                self.scope
            };
        let dec = PermissionDecision {
            allowed: true,
            scope: effective_scope,
            expired: false,
        };
        self.decision = Some(dec);
        self.granted_at = Some(Instant::now());
        self.settled_clock.restart(cx);
        self.toast = Some((
            effective_scope.button_label(self.duration_mins, cx),
            Instant::now(),
        ));
        cx.emit(PermissionRequestEvent::DecisionChanged(Some(dec)));
        cx.notify();
    }

    /// Deny the permission request and fold into the denied status line.
    pub fn deny(&mut self, cx: &mut Context<Self>) {
        let dec = PermissionDecision {
            allowed: false,
            scope: self.scope,
            expired: false,
        };
        self.decision = Some(dec);
        self.granted_at = None;
        cx.emit(PermissionRequestEvent::DecisionChanged(Some(dec)));
        cx.notify();
    }

    /// Revoke an active grant (or clear a denial) and unfold back to the request card.
    pub fn revoke(&mut self, cx: &mut Context<Self>) {
        if self.decision.take().is_some() {
            self.granted_at = None;
            cx.emit(PermissionRequestEvent::DecisionChanged(None));
            cx.notify();
        }
    }
}

impl Render for PermissionRequest {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);

        if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() * 0.5).fract();
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
                        .rounded(px(10.0))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(pro_skeleton_bar(220.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(340.0, 32.0, 6.0, phase, cx))
                        .child(pro_skeleton_bar(360.0, 72.0, 8.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::KeyLock,
                    tr(cx, "No pending permission requests", "暂无待授权权限请求"),
                    tr(
                        cx,
                        "Granular scope requests appear here when an agent encounters a restricted capability.",
                        "当智能体尝试调用尚未获批的细粒度权限时，授权卡片将显示于此。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        // Tick timed grant countdown if active
        let total_grant_secs = self.duration_mins.saturating_mul(60);
        let mut timed_remaining: Option<(u32, f32)> = None;
        if let Some(dec) = self.decision
            && dec.allowed
            && !dec.expired
            && dec.scope == PermissionScope::Timed
            && let Some(started) = self.granted_at
        {
            let elapsed = started.elapsed().as_secs_f32();
            let total_f = total_grant_secs as f32;
            if elapsed >= total_f {
                let exp = PermissionDecision {
                    allowed: false,
                    scope: PermissionScope::Timed,
                    expired: true,
                };
                self.decision = Some(exp);
                self.granted_at = None;
                cx.emit(PermissionRequestEvent::DecisionChanged(Some(exp)));
            } else {
                let rem = (total_f - elapsed).ceil() as u32;
                let ratio = ((total_f - elapsed) / total_f).clamp(0.0, 1.0);
                timed_remaining = Some((rem, ratio));
                if !disabled_motion(cx) {
                    w.request_animation_frame();
                }
            }
        }

        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.0;
        if self.decision.is_none() && !disabled_motion(cx) {
            w.request_animation_frame();
        }

        let cw = self.measured_width.get();
        let narrow = cw < 340.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.08)
        } else {
            hsla(0.0, 0.0, 0.0, 0.06)
        };
        let wash_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.04)
        } else {
            hsla(0.0, 0.0, 0.0, 0.03)
        };

        let is_pending = self.decision.is_none();

        let action_localized: SharedString = if self.action.as_ref() == "push to main" {
            tr(cx, "push to main", "推送到 main 主分支").into()
        } else {
            self.action.clone()
        };
        let reason_localized: SharedString = if self.reason.as_ref()
            == "The fix for the flaky checkout test passed review and all checks."
        {
            tr(
                cx,
                "The fix for the flaky checkout test passed review and all checks.",
                "针对结账偶发失败用例的修复已通过代码评审与全部自动化检查。",
            )
            .into()
        } else {
            self.reason.clone()
        };

        // 1. Pending Ask Tree
        let risk_tag = match self.risk {
            PermissionRisk::Safe => None,
            PermissionRisk::Risky => Some(
                div()
                    .h(px(18.0))
                    .pl(px(4.5))
                    .pr(px(6.0))
                    .mt(px(1.0))
                    .flex_none()
                    .rounded(px(4.0))
                    .bg(p.amber_soft)
                    .text_color(p.amber_ink)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .child(pro_svg("0 0 24 24", WARN_MINI_SVG, 11.5, p.amber_ink))
                    .child(tr(cx, "Risky", "有风险")),
            ),
            PermissionRisk::Destructive => Some(
                div()
                    .h(px(18.0))
                    .pl(px(4.5))
                    .pr(px(6.0))
                    .mt(px(1.0))
                    .flex_none()
                    .rounded(px(4.0))
                    .bg(p.red_soft)
                    .text_color(p.red_ink)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(3.0))
                    .child(pro_svg("0 0 24 24", DANGER_MINI_SVG, 11.5, p.red_ink))
                    .child(tr(cx, "Destructive", "破坏性")),
            ),
        };

        let target_line = if let Some(m) = &self.agent_model {
            format!("{} · {m}", self.target)
        } else {
            self.target.to_string()
        };

        let visible_preview: Vec<PermissionPreviewItem> = self
            .preview
            .iter()
            .take(self.preview_limit)
            .cloned()
            .collect();
        let extra_preview: Vec<PermissionPreviewItem> = self
            .preview
            .iter()
            .skip(self.preview_limit)
            .cloned()
            .collect();
        let extra_cnt = extra_preview.len();

        let render_prev_item = |item: &PermissionPreviewItem, cx: &Context<Self>| {
            let lbl: SharedString =
                if item.label.as_ref() == "Deploys to production after the merge" {
                    tr(
                        cx,
                        "Deploys to production after the merge",
                        "合并后自动触发生产环境部署",
                    )
                    .into()
                } else {
                    item.label.clone()
                };
            let item_open = self.expanded_items.contains(&item.id);
            let item_id = item.id.clone();
            let detail_lines: Vec<(Hsla, &'static str, SharedString)> = match item.id.as_ref() {
                "cart" => vec![
                    (p.red_ink, "−", "if (cart.isLocked) return null;".into()),
                    (
                        p.green_ink,
                        "+",
                        "await cart.acquireCheckoutLease({ timeoutMs: 1200 });".into(),
                    ),
                ],
                "spec" => vec![
                    (
                        p.green_ink,
                        "+",
                        "test('retries transient 409 checkout lock', async () => {".into(),
                    ),
                    (
                        p.green_ink,
                        "+",
                        "  await expect(submitOrder(cart)).resolves.toBeOk();".into(),
                    ),
                ],
                "deploy" => vec![(
                    p.text,
                    "·",
                    tr(
                        cx,
                        "pipeline: web-prod · region: us-east-1 · auto-rollback: on",
                        "流水线: web-prod · 区域: us-east-1 · 自动回滚: 已开启",
                    )
                    .into(),
                )],
                _ => vec![(
                    p.muted,
                    "·",
                    format!(
                        "{} · {}",
                        item.label,
                        item.meta.as_deref().unwrap_or("impact verified")
                    )
                    .into(),
                )],
            };

            let row_face = div()
                .w_full()
                .min_h(px(30.0))
                .px(px(6.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(pro_icon(item.icon, 14.5, p.muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.5))
                        .text_color(p.ink)
                        .truncate()
                        .child(lbl.clone()),
                )
                .when_some(item.meta.clone(), |d, meta| {
                    d.child(
                        div()
                            .flex_none()
                            .font_family(crate::components::pro_chrome::pro_mono_name())
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(meta),
                    )
                })
                .child(pro_icon(
                    if item_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    11.5,
                    p.faint,
                ));

            let row_btn = pro_tactile(
                pro_eid(&self.id, format!("pr-item-{}", item.id)),
                lbl,
                6.0,
                transparent_black(),
                wash_bg,
                transparent_black(),
                transparent_black(),
                item_open,
                row_face,
                cx,
                move |this, _, cx| {
                    if this.expanded_items.contains(&item_id) {
                        this.expanded_items.remove(&item_id);
                    } else {
                        this.expanded_items.insert(item_id.clone());
                    }
                    cx.notify();
                },
            )
            .w_full();

            div()
                .w_full()
                .flex()
                .flex_col()
                .child(row_btn)
                .child(pro_fold(
                    pro_eid(&self.id, format!("pr-item-fold-{}", item.id)),
                    item_open,
                    44.0,
                    0.0,
                    move || {
                        let mut snippet = div()
                            .ml(px(26.0))
                            .mr(px(4.0))
                            .mb(px(4.0))
                            .px(px(8.0))
                            .py(px(5.0))
                            .rounded(px(6.0))
                            .bg(wash_bg)
                            .border_1()
                            .border_color(p.line_soft)
                            .flex()
                            .flex_col()
                            .gap(px(2.0));
                        for (col, sigil, line) in detail_lines {
                            snippet = snippet.child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.0))
                                    .font_family(crate::components::pro_chrome::pro_mono_name())
                                    .text_size(px(10.5))
                                    .line_height(px(15.0))
                                    .text_color(col)
                                    .child(div().w(px(8.0)).flex_none().child(sigil))
                                    .child(div().flex_1().min_w_0().truncate().child(line)),
                            );
                        }
                        snippet.into_any_element()
                    },
                ))
        };

        let mut prev_list = div().w_full().flex().flex_col();
        for item in &visible_preview {
            prev_list = prev_list.child(render_prev_item(item, cx));
        }

        let mut extra_list = div().w_full().flex().flex_col();
        for item in &extra_preview {
            extra_list = extra_list.child(render_prev_item(item, cx));
        }

        let prev_expanded = self.preview_expanded;
        let prev_title_str: SharedString = if self.preview_title.as_ref() == "What will happen" {
            tr(cx, "What will happen", "即将产生的变更影响").into()
        } else {
            self.preview_title.clone()
        };

        let preview_section = (!self.preview.is_empty()).then(|| {
            let total_prev = self.preview.len();
            div()
                .w_full()
                .mt(px(14.0))
                .pt(px(12.0))
                .pb(px(4.0))
                .px(px(10.0))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .px(px(6.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.muted)
                                .child(prev_title_str),
                        )
                        .child(
                            div()
                                .min_w(px(16.0))
                                .h(px(16.0))
                                .px(px(4.5))
                                .rounded(px(4.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(format!("{total_prev}")),
                        ),
                )
                .child(prev_list)
                .when(extra_cnt > 0, |d| {
                    let more_txt: SharedString = if prev_expanded {
                        tr(cx, "Show less", "收起").into()
                    } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                        format!("还有 {extra_cnt} 项影响").into()
                    } else {
                        format!("{extra_cnt} more").into()
                    };
                    d.child(pro_fold(
                        pro_eid(&self.id, "pr-extra-fold"),
                        prev_expanded,
                        (extra_cnt as f32) * 30.0,
                        0.0,
                        move || extra_list.into_any_element(),
                    ))
                    .child(div().ml(px(22.0)).self_start().child(pro_btn(
                        pro_eid(&self.id, "pr-more-btn"),
                        more_txt,
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.preview_expanded = !this.preview_expanded;
                            cx.emit(PermissionRequestEvent::PreviewExpanded(
                                this.preview_expanded,
                            ));
                            cx.notify();
                        },
                    )))
                })
        });

        let avail_scopes = self.available_scopes();
        let cur_scope = self.scope;
        let dur_mins = self.duration_mins;

        let scope_selector = (avail_scopes.len() > 1).then(|| {
            let sensor = pro_selection_sensor();
            let mut strip = div()
                .relative()
                .w_full()
                .p(px(2.5))
                .rounded(px(8.0))
                .bg(fill_bg)
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "pr-scope-pill"),
                    cur_scope.key(),
                    sensor.clone(),
                    6.0,
                    card_bg,
                    Some(p.line_strong),
                    true,
                ));

            for sc in &avail_scopes {
                let sc_val = *sc;
                let selected = sc_val == cur_scope;
                let lbl = sc_val.pill_label(dur_mins, cx);
                strip = strip.child(
                    div()
                        .id(pro_eid(&self.id, format!("scope-{}", sc_val.key())))
                        .role(Role::RadioButton)
                        .aria_label(lbl.clone())
                        .aria_toggled(selected.into())
                        .tab_index(if selected { 0 } else { -1 })
                        .relative()
                        .flex_1()
                        .h(px(25.0))
                        .px(px(6.0))
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
                            this.set_scope(sc_val, cx);
                        }))
                        .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                        .child(div().relative().child(lbl)),
                );
            }
            strip
        });

        let scope_hint: SharedString = match cur_scope {
            PermissionScope::Once => tr(
                cx,
                "Applies to this single action only.",
                "仅对本次请求生效，下次仍将询问。",
            )
            .into(),
            PermissionScope::Timed => {
                if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("授权将在 {dur_mins} 分钟后自动到期，期间可随时撤销。").into()
                } else {
                    format!("Expires automatically in {dur_mins} minutes — revocable anytime.")
                        .into()
                }
            }
            PermissionScope::Always => tr(
                cx,
                "Saved for future runs until you revoke it.",
                "后续运行自动放行，直到你手动撤销。",
            )
            .into(),
        };

        let deny_lbl = tr(cx, "Deny", "拒绝");
        let allow_lbl = cur_scope.button_label(dur_mins, cx);
        let is_destructive = self.risk == PermissionRisk::Destructive;
        let primary_bg = if is_destructive { p.red } else { p.primary };
        let primary_fg: Hsla = if is_destructive {
            rgb(0xffffff).into()
        } else {
            p.on_primary
        };

        let deny_inner = div()
            .flex()
            .items_center()
            .gap(px(7.0))
            .child(deny_lbl)
            .when(!narrow, |d| {
                d.child(
                    div()
                        .h(px(18.0))
                        .px(px(4.5))
                        .rounded(px(4.0))
                        .bg(fill_bg)
                        .flex()
                        .items_center()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child("Esc"),
                )
            });

        let allow_inner = div()
            .flex()
            .items_center()
            .gap(px(7.0))
            .child(pro_swap_label(
                pro_eid(&self.id, "pr-allow-lbl"),
                allow_lbl.clone(),
                13.0,
                FontWeight::MEDIUM,
                primary_fg,
            ))
            .when(!narrow, |d| {
                d.child(
                    div()
                        .h(px(18.0))
                        .px(px(4.5))
                        .rounded(px(4.0))
                        .bg(primary_fg.opacity(0.16))
                        .flex()
                        .items_center()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(primary_fg.opacity(0.78))
                        .child("↵"),
                )
            });

        let ask_tree = div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .pt(px(14.0))
                    .pl(px(16.0))
                    .pr(px(14.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(pro_agent_avatar(
                        self.agent_name.as_ref(),
                        p.blue,
                        28.0,
                        true,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_baseline()
                                    .gap(px(4.0))
                                    .text_size(px(13.0))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(self.agent_name.clone()),
                                    )
                                    .child(div().text_color(p.text).child(tr(
                                        cx,
                                        "wants to",
                                        "请求执行",
                                    )))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(action_localized.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .min_w_0()
                                    .child(pro_app_tile(self.tool_app.as_ref(), 16.0, cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_size(px(12.0))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(target_line),
                                    ),
                            ),
                    )
                    .when_some(risk_tag, |d, tag| d.child(tag)),
            )
            .child(
                div()
                    .mt(px(12.0))
                    .ml(px(if narrow { 16.0 } else { 54.0 }))
                    .mr(px(16.0))
                    .pl(px(10.0))
                    .border_l_2()
                    .border_color(fill_bg)
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .text_color(p.text)
                    .child(reason_localized),
            )
            .when_some(preview_section, |d, ps| d.child(ps))
            .child(
                div()
                    .w_full()
                    .mt(px(if self.preview.is_empty() { 14.0 } else { 4.0 }))
                    .pt(px(12.0))
                    .pb(px(15.0))
                    .px(px(16.0))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .when_some(scope_selector, |d, ss| d.child(ss))
                    .child(div().w_full().overflow_hidden().child(pro_swap_label(
                        pro_eid(&self.id, "pr-scope-hint"),
                        scope_hint,
                        11.0,
                        FontWeight::NORMAL,
                        p.muted,
                    )))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .child(PrTactileBtn {
                                id: pro_eid(&self.id, "pr-deny-btn"),
                                aria: deny_lbl.into(),
                                flex_fill: narrow,
                                pad_l: 12.0,
                                pad_r: if narrow { 12.0 } else { 8.0 },
                                rest_bg: card_bg,
                                hover_bg: wash_bg,
                                fg: p.ink,
                                border: p.line_strong,
                                elevated: true,
                                content: deny_inner.into_any_element(),
                                on_click: Rc::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.deny(cx);
                                })),
                            })
                            .child(PrTactileBtn {
                                id: pro_eid(&self.id, "pr-allow-btn"),
                                aria: allow_lbl,
                                flex_fill: narrow,
                                pad_l: 12.0,
                                pad_r: if narrow { 12.0 } else { 8.0 },
                                rest_bg: primary_bg,
                                hover_bg: pro_mix_color(primary_bg, primary_fg, 0.12),
                                fg: primary_fg,
                                border: transparent_black(),
                                elevated: true,
                                content: allow_inner.into_any_element(),
                                on_click: Rc::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.allow(cx);
                                })),
                            }),
                    ),
            );

        // 2. Resolved Status Line (with progressive `pro_check_draw` & tactile `Revoke`)
        let dec_val = self.decision.unwrap_or(PermissionDecision {
            allowed: true,
            scope: PermissionScope::Timed,
            expired: false,
        });
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let settled_age = self.settled_clock.time(0.28, w, cx);
        let settled_chk_on = dec_val.allowed && (disabled_motion(cx) || settled_age > 0.012);

        let (icon_el, head_lead, note_line, ghost_lbl): (
            AnyElement,
            SharedString,
            SharedString,
            &'static str,
        ) = if dec_val.allowed {
            let lead: SharedString = match dec_val.scope {
                PermissionScope::Once => tr(cx, "Allowed once", "已允许一次").into(),
                PermissionScope::Timed => {
                    if dur_mins == 60 {
                        tr(cx, "Allowed for 1 hour", "已允许 1 小时").into()
                    } else if zh {
                        format!("已允许 {dur_mins} 分钟").into()
                    } else {
                        format!("Allowed for {dur_mins} min").into()
                    }
                }
                PermissionScope::Always => tr(cx, "Always allowed", "已设为始终允许").into(),
            };
            let note: SharedString = match dec_val.scope {
                PermissionScope::Timed => {
                    let rem_s = timed_remaining.map_or(total_grant_secs, |(r, _)| r);
                    format!("{} · {}", format_remaining_grant(rem_s, zh), self.target).into()
                }
                PermissionScope::Once => {
                    if zh {
                        format!("仅本次调用有效 · {}", self.target).into()
                    } else {
                        format!("Single request grant · {}", self.target).into()
                    }
                }
                PermissionScope::Always => {
                    if zh {
                        format!("长期生效，可随时撤销 · {}", self.target).into()
                    } else {
                        format!("Active until revoked · {}", self.target).into()
                    }
                }
            };
            (
                pro_check_draw(
                    pro_eid(&self.id, "pr-settled-chk"),
                    settled_chk_on,
                    18.0,
                    false,
                    p.green,
                ),
                lead,
                note,
                tr(cx, "Revoke", "撤销"),
            )
        } else if dec_val.expired {
            (
                pro_icon(ProIcon::Clock, 17.0, p.muted),
                tr(cx, "Grant expired", "限时授权已到期").into(),
                format!(
                    "{} · {}",
                    tr(cx, "Timed window ended", "授权窗口已结束"),
                    self.target
                )
                .into(),
                tr(cx, "Ask again", "重新授权"),
            )
        } else {
            (
                pro_svg("0 0 24 24", CIRCLE_CROSS_SVG, 17.5, p.red_ink),
                tr(cx, "Denied", "已拒绝权限").into(),
                format!("{} · {}", tr(cx, "Blocked", "已拦截"), self.target).into(),
                tr(cx, "Change", "更改决定"),
            )
        };

        let drain_ratio = timed_remaining.map(|(_, r)| r);
        let result_row = div()
            .relative()
            .w_full()
            .min_h(px(58.0))
            .pl(px(16.0))
            .pr(px(10.0))
            .py(px(10.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(icon_el)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(5.0))
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(head_lead),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(format!("· {action_localized}")),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(p.muted)
                            .truncate()
                            .child(note_line),
                    ),
            )
            .child(pro_btn(
                pro_eid(&self.id, "pr-revoke-btn"),
                ghost_lbl,
                None,
                ProBtnStyle::Ghost,
                true,
                cx,
                |this, _, cx| {
                    this.revoke(cx);
                },
            ))
            .when_some(drain_ratio, |d, ratio| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(2.0))
                        .bg(wash_bg)
                        .child(div().h_full().w(relative(ratio)).bg(p.green.opacity(0.82))),
                )
            });

        let card_shell = div()
            .w_full()
            .rounded(px(10.0))
            .bg(card_bg)
            .border_1()
            .border_color(p.line_strong)
            .when(!p.dark, |d| d.shadow_xs())
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(pro_fold(
                pro_eid(&self.id, "pr-ask-fold"),
                is_pending,
                298.0,
                0.0,
                move || ask_tree.into_any_element(),
            ))
            .child(pro_fold(
                pro_eid(&self.id, "pr-res-fold"),
                !is_pending,
                58.0,
                0.0,
                move || result_row.into_any_element(),
            ));

        let width_sensor = self.measured_width.clone();
        let a11y_label: SharedString =
            format!("{} wants to {}", self.agent_name, self.action).into();
        let focus_ring = p.focus;

        div()
            .id(self.id.clone())
            .role(Role::Dialog)
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
            .focus_visible(move |s| s.rounded(px(12.0)).border_color(focus_ring))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if this.decision.is_some() {
                    return;
                }
                let avail = this.available_scopes();
                let cur_pos = avail.iter().position(|s| *s == this.scope).unwrap_or(0);
                match ev.keystroke.key.as_str() {
                    "left" => {
                        if !avail.is_empty() {
                            let next = avail[cur_pos.saturating_sub(1)];
                            this.set_scope(next, cx);
                        }
                        cx.stop_propagation();
                    }
                    "right" => {
                        if !avail.is_empty() {
                            let next = avail[(cur_pos + 1).min(avail.len() - 1)];
                            this.set_scope(next, cx);
                        }
                        cx.stop_propagation();
                    }
                    "enter" => {
                        this.allow(cx);
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.deny(cx);
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
                    .child(card_shell),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{PermissionRequest, PermissionRisk, PermissionScope, format_remaining_grant};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn format_remaining_grant_renders_clean_tabular_text() {
        assert_eq!(format_remaining_grant(3598, false), "59 min 58s left");
        assert_eq!(format_remaining_grant(42, false), "42s left");
    }

    #[gpui::test]
    fn permission_request_grants_revokes_and_blocks_destructive_always(cx: &mut TestAppContext) {
        let req = cx.new(|cx| PermissionRequest::new("test-permission-request", cx));

        req.update(cx, |r, cx| {
            assert_eq!(r.scope(), PermissionScope::Timed);
            assert!(r.decision().is_none());

            r.allow(cx);
            assert_eq!(
                r.decision().map(|d| (d.allowed, d.scope)),
                Some((true, PermissionScope::Timed))
            );

            r.revoke(cx);
            assert!(r.decision().is_none());

            r.set_scope(PermissionScope::Always, cx);
            assert_eq!(r.scope(), PermissionScope::Always);
        });

        let destructive = cx.new(|cx| {
            PermissionRequest::new("test-destructive", cx)
                .with_risk(PermissionRisk::Destructive)
                .with_scope(PermissionScope::Always)
        });
        destructive.update(cx, |r, cx| {
            assert_eq!(r.scope(), PermissionScope::Once);
            r.set_scope(PermissionScope::Always, cx);
            assert_eq!(r.scope(), PermissionScope::Once);
        });
    }
}
