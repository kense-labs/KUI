//! Measured skeleton blocks that resolve in place into host-provided content.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, Slide, disabled_motion},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    f32::consts::TAU,
    rc::Rc,
};
use web_time::Instant;

const STAGGER: f32 = 0.04;
const BLOCK_SECONDS: f32 = 0.34;
const REDUCED_SECONDS: f32 = 0.13;

/// Corner treatment of the placeholder.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MorphRadius {
    #[default]
    Theme,
    Fixed(Pixels),
    Circle,
}

type StyleHook = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type Layout = Box<dyn Fn(&mut MorphLayout<'_>) -> AnyElement>;

/// The placeholder and semantics of one keyed piece of a layout.
pub struct MorphBlock {
    id: ElementId,
    width: DefiniteLength,
    height: Option<DefiniteLength>,
    radius: MorphRadius,
    lines: Option<usize>,
    line_height: Pixels,
    role: Option<Role>,
    style: Option<StyleHook>,
}

impl MorphBlock {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            width: relative(1.),
            height: None,
            radius: MorphRadius::Theme,
            lines: None,
            line_height: px(20.),
            role: None,
            style: None,
        }
    }

    /// Sets the skeleton width in pixels, rems or a fraction of its parent.
    pub fn with_width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the skeleton height. A text block otherwise uses its line boxes.
    pub fn with_height(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn with_radius(mut self, radius: MorphRadius) -> Self {
        self.radius = radius;
        self
    }

    /// Displays one or more lines with a shorter final line.
    pub fn with_lines(mut self, lines: usize) -> Self {
        self.lines = Some(lines.max(1));
        self
    }

    pub fn with_line_height(mut self, height: Pixels) -> Self {
        if f32::from(height).is_finite() && height > px(0.) {
            self.line_height = height;
        }
        self
    }

    /// Applies semantics when the block is ready.
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    fn placeholder_height(&self) -> DefiniteLength {
        self.height.unwrap_or_else(|| {
            self.lines
                .map_or(px(12.), |count| self.line_height * count as f32)
                .into()
        })
    }

    fn shape(&self, theme_radius: Pixels, ink: Hsla, opacity: f32) -> Shape {
        Shape {
            radius: match self.radius {
                MorphRadius::Theme => Some(theme_radius),
                MorphRadius::Fixed(value) => Some(value.max(px(0.))),
                MorphRadius::Circle => None,
            },
            lines: self.lines,
            ink: ink.opacity(opacity),
        }
    }
}

#[derive(Clone, Copy)]
struct Shape {
    radius: Option<Pixels>,
    lines: Option<usize>,
    ink: Hsla,
}

impl Shape {
    fn paint(self, bounds: Bounds<Pixels>, w: &mut Window) {
        let count = self.lines.unwrap_or(1);
        let row_height = bounds.size.height / count as f32;
        let height = self.lines.map_or(row_height, |_| row_height * 0.6);
        let top = bounds.top() + (row_height - height) / 2.;
        let radius = self.radius.unwrap_or(height / 2.);
        for index in 0..count {
            let width = if count > 1 && index + 1 == count {
                bounds.size.width * 0.62
            } else {
                bounds.size.width
            };
            let rect = Bounds::new(
                point(bounds.left(), top + row_height * index as f32),
                size(width, height),
            );
            w.paint_quad(
                fill(rect, self.ink).corner_radii(radius.min(width / 2.).min(height / 2.)),
            );
        }
    }
}

type BlockSizes = Rc<RefCell<HashMap<ElementId, Size<Pixels>>>>;

/// Builds the GPUI content tree and assigns stagger order to its blocks.
pub struct MorphLayout<'a> {
    window: &'a mut Window,
    app: &'a mut App,
    sizes: BlockSizes,
    loading: bool,
    reveal_age: Option<f32>,
    pulse: f32,
    stagger: f32,
    reduced: bool,
    ink: Hsla,
    radius: Pixels,
    index: usize,
    running: bool,
}

