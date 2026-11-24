//! Soft-blurring content crossfade whose container glides smoothly to the next size.
//!
//! Swaps arbitrary content whenever `content_key` changes, cross-fading and
//! blurring outgoing and incoming layers while morphing height (or both width
//! and height) along a critically-damped spring. Swaps instantly when
//! [`Theme::reduced_motion`](crate::foundation::Theme::reduced_motion) is enabled.

use crate::{
    components::pro_chrome::{ProBtnStyle, ProPalette, pro_btn, pro_tactile},
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

type LayerBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Evaluate `cubic-bezier(0.22, 1.0, 0.36, 1.0)` on `t` in `0.0..=1.0`.
fn bx_ease_out(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    if t <= 0. || t >= 1. {
        return t;
    }
    let mut u = t;
    for _ in 0..6 {
        let inv = 1. - u;
        let x = 3. * inv * inv * u * 0.22 + 3. * inv * u * u * 0.36 + u * u * u;
        let dx = 3. * inv * inv * 0.22 + 6. * inv * u * (0.36 - 0.22) + 3. * u * u * (1. - 0.36);
        if dx.abs() < 1e-5 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0., 1.);
    }
    let inv = 1. - u;
    3. * inv * inv * u + 3. * inv * u * u + u * u * u
}

/// Evaluate CSS `ease-in` (`cubic-bezier(0.42, 0.0, 1.0, 1.0)`) on `t` in `0.0..=1.0`.
fn bx_ease_in(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    if t <= 0. || t >= 1. {
        return t;
    }
    let mut u = t;
    for _ in 0..6 {
        let inv = 1. - u;
        let x = 3. * inv * inv * u * 0.42 + 3. * inv * u * u + u * u * u;
        let dx = 3. * inv * inv * 0.42 + 6. * inv * u * (1. - 0.42);
        if dx.abs() < 1e-5 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0., 1.);
    }
    u * u * (3. - 2. * u)
}

/// Dimensions morphed by [`BlurCrossfade`] while swapping content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlurMorph {
    /// Glide container height to match incoming content while filling available width.
    #[default]
    Height,
    /// Glide both container width and height to wrap inline content (such as button labels).
    Both,
    /// Keep layout unconstrained and cross-fade in place without size morphing.
    None,
}

enum ContentSource {
    Once(AnyElement),
    Repeatable(LayerBuilder),
}

struct BxState {
    key: SharedString,
    measured: Rc<Cell<(f32, f32)>>,
    target_w: f32,
    target_h: f32,
    outgoing_size: (f32, f32),
    current_fn: Option<LayerBuilder>,
    outgoing_fn: Option<LayerBuilder>,
    w_slide: Slide,
    h_slide: Slide,
    drift_slide: Slide,
    clock: PhaseClock,
    swapping: bool,
}

impl BxState {
    fn new(key: SharedString, current_fn: Option<LayerBuilder>) -> Self {
        Self {
            key,
            measured: Rc::new(Cell::new((0., 0.))),
            target_w: 0.,
            target_h: 0.,
            outgoing_size: (0., 0.),
            current_fn,
            outgoing_fn: None,
            w_slide: Slide::new(0.),
            h_slide: Slide::new(0.),
            drift_slide: Slide::new(0.),
            clock: PhaseClock::default(),
            swapping: false,
        }
    }
}

/// Soft-blur crossfade container that morphs smoothly between content states.
///
/// # Examples
///
/// ```
/// use gpui::{div, prelude::*};
/// use kui::components::blur_crossfade::{BlurCrossfade, BlurMorph};
///
/// let label = BlurCrossfade::new("save-status", "saved", div().child("Saved"))
///     .with_morph(BlurMorph::Both)
///     .with_duration_ms(420.);
/// ```
#[derive(IntoElement)]
pub struct BlurCrossfade {
    id: ElementId,
    content_key: SharedString,
    source: ContentSource,
    duration_ms: f32,
    blur_px: f32,
    direction: f32,
    morph: BlurMorph,
    inline: bool,
}

