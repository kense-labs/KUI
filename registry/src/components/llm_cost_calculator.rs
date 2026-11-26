//! A local estimate based only on prices and usage supplied by the host.
use crate::{
    components::slider::{SlideValue, Slider},
    primitives::{
        brand_mark::brand_mark,
        family::{Paint, Tone, face},
        presentation::{column, hint, row, tr},
    },
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug)]
pub struct LlmModelPrice {
    pub id: SharedString,
    pub name: SharedString,
    pub provider: SharedString,
    pub input: f64,
    pub cached_input: Option<f64>,
    pub output: f64,
}
impl LlmModelPrice {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        provider: impl Into<SharedString>,
        input: f64,
        output: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: provider.into(),
            input,
            cached_input: None,
            output,
        }
    }
    pub fn with_cached_input(mut self, price: f64) -> Self {
        self.cached_input = Some(price);
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LlmUsage {
    pub input_tokens: f64,
    pub output_tokens: f64,
    pub requests_per_day: f64,
    pub cache_enabled: bool,
    pub cache_hit_rate: f64,
    pub days_per_month: f64,
}
impl LlmUsage {
    pub fn new(input_tokens: f64, output_tokens: f64, requests_per_day: f64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            requests_per_day,
            cache_enabled: false,
            cache_hit_rate: 0.,
            days_per_month: 30.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LlmCost {
    pub input: f64,
    pub cached_input: f64,
    pub output: f64,
    pub total: f64,
    pub per_request: f64,
}
/// Prices are USD per million tokens; all returned costs are USD.
/// Invalid, negative, non-finite or overflowing inputs return None.
pub fn compute_llm_cost(model: &LlmModelPrice, usage: LlmUsage) -> Option<LlmCost> {
    let quantities = [
        model.input,
        model.output,
        usage.input_tokens,
        usage.output_tokens,
        usage.requests_per_day,
        usage.days_per_month,
    ];
    if quantities.iter().any(|v| !v.is_finite() || *v < 0.)
        || !usage.cache_hit_rate.is_finite()
        || !(0. ..=1.).contains(&usage.cache_hit_rate)
    {
        return None;
    }
    let cached_price = model.cached_input.unwrap_or(model.input);
    if !cached_price.is_finite() || cached_price < 0. {
        return None;
    }
    let share = if usage.cache_enabled {
        usage.cache_hit_rate
    } else {
        0.
    };
    let multiplier = usage.requests_per_day * usage.days_per_month / 1_000_000.;
    let input = usage.input_tokens * (1. - share) * model.input * multiplier;
    let cached_input = usage.input_tokens * share * cached_price * multiplier;
    let output = usage.output_tokens * model.output * multiplier;
    let per_request = (usage.input_tokens * ((1. - share) * model.input + share * cached_price)
        + usage.output_tokens * model.output)
        / 1_000_000.;
    let total = input + cached_input + output;
    [input, cached_input, output, total, per_request]
        .iter()
        .all(|v| v.is_finite())
        .then_some(LlmCost {
            input,
            cached_input,
            output,
            total,
            per_request,
        })
}
#[derive(Clone, Debug)]
pub enum LlmCostEvent {
    UsageChanged(LlmUsage),
    Selected(SharedString),
}
/// `LlmCostCalculator` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::llm_cost_calculator::{LlmCostCalculator, LlmModelPrice, LlmUsage};
///
/// let prices = vec![LlmModelPrice::new("sample", "Sample model", "Example", 1.0, 3.0)];
/// let usage = LlmUsage::new(8000.0, 500.0, 5000.0);
/// let control = cx.new(|cx| LlmCostCalculator::new(prices, usage, cx).with_simulated(true));
/// # }
/// ```
pub struct LlmCostCalculator {
    models: Vec<LlmModelPrice>,
    usage: LlmUsage,
    initial: LlmUsage,
    selected: Option<SharedString>,
    sliders: Vec<Entity<Slider>>,
    focus: Vec<FocusHandle>,
    simulated: bool,
    hovered: Option<usize>,
}
impl EventEmitter<LlmCostEvent> for LlmCostCalculator {}
impl LlmCostCalculator {
    pub fn new(models: Vec<LlmModelPrice>, usage: LlmUsage, cx: &mut Context<Self>) -> Self {
        let sliders = Self::make_sliders(usage, cx);
        let selected = models.first().map(|m| m.id.clone());
        Self {
            models,
            usage,
            initial: usage,
            selected,
            sliders,
            focus: vec![],
            simulated: false,
            hovered: None,
        }
    }
    pub fn with_simulated(mut self, simulated: bool) -> Self {
        self.simulated = simulated;
        self
    }
    fn make_sliders(usage: LlmUsage, cx: &mut Context<Self>) -> Vec<Entity<Slider>> {
        let settings = [
            (
                tr(cx, "Input tokens", "输入 token"),
                200_000.,
                100.,
                usage.input_tokens,
            ),
            (
                tr(cx, "Output tokens", "输出 token"),
                16_000.,
                100.,
                usage.output_tokens,
            ),
            (
                tr(cx, "Requests per day", "每日请求"),
                1_000_000.,
                100.,
                usage.requests_per_day,
            ),
            (
                tr(cx, "Cache hit rate", "缓存命中率"),
                100.,
                1.,
                usage.cache_hit_rate * 100.,
            ),
        ];
        let mut sliders = Vec::new();
        for (index, (label, max, step, value)) in settings.into_iter().enumerate() {
            let value = if value.is_finite() {
                value.clamp(0., max)
            } else {
                0.
            };
            let slider = cx.new(|cx| {
                Slider::new(
                    label,
                    0. ..=max,
                    step,
                    SlideValue {
                        low: value,
                        high: None,
                    },
                    cx,
                )
            });
            cx.subscribe(&slider, move |s, _, value: &SlideValue, cx| {
                match index {
                    0 => s.usage.input_tokens = value.low,
                    1 => s.usage.output_tokens = value.low,
                    2 => s.usage.requests_per_day = value.low,
                    _ => s.usage.cache_hit_rate = value.low / 100.,
                };
                cx.emit(LlmCostEvent::UsageChanged(s.usage));
                cx.notify();
            })
            .detach();
            sliders.push(slider);
        }
        sliders
    }
    pub fn set_usage(&mut self, usage: LlmUsage, cx: &mut Context<Self>) {
        self.usage = usage;
        self.sliders = Self::make_sliders(usage, cx);
        cx.notify();
    }
    pub fn set_models(&mut self, models: Vec<LlmModelPrice>, cx: &mut Context<Self>) {
        self.models = models;
        if self
            .selected
            .as_ref()
            .is_none_or(|id| !self.models.iter().any(|m| &m.id == id))
        {
            self.selected = self.models.first().map(|m| m.id.clone());
        }
        cx.notify();
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        let initial = self.initial;
        self.set_usage(initial, cx);
        cx.emit(LlmCostEvent::UsageChanged(initial));
    }
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(model) = self.models.get(index)
            && self.selected.as_ref() != Some(&model.id)
        {
            self.selected = Some(model.id.clone());
            cx.emit(LlmCostEvent::Selected(model.id.clone()));
            cx.notify();
        }
    }
}
fn tabular() -> FontFeatures {
    FontFeatures(vec![("tnum".into(), 1)].into())
}

fn group_whole(whole: &str) -> String {
    let mut out = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn money(v: Option<f64>) -> String {
    v.map_or_else(
        || "—".into(),
        |v| {
            if v >= 100_000. {
                if v >= 1_000_000. {
                    format!("${:.2}M", v / 1_000_000.)
                } else {
                    format!("${:.1}K", v / 1_000.)
                }
            } else {
                let raw = format!("{:.2}", v.max(0.));
                let (whole, frac) = raw.split_once('.').unwrap_or((&raw, "00"));
                format!("${}.{frac}", group_whole(whole))
            }
        },
    )
}

fn model_badge(model: &LlmModelPrice, side: f32, p: Paint, cx: &App) -> Div {
    let mark = brand_mark(&model.name, side - 8., cx)
        .or_else(|| brand_mark(&model.provider, side - 8., cx));
    let fallback_char = model
        .provider
        .chars()
        .next()
        .or_else(|| model.name.chars().next())
        .unwrap_or('M')
        .to_uppercase()
        .to_string();
    div()
        .size(px(side))
        .flex_none()
        .rounded(px(6.))
        .bg(p.tray)
        .border_1()
        .border_color(p.line)
        .flex()
        .items_center()
        .justify_center()
        .child(mark.unwrap_or_else(|| {
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.secondary)
                .child(fallback_char)
                .into_any_element()
        }))
}

impl Render for LlmCostCalculator {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while self.focus.len() < self.models.len() {
            self.focus.push(cx.focus_handle().tab_stop(true));
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let mut ranking: Vec<usize> = (0..self.models.len()).collect();
        ranking.sort_by(|a, b| {
            match (
                compute_llm_cost(&self.models[*a], self.usage),
                compute_llm_cost(&self.models[*b], self.usage),
            ) {
                (Some(a), Some(b)) => a.total.total_cmp(&b.total),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(b),
            }
        });
        let selected = self
            .selected
            .as_ref()
            .and_then(|id| self.models.iter().position(|m| &m.id == id));
        let selected_model = selected.and_then(|i| self.models.get(i));
        let selected_cost = selected.and_then(|i| compute_llm_cost(&self.models[i], self.usage));
        let maximum = ranking
            .iter()
            .filter_map(|i| compute_llm_cost(&self.models[*i], self.usage).map(|c| c.total))
            .fold(0f64, f64::max);
        let mut list = column()
            .id("llm-cost-models")
            .role(Role::List)
            .w_full()
            .gap(px(4.))
            .pt(px(14.))
            .border_t_1()
            .border_color(p.line);
        for (rank, &index) in ranking.iter().enumerate() {
            let model = &self.models[index];
            let cost = compute_llm_cost(model, self.usage);
            let amount = money(cost.map(|c| c.total));
            let is_selected = self.selected.as_ref() == Some(&model.id);
            let share = if maximum > 0. {
                cost.map_or(0., |c| (c.total / maximum).clamp(0.03, 1.) as f32)
            } else {
                0.
            };
            let label = format!(
                "{} · {} · {} {} · {}",
                model.name,
                model.provider,
                tr(cx, "Rank", "排名"),
                rank + 1,
                amount
            );
            let previous = ranking[rank.saturating_sub(1)];
            let next = ranking[(rank + 1).min(ranking.len() - 1)];
            let first = ranking[0];
            let last = ranking[ranking.len() - 1];
            let hot = self.hovered == Some(index);
            let item = face(
                ("llm-cost-model", index),
                label,
                46.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .w_full()
            .h(px(46.))
            .px(px(10.))
            .border_1()
            .border_color(if is_selected {
                p.line.blend(p.bright.opacity(0.42))
            } else {
                gpui::transparent_black()
            })
            .bg(if is_selected {
                p.tray
            } else if hot {
                p.ink.opacity(0.035)
            } else {
                gpui::transparent_black()
            })
            .when(!is_selected, |b| b.hover(|s| s.bg(p.ink.opacity(0.04))))
            .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                s.hovered = if *over { Some(index) } else { None };
                cx.notify();
            }))
            .track_focus(&self.focus[index])
            .aria_toggled(is_selected.into())
            .child(
                row()
                    .w_full()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(14.))
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .font_features(tabular())
                            .text_color(p.faint)
                            .text_center()
                            .child((rank + 1).to_string()),
                    )
                    .child(model_badge(model, 24., p, cx))
                    .child(
                        div()
                            .w(px(138.))
                            .min_w(px(0.))
                            .truncate()
                            .text_size(px(13.5))
                            .font_weight(if is_selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .child(model.name.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(36.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(p.ink.opacity(0.07))
                            .child(div().h_full().w(relative(share)).rounded_full().bg(
                                if is_selected {
                                    p.bright
                                } else if hot {
                                    p.ink.opacity(0.36)
                                } else {
                                    p.ink.opacity(0.22)
                                },
                            )),
                    )
                    .child(
                        div()
                            .w(px(78.))
                            .text_right()
                            .font_features(tabular())
                            .font_weight(if is_selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .text_color(if is_selected { p.ink } else { p.secondary })
                            .child(amount),
                    ),
            )
            .on_click(cx.listener(move |s, _, _, cx| s.select(index, cx)))
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                let target = match e.keystroke.key.as_str() {
                    "enter" | "space" => Some(index),
                    "up" | "left" => Some(previous),
                    "down" | "right" => Some(next),
                    "home" => Some(first),
                    "end" => Some(last),
                    _ => None,
                };
                if let Some(target) = target {
                    s.select(target, cx);
                    if let Some(focus) = s.focus.get(target) {
                        focus.focus(w, cx);
                    }
                    cx.stop_propagation();
                }
            }));
            list = list.child(item);
        }
        let model_name = selected_model.map_or_else(
            || tr(cx, "No model selected", "未选择模型").into(),
            |m| m.name.clone(),
        );
        let header_mark = selected_model.and_then(|m| {
            brand_mark(&m.name, 14., cx).or_else(|| brand_mark(&m.provider, 14., cx))
        });
        let cache_label = if self.usage.cache_hit_rate.is_finite() {
            format!("{:.0}%", self.usage.cache_hit_rate * 100.)
        } else {
            "—".into()
        };
        let mut panel = column()
            .id("llm-cost-panel")
            .role(Role::Group)
            .aria_label(tr(cx, "Estimated monthly cost", "每月费用估算"))
            .w_full()
            .max_w(px(470.))
            .gap(px(18.))
            .p(px(24.))
            .rounded(radius)
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .gap(px(12.))
                    .items_start()
                    .justify_between()
                    .pb(px(14.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(
                        column()
                            .min_w_0()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(tr(cx, "Estimated monthly cost", "每月费用估算")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(18.))
                                    .text_color(p.faint)
                                    .child(tr(
                                        cx,
                                        "Prices per one million tokens",
                                        "每百万 token 的价格",
                                    )),
                            ),
                    )
                    .child(
                        column()
                            .items_end()
                            .gap(px(3.))
                            .child(
                                div()
                                    .text_size(px(26.))
                                    .line_height(px(28.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .font_features(tabular())
                                    .child(money(selected_cost.map(|c| c.total))),
                            )
                            .child(
                                row()
                                    .items_center()
                                    .gap(px(5.))
                                    .when_some(header_mark, |r, mark| r.child(mark))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(p.secondary)
                                            .child(model_name),
                                    ),
                            ),
                    ),
            )
            .child(
                column()
                    .w_full()
                    .gap(px(14.))
                    .children(self.sliders.iter().cloned()),
            );
        panel = panel.child(
            row()
                .justify_between()
                .items_center()
                .gap_2()
                .flex_wrap()
                .child(
                    face(
                        "llm-cache",
                        tr(cx, "Cached input", "输入缓存"),
                        34.,
                        radius.into(),
                        if self.usage.cache_enabled {
                            Tone::Primary
                        } else {
                            Tone::Surface
                        },
                        cx,
                    )
                    .px(px(12.))
                    .when(!self.usage.cache_enabled, |b| {
                        b.border_1().border_color(p.line)
                    })
                    .aria_toggled(self.usage.cache_enabled.into())
                    .child(format!(
                        "{} · {}",
                        tr(cx, "Cached input", "输入缓存"),
                        cache_label
                    ))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.usage.cache_enabled = !s.usage.cache_enabled;
                        cx.emit(LlmCostEvent::UsageChanged(s.usage));
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                            s.usage.cache_enabled = !s.usage.cache_enabled;
                            cx.emit(LlmCostEvent::UsageChanged(s.usage));
                            cx.notify();
                            cx.stop_propagation();
                        }
                    })),
                )
                .child(
                    face(
                        "llm-reset",
                        tr(cx, "Reset", "重置"),
                        34.,
                        radius.into(),
                        Tone::Surface,
                        cx,
                    )
                    .px(px(12.))
                    .border_1()
                    .border_color(p.line)
                    .hover(|s| s.bg(p.tray))
                    .child(tr(cx, "Reset", "重置"))
                    .on_click(cx.listener(|s, _, _, cx| s.reset(cx)))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                            s.reset(cx);
                            cx.stop_propagation();
                        }
                    })),
                ),
        );
        if self.models.is_empty() {
            panel = panel.child(hint(tr(cx, "No prices provided", "尚未提供价格"), p));
        } else {
            panel = panel.child(list);
        }
        if let Some(cost) = selected_cost {
            let breakdown_row = |label: &'static str, val: String| {
                row()
                    .justify_between()
                    .items_center()
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .child(div().text_color(p.secondary).child(label))
                    .child(
                        div()
                            .font_features(tabular())
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(val),
                    )
            };
            panel = panel.child(
                column()
                    .w_full()
                    .p(px(12.))
                    .gap(px(6.))
                    .rounded(radius)
                    .bg(p.tray)
                    .border_1()
                    .border_color(p.line)
                    .child(breakdown_row(
                        tr(cx, "Input tokens", "输入费用"),
                        money(Some(cost.input)),
                    ))
                    .child(breakdown_row(
                        tr(cx, "Cached input", "缓存输入"),
                        money(Some(cost.cached_input)),
                    ))
                    .child(breakdown_row(
                        tr(cx, "Output tokens", "输出费用"),
                        money(Some(cost.output)),
                    ))
                    .child(
                        div()
                            .pt(px(6.))
                            .mt(px(2.))
                            .border_t_1()
                            .border_color(p.line)
                            .child(breakdown_row(
                                tr(cx, "Cost per request", "每次请求成本"),
                                format!("${:.5}", cost.per_request),
                            )),
                    ),
            );
        } else if !self.models.is_empty() {
            panel = panel.child(div().text_color(p.danger).child(tr(
                cx,
                "Invalid price or usage; enter finite non-negative values and a cache rate between 0 and 1.",
                "价格或用量无效；请输入有限的非负数和 0 到 1 之间的缓存比例。",
            )));
        }
        panel
    }
}
#[cfg(test)]
mod tests {
    use super::{LlmModelPrice, LlmUsage, compute_llm_cost};
    #[test]
    fn cached_inputs_and_zero_traffic_are_computed_correctly() {
        let model = LlmModelPrice::new("a", "A", "", 2., 8.).with_cached_input(0.5);
        let mut usage = LlmUsage::new(1_000_000., 500_000., 10.);
        usage.cache_enabled = true;
        usage.cache_hit_rate = 0.5;
        let result = compute_llm_cost(&model, usage).unwrap();
        assert!((result.total - 1575.).abs() < 1e-6);
        usage.requests_per_day = 0.;
        let result = compute_llm_cost(&model, usage).unwrap();
        assert_eq!(result.total, 0.);
        assert!((result.per_request - 5.25).abs() < 1e-6);
    }
    #[test]
    fn bad_inputs_do_not_produce_false_estimates() {
        let model = LlmModelPrice::new("a", "A", "", f64::NAN, 1.);
        assert!(compute_llm_cost(&model, LlmUsage::new(1., 1., 1.)).is_none());
    }
}
