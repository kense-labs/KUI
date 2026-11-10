//! An illustrative, host-supplied gather / act / verify timeline; it never calls tools.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        family::{Paint, Tone, disabled_motion, face},
        glyph::{Glyph, icon, spinner},
        kense_mark,
        nav::tr,
    },
};
use gpui::{prelude::*, *};
use std::{
    f32::consts::{FRAC_PI_2, PI, TAU},
    sync::{Arc, OnceLock},
};
use web_time::Instant;

#[derive(Clone)]
pub struct LoopCall {
    pub tool: SharedString,
    pub action: SharedString,
    pub result: SharedString,
    pub failed: bool,
}
impl LoopCall {
    pub fn new(
        tool: impl Into<SharedString>,
        action: impl Into<SharedString>,
        result: impl Into<SharedString>,
    ) -> Self {
        Self {
            tool: tool.into(),
            action: action.into(),
            result: result.into(),
            failed: false,
        }
    }
}
#[derive(Clone)]
pub struct LoopIteration {
    pub context: SharedString,
    pub detail: SharedString,
    pub tokens: u64,
    pub calls: Vec<LoopCall>,
    pub passed: bool,
    pub verification: SharedString,
}
impl LoopIteration {
    pub fn new(
        context: impl Into<SharedString>,
        tokens: u64,
        passed: bool,
        verification: impl Into<SharedString>,
    ) -> Self {
        Self {
            context: context.into(),
            detail: "".into(),
            tokens,
            calls: Vec::new(),
            passed,
            verification: verification.into(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopStation {
    Gather,
    Act,
    Verify,
}
#[derive(Clone, Debug, PartialEq)]
pub enum AgentLoopEvent {
    Station {
        iteration: usize,
        station: LoopStation,
    },
    Done {
        passed: bool,
    },
    Speed(f32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cue {
    Gather,
    Call(usize),
    ResolveCall(usize),
    Verify,
    ResolveCheck,
    Finish,
}
#[derive(Clone, Copy)]
struct Beat {
    iteration: usize,
    cue: Cue,
}
impl Beat {
    fn station(self) -> LoopStation {
        match self.cue {
            Cue::Gather | Cue::Finish => LoopStation::Gather,
            Cue::Call(_) | Cue::ResolveCall(_) => LoopStation::Act,
            Cue::Verify | Cue::ResolveCheck => LoopStation::Verify,
        }
    }
    fn duration(self) -> f32 {
        match self.cue {
            Cue::Gather => 0.95,
            Cue::Call(_) => 0.72,
            Cue::ResolveCall(_) => 1.00,
            Cue::Verify => 0.72,
            Cue::ResolveCheck => 1.12,
            Cue::Finish => 0.76,
        }
    }
    fn step_stop(self) -> bool {
        matches!(
            self.cue,
            Cue::Gather | Cue::ResolveCall(_) | Cue::ResolveCheck | Cue::Finish
        )
    }
}
fn timeline(iterations: &[LoopIteration]) -> Vec<Beat> {
    let mut beats = Vec::new();
    for (iteration, item) in iterations.iter().enumerate() {
        let mut push = |cue| beats.push(Beat { iteration, cue });
        push(Cue::Gather);
        for call in 0..item.calls.len() {
            push(Cue::Call(call));
            push(Cue::ResolveCall(call));
        }
        push(Cue::Verify);
        push(Cue::ResolveCheck);
    }
    if let Some(iteration) = iterations.len().checked_sub(1) {
        beats.push(Beat {
            iteration,
            cue: Cue::Finish,
        });
    }
    beats
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn station_angle(station: LoopStation) -> f32 {
    match station {
        LoopStation::Gather => -FRAC_PI_2,
        LoopStation::Act => PI / 6.,
        LoopStation::Verify => 5. * PI / 6.,
    }
}

#[derive(Default)]
struct Lanes {
    check: Option<(usize, Beat)>,
    tool: Option<(usize, Beat)>,
    gathered: usize,
}
fn landed_lanes(beats: &[Beat]) -> Lanes {
    let mut lanes = Lanes::default();
    for (index, beat) in beats.iter().copied().enumerate() {
        match beat.cue {
            Cue::Gather => lanes.gathered = beat.iteration + 1,
            Cue::Call(_) | Cue::ResolveCall(_) => lanes.tool = Some((index, beat)),
            Cue::Verify | Cue::ResolveCheck => lanes.check = Some((index, beat)),
            Cue::Finish => {}
        }
    }
    lanes
}

pub struct AgentLoop {
    title: SharedString,
    task: SharedString,
    summary: SharedString,
    model: SharedString,
    iterations: Vec<LoopIteration>,
    beats: Vec<Beat>,
    landed: usize,
    elapsed: f32,
    playing: bool,
    autoplay: bool,
    step_to: Option<usize>,
    speed: f32,
    last_frame: Option<Instant>,
    in_view: bool,
    visibility: fn() -> bool,
    measured_width: Option<f32>,
    next_focus: Option<FocusHandle>,
    restart_focus: Option<FocusHandle>,
}
impl EventEmitter<AgentLoopEvent> for AgentLoop {}
impl AgentLoop {
    pub fn new(task: impl Into<SharedString>, iterations: Vec<LoopIteration>) -> Self {
        let beats = timeline(&iterations);
        Self {
            title: "Agent loop".into(),
            task: task.into(),
            summary: "".into(),
            model: "Kense".into(),
            iterations,
            beats,
            landed: 0,
            elapsed: 0.,
            playing: false,
            autoplay: false,
            step_to: None,
            speed: 1.,
            last_frame: None,
            in_view: true,
            visibility: || true,
            measured_width: None,
            next_focus: None,
            restart_focus: None,
        }
    }
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }
    pub fn summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = summary.into();
        self
    }
    pub fn model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = model.into();
        self
    }
    /// Starts the first run once visible. With reduced motion, shows its result instead.
    pub fn autoplay(mut self, enabled: bool) -> Self {
        self.autoplay = enabled;
        self
    }
    /// Lets an embedding host report whether the preview is visible.
    pub fn visibility(mut self, visible: fn() -> bool) -> Self {
        self.visibility = visible;
        self
    }
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        self.autoplay = false;
        self.playing = playing && !self.finished() && !self.beats.is_empty();
        self.step_to = None;
        self.last_frame = None;
        cx.notify();
    }
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        self.landed = 0;
        self.elapsed = 0.;
        self.set_playing(false, cx);
    }
    pub fn set_speed(&mut self, speed: f32, cx: &mut Context<Self>) {
        self.speed = if speed.is_finite() {
            speed.clamp(0., 2.)
        } else {
            1.
        };
        self.last_frame = None;
        cx.emit(AgentLoopEvent::Speed(self.speed));
        cx.notify();
    }
    pub fn step(&mut self, cx: &mut Context<Self>) {
        let Some(stop) = self
            .beats
            .iter()
            .enumerate()
            .skip(self.landed)
            .find_map(|(index, beat)| beat.step_stop().then_some(index + 1))
        else {
            return;
        };
        self.set_playing(true, cx);
        self.step_to = Some(stop);
        if disabled_motion(cx) {
            while self.landed < stop {
                self.advance(cx);
            }
        }
    }
    fn finished(&self) -> bool {
        !self.beats.is_empty() && self.landed >= self.beats.len()
    }
    fn advance(&mut self, cx: &mut Context<Self>) {
        let Some(beat) = self.beats.get(self.landed).copied() else {
            return;
        };
        self.landed += 1;
        self.elapsed = 0.;
        match beat.cue {
            Cue::Gather | Cue::Call(_) | Cue::Verify => cx.emit(AgentLoopEvent::Station {
                iteration: beat.iteration,
                station: beat.station(),
            }),
            Cue::Finish => cx.emit(AgentLoopEvent::Done {
                passed: self.iterations[beat.iteration].passed,
            }),
            _ => {}
        }
        if self.finished() || self.step_to == Some(self.landed) {
            self.playing = false;
            self.step_to = None;
            self.last_frame = None;
        }
    }
    fn tick(&mut self, visible: bool, reduce: bool, w: &Window, cx: &mut Context<Self>) {
        if !visible {
            self.last_frame = None;
            return;
        }
        if std::mem::take(&mut self.autoplay) && !self.beats.is_empty() {
            if reduce {
                self.landed = self.beats.len();
            } else {
                self.playing = true;
            }
        }
        let rate = if self.step_to.is_some() && self.speed == 0. {
            1.
        } else {
            self.speed
        };
        if !self.playing || rate == 0. {
            self.last_frame = None;
            return;
        }
        let now = Instant::now();
        let dt = self.last_frame.replace(now).map_or(0., |last| {
            now.saturating_duration_since(last).as_secs_f32().min(0.1)
        });
        if let Some(next) = self.beats.get(self.landed) {
            self.elapsed += dt * rate;
            if self.elapsed >= next.duration() {
                self.advance(cx);
            }
        }
        if self.playing {
            w.request_animation_frame();
        }
    }
}

struct LoopLayout {
    ring_width: f32,
    radius: f32,
    center_y: f32,
    ring_height: f32,
    horizontal: bool,
    split: bool,
    compact: bool,
}
impl LoopLayout {
    fn new(width: f32) -> Self {
        let horizontal = width >= 604.;
        let ring_width = if horizontal { width - 220. } else { width };
        let compact = ring_width < 355.;
        let radius = if compact {
            (ring_width * 0.22).clamp(40., 76.)
        } else {
            94.
        };
        let center_y = 34. + radius;
        Self {
            ring_width,
            radius,
            center_y,
            ring_height: if compact {
                center_y + radius + 17.
            } else {
                242.
            },
            horizontal,
            split: horizontal || ring_width >= 440.,
            compact,
        }
    }
    fn station_x(&self, angle: f32) -> f32 {
        self.ring_width * 0.5 + self.radius * angle.cos()
    }
}

fn point_on_ring(center: Point<Pixels>, radius: f32, angle: f32) -> Point<Pixels> {
    center + point(px(radius * angle.cos()), px(radius * angle.sin()))
}
fn stroke_arc(
    w: &mut Window,
    center: Point<Pixels>,
    radius: f32,
    angles: (f32, f32),
    width: f32,
    ink: Hsla,
) {
    let (from, to) = angles;
    if to <= from {
        return;
    }
    let segments = ((to - from).abs() * 18.).ceil().max(1.) as usize;
    let mut path = PathBuilder::stroke(px(width));
    path.move_to(point_on_ring(center, radius, from));
    for i in 1..=segments {
        path.line_to(point_on_ring(
            center,
            radius,
            from + (to - from) * i as f32 / segments as f32,
        ));
    }
    if let Ok(path) = path.build() {
        w.paint_path(path, ink);
    }
}

fn dot(w: &mut Window, center: Point<Pixels>, diameter: f32, color: Hsla) {
    w.paint_quad(
        fill(
            Bounds::centered_at(center, size(px(diameter), px(diameter))),
            color,
        )
        .corner_radii(px(diameter / 2.)),
    );
}

fn model_mark(mode: ThemeMode) -> Arc<Image> {
    static LIGHT: OnceLock<Arc<Image>> = OnceLock::new();
    static DARK: OnceLock<Arc<Image>> = OnceLock::new();
    let (cache, bytes) = match mode {
        ThemeMode::Light => (&LIGHT, kense_mark::LIGHT),
        ThemeMode::Dark => (&DARK, kense_mark::DARK),
    };
    cache
        .get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())))
        .clone()
}