impl BlurCrossfade {
    /// Create a `BlurCrossfade` keyed by `content_key` around a single GPUI element.
    pub fn new(
        id: impl Into<ElementId>,
        content_key: impl Into<SharedString>,
        child: impl IntoElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            content_key: content_key.into(),
            source: ContentSource::Once(child.into_any_element()),
            duration_ms: 420.,
            blur_px: 10.,
            direction: 0.,
            morph: BlurMorph::Height,
            inline: false,
        }
    }

    /// Create a `BlurCrossfade` with a repeatable view builder so both the leaving copy
    /// and the incoming copy render simultaneously with multi-tap soft-focus blur.
    pub fn from_fn(
        id: impl Into<ElementId>,
        content_key: impl Into<SharedString>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            content_key: content_key.into(),
            source: ContentSource::Repeatable(Rc::new(render)),
            duration_ms: 420.,
            blur_px: 10.,
            direction: 0.,
            morph: BlurMorph::Height,
            inline: false,
        }
    }

    /// Create an inline `BlurMorph::Both` text crossfade tailored for stateful button labels.
    pub fn label(
        id: impl Into<ElementId>,
        content_key: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        let text: SharedString = text.into();
        Self::from_fn(id, content_key, move |_, _| {
            div()
                .whitespace_nowrap()
                .child(text.clone())
                .into_any_element()
        })
        .with_morph(BlurMorph::Both)
        .with_inline(true)
    }

    /// Construct the interactive release-notes showcase card.
    pub fn demo(
        id: impl Into<ElementId>,
        cx: &mut Context<BlurCrossfadeShowcase>,
    ) -> BlurCrossfadeShowcase {
        BlurCrossfadeShowcase::new(id, cx)
    }

    /// Set swap and size-glide duration in milliseconds (`420.0` default).
    pub fn with_duration_ms(mut self, ms: f32) -> Self {
        if ms.is_finite() {
            self.duration_ms = ms.clamp(60., 4000.);
        }
        self
    }

    /// Set peak Gaussian blur radius in pixels at the outer ends of the swap (`10.0` default).
    pub fn with_blur_px(mut self, blur_px: f32) -> Self {
        if blur_px.is_finite() {
            self.blur_px = blur_px.clamp(0., 40.);
        }
        self
    }

    /// Choose which container axes glide to match incoming content (`BlurMorph::Height` default).
    pub fn with_morph(mut self, morph: BlurMorph) -> Self {
        self.morph = morph;
        if morph == BlurMorph::Both {
            self.inline = true;
        }
        self
    }

    /// Render as a shrink-wrapped inline box (`span` behavior inside buttons and pills).
    pub fn with_inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Shorthand for `.with_inline(true)`.
    pub fn inline(self) -> Self {
        self.with_inline(true)
    }

    /// Set directional horizontal drift bias (`-1.0` backward .. `+1.0` forward; `0.0` default).
    pub fn with_direction(mut self, direction: f32) -> Self {
        if direction.is_finite() {
            self.direction = direction.clamp(-1., 1.);
        }
        self
    }

    /// Configured swap duration in milliseconds.
    pub fn duration_ms(&self) -> f32 {
        self.duration_ms
    }

    /// Configured peak blur radius in pixels.
    pub fn blur_px(&self) -> f32 {
        self.blur_px
    }

    /// Configured directional drift bias (`-1.0..=1.0`).
    pub fn direction(&self) -> f32 {
        self.direction
    }

    /// Configured size morph mode.
    pub fn morph(&self) -> BlurMorph {
        self.morph
    }
}

struct BlurPass {
    scope_id: ElementId,
    tag: &'static str,
    opacity: f32,
    blur_radius: f32,
    scale_delta: f32,
    drift_x: f32,
    center_both: bool,
}

fn render_blurred_layer(
    pass: BlurPass,
    builder: &LayerBuilder,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let opacity = pass.opacity.clamp(0., 1.);
    if opacity <= 0.004 {
        return div().into_any_element();
    }
    if pass.blur_radius <= 0.25 && pass.scale_delta.abs() < 0.001 && pass.drift_x.abs() < 0.15 {
        let base = div()
            .id((pass.scope_id, pass.tag))
            .opacity(opacity)
            .child(builder(w, cx));
        return if pass.center_both {
            base.flex_none().whitespace_nowrap().into_any_element()
        } else {
            base.w_full().into_any_element()
        };
    }

    let r = pass.blur_radius * 0.36;
    let radial_push = pass.scale_delta * 75.0;
    let shift_x = pass.drift_x;
    let shift_y = -pass.scale_delta * 45.0;
    let taps: [(f32, f32, f32); 5] = [
        (shift_x, shift_y, 0.34),
        (shift_x - r - radial_push * 0.4, -r * 0.72 + shift_y, 0.165),
        (shift_x + r + radial_push * 0.4, -r * 0.72 + shift_y, 0.165),
        (
            shift_x - r * 0.85 - radial_push * 0.3,
            r * 0.72 + shift_y,
            0.165,
        ),
        (
            shift_x + r * 0.85 + radial_push * 0.3,
            r * 0.72 + shift_y,
            0.165,
        ),
    ];

    let mut stack = div().id((pass.scope_id.clone(), pass.tag)).relative();
    if pass.center_both {
        stack = stack.flex_none().whitespace_nowrap();
    } else {
        stack = stack.w_full();
    }

    for (idx, (dx, dy, weight)) in taps.into_iter().enumerate() {
        let tap_alpha = (opacity * weight * 1.18).clamp(0., 1.);
        if tap_alpha <= 0.003 {
            continue;
        }
        let tap_id: ElementId = (
            pass.scope_id.clone(),
            SharedString::from(format!("{}-{idx}", pass.tag)),
        )
            .into();
        if idx == 0 {
            stack = stack.child(
                div()
                    .id(tap_id)
                    .relative()
                    .left(px(dx))
                    .top(px(dy))
                    .opacity(tap_alpha)
                    .when(pass.center_both, |d| d.flex_none().whitespace_nowrap())
                    .when(!pass.center_both, |d| d.w_full())
                    .child(builder(w, cx)),
            );
        } else {
            stack = stack.child(
                div()
                    .id(tap_id)
                    .absolute()
                    .top(px(dy))
                    .left(px(dx))
                    .when(!pass.center_both, |d| d.right(px(-dx)))
                    .when(pass.center_both, |d| d.whitespace_nowrap())
                    .opacity(tap_alpha)
                    .child(builder(w, cx)),
            );
        }
    }
    stack.into_any_element()
}

