//! Compact multi-model reasoning breakdown card with cognitive bar, effort meters, and source trace.
//!
//! Summarizes how an agent spent its thinking time across cognitive step kinds (`Search`, `Read`,
//! `Plan`, `Verify`) with a twinkling agent mark, multi-segment proportional bar, cross-linked hover
//! highlighting, clickable model token/cost disclosures, and collapsible sources and step traces.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_app_tile, pro_eid, pro_empty_state, pro_fold,
        pro_shimmer_text, pro_skeleton_bar, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, person_initials, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const CARD_MAX_W: f32 = 428.0;

/// Cognitive activity category of a [`ReasonStep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ReasonKind {
    /// Querying documentation, web pages, or issue trackers.
    #[default]
    Search,
    /// Reading and synthesizing retrieved files or schemas.
    Read,
    /// Designing architecture, retries, or code edits.
    Plan,
    /// Running sandboxed checks, tests, or invariants.
    Verify,
}

impl ReasonKind {
    /// Ordered canonical list of cognitive step kinds.
    pub const ALL: [Self; 4] = [Self::Search, Self::Read, Self::Plan, Self::Verify];

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Search => tr(cx, "Search", "检索"),
            Self::Read => tr(cx, "Read", "阅读分析"),
            Self::Plan => tr(cx, "Plan", "方案规划"),
            Self::Verify => tr(cx, "Verify", "沙箱验证"),
        }
    }

    fn icon(self) -> ProIcon {
        match self {
            Self::Search => ProIcon::Search,
            Self::Read => ProIcon::PageDoc,
            Self::Plan => ProIcon::Code,
            Self::Verify => ProIcon::Shield,
        }
    }

    fn color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Search => p.red,
            Self::Read => p.amber,
            Self::Plan => p.blue,
            Self::Verify => p.green,
        }
    }
}

/// Reasoning compute effort level (`1..=4` lit bars) of a [`ReasonModel`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffortLevel {
    /// Light pass (`1` bar).
    Low,
    /// Balanced pass (`2` bars).
    #[default]
    Medium,
    /// Deep pass (`3` bars).
    High,
    /// Maximum extended thinking (`4` bars).
    Max,
}

impl EffortLevel {
    fn lit_bars(self) -> usize {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Max => 4,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Low => tr(cx, "low effort", "低推理强度"),
            Self::Medium => tr(cx, "medium effort", "中推理强度"),
            Self::High => tr(cx, "high effort", "高推理强度"),
            Self::Max => tr(cx, "max effort", "最高推理强度"),
        }
    }
}

/// One sequential reasoning step rendered as a bar segment and trace item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReasonStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Cognitive step category (`Search`, `Read`, `Plan`, or `Verify`).
    pub kind: ReasonKind,
    /// Concise summary of what this step did.
    pub label: SharedString,
    /// Optional Chinese step summary.
    pub label_zh: Option<SharedString>,
    /// Participating model identifier linking this step to a [`ReasonModel`].
    pub model_id: Option<SharedString>,
    /// Elapsed milliseconds spent in this step.
    pub ms: u64,
    /// Whether this step is still actively executing.
    pub running: bool,
}

impl ReasonStep {
    /// Create a [`ReasonStep`] with `id`, `kind`, `label`, and `ms`.
    pub fn new(
        id: impl Into<SharedString>,
        kind: ReasonKind,
        label: impl Into<SharedString>,
        ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            label_zh: None,
            model_id: None,
            ms,
            running: false,
        }
    }

    /// Provide a localized Chinese step summary.
    pub fn with_label_zh(mut self, label_zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(label_zh.into());
        self
    }

    /// Link this step to a participating [`ReasonModel`] id.
    pub fn with_model(mut self, model_id: impl Into<SharedString>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    /// Mark this step as actively executing (`running = true`).
    pub fn with_running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> &'a str {
        match &self.label_zh {
            Some(zh) => tr(cx, self.label.as_ref(), zh.as_ref()),
            None => self.label.as_ref(),
        }
    }
}

/// One participating model row in [`ReasoningSummary`].
#[derive(Clone, Debug, PartialEq)]
pub struct ReasonModel {
    /// Stable model identifier referenced by `ReasonStep::model_id`.
    pub id: SharedString,
    /// Model display name (for example `"Claude Opus 4.6"`).
    pub name: SharedString,
    /// Artwork lookup key passed to [`pro_app_artwork`] (`"claude"`, `"xai"`, `"openai"`, `"gemini"`).
    pub icon: SharedString,
    /// Compute effort level (`Low`, `Medium`, `High`, or `Max`).
    pub effort: EffortLevel,
    /// Role played during the answer (`"Researching"`, `"Planning"`, `"Verifying"`).
    pub role: SharedString,
    /// Optional Chinese role label.
    pub role_zh: Option<SharedString>,
    /// Model provider name (`"Anthropic"`, `"xAI"`, `"OpenAI"`).
    pub provider: SharedString,
    /// Prompt / context input token count.
    pub input_tokens: u64,
    /// Generated reasoning + output token count.
    pub output_tokens: u64,
    /// Incremental USD cost attributed to this model.
    pub cost: f64,
    /// One-line explanation of why the router selected this model.
    pub note: Option<SharedString>,
    /// Optional Chinese selection note.
    pub note_zh: Option<SharedString>,
}

