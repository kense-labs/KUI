//! A reaction bar with rolling counts and a keyboard-operated picker.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Slide, Tone, ease, face, measure},
        glyph::{Glyph, icon, transformed},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionOption {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Glyph,
}
impl ReactionOption {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, icon: Glyph) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reaction {
    pub id: SharedString,
    pub count: u32,
    pub reacted: bool,
}
impl Reaction {
    pub fn new(id: impl Into<SharedString>, count: u32, reacted: bool) -> Self {
        Self {
            id: id.into(),
            count,
            reacted,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionChanged(pub Reaction);
struct Pill {
    count: u32,
    old: u32,
    mine: bool,
    clock: PhaseClock,
    show: Slide,
    kick: Slide,
    focus: FocusHandle,
}
pub struct Reactions {
    id: ElementId,
    options: Vec<ReactionOption>,
    pills: Vec<Pill>,
    order: Vec<usize>,
    open: bool,
    active: usize,
    add: FocusHandle,
    picker: FocusHandle,
    origin: Rc<Cell<Bounds<Pixels>>>,
    slide: Slide,
    high: Slide,
}
impl EventEmitter<ReactionChanged> for Reactions {}
impl Reactions {
    pub fn new(
        id: impl Into<ElementId>,
        options: Vec<ReactionOption>,
        initial: Vec<Reaction>,
        cx: &App,
    ) -> Self {
        let mut pills: Vec<_> = options
            .iter()
            .map(|_| Pill {
                count: 0,
                old: 0,
                mine: false,
                clock: Default::default(),
                show: Slide::snappy(0.),
                kick: Slide::snappy(1.),
                focus: cx.focus_handle().tab_stop(true),
            })
            .collect();
        let mut order = vec![];
        for r in initial {
            if let Some(i) = options.iter().position(|o| o.id == r.id)
                && r.count > 0
                && !order.contains(&i)
            {
                pills[i].count = r.count;
                pills[i].mine = r.reacted;
                pills[i].old = r.count;
                pills[i].show.reset(1.);
                order.push(i)
            }
        }
        Self {
            id: id.into(),
            options,
            pills,
            order,
            open: false,
            active: 0,
            add: cx.focus_handle().tab_stop(true),
            picker: cx.focus_handle(),
            origin: Default::default(),
            slide: Slide::snappy(0.),
            high: Slide::snappy(0.),
        }
    }
    pub fn values(&self) -> Vec<Reaction> {
        self.order
            .iter()
            .filter(|&&i| self.pills[i].count > 0)
            .map(|&i| {
                Reaction::new(
                    self.options[i].id.clone(),
                    self.pills[i].count,
                    self.pills[i].mine,
                )
            })
            .collect()
    }
    fn toggle(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        let p = &mut self.pills[i];
        p.old = p.count;
        if p.mine {
            p.count = p.count.saturating_sub(1)
        } else {
            p.count = p.count.saturating_add(1)
        }
        p.mine = !p.mine;
        p.clock.restart(cx);
        let from = p.kick.value(cx);
        p.kick.reset((from + 0.28).min(1.7));
        p.kick.set(1., cx);
        p.show.set(if p.count > 0 { 1. } else { 0. }, cx);
        if !self.order.contains(&i) {
            self.order.push(i)
        }
        if p.count == 0 && p.focus.is_focused(w) {
            self.add.focus(w, cx)
        }
        cx.emit(ReactionChanged(Reaction::new(
            self.options[i].id.clone(),
            p.count,
            p.mine,
        )));
        cx.notify()
    }
    fn popup(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.open = open && !self.options.is_empty();
        self.slide.set(if self.open { 1. } else { 0. }, cx);
        if self.open {
            self.active = 0;
            self.high.reset(0.);
            self.picker.focus(w, cx)
        } else {
            self.add.focus(w, cx)
        }
        cx.notify()
    }
}
impl Render for Reactions {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = Theme::global(cx).tokens.radius.clone();
        let l = UiLanguage::current(cx);
        self.order
            .retain(|&i| self.pills[i].count > 0 || self.pills[i].show.get(w, cx) > 0.008);
        let mut bar = div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(l.text("Reactions", "回应"))
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_y(px(6.))
            .max_w_full();
        for &i in &self.order {
            let state = &self.pills[i];
            let opt = &self.options[i];
            let t = ease(state.clock.time(0.2, w, cx) / 0.2);
            let show = state.show.get(w, cx);
            let ink = if state.mine { p.ink } else { p.secondary };
            let count = state.count.to_string();
            let old = state.old.to_string();
            let digits = measure(&count.clone().into(), 13., w).max(8.);
            let width = (44. + digits) * show;
            let direction = if state.count >= state.old { 1. } else { -1. };
            let name = format!(
                "{} · {}{}",
                opt.label,
                state.count,
                if state.mine {
                    l.text(", including yours", "，含你的回应")
                } else {
                    ""
                }
            );
            let slot = div()
                .relative()
                .w(px(digits + 1.))
                .h(px(18.))
                .overflow_hidden()
                .text_size(px(13.))
                .line_height(px(18.))
                .when(t < 1., |e| {
                    e.child(
                        div()
                            .absolute()
                            .top(px(-direction * t * 18.))
                            .opacity(1. - t)
                            .child(old),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .top(px(direction * (1. - t) * 18.))
                        .opacity(t)
                        .child(count),
                );
            let b = face(("pill", i), name, 32., r.md.into(), Tone::Surface, cx)
                .track_focus(&state.focus)
                .gap(px(5.))
                .w(px(width))
                .mr(px(6. * show))
                .overflow_hidden()
                .opacity(show)
                .border_1()
                .border_color(if state.mine { p.faint } else { p.line })
                .bg(if state.mine { p.hover } else { p.panel })
                .aria_toggled(state.mine.into())
                .hover(move |s| s.bg(p.hover))
                .child(transformed(
                    opt.icon,
                    16.,
                    ink,
                    0.,
                    state.kick.get(w, cx),
                    (0., 0.),
                    1.,
                ))
                .child(slot)
                .on_click(cx.listener(move |v, _, w, cx| v.toggle(i, w, cx)));
            bar = bar.child(b)
        }
        let add = l.text("Add reaction", "添加回应");
        let origin = self.origin.clone();
        let b = face("add", add, 32., r.md.into(), Tone::Surface, cx)
            .w(px(38.))
            .border_1()
            .border_color(p.line)
            .track_focus(&self.add)
            .aria_expanded(self.open)
            .hover(move |s| s.bg(p.hover))
            .child(icon(
                if self.open {
                    Glyph::Close
                } else {
                    Glyph::SmilePlus
                },
                16.,
                p.secondary,
            ))
            .on_click(cx.listener(|v, _, w, cx| v.popup(!v.open, w, cx)));
        bar = bar.child(
            div()
                .on_children_prepainted(move |b, _, _| {
                    if let Some(b) = b.first() {
                        origin.set(*b)
                    }
                })
                .child(b),
        );
        let t = self.slide.get(w, cx);
        if self.open || t > 0.015 {
            let n = self.options.len();
            let gap = 2.;
            let gaps = gap * n.saturating_sub(1) as f32;
            let side = ((f32::from(w.viewport_size().width) - 32. - gaps) / n.max(1) as f32)
                .clamp(16., 40.);
            let width = side * n as f32 + gaps + 12.;
            let a = self.origin.get();
            let active = self.active;
            let opts = self.options.clone();
            let origin = self.origin.clone();
            let mut tray = div()
                .id("picker")
                .role(Role::Toolbar)
                .aria_label(add)
                .occlude()
                .relative()
                .flex()
                .items_center()
                .gap(px(gap))
                .p(px(5.))
                .child(
                    div()
                        .absolute()
                        .left(px(5. + self.high.get(w, cx) * (side + gap)))
                        .top(px(5.))
                        .w(px(side))
                        .h(px(40.))
                        .bg(p.hover)
                        .rounded(r.sm),
                );
            for (i, o) in opts.iter().enumerate() {
                let label = o.label.clone();
                tray = tray.child(
                    div()
                        .id(("pick", i))
                        .debug_selector(move || label.to_string())
                        .aria_label(o.label.clone())
                        .aria_toggled(self.pills[i].mine.into())
                        .role(Role::Button)
                        .when(i == active, |e| e.aria_active_descendant())
                        .w(px(side))
                        .h(px(40.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .when(self.pills[i].mine, |e| {
                            e.rounded(r.sm)
                                .bg(p.ink.opacity(0.10))
                                .border_1()
                                .border_color(p.ink.opacity(0.45))
                        })
                        .child(icon(
                            o.icon,
                            20.,
                            if self.pills[i].mine || i == active {
                                p.ink
                            } else {
                                p.secondary
                            },
                        ))
                        .on_hover(cx.listener(move |v, on, _, cx| {
                            if *on && v.open && v.active != i {
                                v.active = i;
                                v.high.set(i as f32, cx);
                                cx.notify()
                            }
                        }))
                        .on_click(cx.listener(move |v, _, w, cx| {
                            if v.open {
                                v.toggle(i, w, cx);
                                v.popup(false, w, cx)
                            }
                        })),
                );
            }
            let popup = div()
                .id("picker-focus")
                .track_focus(&self.picker)
                .tab_stop(false)
                .flex()
                .flex_col()
                .items_center()
                .w(px(width))
                .opacity(t)
                .rounded(r.md)
                .bg(p.panel)
                .shadow_lg()
                .occlude()
                .border_1()
                .border_color(p.line)
                .child(tray)
                .child(
                    div()
                        .pb(px(5.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(11.))
                        .text_color(p.secondary)
                        .when(self.pills[active].mine, |e| {
                            e.child(icon(Glyph::Check, 12., p.secondary))
                        })
                        .child(opts[active].label.clone()),
                )
                .on_mouse_down_out(cx.listener(move |v, e: &MouseDownEvent, w, cx| {
                    if v.open && !origin.get().contains(&e.position) {
                        v.popup(false, w, cx)
                    }
                }))
                .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
                    if !v.open {
                        return;
                    }
                    let k = e.keystroke.key.as_str();
                    match k {
                        "escape" | "tab" => v.popup(false, w, cx),
                        "enter" | "space" => {
                            v.toggle(v.active, w, cx);
                            v.popup(false, w, cx)
                        }
                        "left" | "up" => v.active = (v.active + n - 1) % n,
                        "right" | "down" => v.active = (v.active + 1) % n,
                        "home" => v.active = 0,
                        "end" => v.active = n - 1,
                        _ => return,
                    }
                    v.high.set(v.active as f32, cx);
                    cx.stop_propagation();
                    w.prevent_default();
                    cx.notify()
                }));
            let x = (f32::from(a.center().x) - width / 2.).clamp(
                6.,
                (f32::from(w.viewport_size().width) - width - 6.).max(6.),
            );
            let y = if a.top() > px(85.) {
                a.top() - px(76.)
            } else {
                a.bottom() + px(8.)
            };
            bar = bar.child(
                deferred(
                    anchored()
                        .position(point(px(x), y + px((1. - t) * 5.)))
                        .child(popup),
                )
                .with_priority(3),
            );
        }
        bar
    }
}
