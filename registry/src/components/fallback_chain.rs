//! Model failover flow diagram routing a request through prioritized LLM providers until one answers.
//!
//! Connects an incoming `Request` node across smooth cubic curves to an ordered stack of primary
//! and fallback models (`Claude`, `GPT`, `Gemini`, `DeepSeek`) and onward into `Response`. Failed
//! branches break red with HTTP 429/529 reasons, the active attempt shimmers with a traveling
//! light pulse, and interactive toggles let operators simulate outages on any model in the chain.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_check_draw, pro_connector_pulse,
        pro_eid, pro_empty_state, pro_mix_color, pro_rolling_number, pro_shimmer_text,
        pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

#[allow(clippy::too_many_arguments)]
fn paint_bezier_link(
    p0: Point<Pixels>,
    p1: Point<Pixels>,
    base_color: Hsla,
    pulse_color: Hsla,
    progress: Option<f32>,
    broken_red: bool,
    stroke_w: f32,
    w: &mut Window,
) {
    let x0 = f32::from(p0.x);
    let y0 = f32::from(p0.y);
    let x1 = f32::from(p1.x);
    let y1 = f32::from(p1.y);
    let dx = x1 - x0;
    let c1 = (x0 + dx * 0.55_f32, y0);
    let c2 = (x0 + dx * 0.45_f32, y1);

    let steps = 32usize;
    let mut pts = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let inv = 1.0 - t;
        let bx = inv * inv * inv * x0
            + 3.0 * inv * inv * t * c1.0
            + 3.0 * inv * t * t * c2.0
            + t * t * t * x1;
        let by = inv * inv * inv * y0
            + 3.0 * inv * inv * t * c1.1
            + 3.0 * inv * t * t * c2.1
            + t * t * t * y1;
        pts.push(point(px(bx), px(by)));
    }

    if broken_red {
        // Draw two segments with a crisp break gap around t = 0.72 and a tiny cross tick
        let split_a = 20usize;
        let split_b = 25usize;
        let mut s1 = PathBuilder::stroke(px(stroke_w));
        for (i, &pt) in pts.iter().take(split_a + 1).enumerate() {
            if i == 0 {
                s1.move_to(pt);
            } else {
                s1.line_to(pt);
            }
        }
        if let Ok(path) = s1.build() {
            w.paint_path(path, base_color);
        }
        let mut s2 = PathBuilder::stroke(px(stroke_w));
        for (i, &pt) in pts.iter().skip(split_b).enumerate() {
            if i == 0 {
                s2.move_to(pt);
            } else {
                s2.line_to(pt);
            }
        }
        if let Ok(path) = s2.build() {
            w.paint_path(path, base_color.opacity(0.45));
        }
        let mid_pt = pts[22];
        let mx = f32::from(mid_pt.x);
        let my = f32::from(mid_pt.y);
        let mut x_a = PathBuilder::stroke(px(1.5));
        x_a.move_to(point(px(mx - 3.2), px(my - 3.2)));
        x_a.line_to(point(px(mx + 3.2), px(my + 3.2)));
        if let Ok(pa) = x_a.build() {
            w.paint_path(pa, base_color);
        }
        let mut x_b = PathBuilder::stroke(px(1.5));
        x_b.move_to(point(px(mx + 3.2), px(my - 3.2)));
        x_b.line_to(point(px(mx - 3.2), px(my + 3.2)));
        if let Ok(pb) = x_b.build() {
            w.paint_path(pb, base_color);
        }
        return;
    }

    let mut base = PathBuilder::stroke(px(stroke_w));
    for (i, &pt) in pts.iter().enumerate() {
        if i == 0 {
            base.move_to(pt);
        } else {
            base.line_to(pt);
        }
    }
    if let Ok(path) = base.build() {
        w.paint_path(path, base_color);
    }

    if let Some(prog) = progress {
        let prog = prog.clamp(0.0, 1.0);
        let total = steps as f32;
        let head = prog * total;
        let span = 8.5_f32;
        for i in 0..steps {
            let mid = i as f32 + 0.5_f32;
            let dist = (mid - head).abs();
            if dist <= span {
                let falloff = (1.0 - dist / span).clamp(0.0, 1.0);
                let alpha = falloff * falloff;
                let mut seg = PathBuilder::stroke(px(stroke_w + 0.9 * alpha));
                seg.move_to(pts[i]);
                seg.line_to(pts[i + 1]);
                if let Ok(p) = seg.build() {
                    w.paint_path(p, pulse_color.opacity(alpha));
                }
            }
        }
    }
}

/// Live or settled status of one [`FallbackAttempt`] in [`FallbackChain`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttemptStatus {
    /// Queued behind an earlier model in priority order.
    #[default]
    Waiting,
    /// Currently receiving the request stream.
    Trying,
    /// Timed out, rate-limited, or returned a 5xx provider error.
    Failed,
    /// Returned a valid completion that routed onward to `Response`.
    Answered,
    /// Bypassed because an earlier higher-priority model already answered.
    Skipped,
}

/// Endpoint descriptor for the incoming `Request` or outgoing `Response` node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainEndpoint {
    /// Source or destination label (`"HubSpot · Ticket #4821"` or `"Slack · #support"`).
    pub name: SharedString,
    /// Brand artwork key (`"hubspot"`, `"slack"`, `"github"`).
    pub app: SharedString,
    /// Prompt or completion preview text shown in the detail inspector.
    pub text: SharedString,
}

