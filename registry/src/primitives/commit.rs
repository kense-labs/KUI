//! Ticketed asynchronous activation and acknowledgement.
use crate::foundation::Theme;
use crate::primitives::{
    control::activate,
    family::{
        Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face, lettering, measure, press,
    },
    glyph::{Glyph, spinner, transformed},
};
use gpui::{prelude::*, *};
use std::time::Duration;
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Commit {
    pub ticket: u64,
    pub next: bool,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Pending,
    Done,
    Failed,
}
pub struct ActionCore {
    pub id: ElementId,
    pub labels: [SharedString; 4],
    pub marks: [Glyph; 4],
    pub tone: Tone,
    pub widths: [f32; 4],
    width: Slide,
    sized: bool,
    manual: bool,
    previous: Phase,
    painted: Phase,
    clock: PhaseClock,
    pressed: Slide,
    pub momentary: bool,
    started: Option<web_time::Instant>,
    pub height: f32,
    pub trailing: bool,
    pub reset: Option<Duration>,
    pub phase: Phase,
    pub selected: bool,
    ticket: u64,
    pub done_tone: Tone,
    last: Glyph,
    current: Glyph,
}
impl ActionCore {
    pub fn new(id: ElementId, labels: [SharedString; 4], marks: [Glyph; 4]) -> Self {
        Self {
            id,
            labels,
            marks,
            tone: Tone::Primary,
            widths: [159., 112., 108., 159.],
            width: Slide::new(159.),
            sized: false,
            manual: false,
            previous: Phase::Idle,
            painted: Phase::Idle,
            clock: Default::default(),
            pressed: Slide::snappy(0.),
            momentary: true,
            started: None,
            height: 44.,
            trailing: true,
            reset: Some(Duration::from_millis(2400)),
            phase: Phase::Idle,
            selected: false,
            ticket: 0,
            done_tone: Tone::Success,
            last: marks[0],
            current: marks[0],
        }
    }
    pub fn set_widths(&mut self, widths: [f32; 4]) {
        self.manual = true;
        self.widths = widths;
        self.width = Slide::new(widths[0]);
    }
    pub fn set_marks(&mut self, marks: [Glyph; 4]) {
        self.marks = marks;
        self.last = marks[0];
        self.current = marks[0]
    }
    fn stage(&mut self, p: Phase, cx: &App) {
        self.previous = self.painted;
        self.phase = p;
        self.clock.restart(cx);
        self.started = Some(cx.background_executor().now());
        self.last = self.marks[self.painted as usize];
        self.current = self.marks[p as usize];
    }
}
pub trait CommitControl: Render + EventEmitter<Commit> + Sized {
    fn core(&mut self) -> &mut ActionCore;
}
/// Host completed a local action synchronously; it has no working interval.
pub fn finish_inline<T: CommitControl>(v: &mut T, ticket: u64, ok: bool, cx: &mut Context<T>) {
    settle(v, ticket, ok, false, cx)
}
pub fn finish<T: CommitControl>(v: &mut T, ticket: u64, ok: bool, cx: &mut Context<T>) {
    settle(v, ticket, ok, true, cx)
}
fn settle<T: CommitControl>(
    v: &mut T,
    ticket: u64,
    ok: bool,
    asynchronous: bool,
    cx: &mut Context<T>,
) {
    let c = v.core();
    if c.ticket != ticket || c.phase != Phase::Pending {
        return;
    }
    if !asynchronous {
        c.painted = c.previous;
    }
    if asynchronous
        && !c.trailing
        && (c.labels[1].is_empty() || c.painted == Phase::Pending)
        && let Some(start) = c.started
    {
        let min = Duration::from_millis(450);
        let age = cx
            .background_executor()
            .now()
            .saturating_duration_since(start);
        if age < min {
            cx.spawn(async move |e, cx| {
                cx.background_executor().timer(min - age).await;
                let _ = e.update(cx, |v, cx| finish(v, ticket, ok, cx));
            })
            .detach();
            return;
        }
    }
    if ok {
        c.selected = c.momentary || !c.selected
    }
    c.stage(
        if !ok && c.momentary && c.trailing {
            Phase::Idle
        } else if !ok {
            Phase::Failed
        } else if c.selected {
            Phase::Done
        } else {
            Phase::Idle
        },
        cx,
    );
    if let Some(delay) = c.reset.or(if !ok {
        Some(Duration::from_millis(1800))
    } else {
        None
    }) {
        cx.spawn(async move |v, cx| {
            cx.background_executor().timer(delay).await;
            v.update(cx, |v, cx| {
                let c = v.core();
                if c.ticket == ticket {
                    if c.reset.is_some() {
                        c.selected = false
                    }
                    c.stage(if c.selected { Phase::Done } else { Phase::Idle }, cx);
                    cx.notify()
                }
            })
            .ok();
        })
        .detach();
    }
    cx.notify()
}
pub fn render_action<T: CommitControl>(
    v: &mut T,
    w: &mut Window,
    cx: &mut Context<T>,
) -> impl IntoElement {
    let c = v.core();
    let p = Paint::get(cx);
    let radius = Theme::global(cx).tokens.radius.md;
    let phase = c.phase;
    c.painted = phase;
    let label = c.labels[phase as usize].clone();
    let old = c.labels[c.previous as usize].clone();
    let time = c.clock.time(0.48, w, cx);
    let intrinsic = |text: &SharedString| {
        if text.is_empty() {
            c.height
        } else {
            measure(text, 14., w) + if c.trailing { 66. } else { 58. }
        }
    };
    let target = if c.manual {
        c.widths[phase as usize]
    } else {
        intrinsic(&label)
    };
    if !c.sized {
        c.width.reset(target);
        c.sized = true
    } else {
        c.width
            .duration(target, if c.trailing { 0.24 } else { 0.42 }, cx)
    }
    let wide = c.width.get(w, cx);
    let k = 1. - 0.03 * c.pressed.get(w, cx);
    let tones = |phase| {
        if !c.trailing && phase == Phase::Done {
            c.done_tone
        } else if !c.trailing && phase == Phase::Failed {
            Tone::Danger
        } else {
            c.tone
        }
    };
    let tone = tones(phase);
    let (bg, fg) = p.colors(tone);
    let (prev_bg, prev_fg) = p.colors(tones(c.previous));
    let mix = |a: Hsla, b: Hsla| {
        let f = ease(time / 0.24);
        let mut col = b;
        col.l = a.l + (b.l - a.l) * f;
        col.a = a.a + (b.a - a.a) * f;
        col
    };
    let fg = mix(prev_fg, fg);
    let glyph_ink = if tone == Tone::Success {
        p.success
    } else if tone == Tone::Danger {
        p.danger
    } else {
        fg
    };
    let h = c.height;
    let name = if label.is_empty() {
        c.labels[0].clone()
    } else {
        label.clone()
    };
    let mut icon_slot = div().relative().size(px(16. * k)).flex_none();
    if time < 0.16 && c.last != c.current {
        icon_slot = icon_slot.child(div().absolute().inset_0().child(transformed(
            c.last,
            16. * k,
            fg.opacity(1. - ease(time / 0.16)),
            0.,
            1.,
            (
                if c.last == Glyph::Arrow {
                    8. * ease(time / 0.16)
                } else {
                    0.
                },
                0.,
            ),
            1.,
        )))
    }
    let glyph = if phase == Phase::Pending {
        let turn = if disabled_motion(cx) {
            0.
        } else {
            w.request_animation_frame();
            c.clock.age(cx) / 0.72
        };
        spinner(turn, 16. * k, fg.opacity(ease(time / 0.16))).into_any_element()
    } else {
        transformed(
            c.current,
            16. * k,
            glyph_ink.opacity(ease(time / 0.16)),
            0.,
            0.8 + 0.2 * ease(time / 0.22),
            (0., 0.),
            if c.current == Glyph::Check {
                ease((time - 0.05) / 0.24)
            } else {
                1.
            },
        )
        .into_any_element()
    };
    icon_slot = icon_slot.child(glyph);
    let tw = ((wide - 24. - if c.trailing { 42. } else { 34. }) * k).max(0.);
    let text = lettering(old, label.clone(), time, !c.trailing, 14. * k, fg, tw);
    let mut inner = div()
        .w(px(wide * k))
        .h(px(h * k))
        .flex_none()
        .rounded(radius * k)
        .flex()
        .gap(px(8. * k))
        .items_center()
        .justify_center()
        .overflow_hidden()
        .bg(mix(prev_bg, bg))
        .when(tone == Tone::Surface, |e| e.border_1().border_color(p.line))
        .when(tone == Tone::Success || tone == Tone::Danger, |e| {
            e.border_1().border_color(if tone == Tone::Success {
                p.success.opacity(0.4)
            } else {
                p.danger.opacity(0.42)
            })
        });
    inner = if label.is_empty() {
        inner.child(icon_slot)
    } else if c.trailing {
        inner.child(text).child(icon_slot)
    } else {
        inner.child(icon_slot).child(text)
    };
    let el = face(c.id.clone(), name, h, radius.into(), Tone::Clear, cx)
        .w(px(wide))
        .child(inner);
    let el = press(el, cx, |v| &mut v.core().pressed);
    activate(el, cx, |v, _, cx| {
        let c = v.core();
        if c.phase == Phase::Pending {
            return;
        }
        c.ticket = c.ticket.wrapping_add(1);
        let event = Commit {
            ticket: c.ticket,
            next: c.momentary || !c.selected,
        };
        c.stage(Phase::Pending, cx);
        cx.emit(event);
        cx.notify();
    })
}
