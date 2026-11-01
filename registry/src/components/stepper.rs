//! Sequential navigation with controlled advancement and revisitable completed steps.
use crate::primitives::{
    family::{Paint, Slide, Tone, face as button_face},
    glyph::{Glyph, transformed},
    presentation::*,
};
use gpui::{prelude::*, *};
// Marker geometry defines both axes. The larger pointer target never occupies track space.
const MARKER: f32 = 28.;
const HIT: f32 = 32.;
const HALO: f32 = 4.;
const LINE: f32 = 2.;
const VERTICAL_GAP: f32 = 24.;
const VERTICAL_CLEARANCE: f32 = 6.;
const HORIZONTAL_CLEARANCE: f32 = 8.;

type Validator = std::rc::Rc<dyn Fn(&mut Window, &mut App) -> Option<SharedString>>;
#[derive(Clone)]
pub struct Step {
    pub label: SharedString,
    pub description: SharedString,
    content: Option<AnyView>,
    validator: Option<Validator>,
}
impl Step {
    /// Retain the editor or other page state when navigating between steps.
    pub fn with_content(mut self, content: impl Into<AnyView>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Return a localized explanation to keep the current step active.
    pub fn validate_with(
        mut self,
        validate: impl Fn(&mut Window, &mut App) -> Option<SharedString> + 'static,
    ) -> Self {
        self.validator = Some(std::rc::Rc::new(validate));
        self
    }
    pub fn new(label: impl Into<SharedString>, desc: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: desc.into(),
            content: None,
            validator: None,
        }
    }
}
#[derive(Clone, Copy)]
pub enum StepEvent {
    Changed(usize),
    Finished,
}
pub struct Stepper {
    steps: Vec<Step>,
    current: usize,
    furthest: usize,
    vertical: bool,
    compact: bool,
    rails: Vec<Slide>,
    rings: Vec<Slide>,
    message: ChangingText,
    error: Option<usize>,
}
impl EventEmitter<StepEvent> for Stepper {}
impl Stepper {
    pub fn new(steps: Vec<Step>, initial: usize) -> Self {
        let current = initial.min(steps.len().saturating_sub(1));
        Self {
            rails: (0..steps.len())
                .map(|i| Slide::new((i < current) as u8 as f32))
                .collect(),
            rings: (0..steps.len())
                .map(|i| Slide::new((i == current) as u8 as f32))
                .collect(),
            message: ChangingText::new(
                steps
                    .get(current)
                    .map(|s| s.description.clone())
                    .unwrap_or_default(),
            ),
            error: None,
            steps,
            current,
            furthest: current,
            vertical: false,
            compact: false,
        }
    }
    /// Place the marker column beside the active content in vertical orientation.
    pub fn with_compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
    pub fn set_vertical(&mut self, vertical: bool, cx: &mut Context<Self>) {
        self.vertical = vertical;
        cx.notify()
    }
    pub fn select(&mut self, i: usize, cx: &mut Context<Self>) {
        if i < self.steps.len() && self.current != i {
            let direction = (i as f32 - self.current as f32).signum();
            for (n, rail) in self.rails.iter_mut().enumerate() {
                rail.delayed(
                    (n < i) as u8 as f32,
                    0.4,
                    0.035 * n.abs_diff(self.current).min(4) as f32,
                    cx,
                );
                self.rings[n].duration((n == i) as u8 as f32, 0.26, cx);
            }
            self.error = None;
            self.message
                .set(self.steps[i].description.clone(), direction, cx);
            self.current = i;
            self.furthest = self.furthest.max(i);

            cx.emit(StepEvent::Changed(i));
            cx.notify()
        }
    }
}
impl Render for Stepper {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let el = column().w(px(470.)).max_w_full().gap_4();
        let Some(active) = self.steps.get(self.current) else {
            return el;
        };
        let vertical = self.vertical;
        let sidebar = vertical && self.compact;
        let mut rail = if vertical {
            column()
                .items_start()
                .when(sidebar, |r| r.flex_none().w(px(MARKER)))
        } else {
            row().w_full()
        };
        for (i, s) in self.steps.iter().enumerate() {
            let filled = self.rails[i].get(w, cx).clamp(0., 1.);
            let active_ring = self.rings[i].get(w, cx).clamp(0., 1.);
            let error = self.error == Some(i);
            let ink = if error { p.danger } else { p.ink };
            let disc_color = p.panel.blend(ink.opacity(filled));
            let ring_size = (MARKER + 2. * HALO) * (0.6 + 0.4 * active_ring);
            let mut disc = row()
                .relative()
                .size(px(MARKER))
                .debug_selector(move || format!("step-fill-{i}"))
                .flex_none()
                .justify_center()
                .rounded_full()
                .border(px(1. + active_ring * 0.5))
                .border_color(p.line.blend(ink.opacity(filled.max(active_ring))))
                .bg(disc_color)
                .when(filled > 0.001, |d| {
                    d.group_hover("step-marker", move |s| {
                        s.bg(disc_color.blend(p.panel.opacity(0.15)))
                    })
                })
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(ink)
                        .opacity(1. - filled)
                        .child(if error {
                            "!".to_string()
                        } else {
                            (i + 1).to_string()
                        }),
                );
            if filled > 0.001 {
                disc = disc.child(div().absolute().child(transformed(
                    Glyph::Check,
                    14.,
                    p.neutral_inverse.opacity(filled),
                    0.,
                    0.6 + filled * 0.4,
                    (0., 0.),
                    filled,
                )));
            }
            let face = div()
                .debug_selector(move || format!("step-face-{i}"))
                .relative()
                .flex_none()
                .size(px(MARKER))
                .when(active_ring > 0.001, |el| {
                    el.child(
                        div()
                            .absolute()
                            .debug_selector(move || format!("step-ring-{i}"))
                            .left(px((MARKER - ring_size) / 2.))
                            .top(px((MARKER - ring_size) / 2.))
                            .size(px(ring_size))
                            .rounded_full()
                            .bg(if error {
                                p.danger.opacity(0.16 * active_ring)
                            } else {
                                p.hover.opacity(active_ring)
                            }),
                    )
                })
                .child(disc);
            let node = if i <= self.furthest {
                button_face(
                    ("stage", i),
                    s.label.clone(),
                    HIT,
                    HIT / 2.,
                    Tone::Clear,
                    cx,
                )
                .on_click(cx.listener(move |v, _, _, cx| v.select(i, cx)))
                .group("step-marker")
                .debug_selector(move || format!("step-hit-{i}"))
                .aria_selected(i == self.current)
                .absolute()
                .left(px((MARKER - HIT) / 2.))
                .top(px((MARKER - HIT) / 2.))
                .size(px(HIT))
                .p_0()
                .border_0()
                .bg(transparent_black())
                .rounded_full()
                .child(face)
                .into_any_element()
            } else {
                face.into_any_element()
            };
            let marker = div().relative().flex_none().size(px(MARKER)).child(node);
            let has_next = i + 1 < self.steps.len();
            if vertical {
                let mut item = row()
                    .w_full()
                    .relative()
                    .items_start()
                    .gap(px(12.))
                    .child(marker)
                    .when(!sidebar, |e| {
                        e.child(hint(s.label.clone(), p).min_w_0().pt(px(4.)))
                    });
                if has_next {
                    // The connector spans the real row height, including wrapped labels.
                    item = item.pb(px(VERTICAL_GAP)).child(
                        div()
                            .debug_selector(move || format!("step-rail-{i}"))
                            .absolute()
                            .left(px((MARKER - LINE) / 2.))
                            .top(px(MARKER + VERTICAL_CLEARANCE))
                            .bottom(px(VERTICAL_CLEARANCE))
                            .w(px(LINE))
                            .bg(p.line)
                            .child(div().w_full().h(relative(filled)).bg(p.ink)),
                    );
                }
                rail = rail.child(item);
            } else {
                rail = rail.child(marker);
                if has_next {
                    rail = rail.child(
                        div()
                            .debug_selector(move || format!("step-rail-{i}"))
                            .mx(px(HORIZONTAL_CLEARANCE))
                            .flex_1()
                            .h(px(LINE))
                            .bg(p.line)
                            .child(div().h_full().w(relative(filled)).bg(p.ink)),
                    );
                }
            }
        }
        let caption = !sidebar || active.content.is_none();
        let body = column()
            .debug_selector(|| "step-body".into())
            .gap_1()
            .when(caption, |e| e.child(title(active.label.clone(), p)))
            .when(caption || self.error.is_some(), |e| {
                e.child(
                    div()
                        .text_color(if self.error.is_some() {
                            p.danger
                        } else {
                            p.secondary
                        })
                        .text_size(px(13.))
                        .child(self.message.render(13., w, cx)),
                )
            })
            .when_some(active.content.clone(), |e, body| {
                e.child(div().w_full().when(caption, |v| v.mt_3()).child(body))
            });
        let actions = row()
            .debug_selector(|| "step-actions".into())
            .justify_between()
            .gap_2()
            .border_t_1()
            .border_color(p.line)
            .pt_4()
            .when(self.current > 0, |el| {
                el.child(
                    command("step-back", tr(cx, "Back", "上一步"), cx, |v, _, cx| {
                        v.select(v.current - 1, cx)
                    })
                    .h(px(44.))
                    .px_4(),
                )
            })
            .child(
                toned_command(
                    "step-next",
                    if self.current + 1 == self.steps.len() {
                        tr(cx, "Finish", "完成")
                    } else {
                        tr(cx, "Continue", "继续")
                    },
                    Tone::Primary,
                    cx,
                    |v, w, cx| {
                        if let Some(error) = v.steps[v.current]
                            .validator
                            .as_ref()
                            .and_then(|test| test(w, cx))
                        {
                            v.error = Some(v.current);
                            v.message.set(error, 1., cx);
                            cx.notify();
                            return;
                        }
                        if v.current + 1 < v.steps.len() {
                            v.select(v.current + 1, cx)
                        } else {
                            cx.emit(StepEvent::Finished)
                        }
                    },
                )
                .h(px(44.))
                .px_4(),
            );
        if sidebar {
            el.child(
                row()
                    .w_full()
                    .items_stretch()
                    .gap(px(20.))
                    .child(rail)
                    .child(
                        column()
                            .flex_1()
                            .min_w_0()
                            .justify_between()
                            .gap_4()
                            .child(body)
                            .child(actions),
                    ),
            )
        } else {
            el.child(rail).child(body).child(actions)
        }
    }
}
