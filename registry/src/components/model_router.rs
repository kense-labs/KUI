//! LLM gateway traffic-splitting diagram with spring-width ribbons, blended cost readout, and deployable fallback policy.
//!
//! Streams light streaks from a gateway port across curved horizontal bands into each routed model,
//! scaling band thickness smoothly via `Slide` springs to match traffic share `%`. Pairs a rolling
//! blended `$/1K tokens` readout and `Cost` / `Balanced` / `Quality` presets with a route table,
//! interactive fallback model pickers, and versioned `Deploy policy` workflow.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_eid, pro_empty_state,
        pro_mark_selected, pro_pill_indicator, pro_popover_presence, pro_rolling_number,
        pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_switch,
        pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

fn sample_route_curve(
    x0: f32,
    y0: f32,
    x_elbow: f32,
    x_join: f32,
    x1: f32,
    y1: f32,
) -> Vec<(f32, f32)> {
    let mut pts = Vec::with_capacity(36);
    // Horizontal stem from gateway
    for i in 0..=6usize {
        let u = i as f32 / 6.0;
        pts.push((x0 + (x_elbow - x0) * u, y0));
    }
    // Smooth cubic S-bend from (x_elbow, y0) to (x_join, y1)
    let span = x_join - x_elbow;
    let c1 = (x_elbow + span * 0.55, y0);
    let c2 = (x_elbow + span * 0.45, y1);
    for i in 1..=20usize {
        let t = i as f32 / 20.0;
        let inv = 1.0 - t;
        let bx = inv * inv * inv * x_elbow
            + 3.0 * inv * inv * t * c1.0
            + 3.0 * inv * t * t * c2.0
            + t * t * t * x_join;
        let by = inv * inv * inv * y0
            + 3.0 * inv * inv * t * c1.1
            + 3.0 * inv * t * t * c2.1
            + t * t * t * y1;
        pts.push((bx, by));
    }
    // Horizontal run into model endpoint
    for i in 1..=8usize {
        let u = i as f32 / 8.0;
        pts.push((x_join + (x1 - x_join) * u, y1));
    }
    pts
}

/// Candidate LLM target in [`ModelRouter`].
#[derive(Clone, Debug, PartialEq)]
pub struct RouterModel {
    /// Stable model identifier referenced by strategy shares and fallback rules.
    pub id: SharedString,
    /// Display name (`"Claude Opus 5.5"`, `"GPT-5.5"`, `"Gemini 3 Flash"`).
    pub name: SharedString,
    /// Provider name shown on hover (`"Anthropic"`, `"OpenAI"`, `"Google"`).
    pub provider: SharedString,
    /// Brand artwork lookup name (`"claude"`, `"gpt"`, `"gemini"`, `"grok"`).
    pub brand: SharedString,
    /// p95 latency in seconds (`2.16`, `1.34`, `0.64`).
    pub p95_secs: f32,
    /// Blended price per 1K tokens in USD (`0.0126`, `0.0048`, `0.0007`).
    pub cost_per_1k: f64,
    /// Optional custom route accent color.
    pub color: Option<Hsla>,
}

impl RouterModel {
    /// Construct a [`RouterModel`] with `id`, `name`, `provider`, `brand`, `p95_secs`, and `cost_per_1k`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        provider: impl Into<SharedString>,
        brand: impl Into<SharedString>,
        p95_secs: f32,
        cost_per_1k: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: provider.into(),
            brand: brand.into(),
            p95_secs,
            cost_per_1k,
            color: None,
        }
    }

    /// Override the ribbon and share-bar color for this model.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Traffic-splitting preset strategy in [`ModelRouter`].
#[derive(Clone, Debug, PartialEq)]
pub struct RouterStrategy {
    /// Stable strategy id (`"cost"`, `"balanced"`, `"quality"`).
    pub id: SharedString,
    /// Button label (`"Cost"`, `"Balanced"`, `"Quality"`).
    pub label: SharedString,
    /// `(model_id, weight)` pairs automatically normalized to `100%`.
    pub shares: Vec<(SharedString, f32)>,
}

impl RouterStrategy {
    /// Construct a [`RouterStrategy`] with `id`, `label`, and `(model_id, weight)` shares.
    pub fn new<I, S>(id: impl Into<SharedString>, label: impl Into<SharedString>, shares: I) -> Self
    where
        I: IntoIterator<Item = (S, f32)>,
        S: Into<SharedString>,
    {
        Self {
            id: id.into(),
            label: label.into(),
            shares: shares.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}

/// Automatic retry rule when a primary routed model errors out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouterFallbackRule {
    /// Whether automatic fallback is enabled.
    pub enabled: bool,
    /// Primary model id watched for errors.
    pub from_model: SharedString,
    /// Backup model id retried on failure.
    pub to_model: SharedString,
}

impl RouterFallbackRule {
    /// Construct a [`RouterFallbackRule`].
    pub fn new(
        enabled: bool,
        from_model: impl Into<SharedString>,
        to_model: impl Into<SharedString>,
    ) -> Self {
        Self {
            enabled,
            from_model: from_model.into(),
            to_model: to_model.into(),
        }
    }
}

/// Complete deployable routing policy snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouterPolicy {
    /// Active strategy id.
    pub strategy_id: SharedString,
    /// Fallback failover rule.
    pub fallback: RouterFallbackRule,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenPicker {
    From,
    To,
}

/// Typed events emitted by [`ModelRouter`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelRouterEvent {
    /// Draft policy changed (strategy switched or fallback edited).
    PolicyEdited(RouterPolicy),
    /// Draft policy reverted back to the currently live baseline.
    Reverted(RouterPolicy),
    /// Draft policy deployed as the new live policy at `version`.
    Deployed { version: u32, policy: RouterPolicy },
}

