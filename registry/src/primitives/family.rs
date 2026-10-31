//! Compact action surfaces scoped to the button family.
use crate::foundation::{Theme, ThemeMode};
use gpui::{prelude::*, *};
use web_time::Instant;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Surface,
    Primary,
    Muted,
    Danger,
    Success,
    Clear,
}
#[derive(Clone, Copy)]
pub struct Paint {
    pub stage: Hsla,
    pub panel: Hsla,
    pub tray: Hsla,
    pub hover: Hsla,
    pub ink: Hsla,
    pub secondary: Hsla,
    pub faint: Hsla,
    pub line: Hsla,
    pub bright: Hsla,
    pub inverse: Hsla,
    pub danger: Hsla,
    pub success: Hsla,
}
impl Paint {
    pub fn get(cx: &App) -> Self {
        Self::for_mode(Theme::global(cx).mode)
    }
    pub fn for_mode(mode: ThemeMode) -> Self {
        let dark = mode == ThemeMode::Dark;
        let [
            stage,
            panel,
            tray,
            hover,
            ink,
            secondary,
            faint,
            line,
            bright,
            inverse,
            danger,
        ] = if dark {
            [
                0x242424, 0x191919, 0x303030, 0x353535, 0xf2f2f2, 0xb5b5b5, 0x898989, 0x2b2b2b,
                0xf2f2f2, 0x141414, 0xff746f,
            ]
        } else {
            [
                0xf5f5f5, 0xffffff, 0xffffff, 0xe9e9e9, 0x1a1a1a, 0x575757, 0x747474, 0xe2e2e2,
                0x181818, 0xfafafa, 0xbd302c,
            ]
        };
        Self {
            stage: rgb(stage).into(),
            panel: rgb(panel).into(),
            tray: rgb(tray).into(),
            hover: rgb(hover).into(),
            ink: rgb(ink).into(),
            secondary: rgb(secondary).into(),
            faint: rgb(faint).into(),
            line: rgb(line).into(),
            bright: rgb(bright).into(),
            inverse: rgb(inverse).into(),
            danger: rgb(danger).into(),
            success: rgb(if dark { 0x53d986 } else { 0x008039 }).into(),
        }
    }
    pub fn colors(self, t: Tone) -> (Hsla, Hsla) {
        match t {
            Tone::Primary => (self.bright, self.inverse),
            Tone::Success => (self.panel.blend(self.success.opacity(0.10)), self.ink),
            Tone::Muted => (self.secondary, self.inverse),
            Tone::Clear => (transparent_black(), self.ink),
            _ => (self.panel, self.ink),
        }
    }
}
pub fn face(
    id: impl Into<ElementId>,
    name: impl Into<SharedString>,
    height: f32,
    r: f32,
    t: Tone,
    cx: &App,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    let (bg, fg) = p.colors(t);
    let name = name.into();
    let ring = Theme::global(cx).tokens.colors.focus_outline;
    div()
        .id(id)
        .debug_selector({
            let name = name.clone();
            move || name.to_string()
        })
        .aria_label(name)
        .role(Role::Button)
        .tab_index(0)
        .h(px(height))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .rounded(px(r))
        .bg(bg)
        .text_color(fg)
        .text_size(px(14.))
        .line_height(px(19.6))
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .cursor_pointer()
        .when(t == Tone::Danger, |e| {
            e.border_1().border_color(p.danger.opacity(0.42))
        })
        .when(t == Tone::Success, |e| {
            e.border_1().border_color(p.success.opacity(0.40))
        })
        .focus_visible(move |s| s.border_1().border_color(ring))
}
/// Analytic critically damped spring. Retargeting preserves instantaneous velocity.
#[derive(Clone, Copy, Debug)]
pub struct Slide {
    from: f32,
    to: f32,
    velocity: f32,
    start: Option<Instant>,
    omega: f32,
}
impl Slide {
    pub fn new(v: f32) -> Self {
        Self {
            from: v,
            to: v,
            velocity: 0.,
            start: None,
            omega: 12.47,
        }
    }
    pub fn snappy(v: f32) -> Self {
        Self {
            omega: 20.14,
            ..Self::new(v)
        }
    }
    pub fn reset(&mut self, v: f32) {
        self.from = v;
        self.to = v;
        self.velocity = 0.;
        self.start = None;
    }
    pub fn set(&mut self, v: f32, cx: &App) {
        self.retarget(v, self.omega, cx);
    }
    pub fn duration(&mut self, v: f32, visual_seconds: f32, cx: &App) {
        self.retarget(v, 5.236 / visual_seconds, cx)
    }
    fn retarget(&mut self, v: f32, omega: f32, cx: &App) {
        if disabled_motion(cx) {
            self.reset(v);
            return;
        }
        if self.to == v {
            return;
        }
        let now = cx.background_executor().now();
        let (pos, vel, _) = self.sample(now);
        self.from = pos;
        self.velocity = vel;
        self.omega = omega;
        self.to = v;
        self.start = Some(now);
    }
    fn sample(&self, now: Instant) -> (f32, f32, bool) {
        let Some(start) = self.start else {
            return (self.to, 0., false);
        };
        let t = now.saturating_duration_since(start).as_secs_f32();
        let d = self.from - self.to;
        let k = self.velocity + self.omega * d;
        let e = (-self.omega * t).exp();
        let offset = (d + k * t) * e;
        let vel = (self.velocity - self.omega * k * t) * e;
        if offset.abs() < 0.0008 && vel.abs() < 0.012 || t > 1.6 {
            (self.to, 0., false)
        } else {
            (self.to + offset, vel, true)
        }
    }
    /// Sample from an input handler without scheduling a frame outside paint.
    pub fn value(&self, cx: &App) -> f32 {
        if disabled_motion(cx) {
            self.to
        } else {
            self.sample(cx.background_executor().now()).0
        }
    }
    pub fn get(&self, w: &Window, cx: &App) -> f32 {
        if disabled_motion(cx) {
            return self.to;
        }
        let (v, _, m) = self.sample(cx.background_executor().now());
        if m {
            w.request_animation_frame()
        }
        v
    }
}
pub fn disabled_motion(cx: &App) -> bool {
    cx.try_global::<Theme>().is_some_and(|t| t.reduced_motion)
}
/// A finite paint clock; never schedules frames once its last phase has finished.
#[derive(Clone, Copy, Default, Debug)]
pub struct PhaseClock(Option<Instant>);
impl PhaseClock {
    pub fn restart(&mut self, cx: &App) {
        self.0 = Some(cx.background_executor().now())
    }
    pub fn age(self, cx: &App) -> f32 {
        self.0.map_or(f32::INFINITY, |s| {
            cx.background_executor()
                .now()
                .saturating_duration_since(s)
                .as_secs_f32()
        })
    }
    pub fn time(self, end: f32, w: &Window, cx: &App) -> f32 {
        if disabled_motion(cx) {
            return end;
        }
        let t = self.age(cx);
        if t < end {
            w.request_animation_frame()
        }
        t.min(end)
    }
}
pub fn ease(t: f32) -> f32 {
    cubic_out(t, 0.16, 0.30)
}
/// Cubic-bezier(.22, 1, .36, 1) used by page-scale wipes.
pub fn standard_ease(x: f32) -> f32 {
    cubic_out(x, 0.22, 0.36)
}
fn cubic_out(x: f32, x1: f32, x2: f32) -> f32 {
    if x <= 0. || x >= 1. {
        return x.clamp(0., 1.);
    }
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..14 {
        let t = (lo + hi) * 0.5;
        let v = 1. - t;
        let at = 3. * x1 * v * v * t + 3. * x2 * v * t * t + t * t * t;
        if at < x { lo = t } else { hi = t }
    }
    1. - (1. - (lo + hi) * 0.5_f32).powi(3)
}
pub fn measure(text: &SharedString, size: f32, w: &Window) -> f32 {
    let mut style = w.text_style();
    style.font_weight = FontWeight::MEDIUM;
    let advance: f32 = w
        .text_system()
        .shape_line(text.clone(), px(size), &[style.to_run(text.len())], None)
        .width
        .into();
    (advance - 0.01 * size * text.chars().count() as f32).max(0.)
}
/// Pointer feedback lives in the element's stable hit box, independent of its paint.
pub fn press<T: 'static>(
    el: Stateful<Div>,
    cx: &Context<T>,
    field: fn(&mut T) -> &mut Slide,
) -> Stateful<Div> {
    el.on_mouse_down(
        MouseButton::Left,
        cx.listener(move |v, _, _, cx| {
            field(v).set(1., cx);
            cx.notify()
        }),
    )
    .on_mouse_up(
        MouseButton::Left,
        cx.listener(move |v, _, _, cx| {
            field(v).set(0., cx);
            cx.notify()
        }),
    )
    .on_mouse_up_out(
        MouseButton::Left,
        cx.listener(move |v, _, _, cx| {
            field(v).set(0., cx);
            cx.notify()
        }),
    )
    .on_key_down(cx.listener(move |v, e: &KeyDownEvent, _, cx| {
        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
            field(v).set(1., cx);
            cx.notify()
        }
    }))
    .on_key_up(cx.listener(move |v, e: &KeyUpEvent, _, cx| {
        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
            field(v).set(0., cx);
            cx.notify()
        }
    }))
}

