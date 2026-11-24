//! Overnight `"While you were away"` recap widget with `PathBuilder` event-peak curve and 3-lane action board.
//!
//! Plots a miniature Gaussian event-peak curve across the offline window with interactive colored
//! dots per overnight milestone, paired below with equal-height `Needs you`, `Failed`, and `Done`
//! lanes where approvals, merges, retries, and Undo/Redo happen directly on `90px` cards.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_check_draw, pro_ease, pro_eid, pro_empty_state, pro_mix_color,
        pro_popover_presence, pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_swap_label,
        pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::activate,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const ICON_DONE_SVG: &str = r#"<circle cx="8" cy="8" r="5.75" stroke="currentColor" stroke-width="1.35"></circle><path d="m5.6 8.2 1.6 1.6 3.2-3.4" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const ICON_NEEDS_SVG: &str = r#"<circle cx="8" cy="5.25" r="2.5" stroke="currentColor" stroke-width="1.35"></circle><path d="M3.25 13.25c.6-2.35 2.45-3.75 4.75-3.75s4.15 1.4 4.75 3.75" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"></path>"#;
const ICON_FAILED_SVG: &str = r#"<path d="M7.13 2.75a1 1 0 0 1 1.74 0l5.2 9.25a1 1 0 0 1-.87 1.5H2.8a1 1 0 0 1-.87-1.5z" stroke="currentColor" stroke-width="1.35" stroke-linejoin="round"></path><path d="M8 6.5v2.75M8 11.25v.01" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"></path>"#;

const CARD_H: f32 = 90.0;
const CHART_H: f32 = 96.0;

/// Lane bucket of an overnight [`RecapEvent`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecapKind {
    /// Waiting on a human approval, merge, or confirmation (`Needs you`).
    Needs,
    /// Halted due to an error (`Failed`, surfaces `Retry`).
    Failed,
    /// Completed autonomously (`Done`, surfaces `Undo`).
    #[default]
    Done,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RecapCardState {
    #[default]
    Open,
    Resolved,
    Undone,
}

/// Single overnight milestone plotted on the curve and grouped into [`AgentRecap`].
#[derive(Clone, Debug, PartialEq)]
pub struct RecapEvent {
    /// Stable event identifier.
    pub id: SharedString,
    /// Lane category (`Needs`, `Failed`, or `Done`).
    pub kind: RecapKind,
    /// Normalized position along the offline window (`0.0..=1.0`).
    pub position: f32,
    /// Display timestamp (`"2:14 AM"`, `"7:05 PM"`).
    pub at: SharedString,
    /// Concise card title.
    pub title: SharedString,
    /// Optional detail or failure reason line.
    pub detail: Option<SharedString>,
    /// Optional provider/app mark key (`"slack"`, `"github"`, `"drive"`, `"linear"`).
    pub logo: Option<SharedString>,
    /// Optional aggregated item count (`38` tickets, `6` PRs) — also scales peak height.
    pub count: Option<u32>,
    /// Primary action label on `Needs` / `Failed` cards (`"Approve"`, `"Merge"`, `"Retry"`).
    pub action: Option<SharedString>,
    /// Past-tense badge shown after resolving (`"Approved"`, `"Merged"`, `"Fixed"`).
    pub done_label: Option<SharedString>,
}

impl RecapEvent {
    /// Construct a new [`RecapEvent`] at normalized time `position` (`0.0..=1.0`).
    pub fn new(
        id: impl Into<SharedString>,
        kind: RecapKind,
        position: f32,
        at: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            position: position.clamp(0.02, 0.98),
            at: at.into(),
            title: title.into(),
            detail: None,
            logo: None,
            count: None,
            action: None,
            done_label: None,
        }
    }

    /// Attach a secondary detail or error description.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the app/provider artwork key (`"slack"`, `"github"`, `"drive"`, `"linear"`).
    pub fn with_logo(mut self, logo: impl Into<SharedString>) -> Self {
        self.logo = Some(logo.into());
        self
    }

    /// Set an aggregated item count pill (`38`, `17`, `6`).
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Customize the primary button label and resolved past-tense badge.
    pub fn with_action_labels(
        mut self,
        action: impl Into<SharedString>,
        done_label: impl Into<SharedString>,
    ) -> Self {
        self.action = Some(action.into());
        self.done_label = Some(done_label.into());
        self
    }

    fn peak_weight(&self) -> f32 {
        match self.count {
            Some(c) if c >= 30 => 0.88,
            Some(c) if c >= 12 => 0.78,
            Some(c) if c >= 4 => 0.66,
            _ => 0.62,
        }
    }
}

