//! A roving radio rating with half steps, pointer scrubbing and layered star motion.
use crate::{
    foundation::Theme,
    primitives::{
        Size,
        color_motion::ColorFade,
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion, ease, standard_ease},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::Instant;

/// A committed rating; zero means cleared. Host updates do not emit this event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RatingChanged(pub f32);
#[derive(Clone, Copy)]
struct Gesture {
    last: f32,
    scrub: bool,
    touch: bool,
}
struct Pop {
    spring: Slide,
    at: Option<Instant>,
}
impl Pop {
    fn new() -> Self {
        Self {
            spring: Slide::snappy(1.),
            at: None,
        }
    }
    fn arm(&mut self, delay: f32, cx: &App) {
        let current = self.spring.value(cx);
        self.spring.reset(current);
        self.at = Some(cx.background_executor().now() + Duration::from_secs_f32(delay));
    }
    fn get(&mut self, w: &Window, cx: &App) -> f32 {
        if disabled_motion(cx) {
            self.at = None;
            self.spring.reset(1.);
            return 1.;
        }
        if let Some(at) = self.at {
            if cx.background_executor().now() >= at {
                self.at = None;
                self.spring.release(1., 9., 0.26, cx);
            } else {
                w.request_animation_frame();
            }
        }
        self.spring.get(w, cx)
    }
}
#[derive(Clone, Copy)]
struct StarInk {
    fill: ColorFade,
    stroke: ColorFade,
}
/// Frame timing and warning ink shared by every star's fill and warning layers.
#[derive(Clone, Copy)]
struct StarPhase {
    warning: Hsla,
    now: Instant,
    reduced: bool,
}
/// Rating state shared by native and Web; `new` takes an initial value.
pub struct Rating {
    label: SharedString,
    description: SharedString,
    empty_label: Option<SharedString>,
    value_labels: Vec<SharedString>,
    name: Option<SharedString>,
    max: usize,
    value: f32,
    preview: Option<f32>,
    held_at: Option<f32>,
    half: bool,
    allow_clear: bool,
    show_value: bool,
    read_only: bool,
    disabled: bool,
    controlled: bool,
    size: Size,
    focus: Vec<FocusHandle>,
    touch: Rc<Cell<u8>>,
    gesture: Option<Gesture>,
    pressed: Option<usize>,
    suppress_until: Option<Instant>,
    pop: Vec<Pop>,
    lift: Vec<Slide>,
    bounds: Vec<Rc<Cell<Bounds<Pixels>>>>,
    fill: Vec<Slide>,
    kept: Vec<Slide>,
    inks: Vec<Option<StarInk>>,
    initialized: bool,
    sweep_from: f32,
    sweep_to: f32,
    last_commit: Option<Instant>,
    pulse: Option<(f32, bool)>,
    word_current: Option<SharedString>,
    word_prev: SharedString,
    word_dir: f32,
    word_clock: PhaseClock,
}
impl EventEmitter<RatingChanged> for Rating {}
impl Rating {
    pub fn new(
        label: impl Into<SharedString>,
        max: usize,
        value: f32,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = if value.is_finite() {
            value.clamp(0., max as f32)
        } else {
            0.
        };
        Self {
            label: label.into(),
            description: "".into(),
            empty_label: None,
            value_labels: Vec::new(),
            name: None,
            max,
            value,
            preview: None,
            held_at: None,
            half: false,
            allow_clear: true,
            show_value: true,
            read_only: false,
            disabled: false,
            controlled: false,
            size: Size::Md,
            focus: (0..max * 2).map(|_| cx.focus_handle()).collect(),
            touch: Default::default(),
            gesture: None,
            pressed: None,
            suppress_until: None,
            pop: (0..max).map(|_| Pop::new()).collect(),
            lift: vec![Slide::snappy(1.); max],
            bounds: (0..max).map(|_| Rc::default()).collect(),
            fill: vec![Slide::snappy(0.); max],
            kept: vec![Slide::new(0.); max],
            inks: vec![None; max],
            initialized: false,
            sweep_from: value,
            sweep_to: value,
            last_commit: None,
            pulse: None,
            word_current: None,
            word_prev: "".into(),
            word_dir: 1.,
            word_clock: Default::default(),
        }
    }
    pub fn with_half(mut self, half: bool) -> Self {
        self.half = half;
        self
    }
    pub fn with_allow_clear(mut self, value: bool) -> Self {
        self.allow_clear = value;
        self
    }
    pub fn with_show_value(mut self, value: bool) -> Self {
        self.show_value = value;
        self
    }
    pub fn with_description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = text.into();
        self
    }
    pub fn with_empty_label(mut self, text: impl Into<SharedString>) -> Self {
        self.empty_label = Some(text.into());
        self
    }
    pub fn with_value_labels(mut self, labels: Vec<SharedString>) -> Self {
        self.value_labels = labels;
        self
    }
    pub fn with_read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
    pub fn with_disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
    /// Keep the visible value host-controlled; emit intent and wait for `set_value`.
    pub fn with_controlled(mut self, value: bool) -> Self {
        self.controlled = value;
        self
    }
    /// Supply a form field name. The host serializes `form_value` when submitting.
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn form_value(&self) -> Option<(SharedString, f32)> {
        self.name.clone().map(|name| (name, self.value()))
    }
    pub fn value(&self) -> f32 {
        self.normalize(self.value)
    }
    fn normalize(&self, value: f32) -> f32 {
        if !value.is_finite() {
            return 0.;
        }
        let step = if self.half { 2. } else { 1. };
        ((value * step).round() / step).clamp(0., self.max as f32)
    }
    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        if value.is_finite() {
            self.value = self.normalize(value);
            cx.notify();
        }
    }
    fn commit(&mut self, next: f32, toggle: bool, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        let step = if self.half { 0.5 } else { 1. };
        let current = self.value();
        let mut chosen = self.normalize(next);
        if toggle && self.allow_clear && chosen == current {
            chosen = 0.;
        }
        if !self.allow_clear {
            chosen = chosen.max(step).min(self.max as f32);
        }
        if let Some(preview) = self.preview.take() {
            self.held_at = Some(preview);
        }
        let now = cx.background_executor().now();
        let wave = self
            .last_commit
            .is_none_or(|last| now.saturating_duration_since(last) > Duration::from_millis(260));
        self.last_commit = Some(now);
        self.pulse = Some((chosen, wave));
        if chosen != current {
            if !self.controlled {
                self.value = chosen;
            }
            cx.emit(RatingChanged(chosen));
        }
        cx.notify();
    }
    fn cell_index(&self, value: f32) -> Option<usize> {
        if self.max == 0 {
            return None;
        }
        Some((value.ceil().max(1.) as usize - 1).min(self.max - 1))
    }
    fn radio_index(&self, value: f32) -> usize {
        let step = if self.half { 0.5 } else { 1. };
        (self.normalize(value).max(step) / step).round() as usize - 1
    }
    fn focus_value(&self, value: f32, w: &mut Window, cx: &mut App) {
        if let Some(focus) = self.focus.get(self.radio_index(value)) {
            focus.focus(w, cx);
        }
    }
    fn pointer_value(&self, index: usize, x: Pixels) -> f32 {
        let b = self.bounds[index].get();
        if self.half && x < b.center().x {
            index as f32 + 0.5
        } else {
            index as f32 + 1.
        }
    }
    fn value_at(&self, x: Pixels) -> f32 {
        for (i, b) in self.bounds.iter().enumerate() {
            let b = b.get();
            if i == 0 && x < b.left() - b.size.width * 0.5 {
                return if self.allow_clear {
                    0.
                } else if self.half {
                    0.5
                } else {
                    1.
                };
            }
            if x <= b.right() + px(1.) || i + 1 == self.max {
                return self.pointer_value(i, x);
            }
        }
        self.value()
    }
    fn begin(&mut self, x: Pixels, touch: bool, cx: &mut Context<Self>) {
        let at = self.value_at(x);
        self.gesture = Some(Gesture {
            last: at,
            scrub: false,
            touch,
        });
        self.pressed = self.cell_index(at);
        if touch {
            self.preview = Some(at);
        }
        cx.notify();
    }
    fn move_pointer(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let at = self.value_at(x);
        let previous_pressed = self.pressed;
        let previous_preview = self.preview;
        if let Some(g) = &mut self.gesture {
            if at != g.last {
                g.scrub = true;
                g.last = at;
            }
            let preview = g.touch || g.scrub;
            self.pressed = self.cell_index(at);
            if preview {
                self.preview = Some(at);
            }
            if self.pressed != previous_pressed || self.preview != previous_preview {
                cx.notify();
            }
            return;
        }
        if self.held_at == Some(at) {
            return;
        }
        self.held_at = None;
        if self.preview != Some(at) {
            self.preview = Some(at);
            cx.notify();
        }
    }
    fn end(&mut self, x: Pixels, cancelled: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(g) = self.gesture.take() else {
            return;
        };
        self.pressed = None;
        if !cancelled && (g.touch || g.scrub) {
            let at = self.value_at(x);
            self.suppress_until = Some(cx.background_executor().now() + Duration::from_millis(500));
            self.commit(at, !g.scrub, cx);
            self.held_at = if g.touch { None } else { Some(at) };
            if g.scrub {
                self.focus_value(at, w, cx);
            }
        }
        if g.touch || cancelled {
            self.preview = None;
        }
        cx.notify();
    }
    fn key(&mut self, key: &str, own: f32, w: &mut Window, cx: &mut Context<Self>) {
        let step = if self.half { 0.5 } else { 1. };
        let current = self.value();
        let next = match key {
            "left" | "down" => current - step,
            "right" | "up" => current + step,
            "home" => step,
            "end" => self.max as f32,
            "enter" | "space" => own,
            "backspace" | "delete" if self.allow_clear => 0.,
            _ if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() && key != "0" => {
                let digit = (key.as_bytes()[0] - b'0') as f32;
                if digit > self.max as f32 {
                    return;
                }
                digit
            }
            _ => return,
        };
        let chosen = if next == 0. {
            0.
        } else {
            next.max(step).min(self.max as f32)
        };
        self.commit(chosen, false, cx);
        self.focus_value(chosen, w, cx);
        w.prevent_default();
        cx.stop_propagation();
    }
    fn word_for(&self, val: f32, lang: UiLanguage) -> Option<SharedString> {
        if val.fract() != 0. || val < 1. {
            return None;
        }
        let i = val as usize - 1;
        if !self.value_labels.is_empty() {
            return self.value_labels.get(i).cloned();
        }
        if self.max == 5 && !self.half {
            return [
                lang.text("Poor", "很差"),
                lang.text("Fair", "欠佳"),
                lang.text("Good", "一般"),
                lang.text("Great", "很好"),
                lang.text("Excellent", "非常棒"),
            ]
            .get(i)
            .map(|s| (*s).into());
        }
        None
    }
    fn text_for(&self, val: f32, lang: UiLanguage) -> SharedString {
        if val <= 0. {
            return self
                .empty_label
                .clone()
                .unwrap_or_else(|| lang.text("Not rated", "未评分").into());
        }
        if let Some(word) = self.word_for(val, lang) {
            return word;
        }
        self.numeric_text(val, lang)
    }
    fn numeric_text(&self, val: f32, lang: UiLanguage) -> SharedString {
        let number = if val.fract() == 0. {
            format!("{val:.0}")
        } else {
            format!("{val:.1}")
        };
        format!("{number} {} {}", lang.text("out of", "/"), self.max).into()
    }
    fn spoken(&self, val: f32, lang: UiLanguage) -> SharedString {
        let number = if val.fract() == 0. {
            format!("{val:.0}")
        } else {
            format!("{val:.1}")
        };
        let score = match lang {
            UiLanguage::English => format!("{number} of {} stars", self.max),
            UiLanguage::Chinese => format!("{number} / {} 星", self.max),
        };
        if let Some(word) = self.word_for(val, lang) {
            format!("{word}, {score}").into()
        } else {
            score.into()
        }
    }
}