/// Stable glyph identities are matched in reading order, including common suffixes.
fn shared_glyphs(
    left: &[(SharedString, f32)],
    right: &[(SharedString, f32)],
) -> Vec<Option<usize>> {
    let stride = right.len() + 1;
    let mut lcs = vec![0u16; (left.len() + 1) * stride];
    for i in (0..left.len()).rev() {
        for j in (0..right.len()).rev() {
            lcs[i * stride + j] = if left[i].0 == right[j].0 {
                1 + lcs[(i + 1) * stride + j + 1]
            } else {
                lcs[(i + 1) * stride + j].max(lcs[i * stride + j + 1])
            };
        }
    }
    let mut out = vec![None; left.len()];
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i].0 == right[j].0 {
            out[i] = Some(j);
            i += 1;
            j += 1
        } else if lcs[(i + 1) * stride + j] >= lcs[i * stride + j + 1] {
            i += 1
        } else {
            j += 1
        }
    }
    out
}
/// Paint two label phases in one measured slot; the outgoing face never affects layout.
pub fn lettering(
    old: SharedString,
    new: SharedString,
    time: f32,
    letters: bool,
    size: f32,
    ink: Hsla,
    width: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, cx| {
            let plain = old == new || time >= 0.48;
            let mut style = w.text_style();
            style.font_weight = FontWeight::MEDIUM;
            let width_of = |s: &SharedString| {
                f32::from(
                    w.text_system()
                        .shape_line(s.clone(), px(size), &[style.to_run(s.len())], None)
                        .width,
                )
            };
            let old_x = (width - width_of(&old)) * 0.5;
            let new_x = (width - width_of(&new)) * 0.5;
            if plain {
                let mut run = style.to_run(new.len());
                run.color = ink;
                let line = w
                    .text_system()
                    .shape_line(new.clone(), px(size), &[run], None);
                let _ = line.paint(
                    b.origin + point(px(new_x), px(0.)),
                    b.size.height,
                    TextAlign::Left,
                    None,
                    w,
                    cx,
                );
                return;
            }
            let pieces = |s: &SharedString| -> Vec<(SharedString, f32)> {
                let line =
                    w.text_system()
                        .shape_line(s.clone(), px(size), &[style.to_run(s.len())], None);
                s.char_indices()
                    .map(|(i, c)| (c.to_string().into(), line.x_for_index(i).into()))
                    .collect()
            };
            let mut runs = Vec::new();
            if letters && old.len() < 256 && new.len() < 256 {
                let left = pieces(&old);
                let right = pieces(&new);
                let pairs = shared_glyphs(&left, &right);
                let mut stable = vec![false; right.len()];
                let out = 1. - standard_ease(time / 0.16);
                let layout = 1. - (1. + 12.47 * time) * (-12.47 * time).exp();
                for ((glyph, dx), matched) in left.into_iter().zip(pairs) {
                    if let Some(j) = matched {
                        let x = old_x + dx;
                        stable[j] = true;
                        runs.push((glyph, x + (new_x + right[j].1 - x) * layout, 0., 1., 0.));
                    } else {
                        runs.push((glyph, old_x + dx, -3. * (1. - out), out, 2. * (1. - out)));
                    }
                }
                let mut order = 0;
                for ((glyph, dx), matched) in right.into_iter().zip(stable) {
                    if !matched {
                        let k = ease((time - (order as f32 * 0.016).min(0.1)) / 0.24);
                        runs.push((glyph, new_x + dx, 4. * (1. - k), k, 2. * (1. - k)));
                        order += 1;
                    }
                }
            } else {
                let incoming = ease(time / 0.16);
                runs.push((old.clone(), old_x, 0., 1. - standard_ease(time / 0.10), 0.));
                runs.push((new.clone(), new_x, 2. * (1. - incoming), incoming, 0.));
            }
            for (s, x, y, alpha, blur) in runs {
                if alpha < 0.001 || s.is_empty() {
                    continue;
                }
                let taps = if blur > 0.22 { 5 } else { 1 };
                let mut run = style.to_run(s.len());
                run.color = ink.opacity(alpha / if taps == 5 { 4.3 } else { 1. });
                let line = w.text_system().shape_line(s, px(size), &[run], None);
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
    .w(px(width.max(0.)))
    .h(px(size * 1.4))
    .flex_none()
}

#[cfg(test)]
mod spring_tests {
    use super::{Instant, Slide};
    use std::time::Duration;
    #[test]
    fn reversing_keeps_position_and_velocity() {
        let s = Instant::now();
        let mid = s + Duration::from_millis(75);
        let a = Slide {
            start: Some(s),
            to: 1.,
            ..Slide::new(0.)
        };
        let (x, v, _) = a.sample(mid);
        let reversed = Slide {
            start: Some(mid),
            from: x,
            velocity: v,
            to: 0.,
            ..a
        };
        assert_eq!(reversed.sample(mid), (x, v, true));
        assert!(reversed.sample(mid + Duration::from_millis(240)).0 < x);
        assert_eq!(
            reversed.sample(mid + Duration::from_secs(2)),
            (0., 0., false)
        );
    }
    #[test]
    fn unchanged_suffix_keeps_its_identity_across_label_swaps() {
        let glyphs = |s: &str| {
            s.chars()
                .map(|g| (gpui::SharedString::from(g.to_string()), 0.))
                .collect::<Vec<_>>()
        };
        let old = glyphs("Add to library");
        let new = glyphs("In library");
        let pairs = super::shared_glyphs(&old, &new);
        assert_eq!(
            &pairs[old.len() - 7..],
            &[
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                Some(8),
                Some(9)
            ]
        );
        assert_eq!(
            super::shared_glyphs(&glyphs("添加成员"), &glyphs("移除成员")),
            vec![None, None, Some(2), Some(3)]
        );
    }
}
