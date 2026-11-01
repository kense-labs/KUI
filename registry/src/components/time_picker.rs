//! Fixed-step local wall-clock selection. Keyboard previews are committed explicitly.
use crate::primitives::{
    editor::EditorScrollRegion,
    family::{Paint, Tone, face},
    form_chrome::*,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WallTime(u16);
impl WallTime {
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour < 24 && minute < 60).then(|| Self(u16::from(hour) * 60 + u16::from(minute)))
    }
    pub fn hour(self) -> u8 {
        (self.0 / 60) as u8
    }
    pub fn minute(self) -> u8 {
        (self.0 % 60) as u8
    }
    pub fn display(self, full: bool) -> String {
        let h = self.hour();
        if full {
            format!("{:02}:{:02}", h, self.minute())
        } else {
            format!(
                "{}:{:02} {}",
                if h.is_multiple_of(12) { 12 } else { h % 12 },
                self.minute(),
                if h < 12 { "AM" } else { "PM" }
            )
        }
    }
}
pub struct TimePicker {
    label: SharedString,
    help: SharedString,
    value: WallTime,
    step: u16,
    full: bool,
    open: bool,
    active: u16,
    reveal: FormReveal,
    rolling: RollingValue,
    direction: f32,
    focus: FocusHandle,
    list: FocusHandle,
    scroll: UniformListScrollHandle,
}
impl EventEmitter<WallTime> for TimePicker {}
impl TimePicker {
    pub fn new(label: impl Into<SharedString>, value: WallTime, cx: &mut Context<Self>) -> Self {
        Self {
            label: label.into(),
            help: "".into(),
            value,
            step: 15,
            full: false,
            open: false,
            active: value.0,
            reveal: FormReveal::new(false),
            rolling: Default::default(),
            direction: 1.,
            focus: cx.focus_handle().tab_stop(true),
            list: cx.focus_handle().tab_stop(true),
            scroll: UniformListScrollHandle::new(),
        }
    }
    pub fn with_step(mut self, step: u16) -> Self {
        if matches!(step, 1 | 5 | 10 | 15 | 30) {
            self.step = step
        }
        self
    }
    pub fn with_24h(mut self, yes: bool) -> Self {
        self.full = yes;
        self
    }
    pub fn with_description(mut self, t: impl Into<SharedString>) -> Self {
        self.help = t.into();
        self
    }
    pub fn value(&self) -> WallTime {
        self.value
    }
    fn scroll(&self, strategy: ScrollStrategy) {
        self.scroll
            .scroll_to_item((self.active / self.step) as usize, strategy)
    }
    fn show(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.active = self.value.0 / self.step * self.step;
        self.scroll
            .scroll_to_item_strict((self.active / self.step) as usize, ScrollStrategy::Center);
        self.list.focus(w, cx);
        cx.notify()
    }
    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if let Some(host) = cx.try_global::<EditorScrollRegion>() {
            (host.0)(Bounds::default(), false, false);
        }
        self.focus.focus(w, cx);
        cx.notify()
    }
    fn choose(&mut self, t: WallTime, w: &mut Window, cx: &mut Context<Self>) {
        if self.value != t {
            self.direction = if t.0 > self.value.0 { 1. } else { -1. };
            self.value = t;
            cx.emit(t)
        }
        self.close(w, cx)
    }
}
impl Render for TimePicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let progress = self.reveal.sample(self.open, 0.26, 0.14, w, cx);
        let trigger = field("time", self.label.clone(), cx)
            .relative()
            .child(self.reveal.anchor())
            .track_focus(&self.focus)
            .aria_expanded(self.open)
            .debug_selector(|| "time-trigger".into())
            .gap(px(9.))
            .child(
                div()
                    .flex_none()
                    .size(px(16.))
                    .child(icon(Glyph::Clock, 16., p.faint)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .child(self.rolling.label(
                        self.value.display(self.full),
                        self.direction,
                        w,
                        cx,
                    )),
            )
            .child(crate::primitives::glyph::transformed(
                Glyph::Down,
                16.,
                p.faint,
                std::f32::consts::PI * progress,
                1.,
                (0., 0.),
                1.,
            ))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if matches!(e.keystroke.key.as_str(), "up" | "down" | "space" | "enter") && !s.open
                {
                    s.show(w, cx);
                    w.prevent_default();
                    cx.stop_propagation()
                }
            }))
            .on_click(cx.listener(|s, _, w, cx| {
                if s.open {
                    s.close(w, cx)
                } else {
                    s.show(w, cx)
                }
            }));
        let scroll = self.scroll.0.borrow().base_handle.clone();
        let open = self.open;
        section(340., cx)
            .id("time-field")
            .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, w, cx| {
                if s.open && !s.reveal.contains(e.position) {
                    s.close(w, cx)
                }
            }))
            .child(self.label.clone())
            .child(trigger)
            .when(self.open || progress > 0., |d| {
                d.child(
                    self.reveal.popup(
                        panel(cx)
                            .p(px(5.))
                            .gap_0()
                            .shadow_lg()
                            .occlude()
                            .on_children_prepainted(move |bounds, _, cx| {
                                if open
                                    && let (Some(bounds), Some(host)) =
                                        (bounds.first(), cx.try_global::<EditorScrollRegion>())
                                {
                                    let y = -scroll.offset().y;
                                    (host.0)(*bounds, y > px(0.), y < scroll.max_offset().y)
                                }
                            })
                            .id("times")
                            .debug_selector(|| "time-menu".into())
                            .role(Role::ListBox)
                            .track_focus(&self.list)
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                                match e.keystroke.key.as_str() {
                                    "escape" => s.close(w, cx),
                                    "enter" | "space" => s.choose(WallTime(s.active), w, cx),
                                    "up" => s.active = (s.active + 1440 - s.step) % 1440,
                                    "down" => s.active = (s.active + s.step) % 1440,
                                    "home" => s.active = 0,
                                    "end" => s.active = 1440 - s.step,
                                    _ => return,
                                }
                                if s.open {
                                    s.scroll(ScrollStrategy::Nearest);
                                }
                                cx.stop_propagation();
                                w.prevent_default();
                                cx.notify()
                            }))
                            .child(
                                uniform_list(
                                    "slots",
                                    (1440 / self.step) as usize,
                                    cx.processor(
                                        move |s, indices: std::ops::Range<usize>, _, cx| {
                                            indices
                                                .map(|i| {
                                                    let t = WallTime(i as u16 * s.step);
                                                    let label = t.display(s.full);
                                                    face(
                                                        ("slot", i),
                                                        label.clone(),
                                                        36.,
                                                        crate::foundation::surface_radius(cx)
                                                            .into(),
                                                        Tone::Clear,
                                                        cx,
                                                    )
                                                    .tab_stop(false)
                                                    .role(Role::ListBoxOption)
                                                    .aria_selected(t == s.value)
                                                    .w_full()
                                                    .px(px(12.))
                                                    .justify_between()
                                                    .font_weight(FontWeight::NORMAL)
                                                    .bg(if t.0 == s.active {
                                                        p.stage
                                                    } else {
                                                        transparent_black()
                                                    })
                                                    .child(label)
                                                    .when(t == s.value, |r| {
                                                        r.child(
                                                            div()
                                                                .debug_selector(|| {
                                                                    "time-selected-dot".into()
                                                                })
                                                                .size(px(6.))
                                                                .rounded_full()
                                                                .bg(p.ink),
                                                        )
                                                    })
                                                    .on_hover(cx.listener(
                                                        move |s, over: &bool, _, cx| {
                                                            if *over && s.open && s.active != t.0 {
                                                                s.active = t.0;
                                                                cx.notify()
                                                            }
                                                        },
                                                    ))
                                                    .on_click(cx.listener(move |s, _, w, cx| {
                                                        s.choose(t, w, cx)
                                                    }))
                                                })
                                                .collect()
                                        },
                                    ),
                                )
                                .track_scroll(&self.scroll)
                                .h(px(238.))
                                .w_full(),
                            ),
                        progress,
                        self.open,
                        (0., 250., 6.),
                        w,
                    ),
                )
            })
            .when(!self.help.is_empty(), |d| {
                d.child(hint(self.help.clone(), cx).opacity(1. - progress))
            })
    }
}