impl RenderOnce for BlurCrossfade {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduced = disabled_motion(cx);
        let dur_secs = (self.duration_ms / 1000.).max(0.06);
        let out_secs = dur_secs * 0.8;
        let next_fn = match &self.source {
            ContentSource::Once(_) => None,
            ContentSource::Repeatable(f) => Some(f.clone()),
        };

        let init_key = self.content_key.clone();
        let init_fn = next_fn.clone();
        let state_id: ElementId = (self.id.clone(), "bx-state").into();
        let st = w.use_keyed_state(state_id, cx, |_, _| BxState::new(init_key, init_fn));

        let dir_bias = self.direction;
        let (age, swapping, anim_w, anim_h, drift_x, outgoing_fn, outgoing_size, measured_cell) =
            st.update(cx, |s, cx| {
                let (mw, mh) = s.measured.get();
                if s.target_w <= 0. && mw > 0. {
                    s.target_w = mw;
                    s.target_h = mh;
                    s.w_slide.reset(mw);
                    s.h_slide.reset(mh);
                }
                if s.key != self.content_key {
                    if mw > 0. && mh > 0. {
                        if (s.target_w - mw).abs() > 0.5 && !s.swapping {
                            s.w_slide.reset(mw);
                        }
                        if (s.target_h - mh).abs() > 0.5 && !s.swapping {
                            s.h_slide.reset(mh);
                        }
                        s.outgoing_size = (mw, mh);
                    }
                    s.key = self.content_key.clone();
                    s.outgoing_fn = s.current_fn.take();
                    s.current_fn = next_fn.clone();
                    if reduced {
                        s.swapping = false;
                        s.outgoing_fn = None;
                        s.drift_slide.reset(0.);
                    } else {
                        // Preserve horizontal velocity across rapid directional clicks!
                        let cur_drift = s.drift_slide.value(cx);
                        let impulse = (cur_drift * 0.55 + dir_bias * 11.0).clamp(-18.0, 18.0);
                        s.drift_slide.reset(impulse);
                        s.drift_slide.duration(0., dur_secs, cx);
                        s.swapping = true;
                        s.clock.restart(cx);
                    }
                } else if next_fn.is_some() {
                    s.current_fn = next_fn.clone();
                }

                let age = if s.swapping && !reduced {
                    s.clock.time(dur_secs, w, cx)
                } else {
                    dur_secs
                };
                let w_val = s.w_slide.get(w, cx);
                let h_val = s.h_slide.get(w, cx);
                let d_val = s.drift_slide.get(w, cx);
                let slides_settling = (w_val - s.target_w).abs() > 0.4
                    || (h_val - s.target_h).abs() > 0.4
                    || d_val.abs() > 0.25;
                if s.swapping && age >= dur_secs && !slides_settling {
                    s.swapping = false;
                    s.outgoing_fn = None;
                }
                (
                    age,
                    s.swapping && !reduced,
                    w_val,
                    h_val,
                    if reduced { 0. } else { d_val },
                    s.outgoing_fn.clone(),
                    s.outgoing_size,
                    s.measured.clone(),
                )
            });

        let center_both = self.morph == BlurMorph::Both;
        let t_in = if swapping {
            bx_ease_out((age / dur_secs).clamp(0., 1.))
        } else {
            1.0
        };
        let t_out = if swapping {
            bx_ease_in((age / out_secs).clamp(0., 1.))
        } else {
            1.0
        };

