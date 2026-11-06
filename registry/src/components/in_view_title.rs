//! Reveal a section heading once it becomes substantially visible.
use crate::primitives::family::{Paint, PhaseClock, disabled_motion, ease, standard_ease};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TitleEntrance {
    /// Whole title rises `0.9em` over 0.8s. Matches the source `word` motion on one line.
    Rise,
    Fade,
    /// Source `word`: each word rises `0.9em`.
    Words,
    /// Source `line`: each line rises `112%`.
    Lines,
    /// Source default. Each word rises `0.2em` and fades in.
    #[default]
    Blur,
}
pub struct InViewTitle {
    text: SharedString,
    once: bool,
    entered: bool,
    visible: bool,
    entrance: TitleEntrance,
    heading_level: usize,
    host_visible: bool,
    settled: bool,
    clock: PhaseClock,
}
impl InViewTitle {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            once: true,
            entered: false,
            visible: false,
            entrance: TitleEntrance::Blur,
            heading_level: 2,
            host_visible: true,
            settled: false,
            clock: PhaseClock::default(),
        }
    }
    pub fn with_once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }
    pub fn with_entrance(mut self, entrance: TitleEntrance) -> Self {
        self.entrance = entrance;
        self
    }
    /// Select an accessible heading level from 1 through 6.
    pub fn with_heading_level(mut self, level: usize) -> Self {
        self.heading_level = level.clamp(1, 6);
        self
    }
    /// Disable animation work for a mounted but hidden host.
    pub fn set_host_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.host_visible != visible {
            self.host_visible = visible;
            cx.notify();
        }
    }
    /// Restart the entrance, for a replay control.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.entered = true;
        self.settled = false;
        self.clock.restart(cx);
        cx.notify();
    }
    /// Hosts with their own scroll viewport can explicitly report visibility.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if visible == self.visible {
            return;
        }
        self.visible = visible;
        if visible {
            self.entered = true;
            self.settled = false;
            self.clock.restart(cx);
        } else if !self.once {
            self.settled = false;
            self.clock.restart(cx);
        }
        cx.notify();
    }
}
fn visibility(top: f32, height: f32, viewport: f32) -> (bool, bool) {
    let height = height.max(1.);
    let shown = ((top + height).min(viewport) - top.max(0.)).max(0.);
    (shown / height >= 0.45, top + height <= 0.)
}

/// Source `reveal()`: move uses ease-enter for `settle` seconds, opacity uses ease-standard for `settle * 0.56`.
/// Hiding is one 0.18s standard fade with no stagger.
fn tracks(age: f32, delay: f32, settle: f32, hiding: bool) -> (f32, f32) {
    if hiding {
        let gone = standard_ease((age / 0.18).clamp(0., 1.));
        return (1. - gone, 1. - gone);
    }
    let local = age - delay;
    (
        ease((local / settle).clamp(0., 1.)),
        standard_ease((local / (settle * 0.56)).clamp(0., 1.)),
    )
}

impl Render for InViewTitle {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let hiding = self.entered && !self.visible && !self.once;
        let parts: Vec<String> = match self.entrance {
            TitleEntrance::Words | TitleEntrance::Blur => self
                .text
                .split(' ')
                .filter(|word| !word.is_empty())
                .map(str::to_string)
                .collect(),
            TitleEntrance::Lines => self
                .text
                .split('\n')
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect(),
            TitleEntrance::Rise | TitleEntrance::Fade => vec![self.text.to_string()],
        };
        let count = parts.len().max(1);
        let (settle, step) = match self.entrance {
            TitleEntrance::Lines => (0.86, 0.08),
            TitleEntrance::Words | TitleEntrance::Blur => (0.8, (0.4 / count as f32).min(0.06)),
            TitleEntrance::Rise | TitleEntrance::Fade => (0.8, 0.),
        };
        let end = if hiding {
            0.18
        } else {
            (count.saturating_sub(1) as f32) * step + settle
        };
        let age = if reduced || self.settled {
            end
        } else if self.host_visible && w.is_visible() && (self.visible || hiding) {
            self.clock.time(end, w, cx)
        } else {
            0.
        };
        let style = w.text_style();
        let font_px = f32::from(style.font_size.to_pixels(w.rem_size()));
        let line_px = f32::from(style.line_height.to_pixels(style.font_size, w.rem_size()));
        let travel = match self.entrance {
            TitleEntrance::Blur => 0.2 * font_px,
            TitleEntrance::Lines => 1.12 * line_px,
            TitleEntrance::Fade => 0.,
            TitleEntrance::Rise | TitleEntrance::Words => 0.9 * font_px,
        };
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut row = div()
            .relative()
            .flex()
            .flex_wrap()
            .text_color(p.ink)
            .font_weight(FontWeight::MEDIUM)
            .when(self.entrance == TitleEntrance::Lines, |v| v.flex_col());
        let clip = matches!(
            self.entrance,
            TitleEntrance::Words | TitleEntrance::Lines | TitleEntrance::Rise
        );
        for (i, part) in parts.iter().enumerate() {
            let (rise, fade) = if reduced || self.settled {
                (1., 1.)
            } else {
                tracks(age, i as f32 * step, settle, hiding)
            };
            let word = div()
                .relative()
                .top(px(travel * (1. - rise)))
                .opacity(fade)
                .child(part.clone());
            let (pad_top, pad_x, pad_bottom) = if self.entrance == TitleEntrance::Lines {
                (0.06, 0.12, 0.18)
            } else {
                (0.08, 0.14, 0.2)
            };
            row = row.child(if clip {
                div()
                    .overflow_hidden()
                    .pt(px(pad_top * font_px))
                    .px(px(pad_x * font_px))
                    .pb(px(pad_bottom * font_px))
                    .mt(px(-pad_top * font_px))
                    .mx(px(-pad_x * font_px))
                    .mb(px(-pad_bottom * font_px))
                    .child(word)
            } else {
                word
            });
            if self.entrance != TitleEntrance::Lines && i + 1 < parts.len() {
                row = row.child(" ");
            }
        }
        let content = row;
        div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.first() {
                    let (on_screen, past) = visibility(
                        f32::from(bounds.origin.y),
                        f32::from(bounds.size.height),
                        f32::from(viewport.height),
                    );
                    let _ = weak.update(cx, |s, cx| {
                        if past && !s.entered {
                            s.entered = true;
                            s.visible = true;
                            s.settled = true;
                            cx.notify();
                        } else if !s.once || !s.entered {
                            s.set_visible(on_screen, cx);
                        }
                    });
                }
            })
            .id("in-view-title")
            .role(Role::Heading)
            .aria_level(self.heading_level)
            .aria_label(self.text.replace('\n', " "))
            .child(content)
    }
}

#[cfg(test)]
mod tests {
    use super::{tracks, visibility};
    #[test]
    fn reveal_tracks_follow_the_source_split() {
        assert_eq!(tracks(0., 0., 0.8, false), (0., 0.));
        assert_eq!(tracks(0.8, 0., 0.8, false), (1., 1.));
        let (rise, fade) = tracks(0.8 * 0.56, 0., 0.8, false);
        assert_eq!(fade, 1.);
        assert!(rise < 1.);
        assert_eq!(tracks(0.18, 0.4, 0.8, true), (0., 0.));
    }
    #[test]
    fn reveals_after_45_percent_and_recognizes_past_titles() {
        assert_eq!(visibility(55., 100., 100.), (true, false));
        assert_eq!(visibility(56., 100., 100.), (false, false));
        assert_eq!(visibility(-101., 100., 100.), (false, true));
    }
}
