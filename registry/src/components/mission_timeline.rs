//! Shared-clock multi-agent mission timeline block with recap curve, live Gantt swimlanes, and action inbox.
//!
//! Aligns every agent on one rolling time axis: a `"Since you looked"` event-peak curve on top,
//! multi-lane swimlanes with growing `"now"` bars and handoff connectors on the left, and live
//! `"NEEDS YOU"` approval/retry cards on the right under one unified status filter.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw,
        pro_connector_pulse, pro_eid, pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected,
        pro_mix_color, pro_pill_indicator, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const RECAP_NEEDS_SVG: &str = r#"<circle cx="8" cy="5.25" r="2.5" stroke="currentColor" stroke-width="1.4"></circle><path d="M3.25 13.25c.6-2.35 2.45-3.75 4.75-3.75s4.15 1.4 4.75 3.75" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path>"#;
const RECAP_FAIL_SVG: &str = r#"<path d="M7.13 2.75a1 1 0 0 1 1.74 0l5.2 9.25a1 1 0 0 1-.87 1.5H2.8a1 1 0 0 1-.87-1.5z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"></path><path d="M8 6.5v2.75M8 11.25v.01" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path>"#;

/// Unified status filter controlling the recap curve, swimlanes, and inbox in [`MissionTimelineBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineFilter {
    /// Show every agent lane and recap peak.
    #[default]
    All,
    /// Show actively running agents.
    Running,
    /// Show agents waiting on user input plus failed agents needing attention.
    NeedsYou,
    /// Show failed agents only.
    Failed,
}

impl TimelineFilter {
    const ALL: [Self; 4] = [Self::All, Self::Running, Self::NeedsYou, Self::Failed];

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Running => "running",
            Self::NeedsYou => "needs",
            Self::Failed => "failed",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::NeedsYou => tr(cx, "Needs you", "待处理"),
            Self::Failed => tr(cx, "Failed", "失败"),
        }
    }

    fn next(self) -> Self {
        match self {
            Self::All => Self::Running,
            Self::Running => Self::NeedsYou,
            Self::NeedsYou => Self::Failed,
            Self::Failed => Self::All,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::All => Self::Failed,
            Self::Running => Self::All,
            Self::NeedsYou => Self::Running,
            Self::Failed => Self::NeedsYou,
        }
    }
}

/// Operational status of a lane or Gantt task block in [`MissionTimelineBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimelineAgentStatus {
    /// Actively running up to `"now"`.
    #[default]
    Running,
    /// Waiting on user decision or approval.
    Waiting,
    /// Stopped due to an error.
    Failed,
    /// Completed cleanly.
    Done,
}

impl TimelineAgentStatus {
    /// Check whether this status passes `filter` (`NeedsYou` includes both `Waiting` and `Failed`).
    pub fn matches(self, filter: TimelineFilter) -> bool {
        match filter {
            TimelineFilter::All => true,
            TimelineFilter::Running => self == Self::Running,
            TimelineFilter::NeedsYou => matches!(self, Self::Waiting | Self::Failed),
            TimelineFilter::Failed => self == Self::Failed,
        }
    }
}

/// Horizontal task span rendered on an agent's swimlane in [`MissionTimelineBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTaskSpan {
    /// Stable task identifier.
    pub id: SharedString,
    /// Task label displayed inside the bar.
    pub title: SharedString,
    /// Task execution status.
    pub status: TimelineAgentStatus,
    /// Normalized start time in `0.0..=1.0` across the timeline span.
    pub start_frac: f32,
    /// Normalized end time in `0.0..=1.0` (`1.0` reaches `"now"`).
    pub end_frac: f32,
    /// Duration readout (`"11m"` or `"19m"`).
    pub duration_label: SharedString,
    /// Optional connected tool/app names rendered as leading micro-marks.
    pub tools: Vec<SharedString>,
}

/// Cross-lane handoff connector between two [`TimelineAgentLane`] rows.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineHandoff {
    /// Source agent lane id.
    pub from_agent: SharedString,
    /// Normalized handoff origin time (`0.0..=1.0`).
    pub from_frac: f32,
    /// Target agent lane id.
    pub to_agent: SharedString,
    /// Normalized handoff arrival time (`0.0..=1.0`).
    pub to_frac: f32,
    /// Whether the handoff carries an active light pulse.
    pub live: bool,
}

/// Agent swimlane specification in [`MissionTimelineBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineAgentLane {
    /// Stable agent identifier.
    pub id: SharedString,
    /// Agent display name (`"Launch lead"`, `"Researcher"`, `"Writer"`).
    pub name: SharedString,
    /// Model subtitle (`"Opus 5.5"`, `"Sonnet 5.5"`).
    pub model: SharedString,
    /// Current overall status of this agent.
    pub status: TimelineAgentStatus,
    /// Optional inbox headline when `status == Waiting`.
    pub ask_title: Option<SharedString>,
    /// Optional inbox detail when `status == Waiting`.
    pub ask_detail: Option<SharedString>,
    /// Optional multi-choice quick answers (`vec![]` renders `Reject` / `Approve`).
    pub ask_choices: Vec<SharedString>,
    /// Optional error detail when `status == Failed`.
    pub error: Option<SharedString>,
    /// Formatted elapsed label (`"26m"`).
    pub elapsed: SharedString,
    /// Task blocks plotted on this agent's swimlane.
    pub tasks: Vec<TimelineTaskSpan>,
}

/// Kind of milestone plotted on the `"Since you looked"` curve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimelineEventKind {
    /// Completed milestone.
    #[default]
    Done,
    /// Milestone waiting on user action.
    NeedsYou,
    /// Milestone that failed.
    Failed,
}

/// Recap event peak plotted along `"Since you looked"` in [`MissionTimelineBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineRecapEvent {
    /// Stable event identifier.
    pub id: SharedString,
    /// Owning agent lane identifier.
    pub agent_id: SharedString,
    /// Clock label (`"7:39"`).
    pub at_label: SharedString,
    /// Normalized position in `0.0..=1.0`.
    pub position: f32,
    /// Peak height weight in `0.35..=1.0`.
    pub weight: f32,
    /// Event kind (`Done`, `NeedsYou`, `Failed`).
    pub kind: TimelineEventKind,
    /// Tooltip description.
    pub title: SharedString,
}

/// Events emitted by [`MissionTimelineBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionTimelineEvent {
    /// User changed the unified status filter.
    FilterChanged(TimelineFilter),
    /// User clicked a task bar on one of the swimlanes.
    TaskSelected {
        /// Owning agent lane id.
        agent_id: SharedString,
        /// Selected task span id.
        task_id: SharedString,
    },
    /// User approved a waiting agent (`agent_id`).
    Approved(SharedString),
    /// User rejected or dismissed an agent (`agent_id`).
    Rejected(SharedString),
    /// User answered a waiting agent's choice prompt.
    Answered {
        /// Agent identifier.
        agent_id: SharedString,
        /// Chosen option text.
        choice: SharedString,
    },
    /// User retried a failed agent (`agent_id`).
    Retried(SharedString),
    /// User hovered or left a recap peak dot.
    EventHovered(Option<SharedString>),
}

/// Count lanes matching `filter`.
pub fn count_timeline_lanes(lanes: &[TimelineAgentLane], filter: TimelineFilter) -> usize {
    lanes.iter().filter(|l| l.status.matches(filter)).count()
}

fn lane_color(id: &str, p: &ProPalette) -> Hsla {
    match id {
        "launch" => p.blue,
        "research" => p.purple,
        "writer" => p.teal,
        "release" => p.cyan,
        "support" => p.green,
        "billing" => p.red,
        "ops" => p.orange,
        _ => p.blue,
    }
}

fn eval_timeline_curve(u: f32, peaks: &[(f32, f32)]) -> f32 {
    let mut sum = 0.0_f32;
    for &(pos, w) in peaks {
        let d = (u - pos) / 0.048;
        sum += w * (-d * d).exp();
    }
    sum.clamp(0.0, 0.92)
}

struct TimelineRecapAnim {
    weights: Vec<(f32, Slide)>,
    hot_slides: Vec<(bool, Slide)>,
    cursor_x: Slide,
    cursor_alpha: Slide,
    has_cursor: bool,
}

#[derive(IntoElement)]
struct TimelineRecapCanvas {
    state_id: ElementId,
    events: Vec<(f32, f32, TimelineEventKind, bool, bool)>,
    hovered_pos: Option<f32>,
}

