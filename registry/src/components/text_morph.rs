//! Morph a short label with stable glyphs and a measured spring width.
use crate::primitives::family::{Paint, Slide};
use gpui::{prelude::*, *};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

struct GlyphMotion {
    text: SharedString,
    occurrence: usize,
    position: Slide,
    opacity: Slide,
    top: Slide,
    present: bool,
}
/// Keeps character identity and animated position across rapid value changes.
pub struct TextMorph {
    current: SharedString,
    measured_text: SharedString,
    measured_width: Option<f32>,
    width: Slide,
    glyphs: Vec<GlyphMotion>,
    in_view: bool,
    host_visible: bool,
}
impl TextMorph {
    pub fn new(text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            current: text.clone(),
            measured_text: text,
            measured_width: None,
            width: Slide::new(0.),
            glyphs: Vec::new(),
            in_view: true,
            host_visible: true,
        }
    }
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        let text = text.into();
        if text != self.current {
            self.current = text;
            cx.notify();
        }
    }
    /// Pause frame requests when the containing preview is hidden.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.host_visible != visible {
            self.host_visible = visible;
            cx.notify();
        }
    }
    pub fn text(&self) -> &str {
        self.current.as_ref()
    }
}
impl Render for TextMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut style = w.text_style();
        style.font_weight = FontWeight::MEDIUM;
        let shaped = w.text_system().shape_line(
            self.current.clone(),
            px(20.),
            &[style.to_run(self.current.len())],
            None,
        );
        let target: f32 = shaped.width.into();
        let changed = self.measured_text != self.current;
        let first = self.measured_width.is_none();
        let font_changed = !changed
            && self
                .measured_width
                .is_some_and(|v| (v - target).abs() > 0.5);
        if first || font_changed {
            self.width.reset(target);
        } else if changed {
            self.width.duration(target, 0.42, cx);
        }
        if first || changed || font_changed {
            let mut seen = HashMap::<String, usize>::new();
            let mut old = std::mem::take(&mut self.glyphs);
            let mut next = Vec::new();
            let mut entering = 0usize;
            for (index, grapheme) in self.current.grapheme_indices(true) {
                let count = seen.entry(grapheme.to_owned()).or_default();
                let occurrence = *count;
                *count += 1;
                let target_x: f32 = shaped.x_for_index(index).into();
                if let Some(i) = old
                    .iter()
                    .position(|g| g.text.as_ref() == grapheme && g.occurrence == occurrence)
                {
                    let mut glyph = old.remove(i);
                    glyph.present = true;
                    if font_changed {
                        glyph.position.reset(target_x);
                    } else {
                        glyph.position.duration(target_x, 0.42, cx);
                    }
                    next.push(glyph);
                } else {
                    let order = entering;
                    entering += 1;
                    let mut opacity = Slide::new(if first { 1. } else { 0. });
                    let mut top = Slide::new(if first { 0. } else { 0.14 * 20. });
                    if !first {
                        let delay = enter_delay(order);
                        opacity.delayed(1., 0.28, delay, cx);
                        top.delayed(0., 0.28, delay, cx);
                    }
                    next.push(GlyphMotion {
                        text: grapheme.to_owned().into(),
                        occurrence,
                        position: Slide::new(target_x),
                        opacity,
                        top,
                        present: true,
                    });
                }
            }
            for mut glyph in old {
                glyph.present = false;
                glyph.opacity.duration(0., 0.18, cx);
                glyph.top.duration(-2., 0.18, cx);
                next.push(glyph);
            }
            self.glyphs = next;
        }
        self.measured_width = Some(target);
        self.measured_text = self.current.clone();
        let visible = self.host_visible && self.in_view && w.is_visible();
        let width = if visible {
            self.width.get(w, cx)
        } else {
            self.width.value(cx)
        }
        .max(0.);
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut row = div()
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
            .id("text-morph")
            .role(Role::Label)
            .aria_label(self.current.clone())
            .relative()
            .w(px(width))
            .h(px(f32::from(
                w.text_style()
                    .line_height
                    .to_pixels(AbsoluteLength::Pixels(px(20.)), w.rem_size()),
            )))
            .text_size(px(20.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.ink)
            .whitespace_nowrap()
            .child(canvas(|_, _, _| (), |_, _, _, _| ()).absolute().inset_0());
        for glyph in &self.glyphs {
            let x = if visible {
                glyph.position.get(w, cx)
            } else {
                glyph.position.value(cx)
            };
            let opacity = if visible {
                glyph.opacity.get(w, cx)
            } else {
                glyph.opacity.value(cx)
            }
            .clamp(0., 1.);
            let top = if visible {
                glyph.top.get(w, cx)
            } else {
                glyph.top.value(cx)
            };
            if opacity > 0.001 {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(top))
                        .opacity(opacity)
                        .child(glyph.text.clone()),
                );
            }
        }
        self.glyphs
            .retain(|g| g.present || g.opacity.value(cx) > 0.001);
        row
    }
}

/// New letters wait 0.05s, then 0.032s per letter, and the stagger stops after eight.
fn enter_delay(order: usize) -> f32 {
    0.05 + order.min(8) as f32 * 0.032
}

#[cfg(test)]
mod tests {
    use super::enter_delay;

    #[test]
    fn new_letters_stagger_from_the_source_handoff() {
        assert!((enter_delay(0) - 0.05).abs() < 1e-6);
        assert!((enter_delay(1) - 0.082).abs() < 1e-6);
        assert_eq!(enter_delay(8), enter_delay(20));
    }
}
