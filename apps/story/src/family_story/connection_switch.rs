//! Local failure-injection control for the invitation example.
use gpui::{prelude::*, *};
use kui::{
    foundation::{Theme, ThemeMode},
    primitives::{
        control::{UiLanguage, activate},
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, press, standard_ease},
    },
};

const THUMB: f32 = 18.;
const TRAVEL: f32 = 18.;
const FADE: f32 = 0.24;

pub(super) struct ConnectionSwitch {
    checked: bool,
    position: Slide,
    stretch: Slide,
    hover: Slide,
    fade: PhaseClock,
    fade_from: f32,
    focus: FocusHandle,
    _blur: Subscription,
}

impl EventEmitter<bool> for ConnectionSwitch {}

impl ConnectionSwitch {
    pub(super) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let blur = cx.on_blur(&focus, w, |v: &mut Self, _, cx| {
            v.stretch.set(0., cx);
            cx.notify()
        });
        Self {
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

    fn tint(&self, elapsed: f32) -> f32 {
        let end = if self.checked { 1. } else { 0. };
        self.fade_from + (end - self.fade_from) * standard_ease(elapsed / FADE)
    }
}

impl Render for ConnectionSwitch {
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

        let thumb = div()
            .absolute()
            .left(px(3. + position * (TRAVEL - extra)))
            .top(px(3.))
            .w(px(THUMB + extra))
            .h(px(THUMB))
            .rounded_full()
            .bg(thumb_color)
            .shadow_sm();
        let track = div()
            .relative()
            .flex_none()
            .w(px(42.))
            .h(px(24.))
            .rounded(radius)
            .bg(track_color)
            .child(thumb);
        let label = UiLanguage::current(cx).text("Simulate a lost connection", "模拟网络中断");
        let control = face("connection", label, 44., radius.into(), Tone::Clear, cx)
            .role(Role::Switch)
            .aria_toggled(self.checked.into())
            .track_focus(&self.focus)
            .relative()
            .top(px(10.))
            .justify_start()
            .mt(px(8.))
            .gap(px(10.))
            .on_hover(cx.listener(|v, over: &bool, _, cx| {
                v.hover.duration(if *over { 1. } else { 0. }, 0.16, cx);
                if !over {
                    v.stretch.set(0., cx);
                }
                cx.notify()
            }))
            .child(track)
            .child(label);
        let control = press(control, cx, |v| &mut v.stretch);
        activate(control, cx, |v, _, cx| {
            v.fade_from = v.tint(v.fade.age(cx));
            v.checked = !v.checked;
            v.position
                .duration(if v.checked { 1. } else { 0. }, 0.30, cx);
            if disabled_motion(cx) {
                v.fade = PhaseClock::default()
            } else {
                v.fade.restart(cx)
            }
            cx.emit(v.checked);
            cx.notify()
        })
    }
}