impl RenderOnce for TimelineRecapCanvas {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_events = self.events.clone();
        let init_pos = self.hovered_pos.unwrap_or(0.5);
        let st = w.use_keyed_state(self.state_id, cx, |_, _| TimelineRecapAnim {
            weights: init_events
                .iter()
                .map(|&(_, wt, _, vis, _)| {
                    let v = if vis { wt } else { 0.0 };
                    (v, Slide::new(v))
                })
                .collect(),
            hot_slides: init_events
                .iter()
                .map(|&(_, _, _, _, hot)| (hot, Slide::snappy(f32::from(hot))))
                .collect(),
            cursor_x: Slide::snappy(init_pos),
            cursor_alpha: Slide::snappy(if self.hovered_pos.is_some() { 1. } else { 0. }),
            has_cursor: self.hovered_pos.is_some(),
        });

        let (peaks, dots_meta, cur_x, cur_alpha) = st.update(cx, |s, cx| {
            if s.weights.len() != self.events.len() {
                s.weights = self
                    .events
                    .iter()
                    .map(|&(_, wt, _, vis, _)| {
                        let v = if vis { wt } else { 0.0 };
                        (v, Slide::new(v))
                    })
                    .collect();
                s.hot_slides = self
                    .events
                    .iter()
                    .map(|&(_, _, _, _, hot)| (hot, Slide::snappy(f32::from(hot))))
                    .collect();
            }

            let mut cur_peaks = Vec::with_capacity(self.events.len());
            let mut cur_dots = Vec::with_capacity(self.events.len());
            for (i, &(pos, wt, kind, vis, hot)) in self.events.iter().enumerate() {
                let target_w = if vis { wt } else { 0.0 };
                if (s.weights[i].0 - target_w).abs() > 0.001 {
                    s.weights[i].0 = target_w;
                    s.weights[i].1.duration(target_w, 0.26, cx);
                }
                let w_val = s.weights[i].1.get(w, cx).max(0.0);
                if w_val > 0.003 {
                    cur_peaks.push((pos, w_val));
                }

                let target_hot = vis && hot;
                if s.hot_slides[i].0 != target_hot {
                    s.hot_slides[i].0 = target_hot;
                    s.hot_slides[i].1.set(f32::from(target_hot), cx);
                }
                let hot_amt = s.hot_slides[i].1.get(w, cx).clamp(0.0, 1.0);
                if vis || w_val > 0.02 {
                    let vis_alpha = if wt > 0.01 {
                        (w_val / wt).clamp(0.0, 1.0)
                    } else {
                        f32::from(vis)
                    };
                    cur_dots.push((pos, kind, hot_amt, vis_alpha));
                }
            }

            if let Some(hp) = self.hovered_pos {
                if !s.has_cursor {
                    s.cursor_x.reset(hp);
                    s.has_cursor = true;
                } else {
                    s.cursor_x.set(hp, cx);
                }
                s.cursor_alpha.set(1.0, cx);
            } else {
                s.cursor_alpha.set(0.0, cx);
                if s.cursor_alpha.value(cx) < 0.02 {
                    s.has_cursor = false;
                }
            }

            (
                cur_peaks,
                cur_dots,
                s.cursor_x.get(w, cx).clamp(0.0, 1.0),
                s.cursor_alpha.get(w, cx).clamp(0.0, 1.0),
            )
        });

        let stroke_col = p.ink.opacity(0.46);
        let fill_col = p.ink.opacity(0.08);
        let grid_col = p.line_soft;
        let card_bg = p.card;
        let done_dot = p.green;
        let needs_dot = p.orange;
        let fail_dot = p.red;
        let cursor_col = p.line_strong;

        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8. || bh <= 8. {
                    return;
                }
                let top_pad = 8.0_f32;
                let base_y = bh - 4.0_f32;
                let usable_h = (base_y - top_pad).max(12.0);

                for r in 0..=2usize {
                    let gy = bounds.top() + px((r as f32 / 2.0) * bh);
                    let mut gp = PathBuilder::stroke(px(1.));
                    gp.move_to(point(bounds.left(), gy));
                    gp.line_to(point(bounds.right(), gy));
                    if let Ok(path) = gp.build() {
                        window.paint_path(path, grid_col);
                    }
                }
                for c in 1..8usize {
                    let gx = bounds.left() + px((c as f32 / 8.0) * bw);
                    let mut vp = PathBuilder::stroke(px(1.));
                    vp.move_to(point(gx, bounds.top()));
                    vp.line_to(point(gx, bounds.bottom()));
                    if let Ok(path) = vp.build() {
                        window.paint_path(path, grid_col.opacity(grid_col.a * 0.65));
                    }
                }

                let samples = 112usize;
                let pt_at = |u: f32| -> Point<Pixels> {
                    let y_norm = eval_timeline_curve(u, &peaks);
                    point(
                        bounds.left() + px(u * bw),
                        bounds.top() + px(base_y - y_norm * usable_h),
                    )
                };

                if !peaks.is_empty() {
                    let mut area = PathBuilder::fill();
                    let floor = bounds.top() + px(base_y);
                    area.move_to(point(bounds.left(), floor));
                    for i in 0..=samples {
                        area.line_to(pt_at(i as f32 / samples as f32));
                    }
                    area.line_to(point(bounds.right(), floor));
                    area.close();
                    if let Ok(path) = area.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(fill_col, 0.),
                                linear_color_stop(fill_col.opacity(0.0), 1.),
                            ),
                        );
                    }
                }

                let mut stroke = PathBuilder::stroke(px(1.5));
                for i in 0..=samples {
                    let pt = pt_at(i as f32 / samples as f32);
                    if i == 0 {
                        stroke.move_to(pt);
                    } else {
                        stroke.line_to(pt);
                    }
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, stroke_col);
                }

                if cur_alpha > 0.01 {
                    let cx_pt = bounds.left() + px(cur_x * bw);
                    let mut cur = PathBuilder::stroke(px(1.));
                    cur.move_to(point(cx_pt, bounds.top()));
                    cur.line_to(point(cx_pt, bounds.bottom()));
                    if let Ok(path) = cur.build() {
                        window.paint_path(path, cursor_col.opacity(cursor_col.a * cur_alpha));
                    }
                }

                for &(pos, kind, hot_amt, vis_alpha) in &dots_meta {
                    let pt = pt_at(pos);
                    let dot_c = match kind {
                        TimelineEventKind::Done => done_dot,
                        TimelineEventKind::NeedsYou => needs_dot,
                        TimelineEventKind::Failed => fail_dot,
                    };
                    let ri = px((2.8 + 1.6 * hot_amt) * vis_alpha.max(0.35));
                    let ro = ri + px(1.6);
                    window.paint_quad(
                        fill(
                            Bounds::new(pt - point(ro, ro), size(ro * 2., ro * 2.)),
                            card_bg.opacity(vis_alpha),
                        )
                        .corner_radii(ro),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(pt - point(ri, ri), size(ri * 2., ri * 2.)),
                            dot_c.opacity(dot_c.a * vis_alpha),
                        )
                        .corner_radii(ri),
                    );
                }
            },
        )
        .w_full()
        .h(px(76.))
    }
}

struct TimelineTooltipState {
    x_frac: Slide,
    alpha: Slide,
    was_open: bool,
    last: Option<(SharedString, SharedString, SharedString, TimelineEventKind)>,
    width_cell: Rc<Cell<f32>>,
}

#[derive(IntoElement)]
struct TimelineRecapTooltip {
    state_id: ElementId,
    active: Option<(
        f32,
        SharedString,
        SharedString,
        SharedString,
        TimelineEventKind,
    )>,
}

impl RenderOnce for TimelineRecapTooltip {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_pos = self.active.as_ref().map(|a| a.0).unwrap_or(0.5);
        let init_open = self.active.is_some();
        let init_last = self
            .active
            .as_ref()
            .map(|(_, id, at, title, kind)| (id.clone(), at.clone(), title.clone(), *kind));
        let st = w.use_keyed_state(self.state_id.clone(), cx, |_, _| TimelineTooltipState {
            x_frac: Slide::snappy(init_pos),
            alpha: Slide::snappy(if init_open { 1. } else { 0. }),
            was_open: init_open,
            last: init_last,
            width_cell: Rc::new(Cell::new(560.)),
        });

        let (x_frac, alpha, shown, width_cell) = st.update(cx, |s, cx| {
            if let Some((pos, id, at, title, kind)) = &self.active {
                if !s.was_open {
                    s.x_frac.reset(*pos);
                    s.was_open = true;
                } else {
                    s.x_frac.set(*pos, cx);
                }
                s.alpha.set(1.0, cx);
                s.last = Some((id.clone(), at.clone(), title.clone(), *kind));
            } else {
                s.alpha.set(0.0, cx);
                if s.alpha.value(cx) < 0.02 {
                    s.was_open = false;
                }
            }
            (
                s.x_frac.get(w, cx).clamp(0.0, 1.0),
                s.alpha.get(w, cx).clamp(0.0, 1.0),
                s.last.clone(),
                s.width_cell.clone(),
            )
        });

