//! Redeem a perforated ticket by pulling its stub away.
use crate::{
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        control::UiLanguage,
        direct_touch::Contact,
        family::{Slide, disabled_motion},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};
use web_time::Instant;

const OVERLAP: f32 = 5.;
const NOTCH: f32 = 9.;
const HOLE: f32 = 1.9;
const PITCH: f32 = 10.;
const EDGE_STEP: f32 = 1.25;
const FALL_SECONDS: f32 = 0.52;

type TicketContent = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Emitted once when a user detaches the stub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TicketTornChanged(pub bool);

/// Optional platform services for the live touch target and perforation feedback.
pub struct TicketTearPlatform {
    pub touch_region: fn(Bounds<Pixels>),
    pub pulse: fn(),
}
impl Global for TicketTearPlatform {}

#[derive(Clone, Copy)]
struct Pull {
    start: Point<Pixels>,
    last: Point<Pixels>,
    last_motion: Instant,
    velocity: Point<f32>,
    initial: f32,
    touch: bool,
}

#[derive(Clone, Copy)]
enum Phase {
    Attached,
    Ripping {
        start: Instant,
        from: f32,
        seconds: f32,
    },
    Falling {
        start: Instant,
        release: Point<f32>,
        bend: Point<f32>,
        reduced: bool,
    },
    Torn,
}

#[derive(Clone, Copy)]
struct Frame {
    front: f32,
    bend: Point<f32>,
    travel: Point<f32>,
    angle: f32,
    opacity: f32,
    detached: bool,
}

impl Frame {
    fn new(front: f32, bend: Point<f32>) -> Self {
        Self {
            front,
            bend,
            travel: point(0., 0.),
            angle: 0.,
            opacity: 1.,
            detached: false,
        }
    }

    fn falling(elapsed: f32, release: Point<f32>, bend: Point<f32>, reduced: bool) -> Self {
        let duration = if reduced { 0.16 } else { FALL_SECONDS };
        let progress = (elapsed / duration).clamp(0., 1.);
        let mut frame = Self::new(1., bend);
        frame.detached = true;
        frame.opacity = (1. - progress).powi(2);
        if !reduced {
            let carry = 1. - (-6.5 * elapsed).exp();
            frame.travel = point(
                (18. + release.x.clamp(0., 1000.) * 0.073) * carry,
                620. * elapsed * elapsed - 27. * elapsed
                    + release.y.clamp(0., 1100.) * 0.039 * carry,
            );
            frame.angle = (0.38 + release.x.clamp(0., 1000.) * 0.0002) * carry;
        }
        frame
    }
}

/// A paper ticket with host-rendered content and a one-time pull gesture.
pub struct TicketTear {
    id: ElementId,
    label: SharedString,
    body: TicketContent,
    stub: TicketContent,
    stub_width: Pixels,
    paper: Option<Hsla>,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    focus: FocusHandle,
    contact: Contact,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    activation: Option<Subscription>,
    pull: Option<Pull>,
    phase: Phase,
    progress: f32,
    bend_x: Slide,
    bend_y: Slide,
    finish_task: Option<Task<()>>,
}

impl EventEmitter<TicketTornChanged> for TicketTear {}

