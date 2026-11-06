//! Illustrative model-training sequence; all values are supplied by the host.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
    presentation::{column, hint, row, tr},
};
use gpui::{prelude::*, *};
use std::f32::consts::TAU;

#[derive(Clone, Debug)]
pub struct TrainingConfig {
    pub classes: [SharedString; 2],
    pub samples: usize,
    pub held_out: usize,
    pub batch_size: usize,
    pub epochs: usize,
    pub loss_from: f32,
    pub loss_to: f32,
    pub accuracy: f32,
    pub target: f32,
}
impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            classes: ["Positive".into(), "Negative".into()],
            samples: 12_800,
            held_out: 1_600,
            batch_size: 32,
            epochs: 12,
            loss_from: 0.69,
            loss_to: 0.08,
            accuracy: 94.2,
            target: 90.,
        }
    }
}
#[derive(Clone, Debug)]
pub struct TrainingStepChanged {
    pub step: usize,
}
#[derive(Clone, Debug)]
pub struct TrainingPlaybackChanged {
    pub paused: bool,
}
pub struct ModelTraining {
    config: TrainingConfig,
    elapsed: f32,
    selected: Option<usize>,
    paused: bool,
    visible: bool,
    hovered: bool,
    speed: f32,
    hover_speed: Slide,
    clock: PhaseClock,
    last_age: f32,
}
impl EventEmitter<TrainingStepChanged> for ModelTraining {}
impl EventEmitter<TrainingPlaybackChanged> for ModelTraining {}
const DURATION: f32 = 21.;
const STAGES: [f32; 6] = [0., 0.16, 0.32, 0.48, 0.82, 1.];
impl ModelTraining {
    pub fn new(config: TrainingConfig) -> Self {
        Self {
            config,
            elapsed: 0.,
            selected: None,
            paused: false,
            visible: true,
            hovered: false,
            speed: 1.,
            hover_speed: Slide::new(1.),
            clock: PhaseClock::default(),
            last_age: 0.,
        }
    }
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = if speed.is_finite() {
            speed.clamp(0., 4.)
        } else {
            1.
        };
        self
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_age = 0.;
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.emit(TrainingPlaybackChanged { paused });
            cx.notify();
        }
    }
    pub fn set_step(&mut self, step: usize, cx: &mut Context<Self>) {
        let step = step.min(4);
        self.selected = Some(step);
        self.elapsed = (STAGES[step] + (STAGES[step + 1] - STAGES[step]) * 0.75) * DURATION;
        self.set_paused(true, cx);
        cx.emit(TrainingStepChanged { step });
        cx.notify();
    }
    pub fn set_config(&mut self, config: TrainingConfig, cx: &mut Context<Self>) {
        self.config = config;
        self.elapsed = 0.;
        self.selected = None;
        self.clock.restart(cx);
        self.last_age = 0.;
        cx.notify();
    }
}
fn segment(t: f32) -> usize {
    STAGES
        .windows(2)
        .position(|v| t >= v[0] && t < v[1])
        .unwrap_or(4)
}
fn paint_circle(center: Point<Pixels>, radius: f32, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::fill();
    for i in 0..=18 {
        let a = TAU * i as f32 / 18.;
        let q = point(
            center.x + px(a.cos() * radius),
            center.y + px(a.sin() * radius),
        );
        if i == 0 {
            path.move_to(q)
        } else {
            path.line_to(q)
        }
    }
    path.close();
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
fn line(a: Point<Pixels>, b: Point<Pixels>, width: f32, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::stroke(px(width));
    path.move_to(a);
    path.line_to(b);
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
impl Render for ModelTraining {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let running = self.visible && w.is_visible() && !self.paused && self.speed > 0. && !reduced;
        let target_speed = if self.hovered { 0.35 } else { 1. };
        self.hover_speed.duration(target_speed, 0.5, cx);
        let hover_speed = if running {
            self.hover_speed.get(w, cx)
        } else {
            self.hover_speed.value(cx)
        };
        if running {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
                self.last_age = 0.;
            }
            let age = self.clock.age(cx);
            let delta = (age - self.last_age).clamp(0., 0.1);
            self.last_age = age;
            self.elapsed = (self.elapsed + delta * self.speed * hover_speed) % DURATION;
            self.selected = None;
            w.request_animation_frame();
        } else {
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let t = if reduced && self.selected.is_none() {
            1.
        } else {
            (self.elapsed / DURATION).clamp(0., 1.)
        };
        let stage = self.selected.unwrap_or_else(|| segment(t));
        let learned = ((t - STAGES[2]) / (STAGES[4] - STAGES[2])).clamp(0., 1.);
        let epoch = (learned * self.config.epochs as f32).round() as usize;
        let start = if self.config.loss_from.is_finite() {
            self.config.loss_from
        } else {
            0.69
        };
        let end = if self.config.loss_to.is_finite() {
            self.config.loss_to
        } else {
            0.08
        };
        let loss = start + (end - start) * (1. - (-3. * learned).exp()) / (1. - (-3f32).exp());
        let evaluation = (t - STAGES[4]).max(0.) / (1. - STAGES[4]);
        let accuracy = if self.config.accuracy.is_finite() {
            self.config.accuracy.clamp(0., 100.)
        } else {
            0.
        };
        let target = if self.config.target.is_finite() {
            self.config.target.clamp(0., 100.)
        } else {
            0.
        };
        let status = if evaluation <= 0. {
            tr(cx, "Waiting", "等待")
        } else if evaluation < 0.98 {
            tr(cx, "Running", "评估中")
        } else if accuracy >= target {
            tr(cx, "Passed", "已达标")
        } else {
            tr(cx, "Below target", "未达标")
        };
        let stage_names = [
            tr(cx, "Batch", "批次"),
            tr(cx, "Forward pass", "前向传播"),
            tr(cx, "Backprop", "反向传播"),
            tr(cx, "Epochs", "训练轮次"),
            tr(cx, "Evaluate", "评估"),
        ];
        let captions = [
            tr(
                cx,
                "A batch enters the input layer.",
                "一批样本进入输入层。",
            ),
            tr(
                cx,
                "Signals flow through the network to a prediction.",
                "信号穿过网络并生成预测。",
            ),
            tr(
                cx,
                "The error travels back and adjusts connections.",
                "误差反向传播并调整连接。",
            ),
            tr(
                cx,
                "Repeated batches reduce the loss.",
                "重复训练批次使损失下降。",
            ),
            tr(
                cx,
                "A held-out set checks the trained model.",
                "使用留出样本评估训练后的模型。",
            ),
        ];
        let diagram_label = format!(
            "{}. {}: {}. {}: {loss:.2}. {}: {epoch} / {}. {}: {status} {:.1}%",
            tr(cx, "Illustrative model training", "模型训练示意"),
            tr(cx, "Stage", "阶段"),
            stage_names[stage],
            tr(cx, "Loss", "损失"),
            tr(cx, "Epoch", "轮次"),
            self.config.epochs,
            tr(cx, "Evaluation", "评估"),
            accuracy * evaluation
        );
        let image = div()
            .id("training-diagram")
            .role(Role::Image)
            .aria_label(diagram_label)
            .w_full()
            .min_w(px(250.))
            .h(px(316.))
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, w, _| {
                        let width = f32::from(bounds.size.width);
                        let height = f32::from(bounds.size.height);
                        let left = 28.;
                        let right = (width - 28.).max(left + 1.);
                        let layers = [3usize, 4, 4, 2];
                        let node = |layer: usize, index: usize| {
                            let x = left + (right - left) * layer as f32 / 3.;
                            let count = layers[layer];
                            let y = height / 2. + (index as f32 - (count - 1) as f32 / 2.) * 37.;
                            point(bounds.origin.x + px(x), bounds.origin.y + px(y))
                        };
                        for layer in 0..3 {
                            for i in 0..layers[layer] {
                                for j in 0..layers[layer + 1] {
                                    let a = node(layer, i);
                                    let b = node(layer + 1, j);
                                    let strong = ((i * 3 + j * 7 + layer) % 4) == 0;
                                    line(
                                        a,
                                        b,
                                        if strong { 1. + learned * 1.5 } else { 1. },
                                        if strong {
                                            p.line.blend(p.bright.opacity(learned * 0.55))
                                        } else {
                                            p.line
                                        },
                                        w,
                                    );
                                    if (STAGES[1]..STAGES[4]).contains(&t) {
                                        let phase = (t * 15.
                                            + layer as f32 * 0.19
                                            + i as f32 * 0.07
                                            + j as f32 * 0.03)
                                            .fract();
                                        let phase = if stage == 2 { 1. - phase } else { phase };
                                        let q = point(
                                            a.x + (b.x - a.x) * phase,
                                            a.y + (b.y - a.y) * phase,
                                        );
                                        paint_circle(
                                            q,
                                            2.2,
                                            if stage == 2 {
                                                p.danger.opacity(0.8)
                                            } else {
                                                p.bright.opacity(0.8)
                                            },
                                            w,
                                        );
                                    }
                                }
                            }
                        }
                        for (layer, &count) in layers.iter().enumerate() {
                            for i in 0..count {
                                let q = node(layer, i);
                                paint_circle(q, 7., p.panel, w);
                                paint_circle(
                                    q,
                                    if layer == 3 { 4.5 } else { 4. },
                                    if t >= STAGES[1] {
                                        p.bright.opacity(0.85)
                                    } else {
                                        p.secondary.opacity(0.55)
                                    },
                                    w,
                                );
                            }
                        }
                    },
                )
                .size_full(),
            );
        let graph = div()
            .id("training-loss-chart")
            .role(Role::Image)
            .aria_label(format!(
                "{} {:.2} → {:.2}",
                tr(cx, "Loss", "损失"),
                start,
                loss
            ))
            .w_full()
            .h(px(112.))
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, w, _| {
                        let width = f32::from(bounds.size.width);
                        let height = f32::from(bounds.size.height);
                        let mut path = PathBuilder::stroke(px(1.75));
                        let steps = (48. * learned).ceil() as usize;
                        for i in 0..=steps {
                            let f = (i as f32 / 48.).min(learned);
                            let ratio = (1. - (-3. * f).exp()) / (1. - (-3f32).exp());
                            let q = point(
                                bounds.origin.x + px(f * width),
                                bounds.origin.y + px(8. + ratio * (height - 16.)),
                            );
                            if i == 0 {
                                path.move_to(q)
                            } else {
                                path.line_to(q)
                            }
                        }
                        if let Ok(path) = path.build() {
                            w.paint_path(path, p.bright);
                        }
                    },
                )
                .size_full(),
            );
        let steps = row()
            .gap(px(8.))
            .flex_1()
            .children(stage_names.iter().enumerate().map(|(i, name)| {
                let current = stage == i;
                face(
                    ("training-step", i),
                    format!("{} {}", i + 1, name),
                    44.,
                    0.,
                    Tone::Clear,
                    cx,
                )
                .flex_1()
                .h(px(44.))
                .flex_col()
                .justify_center()
                .gap(px(7.))
                .text_size(px(12.))
                .line_height(px(15.6))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if current { p.ink } else { p.faint })
                .when(!current, |step| step.hover(|s| s.text_color(p.secondary)))
                .aria_toggled(current.into())
                .child(
                    row()
                        .items_baseline()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_color(if current { p.bright } else { p.faint })
                                .child(format!("{}", i + 1)),
                        )
                        .child(name.to_string()),
                )
                .child(
                    div().w_full().h(px(2.)).rounded_full().bg(p.line).child(
                        div()
                            .h_full()
                            .w(relative(if i <= stage { 1. } else { 0. }))
                            .rounded_full()
                            .bg(p.faint),
                    ),
                )
                .on_click(cx.listener(move |s, _, _, cx| s.set_step(i, cx)))
            }));
        column()
            .id("model-training")
            .w_full()
            .max_w(px(960.))
            .gap(px(24.))
            .p(px(16.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .justify_between()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr(cx, "Model training", "模型训练")),
                    )
                    .child(
                        face(
                            "training-playback",
                            if self.paused {
                                tr(cx, "Play", "播放")
                            } else {
                                tr(cx, "Pause", "暂停")
                            },
                            36.,
                            18.,
                            Tone::Clear,
                            cx,
                        )
                        .w(px(36.))
                        .px(px(0.))
                        .border_1()
                        .border_color(p.line)
                        .hover(|s| s.bg(p.tray))
                        .child(if self.paused { "▶" } else { "❚❚" })
                        .on_click(cx.listener(|s, _, _, cx| s.set_paused(!s.paused, cx))),
                    ),
            )
            .child(hint(
                tr(
                    cx,
                    "Illustrative example, not real training metrics",
                    "以下为示意数据，不代表真实训练指标",
                ),
                p,
            ))
            .child(
                row().gap_2().flex_wrap().children(
                    [
                        (tr(cx, "Training set", "训练集"), self.config.samples),
                        (tr(cx, "Held-out set", "留出集"), self.config.held_out),
                    ]
                    .into_iter()
                    .map(|(name, count)| {
                        column()
                            .gap_1()
                            .px(px(10.))
                            .py(px(7.))
                            .rounded(crate::foundation::surface_radius(cx))
                            .bg(p.stage)
                            .child(name)
                            .child(hint(format!("{count} {}", tr(cx, "samples", "条样本")), p))
                    }),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(40.))
                    .child(div().flex_1().min_w(px(250.)).child(image))
                    .child(
                        column()
                            .w(px(232.))
                            .max_w_full()
                            .gap(px(16.))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(16.))
                                    .child(stat(tr(cx, "Loss", "损失"), format!("{loss:.2}"), p))
                                    .child(stat(
                                        tr(cx, "Epoch", "轮次"),
                                        format!("{epoch} / {}", self.config.epochs),
                                        p,
                                    )),
                            )
                            .child(graph),
                    ),
            )
            .child(
                div()
                    .pt(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr(cx, "Evaluation", "评估")),
                    )
                    .child({
                        let (fg, bg, edge) = if evaluation <= 0. {
                            (p.secondary, p.tray, p.line)
                        } else if evaluation < 0.98 {
                            (p.bright, p.bright.opacity(0.09), p.bright.opacity(0.28))
                        } else {
                            (p.success, p.success.opacity(0.10), p.success.opacity(0.30))
                        };
                        div()
                            .pt(px(1.))
                            .pb(px(1.))
                            .px(px(7.))
                            .rounded(px(8.))
                            .text_size(px(12.))
                            .line_height(px(16.2))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(fg)
                            .bg(bg)
                            .border_1()
                            .border_color(edge)
                            .child(status)
                    })
                    .child(
                        div()
                            .min_w(px(54.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("{:.1}%", accuracy * evaluation)),
                    ),
            )
            .child(
                row()
                    .gap_3()
                    .flex_wrap()
                    .children(self.config.classes.iter().enumerate().map(|(i, name)| {
                        let probability = if i == 0 {
                            0.5 + 0.42 * learned
                        } else {
                            0.5 - 0.42 * learned
                        };
                        column()
                            .flex_1()
                            .min_w(px(110.))
                            .gap_1()
                            .child(hint(format!("{} {:.0}%", name, probability * 100.), p))
                            .child(
                                div().w_full().h(px(4.)).rounded(px(2.)).bg(p.line).child(
                                    div()
                                        .h_full()
                                        .w(relative(probability))
                                        .rounded(px(2.))
                                        .bg(p.bright),
                                ),
                            )
                    })),
            )
            .child(steps)
            .child(hint(
                if stage == 0 {
                    format!("{} {}", self.config.batch_size, captions[stage])
                } else {
                    captions[stage].to_string()
                },
                p,
            ))
    }
}

fn stat(label: &str, value: String, p: Paint) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .min_w(px(0.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(20.))
                .text_color(p.secondary)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(24.))
                .line_height(px(27.6))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(value),
        )
}
#[cfg(test)]
mod tests {
    use super::{TrainingConfig, segment};
    #[test]
    fn stage_boundaries_and_defaults_are_stable() {
        assert_eq!(segment(0.), 0);
        assert_eq!(segment(0.16), 1);
        assert_eq!(segment(0.32), 2);
        assert_eq!(segment(0.48), 3);
        assert_eq!(segment(0.82), 4);
        assert_eq!(segment(1.), 4);
        let config = TrainingConfig::default();
        assert_eq!(config.batch_size, 32);
        assert_eq!(config.epochs, 12);
    }
}