        if alpha <= 0.01 {
            return div().into_any_element();
        }
        let Some((ev_id, at_label, title, kind)) = shown else {
            return div().into_any_element();
        };

        let bw = width_cell.get().max(260.);
        let pill_w = 264.0_f32.min(bw - 16.);
        let left_px = (x_frac * bw - pill_w * 0.5).clamp(8.0, (bw - pill_w - 8.0).max(8.0));

        let tip_bg: Hsla = if p.dark {
            rgb(0xf4f4f5).into()
        } else {
            rgb(0x18181b).into()
        };
        let tip_fg: Hsla = if p.dark {
            rgb(0x111113).into()
        } else {
            rgb(0xfafafa).into()
        };

        div()
            .absolute()
            .inset_0()
            .child(
                canvas(
                    move |b, _, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 20. {
                            width_cell.set(nw);
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(4. + 3. * (1. - alpha)))
                    .left(px(left_px))
                    .max_w(px(pill_w))
                    .px(px(8.))
                    .py(px(3.5))
                    .rounded(px(6.))
                    .bg(tip_bg)
                    .opacity(alpha)
                    .text_color(tip_fg)
                    .text_size(px(11.5))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(if kind == TimelineEventKind::Done {
                        pro_check_draw(
                            pro_eid(&self.state_id, format!("tip-chk-{ev_id}")),
                            true,
                            12.,
                            false,
                            p.green,
                        )
                    } else {
                        div()
                            .size(px(6.))
                            .rounded_full()
                            .bg(if kind == TimelineEventKind::Failed {
                                p.red
                            } else {
                                p.orange
                            })
                            .into_any_element()
                    })
                    .child(div().opacity(0.68).child(at_label))
                    .child(div().min_w_0().truncate().child(title)),
            )
            .into_any_element()
    }
}

fn default_lanes() -> Vec<TimelineAgentLane> {
    // 7 lanes matching All 7 | Running 3 | Needs you 3 | Failed 1
    vec![
        TimelineAgentLane {
            id: "launch".into(),
            name: "Launch lead".into(),
            model: "Opus 5.5".into(),
            status: TimelineAgentStatus::Running,
            ask_title: None,
            ask_detail: None,
            ask_choices: Vec::new(),
            error: None,
            elapsed: "26m".into(),
            tasks: vec![
                TimelineTaskSpan {
                    id: "t-plan".into(),
                    title: "Plan launch".into(),
                    status: TimelineAgentStatus::Done,
                    start_frac: 0.02,
                    end_frac: 0.25,
                    duration_label: "7m".into(),
                    tools: vec!["Linear".into()],
                },
                TimelineTaskSpan {
                    id: "t-coord".into(),
                    title: "Coordinate the team".into(),
                    status: TimelineAgentStatus::Running,
                    start_frac: 0.28,
                    end_frac: 1.0,
                    duration_label: "19m".into(),
                    tools: vec!["Notion".into()],
                },
            ],
        },
        TimelineAgentLane {
            id: "research".into(),
            name: "Researcher".into(),
            model: "Sonnet 5.5".into(),
            status: TimelineAgentStatus::Done,
            ask_title: None,
            ask_detail: None,
            ask_choices: Vec::new(),
            error: None,
            elapsed: "19m".into(),
            tasks: vec![
                TimelineTaskSpan {
                    id: "t-comp".into(),
                    title: "Competitor pricing".into(),
                    status: TimelineAgentStatus::Done,
                    start_frac: 0.10,
                    end_frac: 0.46,
                    duration_label: "11m".into(),
                    tools: vec!["Notion".into(), "Figma".into()],
                },
                TimelineTaskSpan {
                    id: "t-stripe-check".into(),
                    title: "Verify Stripe IDs".into(),
                    status: TimelineAgentStatus::Done,
                    start_frac: 0.50,
                    end_frac: 0.78,
                    duration_label: "8m".into(),
                    tools: vec!["Stripe".into()],
                },
            ],
        },
        TimelineAgentLane {
            id: "writer".into(),
            name: "Writer".into(),
            model: "Opus 5.5".into(),
            status: TimelineAgentStatus::Waiting,
            ask_title: Some("Pick the Team plan seat price".into()),
            ask_detail: Some("Free ($0) and Pro ($12) copy ready · confirm Team tier price".into()),
            ask_choices: vec!["$20 / seat".into(), "$24 / seat".into()],
            error: None,
            elapsed: "14m".into(),
            tasks: vec![
                TimelineTaskSpan {
                    id: "t-draft".into(),
                    title: "Draft plan copy".into(),
                    status: TimelineAgentStatus::Done,
                    start_frac: 0.14,
                    end_frac: 0.54,
                    duration_label: "12m".into(),
                    tools: vec!["Notion".into()],
                },
                TimelineTaskSpan {
                    id: "t-team-price".into(),
                    title: "Team seat price?".into(),
                    status: TimelineAgentStatus::Waiting,
                    start_frac: 0.57,
                    end_frac: 1.0,
                    duration_label: "14m".into(),
                    tools: Vec::new(),
                },
            ],
        },
        TimelineAgentLane {
            id: "release".into(),
            name: "Release".into(),
            model: "Sonnet 5.5".into(),
            status: TimelineAgentStatus::Waiting,
            ask_title: Some("Deploy pricing page PR #482".into()),
            ask_detail: Some("All 212 staging checks green · waiting on prod approval".into()),
            ask_choices: Vec::new(),
            error: None,
            elapsed: "4m".into(),
            tasks: vec![
                TimelineTaskSpan {
                    id: "t-stage".into(),
                    title: "Stage PR #482".into(),
                    status: TimelineAgentStatus::Done,
                    start_frac: 0.42,
                    end_frac: 0.68,
                    duration_label: "8m".into(),
                    tools: vec!["GitHub".into()],
                },
                TimelineTaskSpan {
                    id: "t-deploy".into(),
                    title: "Approve deploy".into(),
                    status: TimelineAgentStatus::Waiting,
                    start_frac: 0.71,
                    end_frac: 1.0,
                    duration_label: "4m".into(),
                    tools: vec!["Vercel".into()],
                },
            ],
        },
        TimelineAgentLane {
            id: "support".into(),
            name: "Support lead".into(),
            model: "Sonnet 5.5".into(),
            status: TimelineAgentStatus::Running,
            ask_title: None,
            ask_detail: None,
            ask_choices: Vec::new(),
            error: None,
            elapsed: "29m".into(),
            tasks: vec![TimelineTaskSpan {
                id: "t-tickets".into(),
                title: "Answer 38 open tickets".into(),
                status: TimelineAgentStatus::Running,
                start_frac: 0.05,
                end_frac: 1.0,
                duration_label: "29m".into(),
                tools: vec!["Slack".into()],
            }],
        },
        TimelineAgentLane {
            id: "billing".into(),
            name: "Billing".into(),
            model: "Haiku 4.5".into(),
            status: TimelineAgentStatus::Failed,
            ask_title: None,
            ask_detail: None,
            ask_choices: Vec::new(),
            error: Some("Stripe live refund token expired (HTTP 401)".into()),
            elapsed: "3m".into(),
            tasks: vec![TimelineTaskSpan {
                id: "t-refund".into(),
                title: "Refund $240 Acme".into(),
                status: TimelineAgentStatus::Failed,
                start_frac: 0.45,
                end_frac: 0.74,
                duration_label: "3m".into(),
                tools: vec!["Stripe".into()],
            }],
        },
        TimelineAgentLane {
            id: "ops".into(),
            name: "Ops".into(),
            model: "GPT-5.4".into(),
            status: TimelineAgentStatus::Running,
            ask_title: None,
            ask_detail: None,
            ask_choices: Vec::new(),
            error: None,
            elapsed: "18m".into(),
            tasks: vec![
                TimelineTaskSpan {
                    id: "t-crm".into(),
                    title: "Sync CRM".into(),
                    status: TimelineAgentStatus::Done,
                    start_frac: 0.16,
                    end_frac: 0.48,
                    duration_label: "9m".into(),
                    tools: vec!["HubSpot".into()],
                },
                TimelineTaskSpan {
                    id: "t-keys".into(),
                    title: "Rotate keys".into(),
                    status: TimelineAgentStatus::Running,
                    start_frac: 0.52,
                    end_frac: 1.0,
                    duration_label: "14m".into(),
                    tools: vec!["GitHub".into()],
                },
            ],
        },
    ]
}

