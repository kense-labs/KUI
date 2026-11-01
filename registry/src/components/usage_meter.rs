//! Quota visualization with independently addressable segments.
use crate::primitives::{
    family::{Paint, Slide},
    presentation::*,
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct Allocation {
    pub label: SharedString,
    pub amount: f32,
    pub color: Hsla,
}
impl Allocation {
    pub fn new(label: impl Into<SharedString>, amount: f32, color: Hsla) -> Self {
        Self {
            label: label.into(),
            amount: amount.max(0.),
            color,
        }
    }
}
pub struct UsageMeter {
    label: SharedString,
    unit: SharedString,
    capacity: f32,
    parts: Vec<Allocation>,
    amounts: Vec<Slide>,
    limit: Slide,
    registered: bool,
    selected: Option<usize>,
}
impl UsageMeter {
    pub fn new(
        label: impl Into<SharedString>,
        unit: impl Into<SharedString>,
        capacity: f32,
        parts: Vec<Allocation>,
    ) -> Self {
        Self {
            label: label.into(),
            unit: unit.into(),
            capacity: capacity.max(0.),
            parts,
            amounts: vec![],
            limit: Slide::new(capacity.max(0.)),
            registered: false,
            selected: None,
        }
    }
    fn animate(&mut self, cx: &App) {
        self.amounts.resize(self.parts.len(), Slide::new(0.));
        for (i, (amount, part)) in self.amounts.iter_mut().zip(&self.parts).enumerate() {
            if self.registered {
                amount.duration(part.amount, 0.4, cx)
            } else {
                amount.delayed(part.amount, 0.58, i as f32 * 0.07, cx)
            }
        }
        self.limit.duration(self.capacity, 0.4, cx);
        self.registered = true;
    }
    pub fn set_usage(&mut self, capacity: f32, parts: Vec<Allocation>, cx: &mut Context<Self>) {
        self.capacity = capacity.max(0.);
        self.parts = parts;
        self.selected = None;
        self.animate(cx);
        cx.notify()
    }
}
impl Render for UsageMeter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        if !self.registered {
            self.animate(cx);
        }
        let shown: Vec<_> = self.amounts.iter().map(|s| s.get(w, cx)).collect();
        let sum = shown.iter().sum::<f32>();
        let capacity = self.limit.get(w, cx);
        let total = sum.max(capacity).max(f32::EPSILON);
        let unit = &self.unit;
        let target_sum = self.parts.iter().map(|s| s.amount).sum::<f32>();
        let free = capacity - sum;
        frame(496., p)
            .gap_3()
            .child(
                row()
                    .justify_between()
                    .child(title(self.label.clone(), p))
                    .child(hint(
                        format!(
                            "{:.1} {} {}",
                            free.abs(),
                            unit,
                            if free >= 0. {
                                tr(cx, "free", "可用")
                            } else {
                                tr(cx, "over quota", "超出配额")
                            }
                        ),
                        p,
                    )),
            )
            .child(
                div()
                    .text_size(px(22.))
                    .text_color(p.ink)
                    .child(format!("{sum:.1} / {capacity:.1} {unit}")),
            )
            .child(
                row()
                    .id("usage_meter-meter")
                    .role(Role::Meter)
                    .aria_numeric_value(target_sum as f64)
                    .aria_min_numeric_value(0.)
                    .aria_max_numeric_value(self.capacity as f64)
                    .aria_label(format!("{target_sum:.1} / {:.1} {unit}", self.capacity))
                    .w_full()
                    .h(px(14.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.line)
                    .overflow_hidden()
                    .children(
                        self.parts
                            .iter()
                            .zip(&shown)
                            .enumerate()
                            .map(|(i, (s, width))| {
                                div()
                                    .h_full()
                                    .w(relative((width / total).clamp(0., 1.)))
                                    .bg(s.color)
                                    .opacity(if self.selected.is_none_or(|n| n == i) {
                                        1.
                                    } else {
                                        0.45
                                    })
                            }),
                    ),
            )
            .child(
                row()
                    .flex_wrap()
                    .gap_2()
                    .children(self.parts.iter().enumerate().map(|(i, s)| {
                        command(
                            ("allocation", i),
                            format!("{}  {:.1}", s.label, shown[i]),
                            cx,
                            move |v, _, cx| {
                                v.selected = if v.selected == Some(i) { None } else { Some(i) };
                                cx.notify()
                            },
                        )
                        .border_0()
                        .text_size(px(12.))
                        .child(dot(7., s.color))
                    }))
                    .when(free > 0., |el| {
                        el.child(
                            command(
                                "available-quota",
                                format!("{}  {free:.1}", tr(cx, "Free", "可用")),
                                cx,
                                |v, _, cx| {
                                    let free = v.parts.len();
                                    v.selected = if v.selected == Some(free) {
                                        None
                                    } else {
                                        Some(free)
                                    };
                                    cx.notify()
                                },
                            )
                            .border_0()
                            .text_size(px(12.)),
                        )
                    }),
            )
    }
}
