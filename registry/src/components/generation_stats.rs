//! Inspect token usage and timing for a generated response.
use crate::primitives::family::{Paint, Slide};
use crate::primitives::glyph::{Glyph, icon};
use crate::primitives::ops_surface::*;
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
pub struct GenerationReadout {
    id: SharedString,
    pub model: String,
    data: Metrics,
    details: bool,
    count: Slide,
}
impl GenerationReadout {
    pub fn new(id: impl Into<SharedString>, model: impl Into<String>, data: Metrics) -> Self {
        let count = Slide::new(data.output_tokens as f32);
        Self {
            id: id.into(),
            model: model.into(),
            data,
            details: false,
            count,
        }
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
        let output = self.count.get(w, cx).round() as u32;
        let cost = (d.input_tokens as f64 * d.input_rate + output as f64 * d.output_rate) / 1e6;
        let sampling_ms = d.duration_ms.saturating_sub(d.first_token_ms);
        let average_speed =
            (sampling_ms > 0).then(|| d.output_tokens as f32 * 1000. / sampling_ms as f32);
        let speed = if d.running {
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
            (tr(cx, "First token", "首个词元"), time(d.first_token_ms)),
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
        for (r, cells) in metrics.chunks(2).enumerate() {
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
                        .when(i == 1, |el| {
                            el.pl(px(16.)).border_l_1().border_color(p.line)
                        })
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
                    .child(note(tr(cx, "Tokens", "词元"), cx))
                    .child(note(tr(cx, "Rate / 1M", "每百万词元费率"), cx)),
            )
            .children(
                [
                    (tr(cx, "Input", "输入"), d.input_tokens, d.input_rate),
                    (tr(cx, "Output", "输出"), d.output_tokens, d.output_rate),
                ]
                .map(|(label, count, rate)| {
                    row()
                        .justify_between()
                        .child(note(format!("{label}  {count}"), cx))
                        .child(note(format!("${rate:.2}"), cx))
                }),
            );
        panel(&self.id, 522., cx)
            .child(
                row()
                    .flex_wrap()
                    .child(icon(Glyph::Monitor, 16., p.secondary))
                    .child(title(self.model.clone()).truncate())
                    .child(note(
                        format!(
                            "{} {:.2}s",
                            if d.running {
                                tr(cx, "Generating", "生成中")
                            } else {
                                tr(cx, "✓  Done in", "✓  已完成")
                            },
                            d.duration_ms as f32 / 1000.
                        ),
                        cx,
                    ))
                    .child(div().flex_1())
                    .child(
                        btn(
                            "details",
                            tr(cx, "Details", "详细信息"),
                            self.details,
                            cx,
                            |s, _, cx| {
                                s.details = !s.details;
                                cx.notify()
                            },
                        )
                        .aria_expanded(self.details),
                    ),
            )
            .child(grid)
            .when(details.mounted(), |v| v.child(details.wrap(12., breakdown)))
    }
}