/// Strongly typed events emitted by [`AgentRecap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRecapEvent {
    /// Emitted when the primary action (`Approve`, `Merge`, `Retry`) executes on a card.
    PrimaryAction(SharedString),
    /// Emitted when `Review` is clicked on a `Needs you` card.
    Review(SharedString),
    /// Emitted when `Undo` reverts a resolved action or an autonomous `Done` card.
    Undo(SharedString),
    /// Emitted when `Redo` reapplies an undone `Done` card.
    Redo(SharedString),
    /// Emitted when hovering or selecting a peak dot on the chart.
    SpotlightChanged(Option<SharedString>),
}

impl EventEmitter<AgentRecapEvent> for AgentRecap {}

pub(crate) fn eval_recap_curve(u: f32, peaks: &[(f32, f32)]) -> f32 {
    let sigma = 0.036_f32;
    let mut sum = 0.0_f32;
    for &(pu, amp) in peaks {
        let z = (u - pu) / sigma;
        sum += amp * (-0.5 * z * z).exp();
    }
    sum.clamp(0.0, 0.94)
}

fn default_recap_events(cx: &App) -> Vec<RecapEvent> {
    vec![
        RecapEvent::new(
            "refund",
            RecapKind::Needs,
            0.56,
            "2:14 AM",
            tr(cx, "Refund $240 to Acme Corp", "向 Acme Corp 退款 $240"),
        )
        .with_logo("slack")
        .with_detail(tr(
            cx,
            "Support agent · over the $200 auto-approve limit",
            "客服智能体 · 超出 $200 自动放行额度",
        ))
        .with_action_labels(tr(cx, "Approve", "批准"), tr(cx, "Approved", "已批准")),
        RecapEvent::new(
            "merge",
            RecapKind::Needs,
            0.735,
            "4:37 AM",
            tr(cx, "Merge the checkout fix", "合并结账修复补丁"),
        )
        .with_logo("github")
        .with_detail(tr(
            cx,
            "All 212 tests green · needs maintainer merge",
            "全部 212 项测试通过 · 待维护者合并",
        ))
        .with_action_labels(tr(cx, "Merge", "合并"), tr(cx, "Merged", "已合并")),
        RecapEvent::new(
            "sync",
            RecapKind::Failed,
            0.62,
            "3:02 AM",
            tr(cx, "Contact sync to the CRM", "同步联系人至 CRM"),
        )
        .with_logo("drive")
        .with_detail(tr(
            cx,
            "Token expired after 1,284 of 3,900 contacts",
            "同步至 1,284 / 3,900 位联系人时令牌过期",
        ))
        .with_action_labels(tr(cx, "Retry", "重试"), tr(cx, "Fixed", "已修复")),
        RecapEvent::new(
            "tickets",
            RecapKind::Done,
            0.035,
            "7:05 PM",
            tr(cx, "Answered support tickets", "自动回复客服工单"),
        )
        .with_logo("slack")
        .with_detail(tr(cx, "first reply in 2 min", "平均首响 2 分钟"))
        .with_count(38),
        RecapEvent::new(
            "prs",
            RecapKind::Done,
            0.115,
            "8:10 PM",
            tr(cx, "Closed stale pull requests", "关闭陈旧拉取请求"),
        )
        .with_logo("github")
        .with_count(6),
        RecapEvent::new(
            "issues",
            RecapKind::Done,
            0.285,
            "10:30 PM",
            tr(cx, "Triaged new issues", "完成新问题分级分派"),
        )
        .with_logo("linear")
        .with_detail(tr(cx, "4 marked urgent", "4 项标记为紧急"))
        .with_count(17),
    ]
}

/// Overnight `"While you were away"` recap widget with peak curve and 3-lane triage board.
pub struct AgentRecap {
    id: ElementId,
    title: Option<SharedString>,
    since_label: SharedString,
    until_label: SharedString,
    duration_pill: SharedString,
    events: Vec<RecapEvent>,
    states: HashMap<SharedString, RecapCardState>,
    limit: usize,
    replay_ms: u64,
    spotlight_id: Option<SharedString>,
    last_spotlight_id: Option<SharedString>,
    loading: bool,
    empty: bool,
    started_at: Instant,
    measured_w: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    announcement: SharedString,
}