impl MorphLayout<'_> {
    /// Adds a block in reading order. The content closure runs after its reveal starts.
    pub fn block(
        &mut self,
        block: MorphBlock,
        content: impl FnOnce(&mut Window, &mut App) -> AnyElement,
    ) -> Stateful<Div> {
        let index = self.index;
        self.index += 1;
        let delay = if self.reduced {
            0.
        } else {
            self.stagger * index as f32
        };
        let elapsed = self.reveal_age.map(|age| age - delay);
        let waiting = elapsed.is_some_and(|t| t < 0.);
        let skeleton = self.loading || waiting;
        let size_key = block.id.clone();
        let sizes = self.sizes.clone();
        let shape = block.shape(self.radius, self.ink, self.pulse);
        let mut outer = div().relative().min_w_0().max_w_full().flex_none();
        if skeleton {
            self.running |= waiting;
            let drawing = canvas(
                move |bounds, _, _| {
                    sizes.borrow_mut().insert(size_key, bounds.size);
                },
                move |bounds, _, w, _| shape.paint(bounds, w),
            )
            .absolute()
            .inset_0()
            .size_full();
            outer = outer
                .w(block.width)
                .h(block.placeholder_height())
                .child(drawing);
        } else {
            let visible = elapsed.map_or(1., |t| {
                let duration = if self.reduced { REDUCED_SECONDS } else { 0.16 };
                1. - (1. - (t / duration).clamp(0., 1.)).powi(2)
            });
            let actual = content(self.window, self.app);
            outer = outer.child(div().min_w_0().max_w_full().opacity(visible).child(actual));
            if let Some(t) = elapsed {
                let duration = if self.reduced {
                    REDUCED_SECONDS
                } else {
                    BLOCK_SECONDS
                };
                if t < duration {
                    self.running = true;
                    let from = self.sizes.borrow().get(&block.id).copied();
                    let faded = if self.reduced {
                        visible
                    } else {
                        critical_progress(t, 0.29)
                    };
                    let shape = block.shape(self.radius, self.ink, 1. - faded);
                    let geometry = if self.reduced {
                        1.
                    } else {
                        critical_progress(t, BLOCK_SECONDS)
                    };
                    outer = outer.child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, w, _| {
                                let size = from.map_or(bounds.size, |source| {
                                    size(
                                        source.width
                                            + (bounds.size.width - source.width) * geometry,
                                        source.height
                                            + (bounds.size.height - source.height) * geometry,
                                    )
                                });
                                shape.paint(Bounds::new(bounds.origin, size), w);
                            },
                        )
                        .absolute()
                        .inset_0()
                        .size_full(),
                    );
                }
            }
        }
        let outer = if let Some(style) = block.style {
            style(outer)
        } else {
            outer
        };
        outer
            .id(block.id)
            .when_some(block.role.filter(|_| !skeleton), |e, role| e.role(role))
    }
}

fn critical_progress(age: f32, duration: f32) -> f32 {
    if age >= duration {
        return 1.;
    }
    let x = (age / duration).max(0.) * 7.5;
    1. - (1. + x) * (-x).exp()
}

/// A loading region that clips and animates to its measured ready height.
pub struct SkeletonMorph {
    id: ElementId,
    loading: bool,
    stagger: f32,
    label: Option<SharedString>,
    role: Role,
    style: Option<StyleHook>,
    visibility: Option<Visibility>,
    content: Layout,
    changed: Instant,
    reveal: Option<Instant>,
    sizes: BlockSizes,
    natural_height: Rc<Cell<Option<Pixels>>>,
    target_height: Option<Pixels>,
    height: Slide,
}

impl SkeletonMorph {
    /// The layout closure composes GPUI elements and calls `block` for each animated piece.
    pub fn new(
        id: impl Into<ElementId>,
        loading: bool,
        content: impl Fn(&mut MorphLayout<'_>) -> AnyElement + 'static,
        cx: &App,
    ) -> Self {
        Self {
            id: id.into(),
            loading,
            stagger: STAGGER,
            label: None,
            role: Role::Group,
            style: None,
            visibility: None,
            content: Box::new(content),
            changed: cx.background_executor().now(),
            reveal: None,
            sizes: Rc::default(),
            natural_height: Rc::default(),
            target_height: None,
            height: Slide::new(0.),
        }
    }

    /// Delay in seconds between blocks. The default is 0.04.
    pub fn with_stagger(mut self, stagger: f32) -> Self {
        if stagger.is_finite() && stagger >= 0. {
            self.stagger = stagger;
        }
        self
    }

    pub fn with_loading_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Pauses paint requests when the host hides this region.
    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Reloading discards the previous reveal immediately.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading == loading {
            return;
        }
        let now = cx.background_executor().now();
        self.loading = loading;
        self.changed = now;
        self.reveal = (!loading).then_some(now);
        if loading {
            self.sizes.borrow_mut().clear();
            self.target_height = None;
            self.natural_height.set(None);
        }
        cx.notify();
    }

    fn height_at(&mut self, reduced: bool, now: Instant, cx: &App) -> (Option<Pixels>, bool) {
        let animate = self.reveal.is_some() && !reduced;
        if let Some(measured) = self.natural_height.get()
            && self.target_height != Some(measured)
        {
            if self.target_height.is_none() || !animate {
                self.height.reset(f32::from(measured));
            } else {
                self.height.duration(f32::from(measured), BLOCK_SECONDS, cx);
            }
            self.target_height = Some(measured);
        }
        if !animate || self.target_height.is_none() {
            return (None, false);
        }
        let (value, moving) = self.height.sample_batch(now, false);
        (Some(px(value.max(0.))), moving)
    }
}

