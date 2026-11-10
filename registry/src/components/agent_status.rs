//! Show the current activity of an assistant during a run.
use crate::primitives::brand_mark::brand_mark;
use crate::primitives::ops_surface::*;
use crate::primitives::{
    family::{Paint, PhaseClock, disabled_motion, measure},
    glyph::{Glyph, icon, spinner},
};
use gpui::{prelude::*, *};
pub struct Step {
    pub label: String,
    pub result: String,
    pub complete: bool,
}
pub struct WorkBadge {
    id: SharedString,
    pub agent: String,
    pub activity: String,
    pub elapsed: u32,
    pub steps: Vec<Step>,
    open: bool,
    working: bool,
    ring: PhaseClock,
}
impl WorkBadge {
    pub fn new(
        id: impl Into<SharedString>,
        agent: impl Into<String>,
        activity: impl Into<String>,
        steps: Vec<Step>,
    ) -> Self {
        Self {
            id: id.into(),
            agent: agent.into(),
            activity: activity.into(),
            steps,
            elapsed: 0,
            open: false,
            working: false,
            ring: PhaseClock::default(),
        }
    }
}
impl Render for WorkBadge {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        let details = Fold::get(
            (self.id.clone(), 0usize),
            self.open,
            52. + self.steps.len() as f32 * 49.,
            0.24,
            w,
            cx,
        );
        let id = self.id.clone();
        let working = self.steps.iter().any(|step| !step.complete);
        if working && !self.working {
            self.ring.restart(cx);
        }
        self.working = working;
        let turn = if working && !disabled_motion(cx) {
            w.request_animation_frame();
            self.ring.age(cx) / 1.2
        } else {
            0.
        };
        let text: SharedString = self.activity.clone().into();
        let width = (measure(&text, 14., w).ceil() + 4.)
            .min((f32::from(w.viewport_size().width) - 120.).max(24.));
        let width = animated_value("activity-width", width, 0.24, w, cx);
        column()
            .id(id)
            .items_center()
            .w_full()
            .gap_0()
            .child(
                item(
                    "activity",
                    format!("{} · {}", self.agent, self.activity),
                    cx,
                    |s, _, cx| {
                        s.open = !s.open;
                        cx.notify()
                    },
                )
                .aria_expanded(self.open)
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .child(
                    div()
                        .relative()
                        .size(px(24.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .absolute()
                                .size(px(18.))
                                .rounded_full()
                                .border_1()
                                .border_color(if working { p.line } else { p.success }),
                        )
                        .when(working, |v| {
                            v.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .child(spinner(turn, 24., p.success)),
                            )
                        })
                        .child(if !self.steps.is_empty() && !working {
                            icon(Glyph::Check, 12., p.success).into_any_element()
                        } else {
                            brand_mark(&self.agent, 13., cx).unwrap_or_else(|| {
                                icon(Glyph::Bulb, 12., p.success).into_any_element()
                            })
                        }),
                )
                .child(
                    swap_label("activity-label", text, w, cx)
                        .w(px(width))
                        .flex_none()
                        .whitespace_nowrap()
                        .overflow_hidden(),
                )
                .when(self.elapsed > 0, |el| {
                    el.child(note(format!("{}s", self.elapsed), cx))
                })
                .child(icon(
                    if self.open { Glyph::Up } else { Glyph::Down },
                    12.,
                    p.faint,
                )),
            )
            .when(details.mounted(), |el| {
                let steps = panel(&SharedString::from("steps"), 360., cx)
                    .debug_selector(|| "agent-step-panel".into())
                    .child(title(self.agent.clone()))
                    .children(self.steps.iter().enumerate().map(|(i, s)| {
                        row()
                            .child(icon(
                                if s.complete {
                                    Glyph::Check
                                } else {
                                    Glyph::Clock
                                },
                                15.,
                                if s.complete { p.success } else { p.faint },
                            ))
                            .child(
                                column()
                                    .flex_1()
                                    .gap_0()
                                    .child(div().child(s.label.clone()))
                                    .when(!s.result.is_empty(), |el| {
                                        el.child(
                                            note(s.result.clone(), cx)
                                                .id(("step-result", i))
                                                .debug_selector(move || {
                                                    format!("agent-step-result-{i}")
                                                }),
                                        )
                                    }),
                            )
                    }));
                el.child(
                    div()
                        .w_full()
                        .max_w(px(360.))
                        .child(details.wrap(0., div().w_full().pt(px(6.)).child(steps))),
                )
            })
    }
}
