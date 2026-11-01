//! Inspect token usage and timing for a generated response.
use crate::primitives::family::{Paint, Slide};
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
            self.count.duration(data.output_tokens as f32, 0.18, cx)
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
        let speed = d.output_tokens as f32
            / (d.duration_ms.saturating_sub(d.first_token_ms).max(1) as f32 / 1000.);
        let metrics = [
            (
                tr(cx, "First token", "首个词元"),
                format!("{}ms", d.first_token_ms),
            ),
            (tr(cx, "Speed", "速度"), format!("{:.0} tok/s", speed)),
            (tr(cx, "Output tokens", "输出词元"), output.to_string()),
            (tr(cx, "Cost", "费用"), format!("${cost:.4}")),
        ];
        let samples = d
            .samples
            .iter()
            .copied()
            .skip(d.samples.len().saturating_sub(60))
            .collect::<Vec<_>>();
        let spark = canvas(
            |_, _, _| (),
            move |bounds, _, w, _| {
                if samples.len() < 2 {
                    return;
                }
                let min = samples.iter().copied().fold(f32::INFINITY, f32::min);
                let max = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let mut path = PathBuilder::stroke(px(1.5));
                for (i, &v) in samples.iter().enumerate() {
                    let point = bounds.origin
                        + point(
                            bounds.size.width * (i as f32 / (samples.len() - 1) as f32),
                            bounds.size.height * (1. - (v - min) / (max - min).max(1.)),
                        );
                    if i == 0 {
                        path.move_to(point)
                    } else {
                        path.line_to(point)
                    }
                }
                if let Ok(path) = path.build() {
                    w.paint_path(path, p.ink)
                }
            },
        )
        .w(px(45.))
        .h(px(9.))
        .into_any_element();
        let mut spark = Some(spark);
        let mut grid = column().gap(px(13.));
        for (r, cells) in metrics.chunks(2).enumerate() {
            let mut band = row().items_stretch().gap_0();
            for (i, (label, value)) in cells.iter().enumerate() {
                let mut val = row().child(div().text_size(px(16.)).child(value.clone()));
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
        let details = Fold::get("readout-details", self.details, 32., 0.24, w, cx);
        panel(&self.id, 522., cx)
            .child(
                row()
                    .flex_wrap()
                    .child(title(self.model.clone()))
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
                    .child(btn(
                        "details",
                        tr(cx, "Details", "详细信息"),
                        self.details,
                        cx,
                        |s, _, cx| {
                            s.details = !s.details;
                            cx.notify()
                        },
                    )),
            )
            .child(grid)
            .when(details.mounted(), |v| {
                v.child(details.wrap(
                    12.,
                    column().child(divider(cx)).child(note(
                        format!(
                            "{} {} × ${:.2}/1M  ·  {} {} × ${:.2}/1M",
                            d.input_tokens,
                            tr(cx, "in", "输入"),
                            d.input_rate,
                            d.output_tokens,
                            tr(cx, "out", "输出"),
                            d.output_rate
                        ),
                        cx,
                    )),
                ))
            })
    }
}