fn default_handoffs() -> Vec<TimelineHandoff> {
    vec![
        TimelineHandoff {
            from_agent: "launch".into(),
            from_frac: 0.10,
            to_agent: "research".into(),
            to_frac: 0.10,
            live: false,
        },
        TimelineHandoff {
            from_agent: "launch".into(),
            from_frac: 0.14,
            to_agent: "writer".into(),
            to_frac: 0.14,
            live: true,
        },
        TimelineHandoff {
            from_agent: "research".into(),
            from_frac: 0.46,
            to_agent: "release".into(),
            to_frac: 0.42,
            live: true,
        },
        TimelineHandoff {
            from_agent: "support".into(),
            from_frac: 0.44,
            to_agent: "billing".into(),
            to_frac: 0.45,
            live: false,
        },
    ]
}

fn default_events() -> Vec<TimelineRecapEvent> {
    vec![
        TimelineRecapEvent {
            id: "ev-1".into(),
            agent_id: "research".into(),
            at_label: "7:12".into(),
            position: 0.10,
            weight: 0.62,
            kind: TimelineEventKind::Done,
            title: "Compared seat prices across Linear, Notion & Figma".into(),
        },
        TimelineRecapEvent {
            id: "ev-2".into(),
            agent_id: "ops".into(),
            at_label: "7:18".into(),
            position: 0.22,
            weight: 0.46,
            kind: TimelineEventKind::Done,
            title: "Synced 1,240 CRM contacts to HubSpot".into(),
        },
        TimelineRecapEvent {
            id: "ev-3".into(),
            agent_id: "support".into(),
            at_label: "7:29".into(),
            position: 0.46,
            weight: 0.84,
            kind: TimelineEventKind::Done,
            title: "Answered 24 support tickets in #help".into(),
        },
        TimelineRecapEvent {
            id: "ev-4".into(),
            agent_id: "billing".into(),
            at_label: "7:36".into(),
            position: 0.61,
            weight: 0.56,
            kind: TimelineEventKind::Failed,
            title: "Refund $240 to Acme Corp failed (token expired)".into(),
        },
        TimelineRecapEvent {
            id: "ev-5".into(),
            agent_id: "writer".into(),
            at_label: "7:40".into(),
            position: 0.70,
            weight: 0.58,
            kind: TimelineEventKind::NeedsYou,
            title: "Writer asks: Team plan $20 or $24 per seat?".into(),
        },
        TimelineRecapEvent {
            id: "ev-6".into(),
            agent_id: "release".into(),
            at_label: "7:45".into(),
            position: 0.79,
            weight: 0.64,
            kind: TimelineEventKind::NeedsYou,
            title: "Release asks: Approve PR #482 deploy".into(),
        },
        TimelineRecapEvent {
            id: "ev-7".into(),
            agent_id: "launch".into(),
            at_label: "7:49".into(),
            position: 0.88,
            weight: 0.44,
            kind: TimelineEventKind::Done,
            title: "Verified Stripe webhook signatures on staging".into(),
        },
    ]
}

/// Shared-clock multi-agent timeline block pairing an event-peak recap curve, Gantt swimlanes, and action inbox.
pub struct MissionTimelineBlock {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    since_label: SharedString,
    now_label: SharedString,
    span_minutes: u32,
    filter: TimelineFilter,
    lanes: Vec<TimelineAgentLane>,
    handoffs: Vec<TimelineHandoff>,
    events: Vec<TimelineRecapEvent>,
    selected_task: Option<(SharedString, SharedString)>,
    hovered_event: Option<SharedString>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    boot_at: Instant,
}

impl EventEmitter<MissionTimelineEvent> for MissionTimelineBlock {}