impl ReasonModel {
    /// Create a [`ReasonModel`] with `id`, `name`, `icon`, and `provider`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        icon: impl Into<SharedString>,
        provider: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            icon: icon.into(),
            effort: EffortLevel::Medium,
            role: SharedString::from("Reasoning"),
            role_zh: None,
            provider: provider.into(),
            input_tokens: 0,
            output_tokens: 0,
            cost: 0.0,
            note: None,
            note_zh: None,
        }
    }

    /// Set the thinking effort level (`1..=4` meter bars).
    pub fn with_effort(mut self, effort: EffortLevel) -> Self {
        self.effort = effort;
        self
    }

    /// Set the short role label shown on the right of the model row.
    pub fn with_role(mut self, role: impl Into<SharedString>) -> Self {
        self.role = role.into();
        self
    }

    /// Provide a localized Chinese role label.
    pub fn with_role_zh(mut self, role_zh: impl Into<SharedString>) -> Self {
        self.role_zh = Some(role_zh.into());
        self
    }

    /// Set input and output token counts shown when expanded.
    pub fn with_tokens(mut self, input_tokens: u64, output_tokens: u64) -> Self {
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self
    }

    /// Set the USD cost shown when expanded.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost;
        self
    }

    /// Set the explanatory router note shown when expanded.
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Provide a localized Chinese router note.
    pub fn with_note_zh(mut self, note_zh: impl Into<SharedString>) -> Self {
        self.note_zh = Some(note_zh.into());
        self
    }

    fn display_role<'a>(&'a self, cx: &App) -> &'a str {
        match &self.role_zh {
            Some(zh) => tr(cx, self.role.as_ref(), zh.as_ref()),
            None => self.role.as_ref(),
        }
    }

    fn display_note<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.note.as_deref()?;
        Some(match &self.note_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }
}

/// One searched integration or document source in [`ReasoningSummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReasonSource {
    /// Stable source identifier.
    pub id: SharedString,
    /// Source product or repository name (`"GitHub"`, `"Notion"`, `"Linear"`).
    pub name: SharedString,
    /// App icon key passed to [`pro_app_tile`].
    pub icon: SharedString,
    /// Hit count or document title detail.
    pub detail: SharedString,
    /// Optional Chinese detail text.
    pub detail_zh: Option<SharedString>,
}

impl ReasonSource {
    /// Create a [`ReasonSource`] entry.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        icon: impl Into<SharedString>,
        detail: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            icon: icon.into(),
            detail: detail.into(),
            detail_zh: None,
        }
    }

    /// Provide a localized Chinese source detail.
    pub fn with_detail_zh(mut self, detail_zh: impl Into<SharedString>) -> Self {
        self.detail_zh = Some(detail_zh.into());
        self
    }

    fn display_detail<'a>(&'a self, cx: &App) -> &'a str {
        match &self.detail_zh {
            Some(zh) => tr(cx, self.detail.as_ref(), zh.as_ref()),
            None => self.detail.as_ref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FocusHighlight {
    Kind(ReasonKind),
    Model(SharedString),
    Step(SharedString),
}

/// Typed events emitted by [`ReasoningSummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReasoningSummaryEvent {
    /// User expanded or collapsed a participating model's token/cost details.
    ModelToggled(Option<SharedString>),
    /// User toggled the collapsible searched sources drawer.
    SourcesToggled(bool),
    /// User toggled the collapsible step trace drawer.
    StepsToggled(bool),
}

impl EventEmitter<ReasoningSummaryEvent> for ReasoningSummary {}

fn format_secs_ms(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1_000.0)
}