impl ChainEndpoint {
    /// Construct a [`ChainEndpoint`] with `name`, `app`, and preview `text`.
    pub fn new(
        name: impl Into<SharedString>,
        app: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            name: name.into(),
            app: app.into(),
            text: text.into(),
        }
    }
}

/// One candidate model row in [`FallbackChain`].
#[derive(Clone, Debug, PartialEq)]
pub struct FallbackAttempt {
    /// Stable attempt identifier.
    pub id: SharedString,
    /// Model display name (`"Claude Sonnet 4.5"`, `"GPT-5.4"`, `"Gemini 2.5 Pro"`).
    pub model: SharedString,
    /// Brand artwork lookup name (`"claude"`, `"gpt"`, `"gemini"`, `"deepseek"`).
    pub brand: SharedString,
    /// Provider and region caption (`"Anthropic · us-east"`).
    pub provider: SharedString,
    /// Current runtime status (`Waiting`, `Trying`, `Failed`, `Answered`, `Skipped`).
    pub status: AttemptStatus,
    /// Simulated failure flag for interactive failover testing (`true` causes this model to fail with `failure_reason`).
    pub will_fail: bool,
    /// Error label shown when `status == Failed` (`"Overloaded · 529"`, `"Rate limited · 429"`).
    pub failure_reason: SharedString,
    /// Elapsed latency in milliseconds for this attempt.
    pub duration_ms: u32,
    /// Cost in USD incurred by this attempt (`0.0` on HTTP 429/529).
    pub cost_usd: f64,
}

impl FallbackAttempt {
    /// Construct a [`FallbackAttempt`] with `id`, `model`, `brand`, and `provider`.
    pub fn new(
        id: impl Into<SharedString>,
        model: impl Into<SharedString>,
        brand: impl Into<SharedString>,
        provider: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            brand: brand.into(),
            provider: provider.into(),
            status: AttemptStatus::Waiting,
            will_fail: false,
            failure_reason: "Rate limited · 429".into(),
            duration_ms: 820,
            cost_usd: 0.0018,
        }
    }

    /// Configure whether this model fails during failover simulation and with what error reason.
    pub fn with_simulated_failure(
        mut self,
        will_fail: bool,
        reason: impl Into<SharedString>,
    ) -> Self {
        self.will_fail = will_fail;
        self.failure_reason = reason.into();
        if will_fail {
            self.status = AttemptStatus::Failed;
        }
        self
    }

    /// Set the explicit [`AttemptStatus`].
    pub fn with_status(mut self, status: AttemptStatus) -> Self {
        self.status = status;
        self.will_fail = status == AttemptStatus::Failed;
        self
    }

    /// Set latency (`duration_ms`) and USD cost (`cost_usd`).
    pub fn with_metrics(mut self, duration_ms: u32, cost_usd: f64) -> Self {
        self.duration_ms = duration_ms;
        self.cost_usd = cost_usd;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FocusedNode {
    Request,
    Model(usize),
    Response,
}

/// Typed events emitted by [`FallbackChain`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FallbackChainEvent {
    /// Operator toggled whether `model_id` fails or succeeds and re-ran the failover wave.
    ModelHealthToggled {
        model_id: SharedString,
        will_fail: bool,
    },
    /// Failover wave settled on `Some(model_id)` or exhausted all models (`None`).
    Settled(Option<SharedString>),
    /// Operator pressed `Replay` / `Retry` to re-send the request through the chain.
    Retried,
}

fn default_attempts() -> Vec<FallbackAttempt> {
    vec![
        FallbackAttempt::new(
            "claude-sonnet",
            "Claude Sonnet 4.5",
            "claude",
            "Anthropic · us-east-1",
        )
        .with_simulated_failure(true, "Overloaded · 529")
        .with_metrics(1620, 0.0),
        FallbackAttempt::new("gpt-5", "GPT-5.4", "gpt", "OpenAI · us-central")
            .with_simulated_failure(false, "Rate limited · 429")
            .with_metrics(940, 0.0024),
        FallbackAttempt::new(
            "gemini-pro",
            "Gemini 2.5 Pro",
            "gemini",
            "Vertex AI · us-central1",
        )
        .with_simulated_failure(false, "Timeout · 504")
        .with_metrics(780, 0.0016),
        FallbackAttempt::new(
            "deepseek-v3",
            "DeepSeek V3",
            "deepseek",
            "Fireworks · us-west-2",
        )
        .with_simulated_failure(false, "Upstream reset · 502")
        .with_metrics(690, 0.0007),
    ]
}

fn format_ms(ms: u32) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f32 / 1000.0)
    } else {
        format!("{ms}ms")
    }
}

fn format_cost(usd: f64) -> String {
    if usd <= 0.0 {
        "$0".to_owned()
    } else if usd < 0.0001 {
        "<$0.0001".to_owned()
    } else {
        format!("${usd:.4}")
    }
}