impl Render for SkeletonMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let label = self
            .label
            .clone()
            .unwrap_or_else(|| UiLanguage::current(cx).text("Loading", "正在加载").into());
        let host_visible =
            w.is_visible() && self.visibility.as_ref().is_none_or(|visible| visible(cx));
        let pulse = if self.loading && !reduced {
            let age = now.saturating_duration_since(self.changed).as_secs_f32();
            let phase = (age / 2.8).fract() * TAU;
            let fade = (1. - phase.cos()) * 0.5;
            1. - 0.45 * fade * fade
        } else {
            1.
        };
        let age = self
            .reveal
            .map(|start| now.saturating_duration_since(start).as_secs_f32());
        let (height, moving) = self.height_at(reduced, now, cx);
        let (content, block_motion) = {
            let mut layout = MorphLayout {
                window: w,
                app: cx,
                sizes: self.sizes.clone(),
                loading: self.loading,
                reveal_age: age,
                pulse,
                stagger: self.stagger,
                reduced,
                ink: p.line,
                radius,
                index: 0,
                running: false,
            };
            let content = (self.content)(&mut layout);
            (content, layout.running)
        };
        if self.reveal.is_some() && !block_motion && !moving {
            self.reveal = None;
        }
        if host_visible && (self.loading && !reduced || block_motion || moving) {
            let weak = cx.weak_entity();
            w.on_next_frame(move |_, cx| {
                let _ = weak.update(cx, |_, cx| cx.notify());
            });
        }
        let natural = self.natural_height.clone();
        let weak = cx.weak_entity();
        let mut clip = div()
            .w_full()
            .flex()
            .flex_col()
            .flex_none()
            .overflow_hidden()
            .on_children_prepainted(move |children, w, _| {
                if let Some(inner) = children.first() {
                    let measured = inner.size.height;
                    if natural.replace(Some(measured)) != Some(measured) {
                        let weak = weak.clone();
                        w.on_next_frame(move |_, cx| {
                            let _ = weak.update(cx, |_, cx| cx.notify());
                        });
                    }
                }
            })
            .id("skeleton-morph-clip")
            .debug_selector(|| "skeleton-morph-clip".into())
            .child(div().w_full().flex_none().min_w_0().child(content));
        if let Some(height) = height {
            clip = clip.h(height);
        }
        let root = div().w_full().min_w_0().flex().flex_col();
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.id(self.id.clone())
            .role(self.role)
            .child(clip)
            .when(self.loading, |e| {
                e.child(
                    div()
                        .id("skeleton-morph-status")
                        .role(Role::Status)
                        .aria_label(label)
                        .size(px(0.)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{MorphBlock, SkeletonMorph};
    use gpui::{TestAppContext, VisualTestContext, div, prelude::*, px, relative};
    use std::{cell::Cell, rc::Rc, time::Duration};

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn loading_unmounts_content_and_reload_restarts_the_stagger() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let reads = Rc::new([Cell::new(0), Cell::new(0)]);
        let log = reads.clone();
        let (view, w) = app.add_window_view(|_, cx| {
            SkeletonMorph::new(
                "stagger",
                true,
                move |layout| {
                    let mut body = div().flex().flex_col();
                    for (i, count) in log.iter().enumerate() {
                        body = body.child(layout.block(
                            MorphBlock::new(("row", i)).with_lines(1),
                            |_, _| {
                                count.set(count.get() + 1);
                                div().h(px(20.)).into_any_element()
                            },
                        ));
                    }
                    body.into_any_element()
                },
                cx,
            )
        });
        tick(w, 0);
        assert!(reads.iter().all(|v| v.get() == 0));
        for _ in 0..2 {
            w.update(|_, cx| view.update(cx, |s, cx| s.set_loading(false, cx)));
            tick(w, 22);
            assert!(reads[0].get() > 0);
            assert_eq!(reads[1].get(), 0);
            tick(w, 40);
            assert!(reads[1].get() > 0);
            w.update(|_, cx| view.update(cx, |s, cx| s.set_loading(true, cx)));
            for count in reads.iter() {
                count.set(0);
            }
            tick(w, 14);
            assert!(reads.iter().all(|v| v.get() == 0));
        }
    }

    #[test]
    fn shell_eases_to_measured_height_and_reduced_motion_snaps_geometry() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            SkeletonMorph::new(
                "measured-card",
                true,
                |layout| {
                    div()
                        .w_full()
                        .child(
                            layout.block(
                                MorphBlock::new("body")
                                    .with_width(relative(0.5))
                                    .with_lines(2),
                                |_, _| div().w(px(210.)).h(px(93.)).into_any_element(),
                            ),
                        )
                        .into_any_element()
                },
                cx,
            )
            .with_style(|e| e.w(px(240.)))
        });
        let height =
            |w: &mut VisualTestContext| w.debug_bounds("skeleton-morph-clip").unwrap().size.height;
        tick(w, 0);
        assert_eq!(height(w), px(40.));
        w.update(|_, cx| view.update(cx, |s, cx| s.set_loading(false, cx)));
        tick(w, 0);
        assert_eq!(height(w), px(40.));
        tick(w, 80);
        let mid = height(w);
        assert!(mid > px(40.) && mid < px(93.), "{mid}");
        tick(w, 730);
        assert!((height(w) - px(93.)).abs() < px(0.1));
        w.update(|_, cx| {
            crate::foundation::Theme::global_mut(cx).reduced_motion = true;
            view.update(cx, |s, cx| s.set_loading(true, cx));
        });
        tick(w, 0);
        assert_eq!(height(w), px(40.));
        w.update(|_, cx| view.update(cx, |s, cx| s.set_loading(false, cx)));
        tick(w, 0);
        assert_eq!(height(w), px(93.));
    }
}