fn format_tok(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Compact multi-model reasoning breakdown card with cross-linked highlighting and collapsible traces.
pub struct ReasoningSummary {
    id: ElementId,
    agent_seed: SharedString,
    steps: Vec<ReasonStep>,
    models: Vec<ReasonModel>,
    sources: Vec<ReasonSource>,
    source_count: usize,
    running: bool,
    estimate_ms: Option<u64>,
    expanded_model: Option<SharedString>,
    sources_open: bool,
    steps_open: bool,
    highlight: Option<FocusHighlight>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl ReasoningSummary {
    /// Create a [`ReasoningSummary`] preloaded with a 5-step multi-model reasoning run across 24 sources.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            agent_seed: SharedString::from("kense-reasoner"),
            steps: default_reason_steps(),
            models: default_reason_models(),
            sources: default_reason_sources(),
            source_count: 24,
            running: true,
            estimate_ms: Some(11_400),
            expanded_model: None,
            sources_open: false,
            steps_open: false,
            highlight: None,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Set the deterministic seed used for the 3x3 agent pixel mark.
    pub fn with_agent_seed(mut self, seed: impl Into<SharedString>) -> Self {
        self.agent_seed = seed.into();
        self
    }

    /// Supply ordered reasoning steps.
    pub fn with_steps(mut self, steps: Vec<ReasonStep>) -> Self {
        self.running = steps.iter().any(|s| s.running);
        self.steps = steps;
        self
    }

    /// Supply participating model metadata rows.
    pub fn with_models(mut self, models: Vec<ReasonModel>) -> Self {
        self.models = models;
        self
    }

    /// Supply searched source entries.
    pub fn with_sources(mut self, sources: Vec<ReasonSource>) -> Self {
        self.source_count = self.source_count.max(sources.len());
        self.sources = sources;
        self
    }

    /// Set the total searched source count (`+ N more` shown when higher than `sources.len()`).
    pub fn with_source_count(mut self, source_count: usize) -> Self {
        self.source_count = source_count;
        self
    }

    /// Explicitly control whether the agent is still actively reasoning.
    pub fn with_running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    /// Set the estimated total reasoning duration in milliseconds (renders trailing grey track room).
    pub fn with_estimate_ms(mut self, estimate_ms: Option<u64>) -> Self {
        self.estimate_ms = estimate_ms;
        self
    }

    /// Open the collapsible step trace drawer on initial mount.
    pub fn with_default_steps_open(mut self, open: bool) -> Self {
        self.steps_open = open;
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state placeholder.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the configured reasoning steps.
    pub fn steps(&self) -> &[ReasonStep] {
        &self.steps
    }

    /// Return the configured participating models.
    pub fn models(&self) -> &[ReasonModel] {
        &self.models
    }

    /// Return the configured searched sources.
    pub fn sources(&self) -> &[ReasonSource] {
        &self.sources
    }

    /// Return total base milliseconds across `steps`.
    pub fn total_ms(&self) -> u64 {
        self.steps.iter().map(|s| s.ms).sum()
    }

    /// Return the currently expanded model id, if any.
    pub fn expanded_model_id(&self) -> Option<&str> {
        self.expanded_model.as_deref()
    }

    /// Return whether the searched sources drawer is open.
    pub fn is_sources_open(&self) -> bool {
        self.sources_open
    }

    /// Return whether the step trace drawer is open.
    pub fn is_steps_open(&self) -> bool {
        self.steps_open
    }

    /// Toggle a model's provider/tokens/cost disclosure and emit [`ReasoningSummaryEvent::ModelToggled`].
    pub fn toggle_model(&mut self, model_id: &str, cx: &mut Context<Self>) {
        self.expanded_model = if self.expanded_model.as_deref() == Some(model_id) {
            None
        } else {
            self.models
                .iter()
                .find(|m| m.id.as_ref() == model_id)
                .map(|m| m.id.clone())
        };
        cx.emit(ReasoningSummaryEvent::ModelToggled(
            self.expanded_model.clone(),
        ));
        cx.notify();
    }

    /// Toggle the collapsible searched sources drawer and emit [`ReasoningSummaryEvent::SourcesToggled`].
    pub fn toggle_sources(&mut self, cx: &mut Context<Self>) {
        self.sources_open = !self.sources_open;
        if self.sources_open {
            self.steps_open = false;
        }
        cx.emit(ReasoningSummaryEvent::SourcesToggled(self.sources_open));
        cx.notify();
    }

    /// Toggle the collapsible step trace drawer and emit [`ReasoningSummaryEvent::StepsToggled`].
    pub fn toggle_steps(&mut self, cx: &mut Context<Self>) {
        self.steps_open = !self.steps_open;
        if self.steps_open {
            self.sources_open = false;
        }
        cx.emit(ReasoningSummaryEvent::StepsToggled(self.steps_open));
        cx.notify();
    }

    fn effective_step_ms(&self, step: &ReasonStep, live_bonus_ms: u64) -> u64 {
        if step.running && self.running {
            step.ms.saturating_add(live_bonus_ms)
        } else {
            step.ms
        }
    }
}

fn model_logo_tile(icon: &str, name: &str, side: f32, p: &ProPalette, cx: &App) -> AnyElement {
    let bg: Hsla = if p.dark { rgb(0x232327).into() } else { p.card };
    let art_sz = (side * 0.60).round();
    let content = if let Some(art) = pro_app_artwork(icon, art_sz, cx) {
        art
    } else {
        let init = person_initials(name);
        div()
            .text_size(px(10.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink)
            .child(if init.is_empty() {
                "M".to_string()
            } else {
                init
            })
            .into_any_element()
    };
    div()
        .size(px(side))
        .flex_none()
        .rounded(px(6.))
        .bg(bg)
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .child(content)
        .into_any_element()
}

impl Render for ReasoningSummary {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_s * 1_000.0;
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            p.panel
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.06)
        };

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_s / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(CARD_MAX_W))
                .p(px(16.))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(190., 15., 4., phase, cx))
                .child(pro_skeleton_bar(CARD_MAX_W - 32., 7., 2., phase + 0.1, cx))
                .child(pro_skeleton_bar(CARD_MAX_W - 32., 28., 6., phase + 0.2, cx))
                .child(pro_skeleton_bar(CARD_MAX_W - 32., 28., 6., phase + 0.3, cx))
                .into_any_element();
        }

        if self.empty || self.steps.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(CARD_MAX_W))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state(
                    ProIcon::Memory,
                    tr(cx, "No reasoning steps recorded", "暂无推理记录"),
                    tr(
                        cx,
                        "Attach cognitive steps and model telemetry to summarize how an answer was produced.",
                        "传入认知步骤与模型耗用详情，即可在此汇总答案推理全貌。",
                    ),
                    Some((
                        pro_eid(&self.id, "load-reason"),
                        SharedString::from(tr(cx, "Load reasoning trace", "加载推理摘要示例")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.steps = default_reason_steps();
                            this.models = default_reason_models();
                            this.sources = default_reason_sources();
                            this.running = true;
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let is_live = self.running && self.steps.iter().any(|s| s.running);
        if is_live && !reduced {
            window.request_animation_frame();
        }

        // Smooth bounded live growth on the active segment (oscillating/growing gently up to +1.8s)
        let live_bonus_ms = if is_live && !reduced {
            ((elapsed_ms as u64) % 3_200).min(1_800)
        } else {
            0
        };

        let step_ms_vec: Vec<u64> = self
            .steps
            .iter()
            .map(|s| self.effective_step_ms(s, live_bonus_ms))
            .collect();
        let total_ms: u64 = step_ms_vec.iter().sum::<u64>().max(1);
        let bar_denom_ms = if is_live {
            self.estimate_ms.unwrap_or(total_ms + 1_800).max(total_ms)
        } else {
            total_ms
        };

        // Determine active cross-highlight
        let active_hl = self
            .highlight
            .clone()
            .or_else(|| self.expanded_model.clone().map(FocusHighlight::Model));

        let step_matches = |step: &ReasonStep| -> bool {
            match &active_hl {
                None => true,
                Some(FocusHighlight::Kind(k)) => step.kind == *k,
                Some(FocusHighlight::Model(mid)) => step.model_id.as_ref() == Some(mid),
                Some(FocusHighlight::Step(sid)) => &step.id == sid,
            }
        };

        // Header: twinkling pixel mark + status label + total clock
        let head_label = if is_live {
            pro_shimmer_text(
                pro_eid(&self.id, "head-shim"),
                tr(cx, "Reasoning…", "正在深度推理…").to_string(),
                p.text,
                p.strong,
                13.0,
                FontWeight::NORMAL,
                true,
            )
        } else {
            div()
                .text_size(px(13.))
                .text_color(p.text)
                .child(
                    tr(
                        cx,
                        &format!("Reasoned for {}", format_secs_ms(total_ms)),
                        &format!("推理完成 · 共耗时 {}", format_secs_ms(total_ms)),
                    )
                    .to_string(),
                )
                .into_any_element()
        };

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .min_w_0()
                    .child(pro_agent_avatar(
                        self.agent_seed.as_ref(),
                        p.blue,
                        18.0,
                        is_live,
                        elapsed_ms,
                        cx,
                    ))
                    .child(head_label),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(format_secs_ms(total_ms)),
            );

        // Multi-segment horizontal cognitive bar
        let mut seg_bar = div()
            .id(pro_eid(&self.id, "seg-bar"))
            .role(Role::Image)
            .aria_label(tr(
                cx,
                "Reasoning time breakdown by step",
                "按认知步骤划分的推理耗时条",
            ))
            .w_full()
            .h(px(7.))
            .flex()
            .items_center()
            .gap(px(2.));

        let sweep_phase = (elapsed_s / 1.55).fract();
        for (idx, step) in self.steps.iter().enumerate() {
            let sms = step_ms_vec[idx].max(80);
            let frac = (sms as f32 / bar_denom_ms as f32).clamp(0.03, 1.0);
            let lit = step_matches(step);
            let k_col = step.kind.color(&p);
            let sid_hov = step.id.clone();
            let seg_running = step.running && is_live;

            let seg = div()
                .id(pro_eid(&self.id, format!("seg-{}", step.id)))
                .relative()
                .h_full()
                .w(relative(frac))
                .min_w(px(4.))
                .rounded(px(2.))
                .overflow_hidden()
                .bg(k_col)
                .opacity(if lit { 1.0 } else { 0.25 })
                .cursor_pointer()
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    this.highlight = if *inside {
                        Some(FocusHighlight::Step(sid_hov.clone()))
                    } else if this.highlight == Some(FocusHighlight::Step(sid_hov.clone())) {
                        None
                    } else {
                        this.highlight.clone()
                    };
                    cx.notify();
                }))
                .when(seg_running && !reduced, |d| {
                    d.child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, _| {
                                let w = f32::from(bounds.size.width);
                                if w <= 4.0 {
                                    return;
                                }
                                let band = (w * 0.45).clamp(8.0, 36.0);
                                let bx = -band + sweep_phase * (w + band * 1.3);
                                let cl_l = bx.max(0.0);
                                let cl_r = (bx + band).min(w);
                                if cl_r > cl_l + 0.5 {
                                    let r = Bounds::new(
                                        point(bounds.left() + px(cl_l), bounds.top()),
                                        size(px(cl_r - cl_l), bounds.size.height),
                                    );
                                    window.paint_quad(fill(r, hsla(0., 0., 1., 0.48)));
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                });
            seg_bar = seg_bar.child(seg);
        }

        if bar_denom_ms > total_ms {
            let rest_frac =
                ((bar_denom_ms - total_ms) as f32 / bar_denom_ms as f32).clamp(0.02, 0.5);
            seg_bar = seg_bar.child(div().h_full().w(relative(rest_frac)).rounded(px(2.)).bg(
                if p.dark {
                    hsla(0., 0., 1., 0.06)
                } else {
                    hsla(0., 0., 0., 0.045)
                },
            ));
        }

        // 2-column Kind pills with icons and aggregated durations
        let mut kind_pills = Vec::new();
        for kind in ReasonKind::ALL {
            let sum_k: u64 = self
                .steps
                .iter()
                .enumerate()
                .filter(|(_, s)| s.kind == kind)
                .map(|(i, _)| step_ms_vec[i])
                .sum();
            if sum_k == 0 {
                continue;
            }
            let lit = match &active_hl {
                None => true,
                Some(FocusHighlight::Kind(k)) => *k == kind,
                Some(FocusHighlight::Model(mid)) => self
                    .steps
                    .iter()
                    .any(|s| s.kind == kind && s.model_id.as_ref() == Some(mid)),
                Some(FocusHighlight::Step(sid)) => {
                    self.steps.iter().any(|s| &s.id == sid && s.kind == kind)
                }
            };
            let k_col = kind.color(&p);
            let hov_bg = p.chip;
            let pill = div()
                .id(pro_eid(&self.id, format!("kind-{kind:?}")))
                .role(Role::Button)
                .aria_label(format!("{} {}", kind.label(cx), format_secs_ms(sum_k)))
                .tab_index(0)
                .flex_1()
                .min_w(px(140.))
                .h(px(28.))
                .px(px(6.))
                .rounded(px(6.))
                .opacity(if lit { 1.0 } else { 0.40 })
                .cursor_pointer()
                .hover(move |s| s.bg(hov_bg))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.highlight = Some(FocusHighlight::Kind(kind));
                    } else if this.highlight == Some(FocusHighlight::Kind(kind)) {
                        this.highlight = None;
                    }
                    cx.notify();
                }))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .size(px(20.))
                        .flex_none()
                        .rounded(px(5.))
                        .bg(k_col.opacity(0.13))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(kind.icon(), 12.5, k_col)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.5))
                        .text_color(p.text)
                        .child(kind.label(cx)),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(format_secs_ms(sum_k)),
                );
            kind_pills.push(pill);
        }

        let mut kinds_grid = div().w_full().flex().flex_col().gap(px(2.));
        let mut iter = kind_pills.into_iter();
        while let Some(first) = iter.next() {
            let mut row = div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(first);
            if let Some(second) = iter.next() {
                row = row.child(second);
            } else {
                row = row.child(div().flex_1());
            }
            kinds_grid = kinds_grid.child(row);
        }

        let top_block = div()
            .w_full()
            .px(px(16.))
            .py(px(14.))
            .flex()
            .flex_col()
            .gap(px(11.))
            .child(header)
            .child(seg_bar)
            .child(kinds_grid);

        // Participating models block with 4-bar effort meter & clickable pro_fold token/cost details
        let mut models_block = div()
            .w_full()
            .px(px(16.))
            .py(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        for model in &self.models {
            let m_steps: Vec<(ReasonKind, u64)> = self
                .steps
                .iter()
                .enumerate()
                .filter(|(_, s)| s.model_id.as_ref() == Some(&model.id))
                .map(|(i, s)| (s.kind, step_ms_vec[i]))
                .collect();
            let m_ms: u64 = m_steps.iter().map(|(_, ms)| *ms).sum();

            // Dominant kind color for the 4-bar effort meter
            let dom_kind = ReasonKind::ALL
                .into_iter()
                .max_by_key(|&k| {
                    m_steps
                        .iter()
                        .filter(|&&(sk, _)| sk == k)
                        .map(|&(_, ms)| ms)
                        .sum::<u64>()
                })
                .unwrap_or(ReasonKind::Plan);
            let dom_col = dom_kind.color(&p);

            let lit = match &active_hl {
                None => true,
                Some(FocusHighlight::Model(mid)) => mid == &model.id,
                Some(FocusHighlight::Kind(k)) => m_steps.iter().any(|&(sk, _)| sk == *k),
                Some(FocusHighlight::Step(sid)) => self
                    .steps
                    .iter()
                    .any(|s| &s.id == sid && s.model_id.as_ref() == Some(&model.id)),
            };

            let lit_bars = model.effort.lit_bars();
            let mut effort_el = div()
                .h(px(9.))
                .px(px(2.))
                .flex_none()
                .flex()
                .items_end()
                .gap(px(1.5));
            for b_i in 0..4usize {
                let bh = [3.5_f32, 5.2, 7.0, 9.0][b_i];
                let bar_on = b_i < lit_bars;
                effort_el =
                    effort_el.child(div().w(px(2.5)).h(px(bh)).rounded(px(0.8)).bg(if bar_on {
                        dom_col
                    } else {
                        p.strong.opacity(0.14)
                    }));
            }

            let is_expanded = self.expanded_model.as_ref() == Some(&model.id);
            let mid_hov = model.id.clone();
            let mid_click = model.id.clone();
            let row_hover = p.chip;
            let aria = format!(
                "{}, {}, {}, {}",
                model.name,
                model.display_role(cx),
                format_secs_ms(m_ms),
                model.effort.label(cx)
            );

            let model_row = div()
                .id(pro_eid(&self.id, format!("mdl-{}", model.id)))
                .role(Role::Button)
                .aria_label(aria)
                .tab_index(0)
                .w_full()
                .min_h(px(32.))
                .px(px(6.))
                .mx(px(-6.))
                .rounded(px(6.))
                .bg(if is_expanded {
                    p.chip
                } else {
                    transparent_black()
                })
                .opacity(if lit { 1.0 } else { 0.38 })
                .cursor_pointer()
                .hover(move |s| s.bg(row_hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.highlight = Some(FocusHighlight::Model(mid_hov.clone()));
                    } else if this.highlight == Some(FocusHighlight::Model(mid_hov.clone())) {
                        this.highlight = None;
                    }
                    cx.notify();
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.toggle_model(mid_click.as_ref(), cx);
                }))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(model_logo_tile(
                    model.icon.as_ref(),
                    model.name.as_ref(),
                    22.0,
                    &p,
                    cx,
                ))
                .child(
                    div()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(model.name.clone()),
                )
                .child(effort_el)
                .child(
                    div()
                        .ml_auto()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(model.display_role(cx).to_string()),
                )
                .child(
                    div()
                        .flex_none()
                        .min_w(px(34.))
                        .text_right()
                        .text_size(px(12.5))
                        .text_color(p.ink)
                        .child(format_secs_ms(m_ms)),
                );

            let detail_fold = {
                let provider = model.provider.to_string();
                let tok_str = tr(
                    cx,
                    &format!(
                        "{} in · {} out",
                        format_tok(model.input_tokens),
                        format_tok(model.output_tokens)
                    ),
                    &format!(
                        "输入 {} · 输出 {}",
                        format_tok(model.input_tokens),
                        format_tok(model.output_tokens)
                    ),
                )
                .to_string();
                let cost_str = format!("${:.3}", model.cost);
                let note_opt = model.display_note(cx).map(str::to_string);
                let effort_txt = model.effort.label(cx).to_string();
                let segs: Vec<(Hsla, f32)> = if m_ms > 0 {
                    m_steps
                        .iter()
                        .map(|&(sk, ms)| (sk.color(&p), (ms as f32 / m_ms as f32).max(0.05)))
                        .collect()
                } else {
                    Vec::new()
                };

                pro_fold(
                    pro_eid(&self.id, format!("mfold-{}", model.id)),
                    is_expanded,
                    58.0,
                    0.0,
                    move || {
                        let mut mini = div().w_full().h(px(4.)).flex().gap(px(2.));
                        for (sc, sf) in segs {
                            mini =
                                mini.child(div().h_full().w(relative(sf)).rounded(px(2.)).bg(sc));
                        }
                        div()
                            .pl(px(30.))
                            .pt(px(4.))
                            .pb(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .child(mini)
                            .child(div().text_size(px(11.5)).text_color(p.text).child(format!(
                                "{provider} · {effort_txt} · {tok_str} · {cost_str}"
                            )))
                            .when_some(note_opt, |d, note| {
                                d.child(
                                    div()
                                        .text_size(px(11.5))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(note),
                                )
                            })
                            .into_any_element()
                    },
                )
            };

            models_block = models_block.child(model_row).child(detail_fold);
        }

        // Footer: stacked source icons + "Searched 24 sources" button on left, Steps toggle on right
        let mut icon_stack = div().flex().items_center().gap(px(3.));
        for src in self.sources.iter().take(4) {
            icon_stack = icon_stack.child(pro_app_tile(src.icon.as_ref(), 18.0, cx));
        }

        let src_hover = p.chip;
        let toggle_hover = p.chip;
        let sources_label = tr(
            cx,
            &format!("Searched {} sources", self.source_count),
            &format!("已检索 {} 个来源", self.source_count),
        )
        .to_string();

        let footer_block = div()
            .w_full()
            .px(px(16.))
            .py(px(9.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .id(pro_eid(&self.id, "src-btn"))
                    .role(Role::Button)
                    .aria_label(sources_label.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(6.))
                    .mx(px(-6.))
                    .rounded(px(6.))
                    .bg(if self.sources_open {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .cursor_pointer()
                    .hover(move |s| s.bg(src_hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.toggle_sources(cx);
                    }))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon_stack)
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(if self.sources_open { p.ink } else { p.text })
                            .child(sources_label),
                    ),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "steps-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Toggle step trace", "展开或收起推理步骤明细"))
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .bg(if self.steps_open {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .cursor_pointer()
                    .hover(move |s| s.bg(toggle_hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.toggle_steps(cx);
                    }))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .text_color(if self.steps_open { p.ink } else { p.muted })
                    .child(pro_icon(
                        ProIcon::List,
                        14.,
                        if self.steps_open { p.ink } else { p.muted },
                    ))
                    .child(
                        tr(
                            cx,
                            &format!("{} steps", self.steps.len()),
                            &format!("{} 步", self.steps.len()),
                        )
                        .to_string(),
                    ),
            );

        // Collapsible Sources drawer (pro_fold)
        let sources_fold = {
            let src_rows: Vec<(AnyElement, String, String)> = self
                .sources
                .iter()
                .map(|s| {
                    (
                        pro_app_tile(s.icon.as_ref(), 20.0, cx),
                        s.name.to_string(),
                        s.display_detail(cx).to_string(),
                    )
                })
                .collect();
            let extra = self.source_count.saturating_sub(self.sources.len());
            let more_txt = (extra > 0).then(|| {
                tr(
                    cx,
                    &format!("+ {extra} more sources"),
                    &format!("+ 其余 {extra} 个来源"),
                )
                .to_string()
            });
            let est_h = (src_rows.len() + usize::from(extra > 0)) as f32 * 30.0 + 14.0;
            pro_fold(
                pro_eid(&self.id, "sources-fold"),
                self.sources_open,
                est_h,
                0.0,
                move || {
                    let mut col = div()
                        .w_full()
                        .px(px(14.))
                        .pt(px(6.))
                        .pb(px(10.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(2.));
                    for (tile, name, det) in src_rows {
                        col = col.child(
                            div()
                                .h(px(30.))
                                .px(px(2.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(tile)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .text_color(p.ink)
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .child(det),
                                ),
                        );
                    }
                    if let Some(m) = more_txt {
                        col = col.child(
                            div()
                                .h(px(26.))
                                .pl(px(28.))
                                .flex()
                                .items_center()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(m),
                        );
                    }
                    col.into_any_element()
                },
            )
        };

        // Collapsible Steps Trace drawer (pro_fold)
        let steps_fold = {
            type StepTraceRow = (ProIcon, Hsla, AnyElement, Option<AnyElement>, String, bool);
            let trace_rows: Vec<StepTraceRow> = self
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let k_col = s.kind.color(&p);
                    let sms = step_ms_vec[i];
                    let s_run = s.running && is_live;
                    let lbl = s.display_label(cx).to_string();
                    let label_el = if s_run {
                        pro_shimmer_text(
                            pro_eid(&self.id, format!("tshim-{}", s.id)),
                            lbl,
                            p.ink,
                            p.strong,
                            12.0,
                            FontWeight::NORMAL,
                            true,
                        )
                    } else {
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .child(lbl)
                            .into_any_element()
                    };
                    let m_art = s
                        .model_id
                        .as_ref()
                        .and_then(|mid| self.models.iter().find(|m| &m.id == mid))
                        .and_then(|m| pro_app_artwork(m.icon.as_ref(), 12.0, cx));
                    (
                        s.kind.icon(),
                        k_col,
                        label_el,
                        m_art,
                        format_secs_ms(sms),
                        s_run,
                    )
                })
                .collect();
            let est_h = trace_rows.len() as f32 * 31.0 + 14.0;
            pro_fold(
                pro_eid(&self.id, "steps-fold"),
                self.steps_open,
                est_h,
                0.0,
                move || {
                    let mut col = div()
                        .w_full()
                        .px(px(14.))
                        .pt(px(6.))
                        .pb(px(10.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(2.));
                    for (ic, k_col, label_el, m_art, t_str, s_run) in trace_rows {
                        col = col.child(
                            div()
                                .h(px(30.))
                                .px(px(2.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .size(px(20.))
                                        .flex_none()
                                        .rounded(px(5.))
                                        .bg(k_col.opacity(0.13))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ic, 12.5, k_col)),
                                )
                                .child(div().flex_1().min_w_0().overflow_hidden().child(label_el))
                                .when_some(m_art, |d, art| d.child(div().opacity(0.85).child(art)))
                                .child(
                                    div()
                                        .min_w(px(34.))
                                        .text_right()
                                        .text_size(px(12.))
                                        .text_color(if s_run { p.ink } else { p.muted })
                                        .child(t_str),
                                ),
                        );
                    }
                    col.into_any_element()
                },
            )
        };

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(CARD_MAX_W))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .when(!p.dark, |d| d.shadow_sm())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(top_block)
                    .child(models_block)
                    .child(footer_block)
                    .child(sources_fold)
                    .child(steps_fold),
            )
            .into_any_element()
    }
}