impl TicketTear {
    /// Creates a ticket from independently rendered body and stub content.
    pub fn new(
        body: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        stub: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: "ticket-tear".into(),
            label: UiLanguage::current(cx)
                .text("Tear off stub", "撕下票根")
                .into(),
            body: Box::new(body),
            stub: Box::new(stub),
            stub_width: px(104.),
            paper: None,
            style: None,
            focus: cx.focus_handle().tab_stop(true),
            contact: Contact::default(),
            bounds: Default::default(),
            activation: None,
            pull: None,
            phase: Phase::Attached,
            progress: 0.,
            bend_x: Slide::new(0.),
            bend_y: Slide::new(0.),
            finish_task: None,
        }
    }

    /// Sets the stable root ID.
    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the accessible action name on the stub.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the width of the stub before the seam overlap.
    pub fn with_stub_width(mut self, width: Pixels) -> Self {
        self.stub_width = width.max(px(24.));
        self
    }

    /// Overrides the body paper color.
    pub fn with_paper(mut self, paper: impl Into<Hsla>) -> Self {
        self.paper = Some(paper.into());
        self
    }

    /// Configures the root layout.
    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Starts with a detached stub when true. The default is false.
    pub fn with_default_torn(mut self, torn: bool) -> Self {
        self.phase = if torn { Phase::Torn } else { Phase::Attached };
        self.progress = if torn { 1. } else { 0. };
        self
    }

    /// Whether the stub has already detached.
    pub fn is_torn(&self) -> bool {
        matches!(self.phase, Phase::Falling { .. } | Phase::Torn)
    }

    /// Updates host state. False replaces the stub and cancels pending motion.
    pub fn set_torn(&mut self, torn: bool, cx: &mut Context<Self>) {
        if torn {
            self.finish(point(0., 0.), false, cx);
        } else {
            self.finish_task = None;
            self.pull = None;
            self.phase = Phase::Attached;
            self.progress = 0.;
            self.bend_x.reset(0.);
            self.bend_y.reset(0.);
            cx.notify();
        }
    }

    fn finish(&mut self, release: Point<f32>, emit: bool, cx: &mut Context<Self>) {
        if !matches!(self.phase, Phase::Attached) {
            return;
        }
        self.pull = None;
        let reduced = disabled_motion(cx);
        if reduced || self.progress >= 1. {
            self.detach(release, emit, cx);
            return;
        }
        let speed = pull_distance(release);
        let seconds = (0.30 - 0.12 * self.progress - 0.00010 * speed).clamp(0.11, 0.30);
        self.phase = Phase::Ripping {
            start: cx.background_executor().now(),
            from: self.progress,
            seconds,
        };
        self.finish_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_secs_f32(seconds))
                .await;
            let _ = view.update(cx, |s, cx| s.detach(release, emit, cx));
        }));
        cx.notify();
    }

    fn detach(&mut self, release: Point<f32>, emit: bool, cx: &mut Context<Self>) {
        if self.is_torn() {
            return;
        }
        let reduced = disabled_motion(cx);
        self.phase = Phase::Falling {
            start: cx.background_executor().now(),
            release,
            bend: if reduced {
                point(0., 0.)
            } else {
                point(self.bend_x.value(cx), self.bend_y.value(cx))
            },
            reduced,
        };
        self.progress = 1.;
        if emit {
            cx.emit(TicketTornChanged(true));
        }
        cx.notify();
    }

    fn height(&self) -> f32 {
        f32::from(self.bounds.get().size.height).max(1.)
    }

    fn start_pull(
        &mut self,
        position: Point<Pixels>,
        touch: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(self.phase, Phase::Attached) || self.pull.is_some() {
            return;
        }
        self.focus.focus(window, cx);
        self.pull = Some(Pull {
            start: position,
            last: position,
            last_motion: cx.background_executor().now(),
            velocity: point(0., 0.),
            initial: self.progress,
            touch,
        });
        cx.notify();
    }

    fn move_pull(&mut self, position: Point<Pixels>, touch: bool, cx: &mut Context<Self>) {
        let Some(pull) = self.pull.filter(|pull| pull.touch == touch) else {
            return;
        };
        let height = self.height();
        let travel = position - pull.start;
        let distance = point(f32::from(travel.x), f32::from(travel.y));
        let now = cx.background_executor().now();
        if position != pull.last {
            let elapsed = now
                .saturating_duration_since(pull.last_motion)
                .as_secs_f32()
                .max(0.008);
            let delta = position - pull.last;
            self.pull = Some(Pull {
                last: position,
                last_motion: now,
                velocity: point(f32::from(delta.x) / elapsed, f32::from(delta.y) / elapsed),
                ..pull
            });
        }
        let progress = (pull.initial + pull_distance(distance) / height).clamp(self.progress, 1.);
        if touch
            && !disabled_motion(cx)
            && perforations_at(height, progress * height)
                > perforations_at(height, self.progress * height)
            && let Some(platform) = cx.try_global::<TicketTearPlatform>()
        {
            (platform.pulse)();
        }
        self.progress = progress;
        self.bend_x.duration(distance.x.clamp(0., 84.), 0.22, cx);
        self.bend_y.duration(distance.y.clamp(0., 88.), 0.22, cx);
        if progress >= 1. {
            let velocity = self.pull.map_or(point(0., 0.), |pull| pull.velocity);
            self.finish(velocity, true, cx);
        }
        cx.notify();
    }

    fn release_pull(&mut self, touch: bool, cx: &mut Context<Self>) {
        let Some(pull) = self.pull.filter(|pull| pull.touch == touch) else {
            return;
        };
        let now = cx.background_executor().now();
        let fresh = now.saturating_duration_since(pull.last_motion) < Duration::from_millis(82);
        let traveled = (self.progress - pull.initial) * self.height();
        if self.progress >= 0.83
            || (fresh && traveled >= 12. && pull_distance(pull.velocity) >= 620.)
        {
            self.finish(pull.velocity, true, cx);
        } else {
            self.cancel_pull(cx);
        }
    }

    fn cancel_pull(&mut self, cx: &mut Context<Self>) {
        if self.pull.take().is_some() {
            self.bend_x.duration(0., 0.31, cx);
            self.bend_y.duration(0., 0.31, cx);
            cx.notify();
        }
    }

    fn frame(&self, window: &Window, cx: &App) -> Frame {
        let now = cx.background_executor().now();
        let visible = window.is_visible();
        let reduced = disabled_motion(cx);
        if let Phase::Falling {
            start,
            release,
            bend,
            reduced,
        } = self.phase
        {
            let frame = Frame::falling(
                now.saturating_duration_since(start).as_secs_f32(),
                release,
                bend,
                reduced,
            );
            if visible && frame.opacity > 0. {
                window.request_animation_frame();
            }
            return frame;
        }
        if matches!(self.phase, Phase::Torn) {
            return Frame::falling(1., point(0., 0.), point(0., 0.), true);
        }
        let bend = if reduced || !visible {
            point(0., 0.)
        } else {
            let (x, moving_x) = self.bend_x.sample_batch(now, false);
            let (y, moving_y) = self.bend_y.sample_batch(now, false);
            if moving_x || moving_y {
                window.request_animation_frame();
            }
            point(x, y)
        };
        let mut progress = self.progress;
        if let Phase::Ripping {
            start,
            from,
            seconds,
        } = self.phase
        {
            let t = (now.saturating_duration_since(start).as_secs_f32() / seconds).clamp(0., 1.);
            progress = from + (1. - from) * (0.35 * t + 0.65 * t.powi(3));
            if visible && t < 1. {
                window.request_animation_frame();
            }
        }
        Frame::new(progress, bend)
    }
}

