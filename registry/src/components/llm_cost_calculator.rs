//! A local estimate based only on prices and usage supplied by the host.
use crate::{
    components::slider::{SlideValue, Slider},
    primitives::{
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
                format!("${v:.2}")
            }
        },
    )
}
impl Render for LlmCostCalculator {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while self.focus.len() < self.models.len() {
            self.focus.push(cx.focus_handle().tab_stop(true));
        }
        let p = Paint::get(cx);
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
        let selected_cost = selected.and_then(|i| compute_llm_cost(&self.models[i], self.usage));
        let maximum = ranking
            .iter()
            .filter_map(|i| compute_llm_cost(&self.models[*i], self.usage).map(|c| c.total))
            .fold(0f64, f64::max);
        let mut list = column()
            .id("llm-cost-models")
            .role(Role::List)
            .w_full()
            .gap(px(0.))
            .pt(px(16.))
            .border_t_1()
            .border_color(p.line);
        for (rank, &index) in ranking.iter().enumerate() {
            let model = &self.models[index];
            let cost = compute_llm_cost(model, self.usage);
            let amount = money(cost.map(|c| c.total));
            let selected = self.selected.as_ref() == Some(&model.id);
            let share = if maximum > 0. {
                cost.map_or(0., |c| (c.total / maximum).clamp(0., 1.) as f32)
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
            let item = face(("llm-cost-model", index), label, 44., 2., Tone::Clear, cx)
                .w_full()
                .h(px(44.))
                .px(px(8.))
                .bg(if selected {
                    p.bright.opacity(0.07)
                } else if hot {
                    p.ink.opacity(0.04)
                } else {
                    gpui::transparent_black()
                })
                .when(!selected, |b| b.hover(|s| s.bg(p.ink.opacity(0.04))))
                .when(selected, |b| b.hover(|s| s.bg(p.bright.opacity(0.10))))
                .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                    s.hovered = if *over { Some(index) } else { None };
                    cx.notify();
                }))
                .track_focus(&self.focus[index])
                .aria_toggled(selected.into())
                .child(
                    row()
                        .w_full()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .w(px(16.))
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.faint)
                                .text_center()
                                .child((rank + 1).to_string()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .max_w(px(168.))
                                .min_w(px(0.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(model.name.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h(px(6.))
                                .rounded_full()
                                .bg(p.ink.opacity(0.06))
                                .child(div().h_full().w(relative(share)).rounded_full().bg(
                                    if selected {
                                        p.bright
                                    } else if hot {
                                        p.ink.opacity(0.34)
                                    } else {
                                        p.ink.opacity(0.24)
                                    },
                                )),
                        )
                        .child(
                            div()
                                .w(px(88.))
                                .text_right()
                                .text_color(if selected { p.ink } else { p.secondary })
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
        let model_name = selected.and_then(|i| self.models.get(i)).map_or_else(
            || tr(cx, "No model selected", "未选择模型").into(),
            |m| m.name.clone(),
        );
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
            .max_w(px(720.))
            .gap(px(24.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .gap(px(16.))
                    .flex_wrap()
                    .justify_between()
                    .child(
                        column()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(tr(cx, "Estimated monthly cost", "每月费用估算")),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
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
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .line_height(px(26.))
                                    .child(money(selected_cost.map(|c| c.total))),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.faint)
                                    .child(model_name),
                            ),
                    ),
            )
            .children(self.sliders.iter().cloned());
        panel = panel.child(
            row()
                .gap_2()
                .flex_wrap()
                .child(
                    face(
                        "llm-cache",
                        tr(cx, "Cached input", "输入缓存"),
                        36.,
                        crate::foundation::surface_radius(cx).into(),
                        if self.usage.cache_enabled {
                            Tone::Primary
                        } else {
                            Tone::Surface
                        },
                        cx,
                    )
                    .px(px(10.))
                    .aria_toggled(self.usage.cache_enabled.into())
                    .child(format!(
                        "{} {}",
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
                        36.,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Surface,
                        cx,
                    )
                    .px(px(10.))
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
            panel = panel.child(
                row()
                    .gap(px(16.))
                    .flex_wrap()
                    .child(hint(
                        format!("{} {}", tr(cx, "Input", "输入"), money(Some(cost.input))),
                        p,
                    ))
                    .child(hint(
                        format!(
                            "{} {}",
                            tr(cx, "Cached", "缓存"),
                            money(Some(cost.cached_input))
                        ),
                        p,
                    ))
                    .child(hint(
                        format!("{} {}", tr(cx, "Output", "输出"), money(Some(cost.output))),
                        p,
                    ))
                    .child(hint(
                        format!(
                            "{} ${:.5}",
                            tr(cx, "Per request", "每次请求"),
                            cost.per_request
                        ),
                        p,
                    )),
            );
        } else if !self.models.is_empty() {
            panel=panel.child(div().text_color(p.danger).child(tr(cx,"Invalid price or usage; enter finite non-negative values and a cache rate between 0 and 1.","价格或用量无效；请输入有限的非负数和 0 到 1 之间的缓存比例。")));
        }
        panel.child(hint(
            if self.simulated {
                tr(
                    cx,
                    "Sample prices and usage · illustrative estimate only",
                    "样例价格和用量 · 仅供演示",
                )
            } else {
                tr(
                    cx,
                    "Estimate from host-provided prices and usage",
                    "根据宿主提供的价格和用量估算",
                )
            },
            p,
        ))
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