impl AgentRecap {
    /// Create an [`AgentRecap`] preloaded with the 6-event overnight window (`6:40 PM` to `8:10 AM`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            since_label: "6:40 PM".into(),
            until_label: "8:10 AM".into(),
            duration_pill: "13h 30m".into(),
            events: default_recap_events(cx),
            states: HashMap::new(),
            limit: 3,
            replay_ms: 1_200,
            spotlight_id: None,
            last_spotlight_id: None,
            loading: false,
            empty: false,
            started_at: Instant::now(),
            measured_w: Rc::new(Cell::new(804.0)),
            toast: None,
            announcement: SharedString::default(),
        }
    }

    /// Override the main heading (`"While you were away"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Customize the offline window labels (`since`, `until`, and elapsed `duration` badge).
    pub fn with_window(
        mut self,
        since: impl Into<SharedString>,
        until: impl Into<SharedString>,
        duration: impl Into<SharedString>,
    ) -> Self {
        self.since_label = since.into();
        self.until_label = until.into();
        self.duration_pill = duration.into();
        self
    }

    /// Provide custom overnight [`RecapEvent`] items.
    pub fn with_events(mut self, events: Vec<RecapEvent>) -> Self {
        self.events = events;
        self
    }

    /// Set the maximum cards shown per lane (`3` default; lanes reserve equal height for `limit`).
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// Configure the initial curve replay duration in milliseconds (`0` shows everything immediately).
    pub fn with_replay_ms(mut self, replay_ms: u64) -> Self {
        self.replay_ms = replay_ms;
        self
    }

    /// Render skeleton placeholders while overnight events load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty overnight recap state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    fn card_state(&self, id: &SharedString) -> RecapCardState {
        self.states.get(id).copied().unwrap_or(RecapCardState::Open)
    }

    fn run_primary(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let Some(ev) = self.events.iter().find(|e| e.id == id) else {
            return;
        };
        let done_lbl = ev.done_label.clone().unwrap_or_else(|| match ev.kind {
            RecapKind::Needs => tr(cx, "Approved", "已批准").into(),
            RecapKind::Failed => tr(cx, "Fixed", "已修复").into(),
            RecapKind::Done => tr(cx, "Done", "已完成").into(),
        });
        let msg: SharedString = format!("{done_lbl}: {}", ev.title).into();
        self.states.insert(id.clone(), RecapCardState::Resolved);
        self.announcement = msg.clone();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentRecapEvent::PrimaryAction(id));
        cx.notify();
    }

    fn run_review(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if let Some(ev) = self.events.iter().find(|e| e.id == id) {
            let msg: SharedString =
                format!("{}: {}", tr(cx, "Opened review", "已打开复核"), ev.title).into();
            self.toast = Some((msg, Instant::now()));
        }
        self.last_spotlight_id = Some(id.clone());
        self.spotlight_id = Some(id.clone());
        cx.emit(AgentRecapEvent::Review(id));
        cx.notify();
    }

    fn run_undo(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let Some(ev) = self.events.iter().find(|e| e.id == id) else {
            return;
        };
        let next_state = match ev.kind {
            RecapKind::Done => RecapCardState::Undone,
            RecapKind::Needs | RecapKind::Failed => RecapCardState::Open,
        };
        self.states.insert(id.clone(), next_state);
        let msg: SharedString = format!("{}: {}", tr(cx, "Undid", "已撤销"), ev.title).into();
        self.announcement = msg.clone();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentRecapEvent::Undo(id));
        cx.notify();
    }

    fn run_redo(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let Some(ev) = self.events.iter().find(|e| e.id == id) else {
            return;
        };
        self.states.insert(id.clone(), RecapCardState::Open);
        let msg: SharedString = format!("{}: {}", tr(cx, "Reapplied", "已重做"), ev.title).into();
        self.announcement = msg.clone();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentRecapEvent::Redo(id));
        cx.notify();
    }

    fn set_spotlight(&mut self, next: Option<SharedString>, cx: &mut Context<Self>) {
        if self.spotlight_id != next {
            if let Some(id) = &next {
                self.last_spotlight_id = Some(id.clone());
            }
            self.spotlight_id = next.clone();
            cx.emit(AgentRecapEvent::SpotlightChanged(next));
            cx.notify();
        }
    }

    fn live_counts(&self) -> (usize, usize, usize) {
        let mut done_n = 0usize;
        let mut needs_n = 0usize;
        let mut failed_n = 0usize;
        for ev in &self.events {
            let st = self.card_state(&ev.id);
            match (ev.kind, st) {
                (RecapKind::Needs, RecapCardState::Open) => needs_n += 1,
                (RecapKind::Needs, RecapCardState::Resolved) => done_n += 1,
                (RecapKind::Failed, RecapCardState::Open) => failed_n += 1,
                (RecapKind::Failed, RecapCardState::Resolved) => done_n += 1,
                (RecapKind::Done, RecapCardState::Open | RecapCardState::Resolved) => done_n += 1,
                (_, RecapCardState::Undone) => {}
            }
        }
        (done_n, needs_n, failed_n)
    }

    fn render_card(&self, ev: &RecapEvent, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let state = self.card_state(&ev.id);
        let is_hot = self.spotlight_id.as_ref() == Some(&ev.id);
        let is_quiet = matches!(state, RecapCardState::Resolved | RecapCardState::Undone);

        let is_done_check = state == RecapCardState::Resolved
            || (ev.kind == RecapKind::Done && state == RecapCardState::Open);

        let mark_el: AnyElement = if state == RecapCardState::Resolved {
            pro_check_draw(
                pro_eid(&self.id, format!("rcard-chk-{}", ev.id)),
                true,
                18.0,
                true,
                p.green_ink,
            )
        } else if let Some(logo) = &ev.logo
            && let Some(art) = pro_app_artwork(logo, 11.5, cx)
        {
            div()
                .size(px(18.0))
                .flex_none()
                .rounded(px(5.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .opacity(if state == RecapCardState::Undone {
                    0.5
                } else {
                    1.0
                })
                .child(art)
                .into_any_element()
        } else if is_done_check {
            pro_check_draw(
                pro_eid(&self.id, format!("rcard-chk-{}", ev.id)),
                true,
                18.0,
                true,
                p.green_ink,
            )
        } else {
            div()
                .size(px(18.0))
                .flex_none()
                .rounded(px(5.0))
                .bg(p.chip)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::Sparkles, 11.0, p.muted))
                .into_any_element()
        };

        let detail_color = if ev.kind == RecapKind::Failed && state == RecapCardState::Open {
            p.red
        } else {
            p.muted
        };

        let top_row = div()
            .w_full()
            .h(px(20.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(mark_el)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if is_quiet { p.muted } else { p.ink })
                    .truncate()
                    .child(ev.title.clone()),
            );

        let meta_row = div()
            .w_full()
            .h(px(18.0))
            .pl(px(26.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(12.0))
                    .text_color(detail_color)
                    .truncate()
                    .child(ev.detail.clone().unwrap_or_default()),
            )
            .when_some(ev.count, |d, cnt| {
                d.child(
                    div()
                        .h(px(16.0))
                        .px(px(5.0))
                        .flex_none()
                        .rounded(px(4.0))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("rcard-cnt-{}", ev.id)),
                            cnt as f64,
                            0,
                            "",
                            "",
                            10.5,
                            FontWeight::MEDIUM,
                            p.text,
                        )),
                )
            })
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(p.faint)
                    .child(ev.at.clone()),
            );

        // Footer row (`height: 24px`): state pill on left + tactile action buttons on right.
        let ev_id = ev.id.clone();
        let hover_ghost = p.hover;
        let hover_main = pro_mix_color(card_bg, p.ink, 0.04);

        let foot_row = match (ev.kind, state) {
            (RecapKind::Needs, RecapCardState::Open) => {
                let act_lbl: SharedString = ev
                    .action
                    .clone()
                    .unwrap_or_else(|| tr(cx, "Approve", "批准").into());
                let rev_id = ev_id.clone();
                let pri_id = ev_id.clone();
                div()
                    .w_full()
                    .h(px(24.0))
                    .mt(px(6.0))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(4.0))
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("rev-{ev_id}")),
                            format!("Review: {}", ev.title),
                            6.0,
                            transparent_black(),
                            hover_ghost,
                            transparent_black(),
                            transparent_black(),
                            false,
                            div()
                                .h(px(24.0))
                                .px(px(8.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .flex()
                                .items_center()
                                .child(tr(cx, "Review", "查看")),
                            cx,
                            move |this, _, cx| {
                                cx.stop_propagation();
                                this.run_review(rev_id.clone(), cx);
                            },
                        )
                        .w_auto(),
                    )
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("act-{ev_id}")),
                            format!("{act_lbl}: {}", ev.title),
                            6.0,
                            card_bg,
                            hover_main,
                            p.line_strong,
                            p.line_strong,
                            false,
                            div()
                                .h(px(24.0))
                                .px(px(9.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .flex()
                                .items_center()
                                .child(act_lbl),
                            cx,
                            move |this, _, cx| {
                                cx.stop_propagation();
                                this.run_primary(pri_id.clone(), cx);
                            },
                        )
                        .w_auto()
                        .when(!p.dark, |d| d.shadow_xs()),
                    )
            }
            (RecapKind::Failed, RecapCardState::Open) => {
                let act_lbl: SharedString = ev
                    .action
                    .clone()
                    .unwrap_or_else(|| tr(cx, "Retry", "重试").into());
                let pri_id = ev_id.clone();
                div()
                    .w_full()
                    .h(px(24.0))
                    .mt(px(6.0))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("retry-{ev_id}")),
                            format!("{act_lbl}: {}", ev.title),
                            6.0,
                            card_bg,
                            hover_main,
                            p.line_strong,
                            p.line_strong,
                            false,
                            div()
                                .h(px(24.0))
                                .px(px(9.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .flex()
                                .items_center()
                                .child(act_lbl),
                            cx,
                            move |this, _, cx| {
                                cx.stop_propagation();
                                this.run_primary(pri_id.clone(), cx);
                            },
                        )
                        .w_auto()
                        .when(!p.dark, |d| d.shadow_xs()),
                    )
            }
            (_, RecapCardState::Undone) => {
                let redo_id = ev_id.clone();
                div()
                    .w_full()
                    .h(px(24.0))
                    .mt(px(6.0))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .h(px(20.0))
                            .px(px(6.0))
                            .rounded(px(5.0))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("rcard-st-{ev_id}")),
                                tr(cx, "Undone", "已撤销"),
                                11.5,
                                FontWeight::MEDIUM,
                                p.muted,
                            )),
                    )
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("redo-{ev_id}")),
                            format!("Redo: {}", ev.title),
                            6.0,
                            transparent_black(),
                            hover_ghost,
                            transparent_black(),
                            transparent_black(),
                            false,
                            div()
                                .h(px(24.0))
                                .px(px(8.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .flex()
                                .items_center()
                                .child(tr(cx, "Redo", "重做")),
                            cx,
                            move |this, _, cx| {
                                cx.stop_propagation();
                                this.run_redo(redo_id.clone(), cx);
                            },
                        )
                        .w_auto(),
                    )
            }
            _ => {
                let badge_lbl: SharedString = if state == RecapCardState::Resolved {
                    ev.done_label.clone().unwrap_or_else(|| match ev.kind {
                        RecapKind::Needs => tr(cx, "Approved", "已批准").into(),
                        RecapKind::Failed => tr(cx, "Fixed", "已修复").into(),
                        RecapKind::Done => tr(cx, "Done", "已完成").into(),
                    })
                } else {
                    tr(cx, "Done", "已完成").into()
                };
                let undo_id = ev_id.clone();
                div()
                    .w_full()
                    .h(px(24.0))
                    .mt(px(6.0))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .h(px(20.0))
                            .px(px(6.0))
                            .rounded(px(5.0))
                            .bg(p.green_soft)
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("rcard-pill-chk-{ev_id}")),
                                true,
                                12.0,
                                false,
                                p.green_ink,
                            ))
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("rcard-st-{ev_id}")),
                                badge_lbl,
                                11.5,
                                FontWeight::MEDIUM,
                                p.green_ink,
                            )),
                    )
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("undo-{ev_id}")),
                            format!("Undo: {}", ev.title),
                            6.0,
                            transparent_black(),
                            hover_ghost,
                            transparent_black(),
                            transparent_black(),
                            false,
                            div()
                                .h(px(24.0))
                                .px(px(8.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .flex()
                                .items_center()
                                .child(tr(cx, "Undo", "撤销")),
                            cx,
                            move |this, _, cx| {
                                cx.stop_propagation();
                                this.run_undo(undo_id.clone(), cx);
                            },
                        )
                        .w_auto(),
                    )
            }
        };

        let hov_id = ev.id.clone();
        let click_id = ev.id.clone();
        let border_c = if is_hot { p.blue.opacity(0.65) } else { p.line };
        pro_tactile(
            pro_eid(&self.id, format!("rcard-{}", ev.id)),
            format!("{} · {}", ev.at, ev.title),
            10.0,
            card_bg,
            pro_mix_color(card_bg, p.ink, 0.02),
            border_c,
            p.blue.opacity(0.55),
            is_hot,
            div()
                .w_full()
                .h(px(CARD_H))
                .px(px(10.0))
                .py(px(9.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(top_row)
                        .child(meta_row),
                )
                .child(foot_row),
            cx,
            move |this, _, cx| {
                let next = if this.spotlight_id.as_ref() == Some(&click_id) {
                    None
                } else {
                    Some(click_id.clone())
                };
                this.set_spotlight(next, cx);
            },
        )
        .when(!p.dark, |d| d.shadow_xs())
        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
            let next = if *inside {
                Some(hov_id.clone())
            } else if this.spotlight_id.as_ref() == Some(&hov_id) {
                None
            } else {
                this.spotlight_id.clone()
            };
            this.set_spotlight(next, cx);
        }))
        .into_any_element()
    }
}