fn default_reason_steps() -> Vec<ReasonStep> {
    vec![
        ReasonStep::new(
            "s1",
            ReasonKind::Search,
            "Searched API docs & GitHub issues for 429 limits",
            1_800,
        )
        .with_label_zh("检索 API 文档与 GitHub Issue 中的 429 限流规则")
        .with_model("opus"),
        ReasonStep::new(
            "s2",
            ReasonKind::Search,
            "Queried Linear tickets for burst incidents",
            1_400,
        )
        .with_label_zh("检索 Linear 工单中的突发限流故障单")
        .with_model("opus"),
        ReasonStep::new(
            "s3",
            ReasonKind::Read,
            "Compared token bucket headers across three tiers",
            2_100,
        )
        .with_label_zh("对比三档套餐的令牌桶响应头与窗口策略")
        .with_model("grok"),
        ReasonStep::new(
            "s4",
            ReasonKind::Plan,
            "Designed adaptive backoff & queue jitter policy",
            2_300,
        )
        .with_label_zh("设计自适应指数退避与队列抖动调度策略")
        .with_model("gpt"),
        ReasonStep::new(
            "s5",
            ReasonKind::Verify,
            "Verifying retry schedule against sandbox burst",
            1_900,
        )
        .with_label_zh("在沙箱突发流量下验证重试退避曲线")
        .with_model("gpt")
        .with_running(true),
    ]
}