// Star coordinate data: Copyright (c) 2026 Lucide Icons and Contributors.
// ISC License: Permission to use, copy, modify, and/or distribute this software
// for any purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
// REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
// AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
// INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
// LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
// OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
// PERFORMANCE OF THIS SOFTWARE.
fn star_path(bounds: Bounds<Pixels>, scale: f32, stroke: bool) -> Option<Path<Pixels>> {
    let unit = f32::from(bounds.size.width) / 24. * scale;
    let convert =
        |x: f32, y: f32| bounds.center() + point(px((x - 12.) * unit), px((y - 12.) * unit));
    let mut path = if stroke {
        PathBuilder::stroke(px(1.75 * unit))
    } else {
        PathBuilder::fill()
    };
    path.move_to(convert(11.525, 2.295));
    for (x, y, r, sweep) in [
        (12.475, 2.295, 0.53, true),
        (14.785, 6.974, 0., false),
        (16.38, 8.134, 2.123, false),
        (21.546, 8.89, 0., false),
        (21.84, 9.794, 0.53, true),
        (18.104, 13.432, 0., false),
        (17.493, 15.31, 2.123, false),
        (18.375, 20.45, 0., false),
        (17.604, 21.01, 0.53, true),
        (12.986, 18.582, 0., false),
        (11.013, 18.582, 2.122, false),
        (6.396, 21.01, 0., false),
        (5.626, 20.45, 0.53, true),
        (6.507, 15.311, 0., false),
        (5.896, 13.432, 2.122, false),
        (2.16, 9.795, 0., false),
        (2.454, 8.889, 0.53, true),
        (7.619, 8.134, 0., false),
        (9.216, 6.974, 2.122, false),
    ] {
        if r == 0. {
            path.line_to(convert(x, y));
        } else {
            let r = px(r * unit);
            path.arc_to(point(r, r), px(0.), false, sweep, convert(x, y));
        }
    }
    path.close();
    path.build().ok()
}
fn draw_star(w: &mut Window, b: Bounds<Pixels>, scale: f32, fill: Hsla, stroke: Hsla) {
    if fill.a > 0.
        && let Some(p) = star_path(b, scale, false)
    {
        w.paint_path(p, fill);
    }
    if stroke.a > 0.
        && let Some(p) = star_path(b, scale, true)
    {
        w.paint_path(p, stroke);
    }
}
fn star(
    side: f32,
    scale: f32,
    fill: f32,
    kept: f32,
    base: StarInk,
    phase: StarPhase,
) -> impl IntoElement {
    let StarPhase {
        warning,
        now,
        reduced,
    } = phase;
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let duration = if reduced { 0. } else { 0.24 };
            draw_star(
                w,
                b,
                scale,
                base.fill.value(now, duration).0,
                base.stroke.value(now, duration).0,
            );
            let mut mask = b;
            mask.origin =
                b.center() - point(b.size.width * scale * 0.5, b.size.height * scale * 0.5);
            mask.size.height *= scale;
            for (amount, color) in [(kept, warning.opacity(0.30)), (fill, warning)] {
                if amount > 0.001 {
                    mask.size.width = b.size.width * scale * amount.clamp(0., 1.);
                    w.with_content_mask(Some(ContentMask { bounds: mask }), |w| {
                        draw_star(w, b, scale, color, color)
                    });
                }
            }
        },
    )
    .size(px(side))
    .flex_none()
}
fn words(text: &str) -> Vec<SharedString> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut previous = None;
    for (at, ch) in text.char_indices() {
        let space = ch.is_whitespace();
        if previous.is_some_and(|p| p != space) {
            result.push(text[start..at].to_string().into());
            start = at;
        }
        previous = Some(space);
    }
    if start < text.len() {
        result.push(text[start..].to_string().into());
    }
    result
}
impl Render for Rating {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Theme::global(cx).tokens.colors.clone();
        let reduced = disabled_motion(cx);
        let lang = UiLanguage::current(cx);
        let now = cx.background_executor().now();
        let current = self.value();
        let target = self.preview.unwrap_or(current);
        let interactive = !self.disabled && !self.read_only;
        if !self.initialized {
            for i in 0..self.max {
                self.fill[i].reset((target - i as f32).clamp(0., 1.));
                self.kept[i].reset((current - i as f32).clamp(0., 1.));
            }
            self.sweep_from = target;
            self.sweep_to = target;
            self.initialized = true;
        }
        if self.sweep_to != target {
            self.sweep_from = self.sweep_to;
            self.sweep_to = target;
        }
        let direction = if self.sweep_to >= self.sweep_from {
            1.
        } else {
            -1.
        };
        if let Some((chosen, wave)) = self.pulse
            && chosen == current
        {
            for (i, pop) in self.pop.iter_mut().enumerate() {
                if current > 0.
                    && i < (current.ceil() as usize)
                    && (wave || i + 1 == current.ceil() as usize)
                {
                    pop.arm(if wave { i as f32 * 0.035 } else { 0. }, cx);
                }
            }
            self.pulse = None;
        }
        let (cell, glyph) = match self.size {
            Size::Sm => (28., 18.),
            Size::Md => (36., 24.),
            Size::Lg => (44., 30.),
        };
        let step = if self.half { 0.5 } else { 1. };
        let active = self.radio_index(current);
        let rated = target > 0.;
        let mut stars = div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(2.))
            .mx(px((glyph - cell) / 2.));
        for i in 0..self.max {
            let fill_target = (target - i as f32).clamp(0., 1.);
            let kept_target = (current - i as f32).clamp(0., 1.);
            let order = if self.sweep_to > self.sweep_from {
                i as isize - self.sweep_from.floor() as isize
            } else {
                self.sweep_from.ceil() as isize - 1 - i as isize
            };
            self.fill[i].delayed(fill_target, 0.26, order.max(0) as f32 * 0.035, cx);
            self.kept[i].duration(kept_target, 0.4, cx);
            let fill = self.fill[i].get(w, cx);
            let kept = self.kept[i].get(w, cx);
            let lift = if self.pressed == Some(i) && !reduced {
                0.86
            } else if rated && fill_target == 0. {
                0.9
            } else {
                1.
            };
            self.lift[i].duration(lift, 0.26, cx);
            let scale = self.lift[i].get(w, cx) * self.pop[i].get(w, cx);
            let base_fill = if rated {
                colors.border
            } else {
                transparent_black()
            };
            let base_stroke = if rated {
                colors.border
            } else {
                colors.border_strong
            };
            let fade = |to| ColorFade {
                from: to,
                to,
                at: now,
            };
            let ink = self.inks[i].get_or_insert(StarInk {
                fill: fade(base_fill),
                stroke: fade(base_stroke),
            });
            let (_, a) = ink
                .fill
                .retarget(base_fill, now, if reduced { 0. } else { 0.24 });
            let (_, b) = ink
                .stroke
                .retarget(base_stroke, now, if reduced { 0. } else { 0.24 });
            if a || b {
                w.request_animation_frame();
            }
            let bounds = self.bounds[i].clone();
            let mut cell_view = div()
                .relative()
                .flex_none()
                .size(px(cell))
                .flex()
                .items_center()
                .justify_center()
                .child(star(
                    glyph,
                    scale,
                    fill,
                    kept,
                    *ink,
                    StarPhase {
                        warning: colors.warning,
                        now,
                        reduced,
                    },
                ))
                .child(
                    canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
                        .absolute()
                        .inset_0(),
                );
            if !self.read_only {
                for half in [true, false] {
                    if half && !self.half {
                        continue;
                    }
                    let own = i as f32 + if half { 0.5 } else { 1. };
                    let radio = self.radio_index(own);
                    let label = self.spoken(own, lang);
                    let mut hit = div()
                        .absolute()
                        .inset_0()
                        .rounded(crate::foundation::surface_radius(cx))
                        .when(self.half, |e| {
                            if half {
                                e.right(relative(0.5))
                            } else {
                                e.left(relative(0.5))
                            }
                        })
                        .id(("rating-radio", radio))
                        .role(Role::RadioButton)
                        .aria_label(label.clone())
                        .aria_toggled((current == own).into())
                        .debug_selector({
                            let name = self.label.clone();
                            let max = self.max;
                            move || format!("{name}: {own} of {max}")
                        })
                        .track_focus(&self.focus[radio])
                        .tab_index(if interactive && radio == active {
                            0
                        } else {
                            -1
                        });
                    if interactive {
                        hit = hit
                            .cursor_pointer()
                            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                s.key(&e.keystroke.key, own, w, cx)
                            }))
                            .on_click(cx.listener(move |s, e: &ClickEvent, w, cx| {
                                // Enter and space on the focused star arrive
                                // twice: once as the key handler's selection,
                                // once as the activation click GPUI synthesizes
                                // on key-up. Only a pointer click may toggle a
                                // committed value off; a keyboard activation is
                                // a plain selection, or holding the key would
                                // clear the score while it is being read.
                                if !matches!(e, ClickEvent::Keyboard(_)) {
                                    if s.suppress_until
                                        .is_some_and(|until| cx.background_executor().now() < until)
                                    {
                                        return;
                                    }
                                    s.focus_value(own, w, cx);
                                    s.commit(own, true, cx);
                                }
                            }));
                    }
                    cell_view = cell_view.child(hit);
                }
            }
            stars = stars.child(cell_view);
        }
        let mut stars = stars
            .id("rating-stars")
            .role(if self.read_only {
                Role::Image
            } else {
                Role::RadioGroup
            })
            .aria_label(if self.read_only {
                let score = if current > 0. {
                    self.numeric_text(current, lang)
                } else {
                    self.text_for(current, lang)
                };
                format!("{}: {score}", self.label)
            } else {
                self.label.to_string()
            })
            .when(!self.read_only, |d| {
                d.aria_description(self.description.clone())
            });
        if interactive {
            stars = stars
                .on_hover(cx.listener(|s, over: &bool, _, cx| {
                    if !over && !s.gesture.is_some_and(|g| g.touch) {
                        s.gesture = None;
                        s.held_at = None;
                        s.pressed = None;
                        s.preview = None;
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, _, cx| {
                        if !s.gesture.is_some_and(|g| g.touch) {
                            s.begin(e.position.x, false, cx);
                        }
                    }),
                )
                .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                    if !s.gesture.is_some_and(|g| g.touch) {
                        s.move_pointer(e.position.x, cx);
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseUpEvent, w, cx| {
                        if !s.gesture.is_some_and(|g| g.touch) {
                            s.end(e.position.x, false, w, cx);
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseUpEvent, w, cx| {
                        if !s.gesture.is_some_and(|g| g.touch) {
                            s.end(e.position.x, true, w, cx);
                        }
                    }),
                );
            let touch = self.touch.clone();
            let gesture = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
                TouchPhase::Started => s.begin(e.position.x, true, cx),
                TouchPhase::Moved => s.move_pointer(e.position.x, cx),
                TouchPhase::Ended => s.end(e.position.x, false, w, cx),
                TouchPhase::Cancelled => s.end(e.position.x, true, w, cx),
            });
            stars = stars.child(
                canvas(
                    |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                    move |_, hit, w, _| {
                        w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                            if !phase.bubble() {
                                return;
                            }
                            if e.phase == TouchPhase::Started {
                                touch.set(u8::from(hit.is_hovered(w)));
                                if touch.get() == 1 {
                                    gesture(e, w, cx);
                                }
                                return;
                            }
                            if touch.get() == 1 && e.phase == TouchPhase::Moved {
                                let dx = f32::from((e.position.x - e.start_position.x).abs());
                                let dy = f32::from((e.position.y - e.start_position.y).abs());
                                if dy > 4. && dy >= dx {
                                    touch.set(0);
                                    let cancelled = TouchDragEvent {
                                        phase: TouchPhase::Cancelled,
                                        ..e.clone()
                                    };
                                    gesture(&cancelled, w, cx);
                                    return;
                                }
                                if dx > 4. && dx > dy {
                                    touch.set(2);
                                }
                            }
                            if touch.get() == 2
                                || touch.get() == 1
                                    && matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                            {
                                w.prevent_default();
                                gesture(e, w, cx);
                                cx.stop_propagation();
                            }
                            if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                touch.set(0);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        }
        let text = self.text_for(target, lang);
        if self.word_current.as_ref() != Some(&text)
            && let Some(previous) = self.word_current.replace(text.clone())
        {
            self.word_prev = previous;
            self.word_dir = direction;
            self.word_clock.restart(cx);
        }
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(8.))
            .gap_y(px(2.))
            .when(self.disabled, |d| d.opacity(0.5))
            .child(stars);
        if self.show_value {
            let mut style = w.text_style();
            style.font_weight = FontWeight::NORMAL;
            let shape = |text: SharedString| {
                w.text_system()
                    .shape_line(text.clone(), px(14.), &[style.to_run(text.len())], None)
            };
            let steps = (self.max as f32 / step).round() as usize;
            let width = (0..=steps)
                .map(|i| f32::from(shape(self.text_for(i as f32 * step, lang)).width))
                .fold(0., f32::max);
            let age = self.word_clock.time(0.24, w, cx);
            let enter = ease((age / 0.24).clamp(0., 1.));
            let exit = standard_ease((age / 0.12).clamp(0., 1.));
            let old = words(&self.word_prev);
            let new = words(&text);
            let dir = self.word_dir;
            let ink = if target == 0. {
                colors.text_muted
            } else {
                colors.text_secondary
            };
            let mut runs: Vec<(SharedString, f32, f32, f32, f32)> = Vec::new();
            let mut x = 0.;
            for (i, word) in new.iter().enumerate() {
                if old.get(i) == Some(word) {
                    runs.push((word.clone(), x, 0., 1., 0.));
                } else {
                    runs.push((
                        word.clone(),
                        x,
                        (1. - enter) * 4.2 * dir,
                        enter,
                        (1. - enter) * 4.,
                    ));
                }
                x += f32::from(shape(word.clone()).width);
            }
            if exit < 1. {
                let mut x = 0.;
                for (i, word) in old.iter().enumerate() {
                    if new.get(i) != Some(word) {
                        runs.push((word.clone(), x, -exit * 3.5 * dir, 1. - exit, exit * 2.));
                    }
                    x += f32::from(shape(word.clone()).width);
                }
            }
            let readout = canvas(
                |_, _, _| (),
                move |b, _, w, cx| {
                    for (word, x, y, alpha, blur) in &runs {
                        if *alpha < 0.001 || word.is_empty() {
                            continue;
                        }
                        let taps = if *blur > 0.22 { 5 } else { 1 };
                        let mut run = style.to_run(word.len());
                        run.color = ink.opacity(*alpha / if taps == 5 { 4.3 } else { 1. });
                        let line = w
                            .text_system()
                            .shape_line(word.clone(), px(14.), &[run], None);
                        for (dx, dy) in [(0., 0.), (-0.7, 0.), (0.7, 0.), (0., -0.7), (0., 0.7)]
                            .into_iter()
                            .take(taps)
                        {
                            let _ = line.paint(
                                b.origin + point(px(x + dx * blur), px(y + dy * blur)),
                                b.size.height,
                                TextAlign::Left,
                                None,
                                w,
                                cx,
                            );
                        }
                    }
                },
            )
            .w(px(width))
            .h(px(19.6))
            .flex_none();
            row = row.child(readout);
        }
        div()
            .id("rating")
            .flex()
            .flex_col()
            .gap(px(6.))
            .text_color(colors.foreground)
            .when(!self.label.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(14.))
                        .line_height(relative(1.4))
                        .font_weight(FontWeight::MEDIUM)
                        .child(self.label.clone()),
                )
            })
            .child(row)
            .when(!self.description.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .line_height(relative(1.4))
                        .text_color(colors.text_muted)
                        .child(self.description.clone()),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Pop, Rating, RatingChanged};
    use gpui::{TestAppContext, TouchEvent, TouchId, TouchPhase, px, size};
    use std::{cell::Cell, rc::Rc, time::Duration};
    #[test]
    fn half_steps_have_independent_focus_and_home_selects_the_first_step() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) =
            app.add_window_view(|_, cx| Rating::new("Score", 5, 2.5, cx).with_half(true));
        w.run_until_parked();
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                assert_eq!(s.value(), 2.5);
                assert_eq!(s.radio_index(2.5), 4);
                s.key("home", 2.5, w, cx);
                assert_eq!(s.value(), 0.5);
                s.key("left", 0.5, w, cx);
                assert_eq!(s.value(), 0.);
                s.key("end", 0.5, w, cx);
                assert_eq!(s.value(), 5.);
                s.key("enter", 3.5, w, cx);
                assert_eq!(s.value(), 3.5);
            })
        });
        assert!(w.debug_bounds("Score: 0.5 of 5").is_some());
        assert!(w.debug_bounds("Score: 5 of 5").is_some());
    }
    #[test]
    fn controlled_commit_emits_intent_and_keeps_the_value_until_host_acknowledges() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            Rating::new("Score", 5, 2., cx)
                .with_controlled(true)
                .with_name("quality")
        });
        let last = Rc::new(Cell::new(0.));
        let result = last.clone();
        let _sub =
            w.update(|_, cx| cx.subscribe(&view, move |_, e: &RatingChanged, _| result.set(e.0)));
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.commit(4., false, cx);
                assert_eq!(s.value(), 2.);
                assert_eq!(s.form_value().unwrap().1, 2.);
                s.set_value(4., cx);
                assert_eq!(s.value(), 4.);
            })
        });
        w.run_until_parked();
        assert_eq!(last.get(), 4.);
    }
    #[test]
    fn mouse_scrub_commits_once_and_does_not_toggle_when_returning_to_the_start() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| Rating::new("Score", 5, 2., cx));
        w.simulate_resize(size(px(400.), px(200.)));
        w.run_until_parked();
        let first = w.debug_bounds("Score: 2 of 5").unwrap().center();
        let last = w.debug_bounds("Score: 5 of 5").unwrap().center();
        let calls = Rc::new(Cell::new(0));
        let copy = calls.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&view, move |_, _: &RatingChanged, _| {
                copy.set(copy.get() + 1)
            })
        });
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                s.begin(first.x, false, cx);
                s.move_pointer(last.x, cx);
                s.move_pointer(first.x, cx);
                s.end(first.x, false, w, cx);
                assert_eq!(s.value(), 2.);
                assert!(s.suppress_until.is_some());
            })
        });
        w.run_until_parked();
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn physical_pop_starts_at_rest_then_peaks_after_the_press() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        app.update(|cx| {
            let mut pop = Pop::new();
            pop.arm(0., cx);
            pop.spring.release(1., 9., 0.26, cx);
            assert_eq!(pop.spring.value(cx), 1.);
            let _ = pop;
        });
        let (view, w) = app.add_window_view(|_, cx| Rating::new("Score", 5, 0., cx));
        w.update(|_, cx| view.update(cx, |s, cx| s.commit(4., false, cx)));
        w.run_until_parked();
        w.executor().advance_clock(Duration::from_millis(50));
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
        w.update(|_, cx| {
            let s = view.read(cx);
            let scale = s.pop[0].spring.value(cx);
            assert!(scale > 1.10 && scale < 1.19, "{scale}");
        });
    }
    #[test]
    fn half_mode_uses_numbers_and_all_sizes_keep_the_source_cell_dimensions() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        for (size_mode, cell) in [
            (crate::primitives::Size::Sm, 28.),
            (crate::primitives::Size::Md, 36.),
            (crate::primitives::Size::Lg, 44.),
        ] {
            let (view, w) = app.add_window_view(|_, cx| {
                Rating::new("Score", 5, 3., cx)
                    .with_half(true)
                    .with_size(size_mode)
            });
            w.run_until_parked();
            w.update(|_, cx| {
                let s = view.read(cx);
                assert_eq!(
                    s.text_for(3., crate::primitives::control::UiLanguage::English)
                        .as_ref(),
                    "3 out of 5"
                );
                assert!((f32::from(s.bounds[0].get().size.width) - cell).abs() < 0.01);
            });
        }
    }
    #[test]
    fn cancelled_touch_does_not_commit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| Rating::new("Score", 5, 2., cx));
        w.run_until_parked();
        let count = Rc::new(Cell::new(0));
        let events = count.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&view, move |_, _: &RatingChanged, _| {
                events.set(events.get() + 1)
            })
        });
        let first = w.debug_bounds("Score: 1 of 5").unwrap().center();
        let last = w.debug_bounds("Score: 5 of 5").unwrap().center();
        for (phase, position) in [
            (TouchPhase::Started, first),
            (TouchPhase::Moved, last),
            (TouchPhase::Cancelled, last),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(1),
                phase,
                position,
                ..Default::default()
            });
            w.executor().advance_clock(Duration::from_millis(16));
            w.run_until_parked();
            w.update(|w, cx| {
                w.simulate_next_frame(cx);
                w.refresh();
            });
            w.run_until_parked();
        }
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 2.);
        assert_eq!(count.get(), 0);
    }
    #[test]
    fn real_pointer_click_and_keyboard_home_reach_the_radio_control() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) =
            app.add_window_view(|_, cx| Rating::new("Score", 5, 0., cx).with_half(true));
        w.run_until_parked();
        let target = w.debug_bounds("Score: 3.5 of 5").unwrap().center();
        w.simulate_click(target, gpui::Modifiers::default());
        w.run_until_parked();
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 3.5);
        w.simulate_keystrokes("home");
        w.run_until_parked();
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 0.5);
        w.simulate_keystrokes("backspace");
        w.run_until_parked();
        assert_eq!(w.update(|_, cx| view.read(cx).value()), 0.);
    }
}
