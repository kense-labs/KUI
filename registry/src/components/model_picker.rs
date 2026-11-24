//! Interactive LLM selector menu with spring capability bars, thinking effort, and rolling cost.
//!
//! Displays a trigger pill and floating model menu where hovering or arrow-keying
//! any model smoothly fills its `Speed`, `Quality`, and `Cost` spring bars,
//! toggling `"Extended thinking"` unfolds `Low`/`Medium`/`High` effort, and the
//! footer tray rolls first-token latency and per-1M token pricing.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
        pro_swap_label, pro_switch, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

const SVG_BULB: &str = r#"<path d="M5.3 10.6h3.4M5.8 12.3h2.4M7 1.7a3.9 3.9 0 0 0-2.3 7c.4.3.6.7.6 1.1v.1h3.4v-.1c0-.4.2-.8.6-1.1A3.9 3.9 0 0 0 7 1.7Z" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Reasoning effort level unfolded when `"Extended thinking"` is enabled in [`ModelPicker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThinkingEffort {
    /// Light reasoning pass (`×1.6` first-token time).
    Low,
    /// Balanced reasoning pass (`×2.5` first-token time).
    #[default]
    Medium,
    /// Deep multi-step reasoning pass (`×4.0` first-token time).
    High,
}

impl ThinkingEffort {
    /// Latency multiplier applied while `"Extended thinking"` is enabled.
    pub fn latency_multiplier(self) -> f64 {
        match self {
            Self::Low => 1.6,
            Self::Medium => 2.5,
            Self::High => 4.0,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Low => tr(cx, "Low", "低"),
            Self::Medium => tr(cx, "Medium", "中"),
            Self::High => tr(cx, "High", "高"),
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Low,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Low => Self::High,
            Self::Medium => Self::Low,
            Self::High => Self::Medium,
        }
    }
}

/// Horizontal alignment of the [`ModelPicker`] menu relative to its trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelMenuAlign {
    /// Align menu's leading edge with the trigger.
    #[default]
    Start,
    /// Align menu's trailing edge with the trigger.
    End,
}

/// Vertical placement of the [`ModelPicker`] menu relative to its trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModelMenuSide {
    /// Open underneath the trigger.
    #[default]
    Bottom,
    /// Open above the trigger (for bottom-docked chat composers).
    Top,
}

/// Capability scores (`1..=5`), baseline latency, and token pricing for a [`PickerModel`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerModelMetrics {
    /// Throughput score (`1..=5`).
    pub speed: u8,
    /// Reasoning and output quality score (`1..=5`).
    pub quality: u8,
    /// Relative cost tier (`1..=5`, where `5` is highest cost).
    pub cost: u8,
    /// Baseline time-to-first-token in milliseconds without extended thinking.
    pub latency_ms: u32,
    /// Price in USD per `1M` blended tokens.
    pub price_per_m: f64,
}

/// Selectable LLM entry inside [`ModelPicker`].
#[derive(Clone, Debug, PartialEq)]
pub struct PickerModel {
    /// Stable model identifier (for example `"claude-sonnet"`).
    pub id: SharedString,
    /// Display name shown in the trigger and option row.
    pub name: SharedString,
    /// Brand lookup key passed to `pro_app_tile` / `pro_app_artwork`.
    pub provider: SharedString,
    /// Two-to-three-word subtitle shown below the model name.
    pub description: SharedString,
    /// Capability scores and pricing; `None` renders `"Varies"` for dynamic routers like `Auto`.
    pub metrics: Option<PickerModelMetrics>,
}

impl PickerModel {
    /// Construct a [`PickerModel`] without fixed metrics (`"Varies"` router).
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        provider: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: provider.into(),
            description: description.into(),
            metrics: None,
        }
    }

    /// Attach `1..=5` capability scores, baseline latency (`ms`), and USD per `1M` token price.
    pub fn with_metrics(
        mut self,
        speed: u8,
        quality: u8,
        cost: u8,
        latency_ms: u32,
        price_per_m: f64,
    ) -> Self {
        self.metrics = Some(PickerModelMetrics {
            speed: speed.clamp(1, 5),
            quality: quality.clamp(1, 5),
            cost: cost.clamp(1, 5),
            latency_ms,
            price_per_m: price_per_m.max(0.0),
        });
        self
    }
}