fn pull_distance(delta: Point<f32>) -> f32 {
    (0.55 * delta.x.max(0.) + 0.85 * delta.y.max(0.)) / 0.7
}

fn first_perforation(height: f32) -> f32 {
    let center = height * 0.5;
    center - ((center - NOTCH - HOLE) / PITCH).floor() * PITCH
}

fn perforations_at(height: f32, y: f32) -> usize {
    ((y.min(height - NOTCH - HOLE) - first_perforation(height)) / PITCH + 1.)
        .max(0.)
        .floor() as usize
}

#[derive(Clone, Copy)]
struct Geometry {
    width: f32,
    height: f32,
    seam: f32,
    front: f32,
    radius: f32,
    reduced: bool,
}

impl Geometry {
    fn new(bounds: Bounds<Pixels>, stub: f32, frame: Frame, radius: f32, reduced: bool) -> Self {
        let width = f32::from(bounds.size.width).max(0.);
        let height = f32::from(bounds.size.height).max(0.);
        Self {
            width,
            height,
            seam: (width - stub).max(0.),
            front: frame.front * height,
            radius: radius.min(width * 0.25).min(height * 0.5),
            reduced,
        }
    }

    fn cutout(self, y: f32) -> f32 {
        let notch_y = y.min(self.height - y);
        let hole_y = (y - self.height / 2. + 0.5 * PITCH).rem_euclid(PITCH) - 0.5 * PITCH;
        if notch_y < NOTCH {
            (NOTCH * NOTCH - notch_y * notch_y).sqrt()
        } else if hole_y.abs() < HOLE {
            (HOLE * HOLE - hole_y * hole_y).sqrt()
        } else {
            0.
        }
    }