/// Model failover flow diagram with live attempt wave, red broken branches, and interactive outage toggles.
pub struct FallbackChain {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    route: SharedString,
    request: ChainEndpoint,
    response: ChainEndpoint,
    attempts: Vec<FallbackAttempt>,
    controlled: bool,
    run_started: Instant,
    settled_emitted: bool,
    inspected: FocusedNode,
    hovered_idx: Option<usize>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl FallbackChain {
    /// Create a [`FallbackChain`] preloaded with 4 prioritized models (`Claude` 529 -> `GPT-5.4` answers -> `Response`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        let mut chain = Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Summarize support ticket".into(),
            route: "kense/auto".into(),
            request: ChainEndpoint::new(
                "HubSpot · Ticket #4821",
                "hubspot",
                "Summarize this ticket for the on-call engineer: what broke, who is affected, and what the customer already tried.",
            ),
            response: ChainEndpoint::new(
                "Slack · #support",
                "slack",
                "SSO callback fails on Safari 18 due to SameSite=Strict cookie drop; 14 enterprise seats affected; customer cleared cache with no change.",
            ),
            attempts: default_attempts(),
            controlled: false,
            run_started: now,
            settled_emitted: false,
            inspected: FocusedNode::Model(1),
            hovered_idx: None,
            boot_at: now,
            measured_width: Rc::new(Cell::new(720.0)),
            toast: None,
            loading: false,
            empty: false,
        };
        chain.recompute_statuses_at(99.0);
        // Start with a fresh animated wave on mount when motion is enabled
        chain.run_started = now;
        chain
    }

    /// Override the heading above the diagram.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Override the router slug displayed under `Request` (defaults to `"kense/auto"`).
    pub fn with_route(mut self, route: impl Into<SharedString>) -> Self {
        self.route = route.into();
        self
    }

    /// Configure the incoming `Request` node metadata and prompt preview.
    pub fn with_request(mut self, request: ChainEndpoint) -> Self {
        self.request = request;
        self
    }

    /// Configure the outgoing `Response` node metadata and completion preview.
    pub fn with_response(mut self, response: ChainEndpoint) -> Self {
        self.response = response;
        self
    }

    /// Supply explicit [`FallbackAttempt`] rows in priority order.
    pub fn with_attempts(mut self, attempts: Vec<FallbackAttempt>) -> Self {
        self.attempts = attempts;
        self.controlled = true;
        self
    }

    /// Toggle loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Current slice of fallback attempts.
    pub fn attempts(&self) -> &[FallbackAttempt] {
        &self.attempts
    }

    /// Model id that answered the request, if settled.
    pub fn answered_model_id(&self) -> Option<SharedString> {
        self.attempts
            .iter()
            .find(|a| a.status == AttemptStatus::Answered)
            .map(|a| a.id.clone())
    }

    /// Whether every candidate model in the chain failed.
    pub fn all_failed(&self) -> bool {
        !self.attempts.is_empty()
            && self
                .attempts
                .iter()
                .all(|a| a.status == AttemptStatus::Failed)
    }

    /// Toggle whether model `id` fails or succeeds, then re-run the failover wave.
    pub fn toggle_model_failure(&mut self, id: &str, cx: &mut Context<Self>) -> Option<bool> {
        let idx = self.attempts.iter().position(|a| a.id.as_ref() == id)?;
        self.controlled = false;
        self.attempts[idx].will_fail = !self.attempts[idx].will_fail;
        let will_fail = self.attempts[idx].will_fail;
        self.inspected = FocusedNode::Model(idx);
        self.run_started = Instant::now();
        self.settled_emitted = false;
        if disabled_motion(cx) {
            self.recompute_statuses_at(99.0);
        } else {
            self.recompute_statuses_at(0.0);
        }
        cx.emit(FallbackChainEvent::ModelHealthToggled {
            model_id: id.to_owned().into(),
            will_fail,
        });
        cx.notify();
        Some(will_fail)
    }

    /// Replay the request from attempt #1 through the current health configuration.
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        self.controlled = false;
        self.run_started = Instant::now();
        self.settled_emitted = false;
        if disabled_motion(cx) {
            self.recompute_statuses_at(99.0);
        } else {
            self.recompute_statuses_at(0.0);
        }
        cx.emit(FallbackChainEvent::Retried);
        cx.notify();
    }

    fn recompute_statuses_at(&mut self, elapsed_s: f32) -> bool {
        let stage_dur = 0.95_f32;
        let mut cursor_t = 0.0_f32;
        let mut winner_found = false;
        let mut still_animating = false;

        for (idx, att) in self.attempts.iter_mut().enumerate() {
            if winner_found {
                att.status = AttemptStatus::Skipped;
                continue;
            }
            if elapsed_s < cursor_t {
                att.status = AttemptStatus::Waiting;
                still_animating = true;
            } else if elapsed_s < cursor_t + stage_dur {
                att.status = AttemptStatus::Trying;
                self.inspected = FocusedNode::Model(idx);
                still_animating = true;
                // Remaining attempts stay Waiting until this attempt finishes
                for rest in self.attempts.iter_mut().skip(idx + 1) {
                    rest.status = AttemptStatus::Waiting;
                }
                break;
            } else if att.will_fail {
                att.status = AttemptStatus::Failed;
                cursor_t += stage_dur;
            } else {
                att.status = AttemptStatus::Answered;
                self.inspected = FocusedNode::Model(idx);
                winner_found = true;
            }
        }
        still_animating
    }

    fn localize_str<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Summarize support ticket" => {
                tr(cx, "Summarize support ticket", "汇总客服工单故障详情")
            }
            "Overloaded · 529" => tr(cx, "Overloaded · 529", "负载过高 · 529"),
            "Rate limited · 429" => tr(cx, "Rate limited · 429", "触发限流 · 429"),
            "Timeout · 504" => tr(cx, "Timeout · 504", "网关超时 · 504"),
            "Upstream reset · 502" => tr(cx, "Upstream reset · 502", "上游连接重置 · 502"),
            other => other,
        }
    }
}

impl EventEmitter<FallbackChainEvent> for FallbackChain {}

