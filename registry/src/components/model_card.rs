//! Inspect a model’s capabilities and select it for a task.
use crate::primitives::brand_mark::brand_mark;
use crate::primitives::family::{Paint, Slide};
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
pub struct Model {
    pub name: String,
    pub provider: String,
    pub description: String,
    pub features: Vec<String>,
    pub context: u32,
    pub max_output: u32,
    pub input_rate: f32,
    pub output_rate: f32,
    pub scores: Vec<(String, f32)>,
}
pub struct ModelSheet {
    id: SharedString,
    pub models: Vec<Model>,
    selected: usize,
    comparison: Option<usize>,
    volume: u32,
    track: std::rc::Rc<std::cell::Cell<Bounds<Pixels>>>,
    dragging: bool,
    price: Slide,
}
pub struct ModelChanged(pub usize);
impl EventEmitter<ModelChanged> for ModelSheet {}
impl ModelSheet {
    pub fn new(id: impl Into<SharedString>, models: Vec<Model>) -> Self {
        let p = models.first().map_or(0., |m| m.input_rate);
        Self {
            id: id.into(),
            models,
            selected: 0,
            comparison: None,
            volume: 100,
            track: Default::default(),
            dragging: false,
            price: Slide::new(p),
        }
    }
    fn scrub(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let b = self.track.get();
        if b.size.width > px(0.) {
            self.volume =
                (1. + 999. * ((x - b.left()) / b.size.width).clamp(0., 1.)).round() as u32;
            cx.notify()
        }
    }
    pub fn choose(&mut self, i: usize, cx: &mut Context<Self>) {
        if i < self.models.len() && i != self.selected {
            self.selected = i;
            self.comparison = None;
            self.price.set(self.models[i].input_rate, cx);
            cx.emit(ModelChanged(i));
            cx.notify()
        }
    }
}
impl Render for ModelSheet {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        let mut r = panel(&self.id, 522., cx);
        let Some(m) = self.models.get(self.selected) else {
            return r.child(note(tr(cx, "No models", "暂无模型"), cx));
        };
        r = r
            .child(
                row()
                    .flex_wrap()
                    .children(self.models.iter().enumerate().map(|(i, m)| {
                        btn(
                            ("model", i),
                            m.name.clone(),
                            self.selected == i,
                            cx,
                            move |s, _, cx| s.choose(i, cx),
                        )
                    })),
            )
            .child(
                row()
                    .child(
                        brand_mark(&m.name, 23., cx)
                            .or_else(|| brand_mark(&m.provider, 23., cx))
                            .unwrap_or_else(|| {
                                avatar(&m.name, self.selected, cx).into_any_element()
                            }),
                    )
                    .child(
                        column()
                            .flex_1()
                            .gap_0()
                            .child(
                                swap_label("model-name", m.name.clone(), w, cx)
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(note(m.provider.clone(), cx)),
                    )
                    .when(self.models.len() > 1, |el| {
                        el.child(btn(
                            "compare",
                            tr(cx, "Compare", "比较"),
                            self.comparison.is_some(),
                            cx,
                            |s, _, cx| {
                                s.comparison = if s.comparison.is_some() {
                                    None
                                } else {
                                    Some((s.selected + 1) % s.models.len())
                                };
                                cx.notify()
                            },
                        ))
                    }),
            )
            .child(
                swap_label("model-description", m.description.clone(), w, cx)
                    .text_color(p.secondary)
                    .text_size(px(12.)),
            )
            .child(
                row()
                    .flex_wrap()
                    .children(m.features.iter().map(|s| badge(s.clone(), cx))),
            )
            .child(divider(cx));
        if let Some(i) = self.comparison.and_then(|i| self.models.get(i).map(|_| i)) {
            let other = &self.models[i];
            r = r
                .child(
                    row().flex_wrap().children(
                        self.models
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != self.selected)
                            .map(|(j, m)| {
                                btn(("rival", j), m.name.clone(), j == i, cx, move |s, _, cx| {
                                    s.comparison = Some(j);
                                    cx.notify()
                                })
                            }),
                    ),
                )
                .child(
                    row()
                        .child(div().flex_1())
                        .child(
                            swap_label("model-name", m.name.clone(), w, cx)
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(title(other.name.clone())),
                );
            for (l, a, b) in [
                (
                    tr(cx, "Context", "上下文"),
                    m.context as f32,
                    other.context as f32,
                ),
                (
                    tr(cx, "Input / 1M", "每百万输入"),
                    m.input_rate,
                    other.input_rate,
                ),
                (
                    tr(cx, "Output / 1M", "每百万输出"),
                    m.output_rate,
                    other.output_rate,
                ),
            ] {
                r = r.child(
                    row()
                        .child(div().flex_1().child(l))
                        .child(badge(a.to_string(), cx))
                        .child(badge(b.to_string(), cx)),
                )
            }
        } else {
            r = r
                .child(
                    row()
                        .flex_wrap()
                        .child(
                            column()
                                .flex_1()
                                .child(note(tr(cx, "Context window", "上下文窗口"), cx))
                                .child(title(format!(
                                    "{:.0}",
                                    animated_value("context-size", m.context as f32, 0.24, w, cx)
                                ))),
                        )
                        .child(
                            column()
                                .flex_1()
                                .child(note(tr(cx, "Max output", "输出上限"), cx))
                                .child(title(format!(
                                    "{:.0}",
                                    animated_value(
                                        "output-limit",
                                        m.max_output as f32,
                                        0.24,
                                        w,
                                        cx
                                    )
                                ))),
                        ),
                )
                .child(note(
                    format!(
                        "{} · ${:.2} / ${:.2}",
                        tr(cx, "Input / output per million", "每百万输入 / 输出"),
                        self.price.get(w, cx),
                        m.output_rate
                    ),
                    cx,
                ))
                .child(
                    row()
                        .justify_between()
                        .child(note(tr(cx, "Monthly volume", "每月用量"), cx))
                        .child(title(format!("{}M", self.volume))),
                )
                .child(
                    div()
                        .on_children_prepainted({
                            let track = self.track.clone();
                            move |b, _, _| {
                                if let Some(b) = b.first() {
                                    track.set(*b)
                                }
                            }
                        })
                        .id("volume")
                        .role(Role::Slider)
                        .aria_label(tr(cx, "Monthly volume", "每月用量"))
                        .tab_index(0)
                        .w_full()
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .px(px(6.))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|s, e: &MouseDownEvent, _, cx| {
                                s.dragging = true;
                                s.scrub(e.position.x, cx)
                            }),
                        )
                        .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                            if s.dragging {
                                s.scrub(e.position.x, cx)
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|s, _, _, _| s.dragging = false),
                        )
                        .on_mouse_up_out(
                            MouseButton::Left,
                            cx.listener(|s, _, _, _| s.dragging = false),
                        )
                        .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                            match e.keystroke.key.as_str() {
                                "left" | "down" => s.volume = s.volume.saturating_sub(5).max(1),
                                "right" | "up" => s.volume = (s.volume + 5).min(1000),
                                "home" => s.volume = 1,
                                "end" => s.volume = 1000,
                                _ => return,
                            }
                            cx.notify();
                            cx.stop_propagation()
                        }))
                        .child(
                            div()
                                .relative()
                                .w_full()
                                .h(px(4.))
                                .bg(p.stage)
                                .rounded(radius)
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative((self.volume - 1) as f32 / 999.))
                                        .rounded(radius)
                                        .bg(p.ink),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(relative((self.volume - 1) as f32 / 999.))
                                        .ml(px(-6.))
                                        .top(px(-4.))
                                        .size(px(12.))
                                        .rounded_full()
                                        .bg(p.ink),
                                ),
                        ),
                )
                .child(
                    row()
                        .justify_between()
                        .child(note(tr(cx, "Estimated monthly cost", "预估月费用"), cx))
                        .child(title(format!(
                            "${:.2}",
                            self.volume as f32 * (m.input_rate * 0.8 + m.output_rate * 0.2)
                        ))),
                )
                .children(m.scores.iter().enumerate().map(|(i, (s, n))| {
                    let value = animated_value(("benchmark", i), *n, 0.26, w, cx);
                    column()
                        .gap(px(2.))
                        .child(note(format!("{} · {:.0}%", s, value * 100.), cx))
                        .child(bar(value, p.success, cx))
                }))
        }
        r
    }
}
