//! Interactive agent tool-call topology, live invocation log, and per-tool inspector.
//!
//! Renders an autonomous agent above its connected tools as an org chart,
//! compact split map, or pan-zoom canvas, paired with a resizable invocation
//! table and right-hand inspector for approvals, access modes, retries, and JSON payloads.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProSelectionSensor, pro_agent_avatar, pro_app_tile, pro_btn,
            pro_eid, pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected,
            pro_pill_indicator, pro_presence, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_spring_f32, pro_view_transition, pro_view_transition_fill, tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc as StdArc, OnceLock},
};
use web_time::Instant;

const SVG_WRENCH: &str = r#"<path d="M13 11L18 6" stroke="currentColor" stroke-width="1.9"></path><path d="M19 7L17 5L19.5 3.5L20.5 4.5L19 7Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M4.02513 8.97487C3.01416 7.96391 2.75095 6.48836 3.23548 5.23548L4.65748 6.65748H6.65748V4.65748L5.23548 3.23548C6.48836 2.75095 7.96391 3.01416 8.97487 4.02513C9.98621 5.03647 10.2493 6.51274 9.76398 7.76593L16.2341 14.236C17.4873 13.7507 18.9635 14.0138 19.9749 15.0251C20.9858 16.0361 21.2491 17.5116 20.7645 18.7645L19.3425 17.3425L17.3425 17.3425V19.3425L18.7645 20.7645C17.5116 21.2491 16.0361 20.9858 15.0251 19.9749C14.0145 18.9643 13.7511 17.4895 14.2349 16.2369L7.76312 9.76507C6.51053 10.2489 5.03571 9.98546 4.02513 8.97487Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12.203 14.5L6.59897 20.1041C6.07115 20.6319 5.2154 20.6319 4.68758 20.1041L3.89586 19.3124C3.36805 18.7846 3.36805 17.9288 3.89586 17.401L9.49994 11.7969" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_CALLS_SWAP: &str = r#"<path d="M19 9H6.65856C5.65277 9 5.14987 9 5.02472 8.69134C4.89957 8.38268 5.25517 8.01942 5.96637 7.29289L8.21091 5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5 15H17.3414C18.3472 15 18.8501 15 18.9753 15.3087C19.1004 15.6173 18.7448 15.9806 18.0336 16.7071L15.7891 19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_CHECK_CIRCLE: &str = r#"<path d="M22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12Z" stroke="currentColor" stroke-width="1.9"></path><path d="M8 12.5L10.5 15L16 9" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_STOP_SQUARE: &str = r#"<rect x="6" y="6" width="12" height="12" rx="2.5" stroke="currentColor" stroke-width="2.4"></rect>"#;

const SVG_PAUSE_BARS: &str = r#"<path d="M4 7C4 5.58579 4 4.87868 4.43934 4.43934C4.87868 4 5.58579 4 7 4C8.41421 4 9.12132 4 9.56066 4.43934C10 4.87868 10 5.58579 10 7V17C10 18.4142 10 19.1213 9.56066 19.5607C9.12132 20 8.41421 20 7 20C5.58579 20 4.87868 20 4.43934 19.5607C4 19.1213 4 18.4142 4 17V7Z" stroke="currentColor" stroke-width="1.9"></path><path d="M14 7C14 5.58579 14 4.87868 14.4393 4.43934C14.8787 4 15.5858 4 17 4C18.4142 4 19.1213 4 19.5607 4.43934C20 4.87868 20 5.58579 20 7V17C20 18.4142 20 19.1213 19.5607 19.5607C19.1213 20 18.4142 20 17 20C15.5858 20 14.8787 20 14.4393 19.5607C14 19.1213 14 18.4142 14 17V7Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const CHROME_MARK_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" width="48" height="48"><path d="M24 24L4.9474 13A22 22 0 0 1 43.0526 13Z" fill="#EA4335"/><path d="M24 24L43.0526 13A22 22 0 0 1 24 46Z" fill="#FBBC04"/><path d="M24 24L24 46A22 22 0 0 1 4.9474 13Z" fill="#34A853"/><circle cx="24" cy="24" r="10.5" fill="#ffffff"/><circle cx="24" cy="24" r="8.2" fill="#1A73E8"/></svg>"##;

const MACOS_FOLDER_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 26" width="48" height="39"><defs><linearGradient id="kui_atc_fb" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3E9FDF"/><stop offset="1" stop-color="#2B88D2"/></linearGradient><linearGradient id="kui_atc_ff" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#74C9F9"/><stop offset="1" stop-color="#5BB6EE"/></linearGradient></defs><path d="M3.2 0h7.6c1.1 0 1.8.3 2.6 1l1.5 1.3c.6.5 1.2.7 2 .7h11.9A3.2 3.2 0 0 1 32 6.2v16.6a3.2 3.2 0 0 1-3.2 3.2H3.2A3.2 3.2 0 0 1 0 22.8V3.2A3.2 3.2 0 0 1 3.2 0Z" fill="url(#kui_atc_fb)"/><rect y="5" width="32" height="21" rx="3.2" fill="url(#kui_atc_ff)"/><rect x="0.6" y="5.3" width="30.8" height="0.8" rx="0.4" fill="#ffffff" opacity="0.28"/><rect y="21.2" width="32" height="0.7" fill="#ffffff" opacity="0.22"/><rect y="22.8" width="32" height="0.7" fill="#ffffff" opacity="0.16"/></svg>"##;

/// Execution permission policy for a connected agent tool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolAccessMode {
    /// Agent invokes the tool autonomously without human confirmation.
    #[default]
    Allowed,
    /// Agent pauses on each invocation until a human approves or rejects it.
    AskFirst,
    /// Tool is temporarily disabled for this agent.
    Paused,
}

impl ToolAccessMode {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Allowed => tr(cx, "Allowed", "自动允许"),
            Self::AskFirst => tr(cx, "Ask first", "需先确认"),
            Self::Paused => tr(cx, "Paused", "已暂停"),
        }
    }
}

/// Lifecycle state of an individual tool invocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolCallStatus {
    /// Currently executing against the target tool.
    Running,
    /// Blocked awaiting human approval (`Needs you`).
    Waiting,
    /// Completed successfully.
    #[default]
    Done,
    /// Returned an upstream or validation error.
    Failed,
    /// Denied by a human reviewer.
    Rejected,
}

impl ToolCallStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Needs you", "待审批"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
            Self::Rejected => tr(cx, "Rejected", "已拒绝"),
        }
    }
}

/// Time window filter applied to tool-call metrics and log rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolCallsRange {
    /// Calls recorded today (`age_days == 0`).
    #[default]
    Today,
    /// Calls recorded within the last 7 days.
    SevenDays,
    /// Calls recorded within the last 30 days.
    ThirtyDays,
}

impl ToolCallsRange {
    fn max_age_days(self) -> u8 {
        match self {
            Self::Today => 0,
            Self::SevenDays => 7,
            Self::ThirtyDays => 30,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Today => tr(cx, "Today", "今天"),
            Self::SevenDays => tr(cx, "7d", "近 7 天"),
            Self::ThirtyDays => tr(cx, "30d", "近 30 天"),
        }
    }
}

/// Topology visualization mode for the upper agent-and-tools area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolCallsLayout {
    /// Agent on top with tools arranged in two rows beneath an orthogonal bus.
    #[default]
    Org,
    /// Agent in the center flanked by left and right tool columns.
    Compact,
    /// Pan-and-zoom dotted canvas with full detail cards.
    Canvas,
}

impl ToolCallsLayout {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Org => tr(cx, "Org chart", "组织树视图"),
            Self::Compact => tr(cx, "Compact", "紧凑分栏"),
            Self::Canvas => tr(cx, "Canvas", "自由画布"),
        }
    }
}

/// Active entity inspected in the right-hand detail sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolCallsSelection {
    /// Root agent overview and connected tools list.
    Agent,
    /// Specific connected tool by `tool_id`.
    Tool(SharedString),
    /// Specific invocation log entry by `call_id`.
    Call(SharedString),
}

/// Parameter descriptor shown inside a tool's schema section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolParamSpec {
    pub name: SharedString,
    pub kind: SharedString,
    pub required: bool,
}

impl ToolParamSpec {
    /// Construct a parameter schema entry.
    pub fn new(
        name: impl Into<SharedString>,
        kind: impl Into<SharedString>,
        required: bool,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            required,
        }
    }
}

/// Identity and model configuration of the supervised agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentProfile {
    pub id: SharedString,
    pub name: SharedString,
    pub task: SharedString,
    pub model: SharedString,
    pub maker: SharedString,
}

impl Default for AgentProfile {
    fn default() -> Self {
        Self {
            id: "ops-agent".into(),
            name: "Ops agent".into(),
            task: "Monitors payment failure spikes, checks issuer status pages and runbooks, opens Linear incidents, and drafts #ops updates.".into(),
            model: "Sonnet 5.5".into(),
            maker: "Claude".into(),
        }
    }
}

/// Connected integration or built-in capability exposed to the agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedTool {
    pub id: SharedString,
    pub name: SharedString,
    pub fn_name: SharedString,
    pub app: SharedString,
    pub source: SharedString,
    pub description: SharedString,
    pub mode: ToolAccessMode,
    pub params: Vec<ToolParamSpec>,
}

/// Single recorded invocation of a [`ConnectedTool`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallRecord {
    pub id: SharedString,
    pub tool_id: SharedString,
    pub timestamp: SharedString,
    pub age_days: u8,
    pub duration_ms: Option<u32>,
    pub status: ToolCallStatus,
    pub input_summary: SharedString,
    pub input_json: SharedString,
    pub output_json: SharedString,
    pub error_message: Option<SharedString>,
    pub tokens: Option<u32>,
    pub run_id: SharedString,
}

/// Events emitted by [`AgentToolCallsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentToolCallsEvent {
    /// Sidebar item clicked.
    Navigate(SharedString),
    /// Inspector selection changed (`None` when closed).
    Select(Option<ToolCallsSelection>),
    /// Pending `Waiting` call approved (`true`) or rejected (`false`).
    Resolve {
        call_id: SharedString,
        approved: bool,
    },
    /// Failed call retried from the inspector.
    Retry(SharedString),
    /// Tool execution policy changed (`Allowed`, `AskFirst`, `Paused`).
    ModeChange {
        tool_id: SharedString,
        mode: ToolAccessMode,
    },
    /// Tool disconnected from the agent after confirmation.
    Disconnect(SharedString),
    /// New tool connected from the integration picker.
    ConnectTool(SharedString),
    /// Connect-tool picker opened.
    AddTool,
    /// Agent paused (`true`) or resumed (`false`).
    Pause(bool),
}

impl EventEmitter<AgentToolCallsEvent> for AgentToolCallsScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogTab {
    All,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolPanelTab {
    Overview,
    Calls,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CopySlot {
    Input,
    Output,
}

/// Computed aggregate telemetry for a slice of [`ToolCallRecord`]s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToolCallMetrics {
    pub total: usize,
    pub failed: usize,
    pub waiting: usize,
    pub running: usize,
    pub success_pct: u32,
    pub avg_duration_ms: Option<u32>,
    pub total_tokens: u32,
}

/// Compute aggregate count, success rate, mean latency, and context tokens.
pub fn summarize_calls(calls: &[ToolCallRecord]) -> ToolCallMetrics {
    if calls.is_empty() {
        return ToolCallMetrics::default();
    }
    let total = calls.len();
    let mut failed = 0usize;
    let mut waiting = 0usize;
    let mut running = 0usize;
    let mut dur_sum: u64 = 0;
    let mut dur_count: u64 = 0;
    let mut total_tokens: u32 = 0;

    for c in calls {
        match c.status {
            ToolCallStatus::Failed => failed += 1,
            ToolCallStatus::Waiting => waiting += 1,
            ToolCallStatus::Running => running += 1,
            ToolCallStatus::Done | ToolCallStatus::Rejected => {}
        }
        if c.status != ToolCallStatus::Running
            && let Some(ms) = c.duration_ms
        {
            dur_sum += u64::from(ms);
            dur_count += 1;
        }
        if let Some(tok) = c.tokens {
            total_tokens = total_tokens.saturating_add(tok);
        }
    }

    let non_failed = total.saturating_sub(failed);
    let success_pct = ((non_failed as f64 / total as f64) * 100.0).round() as u32;
    let avg_duration_ms = (dur_count > 0).then(|| (dur_sum / dur_count) as u32);

    ToolCallMetrics {
        total,
        failed,
        waiting,
        running,
        success_pct,
        avg_duration_ms,
        total_tokens,
    }
}

fn format_duration_ms(ms: u32) -> String {
    if ms >= 10_000 {
        let secs = (ms as f32 / 1000.).round() as u32;
        format!("{secs}s")
    } else if ms >= 1000 {
        format!("{:.1}s", ms as f32 / 1000.)
    } else {
        format!("{ms}ms")
    }
}

fn format_tokens_compact(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f32 / 1_000_000.)
    } else if tokens >= 1000 {
        format!("{:.1}K", tokens as f32 / 1000.)
    } else {
        tokens.to_string()
    }
}

fn render_tool_tile(app: &str, side: f32, cx: &App) -> AnyElement {
    static SPECIAL: [OnceLock<StdArc<Image>>; 2] = [const { OnceLock::new() }; 2];
    let lower = app.trim().to_ascii_lowercase();
    let special = match lower.as_str() {
        "web" | "chrome" | "browser" => Some((0, CHROME_MARK_SVG)),
        "files" | "finder" | "folder" => Some((1, MACOS_FOLDER_SVG)),
        _ => None,
    };
    if let Some((slot, svg_doc)) = special {
        let p = ProPalette::get(cx);
        let radius = (side * 0.27).clamp(5., 8.);
        let mark_side = (side * 0.56).round();
        let tile_bg: Hsla = if p.dark {
            rgb(0x222226).into()
        } else {
            rgb(0xffffff).into()
        };
        let image = SPECIAL[slot]
            .get_or_init(|| {
                StdArc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    svg_doc.as_bytes().to_vec(),
                ))
            })
            .clone();
        return div()
            .size(px(side))
            .flex_none()
            .rounded(px(radius))
            .bg(tile_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_center()
            .child(img(image).size(px(mark_side)).flex_none())
            .into_any_element();
    }
    pro_app_tile(app, side, cx)
}

fn render_spinner_arc(side: f32, color: Hsla, phase_rad: f32) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let center = bounds.center();
            let stroke = (side * 0.16).clamp(1.5, 2.2);
            let r =
                (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) - stroke) * 0.5;
            if r <= 1. {
                return;
            }
            let steps = 24usize;
            let mut ring = PathBuilder::stroke(px(stroke));
            for i in 0..=steps {
                let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
                let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    ring.move_to(pt);
                } else {
                    ring.line_to(pt);
                }
            }
            if let Ok(path) = ring.build() {
                window.paint_path(path, color.opacity(0.22));
            }
            let mut arc = PathBuilder::stroke(px(stroke));
            let arc_steps = 10usize;
            for i in 0..=arc_steps {
                let a = phase_rad - std::f32::consts::FRAC_PI_2
                    + (i as f32 / arc_steps as f32) * std::f32::consts::FRAC_PI_2 * 1.15;
                let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    arc.move_to(pt);
                } else {
                    arc.line_to(pt);
                }
            }
            if let Ok(path) = arc.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn build_ortho_points(ax: f32, ay: f32, bus_y: f32, tx: f32, ty: f32) -> Vec<(f32, f32)> {
    let dx = tx - ax;
    if dx.abs() < 1.5 {
        return vec![(ax, ay), (tx, ty)];
    }
    let sign = dx.signum();
    let r = 7.0_f32
        .min((bus_y - ay).abs() * 0.45)
        .min((ty - bus_y).abs() * 0.45)
        .min(dx.abs() * 0.45);
    let mut pts = Vec::with_capacity(16);
    pts.push((ax, ay));
    pts.push((ax, bus_y - r));
    for step in 1..=4 {
        let t = step as f32 / 4.;
        let ang = if sign > 0. {
            std::f32::consts::PI - t * std::f32::consts::FRAC_PI_2
        } else {
            t * std::f32::consts::FRAC_PI_2
        };
        let cx_c = ax + sign * r;
        let cy_c = bus_y - r;
        pts.push((cx_c + r * ang.cos(), cy_c + r * ang.sin()));
    }
    pts.push((tx - sign * r, bus_y));
    for step in 1..=4 {
        let t = step as f32 / 4.;
        let ang = if sign > 0. {
            -std::f32::consts::FRAC_PI_2 + t * std::f32::consts::FRAC_PI_2
        } else {
            -std::f32::consts::FRAC_PI_2 - t * std::f32::consts::FRAC_PI_2
        };
        let cx_c = tx - sign * r;
        let cy_c = bus_y + r;
        pts.push((cx_c + r * ang.cos(), cy_c + r * ang.sin()));
    }
    pts.push((tx, ty));
    pts
}

fn build_horiz_ortho_points(ax: f32, ay: f32, bus_x: f32, tx: f32, ty: f32) -> Vec<(f32, f32)> {
    let dy = ty - ay;
    if dy.abs() < 1.5 {
        return vec![(ax, ay), (tx, ty)];
    }
    vec![(ax, ay), (bus_x, ay), (bus_x, ty), (tx, ty)]
}

