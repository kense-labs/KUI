//! Inspect token usage and timing for a generated response.
use crate::primitives::{
    brand_mark::brand_mark,
    clipboard,
    family::{Paint, Slide, disabled_motion},
    glyph::{Glyph, icon, spinner},
    ops_surface::*,
};
use gpui::{prelude::*, *};
#[derive(Clone, Default)]
pub struct Metrics {
    pub first_token_ms: u32,
    pub duration_ms: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub input_rate: f64,
    pub output_rate: f64,
    pub running: bool,
    pub samples: Vec<f32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerationState {
    Waiting,
    Streaming,
    Done,
    Stopped,
    Failed(SharedString),
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum ReadoutVariant {
    #[default]
    Card,
    Inline,
}

#[derive(Clone, Copy, Default)]
pub struct UsageBreakdown {
    pub cached_tokens: u32,
    pub reasoning_tokens: u32,
    pub cached_input_rate: f64,
}

pub struct RetryGeneration;

pub struct GenerationReadout {
    id: SharedString,
    pub model: String,
    data: Metrics,
    details: bool,
    count: Slide,
    state: Option<GenerationState>,
    usage: UsageBreakdown,
    variant: ReadoutVariant,
    measured_width: Option<Pixels>,
    copied: Option<bool>,
    visibility: fn() -> bool,
    clock: web_time::Instant,
}
impl EventEmitter<RetryGeneration> for GenerationReadout {}
impl GenerationReadout {
    pub fn new(id: impl Into<SharedString>, model: impl Into<String>, data: Metrics) -> Self {
        let count = Slide::new(data.output_tokens as f32);
        Self {
            id: id.into(),
            model: model.into(),
            data,
            details: false,
            count,
            state: None,
            usage: UsageBreakdown::default(),
            variant: ReadoutVariant::Card,
            measured_width: None,
            copied: None,
            visibility: || true,
            clock: web_time::Instant::now(),
        }
    }
    pub fn variant(mut self, variant: ReadoutVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn visibility(mut self, visible: fn() -> bool) -> Self {
        self.visibility = visible;
        self
    }

    pub fn set_state(&mut self, state: GenerationState, cx: &mut Context<Self>) {
        self.state = Some(state);
        self.copied = None;
        cx.notify();
    }

    pub fn set_usage(&mut self, usage: UsageBreakdown, cx: &mut Context<Self>) {
        self.usage = usage;
        cx.notify();
    }

    fn copy(&mut self, cost: f64, speed: Option<f32>, cx: &mut Context<Self>) {
        let data = serde_json::json!({
            "model": self.model,
            "first_token_ms": self.data.first_token_ms,
            "duration_ms": self.data.duration_ms,
            "input_tokens": self.data.input_tokens,
            "cached_input_tokens": self.usage.cached_tokens.min(self.data.input_tokens),
            "output_tokens": self.data.output_tokens,
            "reasoning_tokens": self.usage.reasoning_tokens,
            "tokens_per_second": speed,
            "cost_usd": cost,
        });
        let pending = clipboard::write(&data.to_string(), cx);
        cx.spawn(async move |weak, cx| {
            let ok = pending.await;
            let _ = weak.update(cx, |s, cx| {
                s.copied = Some(ok);
                cx.notify();
            });
        })
        .detach();
    }

    /// Reports whether the latency and cost breakdown is open.
    pub fn details_open(&self) -> bool {
        self.details
    }

    pub fn set_metrics(&mut self, data: Metrics, cx: &mut Context<Self>) {
        if data.output_tokens < self.data.output_tokens {
            self.count.reset(data.output_tokens as f32)
        } else {
            self.count.duration(data.output_tokens as f32, 0.1, cx)
        }
        self.data = data;
        cx.notify()
    }
}
impl Render for GenerationReadout {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let d = &self.data;
        let p = Paint::get(cx);
        let state = self.state.clone().unwrap_or(if d.running {
            GenerationState::Streaming
        } else {
            GenerationState::Done
        });
        let width = self
            .measured_width
            .unwrap_or(w.viewport_size().width - px(40.));
        let narrow = width < px(440.);
        let grid_wide = width >= px(600.);
        let inline = self.variant == ReadoutVariant::Inline;
        let motion = !disabled_motion(cx) && (self.visibility)() && w.is_visible();
        let pending = matches!(state, GenerationState::Waiting | GenerationState::Streaming);
        let output = self.count.get(w, cx).round() as u32;
        let cached = self.usage.cached_tokens.min(d.input_tokens);
        let uncached = d.input_tokens - cached;
        let cost = (uncached as f64 * d.input_rate
            + cached as f64 * self.usage.cached_input_rate
            + output as f64 * d.output_rate)
            / 1e6;
        let (_, glyph, status_ink) = match &state {
            GenerationState::Waiting => ("Waiting", Glyph::Clock, p.secondary),
            GenerationState::Streaming => ("Streaming", Glyph::Play, p.secondary),
            GenerationState::Done => ("Done", Glyph::Check, p.success),
            GenerationState::Stopped => ("Stopped", Glyph::Pause, p.secondary),
            GenerationState::Failed(_) => ("Failed", Glyph::Alert, p.danger),
        };
        let sampling_ms = d.duration_ms.saturating_sub(d.first_token_ms);
        let average_speed =
            (sampling_ms > 0).then(|| d.output_tokens as f32 * 1000. / sampling_ms as f32);
        let speed = if state == GenerationState::Streaming {
            d.samples
                .iter()
                .rev()
                .copied()
                .find(|value| value.is_finite() && *value >= 0.)
                .or(average_speed)
        } else {
            average_speed
        };
        let time = |value| {
            if value == 0 {
                "—".into()
            } else {
                format!("{value}ms")
            }
        };
        let metrics = [
            (
                tr(cx, "First token", "首个词元"),
                time(if state == GenerationState::Waiting {
                    d.duration_ms
                } else {
                    d.first_token_ms
                }),
            ),
            (
                tr(cx, "Speed", "速度"),
                speed.map_or_else(|| "—".into(), |v| format!("{v:.0} tok/s")),
            ),
            (tr(cx, "Output tokens", "输出词元"), output.to_string()),
            (tr(cx, "Cost", "费用"), format!("${cost:.4}")),
        ];
        let samples = d
            .samples
            .iter()
            .copied()
            .skip(d.samples.len().saturating_sub(48))
            .filter(|value| value.is_finite() && *value >= 0.)
            .collect::<Vec<_>>();
        let spark = canvas(
            |_, _, _| (),
            move |bounds, _, w, _| {
                if samples.len() < 2 {
                    return;
                }
                let min = samples.iter().copied().fold(f32::INFINITY, f32::min);
                let max = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let mut path = PathBuilder::stroke(px(1.25));
                let mut last = bounds.origin;
                for (i, &v) in samples.iter().enumerate() {
                    let before = samples[i.saturating_sub(1)];
                    let after = samples[(i + 1).min(samples.len() - 1)];
                    let v = before * 0.25 + v * 0.5 + after * 0.25;
                    let point = bounds.origin
                        + point(
                            px(1.)
                                + (bounds.size.width - px(2.))
                                    * (i as f32 / (samples.len() - 1) as f32),
                            px(2.)
                                + (bounds.size.height - px(4.))
                                    * (1. - (v - min) / (max - min).max(1.)),
                        );
                    if i == 0 {
                        path.move_to(point)
                    } else {
                        let x = (last.x + point.x) / 2.;
                        path.cubic_bezier_to(
                            point,
                            gpui::point(x, last.y),
                            gpui::point(x, point.y),
                        );
                    }
                    last = point;
                }
                if let Ok(path) = path.build() {
                    w.paint_path(path, p.ink)
                }
            },
        )
        .w(px(56.))
        .h(px(20.))
        .flex_shrink(1.)
        .into_any_element();
        let mut spark = Some(spark);
        let mut grid = column().gap(px(13.));
        let columns = if grid_wide { 4 } else { 2 };
        for (r, cells) in metrics.chunks(columns).enumerate() {
            let mut band = row().items_stretch().gap_0();
            for (i, (label, value)) in cells.iter().enumerate() {
                let mut val = row().gap_1().child(
                    div()
                        .text_size(px(16.))
                        .line_height(px(24.))
                        .flex_none()
                        .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                        .when(r == 0 && i == 1, |text| text.min_w(px(70.)))
                        .child(value.clone()),
                );
                if r == 0
                    && i == 1
                    && let Some(s) = spark.take()
                {
                    val = val.child(s)
                }
                band = band.child(
                    column()
                        .flex_1()
                        .gap(px(2.))
                        .min_w_0()
                        .when(i > 0, |el| el.pl(px(16.)).border_l_1().border_color(p.line))
                        .child(note(*label, cx))
                        .child(val),
                )
            }
            grid = grid.child(band)
        }
        let details = Fold::get("readout-details", self.details, 144., 0.26, w, cx);
        let first_share = d.first_token_ms.min(d.duration_ms) as f32 / d.duration_ms.max(1) as f32;
        let breakdown = column()
            .gap_2()
            .child(divider(cx))
            .child(note(tr(cx, "Latency", "延迟"), cx))
            .child(
                div()
                    .flex()
                    .h(px(6.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(p.hover)
                    .child(div().h_full().w(relative(first_share)).bg(p.secondary))
                    .child(
                        div()
                            .h_full()
                            .w(relative(1. - first_share))
                            .bg(p.bright.opacity(0.68)),
                    ),
            )
            .child(
                row()
                    .justify_between()
                    .flex_wrap()
                    .child(note(
                        format!(
                            "{} {}",
                            tr(cx, "First token", "首个词元"),
                            time(d.first_token_ms)
                        ),
                        cx,
                    ))
                    .child(note(
                        format!("{} {}", tr(cx, "Generation", "生成"), time(sampling_ms)),
                        cx,
                    ))
                    .child(note(
                        format!("{} {}", tr(cx, "Total", "总计"), time(d.duration_ms)),
                        cx,
                    )),
            )
            .child(divider(cx))
            .child(
                row()
                    .justify_between()
                    .child(note(tr(cx, "Tokens", "词元"), cx).flex_1())
                    .when(!narrow, |v| {
                        v.child(note(tr(cx, "Rate / 1M", "每百万费率"), cx).w(px(90.)))
                    })
                    .child(note(tr(cx, "Cost", "费用"), cx).w(px(64.)).text_right()),
            )
            .children(
                [
                    (tr(cx, "Input", "输入"), uncached, d.input_rate, true),
                    (
                        tr(cx, "Cached input", "缓存输入"),
                        cached,
                        self.usage.cached_input_rate,
                        cached > 0,
                    ),
                    (tr(cx, "Output", "输出"), output, d.output_rate, true),
                    (
                        tr(cx, "Reasoning", "推理"),
                        self.usage.reasoning_tokens,
                        0.,
                        self.usage.reasoning_tokens > 0,
                    ),
                ]
                .into_iter()
                .filter(|v| v.3)
                .map(|(label, count, rate, _)| {
                    row()
                        .justify_between()
                        .child(note(format!("{label}  {count}"), cx).flex_1())
                        .when(!narrow, |v| {
                            v.child(note(format!("${rate:.2}"), cx).w(px(90.)))
                        })
                        .child(
                            note(format!("${:.4}", count as f64 * rate / 1e6), cx)
                                .w(px(64.))
                                .text_right(),
                        )
                }),
            )
            .child(divider(cx))
            .child(
                row()
                    .justify_between()
                    .child(title(format!("{} · ${cost:.4}", tr(cx, "Total", "总计"))))
                    .child(btn(
                        "copy",
                        match self.copied {
                            Some(true) => tr(cx, "Copied", "已复制"),
                            Some(false) => tr(cx, "Copy failed", "复制失败"),
                            None => tr(cx, "Copy stats", "复制统计"),
                        },
                        false,
                        cx,
                        move |s, _, cx| s.copy(cost, speed, cx),
                    )),
            );
        let seconds = d.duration_ms as f32 / 1000.;
        let status = match &state {
            GenerationState::Waiting => tr(cx, "Waiting", "等待首个词元").to_owned(),
            GenerationState::Streaming => tr(cx, "Streaming", "生成中").to_owned(),
            GenerationState::Done => format!("{} {seconds:.2}s", tr(cx, "Done in", "已完成")),
            GenerationState::Stopped => format!("{} {seconds:.2}s", tr(cx, "Stopped at", "已停止")),
            GenerationState::Failed(_) => tr(cx, "Failed", "失败").to_owned(),
        };
        let status_icon = if pending && motion {
            w.request_animation_frame();
            spinner(self.clock.elapsed().as_secs_f32() * 1.25, 12., status_ink).into_any_element()
        } else if pending {
            spinner(0., 12., status_ink).into_any_element()
        } else {
            icon(glyph, 12., status_ink).into_any_element()
        };
        let model_icon = brand_mark(&self.model, 16., cx)
            .unwrap_or_else(|| icon(Glyph::Monitor, 16., p.secondary).into_any_element());
        let details_control = if narrow {
            icon_btn(
                "details",
                tr(cx, "Details", "详细信息"),
                if self.details { Glyph::Up } else { Glyph::Down },
                cx,
                |s, _, cx| {
                    s.details = !s.details;
                    cx.notify();
                },
            )
        } else {
            btn(
                "details",
                tr(cx, "Details", "详细信息"),
                false,
                cx,
                |s, _, cx| {
                    s.details = !s.details;
                    cx.notify();
                },
            )
            .child(icon(
                if self.details { Glyph::Up } else { Glyph::Down },
                12.,
                p.secondary,
            ))
        }
        .aria_expanded(self.details);
        let weak = cx.weak_entity();
        let metrics_content = if inline {
            row()
                .flex_wrap()
                .gap_2()
                .children(
                    metrics
                        .into_iter()
                        .map(|(label, value)| note(format!("{label}: {value}"), cx)),
                )
                .into_any_element()
        } else {
            grid.into_any_element()
        };
        let error = if let GenerationState::Failed(error) = state {
            Some(error)
        } else {
            None
        };
        let has_error = error.is_some();
        let error_fold = Fold::get("generation-error", has_error, 40., 0.26, w, cx);
        panel(&self.id, 760., cx)
            .debug_selector(|| "generation-readout".to_owned())
            .role(Role::Group)
            .aria_label(format!("{} {}", self.model, tr(cx, "stats", "统计")))
            .when(inline, |v| v.p_0().border_0().bg(transparent_black()))
            .child(
                div()
                    .w_full()
                    .on_children_prepainted(move |children, _, cx| {
                        if let Some(b) = children.first() {
                            let width = b.size.width + px(32.);
                            let _ = weak.update(cx, |s, cx| {
                                if s.measured_width != Some(width) {
                                    s.measured_width = Some(width);
                                    cx.notify();
                                }
                            });
                        }
                    })
                    .child(
                        row()
                            .w_full()
                            .gap(px(7.))
                            .child(model_icon)
                            .child(title(self.model.clone()).truncate().text_size(px(13.)))
                            .child(
                                row().gap_1().flex_none().child(status_icon).child(
                                    swap_label("generation-status", status, w, cx)
                                        .text_size(px(12.))
                                        .text_color(p.secondary),
                                ),
                            )
                            .child(div().flex_1())
                            .child(details_control),
                    ),
            )
            .child(metrics_content)
            .when(error_fold.mounted(), |v| {
                v.child(
                    error_fold.wrap(
                        12.,
                        row()
                            .justify_between()
                            .child(note(error.unwrap_or_default(), cx).text_color(p.danger))
                            .child(
                                btn(
                                    "retry-generation",
                                    tr(cx, "Retry", "重试"),
                                    false,
                                    cx,
                                    |s, _, cx| {
                                        if matches!(s.state, Some(GenerationState::Failed(_))) {
                                            cx.emit(RetryGeneration);
                                        }
                                    },
                                )
                                .when(!has_error, |v| v.tab_index(-1).cursor_default()),
                            ),
                    ),
                )
            })
            .when(details.mounted(), |v| v.child(details.wrap(12., breakdown)))
    }
}