        let in_blur = if swapping {
            (self.blur_px + drift_x.abs() * 0.22) * (1. - t_in)
        } else {
            0.
        };
        let in_scale_delta = if swapping { -0.02 * (1. - t_in) } else { 0. };
        let out_blur = (self.blur_px + drift_x.abs() * 0.22) * t_out;
        let out_scale_delta = 0.02 * t_out;

        let incoming_el = match self.source {
            ContentSource::Repeatable(builder) => render_blurred_layer(
                BlurPass {
                    scope_id: self.id.clone(),
                    tag: "bx-in",
                    opacity: t_in,
                    blur_radius: in_blur,
                    scale_delta: in_scale_delta,
                    drift_x,
                    center_both,
                },
                &builder,
                w,
                cx,
            ),
            ContentSource::Once(el) => div()
                .relative()
                .left(px(if swapping { drift_x } else { 0. }))
                .top(px(if swapping { 1.5 * (1. - t_in) } else { 0. }))
                .opacity(t_in)
                .when(center_both, |d| d.flex_none().whitespace_nowrap())
                .when(!center_both, |d| d.w_full())
                .child(el)
                .into_any_element(),
        };

        let weak_st = st.downgrade();
        let morph = self.morph;
        let measure_wrap = div()
            .flex_none()
            .when(center_both, |d| d.whitespace_nowrap())
            .when(!center_both && !self.inline, |d| d.w_full())
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(b) = bounds.first() else {
                    return;
                };
                let nw = f32::from(b.size.width);
                let nh = f32::from(b.size.height);
                if nw <= 0. || nh <= 0. {
                    return;
                }
                measured_cell.set((nw, nh));
                let _ = weak_st.update(cx, |s, cx| {
                    let w_changed = (s.target_w - nw).abs() > 0.35;
                    let h_changed = (s.target_h - nh).abs() > 0.35;
                    if !w_changed && !h_changed {
                        return;
                    }
                    let first = s.target_w <= 0. && s.target_h <= 0.;
                    s.target_w = nw;
                    s.target_h = nh;
                    if first || reduced || morph == BlurMorph::None {
                        s.w_slide.reset(nw);
                        s.h_slide.reset(nh);
                    } else if s.swapping {
                        if morph == BlurMorph::Both && w_changed {
                            s.w_slide.duration(nw, dur_secs, cx);
                        } else {
                            s.w_slide.reset(nw);
                        }
                        if h_changed {
                            s.h_slide.duration(nh, dur_secs, cx);
                        }
                        window.request_animation_frame();
                    } else {
                        s.w_slide.reset(nw);
                        s.h_slide.reset(nh);
                    }
                });
            })
            .child(incoming_el);

        // Unconstrained axis wrapper so incoming natural width/height is measured cleanly
        // even while the outer box is gliding from the previous dimensions.
        let inner_flow = if center_both {
            div()
                .flex()
                .flex_row()
                .justify_center()
                .flex_none()
                .child(div().flex().flex_col().flex_none().child(measure_wrap))
        } else {
            div()
                .flex()
                .flex_col()
                .w_full()
                .flex_none()
                .child(measure_wrap)
        };

        let mut root = div().id(self.id.clone()).relative();
        if center_both || self.inline {
            root = root
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .flex_none();
        } else {
            root = root.flex().flex_col().w_full();
        }

        if swapping && morph != BlurMorph::None {
            if morph == BlurMorph::Both && anim_w > 0.5 {
                root = root.w(px(anim_w));
            }
            if anim_h > 0.5 {
                root = root.h(px(anim_h));
            }
            root = root.overflow_hidden();
        }

        if swapping && t_out < 0.996 {
            if let Some(out_builder) = outgoing_fn {
                let out_el = render_blurred_layer(
                    BlurPass {
                        scope_id: self.id.clone(),
                        tag: "bx-out",
                        opacity: 1. - t_out,
                        blur_radius: out_blur,
                        scale_delta: out_scale_delta,
                        drift_x: -dir_bias * 9.0 * t_out,
                        center_both,
                    },
                    &out_builder,
                    w,
                    cx,
                );
                root = root.child(
                    div()
                        .absolute()
                        .top_0()
                        .when(center_both, |d| {
                            d.left_0().right_0().flex().flex_row().justify_center()
                        })
                        .when(!center_both, |d| d.left_0().right_0())
                        .child(out_el),
                );
            } else if outgoing_size.0 > 1. && outgoing_size.1 > 1. {
                let dark = Theme::global(cx).mode == ThemeMode::Dark;
                let veil = if dark {
                    hsla(0., 0., 1., 0.045 * (1. - t_out))
                } else {
                    hsla(0., 0., 0., 0.035 * (1. - t_out))
                };
                let (ow, oh) = outgoing_size;
                root = root.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(px(ow))
                        .h(px(oh))
                        .rounded(px(6.))
                        .bg(veil),
                );
            }
        }

        root.child(inner_flow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PublishState {
    Idle,
    Publishing,
    Published,
}

#[derive(Clone, Copy)]
enum NoteTone {
    Emerald,
    Blue,
    Amber,
}

struct ReleaseNote {
    key: &'static str,
    tag_en: &'static str,
    tag_zh: &'static str,
    body_en: &'static str,
    body_zh: &'static str,
    tone: NoteTone,
}

const RELEASE_NOTES: [ReleaseNote; 3] = [
    ReleaseNote {
        key: "note-shipped",
        tag_en: "Shipped",
        tag_zh: "已上线",
        body_en: "Dark mode is live for everyone.",
        body_zh: "深色模式现已向所有用户开放。",
        tone: NoteTone::Emerald,
    },
    ReleaseNote {
        key: "note-improved",
        tag_en: "Improved",
        tag_zh: "体验优化",
        body_en: "Search now indexes comments, code blocks, and archived threads in under 40 ms with instant keyboard navigation.",
        body_zh: "全局搜索现可在 40 毫秒内索引评论、代码块与归档讨论串，并支持纯键盘瞬时跳转。",
        tone: NoteTone::Blue,
    },
    ReleaseNote {
        key: "note-fixed",
        tag_en: "Fixed",
        tag_zh: "问题修复",
        body_en: "Popover menus stay anchored when resizing split panels or zooming the canvas.",
        body_zh: "调整分栏宽度或缩放画布时，浮层菜单始终保持精准锚定。",
        tone: NoteTone::Amber,
    },
];

/// Interactive release-notes showcase demonstrating [`BlurCrossfade`] on both an
/// inline stateful button (`BlurMorph::Both`) and variable-height cards (`BlurMorph::Height`).
pub struct BlurCrossfadeShowcase {
    id: ElementId,
    focus: FocusHandle,
    step: usize,
    last_dir: f32,
    publish_state: PublishState,
    publish_seq: usize,
    dots: [Slide; 3],
    pill_left: Slide,
    pill_right: Slide,
}

impl BlurCrossfadeShowcase {
    /// Construct a new release-notes showcase subscribed to [`Theme`] updates.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            step: 0,
            last_dir: 1.0,
            publish_state: PublishState::Idle,
            publish_seq: 0,
            dots: [Slide::snappy(18.), Slide::snappy(6.), Slide::snappy(6.)],
            pill_left: Slide::snappy(0.),
            pill_right: Slide::snappy(18.),
        }
    }

    /// Advance to the next changelog note.
    pub fn next_step(&mut self, cx: &mut Context<Self>) {
        self.step_with_dir((self.step + 1) % RELEASE_NOTES.len(), 1.0, cx);
    }

    /// Step backward to the previous changelog note.
    pub fn prev_step(&mut self, cx: &mut Context<Self>) {
        let prev = (self.step + RELEASE_NOTES.len() - 1) % RELEASE_NOTES.len();
        self.step_with_dir(prev, -1.0, cx);
    }

    /// Jump directly to a changelog note index (`0..3`).
    pub fn set_step(&mut self, index: usize, cx: &mut Context<Self>) {
        let clamped = index % RELEASE_NOTES.len();
        let dir = if clamped >= self.step { 1.0 } else { -1.0 };
        self.step_with_dir(clamped, dir, cx);
    }

    fn step_with_dir(&mut self, index: usize, dir: f32, cx: &mut Context<Self>) {
        let clamped = index % RELEASE_NOTES.len();
        if self.step != clamped {
            let prev = self.step;
            self.step = clamped;
            self.last_dir = dir;
            for (i, slide) in self.dots.iter_mut().enumerate() {
                slide.duration(if i == clamped { 18. } else { 6. }, 0.24, cx);
            }
            // Asymmetric 4-edge liquid pill stretch across `[0..18]`, `[23..41]`, `[46..64]`
            let target_left = (clamped as f32) * 11.0;
            let target_right = target_left + 18.0;
            let moving_right = clamped > prev;
            if moving_right {
                self.pill_right.duration(target_right, 0.17, cx);
                self.pill_left.duration(target_left, 0.28, cx);
            } else {
                self.pill_left.duration(target_left, 0.17, cx);
                self.pill_right.duration(target_right, 0.28, cx);
            }
            cx.notify();
        }
    }

    /// Trigger the `Publish` -> `Publishing…` -> `Published` morph cycle.
    pub fn trigger_publish(&mut self, cx: &mut Context<Self>) {
        if self.publish_state == PublishState::Publishing {
            return;
        }
        if self.publish_state == PublishState::Published {
            self.publish_state = PublishState::Idle;
            cx.notify();
            return;
        }
        self.publish_state = PublishState::Publishing;
        self.publish_seq = self.publish_seq.wrapping_add(1);
        let seq = self.publish_seq;
        cx.notify();

        if disabled_motion(cx) {
            self.publish_state = PublishState::Published;
            cx.notify();
            return;
        }

        cx.spawn(async move |entity, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(950))
                .await;
            let _ = entity.update(cx, |this, cx| {
                if this.publish_seq == seq {
                    this.publish_state = PublishState::Published;
                    cx.notify();
                }
            });
            cx.background_executor()
                .timer(Duration::from_millis(2200))
                .await;
            let _ = entity.update(cx, |this, cx| {
                if this.publish_seq == seq && this.publish_state == PublishState::Published {
                    this.publish_state = PublishState::Idle;
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

impl Render for BlurCrossfadeShowcase {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let lang = UiLanguage::current(cx);

        let card_bg: Hsla = if dark {
            rgb(0x232326).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.08)
        };
        let title_fg: Hsla = if dark {
            rgb(0xededed).into()
        } else {
            rgb(0x171717).into()
        };
        let sub_fg: Hsla = rgb(0x8f8f8f).into();
        let panel_bg: Hsla = if dark {
            hsla(0., 0., 1., 0.04)
        } else {
            rgb(0xf7f7f8).into()
        };
        let body_fg: Hsla = if dark {
            rgb(0xc8c8cc).into()
        } else {
            rgb(0x3b3b40).into()
        };

        let primary_bg: Hsla = if dark {
            rgb(0xededef).into()
        } else {
            rgb(0x26252b).into()
        };
        let primary_fg: Hsla = if dark {
            rgb(0x171717).into()
        } else {
            rgb(0xffffff).into()
        };

        let chip_bg: Hsla = if dark {
            rgb(0x2c2c30).into()
        } else {
            rgb(0xffffff).into()
        };
        let chip_border: Hsla = if dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.10)
        };
        let chip_fg: Hsla = if dark {
            rgb(0xededed).into()
        } else {
            rgb(0x171717).into()
        };

        let dot_active: Hsla = if dark {
            rgb(0xededef).into()
        } else {
            rgb(0x171717).into()
        };
        let dot_idle: Hsla = if dark {
            hsla(0., 0., 1., 0.20)
        } else {
            rgb(0xe0e0e2).into()
        };

        let (pub_key, pub_label) = match self.publish_state {
            PublishState::Idle => ("idle", lang.text("Publish", "发布")),
            PublishState::Publishing => ("publishing", lang.text("Publishing…", "发布中…")),
            PublishState::Published => ("published", lang.text("Published", "已发布")),
        };
        let (sub_key, sub_label) = match self.publish_state {
            PublishState::Published => (
                "sub-live",
                lang.text("3 changes · live", "3 项更新 · 已发布"),
            ),
            _ => (
                "sub-draft",
                lang.text("3 changes · draft", "3 项更新 · 草稿"),
            ),
        };

        let note = &RELEASE_NOTES[self.step];
        let (tag_bg, tag_fg): (Hsla, Hsla) = match note.tone {
            NoteTone::Emerald => {
                if dark {
                    (hsla(160. / 360., 0.84, 0.39, 0.14), rgb(0x34d399).into())
                } else {
                    (hsla(160. / 360., 0.84, 0.39, 0.12), rgb(0x0f9f6e).into())
                }
            }
            NoteTone::Blue => {
                if dark {
                    (hsla(217. / 360., 0.91, 0.60, 0.15), rgb(0x60a5fa).into())
                } else {
                    (hsla(217. / 360., 0.91, 0.60, 0.12), rgb(0x2563eb).into())
                }
            }
            NoteTone::Amber => {
                if dark {
                    (hsla(38. / 360., 0.92, 0.50, 0.15), rgb(0xfbbf24).into())
                } else {
                    (hsla(38. / 360., 0.92, 0.50, 0.13), rgb(0xb45309).into())
                }
            }
        };
        let tag_text = SharedString::from(lang.text(note.tag_en, note.tag_zh));
        let body_text = SharedString::from(lang.text(note.body_en, note.body_zh));

        let p = ProPalette::get(cx);
        let focus_ring = theme.tokens.colors.focus_outline;
        let pub_btn_id: ElementId = (self.id.clone(), "publish-btn").into();
        let next_btn_id: ElementId = (self.id.clone(), "next-btn").into();
        let pub_bx_id: ElementId = (self.id.clone(), "publish-bx").into();
        let sub_bx_id: ElementId = (self.id.clone(), "sub-bx").into();
        let note_bx_id: ElementId = (self.id.clone(), "note-bx").into();

        let dot_widths = [
            self.dots[0].get(w, cx),
            self.dots[1].get(w, cx),
            self.dots[2].get(w, cx),
        ];
        let p_left = self.pill_left.get(w, cx);
        let p_right = self.pill_right.get(w, cx);
        let stretch_w = (p_right - p_left).max(6.);

        let mut dots_hit_row = div().relative().flex().items_center().gap(px(5.));
        for (idx, width_px) in dot_widths.into_iter().enumerate() {
            let is_active = idx == self.step;
            let dot_id: ElementId =
                (self.id.clone(), SharedString::from(format!("dot-{idx}"))).into();
            let w_dot = width_px.max(6.);
            dots_hit_row =
                dots_hit_row.child(div().w(px(w_dot)).h(px(14.)).flex().items_center().child(
                    pro_tactile(
                        dot_id,
                        format!("Step {}", idx + 1),
                        999.,
                        if is_active {
                            transparent_black()
                        } else {
                            dot_idle
                        },
                        dot_active.opacity(0.45),
                        transparent_black(),
                        transparent_black(),
                        is_active,
                        div().w_full().h(px(6.)),
                        cx,
                        move |this, _, cx| {
                            this.set_step(idx, cx);
                        },
                    ),
                ));
        }
        let dots_row = div()
            .relative()
            .h(px(14.))
            .flex()
            .items_center()
            .child(dots_hit_row)
            .child(
                div()
                    .absolute()
                    .left(px(p_left))
                    .top(px(4.))
                    .w(px(stretch_w))
                    .h(px(6.))
                    .rounded(px(999.))
                    .bg(dot_active),
            );

        let primary_hover = if dark {
            rgb(0xffffff).into()
        } else {
            rgb(0x171717).into()
        };
        let publish_btn = div().flex_none().child(pro_tactile(
            pub_btn_id,
            pub_label,
            9.,
            primary_bg,
            primary_hover,
            transparent_black(),
            p.line_strong,
            self.publish_state == PublishState::Published,
            div()
                .h(px(30.))
                .px(px(12.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(primary_fg)
                .text_size(px(12.5))
                .line_height(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .child(BlurCrossfade::label(pub_bx_id, pub_key, pub_label)),
            cx,
            |this, _, cx| {
                this.trigger_publish(cx);
            },
        ));
        let next_btn = pro_btn(
            next_btn_id,
            lang.text("Next", "下一条"),
            None,
            ProBtnStyle::Secondary,
            true,
            cx,
            |this, _, cx| {
                this.next_step(cx);
            },
        );
        let _ = (chip_bg, chip_border, chip_fg);

        let card_shadows = if dark {
            vec![
                BoxShadow {
                    color: hsla(0., 0., 0., 0.35),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(2.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., 0.42),
                    offset: point(px(0.), px(10.)),
                    blur_radius: px(28.),
                    spread_radius: px(-12.),
                    inset: false,
                },
            ]
        } else {
            vec![
                BoxShadow {
                    color: hsla(0., 0., 0., 0.04),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(2.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., 0.08),
                    offset: point(px(0.), px(8.)),
                    blur_radius: px(24.),
                    spread_radius: px(-12.),
                    inset: false,
                },
            ]
        };

        let dir_bias = self.last_dir;
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .p(px(24.))
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(18.)).border_color(focus_ring))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "right" | "down" | "j" => {
                        this.next_step(cx);
                        cx.stop_propagation();
                    }
                    "left" | "up" | "k" => {
                        this.prev_step(cx);
                        cx.stop_propagation();
                    }
                    "enter" => {
                        this.trigger_publish(cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .w(px(360.))
                    .max_w_full()
                    .rounded(px(16.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(card_shadows)
                    .p(px(17.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .line_height(px(19.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(title_fg)
                                            .child(lang.text("Release 2.4", "版本 2.4")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(sub_fg)
                                            .child(
                                                BlurCrossfade::label(sub_bx_id, sub_key, sub_label)
                                                    .with_morph(BlurMorph::None)
                                                    .with_blur_px(6.)
                                                    .with_duration_ms(340.),
                                            ),
                                    ),
                            )
                            .child(publish_btn),
                    )
                    .child(
                        div()
                            .mt(px(15.))
                            .w_full()
                            .rounded(px(12.))
                            .bg(panel_bg)
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                BlurCrossfade::from_fn(note_bx_id, note.key, move |_, _| {
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .items_start()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .px(px(9.))
                                                .py(px(2.5))
                                                .rounded(px(999.))
                                                .bg(tag_bg)
                                                .text_color(tag_fg)
                                                .text_size(px(11.))
                                                .line_height(px(17.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(tag_text.clone()),
                                        )
                                        .child(
                                            div()
                                                .w_full()
                                                .text_size(px(13.))
                                                .line_height(px(19.5))
                                                .text_color(body_fg)
                                                .child(body_text.clone()),
                                        )
                                        .into_any_element()
                                })
                                .with_morph(BlurMorph::Height)
                                .with_direction(dir_bias)
                                .with_duration_ms(420.)
                                .with_blur_px(10.),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(12.))
                            .w_full()
                            .h(px(30.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(dots_row)
                            .child(next_btn),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlurCrossfade, BlurCrossfadeShowcase, BlurMorph, PublishState, bx_ease_in, bx_ease_out,
    };
    use gpui::{TestAppContext, div, prelude::*, px, size};
    use std::time::Duration;

    #[test]
    fn easing_curves_preserve_endpoints_and_monotonic_progress() {
        assert!((bx_ease_out(0.) - 0.).abs() < 1e-5);
        assert!((bx_ease_out(1.) - 1.).abs() < 1e-5);
        assert!(bx_ease_out(0.35) > 0.55, "ease-out should accelerate early");

        assert!((bx_ease_in(0.) - 0.).abs() < 1e-5);
        assert!((bx_ease_in(1.) - 1.).abs() < 1e-5);
        assert!(bx_ease_in(0.35) < 0.35, "ease-in should start gently");
    }

    #[test]
    fn builder_clamps_invalid_durations_and_switches_inline_on_both_morph() {
        let bx = BlurCrossfade::new("demo", "k1", div().child("Hello"))
            .with_duration_ms(10.)
            .with_blur_px(99.)
            .with_direction(4.0)
            .with_morph(BlurMorph::Both);
        assert_eq!(bx.duration_ms(), 60.);
        assert_eq!(bx.blur_px(), 40.);
        assert_eq!(bx.direction(), 1.0);
        assert_eq!(bx.morph(), BlurMorph::Both);
    }

    #[test]
    fn showcase_cycles_notes_and_publish_state_under_test_app() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, window) = app.add_window_view(|_, cx| BlurCrossfadeShowcase::new("bx-test", cx));
        window.run_until_parked();

        assert_eq!(window.update(|_, cx| view.read(cx).step), 0);
        window.update(|_, cx| {
            view.update(cx, |s, cx| s.next_step(cx));
        });
        assert_eq!(window.update(|_, cx| view.read(cx).step), 1);

        window.update(|_, cx| {
            view.update(cx, |s, cx| s.trigger_publish(cx));
        });
        assert_eq!(
            window.update(|_, cx| view.read(cx).publish_state),
            PublishState::Publishing
        );
    }

    #[gpui::test]
    fn rapid_step_navigation_stretches_liquid_pill_and_preserves_direction(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (view, vcx) = cx.add_window_view(|_, cx| BlurCrossfadeShowcase::new("bx-rapid", cx));
        vcx.simulate_resize(size(px(520.), px(360.)));
        vcx.run_until_parked();

        // Rapidly step forward twice -> pill_right leads ahead of pill_left (`stretch > 18px`)
        vcx.update(|_, app| {
            view.update(app, |s, cx| {
                s.next_step(cx);
                s.next_step(cx);
            });
        });
        exec.advance_clock(Duration::from_millis(80));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let s = view.read(app);
            assert_eq!(s.step, 2);
            assert_eq!(s.last_dir, 1.0);
            let span = s.pill_right.value(app) - s.pill_left.value(app);
            assert!(
                span > 19.0,
                "expected asymmetric liquid stretch > 19px mid-flight, got {span}"
            );
        });

        // Step backward -> flips direction to `-1.0` and settles cleanly to step 1 `[11..29]`
        vcx.update(|_, app| {
            view.update(app, |s, cx| s.prev_step(cx));
        });
        exec.advance_clock(Duration::from_millis(380));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let s = view.read(app);
            assert_eq!(s.step, 1);
            assert_eq!(s.last_dir, -1.0);
            assert!((s.pill_left.value(app) - 11.0).abs() < 0.5);
            assert!((s.pill_right.value(app) - 29.0).abs() < 0.5);
        });
    }
}
