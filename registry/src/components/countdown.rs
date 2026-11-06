//! A deadline display driven by a monotonic clock, supporting large display reels and compact inline pills.
use crate::primitives::{
    control::UiLanguage,
    family::{Paint, Slide, measure},
};
use gpui::{prelude::*, *};
use std::time::Duration;
use web_time::Instant;

/// Emitted once when the timer reaches zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CountdownFinished;

/// Display density for a countdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CountdownVariant {
    #[default]
    Large,
    Compact,
}

struct PairReel {
    shown: Option<[char; 2]>,
    previous: [char; 2],
    slide: Slide,
}
impl Default for PairReel {
    fn default() -> Self {
        Self {
            shown: None,
            previous: ['0', '0'],
            slide: Slide::new(1.),
        }
    }
}
impl PairReel {
    fn render(&mut self, value: u64, font_size: f32, line: f32, w: &Window, cx: &App) -> Div {
        let formatted = format!("{:02}", value.min(99));
        let mut chars = formatted.chars();
        let next = [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')];
        if self.shown != Some(next)
            && let Some(prev) = self.shown.replace(next)
        {
            self.previous = prev;
            self.slide.reset(0.);
            self.slide.duration(1., 0.26, cx);
        }
        let h = line;
        let travel = self.slide.get(w, cx).clamp(0., 1.);
        let cell_w = measure(&"0".into(), font_size, w)
            .ceil()
            .max(font_size * 0.58);
        let mut row = div()
            .flex()
            .items_center()
            .h(px(h))
            .text_size(px(font_size))
            .line_height(px(h))
            .font_weight(FontWeight::MEDIUM);
        for (i, digit) in next.iter().enumerate() {
            let cur = digit.to_string();
            let prev = self.previous[i].to_string();
            if travel >= 0.999 || cur == prev {
                row = row.child(
                    div()
                        .w(px(cell_w))
                        .h(px(h))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(cur),
                );
            } else {
                row = row.child(
                    div()
                        .relative()
                        .w(px(cell_w))
                        .h(px(h))
                        .overflow_hidden()
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .top(px(-travel * font_size))
                                .opacity(1. - travel)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(prev),
                        )
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .top(px((1. - travel) * font_size))
                                .opacity(travel)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(cur),
                        ),
                );
            }
        }
        row
    }
}