fn paint_polyline_slice(
    origin: Point<Pixels>,
    pts: &[(f32, f32)],
    t0: f32,
    t1: f32,
    width: f32,
    color: Hsla,
    window: &mut Window,
) {
    if pts.len() < 2 || t1 <= t0 {
        return;
    }
    let mut seg_lens = Vec::with_capacity(pts.len() - 1);
    let mut total = 0.0_f32;
    for pair in pts.windows(2) {
        let len = (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
        seg_lens.push(len);
        total += len;
    }
    if total <= 1. {
        return;
    }
    let start_d = t0.clamp(0., 1.) * total;
    let end_d = t1.clamp(0., 1.) * total;
    if end_d - start_d <= 0.5 {
        return;
    }

    let mut pb = PathBuilder::stroke(px(width));
    let mut started = false;
    let mut acc = 0.0_f32;
    for (idx, pair) in pts.windows(2).enumerate() {
        let len = seg_lens[idx];
        let next_acc = acc + len;
        if next_acc >= start_d && acc <= end_d && len > 0.01 {
            let u0 = ((start_d - acc) / len).clamp(0., 1.);
            let u1 = ((end_d - acc) / len).clamp(0., 1.);
            let x0 = pair[0].0 + (pair[1].0 - pair[0].0) * u0;
            let y0 = pair[0].1 + (pair[1].1 - pair[0].1) * u0;
            let x1 = pair[0].0 + (pair[1].0 - pair[0].0) * u1;
            let y1 = pair[0].1 + (pair[1].1 - pair[0].1) * u1;
            if !started {
                pb.move_to(origin + point(px(x0), px(y0)));
                started = true;
            }
            pb.line_to(origin + point(px(x1), px(y1)));
        }
        acc = next_acc;
    }
    if started && let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn default_tools() -> Vec<ConnectedTool> {
    vec![
        ConnectedTool {
            id: "web".into(),
            name: "Web".into(),
            fn_name: "web.search".into(),
            app: "Web".into(),
            source: "Built-in · Browser".into(),
            description:
                "Searches live web pages and upstream status dashboards with headless Chromium."
                    .into(),
            mode: ToolAccessMode::Allowed,
            params: vec![
                ToolParamSpec::new("query", "string", false),
                ToolParamSpec::new("url", "string", false),
                ToolParamSpec::new("timeout_ms", "integer", false),
            ],
        },
        ConnectedTool {
            id: "files".into(),
            name: "Files".into(),
            fn_name: "fs.read_write".into(),
            app: "Files".into(),
            source: "Workspace".into(),
            description:
                "Reads runbooks and writes diagnostic CSV exports inside the workspace sandbox."
                    .into(),
            mode: ToolAccessMode::Allowed,
            params: vec![
                ToolParamSpec::new("path", "string", true),
                ToolParamSpec::new("encoding", "string", false),
            ],
        },
        ConnectedTool {
            id: "notion".into(),
            name: "Notion".into(),
            fn_name: "notion.search".into(),
            app: "Notion".into(),
            source: "MCP · Notion".into(),
            description:
                "Queries incident postmortems, issuer guides, and payment runbooks in Notion."
                    .into(),
            mode: ToolAccessMode::Allowed,
            params: vec![
                ToolParamSpec::new("query", "string", true),
                ToolParamSpec::new("top_k", "integer", false),
            ],
        },
        ConnectedTool {
            id: "stripe".into(),
            name: "Stripe".into(),
            fn_name: "stripe.payment_intents.retrieve".into(),
            app: "Stripe".into(),
            source: "API · Stripe".into(),
            description:
                "Inspects payment intents, decline codes, issuer metadata, and refund attempts."
                    .into(),
            mode: ToolAccessMode::Allowed,
            params: vec![
                ToolParamSpec::new("payment_id", "string", true),
                ToolParamSpec::new("expand", "string[]", false),
            ],
        },
        ConnectedTool {
            id: "linear".into(),
            name: "Linear".into(),
            fn_name: "linear.issues.create".into(),
            app: "Linear".into(),
            source: "API · Linear".into(),
            description:
                "Creates and updates prioritized engineering and support incident tickets.".into(),
            mode: ToolAccessMode::Allowed,
            params: vec![
                ToolParamSpec::new("title", "string", true),
                ToolParamSpec::new("team", "string", true),
                ToolParamSpec::new("priority", "enum", false),
            ],
        },
        ConnectedTool {
            id: "slack".into(),
            name: "Slack".into(),
            fn_name: "slack.chat.postMessage".into(),
            app: "Slack".into(),
            source: "API · Slack".into(),
            description: "Posts incident summaries and status alerts to shared workspace channels."
                .into(),
            mode: ToolAccessMode::AskFirst,
            params: vec![
                ToolParamSpec::new("channel", "string", true),
                ToolParamSpec::new("text", "string", true),
            ],
        },
        ConnectedTool {
            id: "gmail".into(),
            name: "Gmail".into(),
            fn_name: "gmail.messages.send".into(),
            app: "Gmail".into(),
            source: "API · Gmail".into(),
            description: "Sends external customer and partner advisories when explicitly enabled."
                .into(),
            mode: ToolAccessMode::Paused,
            params: vec![
                ToolParamSpec::new("to", "string", true),
                ToolParamSpec::new("subject", "string", true),
                ToolParamSpec::new("body", "string", true),
            ],
        },
    ]
}

fn catalog_extra_tools() -> Vec<ConnectedTool> {
    vec![
        ConnectedTool {
            id: "github".into(),
            name: "GitHub".into(),
            fn_name: "github.pulls.list".into(),
            app: "GitHub".into(),
            source: "MCP · GitHub".into(),
            description: "Checks recent payment-service deployments and open pull requests.".into(),
            mode: ToolAccessMode::Allowed,
            params: vec![
                ToolParamSpec::new("repo", "string", true),
                ToolParamSpec::new("state", "string", false),
            ],
        },
        ConnectedTool {
            id: "calendar".into(),
            name: "Google Calendar".into(),
            fn_name: "calendar.events.list".into(),
            app: "Google Calendar".into(),
            source: "API · Google".into(),
            description: "Looks up the active on-call rotation and incident bridge schedule."
                .into(),
            mode: ToolAccessMode::Allowed,
            params: vec![ToolParamSpec::new("calendar_id", "string", true)],
        },
        ConnectedTool {
            id: "postgres".into(),
            name: "Postgres".into(),
            fn_name: "db.readonly_query".into(),
            app: "Postgres".into(),
            source: "MCP · Warehouse".into(),
            description: "Runs read-only SQL aggregations over checkout and settlement tables."
                .into(),
            mode: ToolAccessMode::AskFirst,
            params: vec![
                ToolParamSpec::new("sql", "string", true),
                ToolParamSpec::new("limit", "integer", false),
            ],
        },
        ConnectedTool {
            id: "drive".into(),
            name: "Google Drive".into(),
            fn_name: "drive.files.get".into(),
            app: "Google Drive".into(),
            source: "API · Google".into(),
            description: "Reads shared finance settlement sheets and SLA attachments.".into(),
            mode: ToolAccessMode::Allowed,
            params: vec![ToolParamSpec::new("file_id", "string", true)],
        },
    ]
}

fn default_calls() -> Vec<ToolCallRecord> {
    vec![
        ToolCallRecord {
            id: "call-15".into(),
            tool_id: "web".into(),
            timestamp: "09:40:58".into(),
            age_days: 0,
            duration_ms: Some(3000),
            status: ToolCallStatus::Running,
            input_summary: r#"query="commerzbank card payments declined today""#.into(),
            input_json: "{\n  \"query\": \"commerzbank card payments declined today\",\n  \"region\": \"DE\",\n  \"max_results\": 5\n}".into(),
            output_json: "{\n  \"status\": \"streaming\",\n  \"fetched_sources\": 3\n}".into(),
            error_message: None,
            tokens: None,
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-14".into(),
            tool_id: "slack".into(),
            timestamp: "09:40:22".into(),
            age_days: 0,
            duration_ms: None,
            status: ToolCallStatus::Waiting,
            input_summary: r##"channel="#ops"  text="Card payments from DE fail since 09:12 – 41 so far, almost all do_not_honor from one issuer. LIN-482 is open.""##.into(),
            input_json: "{\n  \"channel\": \"#ops\",\n  \"text\": \"Card payments from DE fail since 09:12 – 41 so far, almost all do_not_honor from one issuer. LIN-482 is open.\"\n}".into(),
            output_json: "{\n  \"pending_approval\": true,\n  \"policy\": \"ask_first\"\n}".into(),
            error_message: None,
            tokens: None,
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-13".into(),
            tool_id: "files".into(),
            timestamp: "09:40:08".into(),
            age_days: 0,
            duration_ms: Some(420),
            status: ToolCallStatus::Done,
            input_summary: r#"path="exports/failed-payments-2026-11-19.csv""#.into(),
            input_json: "{\n  \"path\": \"exports/failed-payments-2026-11-19.csv\",\n  \"rows\": 41\n}".into(),
            output_json: "{\n  \"written_bytes\": 6840,\n  \"sha256\": \"9c4b1e8a...41f0\"\n}".into(),
            error_message: None,
            tokens: Some(2100),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-12".into(),
            tool_id: "linear".into(),
            timestamp: "09:39:50".into(),
            age_days: 0,
            duration_ms: Some(820),
            status: ToolCallStatus::Done,
            input_summary: r#"title="Card payments from DE failing since 09:12"  team="Payments"  priority="High""#.into(),
            input_json: "{\n  \"title\": \"Card payments from DE failing since 09:12\",\n  \"team\": \"Payments\",\n  \"priority\": \"High\"\n}".into(),
            output_json: "{\n  \"identifier\": \"LIN-482\",\n  \"url\": \"https://linear.app/northwind/issue/LIN-482\"\n}".into(),
            error_message: None,
            tokens: Some(140),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-11".into(),
            tool_id: "stripe".into(),
            timestamp: "09:39:20".into(),
            age_days: 0,
            duration_ms: Some(640),
            status: ToolCallStatus::Done,
            input_summary: r#"payment_id="pi_3QK8xL2eZvKYlo2C1""#.into(),
            input_json: "{\n  \"payment_id\": \"pi_3QK8xL2eZvKYlo2C1\",\n  \"expand\": [\"latest_charge\"]\n}".into(),
            output_json: "{\n  \"id\": \"pi_3QK8xL2eZvKYlo2C1\",\n  \"status\": \"requires_payment_method\",\n  \"decline_code\": \"do_not_honor\",\n  \"issuer_country\": \"DE\"\n}".into(),
            error_message: None,
            tokens: Some(410),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-10".into(),
            tool_id: "stripe".into(),
            timestamp: "09:38:55".into(),
            age_days: 0,
            duration_ms: Some(380),
            status: ToolCallStatus::Failed,
            input_summary: r#"payment_id="pi_3QK8xL2eZvKYl""#.into(),
            input_json: "{\n  \"payment_id\": \"pi_3QK8xL2eZvKYl\"\n}".into(),
            output_json: "{\n  \"error\": {\n    \"type\": \"invalid_request_error\",\n    \"code\": \"resource_missing\",\n    \"param\": \"intent\"\n  }\n}".into(),
            error_message: Some("resource_missing: No such payment_intent: 'pi_3QK8xL2eZvKYl'".into()),
            tokens: Some(60),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-09".into(),
            tool_id: "notion".into(),
            timestamp: "09:38:20".into(),
            age_days: 0,
            duration_ms: Some(920),
            status: ToolCallStatus::Done,
            input_summary: r#"query="card declined do_not_honor"  top_k=3"#.into(),
            input_json: "{\n  \"query\": \"card declined do_not_honor\",\n  \"top_k\": 3\n}".into(),
            output_json: "{\n  \"matches\": [\n    { \"title\": \"Issuer Outage Playbook\", \"score\": 0.94 },\n    { \"title\": \"DE SEPA & Card Escalation\", \"score\": 0.87 }\n  ]\n}".into(),
            error_message: None,
            tokens: Some(1200),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-08".into(),
            tool_id: "files".into(),
            timestamp: "09:36:50".into(),
            age_days: 0,
            duration_ms: Some(180),
            status: ToolCallStatus::Done,
            input_summary: r#"path="runbooks/issuer-outage.md""#.into(),
            input_json: "{\n  \"path\": \"runbooks/issuer-outage.md\"\n}".into(),
            output_json: "{\n  \"content\": \"# Issuer Outage Runbook\\n1. Verify Stripe status...\",\n  \"lines\": 64\n}".into(),
            error_message: None,
            tokens: Some(860),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-07".into(),
            tool_id: "web".into(),
            timestamp: "09:36:00".into(),
            age_days: 0,
            duration_ms: Some(1900),
            status: ToolCallStatus::Done,
            input_summary: r#"url="https://status.stripe.com""#.into(),
            input_json: "{\n  \"url\": \"https://status.stripe.com\"\n}".into(),
            output_json: "{\n  \"http_status\": 200,\n  \"indicator\": \"none\",\n  \"summary\": \"All systems operational\"\n}".into(),
            error_message: None,
            tokens: Some(940),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-06".into(),
            tool_id: "web".into(),
            timestamp: "09:35:20".into(),
            age_days: 0,
            duration_ms: Some(2300),
            status: ToolCallStatus::Done,
            input_summary: r#"query="commerzbank card outage today""#.into(),
            input_json: "{\n  \"query\": \"commerzbank card outage today\"\n}".into(),
            output_json: "{\n  \"results\": 4,\n  \"top_domain\": \"allestoerungen.de\"\n}".into(),
            error_message: None,
            tokens: Some(1800),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-05".into(),
            tool_id: "web".into(),
            timestamp: "09:34:40".into(),
            age_days: 0,
            duration_ms: Some(15_000),
            status: ToolCallStatus::Failed,
            input_summary: r#"url="https://www.commerzbank.de/status""#.into(),
            input_json: "{\n  \"url\": \"https://www.commerzbank.de/status\",\n  \"timeout_ms\": 15000\n}".into(),
            output_json: "{\n  \"error\": {\n    \"code\": \"ETIMEDOUT\",\n    \"timeout_ms\": 15000\n  }\n}".into(),
            error_message: Some("ETIMEDOUT: upstream host did not respond within 15000ms".into()),
            tokens: Some(0),
            run_id: "run_8f3a2c".into(),
        },
        ToolCallRecord {
            id: "call-04".into(),
            tool_id: "slack".into(),
            timestamp: "09:19:00".into(),
            age_days: 0,
            duration_ms: None,
            status: ToolCallStatus::Rejected,
            input_summary: r##"channel="#general"  text="Heads up: payments are down""##.into(),
            input_json: "{\n  \"channel\": \"#general\",\n  \"text\": \"Heads up: payments are down\"\n}".into(),
            output_json: "{\n  \"rejected_by\": \"Alex Morgan\",\n  \"reason\": \"Post to #ops instead of #general\"\n}".into(),
            error_message: None,
            tokens: None,
            run_id: "run_7d10e4".into(),
        },
        ToolCallRecord {
            id: "call-03".into(),
            tool_id: "linear".into(),
            timestamp: "08:53:00".into(),
            age_days: 0,
            duration_ms: Some(760),
            status: ToolCallStatus::Done,
            input_summary: r#"title="Refund for order 10482 failed twice"  team="Support"  priority="Medium""#.into(),
            input_json: "{\n  \"title\": \"Refund for order 10482 failed twice\",\n  \"team\": \"Support\",\n  \"priority\": \"Medium\"\n}".into(),
            output_json: "{\n  \"identifier\": \"SUP-194\",\n  \"status\": \"Triage\"\n}".into(),
            error_message: None,
            tokens: Some(130),
            run_id: "run_7d10e4".into(),
        },
        ToolCallRecord {
            id: "call-02".into(),
            tool_id: "notion".into(),
            timestamp: "08:29:00".into(),
            age_days: 0,
            duration_ms: Some(880),
            status: ToolCallStatus::Done,
            input_summary: r#"query="refund failed insufficient funds"  top_k=3"#.into(),
            input_json: "{\n  \"query\": \"refund failed insufficient funds\",\n  \"top_k\": 3\n}".into(),
            output_json: "{\n  \"matches\": [\n    { \"title\": \"Refund Retry Policy\", \"score\": 0.91 }\n  ]\n}".into(),
            error_message: None,
            tokens: Some(980),
            run_id: "run_7d10e4".into(),
        },
        ToolCallRecord {
            id: "call-01".into(),
            tool_id: "stripe".into(),
            timestamp: "08:11:00".into(),
            age_days: 0,
            duration_ms: Some(640),
            status: ToolCallStatus::Done,
            input_summary: r#"payment_id="re_3QK2bA2eZvKYlo2R7""#.into(),
            input_json: "{\n  \"payment_id\": \"re_3QK2bA2eZvKYlo2R7\"\n}".into(),
            output_json: "{\n  \"id\": \"re_3QK2bA2eZvKYlo2R7\",\n  \"status\": \"succeeded\",\n  \"amount\": 4900\n}".into(),
            error_message: None,
            tokens: Some(310),
            run_id: "run_7d10e4".into(),
        },
        ToolCallRecord {
            id: "call-hist-2".into(),
            tool_id: "stripe".into(),
            timestamp: "Nov 17 · 18:14".into(),
            age_days: 2,
            duration_ms: Some(510),
            status: ToolCallStatus::Done,
            input_summary: r#"payment_id="pi_3QJ1mP2eZvKYlo9A4""#.into(),
            input_json: "{\n  \"payment_id\": \"pi_3QJ1mP2eZvKYlo9A4\"\n}".into(),
            output_json: "{\n  \"id\": \"pi_3QJ1mP2eZvKYlo9A4\",\n  \"status\": \"succeeded\"\n}".into(),
            error_message: None,
            tokens: Some(390),
            run_id: "run_5a91c0".into(),
        },
        ToolCallRecord {
            id: "call-hist-1".into(),
            tool_id: "notion".into(),
            timestamp: "Nov 04 · 14:05".into(),
            age_days: 15,
            duration_ms: Some(840),
            status: ToolCallStatus::Done,
            input_summary: r#"query="chargeback dispute evidence"  top_k=5"#.into(),
            input_json: "{\n  \"query\": \"chargeback dispute evidence\",\n  \"top_k\": 5\n}".into(),
            output_json: "{\n  \"matches\": 5,\n  \"top_doc\": \"Dispute SLA Checklist\"\n}".into(),
            error_message: None,
            tokens: Some(1450),
            run_id: "run_2b04d8".into(),
        },
    ]
}

/// KUI Pro screen inspecting one agent's connected tool org chart and invocation log.
pub struct AgentToolCallsScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    agent: AgentProfile,
    tools: Vec<ConnectedTool>,
    calls: Vec<ToolCallRecord>,
    range: ToolCallsRange,
    layout: ToolCallsLayout,
    live: bool,
    paused: bool,
    loading: bool,
    empty: bool,
    log_tab: LogTab,
    tool_filter: Option<SharedString>,
    selection: Option<ToolCallsSelection>,
    last_selection: Option<ToolCallsSelection>,
    tool_panel_tab: ToolPanelTab,
    confirm_disconnect: bool,
    headers_open: bool,
    copied: Option<CopySlot>,
    agent_menu_open: bool,
    range_menu_open: bool,
    picker_open: bool,
    run_folded: bool,
    run_height: f32,
    resizing_run: Option<(f32, f32)>,
    canvas_zoom: f32,
    canvas_pan: (f32, f32),
    search_open: bool,
    search_editor: Option<Entity<Editor>>,
    measured_width: Rc<Cell<f32>>,
    boot_at: Instant,
    retry_seq: u32,
}

impl AgentToolCallsScreen {
    /// Construct a new [`AgentToolCallsScreen`] preloaded with the `Northwind` ops telemetry dataset.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new(
                        "orchestrations",
                        tr(shell_cx, "Orchestrations", "编排"),
                        ProIcon::GitBranch,
                    ),
                    ShellNavItem::new("runs", tr(shell_cx, "Runs", "运行记录"), ProIcon::Play)
                        .with_count(3),
                    ShellNavItem::new("agents", tr(shell_cx, "Agents", "智能体"), ProIcon::Users),
                    ShellNavItem::new(
                        "traces",
                        tr(shell_cx, "Traces", "调用追踪"),
                        ProIcon::UsageChart,
                    ),
                    ShellNavItem::new("tools", tr(shell_cx, "Tools", "工具库"), ProIcon::Sliders),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "knowledge",
                            tr(shell_cx, "Knowledge", "知识库"),
                            ProIcon::PageDoc,
                        ),
                        ShellNavItem::new(
                            "settings",
                            tr(shell_cx, "Settings", "设置"),
                            ProIcon::Settings,
                        ),
                    ],
                ),
            ];
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Northwind")
                .with_user("Alex Morgan", "alex@northwind.co")
                .with_groups(groups)
                .with_side_cards(false)
                .with_active("agents")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });

        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                cx.emit(AgentToolCallsEvent::Navigate(label.clone()));
            }
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            standalone: false,
            agent: AgentProfile::default(),
            tools: default_tools(),
            calls: default_calls(),
            range: ToolCallsRange::Today,
            layout: ToolCallsLayout::Org,
            live: true,
            paused: false,
            loading: false,
            empty: false,
            log_tab: LogTab::All,
            tool_filter: None,
            selection: None,
            last_selection: None,
            tool_panel_tab: ToolPanelTab::Overview,
            confirm_disconnect: false,
            headers_open: false,
            copied: None,
            agent_menu_open: false,
            range_menu_open: false,
            picker_open: false,
            run_folded: false,
            run_height: 236.,
            resizing_run: None,
            canvas_zoom: 1.0,
            canvas_pan: (0., 0.),
            search_open: false,
            search_editor: None,
            measured_width: Rc::new(Cell::new(920.)),
            boot_at: Instant::now(),
            retry_seq: 1,
        }
    }

    /// Render without the surrounding [`AppShell`] sidebar chrome.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace title in the [`AppShell`] sidebar.
    pub fn with_workspace(
        self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.shell.update(cx, |s, shell_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", shell_cx)).with_workspace(ws);
        });
        self
    }

    /// Replace the supervised [`AgentProfile`].
    pub fn with_agent(mut self, agent: AgentProfile) -> Self {
        self.agent = agent;
        self
    }

    /// Replace the connected tools list.
    pub fn with_tools(mut self, tools: Vec<ConnectedTool>) -> Self {
        self.tools = tools;
        self
    }

    /// Replace the recorded tool calls.
    pub fn with_calls(mut self, calls: Vec<ToolCallRecord>) -> Self {
        self.calls = calls;
        self
    }

    /// Set the initial time window filter (`Today`, `SevenDays`, `ThirtyDays`).
    pub fn with_range(mut self, range: ToolCallsRange) -> Self {
        self.range = range;
        self
    }

    /// Set the initial topology layout (`Org`, `Compact`, `Canvas`).
    pub fn with_layout(mut self, layout: ToolCallsLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Control whether live call timers and connector light pulses animate.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Enable or disable skeleton placeholder rendering.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the zero-tools empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Update loading skeleton state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Update empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        if empty {
            self.selection = None;
        }
        cx.notify();
    }

    /// Return whether the agent is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Return the active right-panel selection, if open.
    pub fn selection(&self) -> Option<&ToolCallsSelection> {
        self.selection.as_ref()
    }

    fn select(&mut self, next: Option<ToolCallsSelection>, cx: &mut Context<Self>) {
        self.confirm_disconnect = false;
        self.copied = None;
        self.agent_menu_open = false;
        self.range_menu_open = false;
        if self.selection == next {
            self.selection = None;
        } else {
            self.selection = next;
        }
        cx.emit(AgentToolCallsEvent::Select(self.selection.clone()));
        cx.notify();
    }

    fn calls_in_range(&self) -> Vec<&ToolCallRecord> {
        let max_age = self.range.max_age_days();
        self.calls
            .iter()
            .filter(|c| c.age_days <= max_age)
            .collect()
    }

    fn visible_calls(&self, cx: &App) -> Vec<ToolCallRecord> {
        let max_age = self.range.max_age_days();
        let q = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_ascii_lowercase())
            .unwrap_or_default();
        self.calls
            .iter()
            .filter(|c| c.age_days <= max_age)
            .filter(|c| match self.log_tab {
                LogTab::All => true,
                LogTab::Failed => c.status == ToolCallStatus::Failed,
            })
            .filter(|c| {
                self.tool_filter
                    .as_ref()
                    .is_none_or(|tid| tid == &c.tool_id)
            })
            .filter(|c| {
                if q.is_empty() {
                    return true;
                }
                let tool_name = self
                    .find_tool(&c.tool_id)
                    .map(|t| t.name.to_ascii_lowercase())
                    .unwrap_or_else(|| c.tool_id.to_ascii_lowercase());
                tool_name.contains(&q)
                    || c.input_summary.to_ascii_lowercase().contains(&q)
                    || c.run_id.to_ascii_lowercase().contains(&q)
                    || c.timestamp.to_ascii_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    fn find_tool(&self, tool_id: &str) -> Option<&ConnectedTool> {
        self.tools.iter().find(|t| t.id.as_ref() == tool_id)
    }

    fn tool_live_status(&self, tool: &ConnectedTool) -> Option<ToolCallStatus> {
        if tool.mode == ToolAccessMode::Paused {
            return None;
        }
        let max_age = self.range.max_age_days();
        let mut has_waiting = false;
        for c in &self.calls {
            if c.age_days <= max_age && c.tool_id == tool.id {
                if c.status == ToolCallStatus::Running {
                    return Some(ToolCallStatus::Running);
                }
                if c.status == ToolCallStatus::Waiting {
                    has_waiting = true;
                }
            }
        }
        has_waiting.then_some(ToolCallStatus::Waiting)
    }

    fn resolve_call(&mut self, call_id: &str, approved: bool, cx: &mut Context<Self>) {
        if let Some(call) = self.calls.iter_mut().find(|c| c.id.as_ref() == call_id)
            && call.status == ToolCallStatus::Waiting
        {
            if approved {
                call.status = ToolCallStatus::Done;
                call.duration_ms = Some(340);
                call.tokens = Some(190);
                call.output_json =
                    "{\n  \"ok\": true,\n  \"ts\": \"1763545222.0041\",\n  \"channel\": \"C04OPS\"\n}"
                        .into();
            } else {
                call.status = ToolCallStatus::Rejected;
                call.output_json =
                    "{\n  \"rejected_by\": \"Alex Morgan\",\n  \"status\": \"denied\"\n}".into();
            }
            cx.emit(AgentToolCallsEvent::Resolve {
                call_id: call_id.to_owned().into(),
                approved,
            });
            cx.notify();
        }
    }

    fn retry_call(&mut self, call_id: &str, cx: &mut Context<Self>) {
        let Some(orig) = self
            .calls
            .iter()
            .find(|c| c.id.as_ref() == call_id)
            .cloned()
        else {
            return;
        };
        let seq = self.retry_seq;
        self.retry_seq = self.retry_seq.wrapping_add(1);
        let new_id: SharedString = format!("call-retry-{seq}").into();
        let retried = ToolCallRecord {
            id: new_id.clone(),
            tool_id: orig.tool_id,
            timestamp: "09:41:12".into(),
            age_days: 0,
            duration_ms: Some(410),
            status: ToolCallStatus::Done,
            input_summary: orig.input_summary,
            input_json: orig.input_json,
            output_json: format!(
                "{{\n  \"ok\": true,\n  \"retried_from\": \"{call_id}\",\n  \"http_status\": 200\n}}"
            )
            .into(),
            error_message: None,
            tokens: Some(320),
            run_id: orig.run_id,
        };
        self.calls.insert(0, retried);
        self.selection = Some(ToolCallsSelection::Call(new_id));
        cx.emit(AgentToolCallsEvent::Retry(call_id.to_owned().into()));
        cx.notify();
    }

    fn set_tool_mode(&mut self, tool_id: &str, mode: ToolAccessMode, cx: &mut Context<Self>) {
        if let Some(t) = self.tools.iter_mut().find(|t| t.id.as_ref() == tool_id) {
            t.mode = mode;
            cx.emit(AgentToolCallsEvent::ModeChange {
                tool_id: tool_id.to_owned().into(),
                mode,
            });
            cx.notify();
        }
    }

    fn disconnect_tool(&mut self, tool_id: &str, cx: &mut Context<Self>) {
        self.tools.retain(|t| t.id.as_ref() != tool_id);
        self.confirm_disconnect = false;
        if matches!(
            &self.selection,
            Some(ToolCallsSelection::Tool(tid)) if tid.as_ref() == tool_id
        ) {
            self.selection = None;
            cx.emit(AgentToolCallsEvent::Select(None));
        }
        cx.emit(AgentToolCallsEvent::Disconnect(tool_id.to_owned().into()));
        cx.notify();
    }

    fn copy_payload(&mut self, slot: CopySlot, payload: SharedString, cx: &mut Context<Self>) {
        let task = clipboard::write(&payload, cx);
        cx.spawn(async move |weak, cx| {
            let _ = task.await;
            let _ = weak.update(cx, |this, cx| {
                this.copied = Some(slot);
                cx.notify();
            });
        })
        .detach();
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.picker_open {
                self.picker_open = false;
                cx.notify();
                return;
            }
            if self.agent_menu_open || self.range_menu_open {
                self.agent_menu_open = false;
                self.range_menu_open = false;
                cx.notify();
                return;
            }
            if self.search_open {
                if let Some(ed) = &self.search_editor {
                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                }
                self.search_open = false;
                cx.notify();
                return;
            }
            if self.selection.is_some() {
                self.select(None, cx);
                return;
            }
        }

        if self
            .search_editor
            .as_ref()
            .is_some_and(|ed| ed.read(cx).focus_handle(cx).is_focused(w))
        {
            return;
        }

        if key == "/" {
            self.search_open = true;
            if let Some(ed) = &self.search_editor {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
            cx.notify();
            return;
        }

        if self.layout == ToolCallsLayout::Canvas && ev.keystroke.modifiers.alt {
            match key {
                "left" => {
                    self.canvas_pan.0 += 28.;
                    cx.notify();
                    return;
                }
                "right" => {
                    self.canvas_pan.0 -= 28.;
                    cx.notify();
                    return;
                }
                "up" => {
                    self.canvas_pan.1 += 28.;
                    cx.notify();
                    return;
                }
                "down" => {
                    self.canvas_pan.1 -= 28.;
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }
        if self.layout == ToolCallsLayout::Canvas {
            if key == "=" || key == "+" {
                self.canvas_zoom = (self.canvas_zoom + 0.1).clamp(0.6, 1.6);
                cx.notify();
                return;
            }
            if key == "-" {
                self.canvas_zoom = (self.canvas_zoom - 0.1).clamp(0.6, 1.6);
                cx.notify();
                return;
            }
        }

        let visible = self.visible_calls(cx);
        match key {
            "j" | "down" => {
                if let Some(ToolCallsSelection::Call(cid)) = &self.selection {
                    if let Some(pos) = visible.iter().position(|c| &c.id == cid) {
                        let next_idx = (pos + 1).min(visible.len().saturating_sub(1));
                        if let Some(next_call) = visible.get(next_idx) {
                            self.selection = Some(ToolCallsSelection::Call(next_call.id.clone()));
                            cx.emit(AgentToolCallsEvent::Select(self.selection.clone()));
                            cx.notify();
                        }
                    }
                } else if let Some(first) = visible.first() {
                    self.selection = Some(ToolCallsSelection::Call(first.id.clone()));
                    cx.emit(AgentToolCallsEvent::Select(self.selection.clone()));
                    cx.notify();
                }
            }
            "k" | "up" => {
                if let Some(ToolCallsSelection::Call(cid)) = &self.selection
                    && let Some(pos) = visible.iter().position(|c| &c.id == cid)
                    && pos > 0
                    && let Some(prev_call) = visible.get(pos - 1)
                {
                    self.selection = Some(ToolCallsSelection::Call(prev_call.id.clone()));
                    cx.emit(AgentToolCallsEvent::Select(self.selection.clone()));
                    cx.notify();
                }
            }
            "home" => {
                self.select(Some(ToolCallsSelection::Agent), cx);
            }
            "end" => {
                if let Some(last_tool) = self.tools.last() {
                    self.select(Some(ToolCallsSelection::Tool(last_tool.id.clone())), cx);
                }
            }
            _ => {}
        }
    }

    fn render_status_chip(
        status: ToolCallStatus,
        compact_square: bool,
        spin_rad: f32,
        cx: &App,
    ) -> Div {
        let p = ProPalette::get(cx);
        let (bg, fg) = match status {
            ToolCallStatus::Running => (p.accent_soft, p.blue),
            ToolCallStatus::Waiting => (p.amber_soft, p.amber_ink),
            ToolCallStatus::Done => (p.green_soft, p.green_ink),
            ToolCallStatus::Failed => (p.red_soft, p.red_ink),
            ToolCallStatus::Rejected => (p.chip, p.muted),
        };
        let glyph_side = if compact_square { 10. } else { 11. };
        let glyph = match status {
            ToolCallStatus::Running => render_spinner_arc(glyph_side, fg, spin_rad),
            ToolCallStatus::Waiting => pro_icon(ProIcon::Clock, glyph_side, fg),
            ToolCallStatus::Done => pro_icon(ProIcon::Check, glyph_side, fg),
            ToolCallStatus::Failed => pro_icon(ProIcon::AlertTriangle, glyph_side, fg),
            ToolCallStatus::Rejected => pro_icon(ProIcon::Close, glyph_side, fg),
        };
        if compact_square {
            div()
                .size(px(16.))
                .rounded(px(4.))
                .bg(bg)
                .flex()
                .items_center()
                .justify_center()
                .child(glyph)
        } else {
            div()
                .h(px(20.))
                .pl(px(5.))
                .pr(px(6.))
                .rounded(px(5.))
                .bg(bg)
                .text_color(fg)
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .whitespace_nowrap()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(glyph)
                .child(status.label(cx))
        }
    }

    fn render_paused_chip(compact_square: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        if compact_square {
            div()
                .size(px(16.))
                .rounded(px(4.))
                .bg(p.chip)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_svg("0 0 24 24", SVG_STOP_SQUARE, 10., p.muted))
        } else {
            div()
                .h(px(20.))
                .pl(px(5.))
                .pr(px(6.))
                .rounded(px(5.))
                .bg(p.chip)
                .text_color(p.muted)
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_svg("0 0 24 24", SVG_STOP_SQUARE, 11., p.muted))
                .child(tr(cx, "Paused", "已暂停"))
        }
    }

    fn render_top_bar(&self, narrow: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let env_label = if self.paused {
            tr(cx, "PAUSED", "已暂停")
        } else {
            tr(cx, "LIVE", "实时")
        };

        let range_ctrl: AnyElement = if narrow {
            activate(
                div()
                    .id(pro_eid(&self.id, "crumb-range"))
                    .role(Role::Button)
                    .aria_label(self.range.label(cx))
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.range.label(cx)),
                    )
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                cx,
                |this, _, cx| {
                    this.range_menu_open = !this.range_menu_open;
                    this.agent_menu_open = false;
                    cx.notify();
                },
            )
            .into_any_element()
        } else {
            let rng_sensor = pro_selection_sensor();
            let rng_key: SharedString = format!("{:?}", self.range).into();
            div()
                .id(pro_eid(&self.id, "crumb-range"))
                .relative()
                .h(px(28.))
                .p(px(2.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "range-pill-ind"),
                    rng_key,
                    rng_sensor.clone(),
                    5.,
                    p.card,
                    Some(p.line),
                    true,
                ))
                .children(
                    [
                        (
                            ToolCallsRange::Today,
                            "rng-tab-today",
                            tr(cx, "Today", "今天"),
                        ),
                        (ToolCallsRange::SevenDays, "rng-tab-7d", tr(cx, "7d", "7天")),
                        (
                            ToolCallsRange::ThirtyDays,
                            "rng-tab-30d",
                            tr(cx, "30d", "30天"),
                        ),
                    ]
                    .into_iter()
                    .map(|(rng, tid_s, lbl)| {
                        let active = self.range == rng;
                        let rng_sensor_item = rng_sensor.clone();
                        activate(
                            div()
                                .id(pro_eid(&self.id, tid_s))
                                .role(Role::Tab)
                                .aria_label(lbl)
                                .tab_index(0)
                                .relative()
                                .h(px(24.))
                                .px(px(8.))
                                .rounded(px(5.))
                                .hover(move |s| {
                                    if active {
                                        s.text_color(p.ink)
                                    } else {
                                        s.text_color(p.ink).bg(p.hover)
                                    }
                                })
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .text_size(px(12.))
                                .font_weight(if active {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if active { p.ink } else { p.muted })
                                .cursor_pointer()
                                .when(active, |d| d.child(pro_mark_selected(&rng_sensor_item)))
                                .child(lbl),
                            cx,
                            move |this, _, cx| {
                                this.range = rng;
                                this.range_menu_open = false;
                                cx.notify();
                            },
                        )
                    }),
                )
                .into_any_element()
        };

        let left_crumbs = div()
            .h(px(28.))
            .rounded(px(6.))
            .bg(p.chip)
            .flex()
            .items_center()
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "crumb-agent"))
                    .role(Role::Button)
                    .aria_label(self.agent.name.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.agent.name.clone()),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .px(px(5.))
                                .h(px(16.))
                                .rounded(px(4.))
                                .bg(p.chip_hover)
                                .flex()
                                .items_center()
                                .text_size(px(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if self.paused { p.amber_ink } else { p.muted })
                                .child(env_label),
                        )
                    })
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                cx,
                |this, _, cx| {
                    this.agent_menu_open = !this.agent_menu_open;
                    this.range_menu_open = false;
                    cx.notify();
                },
            ))
            .child(
                div()
                    .px(px(2.))
                    .text_size(px(13.))
                    .text_color(p.faint)
                    .child("/"),
            )
            .child(range_ctrl);

        let connect_label: SharedString = tr(cx, "Connect tool", "连接工具").into();
        let pause_label: SharedString = if self.paused {
            tr(cx, "Resume", "恢复运行").into()
        } else {
            tr(cx, "Pause agent", "暂停智能体").into()
        };

        let connect_btn = activate(
            div()
                .id(pro_eid(&self.id, "top-connect"))
                .role(Role::Button)
                .aria_label(connect_label.clone())
                .tab_index(0)
                .h(px(28.))
                .px(px(if narrow { 7. } else { 9. }))
                .rounded(px(6.))
                .bg(p.chip)
                .hover(move |s| s.bg(p.chip_hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .child(pro_icon(ProIcon::Plus, 14., p.muted))
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(connect_label),
                    )
                }),
            cx,
            |this, _, cx| {
                this.picker_open = true;
                this.agent_menu_open = false;
                this.range_menu_open = false;
                cx.emit(AgentToolCallsEvent::AddTool);
                cx.notify();
            },
        );

        let paused = self.paused;
        let pause_bg = if paused { p.primary } else { p.chip };
        let pause_hover = if paused {
            p.primary.opacity(0.88)
        } else {
            p.chip_hover
        };
        let pause_fg = if paused { p.on_primary } else { p.ink };
        let pause_ic_fg = if paused { p.on_primary } else { p.muted };

        let pause_btn = activate(
            div()
                .id(pro_eid(&self.id, "top-pause"))
                .role(Role::Button)
                .aria_label(pause_label.clone())
                .tab_index(0)
                .h(px(28.))
                .px(px(if narrow { 7. } else { 9. }))
                .rounded(px(6.))
                .bg(pause_bg)
                .hover(move |s| s.bg(pause_hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .child(if paused {
                    pro_icon(ProIcon::Play, 13., pause_ic_fg)
                } else {
                    pro_svg("0 0 24 24", SVG_PAUSE_BARS, 14., pause_ic_fg)
                })
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(pause_fg)
                            .child(pause_label),
                    )
                }),
            cx,
            |this, _, cx| {
                this.paused = !this.paused;
                cx.emit(AgentToolCallsEvent::Pause(this.paused));
                cx.notify();
            },
        );

        div()
            .w_full()
            .px(px(10.))
            .pt(px(10.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(left_crumbs)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(connect_btn)
                    .child(pause_btn),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_org_area(
        &self,
        area_w: f32,
        elapsed_ms: f32,
        spin_rad: f32,
        flow_phase: f32,
        skel_phase: f32,
        reduced: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let dot_color = if p.dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.14)
        };

        let range_calls = self.calls_in_range();
        let metrics = summarize_calls(
            &self
                .calls
                .iter()
                .filter(|c| c.age_days <= self.range.max_age_days())
                .cloned()
                .collect::<Vec<_>>(),
        );

        let agent_selected = matches!(self.selection, Some(ToolCallsSelection::Agent));
        let selected_tool_id: Option<SharedString> = match &self.selection {
            Some(ToolCallsSelection::Tool(tid)) => Some(tid.clone()),
            Some(ToolCallsSelection::Call(cid)) => self
                .calls
                .iter()
                .find(|c| &c.id == cid)
                .map(|c| c.tool_id.clone()),
            _ => None,
        };

        let agent_mark_color: Hsla = if p.dark {
            rgba(0x4d8dffda).into()
        } else {
            rgba(0x0060e2c7).into()
        };
        let agent_busy = metrics.running > 0 && !self.paused && !self.loading && !self.empty;

        let zoom = if self.layout == ToolCallsLayout::Canvas {
            self.canvas_zoom
        } else {
            1.0
        };
        let display_count = if self.empty {
            1usize
        } else if self.loading {
            7usize
        } else {
            self.tools.len().max(1)
        };

        let is_compact_split = self.layout == ToolCallsLayout::Compact
            && area_w >= 580.
            && !self.empty
            && !(self.tools.is_empty() && !self.loading);

        let cols = if area_w >= 720. {
            4usize
        } else if area_w >= 540. {
            3usize
        } else {
            2usize
        };
        let col_gap = 10.0_f32 * zoom;
        let row_gap = 24.0_f32 * zoom;
        let base_tile_w = if is_compact_split {
            ((area_w - 300.) / 2.).clamp(150., 196.)
        } else if cols == 4 {
            ((area_w - 48. - 30.) / 4.).clamp(148., 176.)
        } else if cols == 3 {
            ((area_w - 36. - 20.) / 3.).clamp(144., 180.)
        } else {
            ((area_w - 28. - 10.) / 2.).clamp(132., 200.)
        };
        let tile_w = base_tile_w * zoom;
        let tile_h = (if self.layout == ToolCallsLayout::Canvas {
            76.0_f32
        } else {
            54.0_f32
        }) * zoom;
        let agent_w = (if is_compact_split {
            210.0_f32
        } else {
            (area_w - 32.).clamp(220., 296.)
        }) * zoom;
        let agent_h = 60.0_f32 * zoom;
        let top_gap = 38.0_f32 * zoom;
        let side_gap = 42.0_f32;
        let compact_vgap = 10.0_f32;
        let left_count = display_count.div_ceil(2);
        let right_count = display_count.saturating_sub(left_count);

        let num_rows = display_count.div_ceil(cols);
        let (map_w, map_h, agent_left, agent_top) = if is_compact_split {
            let mw = 2. * tile_w + 2. * side_gap + agent_w;
            let max_col_n = left_count.max(right_count).max(1);
            let mh = (max_col_n as f32 * tile_h
                + max_col_n.saturating_sub(1) as f32 * compact_vgap)
                .max(agent_h);
            (mw, mh, tile_w + side_gap, (mh - agent_h) * 0.5)
        } else {
            let mw = if self.empty || (display_count == 1 && !self.loading) {
                agent_w.max(tile_w)
            } else {
                cols as f32 * tile_w + (cols.saturating_sub(1)) as f32 * col_gap
            };
            let mh = agent_h
                + top_gap
                + num_rows as f32 * tile_h
                + (num_rows.saturating_sub(1)) as f32 * row_gap;
            (mw, mh, (mw - agent_w) * 0.5, 0.)
        };

        let ax = agent_left + agent_w * 0.5;
        let ay = agent_top + agent_h;

        let slot_pos = |slot: usize| -> (f32, f32) {
            if is_compact_split {
                if slot < left_count {
                    let col_h = left_count as f32 * tile_h
                        + left_count.saturating_sub(1) as f32 * compact_vgap;
                    let start_y = (map_h - col_h) * 0.5;
                    (0., start_y + slot as f32 * (tile_h + compact_vgap))
                } else {
                    let ri = slot - left_count;
                    let col_h = right_count as f32 * tile_h
                        + right_count.saturating_sub(1) as f32 * compact_vgap;
                    let start_y = (map_h - col_h) * 0.5;
                    (
                        tile_w + 2. * side_gap + agent_w,
                        start_y + ri as f32 * (tile_h + compact_vgap),
                    )
                }
            } else {
                let r = slot / cols;
                let c = slot % cols;
                let x = if self.empty
                    || (display_count == 1 && !self.loading && self.tools.is_empty())
                {
                    (map_w - tile_w) * 0.5
                } else {
                    c as f32 * (tile_w + col_gap)
                };
                let y = agent_h + top_gap + r as f32 * (tile_h + row_gap);
                (x, y)
            }
        };

        // Precompute connector descriptor per slot for the canvas behind cards
        #[derive(Clone)]
        struct LinkSpec {
            pts: Vec<(f32, f32)>,
            muted: bool,
            faded: bool,
            dashed: bool,
            flowing: bool,
        }

        let mut links: Vec<LinkSpec> = Vec::with_capacity(display_count);
        for slot in 0..display_count {
            let (sx, sy) = slot_pos(slot);
            let pts = if is_compact_split {
                let agent_cy = agent_top + agent_h * 0.5;
                if slot < left_count {
                    build_horiz_ortho_points(
                        agent_left,
                        agent_cy,
                        agent_left - side_gap * 0.5,
                        sx + tile_w,
                        sy + tile_h * 0.5,
                    )
                } else {
                    let ar = agent_left + agent_w;
                    build_horiz_ortho_points(
                        ar,
                        agent_cy,
                        ar + side_gap * 0.5,
                        sx,
                        sy + tile_h * 0.5,
                    )
                }
            } else {
                let tx = sx + tile_w * 0.5;
                let ty = sy;
                build_ortho_points(ax, ay, ty - 12. * zoom, tx, ty)
            };

            if self.empty || (self.tools.is_empty() && !self.loading) {
                links.push(LinkSpec {
                    pts,
                    muted: true,
                    faded: false,
                    dashed: true,
                    flowing: false,
                });
            } else if self.loading {
                links.push(LinkSpec {
                    pts,
                    muted: true,
                    faded: false,
                    dashed: false,
                    flowing: false,
                });
            } else if let Some(tool) = self.tools.get(slot) {
                let st = self.tool_live_status(tool);
                let faded = selected_tool_id.as_ref().is_some_and(|sel| sel != &tool.id);
                links.push(LinkSpec {
                    pts,
                    muted: tool.mode == ToolAccessMode::Paused,
                    faded,
                    dashed: false,
                    flowing: st == Some(ToolCallStatus::Running)
                        && !self.paused
                        && self.live
                        && !reduced,
                });
            }
        }

        let link_blue: Hsla = if p.dark {
            p.blue.opacity(0.62)
        } else {
            rgb(0x83c7fe).into()
        };
        let link_muted: Hsla = if p.dark {
            hsla(0., 0., 1., 0.14)
        } else {
            hsla(0., 0., 0., 0.12)
        };
        let flow_color: Hsla = if p.dark {
            rgb(0x90cdfe).into()
        } else {
            rgb(0x0070d6).into()
        };
        let port_faded = selected_tool_id.is_some() && !agent_selected;
        let port_rim = if port_faded {
            link_blue.opacity(0.4)
        } else {
            link_blue
        };

        let links_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let origin = bounds.origin;
                // Draw muted links first so active blue connectors sit cleanly on top
                for pass_active in [false, true] {
                    for lk in &links {
                        let is_active = !lk.muted;
                        if is_active != pass_active {
                            continue;
                        }
                        let mut stroke_c = if lk.muted { link_muted } else { link_blue };
                        if lk.faded {
                            stroke_c = stroke_c.opacity(stroke_c.a * 0.35);
                        }
                        let pts = &lk.pts;
                        if lk.dashed {
                            for seg in 0..8 {
                                let t0 = seg as f32 / 8.;
                                let t1 = t0 + 0.07;
                                paint_polyline_slice(origin, pts, t0, t1, 1.5, stroke_c, window);
                            }
                        } else {
                            paint_polyline_slice(origin, pts, 0., 1., 1.5, stroke_c, window);
                        }
                        if lk.flowing {
                            let head = flow_phase;
                            let tail = (flow_phase - 0.28).max(0.);
                            paint_polyline_slice(origin, pts, tail, head, 2.3, flow_color, window);
                        }
                    }
                }
            },
        )
        .absolute()
        .inset_0()
        .size_full();

        // Build top Agent card
        let agent_card: AnyElement = if self.loading {
            div()
                .absolute()
                .left(px(agent_left))
                .top(px(agent_top))
                .w(px(agent_w))
                .h(px(agent_h))
                .px(px(12.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(10.))
                .child(pro_skeleton_bar(30., 30., 8., skel_phase, cx))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(86., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(62., 10., 4., skel_phase, cx)),
                )
                .child(pro_skeleton_bar(56., 18., 5., skel_phase, cx))
                .into_any_element()
        } else {
            let agent_status_chip: Option<Div> = if self.empty || self.tools.is_empty() {
                None
            } else if self.paused {
                Some(Self::render_paused_chip(false, cx))
            } else if metrics.waiting > 0 {
                Some(Self::render_status_chip(
                    ToolCallStatus::Waiting,
                    false,
                    spin_rad,
                    cx,
                ))
            } else if metrics.running > 0 {
                Some(Self::render_status_chip(
                    ToolCallStatus::Running,
                    false,
                    spin_rad,
                    cx,
                ))
            } else {
                Some(Self::render_status_chip(
                    ToolCallStatus::Done,
                    false,
                    spin_rad,
                    cx,
                ))
            };
            let agent_faded = selected_tool_id.is_some() && !agent_selected;
            activate(
                div()
                    .id(pro_eid(&self.id, "agent-node"))
                    .role(Role::Button)
                    .aria_label(self.agent.name.clone())
                    .tab_index(0)
                    .absolute()
                    .left(px(agent_left))
                    .top(px(agent_top))
                    .w(px(agent_w))
                    .h(px(agent_h))
                    .px(px(12.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(if agent_selected { p.focus } else { p.line })
                    .when(agent_faded, |d| d.opacity(0.45))
                    .hover(move |s| s.border_color(p.line_strong))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .child(pro_agent_avatar(
                        &self.agent.id,
                        agent_mark_color,
                        30.,
                        agent_busy,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(19.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(self.agent.name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .min_w_0()
                                    .when_some(
                                        pro_app_artwork(&self.agent.maker, 11., cx),
                                        |d, m| d.child(m),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(15.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(self.agent.model.clone()),
                                    ),
                            ),
                    )
                    .when_some(agent_status_chip, |d, chip| d.child(chip)),
                cx,
                |this, _, cx| this.select(Some(ToolCallsSelection::Agent), cx),
            )
            .into_any_element()
        };

        let mut map_box = div()
            .relative()
            .w(px(map_w))
            .h(px(map_h))
            .when(self.layout == ToolCallsLayout::Canvas, |d| {
                d.ml(px(self.canvas_pan.0)).mt(px(self.canvas_pan.1))
            })
            .child(links_canvas)
            .child(agent_card)
            .when(!is_compact_split, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(ax - 3.5))
                        .top(px(ay - 3.5))
                        .size(px(7.))
                        .rounded_full()
                        .bg(p.card)
                        .border_1()
                        .border_color(port_rim),
                )
            });

        if self.empty || (self.tools.is_empty() && !self.loading) {
            let gx = (map_w - tile_w) * 0.5;
            let gy = agent_h + top_gap;
            map_box = map_box.child(activate(
                div()
                    .id(pro_eid(&self.id, "ghost-connect-first"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Connect first tool", "连接首个工具"))
                    .tab_index(0)
                    .absolute()
                    .left(px(gx))
                    .top(px(gy))
                    .w(px(tile_w.max(210.)))
                    .h(px(tile_h))
                    .px(px(12.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(p.line_strong)
                    .bg(p.panel)
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(28.))
                            .rounded(px(7.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Plus, 14., p.muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Connect tool", "连接工具")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(tr(
                                        cx,
                                        "Give this agent its first tool",
                                        "为该智能体添加首个工具",
                                    )),
                            ),
                    ),
                cx,
                |this, _, cx| {
                    this.picker_open = true;
                    cx.emit(AgentToolCallsEvent::AddTool);
                    cx.notify();
                },
            ));
        } else if self.loading {
            for slot in 0..7usize {
                let (x, y) = slot_pos(slot);
                map_box = map_box.child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(tile_w))
                        .h(px(tile_h))
                        .px(px(10.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(28., 28., 7., skel_phase, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(pro_skeleton_bar(58., 11., 4., skel_phase, cx))
                                .child(pro_skeleton_bar(84., 9., 4., skel_phase, cx)),
                        ),
                );
            }
        } else {
            for (slot, tool) in self.tools.iter().cloned().enumerate() {
                let (x, y) = slot_pos(slot);

                let tool_calls: Vec<&ToolCallRecord> = range_calls
                    .iter()
                    .copied()
                    .filter(|call| call.tool_id == tool.id)
                    .collect();
                let call_n = tool_calls.len();
                let mut dur_sum: u64 = 0;
                let mut dur_cnt: u64 = 0;
                for call in &tool_calls {
                    if call.status != ToolCallStatus::Running
                        && let Some(ms) = call.duration_ms
                    {
                        dur_sum += u64::from(ms);
                        dur_cnt += 1;
                    }
                }
                let avg_ms = (dur_cnt > 0).then(|| (dur_sum / dur_cnt) as u32);
                let live_st = self.tool_live_status(&tool);
                let is_selected = selected_tool_id.as_ref() == Some(&tool.id);
                let is_faded = selected_tool_id.as_ref().is_some_and(|sel| sel != &tool.id);
                let is_off = tool.mode == ToolAccessMode::Paused;

                let corner_badge: Option<Div> = if is_off {
                    Some(Self::render_paused_chip(true, cx))
                } else {
                    live_st.map(|st| Self::render_status_chip(st, true, spin_rad, cx))
                };

                let calls_str = if call_n == 0 {
                    tr(cx, "No calls", "无调用").to_owned()
                } else if call_n == 1 {
                    format!("1 {}", tr(cx, "call", "次调用"))
                } else {
                    format!("{call_n} {}", tr(cx, "calls", "次调用"))
                };

                let secondary_meta: Option<(bool, String)> = if call_n == 0 {
                    None
                } else if let Some(avg) = avg_ms {
                    Some((true, format_duration_ms(avg)))
                } else if tool.mode == ToolAccessMode::AskFirst {
                    Some((false, tr(cx, "Ask first", "需确认").to_owned()))
                } else {
                    None
                };

                let tid = tool.id.clone();
                let tile_opacity = if is_faded && is_off {
                    0.28
                } else if is_faded {
                    0.38
                } else if is_off {
                    0.60
                } else {
                    1.0
                };

                map_box = map_box.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("tool-tile-{tid}")))
                        .role(Role::Button)
                        .aria_label(format!("{}, {}", tool.name, tool.source))
                        .tab_index(0)
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(tile_w))
                        .h(px(tile_h))
                        .px(px(10.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(if is_selected { p.focus } else { p.line })
                        .opacity(tile_opacity)
                        .hover(move |s| s.border_color(p.line_strong))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .cursor_pointer()
                        .child(render_tool_tile(&tool.app, 28., cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .pr(px(if corner_badge.is_some() { 14. } else { 0. }))
                                        .text_size(px(13.))
                                        .line_height(px(17.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(tool.name.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .min_w_0()
                                        .text_size(px(11.5))
                                        .line_height(px(15.))
                                        .text_color(p.muted)
                                        .child(
                                            div()
                                                .flex_none()
                                                .flex()
                                                .items_center()
                                                .gap(px(3.))
                                                .child(pro_svg(
                                                    "0 0 24 24",
                                                    SVG_CALLS_SWAP,
                                                    11.,
                                                    p.faint,
                                                ))
                                                .child(calls_str),
                                        )
                                        .when_some(secondary_meta, |d, (has_clock, txt)| {
                                            d.child(
                                                div()
                                                    .min_w_0()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(2.5))
                                                    .when(has_clock, |inner| {
                                                        inner.child(pro_icon(
                                                            ProIcon::Clock,
                                                            10.5,
                                                            p.faint,
                                                        ))
                                                    })
                                                    .child(div().truncate().child(txt)),
                                            )
                                        }),
                                )
                                .when(self.layout == ToolCallsLayout::Canvas, |d| {
                                    d.child(
                                        div()
                                            .font_family("Fira Code")
                                            .text_size(px(10.5))
                                            .text_color(p.faint)
                                            .truncate()
                                            .child(tool.fn_name.clone()),
                                    )
                                }),
                        )
                        .when_some(corner_badge, |d, badge| {
                            d.child(div().absolute().top(px(6.)).right(px(6.)).child(badge))
                        }),
                    cx,
                    move |this, _, cx| {
                        this.tool_panel_tab = ToolPanelTab::Overview;
                        this.select(Some(ToolCallsSelection::Tool(tid.clone())), cx);
                    },
                ));
            }
        }

        let dots_bg = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                let step = 20.0_f32;
                let mut y = 10.0_f32;
                while y < h {
                    let mut x = 10.0_f32;
                    while x < w {
                        let rect = Bounds::new(
                            bounds.origin + point(px(x), px(y)),
                            size(px(1.5), px(1.5)),
                        );
                        window.paint_quad(fill(rect, dot_color).corner_radii(px(0.75)));
                        x += step;
                    }
                    y += step;
                }
            },
        )
        .absolute()
        .inset_0()
        .size_full();

        let narrow = area_w < 540.;
        let org_vtx_key: SharedString = format!("{:?}-{}", self.layout, self.empty).into();
        div()
            .relative()
            .flex_1()
            .min_h(px(220.))
            .w_full()
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(dots_bg)
            .child(self.render_top_bar(narrow, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .px(px(16.))
                    .py(px(14.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "org-layout-vtx"),
                        org_vtx_key,
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(map_box),
                    )),
            )
            .when(self.layout == ToolCallsLayout::Canvas, |d| {
                let zoom_pct = format!("{}%", (self.canvas_zoom * 100.).round() as u32);
                d.child(
                    div()
                        .absolute()
                        .bottom(px(12.))
                        .left(relative(0.5))
                        .ml(px(-66.))
                        .p(px(4.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "zoom-out"),
                            tr(cx, "Zoom out", "缩小"),
                            ProIcon::Minus,
                            26.,
                            false,
                            cx,
                            |this, _, cx| {
                                this.canvas_zoom = (this.canvas_zoom - 0.1).clamp(0.6, 1.6);
                                cx.notify();
                            },
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "zoom-reset"))
                                .role(Role::Button)
                                .tab_index(0)
                                .px(px(6.))
                                .h(px(26.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .child(zoom_pct),
                            cx,
                            |this, _, cx| {
                                this.canvas_zoom = 1.0;
                                this.canvas_pan = (0., 0.);
                                cx.notify();
                            },
                        ))
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "zoom-in"),
                            tr(cx, "Zoom in", "放大"),
                            ProIcon::Plus,
                            26.,
                            false,
                            cx,
                            |this, _, cx| {
                                this.canvas_zoom = (this.canvas_zoom + 0.1).clamp(0.6, 1.6);
                                cx.notify();
                            },
                        )),
                )
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_run_section(
        &self,
        area_w: f32,
        elapsed_ms: f32,
        spin_rad: f32,
        skel_phase: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let range_slice: Vec<ToolCallRecord> = self
            .calls
            .iter()
            .filter(|c| c.age_days <= self.range.max_age_days())
            .cloned()
            .collect();
        let metrics = summarize_calls(&range_slice);
        let visible = self.visible_calls(cx);

        let show_run_col = area_w >= 820.;
        let show_tokens_col = area_w >= 660.;
        let show_input_col = area_w >= 500.;
        let narrow = area_w < 540.;

        // Tabs: Tool calls | Failed <count>
        let all_active = self.log_tab == LogTab::All;
        let failed_active = self.log_tab == LogTab::Failed;
        let log_sensor = pro_selection_sensor();
        let log_sensor_all = log_sensor.clone();
        let log_sensor_failed = log_sensor.clone();
        let log_tab_key: SharedString = match self.log_tab {
            LogTab::All => "all".into(),
            LogTab::Failed => "failed".into(),
        };

        let tabs = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "log-tab-ind"),
                        log_tab_key,
                        log_sensor,
                        6.,
                        p.chip,
                        None,
                        false,
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "tab-all"))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .hover(move |s| {
                                if all_active {
                                    s.text_color(p.ink)
                                } else {
                                    s.text_color(p.ink).bg(p.hover)
                                }
                            })
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .text_color(if all_active { p.ink } else { p.muted })
                            .cursor_pointer()
                            .when(all_active, |d| d.child(pro_mark_selected(&log_sensor_all)))
                            .child(pro_svg(
                                "0 0 24 24",
                                SVG_WRENCH,
                                14.,
                                if all_active { p.ink } else { p.muted },
                            ))
                            .when(!narrow, |d| d.child(tr(cx, "Tool calls", "工具调用"))),
                        cx,
                        |this, _, cx| {
                            this.log_tab = LogTab::All;
                            cx.notify();
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "tab-failed"))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .hover(move |s| {
                                if failed_active {
                                    s.text_color(p.ink)
                                } else {
                                    s.text_color(p.ink).bg(p.hover)
                                }
                            })
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .text_color(if failed_active { p.ink } else { p.muted })
                            .cursor_pointer()
                            .when(failed_active, |d| {
                                d.child(pro_mark_selected(&log_sensor_failed))
                            })
                            .child(pro_icon(
                                ProIcon::AlertTriangle,
                                14.,
                                if failed_active { p.ink } else { p.muted },
                            ))
                            .when(!narrow, |d| d.child(tr(cx, "Failed", "失败")))
                            .when(!self.loading && !self.empty && metrics.failed > 0, |d| {
                                d.child(
                                    div()
                                        .min_w(px(16.))
                                        .px(px(4.))
                                        .h(px(16.))
                                        .rounded(px(4.))
                                        .bg(p.red_soft)
                                        .text_color(p.red_ink)
                                        .text_size(px(11.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(metrics.failed.to_string()),
                                )
                            }),
                        cx,
                        |this, _, cx| {
                            this.log_tab = LogTab::Failed;
                            cx.notify();
                        },
                    )),
            )
            .when_some(self.tool_filter.clone(), |d, tid| {
                let label = self.find_tool(&tid).map(|t| t.name.clone()).unwrap_or(tid);
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "clear-tool-filter"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(24.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .hover(move |s| s.bg(p.chip_hover))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(label)
                        .child(pro_icon(ProIcon::Close, 11., p.muted)),
                    cx,
                    |this, _, cx| {
                        this.tool_filter = None;
                        cx.notify();
                    },
                ))
            });

        let search_ctrl: AnyElement = if self.search_open {
            div()
                .w(px(if narrow { 120. } else { 176. }))
                .h(px(28.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(p.chip)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(6.))
                .child(pro_icon(ProIcon::Search, 13., p.muted))
                .when_some(self.search_editor.clone(), |d, ed| {
                    d.child(div().flex_1().min_w_0().child(ed))
                })
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "close-search"))
                        .role(Role::Button)
                        .tab_index(0)
                        .size(px(18.))
                        .rounded(px(4.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Close, 11., p.faint)),
                    cx,
                    |this, w, cx| {
                        if let Some(ed) = &this.search_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        this.search_open = false;
                        cx.notify();
                    },
                ))
                .into_any_element()
        } else {
            pro_icon_btn(
                pro_eid(&self.id, "open-search"),
                tr(cx, "Search tool calls", "搜索工具调用"),
                ProIcon::Search,
                28.,
                false,
                cx,
                |this, w, cx| {
                    this.search_open = true;
                    if let Some(ed) = &this.search_editor {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    cx.notify();
                },
            )
            .into_any_element()
        };

        let summary_el = if self.loading {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(14.))
                .child(pro_skeleton_bar(32., 11., 4., skel_phase, cx))
                .child(pro_skeleton_bar(38., 11., 4., skel_phase, cx))
                .child(pro_skeleton_bar(42., 11., 4., skel_phase, cx))
        } else if self.empty {
            div().flex_1()
        } else {
            let avg_text = metrics
                .avg_duration_ms
                .map(format_duration_ms)
                .unwrap_or_else(|| "–".to_owned());
            let tok_text = format_tokens_compact(metrics.total_tokens);
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(if narrow { 10. } else { 16. }))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(pro_svg("0 0 24 24", SVG_WRENCH, 13., p.faint))
                        .child(metrics.total.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(pro_svg("0 0 24 24", SVG_CHECK_CIRCLE, 13., p.faint))
                        .child(format!("{}%", metrics.success_pct)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .when(!narrow, |d| d.child(pro_icon(ProIcon::Clock, 13., p.faint)))
                        .child(avg_text),
                )
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tok_text),
                            )
                            .child(tr(cx, "tokens", "词元")),
                    )
                })
        };

        let target_run_h = if self.run_folded {
            44.
        } else {
            self.run_height
        };
        let run_h = if self.resizing_run.is_some() {
            target_run_h
        } else {
            pro_spring_f32(pro_eid(&self.id, "run-sec-h"), target_run_h, 0.22, w, cx)
        };
        let cur_h = self.run_height;

        let mut section = div()
            .relative()
            .w_full()
            .h(px(run_h))
            .flex_none()
            .bg(p.panel)
            .border_t_1()
            .border_color(p.line_strong)
            .overflow_hidden()
            .flex()
            .flex_col()
            .when(!self.run_folded, |d| {
                d.child(
                    div()
                        .id(pro_eid(&self.id, "run-resize-grip"))
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(5.))
                        .cursor_row_resize()
                        .hover(move |s| s.bg(p.line_strong))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                this.resizing_run = Some((f32::from(ev.position.y), cur_h));
                                cx.notify();
                            }),
                        ),
                )
            })
            .child(
                div()
                    .h(px(44.))
                    .pl(px(10.))
                    .pr(px(8.))
                    .flex_none()
                    .when(run_h > 45.0, |d| d.border_b_1().border_color(p.line))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(tabs)
                    .child(search_ctrl)
                    .child(summary_el)
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "fold-run"),
                        if self.run_folded {
                            tr(cx, "Expand tool calls", "展开调用日志")
                        } else {
                            tr(cx, "Fold tool calls", "折叠调用日志")
                        },
                        if self.run_folded {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        28.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.run_folded = !this.run_folded;
                            cx.notify();
                        },
                    )),
            );

        if self.run_folded && run_h <= 45.0 {
            return section;
        }

        // Table Header + Rows
        if self.empty {
            section = section.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .id("pro-scroll-2945")
                    .overflow_y_scroll()
                    .child(pro_empty_state(
                        ProIcon::Sliders,
                        tr(cx, "No tool calls yet", "尚无工具调用记录"),
                        tr(
                            cx,
                            "Connect a workspace tool so this agent can search, read runbooks, and take actions.",
                            "连接工作区工具后，智能体在此发起的每次调用都会记录其输入与返回结果。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-connect-cta"),
                            tr(cx, "Connect tool", "连接工具").into(),
                            Box::new(|this: &mut Self, _, cx| {
                                this.empty = false;
                                this.picker_open = true;
                                cx.emit(AgentToolCallsEvent::AddTool);
                                cx.notify();
                            }),
                        )),
                        cx,
                    )),
            );
            return section;
        }

        let thead = div()
            .h(px(28.))
            .w_full()
            .flex_none()
            .bg(p.panel)
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .text_size(px(10.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted)
            .child(
                div()
                    .w(px(if narrow { 78. } else { 104. }))
                    .pl(px(16.))
                    .pr(px(8.))
                    .flex_none()
                    .child(tr(cx, "TIME", "时间")),
            )
            .child(
                div()
                    .w(px(if narrow { 116. } else { 142. }))
                    .px(px(8.))
                    .flex_none()
                    .child(tr(cx, "TOOL", "工具")),
            )
            .when(show_input_col, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px(px(8.))
                        .child(tr(cx, "INPUT", "输入参数")),
                )
            })
            .child(
                div()
                    .w(px(108.))
                    .px(px(8.))
                    .flex_none()
                    .child(tr(cx, "STATUS", "状态")),
            )
            .child(
                div()
                    .w(px(76.))
                    .px(px(8.))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .child(tr(cx, "DURATION", "耗时")),
            )
            .when(show_tokens_col, |d| {
                d.child(
                    div()
                        .w(px(70.))
                        .px(px(8.))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .child(tr(cx, "TOKENS", "词元")),
                )
            })
            .when(show_run_col, |d| {
                d.child(
                    div()
                        .w(px(114.))
                        .pl(px(8.))
                        .pr(px(16.))
                        .flex_none()
                        .child(tr(cx, "RUN", "运行 ID")),
                )
            });

        let mut tbody = div()
            .id(pro_eid(&self.id, "calls-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col();

        if self.loading {
            for r in 0..6usize {
                tbody = tbody.child(
                    div()
                        .h(px(34.))
                        .w_full()
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .w(px(if narrow { 78. } else { 104. }))
                                .pl(px(16.))
                                .pr(px(8.))
                                .flex_none()
                                .child(pro_skeleton_bar(54., 10., 3., skel_phase, cx)),
                        )
                        .child(
                            div()
                                .w(px(if narrow { 116. } else { 142. }))
                                .px(px(8.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(20., 20., 5., skel_phase, cx))
                                .child(pro_skeleton_bar(48., 11., 3., skel_phase, cx)),
                        )
                        .when(show_input_col, |d| {
                            d.child(div().flex_1().min_w_0().px(px(8.)).child(pro_skeleton_bar(
                                if r % 2 == 0 { 220. } else { 164. },
                                10.,
                                3.,
                                skel_phase,
                                cx,
                            )))
                        })
                        .child(
                            div()
                                .w(px(108.))
                                .px(px(8.))
                                .flex_none()
                                .child(pro_skeleton_bar(62., 18., 5., skel_phase, cx)),
                        )
                        .child(
                            div()
                                .w(px(76.))
                                .px(px(8.))
                                .flex_none()
                                .flex()
                                .justify_end()
                                .child(pro_skeleton_bar(36., 10., 3., skel_phase, cx)),
                        )
                        .when(show_tokens_col, |d| {
                            d.child(
                                div()
                                    .w(px(70.))
                                    .px(px(8.))
                                    .flex_none()
                                    .flex()
                                    .justify_end()
                                    .child(pro_skeleton_bar(30., 10., 3., skel_phase, cx)),
                            )
                        })
                        .when(show_run_col, |d| {
                            d.child(
                                div()
                                    .w(px(114.))
                                    .pl(px(8.))
                                    .pr(px(16.))
                                    .flex_none()
                                    .child(pro_skeleton_bar(68., 10., 3., skel_phase, cx)),
                            )
                        }),
                );
            }
        } else if visible.is_empty() {
            tbody = tbody.child(
                div()
                    .py(px(28.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(cx, "No matching tool calls", "没有匹配的工具调用")),
            );
        } else {
            let selected_call_id = match &self.selection {
                Some(ToolCallsSelection::Call(cid)) => Some(cid.clone()),
                _ => None,
            };
            let selected_tool_only = match &self.selection {
                Some(ToolCallsSelection::Tool(tid)) => Some(tid.clone()),
                _ => None,
            };

            let mut rows_wrap = div().w_full().flex().flex_col();
            for call in visible {
                let is_open = selected_call_id.as_ref() == Some(&call.id);
                let row_faded = selected_tool_only
                    .as_ref()
                    .is_some_and(|tid| tid != &call.tool_id);
                let (tool_label, tool_app): (SharedString, SharedString) =
                    if let Some(t) = self.find_tool(&call.tool_id) {
                        (t.name.clone(), t.app.clone())
                    } else {
                        let fallback: SharedString = call.tool_id.clone();
                        (fallback.clone(), fallback)
                    };

                let dur_str = if call.status == ToolCallStatus::Running {
                    let base = call.duration_ms.unwrap_or(3000);
                    let live_ms = if self.live && !self.paused {
                        base.saturating_add((elapsed_ms as u32) % 60_000)
                    } else {
                        base
                    };
                    format_duration_ms(live_ms)
                } else {
                    call.duration_ms
                        .map(format_duration_ms)
                        .unwrap_or_else(|| "–".to_owned())
                };

                let tok_str = call
                    .tokens
                    .map(format_tokens_compact)
                    .unwrap_or_else(|| "–".to_owned());

                let cid = call.id.clone();
                let open_bg = p.accent_soft.opacity(if p.dark { 0.55 } else { 0.65 });

                rows_wrap = rows_wrap.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("call-row-{cid}")))
                        .role(Role::Row)
                        .aria_label(format!(
                            "{}, {}, {}",
                            call.timestamp,
                            tool_label,
                            call.status.label(cx)
                        ))
                        .tab_index(0)
                        .h(px(34.))
                        .w_full()
                        .flex_none()
                        .when(is_open, |d| d.bg(open_bg))
                        .when(row_faded, |d| d.opacity(0.42))
                        .hover(move |s| s.bg(if is_open { open_bg } else { p.hover }))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .child(
                            div()
                                .w(px(if narrow { 78. } else { 104. }))
                                .pl(px(16.))
                                .pr(px(8.))
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .truncate()
                                .child(call.timestamp.clone()),
                        )
                        .child(
                            div()
                                .w(px(if narrow { 116. } else { 142. }))
                                .px(px(8.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .child(render_tool_tile(&tool_app, 20., cx))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(tool_label),
                                ),
                        )
                        .when(show_input_col, |d| {
                            d.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .px(px(8.))
                                    .font_family("Fira Code")
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(call.input_summary.clone()),
                            )
                        })
                        .child(
                            div()
                                .w(px(108.))
                                .px(px(8.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .child(Self::render_status_chip(call.status, false, spin_rad, cx)),
                        )
                        .child(
                            div()
                                .w(px(76.))
                                .px(px(8.))
                                .flex_none()
                                .flex()
                                .justify_end()
                                .text_size(px(12.5))
                                .text_color(p.text)
                                .child(dur_str),
                        )
                        .when(show_tokens_col, |d| {
                            d.child(
                                div()
                                    .w(px(70.))
                                    .px(px(8.))
                                    .flex_none()
                                    .flex()
                                    .justify_end()
                                    .text_size(px(12.5))
                                    .text_color(p.text)
                                    .child(tok_str),
                            )
                        })
                        .when(show_run_col, |d| {
                            d.child(
                                div()
                                    .w(px(114.))
                                    .pl(px(8.))
                                    .pr(px(16.))
                                    .flex_none()
                                    .font_family("Fira Code")
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(call.run_id.clone()),
                            )
                        }),
                    cx,
                    move |this, _, cx| {
                        this.select(Some(ToolCallsSelection::Call(cid.clone())), cx);
                    },
                ));
            }
            let log_vtx_key: SharedString =
                format!("{:?}-{:?}-{:?}", self.range, self.log_tab, self.tool_filter).into();
            tbody = tbody.child(pro_view_transition(
                pro_eid(&self.id, "calls-tbody-vtx"),
                log_vtx_key,
                rows_wrap,
            ));
        }

        section.child(thead).child(tbody)
    }

    fn render_floating_panel(
        &self,
        area_w: f32,
        elapsed_ms: f32,
        spin_rad: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let panel_open = self.selection.is_some();
        let active_sel = self
            .selection
            .clone()
            .or_else(|| self.last_selection.clone());
        let p = ProPalette::get(cx);
        let is_sheet = area_w < 640.;

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "panel-close"),
            tr(cx, "Close inspector", "关闭检查器"),
            ProIcon::Close,
            28.,
            false,
            cx,
            |this, _, cx| this.select(None, cx),
        );

        let range_all_calls: Vec<ToolCallRecord> = self
            .calls
            .iter()
            .filter(|c| c.age_days <= self.range.max_age_days())
            .cloned()
            .collect();
        let max_tool_vol = self
            .tools
            .iter()
            .map(|t| range_all_calls.iter().filter(|c| c.tool_id == t.id).count())
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        let max_call_dur = range_all_calls
            .iter()
            .filter_map(|c| c.duration_ms)
            .max()
            .unwrap_or(2500)
            .max(1) as f32;

        let sel_vtx_key: SharedString = match &active_sel {
            Some(ToolCallsSelection::Agent) => "agent".into(),
            Some(ToolCallsSelection::Tool(tid)) => format!("tool:{tid}").into(),
            Some(ToolCallsSelection::Call(cid)) => format!("call:{cid}").into(),
            None => "none".into(),
        };

        let panel_opt: Option<Div> = active_sel.and_then(|selection| match selection {
            ToolCallsSelection::Agent => {
                let range_slice = range_all_calls.clone();
                let metrics = summarize_calls(&range_slice);
                let agent_mark_color: Hsla = if p.dark {
                    rgba(0x4d8dffda).into()
                } else {
                    rgba(0x0060e2c7).into()
                };
                let waiting_call = self
                    .calls
                    .iter()
                    .find(|c| c.status == ToolCallStatus::Waiting)
                    .cloned();

                let avg_str = metrics
                    .avg_duration_ms
                    .map(format_duration_ms)
                    .unwrap_or_else(|| "–".to_owned());

                Some(
                    div()
                        .size_full()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(56.))
                                .pl(px(12.))
                                .pr(px(8.))
                                .border_b_1()
                                .border_color(p.line)
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(pro_agent_avatar(
                                    &self.agent.id,
                                    agent_mark_color,
                                    30.,
                                    metrics.running > 0 && !self.paused,
                                    elapsed_ms,
                                    cx,
                                ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(self.agent.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(format!(
                                                "{} · {}",
                                                self.agent.model, self.agent.maker
                                            )),
                                    ),
                            )
                            .child(close_btn),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "panel-agent-body"))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.ink)
                                    .child(self.agent.task.clone()),
                            )
                            .when_some(waiting_call, |d, wc| {
                                let wid = wc.id.clone();
                                let wid2 = wc.id.clone();
                                d.child(
                                    div()
                                        .p(px(11.))
                                        .rounded(px(8.))
                                        .bg(p.amber_soft)
                                        .border_1()
                                        .border_color(p.amber.opacity(0.28))
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.amber_ink)
                                                .child(pro_icon(ProIcon::Clock, 13., p.amber_ink))
                                                .child(tr(
                                                    cx,
                                                    "Waiting for your approval",
                                                    "等待你的审批",
                                                )),
                                        )
                                        .child(
                                            div()
                                                .font_family("Fira Code")
                                                .text_size(px(11.5))
                                                .line_height(px(17.))
                                                .text_color(p.ink)
                                                .child(wc.input_summary.clone()),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .justify_end()
                                                .gap(px(6.))
                                                .child(pro_btn(
                                                    pro_eid(&self.id, "agent-reject"),
                                                    tr(cx, "Reject", "拒绝"),
                                                    None,
                                                    ProBtnStyle::Secondary,
                                                    true,
                                                    cx,
                                                    move |this, _, cx| {
                                                        this.resolve_call(&wid, false, cx);
                                                    },
                                                ))
                                                .child(pro_btn(
                                                    pro_eid(&self.id, "agent-approve"),
                                                    tr(cx, "Approve", "批准"),
                                                    Some(ProIcon::Check),
                                                    ProBtnStyle::Primary,
                                                    true,
                                                    cx,
                                                    move |this, _, cx| {
                                                        this.resolve_call(&wid2, true, cx);
                                                    },
                                                )),
                                        ),
                                )
                            })
                            .child(self.render_stats_quad(
                                [
                                    (tr(cx, "CALLS", "调用数"), metrics.total.to_string()),
                                    (
                                        tr(cx, "SUCCEEDED", "成功率"),
                                        format!("{}%", metrics.success_pct),
                                    ),
                                    (tr(cx, "AVG TIME", "平均耗时"), avg_str),
                                    (
                                        tr(cx, "TOKENS", "词元"),
                                        format_tokens_compact(metrics.total_tokens),
                                    ),
                                ],
                                cx,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.muted)
                                                    .child(format!(
                                                        "{} · {}",
                                                        tr(cx, "TOOLS", "已连接工具"),
                                                        self.tools.len()
                                                    )),
                                            )
                                            .child(pro_btn(
                                                pro_eid(&self.id, "agent-panel-connect"),
                                                tr(cx, "Connect tool", "连接工具"),
                                                Some(ProIcon::Plus),
                                                ProBtnStyle::Ghost,
                                                true,
                                                cx,
                                                |this, _, cx| {
                                                    this.picker_open = true;
                                                    cx.emit(AgentToolCallsEvent::AddTool);
                                                    cx.notify();
                                                },
                                            )),
                                    )
                                    .children(self.tools.iter().map(|t| {
                                        let tid = t.id.clone();
                                        let t_vol = range_slice
                                            .iter()
                                            .filter(|c| c.tool_id == t.id)
                                            .count();
                                        let vol_ratio = (t_vol as f32 / max_tool_vol).clamp(0., 1.);
                                        let bar_fill = if t.mode == ToolAccessMode::Paused {
                                            p.muted
                                        } else {
                                            p.blue
                                        };
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("ag-tool-{tid}")))
                                                .role(Role::Button)
                                                .tab_index(0)
                                                .h(px(32.))
                                                .px(px(6.))
                                                .rounded(px(7.))
                                                .hover(move |s| s.bg(p.hover))
                                                .flex()
                                                .items_center()
                                                .gap(px(9.))
                                                .cursor_pointer()
                                                .child(render_tool_tile(&t.app, 20., cx))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .text_size(px(13.))
                                                        .text_color(p.ink)
                                                        .truncate()
                                                        .child(t.name.clone()),
                                                )
                                                .child(div().w(px(42.)).flex_none().child(
                                                    pro_spring_bar(
                                                        pro_eid(&self.id, format!("ag-vol-{tid}")),
                                                        vol_ratio,
                                                        4.,
                                                        2.,
                                                        p.line_soft,
                                                        bar_fill,
                                                    ),
                                                ))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(p.muted)
                                                        .child(t.mode.label(cx)),
                                                ),
                                            cx,
                                            move |this, _, cx| {
                                                this.tool_panel_tab = ToolPanelTab::Overview;
                                                this.select(
                                                    Some(ToolCallsSelection::Tool(tid.clone())),
                                                    cx,
                                                );
                                            },
                                        )
                                    })),
                            ),
                    )
                )
            }
            ToolCallsSelection::Tool(tool_id) => {
                let tool = self.find_tool(&tool_id)?.clone();
                let tool_calls: Vec<ToolCallRecord> = self
                    .calls
                    .iter()
                    .filter(|c| c.age_days <= self.range.max_age_days() && c.tool_id == tool.id)
                    .cloned()
                    .collect();
                let t_metrics = summarize_calls(&tool_calls);
                let ov_active = self.tool_panel_tab == ToolPanelTab::Overview;
                let calls_active = self.tool_panel_tab == ToolPanelTab::Calls;
                let tpanel_sensor = pro_selection_sensor();
                let tpanel_sensor_ov = tpanel_sensor.clone();
                let tpanel_sensor_calls = tpanel_sensor.clone();
                let tpanel_key: SharedString = match self.tool_panel_tab {
                    ToolPanelTab::Overview => "ov".into(),
                    ToolPanelTab::Calls => "calls".into(),
                };
                let mode_sensor = pro_selection_sensor();
                let mode_key: SharedString = format!("{:?}", tool.mode).into();
                let vol_share = (t_metrics.total as f32 / max_tool_vol).clamp(0., 1.);
                let avg_ms = t_metrics.avg_duration_ms.unwrap_or(0);
                let lat_ratio = (avg_ms as f32 / max_call_dur).clamp(0., 1.);
                let tool_tab_vtx_key: SharedString =
                    format!("{}:{:?}", tool.id, self.tool_panel_tab).into();

                let waiting_for_tool = tool_calls
                    .iter()
                    .find(|c| c.status == ToolCallStatus::Waiting)
                    .cloned();

                let tid_mode1 = tool.id.clone();
                let tid_mode2 = tool.id.clone();
                let tid_mode3 = tool.id.clone();
                let tid_disc = tool.id.clone();
                let tid_filter = tool.id.clone();

                Some(div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(56.))
                            .pl(px(12.))
                            .pr(px(8.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(render_tool_tile(&tool.app, 30., cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(tool.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(format!("{} · {}", tool.source, tool.fn_name)),
                                    ),
                            )
                            .child(close_btn),
                    )
                    .child(
                        div()
                            .px(px(10.))
                            .pb(px(8.))
                            .border_b_1()
                            .border_color(p.line)
                            .flex_none()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .relative()
                                    .flex()
                                    .items_center()
                                    .gap(px(3.))
                                    .child(pro_pill_indicator(
                                        pro_eid(&self.id, "tpanel-pill-ind"),
                                        tpanel_key,
                                        tpanel_sensor,
                                        6.,
                                        p.chip,
                                        None,
                                        false,
                                    ))
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "tpanel-ov"))
                                            .role(Role::Tab)
                                            .tab_index(0)
                                            .relative()
                                            .h(px(28.))
                                            .px(px(9.))
                                            .rounded(px(6.))
                                            .hover(move |s| {
                                                if ov_active {
                                                    s.text_color(p.ink)
                                                } else {
                                                    s.text_color(p.ink).bg(p.hover)
                                                }
                                            })
                                            .text_size(px(13.))
                                            .text_color(if ov_active { p.ink } else { p.muted })
                                            .flex()
                                            .items_center()
                                            .cursor_pointer()
                                            .when(ov_active, |d| {
                                                d.child(pro_mark_selected(&tpanel_sensor_ov))
                                            })
                                            .child(tr(cx, "Overview", "概览")),
                                        cx,
                                        |this, _, cx| {
                                            this.tool_panel_tab = ToolPanelTab::Overview;
                                            cx.notify();
                                        },
                                    ))
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "tpanel-calls"))
                                            .role(Role::Tab)
                                            .tab_index(0)
                                            .relative()
                                            .h(px(28.))
                                            .px(px(9.))
                                            .rounded(px(6.))
                                            .hover(move |s| {
                                                if calls_active {
                                                    s.text_color(p.ink)
                                                } else {
                                                    s.text_color(p.ink).bg(p.hover)
                                                }
                                            })
                                            .text_size(px(13.))
                                            .text_color(if calls_active { p.ink } else { p.muted })
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .cursor_pointer()
                                            .when(calls_active, |d| {
                                                d.child(pro_mark_selected(&tpanel_sensor_calls))
                                            })
                                            .child(tr(cx, "Calls", "调用记录"))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(p.faint)
                                                    .child(tool_calls.len().to_string()),
                                            ),
                                        cx,
                                        |this, _, cx| {
                                            this.tool_panel_tab = ToolPanelTab::Calls;
                                            cx.notify();
                                        },
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "panel-tool-body"))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p(px(14.))
                            .child(pro_view_transition(
                                pro_eid(&self.id, "tool-tab-vtx"),
                                tool_tab_vtx_key,
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .gap(px(18.))
                            .when(ov_active, |body| {
                                body.child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .child(tool.description.clone()),
                                )
                                .when_some(waiting_for_tool, |d, wc| {
                                    let wid = wc.id.clone();
                                    let wid2 = wc.id.clone();
                                    d.child(
                                        div()
                                            .p(px(11.))
                                            .rounded(px(8.))
                                            .bg(p.amber_soft)
                                            .border_1()
                                            .border_color(p.amber.opacity(0.28))
                                            .flex()
                                            .flex_col()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.amber_ink)
                                                    .child(tr(
                                                        cx,
                                                        "Pending invocation requires approval",
                                                        "有待审批的调用请求",
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .font_family("Fira Code")
                                                    .text_size(px(11.5))
                                                    .text_color(p.ink)
                                                    .child(wc.input_summary.clone()),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .justify_end()
                                                    .gap(px(6.))
                                                    .child(pro_btn(
                                                        pro_eid(&self.id, "tool-reject"),
                                                        tr(cx, "Reject", "拒绝"),
                                                        None,
                                                        ProBtnStyle::Secondary,
                                                        true,
                                                        cx,
                                                        move |this, _, cx| {
                                                            this.resolve_call(&wid, false, cx);
                                                        },
                                                    ))
                                                    .child(pro_btn(
                                                        pro_eid(&self.id, "tool-approve"),
                                                        tr(cx, "Approve", "批准"),
                                                        Some(ProIcon::Check),
                                                        ProBtnStyle::Primary,
                                                        true,
                                                        cx,
                                                        move |this, _, cx| {
                                                            this.resolve_call(&wid2, true, cx);
                                                        },
                                                    )),
                                            ),
                                    )
                                })
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.muted)
                                                .child(tr(cx, "ACCESS", "调用权限")),
                                        )
                                        .child(
                                            div()
                                                .relative()
                                                .p(px(2.))
                                                .rounded(px(8.))
                                                .bg(p.subtle)
                                                .border_1()
                                                .border_color(p.line)
                                                .flex()
                                                .gap(px(2.))
                                                .child(pro_pill_indicator(
                                                    pro_eid(&self.id, "mode-pill-ind"),
                                                    mode_key,
                                                    mode_sensor.clone(),
                                                    6.,
                                                    p.card,
                                                    Some(p.line),
                                                    true,
                                                ))
                                                .child(self.render_mode_option(
                                                    "mode-allowed",
                                                    ToolAccessMode::Allowed,
                                                    tool.mode == ToolAccessMode::Allowed,
                                                    tid_mode1,
                                                    &mode_sensor,
                                                    cx,
                                                ))
                                                .child(self.render_mode_option(
                                                    "mode-ask",
                                                    ToolAccessMode::AskFirst,
                                                    tool.mode == ToolAccessMode::AskFirst,
                                                    tid_mode2,
                                                    &mode_sensor,
                                                    cx,
                                                ))
                                                .child(self.render_mode_option(
                                                    "mode-paused",
                                                    ToolAccessMode::Paused,
                                                    tool.mode == ToolAccessMode::Paused,
                                                    tid_mode3,
                                                    &mode_sensor,
                                                    cx,
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(match tool.mode {
                                                    ToolAccessMode::Allowed => tr(
                                                        cx,
                                                        "Runs automatically whenever the agent calls this function.",
                                                        "智能体发起调用时自动执行，无需人工干预。",
                                                    ),
                                                    ToolAccessMode::AskFirst => tr(
                                                        cx,
                                                        "Pauses before every call so you can review arguments.",
                                                        "每次调用前暂停，待你审阅参数后再执行。",
                                                    ),
                                                    ToolAccessMode::Paused => tr(
                                                        cx,
                                                        "Temporarily blocked; the agent cannot call this tool.",
                                                        "已暂时禁用，智能体不会调用该工具。",
                                                    ),
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.))
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.muted)
                                                .child(tr(cx, "PARAMETERS", "接口参数")),
                                        )
                                        .children(tool.params.iter().enumerate().map(|(pi, prm)| {
                                            div()
                                                .h(px(28.))
                                                .when(pi + 1 < tool.params.len(), |d| {
                                                    d.border_b_1().border_color(p.line_soft)
                                                })
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(
                                                    div()
                                                        .font_family("Fira Code")
                                                        .text_size(px(12.))
                                                        .text_color(p.ink)
                                                        .child(prm.name.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(p.muted)
                                                        .child(prm.kind.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .ml_auto()
                                                        .text_size(px(11.))
                                                        .text_color(p.faint)
                                                        .child(if prm.required {
                                                            tr(cx, "required", "必填")
                                                        } else {
                                                            tr(cx, "optional", "可选")
                                                        }),
                                                )
                                        })),
                                )
                                .child(self.render_stats_quad(
                                    [
                                        (tr(cx, "CALLS", "调用数"), t_metrics.total.to_string()),
                                        (
                                            tr(cx, "AVG TIME", "平均耗时"),
                                            t_metrics
                                                .avg_duration_ms
                                                .map(format_duration_ms)
                                                .unwrap_or_else(|| "–".to_owned()),
                                        ),
                                        (tr(cx, "FAILED", "失败数"), t_metrics.failed.to_string()),
                                        (
                                            tr(cx, "TOKENS", "词元"),
                                            format_tokens_compact(t_metrics.total_tokens),
                                        ),
                                    ],
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(4.))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .text_size(px(11.))
                                                        .child(
                                                            div()
                                                                .text_color(p.muted)
                                                                .child(tr(
                                                                    cx,
                                                                    "Volume vs peak tool",
                                                                    "相对最高频工具占比",
                                                                )),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_color(p.faint)
                                                                .child(format!(
                                                                    "{}%",
                                                                    (vol_share * 100.).round()
                                                                        as u32
                                                                )),
                                                        ),
                                                )
                                                .child(pro_spring_bar(
                                                    pro_eid(&self.id, "tool-vol-bar"),
                                                    vol_share,
                                                    5.,
                                                    2.5,
                                                    p.line_soft,
                                                    p.blue,
                                                )),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(4.))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .text_size(px(11.))
                                                        .child(
                                                            div()
                                                                .text_color(p.muted)
                                                                .child(tr(
                                                                    cx,
                                                                    "Mean latency",
                                                                    "平均响应延迟",
                                                                )),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_color(p.faint)
                                                                .child(
                                                                    t_metrics
                                                                        .avg_duration_ms
                                                                        .map(format_duration_ms)
                                                                        .unwrap_or_else(|| {
                                                                            "–".to_owned()
                                                                        }),
                                                                ),
                                                        ),
                                                )
                                                .child(pro_spring_bar(
                                                    pro_eid(&self.id, "tool-lat-bar"),
                                                    lat_ratio,
                                                    5.,
                                                    2.5,
                                                    p.line_soft,
                                                    if avg_ms > 1500 { p.amber } else { p.teal },
                                                )),
                                        ),
                                )
                                .child(pro_btn(
                                    pro_eid(&self.id, "filter-in-table"),
                                    tr(cx, "Filter table to this tool", "在下方表格中仅看该工具"),
                                    Some(ProIcon::Filter),
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        this.tool_filter = Some(tid_filter.clone());
                                        this.run_folded = false;
                                        cx.notify();
                                    },
                                ))
                            })
                            .when(calls_active, |body| {
                                if tool_calls.is_empty() {
                                    body.child(
                                        div()
                                            .py(px(24.))
                                            .text_size(px(13.))
                                            .text_color(p.muted)
                                            .child(tr(
                                                cx,
                                                "No calls recorded in this range.",
                                                "当前时间范围内暂无调用。",
                                            )),
                                    )
                                } else {
                                    body.children(tool_calls.into_iter().map(|c| {
                                        let cid = c.id.clone();
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("tc-sub-{cid}")))
                                                .role(Role::Button)
                                                .tab_index(0)
                                                .h(px(34.))
                                                .px(px(6.))
                                                .rounded(px(7.))
                                                .hover(move |s| s.bg(p.hover))
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .cursor_pointer()
                                                .child(Self::render_status_chip(
                                                    c.status,
                                                    true,
                                                    spin_rad,
                                                    cx,
                                                ))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .font_family("Fira Code")
                                                        .text_size(px(11.5))
                                                        .text_color(p.ink)
                                                        .truncate()
                                                        .child(c.input_summary.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.5))
                                                        .text_color(p.muted)
                                                        .child(c.timestamp.clone()),
                                                ),
                                            cx,
                                            move |this, _, cx| {
                                                this.select(
                                                    Some(ToolCallsSelection::Call(cid.clone())),
                                                    cx,
                                                );
                                            },
                                        )
                                    }))
                                }
                            }))),
                    )
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(10.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex_none()
                            .child(if self.confirm_disconnect {
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(p.ink)
                                            .child(format!(
                                                "{} {}?",
                                                tr(cx, "Disconnect", "确认断开"),
                                                tool.name
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(pro_btn(
                                                pro_eid(&self.id, "cancel-disc"),
                                                tr(cx, "Cancel", "取消"),
                                                None,
                                                ProBtnStyle::Secondary,
                                                true,
                                                cx,
                                                |this, _, cx| {
                                                    this.confirm_disconnect = false;
                                                    cx.notify();
                                                },
                                            ))
                                            .child(pro_btn(
                                                pro_eid(&self.id, "confirm-disc"),
                                                tr(cx, "Disconnect", "断开连接"),
                                                None,
                                                ProBtnStyle::Danger,
                                                true,
                                                cx,
                                                move |this, _, cx| {
                                                    this.disconnect_tool(&tid_disc, cx);
                                                },
                                            )),
                                    )
                            } else {
                                div().child(pro_btn(
                                    pro_eid(&self.id, "ask-disc"),
                                    format!(
                                        "{} {}",
                                        tr(cx, "Disconnect", "断开连接"),
                                        tool.name
                                    ),
                                    Some(ProIcon::Trash),
                                    ProBtnStyle::Danger,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.confirm_disconnect = true;
                                        cx.notify();
                                    },
                                ))
                            }),
                    ))
            }
            ToolCallsSelection::Call(call_id) => {
                let call = self.calls.iter().find(|c| c.id == call_id)?.clone();
                let (tool_name, tool_fn, tool_app, tool_source) =
                    if let Some(t) = self.find_tool(&call.tool_id) {
                        (
                            t.name.clone(),
                            t.fn_name.clone(),
                            t.app.clone(),
                            t.source.clone(),
                        )
                    } else {
                        (
                            call.tool_id.clone(),
                            call.tool_id.clone(),
                            call.tool_id.clone(),
                            "API".into(),
                        )
                    };

                let dur_str = call
                    .duration_ms
                    .map(format_duration_ms)
                    .unwrap_or_else(|| "–".to_owned());
                let call_ms = call.duration_ms.unwrap_or(0);
                let call_lat_ratio = (call_ms as f32 / max_call_dur).clamp(0., 1.);
                let call_lat_color = match call.status {
                    ToolCallStatus::Failed => p.red,
                    ToolCallStatus::Waiting => p.amber,
                    ToolCallStatus::Running => p.blue,
                    _ if call_ms > 1500 => p.amber,
                    _ => p.teal,
                };
                let tok_str = call
                    .tokens
                    .map(|n| format!("{} {}", format_tokens_compact(n), tr(cx, "tokens", "词元")))
                    .unwrap_or_else(|| tr(cx, "No context added", "未增加上下文").to_owned());

                let cid_app = call.id.clone();
                let cid_rej = call.id.clone();
                let cid_retry = call.id.clone();
                let in_json = call.input_json.clone();
                let out_json = call.output_json.clone();
                let copied_in = self.copied == Some(CopySlot::Input);
                let copied_out = self.copied == Some(CopySlot::Output);

                let http_code = match call.status {
                    ToolCallStatus::Done => "200 OK",
                    ToolCallStatus::Failed => "504 / 404",
                    ToolCallStatus::Running => "102 Processing",
                    ToolCallStatus::Waiting => "Paused (Policy)",
                    ToolCallStatus::Rejected => "403 Rejected",
                };
                let headers_rows = div().pt(px(6.)).flex().flex_col().children(
                    [
                        (tr(cx, "Function", "接口名"), tool_fn.to_string()),
                        (tr(cx, "Source", "来源"), tool_source.to_string()),
                        (tr(cx, "Run ID", "运行批次"), call.run_id.to_string()),
                        (tr(cx, "Status", "响应状态"), http_code.to_owned()),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(ki, (k, v))| {
                        div()
                            .h(px(28.))
                            .when(ki < 3, |r| r.border_b_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .text_size(px(12.))
                            .child(div().text_color(p.muted).child(k))
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(v),
                            )
                    }),
                );

                Some(div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(56.))
                            .pl(px(12.))
                            .pr(px(8.))
                            .border_b_1()
                            .border_color(p.line)
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(render_tool_tile(&tool_app, 30., cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(tool_name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(format!("{} · {}", call.timestamp, tool_fn)),
                                    ),
                            )
                            .child(Self::render_status_chip(call.status, false, spin_rad, cx))
                            .child(close_btn),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "panel-call-body"))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(16.))
                            .when(call.status == ToolCallStatus::Waiting, |d| {
                                d.child(
                                    div()
                                        .p(px(11.))
                                        .rounded(px(8.))
                                        .bg(p.amber_soft)
                                        .border_1()
                                        .border_color(p.amber.opacity(0.28))
                                        .flex()
                                        .flex_col()
                                        .gap(px(9.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(7.))
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.amber_ink)
                                                .child(pro_icon(ProIcon::Clock, 13., p.amber_ink))
                                                .child(format!(
                                                    "{} {}",
                                                    self.agent.name,
                                                    tr(
                                                        cx,
                                                        "wants to execute this tool call",
                                                        "请求执行此工具调用"
                                                    )
                                                )),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .justify_end()
                                                .gap(px(6.))
                                                .child(pro_btn(
                                                    pro_eid(&self.id, "call-reject"),
                                                    tr(cx, "Reject", "拒绝"),
                                                    None,
                                                    ProBtnStyle::Secondary,
                                                    true,
                                                    cx,
                                                    move |this, _, cx| {
                                                        this.resolve_call(&cid_rej, false, cx);
                                                    },
                                                ))
                                                .child(pro_btn(
                                                    pro_eid(&self.id, "call-approve"),
                                                    tr(cx, "Approve", "批准执行"),
                                                    Some(ProIcon::Check),
                                                    ProBtnStyle::Primary,
                                                    true,
                                                    cx,
                                                    move |this, _, cx| {
                                                        this.resolve_call(&cid_app, true, cx);
                                                    },
                                                )),
                                        ),
                                )
                            })
                            .when(call.status == ToolCallStatus::Failed, |d| {
                                let err_msg = call
                                    .error_message
                                    .clone()
                                    .unwrap_or_else(|| "Upstream tool returned an error".into());
                                d.child(
                                    div()
                                        .p(px(11.))
                                        .rounded(px(8.))
                                        .bg(p.red_soft)
                                        .border_1()
                                        .border_color(p.red.opacity(0.26))
                                        .flex()
                                        .flex_col()
                                        .gap(px(9.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_start()
                                                .gap(px(7.))
                                                .child(div().pt(px(2.)).child(pro_icon(
                                                    ProIcon::AlertTriangle,
                                                    13.,
                                                    p.red_ink,
                                                )))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .font_family("Fira Code")
                                                        .text_size(px(11.5))
                                                        .line_height(px(17.))
                                                        .text_color(p.red_ink)
                                                        .child(err_msg),
                                                ),
                                        )
                                        .child(div().flex().justify_end().child(pro_btn(
                                            pro_eid(&self.id, "call-retry"),
                                            tr(cx, "Retry call", "重试该调用"),
                                            Some(ProIcon::Refresh),
                                            ProBtnStyle::Secondary,
                                            true,
                                            cx,
                                            move |this, _, cx| {
                                                this.retry_call(&cid_retry, cx);
                                            },
                                        ))),
                                )
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(p.muted)
                                            .child(format!(
                                                "{dur_str} · {tok_str} · {}",
                                                call.run_id
                                            )),
                                    )
                                    .child(pro_spring_bar(
                                        pro_eid(&self.id, "call-lat-bar"),
                                        call_lat_ratio,
                                        5.,
                                        2.5,
                                        p.line_soft,
                                        call_lat_color,
                                    )),
                            )
                            // Input JSON block
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.muted)
                                                    .child(tr(
                                                        cx,
                                                        "INPUT ARGUMENTS",
                                                        "请求参数 (JSON)",
                                                    )),
                                            )
                                            .child(pro_icon_btn(
                                                pro_eid(&self.id, "copy-input"),
                                                tr(cx, "Copy input JSON", "复制输入 JSON"),
                                                if copied_in {
                                                    ProIcon::Check
                                                } else {
                                                    ProIcon::Copy
                                                },
                                                24.,
                                                copied_in,
                                                cx,
                                                move |this, _, cx| {
                                                    this.copy_payload(
                                                        CopySlot::Input,
                                                        in_json.clone(),
                                                        cx,
                                                    );
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .p(px(11.))
                                            .rounded(px(8.))
                                            .bg(p.subtle)
                                            .border_1()
                                            .border_color(p.line)
                                            .font_family("Fira Code")
                                            .text_size(px(11.5))
                                            .line_height(px(18.))
                                            .text_color(p.ink)
                                            .child(call.input_json.clone()),
                                    ),
                            )
                            // Output JSON block
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.muted)
                                                    .child(tr(
                                                        cx,
                                                        "RESPONSE PAYLOAD",
                                                        "响应结果 (JSON)",
                                                    )),
                                            )
                                            .child(pro_icon_btn(
                                                pro_eid(&self.id, "copy-output"),
                                                tr(cx, "Copy response JSON", "复制响应 JSON"),
                                                if copied_out {
                                                    ProIcon::Check
                                                } else {
                                                    ProIcon::Copy
                                                },
                                                24.,
                                                copied_out,
                                                cx,
                                                move |this, _, cx| {
                                                    this.copy_payload(
                                                        CopySlot::Output,
                                                        out_json.clone(),
                                                        cx,
                                                    );
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .p(px(11.))
                                            .rounded(px(8.))
                                            .bg(p.subtle)
                                            .border_1()
                                            .border_color(p.line)
                                            .font_family("Fira Code")
                                            .text_size(px(11.5))
                                            .line_height(px(18.))
                                            .text_color(p.ink)
                                            .child(call.output_json.clone()),
                                    ),
                            )
                            // Collapsible Headers & Metadata
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "toggle-headers"))
                                            .role(Role::Button)
                                            .tab_index(0)
                                            .h(px(24.))
                                            .rounded(px(5.))
                                            .hover(move |s| s.text_color(p.ink))
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .cursor_pointer()
                                            .child(pro_icon(
                                                if self.headers_open {
                                                    ProIcon::ChevronDown
                                                } else {
                                                    ProIcon::ChevronRight
                                                },
                                                12.,
                                                p.faint,
                                            ))
                                            .child(tr(cx, "HEADERS & METADATA", "请求头与元数据")),
                                        cx,
                                        |this, _, cx| {
                                            this.headers_open = !this.headers_open;
                                            cx.notify();
                                        },
                                    ))
                                    .child(pro_fold(
                                        pro_eid(&self.id, "call-headers-fold"),
                                        self.headers_open,
                                        118.,
                                        0.,
                                        move || headers_rows.into_any_element(),
                                    )),
                            ),
                    ))
            }
        });

        let vtx_eid = pro_eid(&self.id, "floating-panel-vtx");
        pro_presence(
            pro_eid(&self.id, "floating-panel-pres"),
            panel_open && panel_opt.is_some(),
            0.22,
            0.16,
            move |amount| {
                let Some(panel_content) = panel_opt else {
                    return div().into_any_element();
                };
                let container = if is_sheet {
                    div()
                        .absolute()
                        .left(px(8.))
                        .right(px(8.))
                        .bottom(px(8. - 20. * (1. - amount)))
                        .h(px(420.))
                } else {
                    div()
                        .absolute()
                        .top(px(10.))
                        .right(px(10. - 24. * (1. - amount)))
                        .bottom(px(10.))
                        .w(px(384.))
                };
                container
                    .opacity(amount)
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(pro_view_transition_fill(
                        vtx_eid,
                        sel_vtx_key,
                        panel_content,
                    ))
                    .into_any_element()
            },
        )
    }

    fn render_mode_option(
        &self,
        suffix: &'static str,
        mode: ToolAccessMode,
        checked: bool,
        tool_id: SharedString,
        sensor: &ProSelectionSensor,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let sensor_item = sensor.clone();
        let el = div()
            .id(pro_eid(&self.id, suffix))
            .role(Role::RadioButton)
            .tab_index(0)
            .relative()
            .flex_1()
            .h(px(26.))
            .rounded(px(6.))
            .hover(move |s| {
                if checked {
                    s.text_color(p.ink)
                } else {
                    s.text_color(p.ink).bg(p.hover)
                }
            })
            .text_size(px(12.5))
            .font_weight(if checked {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if checked { p.ink } else { p.muted })
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when(checked, |d| d.child(pro_mark_selected(&sensor_item)))
            .child(mode.label(cx));
        activate(el, cx, move |this, _, cx| {
            this.set_tool_mode(&tool_id, mode, cx);
        })
    }

    fn render_stats_quad(&self, cells: [(&str, String); 4], cx: &App) -> Div {
        let p = ProPalette::get(cx);
        div()
            .rounded(px(8.))
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .border_b_1()
                    .border_color(p.line)
                    .child(self.render_stat_cell(cells[0].0, &cells[0].1, true, cx))
                    .child(self.render_stat_cell(cells[1].0, &cells[1].1, false, cx)),
            )
            .child(
                div()
                    .flex()
                    .child(self.render_stat_cell(cells[2].0, &cells[2].1, true, cx))
                    .child(self.render_stat_cell(cells[3].0, &cells[3].1, false, cx)),
            )
    }

    fn render_stat_cell(&self, label: &str, value: &str, border_right: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        div()
            .flex_1()
            .min_w_0()
            .px(px(12.))
            .py(px(9.))
            .when(border_right, |d| d.border_r_1().border_color(p.line))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(label.to_owned()),
            )
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .truncate()
                    .child(value.to_owned()),
            )
    }

    fn render_popovers(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let p = ProPalette::get(cx);
        let mut out = Vec::with_capacity(3);

        let agent_menu_card = div()
            .id(pro_eid(&self.id, "agent-menu-pop"))
            .w(px(216.))
            .p(px(4.))
            .rounded(px(11.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_md()
            .flex()
            .flex_col()
            .gap(px(2.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(px(8.))
                    .py(px(4.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.faint)
                    .child(tr(cx, "Topology layout", "结构布局")),
            )
            .children(
                [
                    ToolCallsLayout::Org,
                    ToolCallsLayout::Compact,
                    ToolCallsLayout::Canvas,
                ]
                .into_iter()
                .enumerate()
                .map(|(li, lay)| {
                    let active = self.layout == lay;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("lay-opt-{li}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.))
                            .text_color(if active { p.ink } else { p.text })
                            .cursor_pointer()
                            .child(lay.label(cx))
                            .when(active, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.layout = lay;
                            this.agent_menu_open = false;
                            cx.notify();
                        },
                    )
                }),
            );
        out.push(
            div()
                .absolute()
                .top(px(42.))
                .left(px(10.))
                .child(pro_presence(
                    pro_eid(&self.id, "agent-menu-pres"),
                    self.agent_menu_open,
                    0.16,
                    0.12,
                    move |amount| {
                        div()
                            .mt(px(-5. * (1. - amount)))
                            .opacity(amount)
                            .child(agent_menu_card)
                            .into_any_element()
                    },
                ))
                .into_any_element(),
        );

        let range_menu_card = div()
            .id(pro_eid(&self.id, "range-menu-pop"))
            .w(px(172.))
            .p(px(4.))
            .rounded(px(11.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_md()
            .flex()
            .flex_col()
            .gap(px(2.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(
                [
                    (ToolCallsRange::Today, tr(cx, "Today", "今天")),
                    (
                        ToolCallsRange::SevenDays,
                        tr(cx, "Last 7 days", "最近 7 天"),
                    ),
                    (
                        ToolCallsRange::ThirtyDays,
                        tr(cx, "Last 30 days", "最近 30 天"),
                    ),
                ]
                .into_iter()
                .enumerate()
                .map(|(ri, (rng, lbl))| {
                    let active = self.range == rng;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("rng-opt-{ri}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.))
                            .text_color(if active { p.ink } else { p.text })
                            .cursor_pointer()
                            .child(lbl)
                            .when(active, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.range = rng;
                            this.range_menu_open = false;
                            cx.notify();
                        },
                    )
                }),
            );
        out.push(
            div()
                .absolute()
                .top(px(42.))
                .left(px(114.))
                .child(pro_presence(
                    pro_eid(&self.id, "range-menu-pres"),
                    self.range_menu_open,
                    0.16,
                    0.12,
                    move |amount| {
                        div()
                            .mt(px(-5. * (1. - amount)))
                            .opacity(amount)
                            .child(range_menu_card)
                            .into_any_element()
                    },
                ))
                .into_any_element(),
        );

        let candidates: Vec<ConnectedTool> = catalog_extra_tools()
            .into_iter()
            .chain(default_tools())
            .filter(|cand| !self.tools.iter().any(|t| t.id == cand.id))
            .collect();
        let picker_scrim_eid = pro_eid(&self.id, "picker-scrim");
        let picker_dismiss = cx.listener(|this, _, _, cx| {
            this.picker_open = false;
            cx.notify();
        });
        let picker_dialog = div()
            .w_full()
            .max_w(px(460.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_xl()
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(48.))
                    .pl(px(16.))
                    .pr(px(10.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Connect a tool", "连接新工具")),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "picker-close"),
                        tr(cx, "Close", "关闭"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.picker_open = false;
                            cx.notify();
                        },
                    )),
            )
            .child(div().p(px(10.)).flex().flex_col().gap(px(4.)).children(
                candidates.into_iter().map(|cand| {
                    let cid = cand.id.clone();
                    let cand_clone = cand.clone();
                    div()
                        .p(px(8.))
                        .rounded(px(9.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(render_tool_tile(&cand.app, 30., cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(cand.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(cand.description.clone()),
                                ),
                        )
                        .child(pro_btn(
                            pro_eid(&self.id, format!("connect-{cid}")),
                            tr(cx, "Connect", "连接"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                let connected_id = cand_clone.id.clone();
                                this.empty = false;
                                this.tools.push(cand_clone.clone());
                                this.picker_open = false;
                                cx.emit(AgentToolCallsEvent::ConnectTool(connected_id));
                                cx.notify();
                            },
                        ))
                }),
            ));
        out.push(pro_presence(
            pro_eid(&self.id, "picker-modal-pres"),
            self.picker_open,
            0.20,
            0.15,
            move |amount| {
                let base_alpha = if p.dark { 0.50 } else { 0.16 };
                div()
                    .id(picker_scrim_eid)
                    .absolute()
                    .inset_0()
                    .bg(hsla(0., 0., 0., base_alpha * amount))
                    .p(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, picker_dismiss)
                    .child(
                        div()
                            .w_full()
                            .max_w(px(460.))
                            .mt(px(10. * (1. - amount)))
                            .opacity(amount)
                            .child(picker_dialog),
                    )
                    .into_any_element()
            },
        ));

        out
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Filter calls…", "筛选调用…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            self.search_editor = Some(ed);
        }

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_secs = self.boot_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_secs * 1000.;

        let has_running = self
            .calls
            .iter()
            .any(|c| c.status == ToolCallStatus::Running);
        let animate = !reduced
            && w.is_visible()
            && (self.loading || (self.live && !self.paused && has_running && !self.empty));
        if animate {
            w.request_animation_frame();
        }

        let spin_rad = if reduced || self.paused {
            0.7
        } else {
            elapsed_secs * std::f32::consts::TAU * 1.15
        };
        let flow_phase = if reduced || self.paused {
            0.5
        } else {
            (elapsed_secs * 0.65).fract()
        };
        let skel_phase = if reduced { 0.25 } else { elapsed_secs * 0.85 };

        let container_w = self.measured_width.get().max(320.);
        if self.selection.is_some() {
            self.last_selection = self.selection.clone();
        }
        let panel_open = self.selection.is_some();
        let target_pr = if panel_open && container_w >= 640. {
            396.
        } else {
            0.
        };
        let anim_pr = pro_spring_f32(pro_eid(&self.id, "main-pr-spring"), target_pr, 0.22, w, cx);
        let effective_w = if anim_pr > 1.0 && container_w >= 640. {
            (container_w - anim_pr - 8.).max(320.)
        } else {
            container_w
        };

        let measure_cell = self.measured_width.clone();
        let org_area = self.render_org_area(
            effective_w,
            elapsed_ms,
            spin_rad,
            flow_phase,
            skel_phase,
            reduced,
            cx,
        );
        let run_section =
            self.render_run_section(effective_w, elapsed_ms, spin_rad, skel_phase, w, cx);
        let floating_panel = self.render_floating_panel(container_w, elapsed_ms, spin_rad, cx);
        let popovers = self.render_popovers(cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(520.))
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key))
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if let Some((start_y, start_h)) = this.resizing_run {
                    let dy = start_y - f32::from(ev.position.y);
                    this.run_height = (start_h + dy).clamp(132., 380.);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.resizing_run.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.agent_menu_open || this.range_menu_open {
                        this.agent_menu_open = false;
                        this.range_menu_open = false;
                        cx.notify();
                    }
                }),
            )
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
                .absolute()
                .size_0(),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .when(anim_pr > 0.5, |d| d.pr(px(anim_pr)))
                    .flex()
                    .flex_col()
                    .child(org_area)
                    .child(run_section),
            )
            .child(floating_panel)
            .children(popovers)
    }
}

impl Render for AgentToolCallsScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ToolAccessMode, ToolCallRecord, ToolCallStatus, ToolCallsRange, default_calls,
        default_tools, format_duration_ms, summarize_calls,
    };

    #[test]
    fn today_summary_matches_reference_telemetry() {
        let today: Vec<ToolCallRecord> = default_calls()
            .into_iter()
            .filter(|c| c.age_days <= ToolCallsRange::Today.max_age_days())
            .collect();
        let stats = summarize_calls(&today);
        assert_eq!(stats.total, 15);
        assert_eq!(stats.failed, 2);
        assert_eq!(stats.waiting, 1);
        assert_eq!(stats.running, 1);
        assert_eq!(stats.success_pct, 87);
        assert_eq!(stats.avg_duration_ms, Some(2070));
        assert_eq!(
            format_duration_ms(stats.avg_duration_ms.unwrap_or(0)),
            "2.1s"
        );
        assert_eq!(stats.total_tokens, 8930);
    }

    #[test]
    fn default_tools_preserve_access_modes_and_per_tool_latency() {
        let tools = default_tools();
        assert_eq!(tools.len(), 7);
        let slack = tools.iter().find(|t| t.id.as_ref() == "slack");
        let gmail = tools.iter().find(|t| t.id.as_ref() == "gmail");
        assert_eq!(slack.map(|t| t.mode), Some(ToolAccessMode::AskFirst));
        assert_eq!(gmail.map(|t| t.mode), Some(ToolAccessMode::Paused));

        let stripe_calls: Vec<ToolCallRecord> = default_calls()
            .into_iter()
            .filter(|c| c.age_days == 0 && c.tool_id.as_ref() == "stripe")
            .collect();
        let stripe_stats = summarize_calls(&stripe_calls);
        assert_eq!(stripe_stats.total, 3);
        assert_eq!(stripe_stats.avg_duration_ms, Some(553));
        assert_eq!(
            stripe_calls
                .iter()
                .filter(|c| c.status == ToolCallStatus::Failed)
                .count(),
            1
        );
    }
}