fn default_reason_models() -> Vec<ReasonModel> {
    vec![
        ReasonModel::new("opus", "Claude Opus 4.6", "claude", "Anthropic")
            .with_effort(EffortLevel::Medium)
            .with_role("Researching")
            .with_role_zh("检索取证")
            .with_tokens(18_400, 1_920)
            .with_cost(0.086)
            .with_note("Picked for fast parallel synthesis across docs and issue threads.")
            .with_note_zh("负责跨文档与工单仓库的高并发证据归纳。"),
        ReasonModel::new("grok", "Grok 3", "xai", "xAI")
            .with_effort(EffortLevel::High)
            .with_role("Analyzing")
            .with_role_zh("对比分析")
            .with_tokens(12_800, 2_450)
            .with_cost(0.064)
            .with_note("Cross-checked rate-limit reset headers against live traces.")
            .with_note_zh("交叉核验限流重置响应头与线上调用日志。"),
        ReasonModel::new("gpt", "GPT-5", "openai", "OpenAI")
            .with_effort(EffortLevel::Max)
            .with_role("Planning & verifying")
            .with_role_zh("规划与验证")
            .with_tokens(24_100, 4_680)
            .with_cost(0.142)
            .with_note("Extended thinking pass to prove zero starvation under burst load.")
            .with_note_zh("启用深度推理证明突发负载下无队列饥饿。"),
    ]
}