fn default_models() -> Vec<RouterModel> {
    vec![
        RouterModel::new(
            "opus",
            "Claude Opus 5.5",
            "Anthropic",
            "claude",
            2.16,
            0.0126,
        ),
        RouterModel::new("gpt", "GPT-5.5", "OpenAI", "gpt", 1.34, 0.0048),
        RouterModel::new("flash", "Gemini 3 Flash", "Google", "gemini", 0.64, 0.0007),
        RouterModel::new("grok", "Grok 5 Fast", "xAI", "grok", 1.09, 0.0019),
    ]
}

fn default_strategies() -> Vec<RouterStrategy> {
    vec![
        RouterStrategy::new(
            "cost",
            "Cost",
            [
                ("opus", 4.0),
                ("gpt", 18.0),
                ("flash", 58.0),
                ("grok", 20.0),
            ],
        ),
        RouterStrategy::new(
            "balanced",
            "Balanced",
            [
                ("opus", 16.0),
                ("gpt", 46.0),
                ("flash", 26.0),
                ("grok", 12.0),
            ],
        ),
        RouterStrategy::new(
            "quality",
            "Quality",
            [("opus", 58.0), ("gpt", 28.0), ("flash", 8.0), ("grok", 6.0)],
        ),
    ]
}

fn model_route_color(idx: usize, custom: Option<Hsla>, p: &ProPalette) -> Hsla {
    if let Some(c) = custom {
        return c;
    }
    match idx % 4 {
        0 => p.red,
        1 => p.amber,
        2 => p.green,
        _ => p.blue,
    }
}