/// Typed events emitted by [`ModelPicker`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelPickerEvent {
    /// User picked a model id.
    ModelChanged(SharedString),
    /// `"Extended thinking"` switch toggled.
    ThinkingChanged(bool),
    /// Thinking effort (`Low`, `Medium`, `High`) changed.
    EffortChanged(ThinkingEffort),
    /// `"Web search"` switch toggled.
    WebSearchChanged(bool),
    /// Menu opened (`true`) or closed (`false`).
    OpenChanged(bool),
}

/// Compute effective `(first_token_seconds, price_per_million_usd)` for the footer tray.
pub fn compute_effective_tray_stats(
    metrics: PickerModelMetrics,
    thinking: bool,
    effort: ThinkingEffort,
    web_search: bool,
) -> (f64, f64) {
    let base_secs = f64::from(metrics.latency_ms) / 1000.0;
    let mult = if thinking {
        effort.latency_multiplier()
    } else {
        1.0
    };
    let search_overhead = if web_search { 0.35 } else { 0.0 };
    let secs = base_secs * mult + search_overhead;
    let price = if thinking {
        match effort {
            ThinkingEffort::Low => metrics.price_per_m * 1.15,
            ThinkingEffort::Medium => metrics.price_per_m * 1.35,
            ThinkingEffort::High => metrics.price_per_m * 1.60,
        }
    } else {
        metrics.price_per_m
    };
    (secs, price)
}

fn default_models() -> Vec<PickerModel> {
    vec![
        PickerModel::new(
            "claude-opus",
            "Claude Opus 4.5",
            "claude",
            "Hardest problems",
        )
        .with_metrics(2, 5, 5, 2100, 25.0),
        PickerModel::new(
            "claude-sonnet",
            "Claude Sonnet 4.5",
            "claude",
            "Everyday work",
        )
        .with_metrics(4, 4, 3, 900, 15.0),
        PickerModel::new("gpt-5", "GPT-5", "openai", "Broad reasoning")
            .with_metrics(4, 5, 4, 1050, 18.0),
        PickerModel::new("gemini-2.5-pro", "Gemini 2.5 Pro", "gemini", "2M context")
            .with_metrics(5, 4, 2, 480, 7.0),
        PickerModel::new("deepseek-r1", "DeepSeek R1", "deepseek", "Open weights")
            .with_metrics(3, 4, 1, 650, 2.0),
        PickerModel::new("auto", "Auto", "kense", "Picks per message"),
    ]
}