impl Focusable for FallbackChain {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for FallbackChain {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        if self.loading {
            let phase = self.boot_at.elapsed().as_secs_f32() * 0.9;
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(740.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(240., 20., 6., phase, cx))
                .child(pro_skeleton_bar(720., 290., 12., phase + 0.15, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.attempts.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(740.))
                .mx_auto()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No fallback models configured", "尚未配置备用模型链"),
                    tr(
                        cx,
                        "Add ordered model candidates to visualize failover routing and latency.",
                        "添加按优先级排列的模型列表即可可视化故障转移路由与耗时。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-fbk"),
                        tr(cx, "Load sample chain", "加载示例故障转移链").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.attempts = default_attempts();
                            this.retry(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        if !self.controlled {
            let run_age = if reduced {
                99.0_f32
            } else {
                self.run_started.elapsed().as_secs_f32()
            };
            let animating = self.recompute_statuses_at(run_age);
            if animating && !reduced {
                w.request_animation_frame();
            } else if !animating && !self.settled_emitted {
                self.settled_emitted = true;
                let winner = self.answered_model_id();
                cx.emit(FallbackChainEvent::Settled(winner));
            }
        }

        let trying_idx = self
            .attempts
            .iter()
            .position(|a| a.status == AttemptStatus::Trying);
        let answered_idx = self
            .attempts
            .iter()
            .position(|a| a.status == AttemptStatus::Answered);
        let all_failed = self.all_failed();

        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        if (trying_idx.is_some() || answered_idx.is_some()) && !reduced {
            w.request_animation_frame();
        }
        let pulse_t = if reduced {
            None
        } else {
            Some(((elapsed_s / 1.35).fract()).clamp(0.0, 1.0))
        };

        let mut total_ms = 0u32;
        let mut total_cost = 0.0_f64;
        for att in &self.attempts {
            match att.status {
                AttemptStatus::Failed | AttemptStatus::Answered => {
                    total_ms += att.duration_ms;
                    if att.status == AttemptStatus::Answered {
                        total_cost += att.cost_usd;
                    }
                }
                AttemptStatus::Trying => {
                    total_ms += 380;
                }
                _ => {}
            }
        }

        let width_cell = self.measured_width.clone();
        let container_w = width_cell.get().max(320.0);
        let show_provider = container_w >= 640.0;
        let compact_stack = container_w < 540.0;

        let area_bg: Hsla = if p.dark {
            rgb(0x19191b).into()
        } else {
            rgb(0xfbfbfb).into()
        };
        let card_bg: Hsla = if p.dark {
            rgb(0x232327).into()
        } else {
            rgb(0xffffff).into()
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.05)
        };
        let link_idle: Hsla = if p.dark {
            hsla(0., 0., 1., 0.18)
        } else {
            hsla(0., 0., 0., 0.15)
        };
        let accent_blue: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0070d6).into()
        };
        let ok_col = p.green;
        let ok_ink = p.green_ink;
        let fail_col = p.red;
        let fail_ink = p.red_ink;
        let hover_border = p.line_strong;

        // Pre-sample per-provider spotlight & opacity springs!
        let row_springs: Vec<(f32, f32)> = self
            .attempts
            .iter()
            .enumerate()
            .map(|(idx, att)| {
                let base_op = match att.status {
                    AttemptStatus::Skipped => 0.52_f32,
                    AttemptStatus::Waiting => 0.78_f32,
                    _ => 1.0_f32,
                };
                let (target_op, target_spt) = match self.hovered_idx {
                    Some(hi) if hi == idx => (1.0_f32, 1.0_f32),
                    Some(_) => (0.34_f32, 0.0_f32),
                    None => (base_op, 0.0_f32),
                };
                let op = pro_spring_f32(
                    pro_eid(&self.id, format!("fbk-op-{idx}")),
                    target_op,
                    0.22,
                    w,
                    cx,
                );
                let spt = pro_spring_f32(
                    pro_eid(&self.id, format!("fbk-spt-{idx}")),
                    target_spt,
                    0.20,
                    w,
                    cx,
                );
                (op, spt)
            })
            .collect();