/// LLM gateway traffic-splitting router card with spring-width ribbons, rolling cost delta, and deployable fallback rule.
pub struct ModelRouter {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    environment: SharedString,
    version: u32,
    models: Vec<RouterModel>,
    strategies: Vec<RouterStrategy>,
    live_policy: RouterPolicy,
    draft_policy: RouterPolicy,
    hovered_model: Option<SharedString>,
    open_picker: Option<OpenPicker>,
    last_picker: Option<OpenPicker>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl ModelRouter {
    /// Create a [`ModelRouter`] preloaded with 4 models, `Cost` / `Balanced` / `Quality` presets, and `Policy v14` live on `chat-prod`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let policy = RouterPolicy {
            strategy_id: "balanced".into(),
            fallback: RouterFallbackRule::new(true, "opus", "gpt"),
        };
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Model router".into(),
            environment: "chat-prod".into(),
            version: 14,
            models: default_models(),
            strategies: default_strategies(),
            live_policy: policy.clone(),
            draft_policy: policy,
            hovered_model: None,
            open_picker: None,
            last_picker: None,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(460.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the router card heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the target environment badge (defaults to `"chat-prod"`).
    pub fn with_environment(mut self, env: impl Into<SharedString>) -> Self {
        self.environment = env.into();
        self
    }

    /// Set the starting live policy version number (defaults to `14`).
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Supply custom [`RouterModel`] entries.
    pub fn with_models(mut self, models: Vec<RouterModel>) -> Self {
        self.models = models;
        self.sanitize_fallback();
        self
    }

    /// Supply custom [`RouterStrategy`] presets.
    pub fn with_strategies(mut self, strategies: Vec<RouterStrategy>) -> Self {
        if let Some(first) = strategies.first()
            && !strategies
                .iter()
                .any(|s| s.id == self.live_policy.strategy_id)
        {
            self.live_policy.strategy_id = first.id.clone();
            self.draft_policy.strategy_id = first.id.clone();
        }
        self.strategies = strategies;
        self
    }

    /// Set the initial live [`RouterPolicy`].
    pub fn with_live_policy(mut self, policy: RouterPolicy) -> Self {
        self.live_policy = policy.clone();
        self.draft_policy = policy;
        self.sanitize_fallback();
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

    /// Current live policy version.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Whether the draft policy differs from the currently deployed live policy.
    pub fn has_unsaved_changes(&self) -> bool {
        self.draft_policy != self.live_policy
    }

    /// Compute normalized `0.0..=100.0` shares for each model under `strategy_id`.
    pub fn normalized_shares(&self, strategy_id: &str) -> Vec<f32> {
        if self.models.is_empty() {
            return Vec::new();
        }
        let strat = self
            .strategies
            .iter()
            .find(|s| s.id.as_ref() == strategy_id)
            .or_else(|| self.strategies.first());

        let raw: Vec<f32> = self
            .models
            .iter()
            .map(|m| {
                strat
                    .and_then(|s| s.shares.iter().find(|(id, _)| id == &m.id).map(|(_, w)| *w))
                    .unwrap_or(1.0)
                    .max(0.0)
            })
            .collect();

        let sum: f32 = raw.iter().sum();
        if sum <= 1e-5 {
            let even = 100.0 / self.models.len() as f32;
            vec![even; self.models.len()]
        } else {
            raw.into_iter().map(|w| (w / sum) * 100.0).collect()
        }
    }

    /// Compute blended USD cost per 1K tokens for `strategy_id`.
    pub fn blended_cost_for(&self, strategy_id: &str) -> f64 {
        let shares = self.normalized_shares(strategy_id);
        self.models
            .iter()
            .zip(shares)
            .map(|(m, pct)| m.cost_per_1k * (pct as f64 / 100.0))
            .sum()
    }

    /// Switch the draft routing strategy (`"cost"`, `"balanced"`, `"quality"`).
    pub fn select_strategy(
        &mut self,
        strategy_id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let sid: SharedString = strategy_id.into();
        if self.draft_policy.strategy_id != sid && self.strategies.iter().any(|s| s.id == sid) {
            self.draft_policy.strategy_id = sid;
            self.open_picker = None;
            cx.emit(ModelRouterEvent::PolicyEdited(self.draft_policy.clone()));
            cx.notify();
        }
    }

    /// Toggle whether fallback failover is enabled.
    pub fn set_fallback_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.draft_policy.fallback.enabled != enabled {
            self.draft_policy.fallback.enabled = enabled;
            self.open_picker = None;
            cx.emit(ModelRouterEvent::PolicyEdited(self.draft_policy.clone()));
            cx.notify();
        }
    }

    /// Pick the source or target fallback model (automatically swapping if both would match).
    pub fn set_fallback_models(
        &mut self,
        from: Option<SharedString>,
        to: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let prev_from = self.draft_policy.fallback.from_model.clone();
        let prev_to = self.draft_policy.fallback.to_model.clone();
        if let Some(f) = from {
            if f == prev_to {
                self.draft_policy.fallback.to_model = prev_from;
            }
            self.draft_policy.fallback.from_model = f;
        }
        if let Some(t) = to {
            if t == self.draft_policy.fallback.from_model {
                self.draft_policy.fallback.from_model = prev_to;
            }
            self.draft_policy.fallback.to_model = t;
        }
        self.open_picker = None;
        cx.emit(ModelRouterEvent::PolicyEdited(self.draft_policy.clone()));
        cx.notify();
    }

    /// Revert draft edits back to the live policy.
    pub fn revert(&mut self, cx: &mut Context<Self>) {
        if self.has_unsaved_changes() {
            self.draft_policy = self.live_policy.clone();
            self.open_picker = None;
            cx.emit(ModelRouterEvent::Reverted(self.live_policy.clone()));
            cx.notify();
        }
    }

    /// Deploy the current draft policy as the new live baseline and increment `version`.
    pub fn deploy(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.has_unsaved_changes() {
            return false;
        }
        self.version += 1;
        self.live_policy = self.draft_policy.clone();
        self.open_picker = None;
        let msg = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("已发布策略 v{} 至 {}", self.version, self.environment)
        } else {
            format!("Deployed Policy v{} to {}", self.version, self.environment)
        };
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(ModelRouterEvent::Deployed {
            version: self.version,
            policy: self.live_policy.clone(),
        });
        cx.notify();
        true
    }

    fn sanitize_fallback(&mut self) {
        if self.models.len() < 2 {
            return;
        }
        if !self
            .models
            .iter()
            .any(|m| m.id == self.draft_policy.fallback.from_model)
        {
            self.draft_policy.fallback.from_model = self.models[0].id.clone();
        }
        if !self
            .models
            .iter()
            .any(|m| m.id == self.draft_policy.fallback.to_model)
            || self.draft_policy.fallback.to_model == self.draft_policy.fallback.from_model
        {
            self.draft_policy.fallback.to_model = self.models[1].id.clone();
        }
    }

    fn localize_label<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Model router" => tr(cx, "Model router", "模型流量路由器"),
            "Cost" => tr(cx, "Cost", "成本优先"),
            "Balanced" => tr(cx, "Balanced", "均衡"),
            "Quality" => tr(cx, "Quality", "质量优先"),
            other => other,
        }
    }
}

impl EventEmitter<ModelRouterEvent> for ModelRouter {}

impl Focusable for ModelRouter {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ModelRouter {
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
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(500.))
                        .p(px(18.))
                        .rounded(px(14.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(160., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(240., 32., 6., phase + 0.1, cx))
                        .child(pro_skeleton_bar(440., 136., 10., phase + 0.2, cx).w_full())
                        .child(pro_skeleton_bar(440., 128., 8., phase + 0.3, cx).w_full()),
                )
                .into_any_element();
        }

