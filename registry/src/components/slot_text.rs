//! Rolling text for short counters and values.
use crate::primitives::family::{Paint, PhaseClock, Slide, disabled_motion, measure};
use gpui::{prelude::*, *};
use unicode_segmentation::UnicodeSegmentation;

struct Reel {
    target: SharedString,
    from: f32,
    to: f32,
    width: Slide,
    measured_width: Option<f32>,
    exiting: bool,
}

fn digit(part: &str) -> Option<u8> {
    (part.len() == 1)
        .then(|| part.as_bytes()[0])
        .filter(u8::is_ascii_digit)
        .map(|value| value - b'0')
}

/// Source landing curve: cubic-bezier(0.12, 0.8, 0.16, 1).
fn landing(x: f32) -> f32 {
    if x <= 0. || x >= 1. {
        return x.clamp(0., 1.);
    }
    let (x1, y1, x2, y2) = (0.12, 0.8, 0.16, 1.);
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..16 {
        let t = (lo + hi) * 0.5;
        let v = 1. - t;
        let at = 3. * x1 * v * v * t + 3. * x2 * v * t * t + t * t * t;
        if at < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    let v = 1. - t;
    3. * y1 * v * v * t + 3. * y2 * v * t * t + t * t * t
}

fn reel_time(duration: f32, stagger: f32, index: usize) -> f32 {
    duration + index as f32 * stagger
}

/// Match digit columns from the right, without treating separators as digits.
/// Prefix/suffix symbols keep their identity; interior separators match by the
/// count of digits following them, so grouping changes do not shift columns.
fn match_reels(old: &[&str], new: &[&str]) -> Vec<Option<usize>> {
    let old_digits: Vec<_> = old
        .iter()
        .enumerate()
        .filter_map(|(i, c)| digit(c).map(|_| i))
        .collect();
    let new_digits: Vec<_> = new
        .iter()
        .enumerate()
        .filter_map(|(i, c)| digit(c).map(|_| i))
        .collect();
    let mut result = vec![None; new.len()];
    let mut used = vec![false; old.len()];
    for (old_i, new_i) in old_digits.iter().rev().zip(new_digits.iter().rev()) {
        result[*new_i] = Some(*old_i);
        used[*old_i] = true;
    }
    for (new_i, part) in new
        .iter()
        .enumerate()
        .filter(|(_, part)| digit(part).is_none())
    {
        let before = new_digits.iter().filter(|index| **index < new_i).count();
        let after = new_digits.len() - before;
        let matched = old.iter().enumerate().find(|(old_i, value)| {
            if used[*old_i] || *value != part || digit(value).is_some() {
                return false;
            }
            let old_before = old_digits.iter().filter(|index| **index < *old_i).count();
            let old_after = old_digits.len() - old_before;
            if before == 0 {
                old_before == 0
            } else if after == 0 {
                old_after == 0
            } else {
                old_before > 0 && old_after == after
            }
        });
        if let Some((old_i, _)) = matched {
            result[new_i] = Some(old_i);
            used[old_i] = true;
        }
    }
    result
}

pub struct SlotText {
    value: SharedString,
    reels: Vec<Reel>,
    clock: PhaseClock,
    duration: f32,
    stagger: f32,
    spins: u8,
    font_size: f32,
}
impl SlotText {
    pub fn new(value: impl Into<SharedString>) -> Self {
        let value = value.into();
        let reels = value
            .graphemes(true)
            .map(|s| {
                let n = digit(s).unwrap_or(0) as f32;
                Reel {
                    target: s.to_owned().into(),
                    from: n,
                    to: n,
                    width: Slide::new(0.),
                    measured_width: None,
                    exiting: false,
                }
            })
            .collect();
        Self {
            value,
            reels,
            clock: PhaseClock::default(),
            duration: 0.9,
            stagger: 0.07,
            spins: 1,
            font_size: 22.,
        }
    }
    pub fn with_duration(mut self, seconds: f32) -> Self {
        if seconds.is_finite() {
            self.duration = seconds.clamp(0.1, 8.);
        }
        self
    }
    pub fn with_stagger(mut self, seconds: f32) -> Self {
        if seconds.is_finite() {
            self.stagger = seconds.clamp(0., 0.3);
        }
        self
    }
    pub fn with_spins(mut self, spins: u8) -> Self {
        self.spins = spins.min(5);
        self
    }
    /// Set the font size in logical pixels; reel geometry scales with it.
    pub fn with_font_size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.font_size = size.clamp(8., 160.);
        }
        self
    }
    pub fn value(&self) -> &str {
        self.value.as_ref()
    }
    pub fn set_value(&mut self, value: impl Into<SharedString>, cx: &mut Context<Self>) {
        let value = value.into();
        if self.value == value {
            return;
        }
        let age = self.clock.age(cx);
        let parts = value.graphemes(true).collect::<Vec<_>>();
        let old_parts = self
            .reels
            .iter()
            .map(|reel| reel.target.as_ref())
            .collect::<Vec<_>>();
        let matches = match_reels(&old_parts, &parts);
        let mut next = Vec::with_capacity(parts.len());
        for (index, part) in parts.iter().enumerate() {
            let previous = matches[index]
                .and_then(|old_index| self.reels.get(old_index).map(|reel| (old_index, reel)));
            let new_digit = digit(part);
            let old_digit = previous.and_then(|(_, reel)| digit(&reel.target));
            let (from, to) = if let (Some(target), Some(_)) = (new_digit, old_digit) {
                let (old_index, reel) = previous.unwrap();
                let progress = landing(
                    (age / reel_time(self.duration, self.stagger, old_index)).clamp(0., 1.),
                );
                let current = reel.from + (reel.to - reel.from) * progress;
                let base = current.floor();
                let distance = (target as f32 - base.rem_euclid(10.)).rem_euclid(10.);
                let to = if reel.target.as_ref() != *part {
                    base + distance + f32::from(self.spins) * 10.
                } else {
                    reel.to.max(current)
                };
                (current, to)
            } else {
                let n = new_digit.unwrap_or(0) as f32;
                (n, n)
            };
            let (width, measured_width) = previous
                .map_or((Slide::new(0.), Some(0.)), |(_, reel)| {
                    (reel.width, reel.measured_width)
                });
            next.push(Reel {
                target: (*part).to_owned().into(),
                from,
                to,
                width,
                measured_width,
                exiting: false,
            });
        }
        let claimed = matches.iter().flatten().copied().collect::<Vec<_>>();
        for (index, reel) in self.reels.iter().enumerate() {
            if claimed.contains(&index) {
                continue;
            }
            let mut gone = Reel {
                target: reel.target.clone(),
                from: reel.from,
                to: reel.to,
                width: reel.width,
                measured_width: Some(0.),
                exiting: true,
            };
            if !reel.exiting {
                gone.width.duration(0., 0.35, cx);
            }
            next.push(gone);
        }
        self.value = value;
        self.reels = next;
        self.clock.restart(cx);
        cx.notify();
    }
}
impl Render for SlotText {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let age = if reduced {
            f32::INFINITY
        } else {
            self.clock.time(
                reel_time(
                    self.duration,
                    self.stagger,
                    self.reels.len().saturating_sub(1),
                ),
                w,
                cx,
            )
        };
        let window = self.font_size * 1.15;
        let pitch = self.font_size * 1.6;
        let pad = self.font_size * 0.225;
        let bleed_y = self.font_size * 0.22;
        let bleed_x = self.font_size * 0.08;
        let mut row = div()
            .id("slot-text")
            .role(Role::Label)
            .aria_label(self.value.clone())
            .flex()
            .items_start()
            .text_color(p.ink)
            .text_size(px(self.font_size))
            .line_height(px(window));
        self.reels.retain(|reel| !reel.exiting || reel.width.value(cx) > 0.5);
        for (index, reel) in self.reels.iter_mut().enumerate() {
            let target_width = if reel.exiting {
                0.
            } else {
                measure(&reel.target, self.font_size, w).max(1.)
            };
            if !reel.exiting {
                match reel.measured_width {
                    None => reel.width.reset(target_width),
                    Some(old) if (old - target_width).abs() > 0.01 => reel.width.duration(
                        target_width,
                        reel_time(self.duration, self.stagger, index).min(0.6),
                        cx,
                    ),
                    _ => {}
                }
                reel.measured_width = Some(target_width);
            }
            let width = reel.width.get(w, cx).max(0.);
            let mut cell = div().relative().w(px(width))
                .h(px(window))
                .flex_none();
            if digit(&reel.target).is_some() {
                let progress = if reduced {
                    1.
                } else {
                    landing((age / reel_time(self.duration, self.stagger, index)).clamp(0., 1.))
                };
                let number = reel.from + (reel.to - reel.from) * progress;
                let floor = number.floor();
                let digit = floor.rem_euclid(10.) as u8;
                let fraction = number - floor;
                let strip = bleed_y - pad;
                cell = cell.child(
                    div()
                        .absolute()
                        .top(px(-bleed_y))
                        .left(px(-bleed_x))
                        .right(px(-bleed_x))
                        .bottom(px(-bleed_y))
                        .overflow_hidden()
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(strip - fraction * pitch))
                                .h(px(pitch))
                                .pt(px(pad))
                                .child(digit.to_string()),
                        )
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(strip + (1. - fraction) * pitch))
                                .h(px(pitch))
                                .pt(px(pad))
                                .child(((digit + 1) % 10).to_string()),
                        ),
                );
            } else {
                cell = cell.child(reel.target.clone());
            }
            row = row.child(cell);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::{landing, match_reels, reel_time, SlotText};
    #[test]
    fn decimal_columns_stay_anchored_when_digit_count_or_grouping_changes() {
        assert_eq!(
            match_reels(&["9", "9"], &["1", "0", "0"]),
            vec![None, Some(0), Some(1)]
        );
        assert_eq!(
            match_reels(&["9", "9", "9"], &["1", ",", "0", "0", "0"]),
            vec![None, None, Some(0), Some(1), Some(2)]
        );
        assert_eq!(
            match_reels(&["$", "9", "9", "9"], &["$", "1", ",", "0", "0", "0"]),
            vec![Some(0), None, None, Some(1), Some(2), Some(3)]
        );
    }
    #[test]
    fn reels_use_the_source_timing() {
        let slot = SlotText::new("12,480");
        assert!((slot.duration - 0.9).abs() < 1e-6);
        assert!((slot.stagger - 0.07).abs() < 1e-6);
        assert!((reel_time(0.9, 0.07, 3) - 1.11).abs() < 1e-6);
        assert_eq!(landing(0.), 0.);
        assert_eq!(landing(1.), 1.);
        assert!(landing(0.5) > 0.5);
    }
    #[test]
    fn grouping_separator_tracks_the_digits_following_it() {
        assert_eq!(
            match_reels(&["1", ",", "0", "0", "0"], &["1", "0", ",", "0", "0", "0"]),
            vec![None, Some(0), Some(1), Some(2), Some(3), Some(4)]
        );
    }
}