fn tokens(value: u64) -> String {
    if value >= 1_000 {
        let whole = value / 1000;
        let decimal = value % 1000 / 100;
        if decimal == 0 {
            format!("{whole}K")
        } else {
            format!("{whole}.{decimal}K")
        }
    } else {
        value.to_string()
    }
}

impl AgentLoop {
    fn ring(
        &self,
        layout: &LoopLayout,
        lanes: &Lanes,
        p: Paint,
        blue: Hsla,
        reduce: bool,
        cx: &App,
    ) -> Div {
        let last = self.landed.checked_sub(1).map(|index| self.beats[index]);
        let next = self.beats.get(self.landed).copied();
        let station = last.map_or(LoopStation::Gather, Beat::station);
        let from = station_angle(station);
        let target = next.map_or(station, Beat::station);
        let progress = next.map_or(0., |beat| self.elapsed / beat.duration());
        let travel = if reduce { 0. } else { ease(progress / 0.88) };
        let finishing = next.is_some_and(|beat| beat.cue == Cue::Finish);
        let angle = if finishing {
            from
        } else {
            from + (station_angle(target) - from).rem_euclid(TAU) * travel
        };
        let trail = if self.playing && !finishing && target != station && !reduce {
            (PI * travel).sin() * 0.85
        } else {
            0.
        };
        let done = self.finished();
        let active = (self.landed > 0 || self.elapsed > 0.) && !done && !finishing;
        let complete = if done {
            1.
        } else if finishing && !reduce {
            ease(progress)
        } else {
            0.
        };
        let passed = self.iterations.last().is_some_and(|item| item.passed);
        let completion_ink = if passed { blue } else { p.danger };
        let (radius, center_y, ring_height, split) = (
            layout.radius,
            layout.center_y,
            layout.ring_height,
            layout.split,
        );
        let pin = |lane: Option<(usize, Beat)>| {
            lane.map(|(index, beat)| {
                let appearing =
                    index + 1 == self.landed && matches!(beat.cue, Cue::Call(_) | Cue::Verify);
                if reduce || !appearing {
                    1.
                } else {
                    ease(self.elapsed / 0.19)
                }
            })
        };
        let (check_pin, tool_pin) = if split || lanes.check.zip(lanes.tool).is_none() {
            (pin(lanes.check), pin(lanes.tool))
        } else if lanes.check.map(|(index, _)| index) > lanes.tool.map(|(index, _)| index) {
            (pin(lanes.check), None)
        } else {
            (None, pin(lanes.tool))
        };
        let orbit = canvas(
            |_, _, _| (),
            move |b, _, w, _| {
                let center = point(b.center().x, b.top() + px(center_y));
                stroke_arc(w, center, radius, (0., TAU), 1.5, p.line);
                stroke_arc(
                    w,
                    center,
                    radius,
                    (-FRAC_PI_2, -FRAC_PI_2 + complete * TAU),
                    1.75,
                    completion_ink,
                );
                for (which, growth) in [
                    (LoopStation::Verify, check_pin),
                    (LoopStation::Act, tool_pin),
                ] {
                    if !split && growth.is_none() {
                        continue;
                    }
                    let pos = point_on_ring(center, radius, station_angle(which));
                    let length = (b.top() + px(ring_height) - pos.y - px(6.)).max(px(0.));
                    w.paint_quad(fill(
                        Bounds::new(pos + point(px(-0.5), px(5.)), size(px(1.), length + px(1.))),
                        p.line,
                    ));
                    if let Some(growth) = growth {
                        w.paint_quad(fill(
                            Bounds::new(
                                pos + point(px(-0.5), px(5.)),
                                size(px(1.), (length + px(1.)) * growth),
                            ),
                            blue.opacity(0.45),
                        ));
                    }
                }
                for a in [-PI / 6., FRAC_PI_2, 7. * PI / 6.] {
                    let pos = point_on_ring(center, radius, a);
                    let radial = point(px(a.cos()), px(a.sin()));
                    let tangent = point(-radial.y, radial.x);
                    let tip = pos + tangent * 3.;
                    let back = pos - tangent * 3.;
                    let mut arrow = PathBuilder::stroke(px(1.35));
                    arrow.move_to(back + radial * 3.);
                    arrow.line_to(tip);
                    arrow.line_to(back - radial * 3.);
                    if let Ok(path) = arrow.build() {
                        w.paint_path(path, if done { completion_ink } else { p.faint });
                    }
                }
                for which in [LoopStation::Gather, LoopStation::Act, LoopStation::Verify] {
                    let pos = point_on_ring(center, radius, station_angle(which));
                    dot(w, pos, 11., if done { completion_ink } else { p.faint });
                    dot(w, pos, 8., p.panel);
                }
                if active {
                    for n in (0..14).rev() {
                        if trail <= 0. {
                            break;
                        }
                        let a = angle - trail * (n + 1) as f32 / 14.;
                        let b = angle - trail * n as f32 / 14.;
                        stroke_arc(
                            w,
                            center,
                            radius,
                            (a, b + 0.004),
                            2.6,
                            blue.opacity(0.77 * (1. - n as f32 / 14.)),
                        );
                    }
                    let pos = point_on_ring(center, radius, angle);
                    dot(
                        w,
                        pos,
                        19.,
                        blue.opacity(if trail == 0. { 0.18 } else { 0.10 }),
                    );
                    dot(w, pos, 10., blue);
                }
            },
        )
        .size_full();
        let labels_below = layout.ring_width < 236.;
        let station_label = |station, glyph, text: &'static str| {
            let ink = p.secondary;
            let left = station == LoopStation::Verify;
            let centered = station == LoopStation::Gather || labels_below;
            let show_icon = !layout.compact || station == LoopStation::Gather;
            div()
                .w_full()
                .flex()
                .items_center()
                .when(centered, |v| v.justify_center())
                .when(!centered && left, |v| v.justify_end())
                .min_w_0()
                .gap_1()
                .text_size(px(if layout.compact { 12. } else { 13. }))
                .line_height(px(16.))
                .text_color(ink)
                .when(show_icon && !left, |v| v.child(icon(glyph, 13., ink)))
                .child(div().min_w_0().truncate().child(text))
                .when(show_icon && left, |v| v.child(icon(glyph, 13., ink)))
        };
        let lower_y = center_y + radius * 0.5;
        let verify_x = layout.station_x(station_angle(LoopStation::Verify));
        let act_x = layout.station_x(station_angle(LoopStation::Act));
        let center_status = if done {
            format!("{} {}", tr(cx, "Loops:", "轮次："), self.iterations.len())
        } else if let Some(beat) = last {
            format!("{} {}", tr(cx, "Loop", "轮次"), beat.iteration + 1)
        } else {
            tr(cx, "Ready", "准备就绪").to_owned()
        };
        div()
            .relative()
            .w_full()
            .h(px(ring_height))
            .flex_none()
            .child(orbit)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .child(station_label(
                        LoopStation::Gather,
                        Glyph::Folder,
                        tr(cx, "Gather context", "收集上下文"),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top(px(if labels_below {
                        lower_y + 12.
                    } else {
                        lower_y - 8.
                    }))
                    .left(px(0.))
                    .w(px(if labels_below {
                        verify_x + 27.
                    } else {
                        (verify_x - 10.).max(0.)
                    }))
                    .child(station_label(
                        LoopStation::Verify,
                        Glyph::Check,
                        tr(cx, "Verify", "验证"),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top(px(if labels_below {
                        lower_y + 12.
                    } else {
                        lower_y - 8.
                    }))
                    .right(px(0.))
                    .w(px(if labels_below {
                        layout.ring_width - act_x + 27.
                    } else {
                        (layout.ring_width - act_x - 10.).max(0.)
                    }))
                    .child(station_label(
                        LoopStation::Act,
                        Glyph::Settings,
                        tr(cx, "Take action", "执行操作"),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top(px(center_y - 37.))
                    .left_0()
                    .right_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(if self.model.as_ref() == "Kense" {
                        img(model_mark(Theme::global(cx).mode))
                            .size(px(22.))
                            .into_any_element()
                    } else {
                        icon(Glyph::Curious, 22., blue).into_any_element()
                    })
                    .child(
                        div()
                            .max_w(px(radius * 1.48))
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.model.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .when(done, |v| {
                                v.child(icon(
                                    if passed { Glyph::Check } else { Glyph::Alert },
                                    12.,
                                    if passed { p.success } else { p.danger },
                                ))
                            })
                            .child(center_status),
                    ),
            )
    }

    fn result(
        &self,
        lane: Option<(usize, Beat)>,
        empty: &'static str,
        p: Paint,
        radius: Pixels,
        reduce: bool,
        cx: &App,
    ) -> Div {
        let panel = div()
            .w_full()
            .h(px(56.))
            .min_w_0()
            .flex()
            .items_center()
            .gap_2()
            .overflow_hidden()
            .px_3()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .text_size(px(12.))
            .line_height(px(17.));
        let Some((index, beat)) = lane else {
            return panel
                .border_dashed()
                .justify_center()
                .text_center()
                .text_color(p.faint)
                .child(div().min_w_0().truncate().child(empty));
        };
        let item = &self.iterations[beat.iteration];
        let pending = matches!(beat.cue, Cue::Call(_) | Cue::Verify);
        let (glyph, title, detail, failed) = match beat.cue {
            Cue::Call(call) | Cue::ResolveCall(call) => {
                let call = &item.calls[call];
                (
                    Glyph::Settings,
                    format!("{} · {}", call.tool, call.action),
                    if pending {
                        tr(cx, "Running tool…", "正在运行工具…").into()
                    } else {
                        call.result.clone()
                    },
                    call.failed,
                )
            }
            Cue::Verify | Cue::ResolveCheck => {
                let label = if pending {
                    tr(cx, "Checking", "正在验证")
                } else if item.passed {
                    tr(cx, "Check passed", "检查通过")
                } else {
                    tr(cx, "Needs another pass", "需要再次检查")
                };
                (
                    Glyph::Check,
                    label.to_owned(),
                    if pending {
                        tr(cx, "Reviewing the result…", "正在核对结果…").into()
                    } else {
                        item.verification.clone()
                    },
                    !item.passed,
                )
            }
            _ => unreachable!("result slots only contain tool and check beats"),
        };
        let entering = if reduce || index + 1 != self.landed || !pending {
            1.
        } else {
            ease(self.elapsed / 0.19)
        };
        panel.relative().child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(6. * (1. - entering)))
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .opacity(0.28 + entering * 0.72)
                .child(icon(glyph, 14., p.secondary))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .child(div().truncate().text_color(p.secondary).child(detail)),
                )
                .map(|v| {
                    if pending {
                        v.child(spinner(
                            if reduce { 0. } else { self.elapsed * 1.25 },
                            15.,
                            p.secondary,
                        ))
                    } else {
                        v.child(icon(
                            if failed { Glyph::Alert } else { Glyph::Check },
                            15.,
                            if failed { p.danger } else { p.success },
                        ))
                    }
                }),
        )
    }

    fn context_cards(
        &self,
        lanes: &Lanes,
        layout: &LoopLayout,
        p: Paint,
        r: Pixels,
        reduce: bool,
        cx: &App,
    ) -> Div {
        let total = self
            .iterations
            .iter()
            .take(lanes.gathered)
            .fold(0u64, |sum, it| sum.saturating_add(it.tokens));
        let count = if !reduce
            && self.playing
            && self.landed > 0
            && self.beats[self.landed - 1].cue == Cue::Gather
        {
            let recent = self.iterations[lanes.gathered - 1].tokens;
            total.saturating_sub(recent).saturating_add(
                (recent as f64 * f64::from(ease(self.elapsed / 0.24))).round() as u64,
            )
        } else {
            total
        };
        let mut section = div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .when(layout.horizontal, |v| {
                v.w(px(204.))
                    .flex_none()
                    .pl_3()
                    .border_l_1()
                    .border_color(p.line)
            })
            .when(!layout.horizontal, |v| {
                v.w_full().pt_3().border_t_1().border_color(p.line)
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(p.secondary)
                    .child(
                        div()
                            .text_color(p.ink)
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr(cx, "Context", "上下文")),
                    )
                    .child(format!("{} tokens", tokens(count))),
            );
        let columns = if layout.horizontal || layout.compact {
            1
        } else {
            2
        };
        for (row, chunk) in self.iterations.chunks(columns).enumerate() {
            let mut slots = div().w_full().flex().gap_2();
            for (col, item) in chunk.iter().enumerate() {
                let received = row * columns + col < lanes.gathered;
                let card = div()
                    .flex_1()
                    .min_w_0()
                    .h(px(56.))
                    .px_2()
                    .rounded(r)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(12.))
                    .line_height(px(16.));
                slots = slots.child(if received {
                    card.child(icon(Glyph::FileText, 14., p.secondary)).child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_1()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(item.context.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_color(p.secondary)
                                            .child(tokens(item.tokens)),
                                    ),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_color(p.faint)
                                    .child(item.detail.clone()),
                            ),
                    )
                } else {
                    card.border_dashed().child(
                        div().w_full().text_center().text_color(p.faint).child(tr(
                            cx,
                            "Waiting for context",
                            "等待收集上下文",
                        )),
                    )
                });
            }
            if chunk.len() < columns {
                slots = slots.child(div().flex_1());
            }
            section = section.child(slots);
        }
        section
    }
}