impl MissionTimelineBlock {
    /// Create a new [`MissionTimelineBlock`] with 7 synchronized agent swimlanes on a 30-minute clock.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Mission timeline".into(),
            since_label: "7:08".into(),
            now_label: "7:53".into(),
            span_minutes: 30,
            filter: TimelineFilter::All,
            lanes: default_lanes(),
            handoffs: default_handoffs(),
            events: default_events(),
            selected_task: None,
            hovered_event: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(894.)),
            toast: None,
            boot_at: Instant::now(),
        }
    }

    /// Set the top-left heading (default `"Mission timeline"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the initial [`TimelineFilter`].
    pub fn with_filter(mut self, filter: TimelineFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Alias matching `defaultFilter` in the reference specification.
    pub fn with_default_filter(self, filter: TimelineFilter) -> Self {
        self.with_filter(filter)
    }

    /// Set the swimlane time window in minutes (default `30`).
    pub fn with_span_minutes(mut self, minutes: u32) -> Self {
        self.span_minutes = minutes.max(5);
        self
    }

    /// Supply custom [`TimelineAgentLane`] swimlanes.
    pub fn with_lanes(mut self, lanes: Vec<TimelineAgentLane>) -> Self {
        self.lanes = lanes;
        self
    }

    /// Supply custom cross-lane [`TimelineHandoff`] links.
    pub fn with_handoffs(mut self, handoffs: Vec<TimelineHandoff>) -> Self {
        self.handoffs = handoffs;
        self
    }

    /// Supply custom `"Since you looked"` [`TimelineRecapEvent`] peaks.
    pub fn with_events(mut self, events: Vec<TimelineRecapEvent>) -> Self {
        self.events = events;
        self
    }

    /// Customize the start and end clock labels (`"7:08"` … `"7:53"`).
    pub fn with_window_labels(
        mut self,
        since_label: impl Into<SharedString>,
        now_label: impl Into<SharedString>,
    ) -> Self {
        self.since_label = since_label.into();
        self.now_label = now_label.into();
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the active [`TimelineFilter`].
    pub fn filter(&self) -> TimelineFilter {
        self.filter
    }

    /// Switch the unified [`TimelineFilter`] and emit [`MissionTimelineEvent::FilterChanged`].
    pub fn set_filter(&mut self, filter: TimelineFilter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            cx.emit(MissionTimelineEvent::FilterChanged(filter));
            cx.notify();
        }
    }

    fn resolve_approve(
        &mut self,
        agent_id: &str,
        choice: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if let Some(lane) = self.lanes.iter_mut().find(|l| l.id.as_ref() == agent_id) {
            lane.status = TimelineAgentStatus::Running;
            lane.ask_title = None;
            lane.ask_detail = None;
            lane.ask_choices.clear();
            for t in &mut lane.tasks {
                if t.status == TimelineAgentStatus::Waiting {
                    t.status = TimelineAgentStatus::Running;
                }
            }
            for ev in &mut self.events {
                if ev.agent_id.as_ref() == agent_id && ev.kind == TimelineEventKind::NeedsYou {
                    ev.kind = TimelineEventKind::Done;
                }
            }
            let id_s = SharedString::from(agent_id.to_owned());
            if let Some(ch) = choice {
                self.toast = Some((
                    format!("{}: {ch}", tr(cx, "Answered", "已选择回复")).into(),
                    Instant::now(),
                ));
                cx.emit(MissionTimelineEvent::Answered {
                    agent_id: id_s,
                    choice: ch,
                });
            } else {
                self.toast = Some((
                    tr(cx, "Approved timeline step", "已批准时间轴步骤").into(),
                    Instant::now(),
                ));
                cx.emit(MissionTimelineEvent::Approved(id_s));
            }
            cx.notify();
        }
    }

    fn resolve_reject(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        if let Some(lane) = self.lanes.iter_mut().find(|l| l.id.as_ref() == agent_id) {
            lane.status = TimelineAgentStatus::Done;
            lane.ask_title = None;
            lane.ask_detail = None;
            lane.error = None;
            for t in &mut lane.tasks {
                if matches!(
                    t.status,
                    TimelineAgentStatus::Waiting | TimelineAgentStatus::Failed
                ) {
                    t.status = TimelineAgentStatus::Done;
                }
            }
            for ev in &mut self.events {
                if ev.agent_id.as_ref() == agent_id {
                    ev.kind = TimelineEventKind::Done;
                }
            }
            self.toast = Some((
                tr(cx, "Closed pending item", "已关闭待处理事项").into(),
                Instant::now(),
            ));
            cx.emit(MissionTimelineEvent::Rejected(SharedString::from(
                agent_id.to_owned(),
            )));
            cx.notify();
        }
    }

    fn resolve_retry(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        if let Some(lane) = self.lanes.iter_mut().find(|l| l.id.as_ref() == agent_id) {
            lane.status = TimelineAgentStatus::Running;
            lane.error = None;
            for t in &mut lane.tasks {
                if t.status == TimelineAgentStatus::Failed {
                    t.status = TimelineAgentStatus::Running;
                    t.end_frac = 1.0;
                }
            }
            for ev in &mut self.events {
                if ev.agent_id.as_ref() == agent_id && ev.kind == TimelineEventKind::Failed {
                    ev.kind = TimelineEventKind::Done;
                }
            }
            self.toast = Some((
                tr(
                    cx,
                    "Retrying failed task on timeline",
                    "正在时间轴上重试任务",
                )
                .into(),
                Instant::now(),
            ));
            cx.emit(MissionTimelineEvent::Retried(SharedString::from(
                agent_id.to_owned(),
            )));
            cx.notify();
        }
    }

    fn highlighted_agent(&self) -> Option<&SharedString> {
        let hid = self.hovered_event.as_ref()?;
        self.events
            .iter()
            .find(|e| &e.id == hid)
            .map(|e| &e.agent_id)
    }

    fn visible_events(&self) -> Vec<&TimelineRecapEvent> {
        self.events
            .iter()
            .filter(|ev| match self.filter {
                TimelineFilter::All => true,
                other => self
                    .lanes
                    .iter()
                    .find(|l| l.id == ev.agent_id)
                    .is_some_and(|l| l.status.matches(other)),
            })
            .collect()
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control {
            return;
        }
        match ev.keystroke.key.as_str() {
            "left" => {
                self.set_filter(self.filter.prev(), cx);
                cx.stop_propagation();
            }
            "right" => {
                self.set_filter(self.filter.next(), cx);
                cx.stop_propagation();
            }
            "home" => {
                self.set_filter(TimelineFilter::All, cx);
                cx.stop_propagation();
            }
            "end" => {
                self.set_filter(TimelineFilter::Failed, cx);
                cx.stop_propagation();
            }
            "escape"
                if self.selected_task.take().is_some() || self.hovered_event.take().is_some() =>
            {
                cx.stop_propagation();
                cx.notify();
            }
            _ => {}
        }
    }

    fn toggle_recap_milestone(&mut self, event_id: &str, cx: &mut Context<Self>) {
        let Some(idx) = self.events.iter().position(|e| e.id.as_ref() == event_id) else {
            return;
        };
        let agent_id = self.events[idx].agent_id.clone();
        match self.events[idx].kind {
            TimelineEventKind::NeedsYou => {
                let first_choice = self
                    .lanes
                    .iter()
                    .find(|l| l.id == agent_id)
                    .and_then(|l| l.ask_choices.first().cloned());
                self.resolve_approve(&agent_id, first_choice, cx);
            }
            TimelineEventKind::Failed => {
                self.resolve_retry(&agent_id, cx);
            }
            TimelineEventKind::Done => {
                self.events[idx].kind = TimelineEventKind::NeedsYou;
                if let Some(lane) = self.lanes.iter().find(|l| l.id == agent_id)
                    && let Some(task) = lane.tasks.last()
                {
                    let tid = task.id.clone();
                    self.selected_task = Some((agent_id.clone(), tid.clone()));
                    cx.emit(MissionTimelineEvent::TaskSelected {
                        agent_id,
                        task_id: tid,
                    });
                }
                cx.notify();
            }
        }
    }

    fn toggle_task_done(&mut self, agent_id: &str, task_id: &str, cx: &mut Context<Self>) {
        if let Some(lane) = self.lanes.iter_mut().find(|l| l.id.as_ref() == agent_id)
            && let Some(task) = lane.tasks.iter_mut().find(|t| t.id.as_ref() == task_id)
        {
            task.status = if task.status == TimelineAgentStatus::Done {
                TimelineAgentStatus::Running
            } else {
                TimelineAgentStatus::Done
            };
            cx.notify();
        }
    }

    fn render_recap_section(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let vis = self.visible_events();
        let done_n = vis
            .iter()
            .filter(|e| e.kind == TimelineEventKind::Done)
            .count();
        let needs_n = vis
            .iter()
            .filter(|e| e.kind == TimelineEventKind::NeedsYou)
            .count();
        let fail_n = vis
            .iter()
            .filter(|e| e.kind == TimelineEventKind::Failed)
            .count();

        let all_events_meta: Vec<(f32, f32, TimelineEventKind, bool, bool)> = self
            .events
            .iter()
            .map(|ev| {
                let is_vis = vis.iter().any(|v| v.id == ev.id);
                let hot = is_vis && self.hovered_event.as_ref() == Some(&ev.id);
                (ev.position, ev.weight, ev.kind, is_vis, hot)
            })
            .collect();
        let hovered_info = self
            .hovered_event
            .as_ref()
            .and_then(|hid| vis.iter().copied().find(|e| &e.id == hid));

        let curve_canvas = TimelineRecapCanvas {
            state_id: pro_eid(&self.id, "mt-rcp-canvas"),
            events: all_events_meta,
            hovered_pos: hovered_info.map(|e| e.position),
        };

        let mut hit_layer = div().absolute().inset_0();
        for ev in &vis {
            let eid = ev.id.clone();
            let eid_click = ev.id.clone();
            let left_frac = (ev.position - 0.045).clamp(0.0, 0.91);
            hit_layer = hit_layer.child(
                div()
                    .id(pro_eid(&self.id, format!("mt-rcp-hit-{eid}")))
                    .role(Role::Button)
                    .aria_label(format!("{} {}", ev.at_label, ev.title))
                    .tab_index(0)
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(left_frac))
                    .w(relative(0.09))
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_event = Some(eid.clone());
                            cx.emit(MissionTimelineEvent::EventHovered(Some(eid.clone())));
                        } else if this.hovered_event.as_ref() == Some(&eid) {
                            this.hovered_event = None;
                            cx.emit(MissionTimelineEvent::EventHovered(None));
                        }
                        cx.notify();
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.toggle_recap_milestone(&eid_click, cx);
                    })),
            );
        }

        let tip_overlay = TimelineRecapTooltip {
            state_id: pro_eid(&self.id, "mt-rcp-tip"),
            active: hovered_info.map(|ev| {
                (
                    ev.position,
                    ev.id.clone(),
                    ev.at_label.clone(),
                    ev.title.clone(),
                    ev.kind,
                )
            }),
        };

        div()
            .w_full()
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .px(px(16.))
            .pt(px(14.))
            .pb(px(10.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Since you looked", "自上次查看以来")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!(
                                        "{} {}",
                                        tr(cx, "Since", "始于"),
                                        self.since_label
                                    ))
                                    .child(
                                        div()
                                            .h(px(18.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .bg(p.chip)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.text)
                                            .child("45m"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_check_draw(
                                        pro_eid(&self.id, "mt-rcp-done-chk"),
                                        done_n > 0,
                                        14.,
                                        false,
                                        p.green,
                                    ))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "mt-rcp-done-num"),
                                        done_n as f64,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(tr(cx, "done", "已完成")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_svg("0 0 16 16", RECAP_NEEDS_SVG, 13., p.orange))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "mt-rcp-needs-num"),
                                        needs_n as f64,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(tr(cx, "need you", "待你确认")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_svg("0 0 16 16", RECAP_FAIL_SVG, 13., p.red))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "mt-rcp-fail-num"),
                                        fail_n as f64,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(tr(cx, "failed", "失败")),
                            ),
                    ),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(76.))
                    .child(curve_canvas)
                    .child(tip_overlay)
                    .child(hit_layer),
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
                    .child(self.now_label.clone()),
            )
    }

    fn render_lanes_card(
        &self,
        avatar_only: bool,
        elapsed_ms: f32,
        pulse_t: f32,
        now_grow: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let highlighted = self.highlighted_agent().cloned();
        let running_n = count_timeline_lanes(&self.lanes, TimelineFilter::Running);
        let needs_n = count_timeline_lanes(&self.lanes, TimelineFilter::NeedsYou);

        let total_tasks: usize = self.lanes.iter().map(|l| l.tasks.len()).sum();
        let completed_tasks: usize = self
            .lanes
            .iter()
            .flat_map(|l| &l.tasks)
            .filter(|t| t.status == TimelineAgentStatus::Done)
            .count();
        let done_ratio = if total_tasks > 0 {
            completed_tasks as f32 / total_tasks as f32
        } else {
            0.0
        };

        let visible_lanes: Vec<(usize, &TimelineAgentLane)> = self
            .lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| l.status.matches(self.filter))
            .collect();

        let row_h = 44.0_f32;
        let name_col_w = if avatar_only { 46.0 } else { 154.0 };
        let step_m = (self.span_minutes / 6).max(1);
        let tick_labels: [String; 5] = [
            format!("−{}m", step_m * 5),
            format!("−{}m", step_m * 4),
            format!("−{}m", step_m * 3),
            format!("−{}m", step_m * 2),
            format!("−{}m", step_m),
        ];

        let axis_header = div()
            .w_full()
            .pt(px(8.))
            .pb(px(4.))
            .pr(px(12.))
            .flex()
            .items_center()
            .child(div().w(px(name_col_w)).flex_none())
            .child(
                div()
                    .relative()
                    .flex_1()
                    .h(px(18.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.5))
                    .text_color(p.faint)
                    .child(tick_labels[0].clone())
                    .child(tick_labels[1].clone())
                    .child(tick_labels[2].clone())
                    .child(tick_labels[3].clone())
                    .child(tick_labels[4].clone())
                    .child(
                        div()
                            .h(px(16.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.blue)
                            .child(tr(cx, "now", "现在")),
                    ),
            );

        let lanes_body = if visible_lanes.is_empty() {
            div()
                .w_full()
                .py(px(32.))
                .text_center()
                .text_size(px(12.5))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "No agent lanes match this filter.",
                    "当前筛选下暂无智能体泳道。",
                ))
        } else {
            let vis_ids: Vec<SharedString> =
                visible_lanes.iter().map(|(_, l)| l.id.clone()).collect();
            let active_handoffs: Vec<(usize, f32, usize, f32, bool)> = self
                .handoffs
                .iter()
                .filter_map(|h| {
                    let r0 = vis_ids.iter().position(|id| id == &h.from_agent)?;
                    let r1 = vis_ids.iter().position(|id| id == &h.to_agent)?;
                    Some((r0, h.from_frac, r1, h.to_frac, h.live))
                })
                .collect();

            let row_count = visible_lanes.len();
            let total_track_h = row_count as f32 * row_h;

            // Left fixed column of agent avatars + names
            let mut left_col = div().w(px(name_col_w)).flex_none().flex().flex_col();
            for (_, lane) in &visible_lanes {
                let faded = highlighted.as_ref().is_some_and(|hid| hid != &lane.id);
                let lane_op = pro_spring_f32(
                    pro_eid(&self.id, format!("mt-lcol-op-{}", lane.id)),
                    if faded { 0.38 } else { 1.0 },
                    0.20,
                    w,
                    cx,
                );
                let col = lane_color(&lane.id, &p);
                let show_ask_pill = lane.status == TimelineAgentStatus::Waiting && !avatar_only;
                let show_fail_pill = lane.status == TimelineAgentStatus::Failed && !avatar_only;

                left_col = left_col.child(
                    div()
                        .h(px(row_h))
                        .pl(px(12.))
                        .pr(px(8.))
                        .border_t_1()
                        .border_color(p.line_soft)
                        .opacity(lane_op)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_agent_avatar(
                            &lane.id,
                            col,
                            22.,
                            lane.status == TimelineAgentStatus::Running,
                            elapsed_ms,
                            cx,
                        ))
                        .when(!avatar_only, |d| {
                            d.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .truncate()
                                                    .child(lane.name.clone()),
                                            )
                                            .when(show_ask_pill, |r| {
                                                r.child(
                                                    div()
                                                        .px(px(4.))
                                                        .rounded(px(4.))
                                                        .bg(p.amber_soft)
                                                        .text_size(px(10.5))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.amber_ink)
                                                        .child("Ask"),
                                                )
                                            })
                                            .when(show_fail_pill, |r| {
                                                r.child(
                                                    div()
                                                        .px(px(4.))
                                                        .rounded(px(4.))
                                                        .bg(p.red_soft)
                                                        .text_size(px(10.5))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.red_ink)
                                                        .child("!"),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(lane.model.clone()),
                                    ),
                            )
                        }),
                );
            }

            // Right shared-axis Gantt track: background grid + handoff curves + interactive task bars
            let grid_line = p.line_soft;
            let now_edge_col = p.blue.opacity(0.32);
            let handoff_base = p.line_strong;
            let handoff_pulse = p.blue;
            let grid_and_handoffs = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let bw = f32::from(bounds.size.width);
                    let bh = f32::from(bounds.size.height);
                    if bw <= 8. || bh <= 8. {
                        return;
                    }
                    // Horizontal lane dividers
                    for r in 0..row_count {
                        let y = bounds.top() + px(r as f32 * row_h);
                        let mut hp = PathBuilder::stroke(px(1.));
                        hp.move_to(point(bounds.left(), y));
                        hp.line_to(point(bounds.right(), y));
                        if let Ok(path) = hp.build() {
                            window.paint_path(path, grid_line);
                        }
                    }
                    // Vertical dashed time ticks
                    for c in 1..6usize {
                        let x = bounds.left() + px((c as f32 / 6.0) * bw);
                        let mut y = 0.0_f32;
                        let mut vp = PathBuilder::stroke(px(1.));
                        while y < bh {
                            let y2 = (y + 3.0).min(bh);
                            vp.move_to(point(x, bounds.top() + px(y)));
                            vp.line_to(point(x, bounds.top() + px(y2)));
                            y += 6.0;
                        }
                        if let Ok(path) = vp.build() {
                            window.paint_path(path, grid_line);
                        }
                    }
                    // Right "now" frontier hairline
                    let now_x = bounds.right() - px(1.);
                    let mut np = PathBuilder::stroke(px(1.));
                    np.move_to(point(now_x, bounds.top()));
                    np.line_to(point(now_x, bounds.bottom()));
                    if let Ok(path) = np.build() {
                        window.paint_path(path, now_edge_col);
                    }

                    // Cross-lane handoff connectors via pro_connector_pulse
                    for (idx, &(r0, f0, r1, f1, live)) in active_handoffs.iter().enumerate() {
                        let p0 = point(
                            bounds.left() + px(f0 * bw),
                            bounds.top() + px((r0 as f32 + 0.5) * row_h),
                        );
                        let p1 = point(
                            bounds.left() + px(f1 * bw),
                            bounds.top() + px((r1 as f32 + 0.5) * row_h),
                        );
                        let prog = if live && !reduced {
                            Some((pulse_t + idx as f32 * 0.25).fract())
                        } else {
                            None
                        };
                        pro_connector_pulse(
                            p0,
                            p1,
                            6.0,
                            true,
                            handoff_base,
                            handoff_pulse,
                            prog,
                            1.25,
                            window,
                        );
                    }
                },
            )
            .absolute()
            .inset_0();

            let mut track_rows = div()
                .relative()
                .w_full()
                .h(px(total_track_h))
                .flex()
                .flex_col();
            for (_, lane) in &visible_lanes {
                let faded = highlighted.as_ref().is_some_and(|hid| hid != &lane.id);
                let row_op = pro_spring_f32(
                    pro_eid(&self.id, format!("mt-trow-op-{}", lane.id)),
                    if faded { 0.38 } else { 1.0 },
                    0.20,
                    w,
                    cx,
                );
                let mut lane_row = div().relative().w_full().h(px(row_h)).opacity(row_op);

                for task in &lane.tasks {
                    let lid = lane.id.clone();
                    let tid = task.id.clone();
                    let s_frac = task.start_frac.clamp(0.0, 0.96);
                    let grows_to_now = matches!(
                        task.status,
                        TimelineAgentStatus::Running | TimelineAgentStatus::Waiting
                    ) && task.end_frac >= 0.98;
                    let spring_end = pro_spring_f32(
                        pro_eid(&self.id, format!("mt-bar-end-{lid}-{tid}")),
                        task.end_frac.clamp(s_frac + 0.04, 1.0),
                        0.32,
                        w,
                        cx,
                    );
                    let e_frac = if grows_to_now && !reduced {
                        (spring_end - 0.045 * (1.0 - now_grow)).clamp(s_frac + 0.04, 1.0)
                    } else {
                        spring_end.clamp(s_frac + 0.04, 1.0)
                    };
                    let width_frac = (e_frac - s_frac).clamp(0.04, 1.0 - s_frac);

                    let (bar_bg, bar_fg, right_edge) = match task.status {
                        TimelineAgentStatus::Done => (p.chip, p.text, None),
                        TimelineAgentStatus::Running => (p.accent_soft, p.blue, Some(p.blue)),
                        TimelineAgentStatus::Waiting => (p.amber_soft, p.amber_ink, Some(p.amber)),
                        TimelineAgentStatus::Failed => (p.red_soft, p.red_ink, Some(p.red)),
                    };

                    let is_sel = self
                        .selected_task
                        .as_ref()
                        .is_some_and(|(a, t)| a == &lid && t == &tid);
                    let is_done = task.status == TimelineAgentStatus::Done;

                    let mut tools_row = div().flex_none().flex().items_center().gap(px(3.));
                    for tool in task.tools.iter().take(2) {
                        tools_row = tools_row.child(pro_app_tile(tool, 14., cx));
                    }

                    let bar_content = div()
                        .relative()
                        .h(px(28.))
                        .px(px(7.))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(bar_fg)
                        .when(is_done, |d| {
                            d.child(pro_check_draw(
                                pro_eid(&self.id, format!("mt-task-chk-{lid}-{tid}")),
                                true,
                                12.5,
                                false,
                                p.green,
                            ))
                        })
                        .when(!task.tools.is_empty(), |d| d.child(tools_row))
                        .child(div().min_w_0().truncate().child(task.title.clone()))
                        .when_some(right_edge, |d, ec| {
                            d.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .right_0()
                                    .w(px(2.5))
                                    .bg(ec),
                            )
                        });

                    lane_row = lane_row.child(
                        div()
                            .absolute()
                            .top(px(8.))
                            .bottom(px(8.))
                            .left(relative(s_frac))
                            .w(relative(width_frac))
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("mt-bar-{lid}-{tid}")),
                                    format!(
                                        "{}: {} ({})",
                                        lane.name, task.title, task.duration_label
                                    ),
                                    6.,
                                    bar_bg,
                                    pro_mix_color(bar_bg, bar_fg, 0.14),
                                    if is_sel { p.focus } else { transparent_black() },
                                    if is_sel {
                                        p.focus
                                    } else {
                                        bar_fg.opacity(0.32)
                                    },
                                    is_sel,
                                    bar_content,
                                    cx,
                                    move |this, _, cx| {
                                        let pair = (lid.clone(), tid.clone());
                                        if this.selected_task.as_ref() == Some(&pair) {
                                            this.selected_task = None;
                                        } else {
                                            this.selected_task = Some(pair);
                                            cx.emit(MissionTimelineEvent::TaskSelected {
                                                agent_id: lid.clone(),
                                                task_id: tid.clone(),
                                            });
                                        }
                                        cx.notify();
                                    },
                                )
                                .size_full()
                                .overflow_hidden(),
                            ),
                    );
                }
                track_rows = track_rows.child(lane_row);
            }

            div()
                .w_full()
                .pr(px(12.))
                .pb(px(8.))
                .flex()
                .items_start()
                .child(left_col)
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .h(px(total_track_h))
                        .child(grid_and_handoffs)
                        .child(track_rows),
                )
        };

        // Selected Gantt task disclosure strip (retaining last selection during fold exit)
        let selected_detail = self.selected_task.as_ref().and_then(|(aid, tid)| {
            let lane = self.lanes.iter().find(|l| &l.id == aid)?;
            let task = lane.tasks.iter().find(|t| &t.id == tid)?;
            Some((lane.clone(), task.clone()))
        });
        let detail_open = selected_detail.is_some();
        let cached_detail = w.use_keyed_state(pro_eid(&self.id, "mt-detail-cache"), cx, |_, _| {
            selected_detail.clone()
        });
        let detail_for_render = cached_detail.update(cx, |slot, _| {
            if selected_detail.is_some() {
                *slot = selected_detail.clone();
            }
            slot.clone()
        });

        let detail_row_el: Option<AnyElement> = detail_for_render.map(|(lane, task)| {
            let aid = lane.id.clone();
            let tid = task.id.clone();
            let is_done = task.status == TimelineAgentStatus::Done;
            let can_approve = lane.status == TimelineAgentStatus::Waiting;
            let can_retry = lane.status == TimelineAgentStatus::Failed;
            let first_choice = lane.ask_choices.first().cloned();

            let aid_tog = aid.clone();
            let tid_tog = tid.clone();
            let step_toggle = pro_tactile(
                pro_eid(&self.id, format!("mt-det-tog-{aid}-{tid}")),
                format!("Toggle {}", task.title),
                6.,
                transparent_black(),
                p.hover,
                transparent_black(),
                transparent_black(),
                false,
                div()
                    .px(px(6.))
                    .py(px(3.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .whitespace_nowrap()
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("mt-det-chk-{aid}-{tid}")),
                        is_done,
                        14.,
                        false,
                        p.green,
                    ))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(lane.name),
                    )
                    .child(div().text_color(p.muted).child("·"))
                    .child(div().text_color(p.ink).child(task.title))
                    .child(
                        div()
                            .text_color(p.faint)
                            .child(format!("({})", task.duration_label)),
                    ),
                cx,
                move |this, _, cx| {
                    this.toggle_task_done(&aid_tog, &tid_tog, cx);
                },
            )
            .flex_none();

            let quick_btn = (can_approve || can_retry).then(|| {
                let target = aid.clone();
                let ch = first_choice.clone();
                pro_btn(
                    pro_eid(&self.id, "mt-detail-quick-btn"),
                    if can_retry {
                        tr(cx, "Retry", "重试")
                    } else {
                        tr(cx, "Resolve", "处理")
                    },
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, _, cx| {
                        if can_retry {
                            this.resolve_retry(&target, cx);
                        } else {
                            this.resolve_approve(&target, ch.clone(), cx);
                        }
                    },
                )
            });

            let close_btn = pro_icon_btn(
                pro_eid(&self.id, "mt-detail-close-btn"),
                tr(cx, "Close detail", "关闭详情"),
                ProIcon::Close,
                24.,
                false,
                cx,
                |this, _, cx| {
                    this.selected_task = None;
                    cx.notify();
                },
            );

            div()
                .w_full()
                .px(px(10.))
                .py(px(7.))
                .border_t_1()
                .border_color(p.line)
                .bg(p.subtle)
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .text_size(px(12.))
                .child(step_toggle)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .when_some(quick_btn, |d, b| d.child(b))
                        .child(close_btn),
                )
                .into_any_element()
        });

        let detail_fold = pro_fold(
            pro_eid(&self.id, "mt-task-detail-fold"),
            detail_open,
            46.,
            0.,
            move || detail_row_el.unwrap_or_else(|| div().into_any_element()),
        );

        div()
            .w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(16.))
                    .py(px(12.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_icon(ProIcon::Sparkles, 15., p.blue))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Agents", "智能体时间轴")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .child(div().size(px(6.)).rounded_full().bg(p.blue))
                                            .child(pro_swap_label(
                                                pro_eid(&self.id, "mt-hdr-running"),
                                                format!(
                                                    "{running_n} {}",
                                                    tr(cx, "running", "运行中")
                                                ),
                                                12.,
                                                FontWeight::NORMAL,
                                                p.muted,
                                            )),
                                    )
                                    .when(needs_n > 0, |d| {
                                        d.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(5.))
                                                .text_color(p.amber_ink)
                                                .child(
                                                    div().size(px(6.)).rounded_full().bg(p.amber),
                                                )
                                                .child(pro_swap_label(
                                                    pro_eid(&self.id, "mt-hdr-needs"),
                                                    format!(
                                                        "{needs_n} {}",
                                                        tr(cx, "need you", "待处理")
                                                    ),
                                                    12.,
                                                    FontWeight::NORMAL,
                                                    p.amber_ink,
                                                )),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(div().w(px(56.)).child(pro_spring_bar(
                                pro_eid(&self.id, "mt-lanes-prog"),
                                done_ratio,
                                4.,
                                2.,
                                p.chip,
                                p.green,
                            )))
                            .child(
                                div()
                                    .h(px(22.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(format!(
                                        "{} {}m",
                                        tr(cx, "Last", "最近"),
                                        self.span_minutes
                                    )),
                            ),
                    ),
            )
            .child(axis_header)
            .child(lanes_body)
            .child(detail_fold)
    }

    fn render_inbox_queue(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let highlighted = self.highlighted_agent().cloned();

        let actionable: Vec<&TimelineAgentLane> = self
            .lanes
            .iter()
            .filter(|l| {
                matches!(
                    l.status,
                    TimelineAgentStatus::Waiting | TimelineAgentStatus::Failed
                ) && l.status.matches(self.filter)
            })
            .collect();

        let lane_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf4f4f5).into()
        };

        let mut cards_col = div().w_full().flex().flex_col().gap(px(6.));
        if actionable.is_empty() {
            cards_col = cards_col.child(
                div()
                    .w_full()
                    .py(px(36.))
                    .px(px(12.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .text_center()
                    .text_size(px(12.5))
                    .text_color(p.muted)
                    .child(pro_check_draw(
                        pro_eid(&self.id, "mt-inb-all-clear"),
                        true,
                        20.,
                        false,
                        p.green,
                    ))
                    .child(tr(cx, "Nothing waiting on you", "暂无待你处理的请求")),
            );
        } else {
            for lane in &actionable {
                let aid = lane.id.clone();
                let faded = highlighted.as_ref().is_some_and(|hid| hid != &aid);
                let card_op = pro_spring_f32(
                    pro_eid(&self.id, format!("mt-inb-op-{aid}")),
                    if faded { 0.40 } else { 1.0 },
                    0.20,
                    w,
                    cx,
                );
                let is_failed = lane.status == TimelineAgentStatus::Failed;

                let (headline, desc, desc_col) = if is_failed {
                    let task_title = lane
                        .tasks
                        .last()
                        .map(|t| t.title.clone())
                        .unwrap_or_else(|| lane.name.clone());
                    (
                        task_title,
                        lane.error
                            .clone()
                            .unwrap_or_else(|| SharedString::from("Execution failed")),
                        p.red_ink,
                    )
                } else {
                    (
                        lane.ask_title.clone().unwrap_or_else(|| lane.name.clone()),
                        lane.ask_detail
                            .clone()
                            .unwrap_or_else(|| SharedString::from("Waiting on your confirmation")),
                        p.text,
                    )
                };

                let actions_row = if is_failed {
                    let aid_dis = aid.clone();
                    let aid_ret = aid.clone();
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("mt-inb-dis-{aid}")),
                            tr(cx, "Dismiss", "忽略"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_reject(&aid_dis, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("mt-inb-ret-{aid}")),
                            tr(cx, "Retry", "重试"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_retry(&aid_ret, cx);
                            },
                        ))
                } else if !lane.ask_choices.is_empty() {
                    let mut row = div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_end()
                        .gap(px(5.));
                    for (ci, ch) in lane.ask_choices.iter().enumerate() {
                        let aid_ch = aid.clone();
                        let ch_val = ch.clone();
                        let is_primary = ci + 1 == lane.ask_choices.len();
                        row = row.child(pro_btn(
                            pro_eid(&self.id, format!("mt-inb-ch-{aid}-{ci}")),
                            ch_val.clone(),
                            None,
                            if is_primary {
                                ProBtnStyle::Primary
                            } else {
                                ProBtnStyle::Secondary
                            },
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_approve(&aid_ch, Some(ch_val.clone()), cx);
                            },
                        ));
                    }
                    row
                } else {
                    let aid_rej = aid.clone();
                    let aid_app = aid.clone();
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("mt-inb-rej-{aid}")),
                            tr(cx, "Reject", "拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_reject(&aid_rej, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("mt-inb-app-{aid}")),
                            tr(cx, "Approve", "批准"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_approve(&aid_app, None, cx);
                            },
                        ))
                };

                cards_col = cards_col.child(
                    div()
                        .w_full()
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(11.))
                        .opacity(card_op)
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(div().mt(px(2.)).child(pro_icon(
                                    if is_failed {
                                        ProIcon::AlertTriangle
                                    } else {
                                        ProIcon::Sparkles
                                    },
                                    14.,
                                    if is_failed { p.red } else { p.amber },
                                )))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .child(headline),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .line_height(px(17.))
                                                .text_color(desc_col)
                                                .child(desc),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(5.))
                                .rounded(px(7.))
                                .bg(p.subtle)
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .child(format!("{} · {}", lane.name, lane.elapsed)),
                                )
                                .child(actions_row),
                        ),
                );
            }
        }

        div()
            .w_full()
            .min_h(px(310.))
            .rounded(px(12.))
            .bg(lane_bg)
            .border_1()
            .border_color(p.line)
            .p(px(6.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .h(px(30.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_icon(ProIcon::Sparkles, 13., p.blue))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "NEEDS YOU", "待你处理")),
                            ),
                    )
                    .child(
                        div()
                            .min_w(px(20.))
                            .h(px(18.))
                            .px(px(5.))
                            .rounded(px(5.))
                            .bg(p.chip_hover)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, "mt-inb-count"),
                                actionable.len().to_string(),
                                11.,
                                FontWeight::MEDIUM,
                                p.ink,
                            )),
                    ),
            )
            .child(cards_col)
    }
}