/// Interactive LLM selector menu with spring capability bars, thinking effort fold, and rolling stats.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::model_picker::{ModelPicker, ModelPickerEvent, ThinkingEffort};
///
/// fn mount(cx: &mut gpui::App) {
///     let picker = cx.new(|cx| {
///         ModelPicker::new("composer-model-picker", cx)
///             .with_value("claude-sonnet")
///             .with_thinking(true)
///             .with_effort(ThinkingEffort::Medium)
///     });
///     cx.subscribe(&picker, |_, ev: &ModelPickerEvent, _| {
///         let _ = ev;
///     })
///     .detach();
/// }
/// ```
pub struct ModelPicker {
    id: ElementId,
    focus: FocusHandle,
    models: Vec<PickerModel>,
    selected_idx: usize,
    inspected_idx: usize,
    pointing: bool,
    thinking: bool,
    effort: ThinkingEffort,
    web_search: bool,
    open: bool,
    align: ModelMenuAlign,
    side: ModelMenuSide,
    demo_chrome: bool,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<ModelPickerEvent> for ModelPicker {}

impl ModelPicker {
    /// Create a [`ModelPicker`] preloaded with `Claude`, `GPT`, `Gemini`, `DeepSeek`, and `Auto`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            models: default_models(),
            selected_idx: 1,
            inspected_idx: 1,
            pointing: false,
            thinking: false,
            effort: ThinkingEffort::Medium,
            web_search: false,
            open: true,
            align: ModelMenuAlign::Start,
            side: ModelMenuSide::Bottom,
            demo_chrome: true,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(520.)),
        }
    }

    /// Supply custom [`PickerModel`] entries.
    pub fn with_models(mut self, models: Vec<PickerModel>) -> Self {
        if !models.is_empty() {
            self.models = models;
            self.selected_idx = 0;
            self.inspected_idx = 0;
        }
        self
    }

    /// Set the initially chosen model by `id`.
    pub fn with_value(mut self, model_id: impl Into<SharedString>) -> Self {
        let target = model_id.into();
        if let Some(pos) = self.models.iter().position(|m| m.id == target) {
            self.selected_idx = pos;
            self.inspected_idx = pos;
        }
        self
    }

    /// Set initial `"Extended thinking"` toggle state.
    pub fn with_thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }

    /// Set initial [`ThinkingEffort`] (`Low`, `Medium`, or `High`).
    pub fn with_effort(mut self, effort: ThinkingEffort) -> Self {
        self.effort = effort;
        self
    }

    /// Set initial `"Web search"` toggle state.
    pub fn with_web_search(mut self, web_search: bool) -> Self {
        self.web_search = web_search;
        self
    }

    /// Control whether the menu starts open (`true` by default for standalone inspection).
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Align the menu with the `Start` or `End` edge of the trigger.
    pub fn with_align(mut self, align: ModelMenuAlign) -> Self {
        self.align = align;
        self
    }

    /// Open the menu below (`Bottom`) or above (`Top`) the trigger.
    pub fn with_side(mut self, side: ModelMenuSide) -> Self {
        self.side = side;
        self
    }

    /// Frame the trigger inside a compact composer toolbar (`true` by default) or bare (`false`).
    pub fn with_demo_chrome(mut self, demo_chrome: bool) -> Self {
        self.demo_chrome = demo_chrome;
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state presentation.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected [`PickerModel`].
    pub fn selected_model(&self) -> Option<&PickerModel> {
        self.models.get(self.selected_idx)
    }

    /// Return whether `"Extended thinking"` is currently enabled.
    pub fn is_thinking(&self) -> bool {
        self.thinking
    }

    /// Return the active [`ThinkingEffort`].
    pub fn effort(&self) -> ThinkingEffort {
        self.effort
    }

    /// Pick a model by index and emit [`ModelPickerEvent::ModelChanged`].
    pub fn select_index(&mut self, index: usize, close_menu: bool, cx: &mut Context<Self>) {
        if let Some(model) = self.models.get(index) {
            let id = model.id.clone();
            self.selected_idx = index;
            self.inspected_idx = index;
            cx.emit(ModelPickerEvent::ModelChanged(id));
            if close_menu && self.open {
                self.open = false;
                cx.emit(ModelPickerEvent::OpenChanged(false));
            }
            cx.notify();
        }
    }

    /// Toggle `"Extended thinking"` and unfold or fold the reasoning effort control.
    pub fn toggle_thinking(&mut self, cx: &mut Context<Self>) {
        self.thinking = !self.thinking;
        cx.emit(ModelPickerEvent::ThinkingChanged(self.thinking));
        cx.notify();
    }

    /// Set the [`ThinkingEffort`] level (`Low`, `Medium`, `High`).
    pub fn set_effort(&mut self, effort: ThinkingEffort, cx: &mut Context<Self>) {
        if self.effort != effort {
            self.effort = effort;
            cx.emit(ModelPickerEvent::EffortChanged(effort));
            cx.notify();
        }
    }

    /// Toggle `"Web search"`.
    pub fn toggle_web_search(&mut self, cx: &mut Context<Self>) {
        self.web_search = !self.web_search;
        cx.emit(ModelPickerEvent::WebSearchChanged(self.web_search));
        cx.notify();
    }

    fn toggle_open(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            self.inspected_idx = self.selected_idx;
        }
        cx.emit(ModelPickerEvent::OpenChanged(self.open));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if !self.open {
            if matches!(key, "enter" | "space" | "down") {
                self.toggle_open(cx);
            }
            return;
        }
        let len = self.models.len();
        match key {
            "escape" => {
                self.open = false;
                self.pointing = false;
                cx.emit(ModelPickerEvent::OpenChanged(false));
                cx.notify();
            }
            "down" if len > 0 => {
                self.pointing = true;
                self.inspected_idx = (self.inspected_idx + 1) % len;
                cx.notify();
            }
            "up" if len > 0 => {
                self.pointing = true;
                self.inspected_idx = if self.inspected_idx == 0 {
                    len - 1
                } else {
                    self.inspected_idx - 1
                };
                cx.notify();
            }
            "home" if len > 0 => {
                self.pointing = true;
                self.inspected_idx = 0;
                cx.notify();
            }
            "end" if len > 0 => {
                self.pointing = true;
                self.inspected_idx = len - 1;
                cx.notify();
            }
            "left" if self.thinking => {
                self.set_effort(self.effort.prev(), cx);
            }
            "right" if self.thinking => {
                self.set_effort(self.effort.next(), cx);
            }
            "enter" | "space" if len > 0 => {
                self.select_index(self.inspected_idx, false, cx);
            }
            _ => {}
        }
    }
}