impl Render for AgentRecap {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_secs = self.started_at.elapsed().as_secs_f32();
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let lane_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf4f4f5).into()
        };
        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "While you were away", "在你离开期间").into());

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_secs / 1.4).fract();
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .p(px(16.0))
                .rounded(px(14.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(180.0, 20.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(160.0, 14.0, 4.0, phase + 0.15, cx)),
                )
                .child(pro_skeleton_bar(420.0, 88.0, 8.0, phase + 0.2, cx))
                .child(div().flex().gap(px(10.0)).children((0..3usize).map(|i| {
                    div().flex_1().child(pro_skeleton_bar(
                        240.0,
                        210.0,
                        12.0,
                        phase + i as f32 * 0.1,
                        cx,
                    ))
                })));
        }

        let replay_secs = (self.replay_ms as f32 / 1_000.0).max(0.0);
        let reveal = if reduced || replay_secs <= 0.001 {
            1.0
        } else {
            let r = (elapsed_secs / replay_secs).clamp(0.0, 1.0);
            if r < 1.0 {
                window.request_animation_frame();
            }
            pro_ease(r)
        };
        if self.toast.is_some() && !reduced {
            window.request_animation_frame();
        }

        let (done_cnt, needs_cnt, failed_cnt) = self.live_counts();
        let head_row = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_start()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .line_height(px(22.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_size(px(12.0))
                            .text_color(p.muted)
                            .child(format!("{} {}", tr(cx, "Since", "始于"), self.since_label))
                            .child(
                                div()
                                    .h(px(18.0))
                                    .px(px(6.0))
                                    .rounded(px(5.0))
                                    .bg(p.chip)
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .flex()
                                    .items_center()
                                    .child(self.duration_pill.clone()),
                            ),
                    ),
            )
            .child(
                div()
                    .pt(px(3.0))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(12.0))
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .child(pro_check_draw(
                                pro_eid(&self.id, "recap-head-done-chk"),
                                done_cnt > 0,
                                14.0,
                                false,
                                p.green_ink,
                            ))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "recap-cnt-done"),
                                done_cnt as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(tr(cx, "done", "已完成")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .child(pro_svg("0 0 16 16", ICON_NEEDS_SVG, 14.0, p.muted))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "recap-cnt-needs"),
                                needs_cnt as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(tr(cx, "need you", "待确认")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .child(pro_svg("0 0 16 16", ICON_FAILED_SVG, 14.0, p.muted))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "recap-cnt-failed"),
                                failed_cnt as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(tr(cx, "failed", "失败")),
                    ),
            );

        if self.empty || self.events.is_empty() {
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .p(px(16.0))
                .rounded(px(14.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(head_row)
                .child(pro_empty_state(
                    ProIcon::Clock,
                    tr(cx, "Quiet night — zero agent events", "离线期间无智能体事件"),
                    tr(
                        cx,
                        "Overnight approvals, failures, and completed background work appear here when you return.",
                        "后台智能体在离线期间产生的待审批、异常与已完成事项会汇总在此。",
                    ),
                    None,
                    cx,
                ));
        }

        // Build peak list sorted by time position.
        let peaks: Vec<(f32, f32)> = self
            .events
            .iter()
            .map(|e| (e.position, e.peak_weight()))
            .collect();

        let grid_c = p.line_soft;
        let stroke_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.50)
        } else {
            hsla(0., 0., 0., 0.42)
        };
        let area_top: Hsla = if p.dark {
            hsla(0., 0., 1., 0.085)
        } else {
            hsla(0., 0., 0., 0.07)
        };

        let tip_ref_id = self
            .spotlight_id
            .as_ref()
            .or(self.last_spotlight_id.as_ref());
        let target_u = tip_ref_id
            .and_then(|sid| self.events.iter().find(|e| &e.id == sid))
            .map(|e| e.position)
            .unwrap_or(0.5);
        let smooth_u = pro_spring_f32(
            pro_eid(&self.id, "recap-spot-u"),
            target_u,
            0.22,
            window,
            cx,
        );
        let spot_alpha = pro_spring_f32(
            pro_eid(&self.id, "recap-spot-alpha"),
            if self.spotlight_id.is_some() {
                1.0
            } else {
                0.0
            },
            0.18,
            window,
            cx,
        );
        let spotlight_u: Option<(f32, f32)> = if spot_alpha > 0.01 {
            Some((smooth_u, spot_alpha))
        } else {
            None
        };
        let cursor_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.28)
        } else {
            hsla(0., 0., 0., 0.22)
        };

        let peaks_for_paint = peaks.clone();
        let chart_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 8.0 || h <= 8.0 {
                    return;
                }
                let pad_top = 10.0_f32;
                let pad_bot = 6.0_f32;
                let usable_h = (h - pad_top - pad_bot).max(10.0);

                // Subtle background grid (4 horizontal bands + 16 vertical columns).
                for r in 0..=4usize {
                    let gy = bounds.top() + px((r as f32 / 4.0) * (h - 1.0));
                    let mut gl = PathBuilder::stroke(px(1.0));
                    gl.move_to(point(bounds.left(), gy));
                    gl.line_to(point(bounds.right(), gy));
                    if let Ok(path) = gl.build() {
                        window.paint_path(path, grid_c);
                    }
                }
                for c in 1..16usize {
                    let gx = bounds.left() + px((c as f32 / 16.0) * w);
                    let mut gl = PathBuilder::stroke(px(1.0));
                    gl.move_to(point(gx, bounds.top()));
                    gl.line_to(point(gx, bounds.bottom()));
                    if let Ok(path) = gl.build() {
                        window.paint_path(path, grid_c);
                    }
                }

                if let Some((su, sa)) = spotlight_u {
                    let cx_line = bounds.left() + px(su * w);
                    let mut cur = PathBuilder::stroke(px(1.0));
                    cur.move_to(point(cx_line, bounds.top()));
                    cur.line_to(point(cx_line, bounds.bottom()));
                    if let Ok(path) = cur.build() {
                        window.paint_path(path, cursor_c.opacity(cursor_c.a * sa));
                    }
                }

                let samples = 140usize;
                let visible_samples =
                    (((samples as f32) * reveal).ceil() as usize).clamp(2, samples);
                let baseline_y = bounds.top() + px(h - pad_bot);
                let map_pt = |i: usize| -> Point<Pixels> {
                    let u = i as f32 / samples as f32;
                    let val = eval_recap_curve(u, &peaks_for_paint);
                    let py = (h - pad_bot) - val * usable_h;
                    point(bounds.left() + px(u * w), bounds.top() + px(py))
                };

                let mut area = PathBuilder::fill();
                let first_pt = map_pt(0);
                let last_pt = map_pt(visible_samples);
                area.move_to(point(first_pt.x, baseline_y));
                area.line_to(first_pt);
                for i in 1..=visible_samples {
                    area.line_to(map_pt(i));
                }
                area.line_to(point(last_pt.x, baseline_y));
                area.close();
                if let Ok(path) = area.build() {
                    window.paint_path(
                        path,
                        linear_gradient(
                            180.0,
                            linear_color_stop(area_top, 0.0),
                            linear_color_stop(area_top.opacity(0.0), 1.0),
                        ),
                    );
                }

                let mut line = PathBuilder::stroke(px(1.5));
                for i in 0..=visible_samples {
                    let pt = map_pt(i);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, stroke_c);
                }
            },
        )
        .absolute()
        .inset_0();

        // Interactive peak dots + floating tooltip.
        let usable_h = CHART_H - 16.0;
        let mut dots_layer = div().absolute().inset_0();
        let dot_neutral: Hsla = if p.dark {
            hsla(0., 0., 1., 0.68)
        } else {
            hsla(0., 0., 0., 0.58)
        };

        for ev in &self.events {
            if ev.position > reveal + 0.02 {
                continue;
            }
            let st = self.card_state(&ev.id);
            let dot_c = match (ev.kind, st) {
                (RecapKind::Needs, RecapCardState::Open) => p.orange,
                (RecapKind::Failed, RecapCardState::Open) => p.red,
                _ => dot_neutral,
            };
            let is_hot = self.spotlight_id.as_ref() == Some(&ev.id);
            let val = eval_recap_curve(ev.position, &peaks);
            let py = (CHART_H - 6.0) - val * usable_h;
            let dot_sz = pro_spring_f32(
                pro_eid(&self.id, format!("pdot-sz-{}", ev.id)),
                if is_hot { 9.0 } else { 6.0 },
                0.20,
                window,
                cx,
            );
            let hit_sz = 18.0_f32;
            let ev_id = ev.id.clone();
            let hov_id = ev.id.clone();
            let focus_c = p.focus;

            dots_layer = dots_layer.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("pdot-{ev_id}")))
                    .role(Role::Button)
                    .aria_label(format!("{} · {}", ev.at, ev.title))
                    .tab_index(0)
                    .absolute()
                    .left(relative(ev.position))
                    .top(px(py - hit_sz * 0.5))
                    .ml(px(-hit_sz * 0.5))
                    .size(px(hit_sz))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(focus_c))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = if *inside {
                            Some(hov_id.clone())
                        } else if this.spotlight_id.as_ref() == Some(&hov_id) {
                            None
                        } else {
                            this.spotlight_id.clone()
                        };
                        this.set_spotlight(next, cx);
                    }))
                    .child(
                        div()
                            .size(px(dot_sz))
                            .rounded_full()
                            .bg(dot_c)
                            .border(px(1.5))
                            .border_color(card_bg),
                    ),
                cx,
                move |this, _, cx| {
                    let next = if this.spotlight_id.as_ref() == Some(&ev_id) {
                        None
                    } else {
                        Some(ev_id.clone())
                    };
                    this.set_spotlight(next, cx);
                },
            ));
        }

        let tip_open = self.spotlight_id.is_some();
        if let Some(sid) = self
            .spotlight_id
            .as_ref()
            .or(self.last_spotlight_id.as_ref())
            && let Some(ev) = self.events.iter().find(|e| &e.id == sid)
        {
            let tip_bg: Hsla = if p.dark {
                rgb(0xfafafa).into()
            } else {
                rgb(0x18181b).into()
            };
            let tip_fg: Hsla = if p.dark {
                rgb(0x111113).into()
            } else {
                rgb(0xfafafa).into()
            };
            let clamped_u = smooth_u.clamp(0.14, 0.86);
            let tip_txt = format!("{} · {}", ev.at, ev.title);
            dots_layer = dots_layer.child(
                div()
                    .absolute()
                    .top(px(4.0))
                    .left(relative(clamped_u))
                    .ml(px(-96.0))
                    .w(px(192.0))
                    .flex()
                    .justify_center()
                    .child(pro_popover_presence(
                        pro_eid(&self.id, "recap-tip-pop"),
                        tip_open,
                        move || {
                            div()
                                .max_w(px(240.0))
                                .px(px(8.0))
                                .py(px(3.0))
                                .rounded(px(6.0))
                                .bg(tip_bg)
                                .shadow_sm()
                                .text_size(px(11.5))
                                .text_color(tip_fg)
                                .truncate()
                                .child(tip_txt)
                                .into_any_element()
                        },
                    )),
            );
        }

        let chart_section = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(CHART_H))
                    .child(chart_canvas)
                    .child(dots_layer),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.5))
                    .text_color(p.faint)
                    .child(self.since_label.clone())
                    .child(self.until_label.clone()),
            );

        // 3-Group Board (`Needs you`, `Failed`, `Done`).
        let width_px = self.measured_w.get().max(280.0);
        let stack_lanes = width_px < 640.0;
        let lane_min_h = if stack_lanes {
            CARD_H
        } else {
            self.limit as f32 * CARD_H + self.limit.saturating_sub(1) as f32 * 4.0
        };

        let lane_specs: [(RecapKind, &'static str, SharedString); 3] = [
            (
                RecapKind::Needs,
                ICON_NEEDS_SVG,
                tr(cx, "Needs you", "待你确认").into(),
            ),
            (
                RecapKind::Failed,
                ICON_FAILED_SVG,
                tr(cx, "Failed", "失败").into(),
            ),
            (
                RecapKind::Done,
                ICON_DONE_SVG,
                tr(cx, "Done", "已完成").into(),
            ),
        ];

        let mut board = div()
            .w_full()
            .mt(px(4.0))
            .flex()
            .when(stack_lanes, |d| d.flex_col())
            .gap(px(10.0));

        for (kind, icon_svg, lane_title) in lane_specs {
            let matching: Vec<RecapEvent> = self
                .events
                .iter()
                .filter(|e| e.kind == kind)
                .cloned()
                .collect();
            let total_in_lane = matching.len();
            let shown_cards: Vec<AnyElement> = matching
                .iter()
                .take(self.limit)
                .map(|ev| self.render_card(ev, cx))
                .collect();

            let cards_area = if shown_cards.is_empty() {
                div()
                    .w_full()
                    .min_h(px(lane_min_h))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(p.faint)
                    .child(tr(cx, "Nothing in this lane", "暂无事项"))
            } else {
                div()
                    .w_full()
                    .min_h(px(lane_min_h))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .children(shown_cards)
            };

            board = board.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .p(px(4.0))
                    .rounded(px(14.0))
                    .bg(lane_bg)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .h(px(32.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(pro_svg("0 0 16 16", icon_svg, 14.0, p.muted))
                                    .child(lane_title),
                            )
                            .child(
                                div()
                                    .h(px(18.0))
                                    .min_w(px(20.0))
                                    .px(px(5.0))
                                    .rounded(px(6.0))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, format!("lane-cnt-{kind:?}")),
                                        total_in_lane as f64,
                                        0,
                                        "",
                                        "",
                                        11.0,
                                        FontWeight::MEDIUM,
                                        p.text,
                                    )),
                            ),
                    )
                    .child(cards_area),
            );
        }

        let measure_cell = self.measured_w.clone();
        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title_str)
            .relative()
            .w_full()
            .min_w_0()
            .pt(px(16.0))
            .px(px(16.0))
            .pb(px(12.0))
            .rounded(px(14.0))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .when(!p.dark, |d| d.shadow_xs())
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 40.0 && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.0)),
            )
            .child(head_row)
            .child(chart_section)
            .child(board)
            .when(!self.announcement.is_empty(), |d| {
                d.child(
                    div()
                        .id(pro_eid(&self.id, "recap-announce"))
                        .role(Role::Status)
                        .aria_label(self.announcement.clone())
                        .size(px(0.0))
                        .overflow_hidden(),
                )
            })
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{RecapEvent, RecapKind, eval_recap_curve};

    #[test]
    fn recap_peak_curve_peaks_at_events_and_decays_away() {
        let ev =
            RecapEvent::new("tickets", RecapKind::Done, 0.35, "7:05 PM", "Support").with_count(38);
        let peaks = [(ev.position, ev.peak_weight())];
        let at_peak = eval_recap_curve(0.35, &peaks);
        let far_away = eval_recap_curve(0.85, &peaks);
        assert!(at_peak > 0.80);
        assert!(far_away < 0.01);
    }
}