        // 1. Top Summary Bar
        let (dot_col, state_text, state_fg): (Hsla, String, Hsla) = if let Some(ti) = trying_idx {
            let txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("第 {} / {} 次尝试", ti + 1, self.attempts.len())
            } else {
                format!("Attempt {} of {}", ti + 1, self.attempts.len())
            };
            (accent_blue, txt, p.ink)
        } else if answered_idx.is_some() {
            let txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已在 {} 内成功响应", format_ms(total_ms))
            } else {
                format!("Answered in {}", format_ms(total_ms))
            };
            (ok_col, txt, ok_ink)
        } else if all_failed {
            let txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("全部 {} 个模型尝试失败", self.attempts.len())
            } else {
                format!("All {} attempts failed", self.attempts.len())
            };
            (fail_col, txt, fail_ink)
        } else {
            (p.faint, tr(cx, "Queued", "等待请求").to_owned(), p.muted)
        };

        let top_bar = div()
            .id(pro_eid(&self.id, "fbk-summary"))
            .role(Role::Status)
            .w_full()
            .min_h(px(28.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .truncate()
                    .child(self.localize_str(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .text_size(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(if answered_idx.is_some() {
                                pro_check_draw(
                                    pro_eid(&self.id, "fbk-sum-chk"),
                                    true,
                                    12.5,
                                    false,
                                    ok_ink,
                                )
                            } else {
                                div()
                                    .size(px(6.))
                                    .rounded_full()
                                    .bg(dot_col)
                                    .into_any_element()
                            })
                            .child(pro_swap_label(
                                pro_eid(&self.id, "fbk-sum-lbl"),
                                state_text,
                                12.0,
                                FontWeight::MEDIUM,
                                state_fg,
                            )),
                    )
                    .child(
                        div()
                            .pl(px(10.))
                            .border_l_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_icon(ProIcon::Clock, 12., p.faint))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "fbk-tot-ms"),
                                total_ms as f64,
                                0,
                                "",
                                "ms",
                                12.0,
                                FontWeight::NORMAL,
                                p.ink,
                            )),
                    )
                    .child(
                        div()
                            .pl(px(10.))
                            .border_l_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "fbk-tot-cost"),
                                total_cost,
                                4,
                                "$",
                                "",
                                12.0,
                                FontWeight::NORMAL,
                                p.ink,
                            )),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "fbk-replay"),
                        if all_failed {
                            tr(cx, "Retry", "重试请求")
                        } else {
                            tr(cx, "Replay", "重播转移")
                        },
                        Some(ProIcon::Refresh),
                        if all_failed {
                            ProBtnStyle::Primary
                        } else {
                            ProBtnStyle::Secondary
                        },
                        true,
                        cx,
                        |this, _, cx| {
                            this.retry(cx);
                        },
                    )),
            );

        // 2. Request & Response Endpoint Nodes
        let ep_w = if compact_stack { 180.0 } else { 142.0 };
        let req_selected = self.inspected == FocusedNode::Request;
        let req_node = pro_tactile(
            pro_eid(&self.id, "fbk-req"),
            format!("Request · {}", self.route),
            10.0,
            card_bg,
            pro_mix_color(card_bg, accent_blue, if p.dark { 0.06 } else { 0.03 }),
            if req_selected {
                accent_blue.opacity(0.6)
            } else {
                p.line
            },
            hover_border,
            req_selected,
            div()
                .w(px(ep_w))
                .h(px(52.))
                .px(px(9.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_app_tile(&self.request.app, 28., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .truncate()
                                .child(tr(cx, "Request", "入站请求")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .line_height(px(15.))
                                .text_color(p.muted)
                                .truncate()
                                .child(self.route.clone()),
                        ),
                ),
            cx,
            |this, _, cx| {
                this.inspected = FocusedNode::Request;
                cx.notify();
            },
        )
        .w(px(ep_w))
        .h(px(52.))
        .flex_none()
        .when(!p.dark, |d| d.shadow_xs());

        let resp_selected = self.inspected == FocusedNode::Response;
        let resp_sub = if answered_idx.is_some() {
            self.response.name.to_string()
        } else if all_failed {
            tr(cx, "No answer", "未获得响应").to_owned()
        } else {
            tr(cx, "Waiting…", "等待中…").to_owned()
        };
        let resp_bdr = if resp_selected {
            accent_blue.opacity(0.6)
        } else if answered_idx.is_some() {
            ok_col.opacity(0.50)
        } else if all_failed {
            fail_col.opacity(0.45)
        } else {
            p.line
        };
        let resp_op = pro_spring_f32(
            pro_eid(&self.id, "fbk-resp-op"),
            if answered_idx.is_some() || all_failed {
                1.0
            } else {
                0.72
            },
            0.22,
            w,
            cx,
        );

        let resp_node = pro_tactile(
            pro_eid(&self.id, "fbk-resp"),
            format!("Response · {resp_sub}"),
            10.0,
            card_bg,
            pro_mix_color(card_bg, ok_col, if p.dark { 0.06 } else { 0.03 }),
            resp_bdr,
            hover_border,
            resp_selected,
            div()
                .w(px(ep_w))
                .h(px(52.))
                .px(px(9.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_app_tile(&self.response.app, 28., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .truncate()
                                .child(tr(cx, "Response", "返回结果")),
                        )
                        .child(pro_swap_label(
                            pro_eid(&self.id, "fbk-resp-sub"),
                            resp_sub,
                            11.0,
                            FontWeight::NORMAL,
                            if answered_idx.is_some() {
                                ok_ink
                            } else if all_failed {
                                fail_ink
                            } else {
                                p.muted
                            },
                        )),
                ),
            cx,
            |this, _, cx| {
                this.inspected = FocusedNode::Response;
                cx.notify();
            },
        )
        .w(px(ep_w))
        .h(px(52.))
        .flex_none()
        .opacity(resp_op)
        .when(!p.dark, |d| d.shadow_xs());

        // 3. Ordered Priority Model Cards (`60px` row height, `10px` gap)
        let row_h = 60.0_f32;
        let row_gap = 10.0_f32;
        let n_models = self.attempts.len();
        let stack_h = n_models as f32 * row_h + (n_models.saturating_sub(1)) as f32 * row_gap;

        let mut models_col = div().flex_1().min_w_0().flex().flex_col().gap(px(row_gap));

        for (idx, att) in self.attempts.iter().enumerate() {
            let aid = att.id.clone();
            let status = att.status;
            let is_sel = self.inspected == FocusedNode::Model(idx);

            let (badge_bg, badge_fg) = match status {
                AttemptStatus::Trying => (accent_blue, rgb(0xffffff).into()),
                AttemptStatus::Failed => (fail_col, rgb(0xffffff).into()),
                AttemptStatus::Answered => (ok_col, rgb(0xffffff).into()),
                AttemptStatus::Waiting | AttemptStatus::Skipped => (card_bg, p.muted),
            };

            let row_border = match status {
                AttemptStatus::Trying => accent_blue.opacity(0.60),
                AttemptStatus::Answered => ok_col.opacity(0.55),
                AttemptStatus::Failed => fail_col.opacity(0.38),
                _ if is_sel => accent_blue.opacity(0.45),
                _ => p.line,
            };

            let status_el: AnyElement = match status {
                AttemptStatus::Trying => pro_shimmer_text(
                    pro_eid(&self.id, format!("try-shim-{aid}")),
                    tr(cx, "Trying…", "正在尝试请求…"),
                    accent_blue,
                    p.strong,
                    11.5,
                    FontWeight::MEDIUM,
                    true,
                ),
                AttemptStatus::Failed => pro_swap_label(
                    pro_eid(&self.id, format!("fbk-st-{aid}")),
                    self.localize_str(&att.failure_reason, cx).to_owned(),
                    11.5,
                    FontWeight::NORMAL,
                    fail_ink,
                ),
                AttemptStatus::Answered => div()
                    .flex()
                    .items_center()
                    .gap(px(4.5))
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("fbk-chk-{aid}")),
                        true,
                        12.0,
                        false,
                        ok_ink,
                    ))
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("fbk-st-{aid}")),
                        tr(cx, "Answered · streamed 200 OK", "成功响应 · 200 OK"),
                        11.5,
                        FontWeight::MEDIUM,
                        ok_ink,
                    ))
                    .into_any_element(),
                AttemptStatus::Skipped => pro_swap_label(
                    pro_eid(&self.id, format!("fbk-st-{aid}")),
                    tr(cx, "Not needed", "无需调用（前序已响应）"),
                    11.5,
                    FontWeight::NORMAL,
                    p.muted,
                ),
                AttemptStatus::Waiting => pro_swap_label(
                    pro_eid(&self.id, format!("fbk-st-{aid}")),
                    if trying_idx.is_some_and(|ti| ti + 1 == idx) {
                        tr(cx, "Up next", "下一顺位备选")
                    } else {
                        tr(cx, "Waiting", "排队待命")
                    },
                    11.5,
                    FontWeight::NORMAL,
                    p.muted,
                ),
            };

            let right_metrics: AnyElement = match status {
                AttemptStatus::Failed | AttemptStatus::Answered => div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("fbk-lat-{aid}")),
                        format_ms(att.duration_ms),
                        11.5,
                        FontWeight::NORMAL,
                        p.ink,
                    ))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(format_cost(if status == AttemptStatus::Answered {
                                att.cost_usd
                            } else {
                                0.0
                            })),
                    )
                    .into_any_element(),
                AttemptStatus::Trying => div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("fbk-lat-{aid}")),
                        "380ms",
                        11.5,
                        FontWeight::NORMAL,
                        p.ink,
                    ))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child("<$0.0001"),
                    )
                    .into_any_element(),
                _ => div()
                    .flex_none()
                    .text_size(px(11.5))
                    .text_color(p.faint)
                    .child("–")
                    .into_any_element(),
            };

            let aid_toggle = aid.clone();
            let will_fail_now = att.will_fail;
            let toggle_chip_bg = if will_fail_now {
                p.red_soft
            } else {
                p.green_soft
            };
            let toggle_chip_fg = if will_fail_now {
                p.red_ink
            } else {
                p.green_ink
            };

            let sim_btn = pro_tactile(
                pro_eid(&self.id, format!("sim-toggle-{aid}")),
                format!(
                    "{} {}",
                    att.model,
                    if will_fail_now {
                        tr(cx, "simulating failure", "模拟故障")
                    } else {
                        tr(cx, "healthy", "健康可用")
                    }
                ),
                5.0,
                toggle_chip_bg,
                pro_mix_color(toggle_chip_bg, toggle_chip_fg, 0.12),
                toggle_chip_fg.opacity(0.24),
                toggle_chip_fg.opacity(0.50),
                false,
                div()
                    .h(px(20.))
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("sim-lbl-{aid}")),
                        if will_fail_now {
                            tr(cx, "Fail 529", "模拟失败")
                        } else {
                            tr(cx, "Healthy", "正常响应")
                        },
                        10.5,
                        FontWeight::MEDIUM,
                        toggle_chip_fg,
                    )),
                cx,
                move |this, _, cx| {
                    this.toggle_model_failure(aid_toggle.as_ref(), cx);
                },
            )
            .role(Role::Switch)
            .aria_toggled((!will_fail_now).into())
            .flex_none()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation());

            let (row_op, _) = row_springs.get(idx).copied().unwrap_or((1.0, 0.0));

            let model_card = pro_tactile(
                pro_eid(&self.id, format!("mrow-fbk-{aid}")),
                format!("#{} {}", idx + 1, att.model),
                10.0,
                card_bg,
                pro_mix_color(
                    card_bg,
                    if status == AttemptStatus::Answered {
                        ok_col
                    } else {
                        accent_blue
                    },
                    if p.dark { 0.05 } else { 0.03 },
                ),
                row_border,
                hover_border,
                is_sel,
                div()
                    .w_full()
                    .h(px(row_h))
                    .pl(px(10.))
                    .pr(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(28.))
                            .flex_none()
                            .relative()
                            .rounded(px(7.))
                            .bg(if p.dark { rgb(0x2b2b30) } else { rgb(0xffffff) })
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                pro_app_artwork(&att.brand, 14.5, cx)
                                    .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 13., p.ink)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .right(px(-5.))
                                    .bottom(px(-4.))
                                    .min_w(px(14.))
                                    .h(px(14.))
                                    .px(px(3.))
                                    .rounded_full()
                                    .bg(badge_bg)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(8.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(badge_fg)
                                    .child((idx + 1).to_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .line_height(px(17.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(att.model.clone()),
                                    )
                                    .when(show_provider, |d| {
                                        d.child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(p.faint)
                                                .truncate()
                                                .child(att.provider.clone()),
                                        )
                                    }),
                            )
                            .child(status_el),
                    )
                    .child(sim_btn)
                    .child(right_metrics),
                cx,
                move |this, _, cx| {
                    this.inspected = FocusedNode::Model(idx);
                    cx.notify();
                },
            )
            .w_full()
            .h(px(row_h))
            .opacity(row_op)
            .when(!p.dark, |d| d.shadow_xs())
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                let next = (*inside).then_some(idx);
                if this.hovered_idx != next {
                    this.hovered_idx = next;
                    cx.notify();
                }
            }));

            models_col = models_col.child(model_card);
        }

        // 4. Left & Right Bezier Connector Canvases (`48px` wide x `stack_h` tall)
        let statuses_vec: Vec<AttemptStatus> = self.attempts.iter().map(|a| a.status).collect();
        let st_left = statuses_vec.clone();
        let sp_left = row_springs.clone();
        let left_links = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);
                let bw = f32::from(bounds.size.width);
                let mid_y = oy + f32::from(bounds.size.height) * 0.5_f32;
                let p_start = point(px(ox), px(mid_y));

                for (i, st) in st_left.iter().enumerate() {
                    let (vis, spt) = sp_left.get(i).copied().unwrap_or((1.0, 0.0));
                    let ry = oy + i as f32 * (row_h + row_gap) + row_h * 0.5_f32;
                    let p_end = point(px(ox + bw), px(ry));
                    match st {
                        AttemptStatus::Failed => {
                            paint_bezier_link(
                                p_start,
                                p_end,
                                fail_col.opacity(vis),
                                fail_col,
                                None,
                                true,
                                1.5 + 0.3 * spt,
                                window,
                            );
                        }
                        AttemptStatus::Trying => {
                            paint_bezier_link(
                                p_start,
                                p_end,
                                accent_blue.opacity(0.65 * vis),
                                accent_blue,
                                pulse_t,
                                false,
                                1.75 + 0.3 * spt,
                                window,
                            );
                        }
                        AttemptStatus::Answered => {
                            paint_bezier_link(
                                p_start,
                                p_end,
                                ok_col.opacity(vis),
                                rgb(0xd8ffe6).into(),
                                pulse_t,
                                false,
                                1.75 + 0.3 * spt,
                                window,
                            );
                        }
                        AttemptStatus::Waiting | AttemptStatus::Skipped => {
                            paint_bezier_link(
                                p_start,
                                p_end,
                                link_idle.opacity(link_idle.a * vis + 0.25 * spt),
                                accent_blue,
                                (spt > 0.5).then_some(pulse_t).flatten(),
                                false,
                                1.3 + 0.35 * spt,
                                window,
                            );
                        }
                    }

                    // Failover cascade pulse stepping down from failed provider i-1 into row i!
                    if i > 0
                        && st_left[i - 1] == AttemptStatus::Failed
                        && matches!(st, AttemptStatus::Trying | AttemptStatus::Answered)
                    {
                        let prev_y =
                            oy + (i - 1) as f32 * (row_h + row_gap) + row_h * 0.5_f32 + 9.0;
                        let cur_y = ry - 9.0;
                        let rail_x = ox + bw - 6.0;
                        let cascade_col = if *st == AttemptStatus::Trying {
                            accent_blue
                        } else {
                            ok_col
                        };
                        pro_connector_pulse(
                            point(px(rail_x), px(prev_y)),
                            point(px(rail_x), px(cur_y)),
                            4.0,
                            true,
                            cascade_col.opacity(0.55 * vis),
                            cascade_col,
                            pulse_t,
                            1.55,
                            window,
                        );
                    }
                }
            },
        )
        .w(px(48.))
        .h(px(stack_h))
        .flex_none();

        let st_right = statuses_vec;
        let sp_right = row_springs;
        let right_links = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);
                let bw = f32::from(bounds.size.width);
                let mid_y = oy + f32::from(bounds.size.height) * 0.5_f32;
                let p_end = point(px(ox + bw), px(mid_y));

                for (i, st) in st_right.iter().enumerate() {
                    let (vis, spt) = sp_right.get(i).copied().unwrap_or((1.0, 0.0));
                    let ry = oy + i as f32 * (row_h + row_gap) + row_h * 0.5_f32;
                    let p_start = point(px(ox), px(ry));
                    if *st == AttemptStatus::Answered {
                        paint_bezier_link(
                            p_start,
                            p_end,
                            ok_col.opacity(vis),
                            rgb(0xd8ffe6).into(),
                            pulse_t,
                            false,
                            1.8 + 0.3 * spt,
                            window,
                        );
                    } else {
                        paint_bezier_link(
                            p_start,
                            p_end,
                            link_idle.opacity(0.6 * link_idle.a * vis),
                            link_idle,
                            None,
                            false,
                            1.2,
                            window,
                        );
                    }
                }
            },
        )
        .w(px(48.))
        .h(px(stack_h))
        .flex_none();

        let flow_stage = if compact_stack {
            div()
                .w_full()
                .p(px(14.))
                .rounded(px(12.))
                .bg(area_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(req_node)
                        .child(resp_node),
                )
                .child(models_col)
        } else {
            // Subtle dot grid painted behind the 5-column flow
            let dot_col_grid: Hsla = if p.dark {
                hsla(0., 0., 1., 0.05)
            } else {
                hsla(0., 0., 0., 0.045)
            };
            let dots_bg = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let w_px = f32::from(bounds.size.width);
                    let h_px = f32::from(bounds.size.height);
                    let ox = f32::from(bounds.origin.x);
                    let oy = f32::from(bounds.origin.y);
                    let step = 14.0_f32;
                    let mut y = 8.0_f32;
                    while y < h_px - 4.0 {
                        let mut x = 8.0_f32;
                        while x < w_px - 4.0 {
                            window.paint_quad(fill(
                                Bounds::new(point(px(ox + x), px(oy + y)), size(px(1.5), px(1.5))),
                                dot_col_grid,
                            ));
                            x += step;
                        }
                        y += step;
                    }
                },
            )
            .absolute()
            .inset_0();

            div()
                .w_full()
                .relative()
                .py(px(22.))
                .px(px(16.))
                .rounded(px(12.))
                .bg(area_bg)
                .border_1()
                .border_color(p.line)
                .when(!p.dark, |d| d.shadow_xs())
                .overflow_hidden()
                .child(dots_bg)
                .child(
                    div()
                        .relative()
                        .w_full()
                        .flex()
                        .items_center()
                        .child(req_node)
                        .child(left_links)
                        .child(models_col)
                        .child(right_links)
                        .child(resp_node),
                )
        };

        // 5. Contextual Detail Strip (shows prompt / model diagnostics / streamed answer for selected node)
        let detail_key = format!("{:?}", self.inspected);
        let detail_card = match &self.inspected {
            FocusedNode::Request => div()
                .flex()
                .items_start()
                .gap(px(10.))
                .child(pro_app_tile(&self.request.app, 24., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format!(
                                    "{} · {}",
                                    self.request.name,
                                    tr(cx, "Incoming prompt", "入站提示词")
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(p.text)
                                .child(self.request.text.clone()),
                        ),
                ),
            FocusedNode::Response => div()
                .flex()
                .items_start()
                .gap(px(10.))
                .child(pro_app_tile(&self.response.app, 24., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format!(
                                    "{} · {}",
                                    self.response.name,
                                    if let Some(wi) = answered_idx {
                                        format!(
                                            "{} {}",
                                            tr(cx, "Answered by", "响应模型："),
                                            self.attempts[wi].model
                                        )
                                    } else {
                                        tr(cx, "Awaiting completion", "等待模型完成响应").to_owned()
                                    }
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(if answered_idx.is_some() {
                                    p.text
                                } else {
                                    p.muted
                                })
                                .child(if answered_idx.is_some() {
                                    self.response.text.clone()
                                } else if all_failed {
                                    tr(
                                        cx,
                                        "Every configured provider failed — switch a model back to Healthy or press Retry.",
                                        "所有候选模型均返回错误——可将任一模型切换回“正常响应”或点击重试。",
                                    )
                                    .into()
                                } else {
                                    tr(cx, "Routing request across fallback chain…", "正在沿备用链路由请求…")
                                        .into()
                                }),
                        ),
                ),
            FocusedNode::Model(idx) => {
                let att = self
                    .attempts
                    .get(*idx)
                    .cloned()
                    .unwrap_or_else(|| self.attempts[0].clone());
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .min_w_0()
                            .child(
                                div()
                                    .size(px(24.))
                                    .rounded(px(6.))
                                    .bg(fill_bg)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        pro_app_artwork(&att.brand, 13., cx).unwrap_or_else(|| {
                                            pro_icon(ProIcon::Sparkles, 12., p.ink)
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(format!(
                                                "#{} {} · {}",
                                                idx + 1,
                                                att.model,
                                                att.provider
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(tr(
                                                cx,
                                                "Click Healthy / Fail 529 on any row to test failover across providers.",
                                                "点击任意模型行的“正常响应 / 模拟失败”按钮即可实时测试跨厂商故障转移。",
                                            )),
                                    ),
                            ),
                    )
            }
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_str(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .w_full()
            .max_w(px(740.))
            .mx_auto()
            .relative()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                canvas(
                    move |bounds, _, _| {
                        let bw = f32::from(bounds.size.width);
                        if bw > 1.0 {
                            width_cell.set(bw);
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .h(px(1.)),
            )
            .child(top_bar)
            .child(flow_stage)
            .child(pro_view_transition(
                pro_eid(&self.id, "fbk-det-vtx"),
                detail_key,
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .child(detail_card),
            ))
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::FallbackChain;
    use crate::foundation::Theme;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn fallback_chain_routes_around_failed_models_and_handles_total_outage(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        cx.update(|cx| {
            let mut t = Theme::global(cx).clone();
            t.reduced_motion = true;
            cx.set_global(t);
        });

        let view = cx.new(|cx| FallbackChain::new("test-fallback-chain", cx));

        view.update(cx, |fc, cx| {
            // Default: #1 Claude fails (529), #2 GPT-5.4 answers!
            fc.recompute_statuses_at(99.0);
            assert_eq!(fc.answered_model_id(), Some("gpt-5".into()));
            assert!(!fc.all_failed());

            // Simulate GPT-5.4 failing too -> Gemini 2.5 Pro (#3) rescues the request!
            fc.toggle_model_failure("gpt-5", cx);
            assert_eq!(fc.answered_model_id(), Some("gemini-pro".into()));

            // Simulate #3 and #4 failing -> total outage (`all_failed() == true`)
            fc.toggle_model_failure("gemini-pro", cx);
            fc.toggle_model_failure("deepseek-v3", cx);
            assert!(fc.all_failed());
            assert_eq!(fc.answered_model_id(), None);

            // Restore #1 Claude -> answers immediately on attempt #1!
            fc.toggle_model_failure("claude-sonnet", cx);
            assert_eq!(fc.answered_model_id(), Some("claude-sonnet".into()));
        });
    }
}