impl Render for ModelPicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let width = self.measured_width.get();
        let measure_cell = self.measured_width.clone();
        let narrow = width < 350.;

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(360.))
                        .p(px(14.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(140., 26., 6., 0.25, cx))
                        .child(pro_skeleton_bar(320., 42., 6., 0.50, cx))
                        .child(pro_skeleton_bar(320., 42., 6., 0.75, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(360.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Sliders,
                            tr(cx, "No models configured", "尚未配置可用模型"),
                            tr(
                                cx,
                                "Add at least one provider model to compare speed, quality, and token cost.",
                                "请添加至少一个大语言模型以对比速度、质量与推理成本。",
                            ),
                            Some((
                                pro_eid(&self.id, "restore-models"),
                                tr(cx, "Load default models", "加载默认模型列表").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                );
        }

        let selected_model = self
            .models
            .get(self.selected_idx)
            .cloned()
            .unwrap_or_else(|| {
                PickerModel::new("claude-sonnet", "Claude Sonnet 4.5", "claude", "")
            });

        let active_idx = self.inspected_idx.min(self.models.len().saturating_sub(1));
        let inspected_model = self.models.get(active_idx).cloned();

        // Trigger button with crossfading model name (`pro_tactile` + `pro_swap_label`) + bulb indicator.
        let trigger_inner = div()
            .h(px(28.))
            .pl(px(6.))
            .pr(px(7.))
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                if let Some(mark) = pro_app_artwork(&selected_model.provider, 15., cx) {
                    mark
                } else {
                    pro_app_tile(&selected_model.provider, 18., cx)
                },
            )
            .child(pro_swap_label(
                pro_eid(&self.id, "trigger-name-lbl"),
                selected_model.name.clone(),
                13.,
                FontWeight::MEDIUM,
                if self.open { p.strong } else { p.text },
            ))
            .when(self.thinking, |d| {
                d.child(pro_svg("0 0 14 14", SVG_BULB, 14., p.blue))
            })
            .child(pro_icon(
                if self.open {
                    ProIcon::ChevronUp
                } else {
                    ProIcon::ChevronDown
                },
                13.,
                p.muted,
            ));

        let trigger_btn = pro_tactile(
            pro_eid(&self.id, "mdp-trigger"),
            format!(
                "Model: {}{}",
                selected_model.name,
                if self.thinking {
                    ", Extended thinking on"
                } else {
                    ""
                }
            ),
            6.,
            if self.open {
                p.hover
            } else {
                transparent_black()
            },
            p.hover,
            transparent_black(),
            transparent_black(),
            self.open,
            trigger_inner,
            cx,
            |this, _, cx| this.toggle_open(cx),
        )
        .flex_none()
        .aria_expanded(self.open);

        let trigger_bar = if self.demo_chrome {
            div()
                .w_full()
                .max_w(px(368.))
                .p(px(10.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .shadow_xs()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .px(px(4.))
                        .text_size(px(13.))
                        .text_color(p.faint)
                        .child(tr(cx, "Ask anything…", "输入任意问题…")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(trigger_btn)
                        .child(
                            div()
                                .size(px(28.))
                                .rounded(px(6.))
                                .bg(p.primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::ArrowUp, 14., p.on_primary)),
                        ),
                )
                .into_any_element()
        } else {
            div()
                .w_full()
                .max_w(px(368.))
                .flex()
                .when(self.align == ModelMenuAlign::End, |d| d.justify_end())
                .child(trigger_btn)
                .into_any_element()
        };

        // Menu Popover (`pro_presence`)
        let menu_body: AnyElement = {
            let models = self.models.clone();
            let selected_idx = self.selected_idx;
            let pointing = self.pointing;
            let thinking = self.thinking;
            let effort = self.effort;
            let web_search = self.web_search;
            let inspected_model = inspected_model.clone();
            let base_id = self.id.clone();
            let entity = cx.entity().clone();

            // Column header (`MODEL` | `SPEED` `QUALITY` `COST`)
            let head = div()
                .h(px(26.))
                .px(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.muted)
                .child(tr(cx, "MODEL", "模型"))
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .w(px(42.))
                                    .text_center()
                                    .child(tr(cx, "SPEED", "速度")),
                            )
                            .child(
                                div()
                                    .w(px(42.))
                                    .text_center()
                                    .child(tr(cx, "QUALITY", "质量")),
                            )
                            .child(div().w(px(42.)).text_center().child(tr(cx, "COST", "成本"))),
                    )
                });

            // Model rows with liquid `pro_pill_indicator`, `pro_check_draw`, and `pro_spring_bar`
            let row_sensor = pro_selection_sensor();
            let mut rows_col =
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&base_id, "mdp-row-pill"),
                        format!("row-{active_idx}"),
                        row_sensor.clone(),
                        7.,
                        p.chip,
                        Some(p.line_soft),
                        false,
                    ));
            let track_col = if p.dark {
                hsla(0., 0., 1., 0.08)
            } else {
                hsla(0., 0., 0., 0.07)
            };
            let rest_fill = if p.dark {
                hsla(0., 0., 1., 0.25)
            } else {
                hsla(0., 0., 0., 0.22)
            };

            for (mi, m) in models.into_iter().enumerate() {
                let is_selected = mi == selected_idx;
                let is_lit = mi == active_idx;
                let dim_row = pointing && !is_lit;
                let ent_hov = entity.clone();
                let ent_clk = entity.clone();
                let lit_t = pro_spring_f32(
                    pro_eid(&base_id, format!("bar-lit-{mi}")),
                    f32::from(is_lit),
                    0.22,
                    w,
                    cx,
                )
                .clamp(0., 1.);

                let meters_el = if narrow {
                    div().into_any_element()
                } else if let Some(met) = m.metrics {
                    let scale = if is_lit { 1.0 } else { 0.42 };
                    let r_speed = (f32::from(met.speed) / 5.0) * scale;
                    let q_bonus = if thinking && is_lit && met.quality < 5 {
                        0.12
                    } else {
                        0.0
                    };
                    let r_qual = ((f32::from(met.quality) / 5.0 + q_bonus).min(1.0)) * scale;
                    let r_cost = (f32::from(met.cost) / 5.0) * scale;
                    let fill_c = pro_mix_color(rest_fill, p.blue, lit_t);

                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().w(px(42.)).child(pro_spring_bar(
                            pro_eid(&base_id, format!("bar-sp-{mi}")),
                            r_speed,
                            4.5,
                            2.25,
                            track_col,
                            fill_c,
                        )))
                        .child(div().w(px(42.)).child(pro_spring_bar(
                            pro_eid(&base_id, format!("bar-ql-{mi}")),
                            r_qual,
                            4.5,
                            2.25,
                            track_col,
                            fill_c,
                        )))
                        .child(div().w(px(42.)).child(pro_spring_bar(
                            pro_eid(&base_id, format!("bar-cs-{mi}")),
                            r_cost,
                            4.5,
                            2.25,
                            track_col,
                            fill_c,
                        )))
                        .into_any_element()
                } else {
                    div()
                        .w(px(142.))
                        .flex_none()
                        .text_center()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(tr(cx, "Varies", "动态匹配"))
                        .into_any_element()
                };

                rows_col = rows_col.child(
                    div()
                        .id(pro_eid(&base_id, format!("mdp-opt-{mi}")))
                        .role(Role::ListBoxOption)
                        .aria_label(format!("{}, {}", m.name, m.description))
                        .aria_selected(is_selected)
                        .tab_index(0)
                        .relative()
                        .min_h(px(44.))
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(7.))
                        .when(dim_row, |d| d.opacity(0.62))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .when(is_lit, |d| d.child(pro_mark_selected(&row_sensor)))
                        .on_hover(move |inside: &bool, _, app| {
                            ent_hov.update(app, |this, cx| {
                                if *inside {
                                    this.pointing = true;
                                    this.inspected_idx = mi;
                                } else if this.inspected_idx == mi {
                                    this.pointing = false;
                                }
                                cx.notify();
                            });
                        })
                        .on_click(move |_, _, app| {
                            ent_clk.update(app, |this, cx| {
                                this.select_index(mi, false, cx);
                            });
                        })
                        .child(pro_app_tile(&m.provider, 20., cx))
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
                                        .gap(px(5.))
                                        .min_w_0()
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .child(m.name),
                                        )
                                        .child(
                                            div()
                                                .w(px(if is_selected { 14. } else { 0. }))
                                                .h(px(14.))
                                                .flex_none()
                                                .overflow_hidden()
                                                .child(pro_check_draw(
                                                    pro_eid(&base_id, format!("mdp-chk-{mi}")),
                                                    is_selected,
                                                    14.,
                                                    false,
                                                    p.blue,
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.5))
                                        .text_color(p.text)
                                        .child(m.description),
                                ),
                        )
                        .child(meters_el),
                );
            }

            // Settings block: Extended thinking (`pro_switch`) -> unfolding Effort (`pro_fold`) + Web search (`pro_switch`)
            let effort_fold_body: AnyElement = {
                let base_id = base_id.clone();
                let entity = entity.clone();
                let eff_sensor = pro_selection_sensor();
                let levels = [
                    ThinkingEffort::Low,
                    ThinkingEffort::Medium,
                    ThinkingEffort::High,
                ];
                let mut seg_strip = div()
                    .relative()
                    .p(px(2.))
                    .rounded(px(6.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&base_id, "eff-pill"),
                        effort.key(),
                        eff_sensor.clone(),
                        4.5,
                        p.card,
                        Some(p.line),
                        true,
                    ));

                for lvl in levels {
                    let active = lvl == effort;
                    let ent = entity.clone();
                    seg_strip = seg_strip.child(
                        div()
                            .id(pro_eid(&base_id, format!("eff-{}", lvl.key())))
                            .role(Role::RadioButton)
                            .aria_label(lvl.label(cx))
                            .aria_selected(active)
                            .tab_index(0)
                            .relative()
                            .h(px(22.))
                            .px(px(8.))
                            .rounded(px(4.5))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if active { p.strong } else { p.text })
                            .on_click(move |_, _, app| {
                                ent.update(app, |this, cx| {
                                    this.set_effort(lvl, cx);
                                });
                            })
                            .when(active, |d| d.child(pro_mark_selected(&eff_sensor)))
                            .child(lvl.label(cx)),
                    );
                }

                div()
                    .pl(px(if narrow { 8. } else { 36. }))
                    .pr(px(8.))
                    .pt(px(2.))
                    .pb(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_size(px(12.)).text_color(p.text).child(tr(
                        cx,
                        "Effort",
                        "思考强度",
                    )))
                    .child(seg_strip)
                    .into_any_element()
            };
            let effort_fold = pro_fold(
                pro_eid(&base_id, "effort-fold"),
                thinking,
                34.,
                0.,
                move || effort_fold_body,
            );

            let ent_think_row = entity.clone();
            let ent_web_row = entity.clone();

            let settings_section = div()
                .mt(px(4.))
                .pt(px(4.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .id(pro_eid(&base_id, "row-thinking"))
                        .h(px(34.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .on_click(move |_, _, app| {
                            ent_think_row.update(app, |this, cx| this.toggle_thinking(cx));
                        })
                        .child(div().w(px(18.)).flex().justify_center().child(pro_svg(
                            "0 0 14 14",
                            SVG_BULB,
                            15.,
                            if thinking { p.blue } else { p.muted },
                        )))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .child(tr(cx, "Extended thinking", "深度思考")),
                        )
                        .child(pro_switch(
                            pro_eid(&base_id, "sw-thinking"),
                            tr(cx, "Extended thinking", "深度思考"),
                            thinking,
                            cx,
                            |this, _, cx| this.toggle_thinking(cx),
                        )),
                )
                .child(effort_fold)
                .child(
                    div()
                        .id(pro_eid(&base_id, "row-web-search"))
                        .h(px(34.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .on_click(move |_, _, app| {
                            ent_web_row.update(app, |this, cx| this.toggle_web_search(cx));
                        })
                        .child(div().w(px(18.)).flex().justify_center().child(pro_icon(
                            ProIcon::Globe,
                            15.,
                            if web_search { p.blue } else { p.muted },
                        )))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .child(tr(cx, "Web search", "联网搜索")),
                        )
                        .child(pro_switch(
                            pro_eid(&base_id, "sw-web"),
                            tr(cx, "Web search", "联网搜索"),
                            web_search,
                            cx,
                            |this, _, cx| this.toggle_web_search(cx),
                        )),
                );

            // Footer tray rolling first-token latency + price per 1M tokens (`pro_rolling_number`)
            let footer_caption = if let Some(ref model) = inspected_model {
                if let Some(met) = model.metrics {
                    let (secs, price) =
                        compute_effective_tray_stats(met, thinking, effort, web_search);
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .min_w_0()
                        .child(div().text_size(px(11.5)).text_color(p.muted).child("~"))
                        .child(pro_rolling_number(
                            pro_eid(&base_id, "roll-ttft"),
                            secs,
                            1,
                            "",
                            "s",
                            12.,
                            FontWeight::SEMIBOLD,
                            p.ink,
                        ))
                        .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                            cx,
                            "first token ·",
                            "首字延迟 ·",
                        )))
                        .child(pro_rolling_number(
                            pro_eid(&base_id, "roll-price"),
                            price,
                            if price < 10.0 && price.fract() > 0.05 {
                                1
                            } else {
                                0
                            },
                            "$",
                            "/1M",
                            12.,
                            FontWeight::SEMIBOLD,
                            p.ink,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "Routes automatically by prompt complexity",
                            "按提示词复杂度自动路由最优模型",
                        ))
                        .into_any_element()
                }
            } else {
                div().into_any_element()
            };

            let tray = div()
                .min_h(px(36.))
                .px(px(12.))
                .py(px(6.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(footer_caption)
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(
                                div()
                                    .h(px(19.))
                                    .px(px(5.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child("↑↓"),
                            )
                            .child(
                                div()
                                    .h(px(19.))
                                    .px(px(5.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child("↵"),
                            ),
                    )
                });

            div()
                .w_full()
                .max_w(px(368.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x1c1c1f) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .child(
                    div()
                        .p(px(5.))
                        .flex()
                        .flex_col()
                        .child(head)
                        .child(rows_col)
                        .child(settings_section),
                )
                .child(tray)
                .into_any_element()
        };
        let menu_el = pro_popover_presence(pro_eid(&self.id, "mdp-menu"), self.open, move || {
            div().w_full().child(menu_body).into_any_element()
        });

        let mut stack = div().w_full().max_w(px(368.)).flex().flex_col().gap(px(6.));
        if self.side == ModelMenuSide::Top {
            stack = stack.child(menu_el).child(trigger_bar);
        } else {
            stack = stack.child(trigger_bar).child(menu_el);
        }

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(tr(cx, "Model picker", "模型选择器"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .w_full()
            .min_w_0()
            .p(px(if narrow { 12. } else { 20. }))
            .flex()
            .flex_col()
            .items_center()
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
            .child(stack)
    }
}

#[cfg(test)]
mod tests {
    use super::{PickerModelMetrics, ThinkingEffort, compute_effective_tray_stats, default_models};

    #[test]
    fn effort_multipliers_scale_first_token_latency_and_price() {
        let sonnet = PickerModelMetrics {
            speed: 4,
            quality: 4,
            cost: 3,
            latency_ms: 1000,
            price_per_m: 15.0,
        };
        let (base_t, base_p) =
            compute_effective_tray_stats(sonnet, false, ThinkingEffort::Medium, false);
        assert!((base_t - 1.0).abs() < 1e-6);
        assert!((base_p - 15.0).abs() < 1e-6);

        let (low_t, _) = compute_effective_tray_stats(sonnet, true, ThinkingEffort::Low, false);
        let (med_t, _) = compute_effective_tray_stats(sonnet, true, ThinkingEffort::Medium, false);
        let (high_t, high_p) =
            compute_effective_tray_stats(sonnet, true, ThinkingEffort::High, true);
        assert!((low_t - 1.6).abs() < 1e-6);
        assert!((med_t - 2.5).abs() < 1e-6);
        assert!((high_t - 4.35).abs() < 1e-6);
        assert!(high_p > base_p);
    }

    #[test]
    fn default_models_include_major_providers_and_auto_router() {
        let models = default_models();
        assert_eq!(models.len(), 6);
        assert!(models.iter().any(|m| m.provider.as_ref() == "claude"));
        assert!(models.iter().any(|m| m.provider.as_ref() == "openai"));
        assert!(models.iter().any(|m| m.provider.as_ref() == "gemini"));
        assert!(models.iter().any(|m| m.provider.as_ref() == "deepseek"));
        assert!(models.last().is_some_and(|m| m.metrics.is_none()));
    }
}