        if self.empty || self.models.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(500.))
                        .rounded(px(14.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Sliders,
                            tr(cx, "No models to route to", "暂无可路由模型"),
                            tr(
                                cx,
                                "Add candidate models and traffic strategies to configure gateway routing.",
                                "添加候选模型与分流策略后即可预览网关流量分配与混合成本。",
                            ),
                            Some((
                                pro_eid(&self.id, "restore-mr"),
                                tr(cx, "Load sample router", "加载示例路由").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    this.models = default_models();
                                    this.sanitize_fallback();
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        if !reduced {
            w.request_animation_frame();
        }
        let elapsed_s = self.boot_at.elapsed().as_secs_f32();

        let width_cell = self.measured_width.clone();
        let card_w = width_cell.get().max(300.0);
        let compact = card_w < 440.0;
        let ultra_narrow = card_w < 380.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.03)
        };
        let sel_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let track_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.07)
        };
        let focus_col = p.focus;

        let shares = self.normalized_shares(&self.draft_policy.strategy_id);
        let draft_cost = self.blended_cost_for(&self.draft_policy.strategy_id);
        let live_cost = self.blended_cost_for(&self.live_policy.strategy_id);
        let pct_delta = if live_cost > 1e-9 {
            ((draft_cost - live_cost) / live_cost) * 100.0
        } else {
            0.0
        };
        let dirty = self.has_unsaved_changes();

        if let Some(pk) = self.open_picker {
            self.last_picker = Some(pk);
        }

        // Sample spring-smoothed share values and per-model hover spotlight opacities!
        let spring_shares: Vec<f32> = shares
            .iter()
            .enumerate()
            .map(|(i, &target)| {
                pro_spring_f32(
                    pro_eid(&self.id, format!("band-sp-{i}")),
                    target,
                    0.38,
                    w,
                    cx,
                )
            })
            .collect();

        let model_fades: Vec<f32> = self
            .models
            .iter()
            .map(|m| {
                let dimmed = self.hovered_model.as_ref().is_some_and(|h| h != &m.id);
                pro_spring_f32(
                    pro_eid(&self.id, format!("m-fade-{}", m.id)),
                    if dimmed { 0.30 } else { 1.0 },
                    0.22,
                    w,
                    cx,
                )
            })
            .collect();

        let model_hovs: Vec<f32> = self
            .models
            .iter()
            .map(|m| {
                let is_hov = self.hovered_model.as_ref() == Some(&m.id);
                pro_spring_f32(
                    pro_eid(&self.id, format!("m-hov-{}", m.id)),
                    if is_hov { 1.0 } else { 0.0 },
                    0.18,
                    w,
                    cx,
                )
            })
            .collect();

        // 1. Header Row
        let head_row = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .truncate()
                    .child(self.localize_label(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .h(px(24.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(fill_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(6.)).rounded_full().bg(p.green))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .child(self.environment.clone()),
                    ),
            );

        // 2. Blended Cost + Preset Switcher
        let delta_badge: AnyElement = if pct_delta.abs() < 0.5 {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child("·")
                .child(tr(cx, "Live policy", "当前线上策略"))
                .into_any_element()
        } else {
            let cheaper = pct_delta < 0.0;
            let sign = if pct_delta > 0.0 { "+" } else { "-" };
            let sfx = tr(cx, "% vs live", "% 对比线上");
            let badge_fg = if cheaper { p.green_ink } else { p.red_ink };
            div()
                .h(px(19.))
                .px(px(6.))
                .rounded(px(5.))
                .bg(if cheaper { p.green_soft } else { p.red_soft })
                .flex()
                .items_center()
                .child(pro_rolling_number(
                    pro_eid(&self.id, "cost-delta"),
                    pct_delta.abs(),
                    0,
                    sign,
                    sfx,
                    11.5,
                    FontWeight::MEDIUM,
                    badge_fg,
                ))
                .into_any_element()
        };

        let cost_block = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "Blended cost", "混合推理成本"))
                    .child(delta_badge),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(6.))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "blended-cost"),
                        draft_cost,
                        4,
                        "$",
                        "",
                        22.,
                        FontWeight::SEMIBOLD,
                        p.strong,
                    ))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "/ 1K tokens",
                        "/ 千 Token",
                    ))),
            );

        let strat_pill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.11)
        } else {
            card_bg
        };
        let strat_sensor = pro_selection_sensor();
        let mut strat_tabs = div()
            .id(pro_eid(&self.id, "strat-tabs"))
            .role(Role::TabList)
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(fill_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "strat-ind"),
                self.draft_policy.strategy_id.clone(),
                strat_sensor.clone(),
                6.0,
                strat_pill_bg,
                Some(p.line),
                !p.dark,
            ));

        for strat in &self.strategies {
            let sid = strat.id.clone();
            let active = sid == self.draft_policy.strategy_id;
            let sid_click = sid.clone();
            let hov_ink = p.strong;
            strat_tabs = strat_tabs.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("stab-{sid}")))
                    .role(Role::RadioButton)
                    .aria_selected(active)
                    .tab_index(if active { 0 } else { -1 })
                    .relative()
                    .h(px(26.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .when(!active, |d| d.hover(move |s| s.text_color(hov_ink)))
                    .focus_visible(move |s| s.border_2().border_color(focus_col))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.strong } else { p.text })
                    .when(active, |d| d.child(pro_mark_selected(&strat_sensor)))
                    .child(self.localize_label(&strat.label, cx).to_owned()),
                cx,
                move |this, _, cx| {
                    this.select_strategy(sid_click.clone(), cx);
                },
            ));
        }

        let summary_row = if compact {
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(10.))
                .child(cost_block)
                .child(strat_tabs)
        } else {
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(12.))
                .child(cost_block)
                .child(strat_tabs)
        };

        // 3. Gateway Traffic-Splitting Ribbon Diagram
        let row_pitch = 30.0_f32;
        let flow_h = (self.models.len() as f32 * row_pitch + 24.0).max(96.0);
        let label_col_w = if ultra_narrow { 118.0_f32 } else { 150.0_f32 };
        let route_colors: Vec<Hsla> = self
            .models
            .iter()
            .enumerate()
            .map(|(i, m)| model_route_color(i, m.color, &p))
            .collect();
        let sp_shares_c = spring_shares.clone();
        let fades_c = model_fades.clone();
        let hovs_c = model_hovs.clone();
        let cols_c = route_colors.clone();
        let muted_port = p.muted;

        let flow_svg = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                let end_x = (bw - label_col_w - 10.0).max(90.0);
                let start_x = 6.0_f32;
                let elbow_x = (end_x * 0.27_f32).max(28.0);
                let join_x = (end_x * 0.56_f32).max(elbow_x + 24.0);
                let mid_y = bh * 0.5_f32;
                let count = sp_shares_c.len();
                let top0 = mid_y - ((count.saturating_sub(1)) as f32 * row_pitch) * 0.5_f32;

                for i in 0..count {
                    let y1 = top0 + i as f32 * row_pitch;
                    let share = sp_shares_c[i].max(0.0);
                    let col = cols_c[i];
                    let fade = fades_c.get(i).copied().unwrap_or(1.0);
                    let hov = hovs_c.get(i).copied().unwrap_or(0.0);

                    let pts = sample_route_curve(start_x, mid_y, elbow_x, join_x, end_x, y1);
                    let band_w = (2.2_f32 + share * 0.22_f32 + 1.2 * hov).clamp(2.2, 17.5);

                    // 1) Outer soft share-proportional ribbon band
                    let mut band = PathBuilder::stroke(px(band_w));
                    for (k, &(px_x, px_y)) in pts.iter().enumerate() {
                        let pt = point(px(ox + px_x), px(oy + px_y));
                        if k == 0 {
                            band.move_to(pt);
                        } else {
                            band.line_to(pt);
                        }
                    }
                    if let Ok(path) = band.build() {
                        window.paint_path(path, col.opacity((0.16 + 0.08 * hov) * fade));
                    }

                    // 2) Crisp centerline with soft AA feather underlay
                    let mut line_aa = PathBuilder::stroke(px(2.6 + 0.55 * hov));
                    let mut line = PathBuilder::stroke(px(1.25 + 0.55 * hov));
                    for (k, &(px_x, px_y)) in pts.iter().enumerate() {
                        let pt = point(px(ox + px_x), px(oy + px_y));
                        if k == 0 {
                            line_aa.move_to(pt);
                            line.move_to(pt);
                        } else {
                            line_aa.line_to(pt);
                            line.line_to(pt);
                        }
                    }
                    if let Ok(path_aa) = line_aa.build() {
                        window.paint_path(path_aa, col.opacity((0.14 + 0.10 * hov) * fade));
                    }
                    if let Ok(path) = line.build() {
                        window.paint_path(path, col.opacity((0.42 + 0.28 * hov) * fade));
                    }

                    // 3) Traveling streaks of light scaled to share %
                    if !reduced && share > 1.0 {
                        let speed = 0.55_f32 + (share / 100.0) * 0.65_f32;
                        let n_streaks = if share >= 35.0 { 2usize } else { 1usize };
                        let total_segs = (pts.len() - 1) as f32;
                        for s_idx in 0..n_streaks {
                            let phase =
                                ((elapsed_s * speed) + (i as f32 * 0.23) + (s_idx as f32 * 0.5))
                                    .fract();
                            let head = phase * total_segs;
                            let span = 6.5_f32;
                            let envelope = (phase * std::f32::consts::PI).sin().clamp(0.0, 1.0);
                            for k in 0..(pts.len() - 1) {
                                let mid = k as f32 + 0.5_f32;
                                let dist = (mid - head).abs();
                                if dist <= span {
                                    let fall = (1.0 - dist / span).clamp(0.0, 1.0);
                                    let a = fall * fall * envelope * fade;
                                    let mut seg = PathBuilder::stroke(px(2.4));
                                    seg.move_to(point(px(ox + pts[k].0), px(oy + pts[k].1)));
                                    seg.line_to(point(
                                        px(ox + pts[k + 1].0),
                                        px(oy + pts[k + 1].1),
                                    ));
                                    if let Ok(sp) = seg.build() {
                                        window.paint_path(sp, col.opacity(0.88 * a));
                                    }
                                }
                            }
                        }
                    }

                    // 4) Solid endpoint dot
                    let r_end = 3.5_f32;
                    window.paint_quad(quad(
                        Bounds::new(
                            point(px(ox + end_x - r_end), px(oy + y1 - r_end)),
                            size(px(r_end * 2.0), px(r_end * 2.0)),
                        ),
                        Corners::all(px(r_end)),
                        col.opacity(fade),
                        Edges::all(px(1.5)),
                        card_bg,
                        BorderStyle::Solid,
                    ));
                }

                // Gateway origin hollow port on left
                let r_gw = 4.0_f32;
                window.paint_quad(quad(
                    Bounds::new(
                        point(px(ox + start_x - r_gw), px(oy + mid_y - r_gw)),
                        size(px(r_gw * 2.0), px(r_gw * 2.0)),
                    ),
                    Corners::all(px(r_gw)),
                    card_bg,
                    Edges::all(px(1.5)),
                    muted_port,
                    BorderStyle::Solid,
                ));
            },
        )
        .absolute()
        .inset_0();

        let top0_labels =
            flow_h * 0.5_f32 - ((self.models.len().saturating_sub(1)) as f32 * row_pitch) * 0.5_f32;
        let mut flow_stage = div().w_full().h(px(flow_h)).relative().child(flow_svg);

        for (i, m) in self.models.iter().enumerate() {
            let y1 = top0_labels + i as f32 * row_pitch;
            let mid = m.id.clone();
            let fade = model_fades.get(i).copied().unwrap_or(1.0);
            flow_stage = flow_stage.child(
                div()
                    .id(pro_eid(&self.id, format!("flbl-{}", m.id)))
                    .absolute()
                    .right_0()
                    .top(px(y1 - 11.0))
                    .w(px(label_col_w))
                    .h(px(22.))
                    .cursor_pointer()
                    .opacity(fade)
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = (*inside).then(|| mid.clone());
                        if this.hovered_model != next {
                            this.hovered_model = next;
                            cx.notify();
                        }
                    }))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(
                        pro_app_artwork(&m.brand, 14., cx)
                            .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 13., p.ink)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .child(m.name.clone()),
                    ),
            );
        }

        // 4. Routes Table (`Role::Table`)
        let mut routes_table = div()
            .id(pro_eid(&self.id, "routes-tbl"))
            .role(Role::Table)
            .w_full()
            .flex()
            .flex_col();

        routes_table = routes_table.child(
            div()
                .id(pro_eid(&self.id, "r-head"))
                .role(Role::Row)
                .h(px(28.))
                .border_b_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .text_size(px(11.))
                .text_color(p.muted)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Routes", "路由分布")),
                )
                .child(
                    div()
                        .w(px(80.))
                        .text_right()
                        .child(tr(cx, "Share", "流量占比")),
                )
                .when(!compact, |d| {
                    d.child(div().w(px(52.)).text_right().child("p95"))
                })
                .child(div().w(px(66.)).text_right().child("$ / 1K")),
        );

        for (i, m) in self.models.iter().enumerate() {
            let pct = shares.get(i).copied().unwrap_or(0.0);
            let col = route_colors[i];
            let mid = m.id.clone();
            let is_hov = self.hovered_model.as_ref() == Some(&mid);
            let fade = model_fades.get(i).copied().unwrap_or(1.0);
            let hov_amt = model_hovs.get(i).copied().unwrap_or(0.0);

            routes_table =
                routes_table.child(
                    div()
                        .id(pro_eid(&self.id, format!("rrow-{}", m.id)))
                        .role(Role::Row)
                        .h(px(32.))
                        .px(px(4.))
                        .mx(px(-4.))
                        .rounded(px(6.))
                        .bg(fill_bg.opacity(fill_bg.a * hov_amt))
                        .opacity(fade)
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            let next = (*inside).then(|| mid.clone());
                            if this.hovered_model != next {
                                this.hovered_model = next;
                                cx.notify();
                            }
                        }))
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .child(div().size(px(7.)).rounded_full().bg(col))
                                .child(
                                    pro_app_artwork(&m.brand, 14., cx)
                                        .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 13., p.ink)),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(13.))
                                        .text_color(p.ink)
                                        .child(m.name.clone()),
                                )
                                .when(is_hov && !compact, |d| {
                                    d.child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child(m.provider.clone()),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .w(px(80.))
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(px(6.))
                                .child(div().w(px(28.)).child(pro_spring_bar(
                                    pro_eid(&self.id, format!("sbar-{}", m.id)),
                                    pct / 100.0,
                                    4.0,
                                    2.0,
                                    track_bg,
                                    col,
                                )))
                                .child(div().w(px(32.)).flex().justify_end().child(
                                    pro_rolling_number(
                                        pro_eid(&self.id, format!("spct-{}", m.id)),
                                        pct as f64,
                                        0,
                                        "",
                                        "%",
                                        12.,
                                        FontWeight::NORMAL,
                                        p.ink,
                                    ),
                                )),
                        )
                        .when(!compact, |d| {
                            d.child(div().w(px(52.)).flex().justify_end().child(
                                pro_rolling_number(
                                    pro_eid(&self.id, format!("rp95-{}", m.id)),
                                    m.p95_secs as f64,
                                    2,
                                    "",
                                    " s",
                                    12.,
                                    FontWeight::NORMAL,
                                    p.ink,
                                ),
                            ))
                        })
                        .child(
                            div()
                                .w(px(66.))
                                .flex()
                                .justify_end()
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("rcost-{}", m.id)),
                                    m.cost_per_1k,
                                    4,
                                    "$",
                                    "",
                                    12.,
                                    FontWeight::NORMAL,
                                    p.ink,
                                )),
                        ),
                );
        }

        // 5. Fallback Rule Bar + Model Pickers (when >= 2 models)
        let fallback_section = if self.models.len() >= 2 {
            let fb_on = self.draft_policy.fallback.enabled;
            let from_m = self
                .models
                .iter()
                .find(|m| m.id == self.draft_policy.fallback.from_model)
                .unwrap_or(&self.models[0])
                .clone();
            let to_m = self
                .models
                .iter()
                .find(|m| m.id == self.draft_policy.fallback.to_model)
                .unwrap_or(&self.models[1])
                .clone();

            let fb_controls_op = pro_spring_f32(
                pro_eid(&self.id, "fb-on-op"),
                if fb_on { 1.0 } else { 0.42 },
                0.20,
                w,
                cx,
            );

            let make_picker_btn = |which: OpenPicker,
                                   model: &RouterModel,
                                   aria_prefix: &str,
                                   cx: &mut Context<Self>| {
                let is_open = self.open_picker == Some(which);
                let suffix = match which {
                    OpenPicker::From => "pick-from",
                    OpenPicker::To => "pick-to",
                };
                pro_tactile(
                    pro_eid(&self.id, suffix),
                    format!("{aria_prefix}: {}", model.name),
                    7.0,
                    card_bg,
                    sel_bg,
                    if is_open { focus_col } else { p.line_strong },
                    focus_col,
                    is_open,
                    div()
                        .h(px(26.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(
                            pro_app_artwork(&model.brand, 13.5, cx)
                                .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 12., p.ink)),
                        )
                        .when(!ultra_narrow, |d| {
                            d.child(
                                div()
                                    .max_w(px(108.))
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.ink)
                                    .child(model.name.clone()),
                            )
                        })
                        .child(pro_icon(ProIcon::ChevronDown, 11., p.muted)),
                    cx,
                    move |this, _, cx| {
                        if this.open_picker == Some(which) {
                            this.open_picker = None;
                        } else {
                            this.last_picker = Some(which);
                            this.open_picker = Some(which);
                        }
                        cx.notify();
                    },
                )
                .aria_expanded(is_open)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
            };

            let picker_which = self.open_picker.or(self.last_picker);
            let picker_open = self.open_picker.is_some();
            let picker_menu = picker_which.map(|which| {
                let menu_bg: Hsla = if p.dark {
                    rgb(0x232326).into()
                } else {
                    rgb(0xffffff).into()
                };
                let cur_id = match which {
                    OpenPicker::From => self.draft_policy.fallback.from_model.clone(),
                    OpenPicker::To => self.draft_policy.fallback.to_model.clone(),
                };
                let mut list = div()
                    .id(pro_eid(&self.id, "fb-menu"))
                    .role(Role::ListBox)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .w(px(196.))
                    .p(px(4.))
                    .rounded(px(9.))
                    .bg(menu_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(2.));

                for m in &self.models {
                    let mid = m.id.clone();
                    let sel = mid == cur_id;
                    let mid_pick = mid.clone();
                    list = list.child(
                        pro_tactile(
                            pro_eid(&self.id, format!("fb-opt-{mid}")),
                            m.name.clone(),
                            6.0,
                            if sel { sel_bg } else { transparent_black() },
                            sel_bg,
                            transparent_black(),
                            transparent_black(),
                            sel,
                            div()
                                .w_full()
                                .h(px(28.))
                                .px(px(8.))
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .child(
                                    pro_app_artwork(&m.brand, 13.5, cx)
                                        .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 12., p.ink)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .text_color(p.ink)
                                        .child(m.name.clone()),
                                )
                                .when(sel, |d| {
                                    d.child(pro_check_draw(
                                        pro_eid(&self.id, format!("fb-chk-{mid}")),
                                        true,
                                        13.0,
                                        false,
                                        p.ink,
                                    ))
                                }),
                            cx,
                            move |this, _, cx| match which {
                                OpenPicker::From => {
                                    this.set_fallback_models(Some(mid_pick.clone()), None, cx)
                                }
                                OpenPicker::To => {
                                    this.set_fallback_models(None, Some(mid_pick.clone()), cx)
                                }
                            },
                        )
                        .role(Role::ListBoxOption)
                        .aria_selected(sel)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                    );
                }

                div()
                    .absolute()
                    .right_0()
                    .bottom(relative(1.0))
                    .pb(px(6.))
                    .child(pro_popover_presence(
                        pro_eid(&self.id, "fb-menu-pres"),
                        picker_open,
                        move || list.into_any_element(),
                    ))
            });

            Some(
                div()
                    .relative()
                    .w_full()
                    .min_h(px(42.))
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(10.))
                    .bg(fill_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_switch(
                                pro_eid(&self.id, "fb-sw"),
                                tr(cx, "Fallback", "启用失败回退"),
                                fb_on,
                                cx,
                                move |this, _, cx| {
                                    this.set_fallback_enabled(!fb_on, cx);
                                },
                            ))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Fallback", "自动回退")),
                            ),
                    )
                    .child(
                        div()
                            .opacity(fb_controls_op)
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(make_picker_btn(
                                OpenPicker::From,
                                &from_m,
                                tr(cx, "If this model fails", "若此模型失败"),
                                cx,
                            ))
                            .child(pro_icon(ProIcon::ArrowRight, 13., p.muted))
                            .child(make_picker_btn(
                                OpenPicker::To,
                                &to_m,
                                tr(cx, "Retry on", "重试于"),
                                cx,
                            )),
                    )
                    .when_some(picker_menu, |d, menu| d.child(menu)),
            )
        } else {
            None
        };

        // 6. Footer Status + Revert / Deploy Policy Buttons
        let foot_status_txt = if dirty {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("有未发布更改 · 对比 v{}", self.version)
            } else {
                format!("Unsaved changes · Policy v{}", self.version)
            }
        } else if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("策略 v{} · 已在 {} 生效", self.version, self.environment)
        } else {
            format!("Policy v{} · Live on {}", self.version, self.environment)
        };

        let deploy_btn: AnyElement = if dirty {
            pro_btn(
                pro_eid(&self.id, "deploy-btn"),
                tr(cx, "Deploy policy", "发布策略"),
                Some(ProIcon::Check),
                ProBtnStyle::Primary,
                true,
                cx,
                |this, _, cx| {
                    this.deploy(cx);
                },
            )
            .into_any_element()
        } else {
            div()
                .id(pro_eid(&self.id, "deploy-disabled"))
                .role(Role::Button)
                .aria_label(tr(cx, "Deploy policy", "发布策略"))
                .aria_description(tr(cx, "No unsaved changes", "当前无未保存更改"))
                .cursor_default()
                .h(px(28.))
                .px(px(10.))
                .rounded(px(7.))
                .bg(fill_bg)
                .border_1()
                .border_color(p.line)
                .opacity(0.55)
                .flex()
                .items_center()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(tr(cx, "Deploy policy", "发布策略"))
                .into_any_element()
        };

        let footer = div()
            .id(pro_eid(&self.id, "mr-footer"))
            .role(Role::Status)
            .pt(px(2.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(div().size(px(6.)).rounded_full().bg(if dirty {
                        p.amber
                    } else {
                        p.green
                    }))
                    .child(foot_status_txt),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when(dirty, |d| {
                        d.child(pro_btn(
                            pro_eid(&self.id, "revert-btn"),
                            tr(cx, "Revert", "还原"),
                            Some(ProIcon::Undo),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| {
                                this.revert(cx);
                            },
                        ))
                    })
                    .child(deploy_btn),
            );

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_label(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "escape" if this.open_picker.is_some() => {
                        this.open_picker = None;
                        cx.notify();
                    }
                    "left" if this.open_picker.is_none() => {
                        if let Some(pos) = this
                            .strategies
                            .iter()
                            .position(|s| s.id == this.draft_policy.strategy_id)
                            && !this.strategies.is_empty()
                        {
                            let next = (pos + this.strategies.len() - 1) % this.strategies.len();
                            let sid = this.strategies[next].id.clone();
                            this.select_strategy(sid, cx);
                        }
                    }
                    "right" if this.open_picker.is_none() => {
                        if let Some(pos) = this
                            .strategies
                            .iter()
                            .position(|s| s.id == this.draft_policy.strategy_id)
                            && !this.strategies.is_empty()
                        {
                            let next = (pos + 1) % this.strategies.len();
                            let sid = this.strategies[next].id.clone();
                            this.select_strategy(sid, cx);
                        }
                    }
                    _ => {}
                }
            }))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                if this.open_picker.is_some() {
                    this.open_picker = None;
                    cx.notify();
                }
            }))
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(492.))
                    .relative()
                    .pt(px(16.))
                    .px(px(18.))
                    .pb(px(14.))
                    .rounded(px(14.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .gap(px(12.))
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
                    .child(head_row)
                    .child(summary_row)
                    .child(flow_stage)
                    .child(routes_table)
                    .when_some(fallback_section, |d, fb| d.child(fb))
                    .child(footer)
                    .child(pro_toast_banner(self.toast.clone())),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::ModelRouter;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn model_router_computes_blended_cost_reverts_and_deploys_versions(cx: &mut TestAppContext) {
        let view = cx.new(|cx| ModelRouter::new("test-model-router", cx));

        view.update(cx, |mr, cx| {
            assert_eq!(mr.version(), 14);
            assert!(!mr.has_unsaved_changes());

            let bal_cost = mr.blended_cost_for("balanced");
            let cost_cost = mr.blended_cost_for("cost");
            let qual_cost = mr.blended_cost_for("quality");
            assert!(cost_cost < bal_cost);
            assert!(qual_cost > bal_cost);

            mr.select_strategy("cost", cx);
            assert!(mr.has_unsaved_changes());

            mr.revert(cx);
            assert!(!mr.has_unsaved_changes());

            mr.select_strategy("quality", cx);
            // Selecting "gpt" as `from` when `to` is already "gpt" swaps cleanly so from != to
            mr.set_fallback_models(Some("gpt".into()), None, cx);
            assert_ne!(
                mr.draft_policy.fallback.from_model,
                mr.draft_policy.fallback.to_model
            );

            assert!(mr.deploy(cx));
            assert_eq!(mr.version(), 15);
            assert!(!mr.has_unsaved_changes());
        });
    }
}
