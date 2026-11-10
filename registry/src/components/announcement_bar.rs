//! Dismissible announcements with optional action, countdown, rotation ring and inverted tone.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    form_chrome::tr,
    glyph::{Glyph, countdown_ring, icon},
};
use gpui::{prelude::*, *};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnnouncementTone {
    #[default]
    Surface,
    Inverted,
}

#[derive(Clone)]
pub struct Announcement {
    pub id: SharedString,
    pub message: SharedString,
    pub action: Option<SharedString>,
    pub countdown: Option<Duration>,
}
impl Announcement {
    pub fn new(id: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            action: None,
            countdown: None,
        }
    }
    pub fn with_action(mut self, label: impl Into<SharedString>) -> Self {
        self.action = Some(label.into());
        self
    }
    /// Count down from this duration using a monotonic clock.
    pub fn with_countdown(mut self, duration: Duration) -> Self {
        self.countdown = Some(duration);
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnouncementEvent {
    Action(SharedString),
    Dismissed,
}
/// Rotates visible messages and sends actions and dismissal to the host.
/// Persistence, navigation and campaign scheduling belong to the host.
pub struct AnnouncementBar {
    messages: Vec<Announcement>,
    tone: AnnouncementTone,
    active: usize,
    dismissed: bool,
    paused: bool,
    user_paused: bool,
    focused: bool,
    focus: FocusHandle,
    focus_subscriptions: Vec<Subscription>,
    deadlines: Vec<Option<web_time::Instant>>,
    rotation_ticks: u8,
    motion: Slide,
    visibility: Slide,
    previous: Option<SharedString>,
    bounds: std::rc::Rc<std::cell::Cell<Bounds<Pixels>>>,
    epoch: u64,
}
impl EventEmitter<AnnouncementEvent> for AnnouncementBar {}
impl AnnouncementBar {
    pub fn new(messages: Vec<Announcement>, cx: &mut Context<Self>) -> Self {
        let now = cx.background_executor().now();
        let deadlines = messages
            .iter()
            .map(|message| {
                message
                    .countdown
                    .and_then(|duration| now.checked_add(duration))
            })
            .collect();
        let result = Self {
            messages,
            tone: AnnouncementTone::default(),
            active: 0,
            dismissed: false,
            paused: false,
            user_paused: false,
            focused: false,
            focus: cx.focus_handle(),
            focus_subscriptions: Vec::new(),
            deadlines,
            rotation_ticks: 0,
            motion: Slide::snappy(1.),
            visibility: Slide::snappy(1.),
            previous: None,
            bounds: Default::default(),
            epoch: 0,
        };
        if result.messages.len() > 1 || result.deadlines.iter().any(Option::is_some) {
            Self::schedule(0, cx);
        }
        result
    }
    pub fn with_tone(mut self, tone: AnnouncementTone) -> Self {
        self.tone = tone;
        self
    }
    pub fn dismissed(&self) -> bool {
        self.dismissed
    }
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        if !self.dismissed {
            self.dismissed = true;
            self.visibility.duration(0., 0.44, cx);
            self.epoch = self.epoch.wrapping_add(1);
            cx.emit(AnnouncementEvent::Dismissed);
            cx.notify();
        }
    }
    pub fn reopen(&mut self, cx: &mut Context<Self>) {
        if self.dismissed {
            self.dismissed = false;
            self.visibility.duration(1., 0.44, cx);
            self.epoch = self.epoch.wrapping_add(1);
            if self.messages.len() > 1 || self.deadlines.iter().any(Option::is_some) {
                Self::schedule(self.epoch, cx);
            }
            cx.notify();
        }
    }
    fn advance(&mut self, cx: &mut Context<Self>) {
        if self.messages.len() > 1 {
            self.previous = self
                .messages
                .get(self.active)
                .map(|message| message.message.clone());
            self.active = (self.active + 1) % self.messages.len();
            self.rotation_ticks = 0;
            self.motion.reset(0.);
            self.motion.duration(1., 0.46, cx);
            cx.notify();
        }
    }
    fn schedule(epoch: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let _ = this.update(cx, |s, cx| {
                if s.epoch != epoch || s.dismissed {
                    return;
                }
                if !s.paused && !s.user_paused && !s.focused {
                    s.rotation_ticks += 1;
                    if s.rotation_ticks >= 6 {
                        s.rotation_ticks = 0;
                        s.advance(cx);
                    }
                }
                if s.deadlines.iter().any(Option::is_some) || s.messages.len() > 1 {
                    cx.notify();
                }
                let now = cx.background_executor().now();
                let pending = s.deadlines.iter().flatten().any(|deadline| *deadline > now);
                if pending || s.messages.len() > 1 {
                    Self::schedule(epoch, cx);
                }
            });
        })
        .detach();
    }
}
impl Render for AnnouncementBar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions
                .push(cx.on_focus_in(&self.focus, w, |s, _, _| s.focused = true));
            self.focus_subscriptions
                .push(cx.on_focus_out(&self.focus, w, |s, _, _, _| s.focused = false));
        }
        let p = Paint::get(cx);
        let visibility = self.visibility.get(w, cx).clamp(0., 1.);
        let measured = self.bounds.get().size;
        let mut root = div()
            .w_full()
            .flex_none()
            .when(measured.height > px(0.), |d| {
                d.h(measured.height * visibility)
            })
            .overflow_hidden()
            .opacity(visibility);
        let Some(message) = self
            .messages
            .get(self.active)
            .cloned()
            .filter(|_| visibility > 0.)
        else {
            return root;
        };
        let inverted = self.tone == AnnouncementTone::Inverted;
        let bg = if inverted { p.bright } else { p.tray };
        let fg = if inverted { p.inverse } else { p.ink };
        let muted = if inverted {
            p.inverse.opacity(0.66)
        } else {
            p.secondary
        };
        let hover_bg = if inverted {
            p.inverse.opacity(0.10)
        } else {
            p.hover
        };
        let opacity = self.motion.get(w, cx).clamp(0., 1.);
        let radius = crate::foundation::surface_radius(cx);
        let viewport = w.viewport_size().width;
        let compact = (if measured.width > px(0.) {
            measured.width
        } else {
            viewport
        }) <= px(560.);
        let left_padding = if compact { 14. } else { 16. };
        let controls_width = if self.messages.len() > 1 { 66. } else { 32. };
        let side_count = if compact { 1. } else { 2. };
        let content_width =
            measured.width - px(left_padding + 8. + side_count * (12. + controls_width));
        let mut next = div()
            .flex()
            .flex_wrap()
            .items_center()
            .when(!compact, |d| d.justify_center())
            .gap_x(px(if compact { 10. } else { 12. }))
            .gap_y(px(2.))
            .py(px(6.))
            .opacity(opacity)
            .child(
                div()
                    .min_w_0()
                    .max_w_full()
                    .whitespace_normal()
                    .text_color(fg)
                    .child(message.message.clone()),
            );
        if let Some(deadline) = self.deadlines[self.active] {
            let remaining = deadline.saturating_duration_since(cx.background_executor().now());
            let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
            let d = seconds / 86400;
            let h = seconds / 3600 % 24;
            let m = seconds / 60 % 60;
            let s = seconds % 60;
            let time_str = if d > 0 {
                format!("{d}d {h:02}:{m:02}:{s:02}")
            } else if h > 0 {
                format!("{h:02}:{m:02}:{s:02}")
            } else {
                format!("{m:02}:{s:02}")
            };
            next = next.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_color(muted)
                    .child(tr(cx, "Ends in", "剩余"))
                    .child(
                        div()
                            .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(fg)
                            .child(time_str),
                    ),
            );
        }
        if let Some(action) = message.action.clone() {
            let id = message.id.clone();
            next = next.child(
                face(
                    "announcement-action",
                    action.clone(),
                    20.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .flex_none()
                .gap(px(4.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                .hover(move |s| s.bg(hover_bg))
                .child(action)
                .child(icon(Glyph::Arrow, 13., fg))
                .on_click(
                    cx.listener(move |_, _, _, cx| cx.emit(AnnouncementEvent::Action(id.clone()))),
                ),
            );
        }
        let mut contents = div().relative().min_w_0().flex_1().child(next);
        if content_width > px(0.) {
            contents = contents.w(content_width).flex_none();
        }
        if opacity < 1.
            && let Some(previous) = &self.previous
        {
            contents = contents.child(
                div()
                    .absolute()
                    .left(px(0.))
                    .right(px(0.))
                    .top(px(0.))
                    .bottom(px(0.))
                    .flex()
                    .items_center()
                    .when(!compact, |d| d.justify_center())
                    .opacity(1. - opacity)
                    .text_color(fg)
                    .child(previous.clone()),
            );
        }
        let mut controls = div().flex().flex_none().items_center().gap(px(2.));
        if self.messages.len() > 1 {
            let progress = 1. - (self.rotation_ticks as f32 / 6.).clamp(0., 1.);
            let next_label = tr(cx, "Next announcement", "下一条公告");
            controls = controls.child(
                face("announcement-next", next_label, 32., 16., Tone::Clear, cx)
                    .size(px(32.))
                    .hover(move |s| s.bg(hover_bg))
                    .child(countdown_ring(progress, 20., muted))
                    .on_click(cx.listener(|s, _, _, cx| s.advance(cx))),
            );
        }
        let dismiss_label: SharedString = tr(cx, "Dismiss announcement", "关闭公告").into();
        controls = controls.child(
            face(
                "announcement-dismiss",
                dismiss_label,
                32.,
                16.,
                Tone::Clear,
                cx,
            )
            .size(px(32.))
            .hover(move |s| s.bg(hover_bg))
            .child(icon(Glyph::Close, 15., muted))
            .on_click(cx.listener(|s, _, _, cx| s.dismiss(cx))),
        );
        let bounds = self.bounds.clone();
        root = root
            .on_children_prepainted(move |children, w, _| {
                if let Some(child) = children.first()
                    && bounds.replace(*child).size != child.size
                {
                    w.request_animation_frame();
                }
            })
            .child(
                div()
                    .id("announcement")
                    .track_focus(&self.focus)
                    .tab_index(-1)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .w_full()
                    .flex_none()
                    .min_h(px(44.))
                    .bg(bg)
                    .when(!inverted, |d| d.border_b_1().border_color(p.line))
                    .pl(px(left_padding))
                    .pr(px(8.))
                    .py(px(6.))
                    .text_size(px(14.))
                    .text_color(fg)
                    .on_hover(cx.listener(|s, hover, _, _| s.paused = *hover))
                    .when(!compact, |d| {
                        d.child(div().flex_none().w(px(controls_width)))
                    })
                    .child(contents)
                    .child(controls),
            );
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{Announcement, AnnouncementBar};
    use gpui::prelude::{ParentElement, Styled};
    use gpui::{
        AppContext, Context, Entity, InteractiveElement, IntoElement, Render, TestAppContext,
        Window, div, px, size,
    };
    use std::time::Duration;

    struct Page(Entity<AnnouncementBar>);

    impl Render for Page {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w_full()
                .flex()
                .flex_col()
                .child(self.0.clone())
                .child(
                    div()
                        .id("after")
                        .debug_selector(|| "after".into())
                        .h(px(20.)),
                )
        }
    }

    #[test]
    fn wrapped_announcement_pushes_following_content_and_collapses_from_its_height() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (page, mut window) = app.add_window_view(|_, cx| {
            Page(cx.new(|cx| {
                AnnouncementBar::new(
                    vec![
                        Announcement::new("long", "An announcement with several words. ".repeat(8))
                            .with_action("Learn more"),
                    ],
                    cx,
                )
            }))
        });
        window.simulate_resize(size(px(320.), px(700.)));
        for _ in 0..4 {
            window.update(|w, _| w.refresh());
            window.run_until_parked();
        }
        let bar = window.update(|_, cx| page.read(cx).0.clone());
        let natural_height = window.update(|_, cx| bar.read(cx).bounds.get().size.height);
        assert!(natural_height > px(120.), "height: {natural_height:?}");
        assert_eq!(window.debug_bounds("after").unwrap().top(), natural_height);
        window.update(|_, cx| bar.update(cx, |bar, cx| bar.dismiss(cx)));
        window.executor().advance_clock(Duration::from_millis(40));
        window.update(|w, _| w.refresh());
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| bar.read(cx).bounds.get().size.height),
            natural_height
        );
        let collapsing = window.debug_bounds("after").unwrap().top();
        assert!(collapsing > px(120.) && collapsing < natural_height);
        window.executor().advance_clock(Duration::from_secs(2));
        window.update(|w, _| w.refresh());
        window.run_until_parked();
        assert_eq!(window.debug_bounds("after").unwrap().top(), px(0.));
    }
}