    fn rag(self, y: f32) -> f32 {
        if self.reduced {
            return 0.;
        }
        let n = (y / EDGE_STEP).round() as u32;
        let hash = n.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((hash ^ (hash >> 15)) & 255) as f32 / 127.5 - 1.;
        let to_hole = (y - self.height / 2. + 0.5 * PITCH).rem_euclid(PITCH) - 0.5 * PITCH;
        (1.55 * (y * 1.09).sin() + 1.35 * noise)
            * (to_hole.abs() / 4.).min(1.)
            * ((self.front - y) / 9.).clamp(0., 1.)
    }

    fn left_boundary(self, y: f32) -> f32 {
        if y > self.front || self.front == 0. {
            (self.seam - OVERLAP).min(self.seam - self.cutout(y))
        } else {
            let edge = self.seam + self.rag(y);
            let hole = self.cutout(y);
            if hole > 0. {
                edge.min(self.seam - hole)
            } else {
                edge
            }
        }
    }

    fn flap_boundary(self, y: f32) -> f32 {
        let edge = self.seam + self.rag(y);
        let hole = self.cutout(y);
        if hole > 0. {
            edge.max(self.seam + hole)
        } else {
            edge
        }
    }

    fn outer_inset(self, y: f32) -> f32 {
        let d = (self.radius - y.min(self.height - y)).max(0.);
        self.radius - (self.radius * self.radius - d * d).max(0.).sqrt()
    }
}

fn edge_rows(start: f32, end: f32) -> impl DoubleEndedIterator<Item = f32> {
    let rows = ((end - start) / EDGE_STEP).ceil().max(1.) as usize;
    (0..=rows).map(move |i| start + (end - start) * i as f32 / rows as f32)
}

fn paper_path(
    start: f32,
    end: f32,
    left: impl Fn(f32) -> f32,
    right: impl Fn(f32) -> f32,
    map: impl Fn(f32, f32) -> Point<Pixels>,
) -> PathBuilder {
    let mut path = PathBuilder::fill();
    path.move_to(map(left(start), start));
    for y in edge_rows(start, end) {
        path.line_to(map(right(y), y));
    }
    for y in edge_rows(start, end).rev() {
        path.line_to(map(left(y), y));
    }
    path.close();
    path
}