fn default_reason_sources() -> Vec<ReasonSource> {
    vec![
        ReasonSource::new("src-gh", "GitHub", "github", "11 issues · 3 PRs")
            .with_detail_zh("11 个 Issue · 3 个 PR"),
        ReasonSource::new("src-nt", "Notion", "notion", "API rate limits")
            .with_detail_zh("API 速率限制规范"),
        ReasonSource::new("src-ln", "Linear", "linear", "6 incident tickets")
            .with_detail_zh("6 张故障复盘工单"),
        ReasonSource::new("src-fg", "Figma", "figma", "Quota warning banner")
            .with_detail_zh("配额预警横幅稿"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ReasonKind, ReasoningSummary};
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn toggles_model_details_and_mutually_exclusive_source_step_drawers(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| ReasoningSummary::new("test-rs", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.steps().len(), 5);
            assert_eq!(v.models().len(), 3);
            assert_eq!(v.total_ms(), 9_500);
            assert!(
                v.steps()
                    .iter()
                    .any(|s| s.kind == ReasonKind::Verify && s.running)
            );

            v.toggle_model("opus", cx);
            assert_eq!(v.expanded_model_id(), Some("opus"));
            v.toggle_model("opus", cx);
            assert_eq!(v.expanded_model_id(), None);

            v.toggle_sources(cx);
            assert!(v.is_sources_open());
            assert!(!v.is_steps_open());

            v.toggle_steps(cx);
            assert!(v.is_steps_open());
            assert!(!v.is_sources_open());
        });
    }
}
