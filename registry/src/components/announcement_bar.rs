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
    Surface,
    #[default]
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
            tone: AnnouncementTone::Inverted,
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
                    if s.rotation_ticks >= 7 {
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
        let mut root = div()
            .w_full()
            .max_h(px(120. * visibility))
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
        let bg = if inverted { p.bright } else { p.panel };
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
        let mut next = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .py(px(6.))
            .opacity(opacity)
            .child(div().text_color(fg).child(message.message.clone()));
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
                    .child(div().font_weight(FontWeight::MEDIUM).text_color(fg).child(time_str)),
            );
        }
        if let Some(action) = message.action.clone() {
            let id = message.id.clone();
            next = next.child(
                face(
                    "announcement-action",
                    action.clone(),
                    28.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .gap(px(4.))
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
                    .justify_center()
                    .opacity(1. - opacity)
                    .text_color(fg)
                    .child(previous.clone()),
            );
        }
        let mut controls = div().flex().items_center().gap(px(2.));
        if self.messages.len() > 1 {
            let progress = 1. - (self.rotation_ticks as f32 / 7.).clamp(0., 1.);
            let next_label = tr(cx, "Next announcement", "下一条公告");
            controls = controls.child(
                face(
                    "announcement-next",
                    next_label,
                    32.,
                    16.,
                    Tone::Clear,
                    cx,
                )
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
        root = root.child(
            div()
                .id("announcement")
                .track_focus(&self.focus)
                .tab_index(-1)
                .flex()
                .items_center()
                .gap(px(12.))
                .w_full()
                .min_h(px(44.))
                .bg(bg)
                .when(!inverted, |d| d.border_b_1().border_color(p.line))
                .pl(px(16.))
                .pr(px(8.))
                .py(px(6.))
                .text_size(px(14.))
                .text_color(fg)
                .on_hover(cx.listener(|s, hover, _, _| s.paused = *hover))
                .child(contents)
                .child(controls),
        );
        root
    }
}
