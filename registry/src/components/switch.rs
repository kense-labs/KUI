//! A labelled setting with a spring-driven thumb.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::activate,
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, press, standard_ease},
    },
};
use gpui::{prelude::*, *};

const THUMB: f32 = 18.;
const TRAVEL: f32 = 18.;
const FADE: f32 = 0.24;

pub struct Switch {
    id: ElementId,
    label: SharedString,
    track_radius: Option<Pixels>,
    disabled: bool,
    external: PhaseClock,
    checked: bool,
    position: Slide,
    stretch: Slide,
    hover: Slide,
    fade: PhaseClock,
    fade_from: f32,
    focus: FocusHandle,
    _blur: Subscription,
}

impl EventEmitter<bool> for Switch {}

impl Switch {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let blur = cx.on_blur(&focus, w, |v: &mut Self, _, cx| {
            v.stretch.set(0., cx);
            cx.notify()
        });
        Self {
            id: id.into(),
            label: label.into(),
            track_radius: None,
            disabled: false,
            external: Default::default(),
            checked: false,
            position: Slide::new(0.),
            stretch: Slide::snappy(0.),
            hover: Slide::snappy(0.),
            fade: PhaseClock::default(),
            fade_from: 0.,
            focus,
            _blur: blur,
        }
    }

    /// Override the capsule radius when embedding the switch in an existing toolbar.
    pub fn with_track_radius(mut self, radius: Pixels) -> Self {
        self.track_radius = Some(radius);
        self
    }
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self.position.reset(u8::from(checked) as f32);
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn is_checked(&self) -> bool {
        self.checked
    }
    /// Synchronize an application change without emitting another input event.
    pub fn set_checked(&mut self, checked: bool, cx: &mut Context<Self>) {
        if self.checked != checked {
            self.toggle(checked, cx);
            self.external.restart(cx)
        }
    }
    fn toggle(&mut self, checked: bool, cx: &mut Context<Self>) {
        self.fade_from = self.tint(self.fade.age(cx));
        self.checked = checked;
        self.position.duration(u8::from(checked) as f32, 0.30, cx);
        self.fade.restart(cx);
        cx.notify()
    }
    fn tint(&self, elapsed: f32) -> f32 {
        let end = if self.checked { 1. } else { 0. };
        self.fade_from + (end - self.fade_from) * standard_ease(elapsed / FADE)
    }
}

impl Render for Switch {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let position = self.position.get(w, cx).clamp(0., 1.);
        let extra = if disabled_motion(cx) {
            0.
        } else {
            5. * self.stretch.get(w, cx).clamp(0., 1.)
        };
        let tint = self.tint(self.fade.time(FADE, w, cx));
        let hover = self.hover.get(w, cx).clamp(0., 1.);
        let track_color = p
            .hover
            .blend(p.ink.opacity(0.08 * hover))
            .blend(p.bright.opacity(tint));
        let thumb_color = Paint::for_mode(ThemeMode::Light)
            .panel
            .blend(p.inverse.opacity(tint));

        let phase = self.external.time(0.34, w, cx) / 0.34;
        let pulse = 1. + 0.16 * (std::f32::consts::PI * phase).sin().max(0.);
        let thumb = div()
            .absolute()
            .left(px(
                3. + position * (TRAVEL - extra) - THUMB * (pulse - 1.) * 0.5
            ))
            .top(px(3.))
            .w(px(THUMB * pulse + extra))
            .h(px(THUMB))
            .rounded_full()
            .bg(thumb_color)
            .shadow_sm();
        let track = div()
            .relative()
            .flex_none()
            .w(px(42.))
            .h(px(24.))
            .rounded(self.track_radius.unwrap_or(px(12.)))
            .bg(track_color)
            .child(thumb);
        let label = self.label.clone();
        let control = face(
            self.id.clone(),
            label.clone(),
            44.,
            radius.into(),
            Tone::Clear,
            cx,
        )
        .role(Role::Switch)
        .aria_toggled(self.checked.into())
        .track_focus(&self.focus)
        .relative()
        .justify_start()
        .gap(px(12.))
        .on_hover(cx.listener(|v, over: &bool, _, cx| {
            v.hover.duration(if *over { 1. } else { 0. }, 0.16, cx);
            if !over {
                v.stretch.set(0., cx);
            }
            cx.notify()
        }))
        .child(track)
        .child(label);
        if self.disabled {
            return control.tab_stop(false).cursor_default().opacity(0.5);
        }
        let control = press(control, cx, |v| &mut v.stretch);
        activate(control, cx, |v, _, cx| {
            v.toggle(!v.checked, cx);
            cx.emit(v.checked);
            cx.notify()
        })
    }
}
