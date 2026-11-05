//! Reveal a sentence as a short sequence of rising words.
use crate::primitives::family::{Paint, PhaseClock, disabled_motion, ease, measure, standard_ease};
use gpui::{prelude::*, *};

/// A one-shot text entrance. Newlines create separate lines.
pub struct TextReveal {
    text: SharedString,
    delay: f32,
    clock: PhaseClock,
    heading_level: Option<usize>,
    in_view: bool,
    host_visible: bool,
}
impl TextReveal {
    pub fn new(text: impl Into<SharedString>, cx: &App) -> Self {
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        Self {
            text: text.into(),
            delay: 0.,
            clock,
            heading_level: None,
            in_view: true,
            host_visible: true,
        }
    }
    /// Delay the first word, in seconds.
    pub fn with_delay(mut self, seconds: f32) -> Self {
        self.delay = seconds.max(0.);
        self
    }
    /// Expose this text as a heading in the page outline (levels 1 through 6).
    pub fn with_heading_level(mut self, level: usize) -> Self {
        self.heading_level = Some(level.clamp(1, 6));
        self
    }
    /// Hosts can pause rendering for a mounted but hidden preview.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.host_visible != visible {
            self.host_visible = visible;
            cx.notify();
        }
    }
    /// Replay the entrance with a new sentence.
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.text = text.into();
        self.clock.restart(cx);
        cx.notify();
    }
}
impl Render for TextReveal {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        let lines: Vec<Vec<String>> = self
            .text
            .split('\n')
            .map(|line| {
                line.split(' ')
                    .filter(|word| !word.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .collect();
        let count = lines.iter().map(Vec::len).sum::<usize>();
        // Source: step = min(stagger.word 0.04, considered 0.48 / word count).
        let step = (0.48 / count.max(1) as f32).min(0.04);
        let reduced = disabled_motion(cx);
        let last = count.saturating_sub(1) as f32;
        let last_line = lines.len().saturating_sub(1) as f32;
        let total = if reduced {
            0.16
        } else {
            self.delay + last * step + last_line * step + 0.76
        };
        let age = if self.host_visible && self.in_view && w.is_visible() {
            if reduced {
                let age = self.clock.age(cx);
                if age < 0.16 {
                    w.request_animation_frame();
                }
                age
            } else {
                self.clock.time(total, w, cx)
            }
        } else {
            self.clock.age(cx).min(total)
        };
        let font_px = 24.;
        let rise_px = 0.62 * font_px;
        let column = measure(&SharedString::from("0"), font_px, w) * 24.;
        let mut index = 0;
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut result = div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(b) = bounds.first() {
                    let visible = b.left() < viewport.width
                        && b.top() < viewport.height
                        && b.right() > px(0.)
                        && b.bottom() > px(0.);
                    let _ = weak.update(cx, |s, cx| {
                        if s.in_view != visible {
                            s.in_view = visible;
                            cx.notify();
                        }
                    });
                }
            })
            .id("text-reveal")
            .when_some(self.heading_level, |d, level| {
                d.role(Role::Heading).aria_level(level)
            })
            .when(self.heading_level.is_none(), |d| d.role(Role::Paragraph))
            .aria_label(self.text.replace('\n', " "))
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(column.max(1.)))
            .text_size(px(font_px))
            .text_color(paint.ink)
            .relative()
            .child(canvas(|_, _, _| (), |_, _, _, _| ()).absolute().inset_0());
        for (line_index, line) in lines.iter().enumerate() {
            let mut row = div().flex().flex_wrap().items_end();
            for (word_index, word) in line.iter().enumerate() {
                let start = self.delay + index as f32 * step + line_index as f32 * step;
                index += 1;
                let (rise, fade) = if reduced {
                    (1., standard_ease((age / 0.16).clamp(0., 1.)))
                } else {
                    let local = age - start;
                    (
                        ease((local / 0.76).clamp(0., 1.)),
                        standard_ease((local / 0.44).clamp(0., 1.)),
                    )
                };
                let pad_x = 0.16 * font_px;
                let pad_top = 0.1 * font_px;
                let pad_bottom = 0.24 * font_px;
                row = row.child(
                    div()
                        .pt(px(pad_top))
                        .px(px(pad_x))
                        .pb(px(pad_bottom))
                        .mt(px(-pad_top))
                        .mx(px(-pad_x))
                        .mb(px(-pad_bottom))
                        .child(
                            div()
                                .relative()
                                .top(px(rise_px * (1. - rise)))
                                .opacity(fade)
                                .child(word.clone()),
                        ),
                );
                if word_index + 1 < line.len() {
                    row = row.child(" ");
                }
            }
            result = result.child(row);
        }
        result
    }
}