impl Render for MissionTimelineBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let has_live = count_timeline_lanes(&self.lanes, TimelineFilter::Running) > 0;
        if (has_live || self.loading) && !reduced {
            w.request_animation_frame();
        }

        let elapsed_secs = self.boot_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_secs * 1000.0;
        let pulse_t = (elapsed_secs / 2.2).fract();
        let now_grow = 0.5 + 0.5 * (elapsed_secs * 1.8).sin();

        let width = self.measured_width.get();
        let stack_columns = width < 850.;
        let avatar_only_col = width < 520.;
        let compact = width < 480.;
        let measure_cell = self.measured_width.clone();

        let heading = if self.title.as_ref() == "Mission timeline" {
            tr(cx, "Mission timeline", "全局任务时间轴").to_owned()
        } else {
            self.title.to_string()
        };

        let filter_sensor = pro_selection_sensor();
        let thumb_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut filter_bar = div()
            .relative()
            .p(px(2.))
            .rounded(px(9.))
            .bg(p.chip)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mt-filter-pill"),
                self.filter.slug(),
                filter_sensor.clone(),
                7.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for f in TimelineFilter::ALL {
            let sel = self.filter == f;
            let count = count_timeline_lanes(&self.lanes, f);
            let pill_content = div()
                .h(px(26.))
                .px(px(9.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if sel { p.strong } else { p.text })
                .child(f.label(cx))
                .child(pro_swap_label(
                    pro_eid(&self.id, format!("mt-fcount-{}", f.slug())),
                    count.to_string(),
                    12.,
                    FontWeight::MEDIUM,
                    p.muted,
                ));
            filter_bar = filter_bar.child(
                pro_tactile(
                    pro_eid(&self.id, format!("mt-filter-{}", f.slug())),
                    format!("{} {count}", f.label(cx)),
                    7.,
                    transparent_black(),
                    if sel { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    false,
                    pill_content,
                    cx,
                    move |this, _, cx| {
                        this.set_filter(f, cx);
                    },
                )
                .relative()
                .flex_none()
                .when(sel, |d| d.child(pro_mark_selected(&filter_sensor))),
            );
        }

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(filter_bar);

        if self.loading {
            let phase = (elapsed_secs * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(top_bar)
                .child(
                    div()
                        .w_full()
                        .h(px(146.))
                        .rounded(px(14.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .child(pro_skeleton_bar(240., 80., 8., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(310.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .child(pro_skeleton_bar(280., 220., 8., phase + 0.2, cx)),
                );
        }

        if self.empty || self.lanes.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                    .flex()
                .flex_col()
                .gap(px(14.))
                .child(top_bar)
                .child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Clock,
                            tr(cx, "No timeline activity yet", "暂无时间轴运行记录"),
                            tr(
                                cx,
                                "Agent swimlanes, handoff connectors, and pending approvals align on one clock here.",
                                "智能体启动后，其并行泳道、跨轨交接与待审批事项将在此统一对齐时间轴。",
                            ),
                            None,
                            cx,
                        )),
                );
        }

        let recap_card = self.render_recap_section(cx);
        let lanes_card =
            self.render_lanes_card(avatar_only_col, elapsed_ms, pulse_t, now_grow, w, cx);
        let inbox_queue = self.render_inbox_queue(w, cx);

        let lower_grid = div()
            .w_full()
            .flex()
            .when(stack_columns, |d| d.flex_col())
            .items_start()
            .gap(px(14.))
            .child(div().flex_1().min_w_0().w_full().child(lanes_card))
            .child(
                div()
                    .when(!stack_columns, |d| d.w(px(292.)).flex_none())
                    .when(stack_columns, |d| d.w_full())
                    .child(inbox_queue),
            );

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if compact { 16. } else { 32. }))
            .py(px(if compact { 20. } else { 28. }))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.)),
            )
            .child(top_bar)
            .child(pro_view_transition(
                pro_eid(&self.id, "mt-body-vtx"),
                SharedString::from("mt-ready"),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(recap_card)
                    .child(lower_grid),
            ))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TimelineAgentStatus, TimelineFilter, count_timeline_lanes, default_events,
        default_handoffs, default_lanes,
    };

    #[test]
    fn default_swimlanes_align_with_7_3_3_1_status_counts() {
        let lanes = default_lanes();
        let handoffs = default_handoffs();
        let events = default_events();

        assert_eq!(count_timeline_lanes(&lanes, TimelineFilter::All), 7);
        assert_eq!(count_timeline_lanes(&lanes, TimelineFilter::Running), 3);
        assert_eq!(count_timeline_lanes(&lanes, TimelineFilter::NeedsYou), 3);
        assert_eq!(count_timeline_lanes(&lanes, TimelineFilter::Failed), 1);
        assert_eq!(handoffs.len(), 4);
        assert_eq!(events.len(), 7);
    }

    #[test]
    fn task_spans_stay_ordered_and_bounded_on_shared_clock() {
        let mut lanes = default_lanes();
        for lane in &lanes {
            assert!(!lane.tasks.is_empty());
            let mut prev_end = 0.0_f32;
            for task in &lane.tasks {
                assert!(task.start_frac >= prev_end - 1e-4);
                assert!(task.end_frac > task.start_frac);
                assert!(task.end_frac <= 1.0);
                prev_end = task.end_frac;
            }
        }

        // Simulate retrying the failed Billing lane
        let billing = lanes
            .iter_mut()
            .find(|l| l.id.as_ref() == "billing")
            .expect("billing lane exists");
        billing.status = TimelineAgentStatus::Running;
        assert_eq!(count_timeline_lanes(&lanes, TimelineFilter::Failed), 0);
        assert_eq!(count_timeline_lanes(&lanes, TimelineFilter::Running), 4);
    }
}