fn paint_paper(window: &mut Window, path: PathBuilder, color: Hsla) {
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn flap_point(shape: Geometry, frame: Frame, x: f32, y: f32) -> Point<f32> {
    let weight = if shape.front > 0. {
        (1. - y / shape.front).clamp(0., 1.)
    } else {
        0.
    };
    let fold = (frame.bend.x + frame.bend.y) * 0.0013 * weight;
    let bent_x = x + weight * frame.bend.x * 0.37 - (x - shape.seam) * fold;
    let bent_y = y + weight * frame.bend.y * 0.31 + (x - shape.seam) * fold * 0.5;
    let pivot = point((shape.seam + shape.width) * 0.5, shape.height * 0.5);
    let dx = bent_x - pivot.x;
    let dy = bent_y - pivot.y;
    let (sin, cos) = frame.angle.sin_cos();
    point(
        pivot.x + dx * cos - dy * sin + frame.travel.x,
        pivot.y + dx * sin + dy * cos + frame.travel.y,
    )
}

fn paint_ticket(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    shape: Geometry,
    frame: Frame,
    paper: Hsla,
    stub_paper: Hsla,
) {
    if shape.width < 1. || shape.height < 1. {
        return;
    }
    let base = |x, y| bounds.origin + point(px(x), px(y));
    let flap = |x, y| {
        let p = flap_point(shape, frame, x, y);
        base(p.x, p.y)
    };
    let front = shape.front.clamp(0., shape.height);
    let shadow_width = if frame.detached {
        shape.seam.max(0.)
    } else {
        shape.width
    };
    window.paint_drop_shadows(
        Bounds::new(bounds.origin, size(px(shadow_width), bounds.size.height)),
        Corners::all(px(shape.radius)),
        &[BoxShadow::new(px(0.), px(6.), black().opacity(0.12)).blur_radius(px(18.))],
    );
    let body = paper_path(
        0.,
        shape.height,
        |y| shape.outer_inset(y),
        |y| shape.left_boundary(y),
        base,
    );
    paint_paper(window, body, paper);
    if front < shape.height {
        let left = |y: f32| {
            if y.min(shape.height - y) < NOTCH {
                (shape.seam - OVERLAP).max(shape.seam + shape.cutout(y))
            } else {
                shape.seam - OVERLAP
            }
        };
        // The two painted pieces overlap at the stationary hinge to avoid a raster seam.
        let mut stub = paper_path(
            (front - 1.).max(0.),
            shape.height,
            left,
            |y| shape.width - shape.outer_inset(y),
            base,
        );
        let count = perforations_at(shape.height, shape.height);
        let first = first_perforation(shape.height);
        for i in 0..count {
            let y = first + i as f32 * PITCH;
            if y <= (front + HOLE).max(NOTCH) || y >= shape.height - NOTCH {
                continue;
            }
            let center = base(shape.seam, y);
            let r = px(HOLE);
            stub.move_to(center + point(r, px(0.)));
            stub.arc_to(point(r, r), px(0.), false, false, center - point(r, px(0.)));
            stub.arc_to(point(r, r), px(0.), false, false, center + point(r, px(0.)));
            stub.close();
        }
        paint_paper(window, stub, stub_paper);
    }
    if front > 0. && frame.opacity > 0. {
        let edge = paper_path(
            0.,
            front,
            |y| shape.flap_boundary(y),
            |y| shape.width - shape.outer_inset(y),
            flap,
        );
        paint_paper(window, edge, stub_paper.opacity(frame.opacity));
    }
    if front > NOTCH && !shape.reduced {
        let mut fibers = PathBuilder::stroke(px(0.6));
        for i in 1..=((front - NOTCH) / 3.7) as usize {
            let y = NOTCH + i as f32 * 3.7;
            if shape.cutout(y) != 0. {
                continue;
            }
            let x = shape.left_boundary(y);
            fibers.move_to(base(x - 0.7, y));
            fibers.line_to(base(x + 1.8 + (i % 3) as f32 * 0.35, y - 0.75));
        }
        paint_paper(window, fibers, paper.blend(stub_paper.opacity(0.33)));
    }
}

impl Render for TicketTear {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.activation.is_none() {
            self.activation = Some(cx.observe_window_activation(window, |s, w, cx| {
                if !w.is_window_active() {
                    s.cancel_pull(cx);
                }
            }));
        }
        let frame = self.frame(window, cx);
        let reduced = disabled_motion(cx);
        let radius = f32::from(surface_radius(cx));
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let paper = self
            .paper
            .unwrap_or_else(|| rgb(if dark { 0x2a2a2a } else { 0xffffff }).into());
        let stub_paper: Hsla = rgb(if dark { 0xbebebe } else { 0x363636 }).into();
        let stub_text: Hsla = rgb(if dark { 0x141414 } else { 0xffffff }).into();
        let current = self.bounds.get();
        let width = f32::from(if current.size.width > px(0.) {
            self.stub_width
                .min((current.size.width - px(24.)).max(px(24.)))
        } else {
            self.stub_width
        });
        let live = matches!(self.phase, Phase::Attached);
        let bounds = self.bounds.clone();
        let measured = bounds.clone();
        let shape = Geometry::new(current, width, frame, radius, reduced);
        let center = point((shape.width + shape.seam) * 0.5, shape.height * 0.5);
        let translated = if frame.detached || shape.front > center.y {
            flap_point(shape, frame, center.x, center.y) - center
        } else {
            point(0., 0.)
        };
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if e.pressed_button == Some(MouseButton::Left) {
                s.move_pull(e.position, false, cx);
            } else if s.pull.is_some_and(|pull| !pull.touch) {
                s.cancel_pull(cx);
            }
        });
        let released = cx.listener(|s, e: &MouseUpEvent, _, cx| {
            s.move_pull(e.position, false, cx);
            s.release_pull(false, cx);
        });
        let content = (self.body)(window, cx);
        let mut root = div()
            .relative()
            .w_full()
            .child(
                canvas(
                    move |bounds, w, _| {
                        let old = measured.replace(bounds);
                        if old.size != bounds.size {
                            w.request_animation_frame();
                        }
                    },
                    move |bounds, _, w, cx| {
                        let geometry = Geometry::new(bounds, width, frame, radius, reduced);
                        paint_ticket(w, bounds, geometry, frame, paper, stub_paper);
                        if live {
                            if let Some(platform) = cx.try_global::<TicketTearPlatform>() {
                                (platform.touch_region)(Bounds::new(
                                    bounds.origin
                                        + point(px((geometry.seam - OVERLAP).max(0.)), px(0.)),
                                    size(px(width + OVERLAP), bounds.size.height),
                                ));
                            }
                            w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                if phase.capture() {
                                    moved(e, w, cx);
                                }
                            });
                            w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                if phase.capture() && e.button == MouseButton::Left {
                                    released(e, w, cx);
                                }
                            });
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(div().w_full().pr(px(width - OVERLAP)).child(content));
        if frame.opacity > 0. {
            root = root.child(
                div()
                    .absolute()
                    .top(px(translated.y))
                    .bottom(px(-translated.y))
                    .right(px(-translated.x))
                    .w(px(width + OVERLAP))
                    .pl(px(OVERLAP))
                    .opacity(frame.opacity)
                    .text_color(stub_text)
                    .child((self.stub)(window, cx)),
            );
        }
        if live {
            let touch = self.contact.listen(cx, |s, e, w, cx| match e.phase {
                TouchPhase::Started => s.start_pull(e.position, true, w, cx),
                TouchPhase::Moved => s.move_pull(e.position, true, cx),
                TouchPhase::Ended => {
                    s.move_pull(e.position, true, cx);
                    s.release_pull(true, cx);
                }
                TouchPhase::Cancelled => s.cancel_pull(cx),
            });
            root = root.child(
                div()
                    .id("ticket-stub")
                    .debug_selector(|| "ticket-stub".into())
                    .role(Role::Button)
                    .aria_label(self.label.clone())
                    .track_focus(&self.focus)
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(px(width + OVERLAP))
                    .rounded(px(radius))
                    .border_1()
                    .border_color(transparent_black())
                    .focus_visible(|s| s.border_color(rgb(0x719bf7)))
                    .cursor(if self.pull.is_some() {
                        CursorStyle::ClosedHand
                    } else {
                        CursorStyle::OpenHand
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, w, cx| {
                            s.start_pull(e.position, false, w, cx);
                        }),
                    )
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if !e.is_held && matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.finish(point(0., 0.), true, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        } else if e.keystroke.key == "escape" {
                            s.cancel_pull(cx);
                        }
                    }))
                    .on_click(cx.listener(|s, e: &ClickEvent, _, cx| {
                        if matches!(e, ClickEvent::Keyboard(_)) {
                            s.finish(point(0., 0.), true, cx);
                        }
                    }))
                    .child(touch),
            );
        }
        let status = if self.is_torn() {
            UiLanguage::current(cx).text("Stub torn off", "票根已撕下")
        } else {
            ""
        };
        root = root.child(
            div()
                .id("ticket-status")
                .role(Role::Status)
                .aria_label(status)
                .absolute()
                .size(px(1.))
                .overflow_hidden()
                .opacity(0.)
                .child(status),
        );
        if let Some(style) = &self.style {
            root = style(root);
        }
        root.id(self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{FALL_SECONDS, Frame, TicketTear, TicketTornChanged, pull_distance};
    use crate::foundation::Theme;
    use gpui::{Modifiers, MouseButton, TestAppContext, div, point, prelude::*, px, size};
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn projects_reference_axis_travel() {
        assert!((pull_distance(point(20., 0.)) - 15.71).abs() < 0.01);
        assert!((pull_distance(point(0., 20.)) - 24.29).abs() < 0.01);
        assert!((pull_distance(point(15., 17.)) - 32.43).abs() < 0.01);
        assert_eq!(pull_distance(point(-12., -20.)), 0.);
    }

    #[test]
    fn falling_keeps_a_fixed_pivot_and_reduced_motion_only_fades() {
        let reduced = Frame::falling(0.08, point(700., 900.), point(0., 0.), true);
        assert_eq!(reduced.travel, point(0., 0.));
        assert_eq!(reduced.angle, 0.);
        assert!(reduced.opacity > 0. && reduced.opacity < 1.);
        let released = Frame::falling(0.23, point(700., 900.), point(20., 40.), false);
        assert!(released.travel.x > 0. && released.travel.y > 0. && released.angle > 0.);
        assert_eq!(
            Frame::falling(FALL_SECONDS, point(0., 0.), point(0., 0.), false).opacity,
            0.
        );
    }

    #[test]
    fn partial_pull_survives_reversal_and_keyboard_detaches_once() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (ticket, window) = app.add_window_view(|_, cx| {
            TicketTear::new(
                |_, _| div().h(px(211.)).child("Event").into_any_element(),
                |_, _| div().child("Seat 12").into_any_element(),
                cx,
            )
        });
        window.simulate_resize(size(px(400.), px(380.)));
        window.run_until_parked();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let recorded = changes.clone();
        let _subscription = window.update(|_, cx| {
            cx.subscribe(&ticket, move |_, event: &TicketTornChanged, _| {
                recorded.borrow_mut().push(event.0);
            })
        });
        let start = window.update(|_, cx| {
            let bounds = ticket.read(cx).bounds.get();
            point(bounds.right() - px(30.), bounds.center().y)
        });
        let mods = Modifiers::default();
        window.simulate_mouse_move(start, None, mods);
        window.simulate_mouse_down(start, MouseButton::Left, mods);
        window.executor().advance_clock(Duration::from_millis(100));
        window.simulate_mouse_move(start + point(px(15.), px(17.)), MouseButton::Left, mods);
        window.simulate_mouse_move(start, MouseButton::Left, mods);
        window.simulate_mouse_up(start, MouseButton::Left, mods);
        window.run_until_parked();
        assert!(changes.borrow().is_empty());
        let front = window.update(|_, cx| ticket.read(cx).progress * ticket.read(cx).height());
        assert!((front - 32.43).abs() < 0.01);
        window.simulate_keystrokes("enter space");
        window.run_until_parked();
        assert!(window.update(|_, cx| ticket.read(cx).is_torn()));
        assert_eq!(*changes.borrow(), [true]);
        window.update(|_, cx| ticket.update(cx, |s, cx| s.set_torn(false, cx)));
        assert_eq!(window.update(|_, cx| ticket.read(cx).progress), 0.);
        assert!(!window.update(|_, cx| ticket.read(cx).is_torn()));
        assert_eq!(*changes.borrow(), [true]);
    }
}