impl Render for AgentLoop {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.next_focus.is_none() {
            self.next_focus = Some(cx.focus_handle().tab_stop(true));
            self.restart_focus = Some(cx.focus_handle().tab_stop(true));
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let mode = Theme::global(cx).mode;
        let blue: Hsla = rgb(if mode == ThemeMode::Dark {
            0x72a5ff
        } else {
            0x3569d6
        })
        .into();
        let reduce = disabled_motion(cx);
        let viewport = w.viewport_size();
        let visible = self.in_view && (self.visibility)() && w.is_visible();
        self.tick(visible, reduce, w, cx);
        let done = self.finished();
        if done && self.next_focus.as_ref().is_some_and(|h| h.is_focused(w)) {
            let target = self.restart_focus.as_ref().unwrap().clone();
            cx.on_next_frame(w, move |_, w, cx| target.focus(w, cx));
        }
        let passed = self.iterations.last().is_some_and(|it| it.passed);
        let (status, status_glyph, status_color) = if done {
            if passed {
                (tr(cx, "Done", "已完成"), Glyph::Check, p.success)
            } else {
                (tr(cx, "Stopped", "已停止"), Glyph::Alert, p.danger)
            }
        } else if self.playing {
            (tr(cx, "Running", "运行中"), Glyph::Play, blue)
        } else if self.landed > 0 || self.elapsed > 0. {
            (tr(cx, "Paused", "已暂停"), Glyph::Pause, p.secondary)
        } else {
            (tr(cx, "Ready", "准备就绪"), Glyph::Clock, p.secondary)
        };
        let available = self
            .measured_width
            .unwrap_or_else(|| (f32::from(viewport.width) - 82.).min(710.));
        let layout = LoopLayout::new(available.max(100.));
        let lanes = landed_lanes(&self.beats[..self.landed]);
        let compact = f32::from(viewport.width) < 460.;
        let inset = if compact { 14. } else { 24. };
        let mut root = div()
            .id("agent-loop")
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(760.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .text_size(px(14.))
            .line_height(px(20.))
            .flex()
            .flex_col()
            .gap_4()
            .p(px(inset))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap_1()
                            .text_color(status_color)
                            .child(icon(status_glyph, 13., status_color))
                            .child(status),
                    ),
            )
            .child(div().min_h(px(40.)).text_color(p.secondary).child(
                if done && !self.summary.is_empty() {
                    self.summary.clone()
                } else {
                    self.task.clone()
                },
            ));
        let mut results = div().w_full().flex().gap_2();
        if layout.split {
            results = results
                .child(div().flex_1().min_w_0().child(self.result(
                    lanes.check,
                    tr(cx, "Check results", "检查结果"),
                    p,
                    radius,
                    reduce,
                    cx,
                )))
                .child(div().flex_1().min_w_0().child(self.result(
                    lanes.tool,
                    tr(cx, "Tool results", "工具结果"),
                    p,
                    radius,
                    reduce,
                    cx,
                )));
        } else {
            let latest = [lanes.check, lanes.tool]
                .into_iter()
                .flatten()
                .max_by_key(|(index, _)| *index);
            results = results.child(self.result(
                latest,
                tr(cx, "Tool and check results", "工具与检查结果"),
                p,
                radius,
                reduce,
                cx,
            ));
        }
        let weak = cx.weak_entity();
        root = root.child(
            div()
                .on_children_prepainted(move |bounds, _, cx| {
                    let Some(b) = bounds.first() else { return };
                    let visible = b.left() < viewport.width
                        && b.top() < viewport.height
                        && b.right() > px(0.)
                        && b.bottom() > px(0.);
                    let width = f32::from(b.size.width);
                    let _ = weak.update(cx, |s, cx| {
                        if s.in_view != visible
                            || s.measured_width
                                .is_none_or(|previous| (previous - width).abs() >= 0.5)
                        {
                            if s.in_view != visible {
                                s.last_frame = None;
                            }
                            s.in_view = visible;
                            s.measured_width = Some(width);
                            cx.notify();
                        }
                    });
                })
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .gap_4()
                        .when(!layout.horizontal, |v| v.flex_col())
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(self.ring(&layout, &lanes, p, blue, reduce, cx))
                                .child(results),
                        )
                        .child(self.context_cards(&lanes, &layout, p, radius, reduce, cx)),
                ),
        );
        if self.beats.is_empty() {
            return root;
        }
        let playing = self.playing;
        let play = if playing {
            tr(cx, "Pause", "暂停")
        } else if done {
            tr(cx, "Play again", "重播")
        } else {
            tr(cx, "Play", "播放")
        };
        let round_control = |id, label, glyph, enabled| {
            face(id, label, 36., 18., Tone::Surface, cx)
                .w(px(36.))
                .border_1()
                .border_color(p.line)
                .when(enabled, |v| {
                    v.hover(move |v| v.bg(p.hover))
                        .active(move |v| v.bg(p.line))
                })
                .when(!enabled, |v| v.opacity(0.38).cursor_default().tab_index(-1))
                .child(icon(glyph, 15., p.ink))
        };
        let mut speeds = div()
            .flex()
            .items_center()
            .gap_1()
            .p(px(3.))
            .rounded(radius)
            .bg(p.stage);
        for (i, speed) in [0.5_f32, 1., 2.].into_iter().enumerate() {
            let selected = (self.speed - speed).abs() < 0.01;
            speeds = speeds.child(
                face(
                    ("loop-speed", i),
                    format!("{speed}×"),
                    27.,
                    f32::from(radius) - 2.,
                    Tone::Clear,
                    cx,
                )
                .min_w(px(28.))
                .aria_selected(selected)
                .child(format!("{speed}×"))
                .px_1()
                .text_size(px(12.))
                .when(selected, |v| v.bg(p.panel).shadow_sm())
                .when(!selected, |v| {
                    v.text_color(p.secondary).hover(move |v| v.bg(p.hover))
                })
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                    let speeds = [0.5_f32, 1., 2.];
                    let current = speeds
                        .iter()
                        .position(|v| (s.speed - v).abs() < 0.01)
                        .unwrap_or(1);
                    let target = match e.keystroke.key.as_str() {
                        "left" => current.saturating_sub(1),
                        "right" => (current + 1).min(2),
                        _ => return,
                    };
                    s.set_speed(speeds[target], cx);
                    cx.stop_propagation();
                }))
                .on_click(cx.listener(move |s, _, _, cx| s.set_speed(speed, cx))),
            );
        }
        root.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .flex_wrap()
                .gap_2()
                .pt_3()
                .border_t_1()
                .border_color(p.line)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            round_control(
                                "loop-play",
                                play,
                                if playing { Glyph::Pause } else { Glyph::Play },
                                true,
                            )
                            .on_click(cx.listener(|s, _, _, cx| {
                                if s.finished() {
                                    s.restart(cx);
                                }
                                s.set_playing(!s.playing, cx);
                            })),
                        )
                        .child(
                            round_control(
                                "loop-step",
                                tr(cx, "Next step", "下一步"),
                                Glyph::Arrow,
                                !done,
                            )
                            .when(!done, |v| {
                                v.track_focus(self.next_focus.as_ref().unwrap())
                                    .on_click(cx.listener(|s, _, _, cx| s.step(cx)))
                            }),
                        )
                        .child(
                            round_control(
                                "loop-restart",
                                tr(cx, "Restart", "重新开始"),
                                Glyph::Undo,
                                true,
                            )
                            .track_focus(self.restart_focus.as_ref().unwrap())
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.restart(cx);
                                s.set_playing(true, cx);
                            })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .when(available >= 340., |v| {
                            v.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(tr(cx, "Speed", "速度")),
                            )
                        })
                        .child(speeds),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Beat, Cue, LoopCall, LoopIteration, landed_lanes, timeline};

    #[test]
    fn results_survive_later_stations_and_resolve_in_host_order() {
        let mut first = LoopIteration::new("logs", 3, false, "retry");
        first.calls.push(LoopCall::new("one", "read", "found"));
        first.calls.push(LoopCall::new("two", "inspect", "done"));
        let second = LoopIteration::new("fix", 4, true, "passed");
        let beats = timeline(&[first, second]);
        let sequence: Vec<_> = beats
            .iter()
            .filter(|beat| beat.step_stop())
            .map(|b| b.cue)
            .collect();
        assert_eq!(
            sequence,
            [
                Cue::Gather,
                Cue::ResolveCall(0),
                Cue::ResolveCall(1),
                Cue::ResolveCheck,
                Cue::Gather,
                Cue::ResolveCheck,
                Cue::Finish
            ]
        );
        let lanes = landed_lanes(&beats[..8]);
        assert_eq!(lanes.gathered, 2);
        assert!(matches!(
            lanes.tool,
            Some((
                _,
                Beat {
                    cue: Cue::ResolveCall(1),
                    ..
                }
            ))
        ));
        assert!(matches!(
            lanes.check,
            Some((
                _,
                Beat {
                    iteration: 0,
                    cue: Cue::ResolveCheck
                }
            ))
        ));
    }
}