/// A countdown whose host supplies the initial duration.
pub struct Countdown {
    label: SharedString,
    complete_label: SharedString,
    variant: CountdownVariant,
    deadline: Instant,
    epoch: u64,
    visible: bool,
    timer: Option<Task<()>>,
    finished: bool,
    transition: Slide,
    reels: [PairReel; 4],
}
impl EventEmitter<CountdownFinished> for Countdown {}
impl Countdown {
    pub fn new(label: impl Into<SharedString>, duration: Duration, cx: &mut Context<Self>) -> Self {
        let now = cx.background_executor().now();
        let mut result = Self {
            label: label.into(),
            complete_label: UiLanguage::current(cx).text("Live now", "现已发布").into(),
            variant: CountdownVariant::Large,
            deadline: now.checked_add(duration).unwrap_or(now),
            epoch: 0,
            visible: true,
            timer: None,
            finished: false,
            transition: Slide::snappy(0.),
            reels: Default::default(),
        };
        result.schedule(cx);
        result
    }
    pub fn with_variant(mut self, variant: CountdownVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn with_complete_label(mut self, label: impl Into<SharedString>) -> Self {
        self.complete_label = label.into();
        self
    }
    pub fn remaining(&self, cx: &App) -> Duration {
        self.deadline
            .saturating_duration_since(cx.background_executor().now())
    }
    pub fn reset(&mut self, duration: Duration, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        self.deadline = now.checked_add(duration).unwrap_or(now);
        self.finished = false;
        self.transition.reset(0.);
        self.schedule(cx);
        cx.notify();
    }
    /// The host reports viewport/document visibility. Time still advances while hidden.
    /// Hidden timers sleep until their deadline instead of waking for display updates.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        if !self.finished && self.remaining(cx).is_zero() {
            self.complete(cx);
        } else {
            self.schedule(cx);
        }
        if visible {
            cx.notify();
        }
    }
    fn complete(&mut self, cx: &mut Context<Self>) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.timer = None;
        if self.visible {
            self.transition.duration(1., 0.24, cx);
        } else {
            self.transition.reset(1.);
        }
        cx.emit(CountdownFinished);
        if self.visible {
            cx.notify();
        }
    }
    fn schedule(&mut self, cx: &mut Context<Self>) {
        self.timer = None;
        self.epoch = self.epoch.wrapping_add(1);
        if self.finished {
            return;
        }
        let remaining = self.remaining(cx);
        let delay = if self.visible && !remaining.is_zero() {
            let nanos = remaining.subsec_nanos();
            let next_second = if nanos == 0 {
                Duration::from_secs(1)
            } else {
                Duration::from_nanos(u64::from(nanos))
            };
            remaining.min(next_second)
        } else {
            remaining
        };
        let epoch = self.epoch;
        self.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |s, cx| {
                if s.epoch != epoch || s.finished {
                    return;
                }
                if s.remaining(cx).is_zero() {
                    s.complete(cx);
                } else {
                    if s.visible {
                        cx.notify();
                    }
                    s.schedule(cx);
                }
            });
        }));
    }
}
impl Render for Countdown {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let remaining = self.remaining(cx);
        let seconds = remaining
            .as_secs()
            .saturating_add(u64::from(remaining.subsec_nanos() > 0));
        let done = self.finished || seconds == 0;
        let language = UiLanguage::current(cx);
        let text = if done {
            format!("{}: {}", self.label, self.complete_label)
        } else if seconds >= 60 {
            format!(
                "{}: {} {} {}",
                self.label,
                language.text("remaining", "剩余"),
                seconds / 60,
                language.text("minutes", "分钟")
            )
        } else {
            format!(
                "{}: {} {} {}",
                self.label,
                language.text("remaining", "剩余"),
                seconds,
                language.text("seconds", "秒")
            )
        };
        if self.variant == CountdownVariant::Compact {
            let mut pill = div()
                .id("countdown")
                .role(Role::Timer)
                .aria_label(text)
                .min_h(px(32.))
                .px(px(12.))
                .rounded(radius)
                .border_1()
                .border_color(if done {
                    p.success.opacity(0.35)
                } else {
                    p.line
                })
                .bg(if done {
                    p.panel.blend(p.success.opacity(0.09))
                } else {
                    p.panel
                })
                .text_color(if done { p.success } else { p.ink })
                .text_size(px(14.))
                .line_height(px(18.2))
                .flex()
                .items_center()
                .gap(px(5.6));
            if done {
                pill = pill
                    .child(div().size(px(8.)).rounded_full().bg(p.success))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.complete_label.clone()),
                    );
            } else {
                let d = seconds / 86400;
                let h = seconds / 3600 % 24;
                let m = seconds / 60 % 60;
                let s = seconds % 60;
                let mut row = div().flex().items_baseline().gap(px(6.));
                if d > 0 {
                    row = row.child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(1.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.reels[0].render(d, 14., 18.2, w, cx))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(18.2))
                                    .text_color(p.faint)
                                    .child(language.text("d", "天")),
                            ),
                    );
                }
                for (idx, (val, unit)) in [
                    (h, language.text("h", "时")),
                    (m, language.text("m", "分")),
                    (s, language.text("s", "秒")),
                ]
                .into_iter()
                .enumerate()
                {
                    row = row.child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(1.))
                            .child(self.reels[idx + 1].render(val, 14., 18.2, w, cx))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(18.2))
                                    .text_color(p.faint)
                                    .child(unit),
                            ),
                    );
                }
                pill = pill.child(row);
            }
            return pill;
        }
        let mut root = div()
            .id("countdown")
            .role(Role::Timer)
            .aria_label(text)
            .flex()
            .items_start()
            .justify_center()
            .text_color(p.ink);
        if done {
            root = root
                .gap(px(14.4))
                .items_center()
                .child(div().size(px(11.)).rounded_full().bg(p.success))
                .child(
                    div()
                        .text_size(px(36.))
                        .line_height(px(39.6))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(self.complete_label.clone()),
                )
        } else {
            let values = [
                (language.text("Days", "天"), seconds / 86400),
                (language.text("Hours", "时"), seconds / 3600 % 24),
                (language.text("Minutes", "分"), seconds / 60 % 60),
                (language.text("Seconds", "秒"), seconds % 60),
            ];
            for (i, (unit, value)) in values.into_iter().enumerate() {
                if i > 0 {
                    root = root.child(
                        div()
                            .px(px(4.8))
                            .relative()
                            .top(px(-2.4))
                            .text_size(px(40.))
                            .line_height(px(40.))
                            .text_color(p.faint.opacity(0.6))
                            .child(":"),
                    );
                }
                root = root.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(6.))
                        .child(self.reels[i].render(value, 40., 40., w, cx))
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(12.))
                                .text_color(p.faint)
                                .child(unit),
                        ),
                );
            }
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{Countdown, CountdownFinished};
    use gpui::TestAppContext;
    use std::{cell::Cell, rc::Rc, time::Duration};

    #[test]
    fn hidden_countdown_only_wakes_at_deadline_and_finishes_once() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) =
            app.add_window_view(|_, cx| Countdown::new("Launch", Duration::from_secs(10), cx));
        let count = Rc::new(Cell::new(0));
        let events = count.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&view, move |_, _: &CountdownFinished, _| {
                events.set(events.get() + 1)
            })
        });
        w.update(|_, cx| view.update(cx, |v, cx| v.set_visible(false, cx)));
        w.run_until_parked();
        let epoch = w.update(|_, cx| view.read(cx).epoch);
        w.executor().advance_clock(Duration::from_secs(4));
        w.run_until_parked();
        assert_eq!(w.update(|_, cx| view.read(cx).epoch), epoch);
        assert_eq!(count.get(), 0);
        w.executor().advance_clock(Duration::from_secs(7));
        w.run_until_parked();
        assert_eq!(count.get(), 1);
        w.update(|_, cx| view.update(cx, |v, cx| v.set_visible(true, cx)));
        w.run_until_parked();
        assert_eq!(count.get(), 1);
    }
}
