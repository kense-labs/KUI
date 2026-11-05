//! An illustrative, host-supplied gather / act / verify timeline; it never calls tools.
use crate::primitives::{
    family::{Paint, Tone, disabled_motion, face},
    nav::tr,
};
use gpui::{prelude::*, *};
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
#[derive(Clone, Copy)]
struct Beat {
    iteration: usize,
    station: LoopStation,
    call: usize,
}
fn timeline(iterations: &[LoopIteration]) -> Vec<Beat> {
    let mut beats = Vec::new();
    for (iteration, item) in iterations.iter().enumerate() {
        beats.push(Beat {
            iteration,
            station: LoopStation::Gather,
            call: 0,
        });
        for call in 0..item.calls.len() {
            beats.push(Beat {
                iteration,
                station: LoopStation::Act,
                call,
            });
        }
        beats.push(Beat {
            iteration,
            station: LoopStation::Verify,
            call: 0,
        });
    }
    beats
}
fn loop_pose(
    progress: f32,
    reduced_motion: bool,
    moves_station: bool,
    playing: bool,
) -> (f32, f32) {
    if reduced_motion || !moves_station {
        return (0., 0.);
    }
    let t = progress.clamp(0., 1.);
    let travel = t * t * (3. - 2. * t);
    let trail = if playing {
        (t * std::f32::consts::PI).sin() * 0.32
    } else {
        0.
    };
    (travel, trail)
}
pub struct AgentLoop {
    title: SharedString,
    task: SharedString,
    summary: SharedString,
    model: SharedString,
    iterations: Vec<LoopIteration>,
    beats: Vec<Beat>,
    beat: usize,
    progress: f32,
    playing: bool,
    done: bool,
    speed: f32,
    last_frame: Option<Instant>,
    in_view: bool,
    next_focus: Option<FocusHandle>,
    restart_focus: Option<FocusHandle>,
    focus_done: bool,
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
            beat: 0,
            progress: 0.,
            playing: false,
            done: false,
            speed: 1.,
            last_frame: None,
            in_view: true,
            next_focus: None,
            restart_focus: None,
            focus_done: false,
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
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.beats.is_empty() || self.done {
            return;
        }
        self.playing = playing;
        self.last_frame = None;
        cx.notify();
    }
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        self.beat = 0;
        self.progress = 0.;
        self.done = false;
        self.focus_done = false;
        self.playing = false;
        self.last_frame = None;
        cx.notify();
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
        if self.done || self.beats.is_empty() {
            return;
        }
        self.playing = false;
        self.last_frame = None;
        self.advance(cx);
        cx.notify();
    }
    fn advance(&mut self, cx: &mut Context<Self>) {
        if self.beat + 1 >= self.beats.len() {
            self.done = true;
            self.playing = false;
            self.progress = 0.;
            cx.emit(AgentLoopEvent::Done {
                passed: self.iterations.last().is_some_and(|it| it.passed),
            });
        } else {
            self.beat += 1;
            self.progress = 0.;
            let beat = self.beats[self.beat];
            cx.emit(AgentLoopEvent::Station {
                iteration: beat.iteration,
                station: beat.station,
            });
        }
    }
}
impl Render for AgentLoop {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.next_focus.is_none() {
            self.next_focus = Some(cx.focus_handle().tab_stop(true));
            self.restart_focus = Some(cx.focus_handle().tab_stop(true));
        }
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let reduce = disabled_motion(cx);
        if self.playing && self.in_view && w.is_visible() && self.speed > 0. {
            let now = Instant::now();
            if let Some(last) = self.last_frame {
                self.progress +=
                    now.saturating_duration_since(last).as_secs_f32().min(0.1) * self.speed / 1.6;
            }
            self.last_frame = Some(now);
            if self.progress >= 1. {
                self.advance(cx);
            }
            if self.playing {
                w.request_animation_frame();
            }
        } else {
            self.last_frame = None;
        }
        if self.done
            && !self.focus_done
            && self.next_focus.as_ref().is_some_and(|h| h.is_focused(w))
        {
            self.focus_done = true;
            let target = self.restart_focus.as_ref().unwrap().clone();
            cx.on_next_frame(w, move |_, w, cx| target.focus(w, cx));
        }
        let current = self.beats.get(self.beat).copied();
        let station = current.map_or(0., |b| match b.station {
            LoopStation::Gather => 0.,
            LoopStation::Act => 1.,
            LoopStation::Verify => 2.,
        });
        let moves_station = current
            .zip(self.beats.get(self.beat + 1))
            .is_some_and(|(a, b)| a.station != b.station || a.iteration != b.iteration);
        // Position depends on the stored timeline, so pause does not jump back.
        let (travel, trail) = loop_pose(self.progress, reduce, moves_station, self.playing);
        let angle = -std::f32::consts::FRAC_PI_2 + (station + travel) * std::f32::consts::TAU / 3.;
        let head = p.bright;
        let line = p.line;
        let ring = canvas(
            |_, _, _| (),
            move |bounds, _, w, _| {
                let center = bounds.center();
                let radius = f32::from(bounds.size.width.min(bounds.size.height)) * 0.34;
                let mut path = PathBuilder::stroke(px(1.5));
                for n in 0..=96 {
                    let a = std::f32::consts::TAU * n as f32 / 96.;
                    let pt = center + point(px(a.cos() * radius), px(a.sin() * radius));
                    if n == 0 {
                        path.move_to(pt);
                    } else {
                        path.line_to(pt);
                    }
                }
                if let Ok(path) = path.build() {
                    w.paint_path(path, line);
                }
                for n in 0..3 {
                    let a = -std::f32::consts::FRAC_PI_2 + n as f32 * std::f32::consts::TAU / 3.;
                    let pos = center + point(px(a.cos() * radius), px(a.sin() * radius));
                    if (station.round() as i32) == n as i32 {
                        w.paint_quad(
                            fill(
                                Bounds::centered_at(pos, size(px(21.), px(21.))),
                                head.opacity(0.32),
                            )
                            .corner_radii(px(10.5)),
                        );
                    }
                    w.paint_quad(
                        fill(
                            Bounds::centered_at(pos, size(px(10.), px(10.))),
                            line,
                        )
                        .corner_radii(px(5.)),
                    );
                }
                for n in (1..=12).rev() {
                    let t = n as f32 / 12.;
                    let a = angle - trail * t;
                    let pos = center + point(px(a.cos() * radius), px(a.sin() * radius));
                    let diameter = 7. * (1. - t * 0.65);
                    w.paint_quad(
                        fill(
                            Bounds::centered_at(pos, size(px(diameter), px(diameter))),
                            head.opacity((1. - t) * 0.42),
                        )
                        .corner_radii(px(diameter / 2.)),
                    );
                }
                let pos = center + point(px(angle.cos() * radius), px(angle.sin() * radius));
                w.paint_quad(
                    fill(Bounds::centered_at(pos, size(px(10.), px(10.))), head)
                        .corner_radii(px(5.)),
                );
            },
        );
        let label = if self.done {
            if self.iterations.last().is_some_and(|v| v.passed) {
                tr(cx, "Done", "已完成")
            } else {
                tr(cx, "Stopped", "已停止")
            }
        } else if self.playing {
            tr(cx, "Running", "运行中")
        } else if self.beat == 0 && self.progress == 0. {
            tr(cx, "Ready", "准备就绪")
        } else {
            tr(cx, "Paused", "已暂停")
        };
        let mut root = div()
            .id("agent-loop")
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(760.))
            .p_4()
            .rounded(r)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_3()
                    .child(self.title.clone())
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(label),
                    ),
            )
            .child(div().text_size(px(13.)).text_color(p.secondary).child(
                if self.done && !self.summary.is_empty() {
                    self.summary.clone()
                } else {
                    self.task.clone()
                },
            ));
        let center = div()
            .relative()
            .size(px(232.))
            .child(ring.size_full())
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(65.))
                    .text_size(px(14.))
                    .text_color(p.secondary)
                    .child(tr(cx, "Gather context", "收集上下文")),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(12.))
                    .left(px(0.))
                    .text_size(px(14.))
                    .text_color(p.secondary)
                    .child(tr(cx, "Verify", "验证")),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(12.))
                    .right(px(0.))
                    .text_size(px(14.))
                    .text_color(p.secondary)
                    .child(tr(cx, "Take action", "执行操作")),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(self.model.clone())
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(format!(
                                "{} {}",
                                tr(cx, "Loop", "轮次"),
                                current.map_or(0, |b| b.iteration + 1)
                            )),
                    ),
            );
        let mut detail = div().flex().flex_col().gap_2().w_full();
        if let Some(beat) = current {
            let item = &self.iterations[beat.iteration];
            let state = match beat.station {
                LoopStation::Gather => format!(
                    "{} · {}",
                    tr(cx, "Gather context", "收集上下文"),
                    item.context
                ),
                LoopStation::Act => item.calls.get(beat.call).map_or_else(
                    || tr(cx, "Take action", "执行操作").to_string(),
                    |c| {
                        format!(
                            "{} · {} — {}{}",
                            c.tool,
                            c.action,
                            c.result,
                            if c.failed {
                                tr(cx, " · failed", " · 失败")
                            } else {
                                ""
                            }
                        )
                    },
                ),
                LoopStation::Verify => format!(
                    "{} · {}",
                    if item.passed {
                        tr(cx, "Check passed", "检查通过")
                    } else {
                        tr(cx, "Retry", "重试")
                    },
                    item.verification
                ),
            };
            detail = detail.child(div().text_color(p.ink).child(state));
            if beat.station == LoopStation::Gather && !item.detail.is_empty() {
                detail = detail.child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(item.detail.clone()),
                );
            }
            for item in self.iterations.iter().take(beat.iteration + 1) {
                detail = detail.child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(format!("{} · {} tokens", item.context, item.tokens)),
                );
            }
        }
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        root = root.child(
            div()
                .on_children_prepainted(move |bounds, _, cx| {
                    if let Some(b) = bounds.first() {
                        let visible = b.left() < viewport.width
                            && b.top() < viewport.height
                            && b.right() > px(0.)
                            && b.bottom() > px(0.);
                        let _ = weak.update(cx, |s, cx| {
                            if s.in_view != visible {
                                s.in_view = visible;
                                s.last_frame = None;
                                cx.notify();
                            }
                        });
                    }
                })
                .flex()
                .flex_col()
                .items_center()
                .gap_3()
                .child(center)
                .child(detail),
        );
        if self.beats.is_empty() {
            return root;
        }
        let play = if self.playing {
            tr(cx, "Pause", "暂停")
        } else {
            tr(cx, "Play", "播放")
        };
        let mut controls = div().flex().flex_wrap().items_center().gap_2();
        if !self.done && !self.beats.is_empty() {
            controls = controls
                .child(
                    face("loop-play", play, 32., 8., Tone::Surface, cx)
                        .px_2()
                        .child(play)
                        .on_click(cx.listener(|s, _, _, cx| s.set_playing(!s.playing, cx))),
                )
                .child(
                    face(
                        "loop-step",
                        tr(cx, "Next step", "下一步"),
                        32.,
                        8.,
                        Tone::Surface,
                        cx,
                    )
                    .px_2()
                    .child(tr(cx, "Next step", "下一步"))
                    .track_focus(self.next_focus.as_ref().unwrap())
                    .on_click(cx.listener(|s, _, _, cx| s.step(cx))),
                );
        }
        controls = controls.child(
            face(
                "loop-restart",
                tr(cx, "Restart", "重新开始"),
                32.,
                8.,
                Tone::Surface,
                cx,
            )
            .px_2()
            .child(tr(cx, "Restart", "重新开始"))
            .track_focus(self.restart_focus.as_ref().unwrap())
            .on_click(cx.listener(|s, _, _, cx| s.restart(cx))),
        );
        for (i, speed) in [0.5_f32, 1., 2.].into_iter().enumerate() {
            controls = controls.child(
                face(
                    ("loop-speed", i),
                    format!("{speed}×"),
                    32.,
                    8.,
                    if (self.speed - speed).abs() < 0.01 {
                        Tone::Primary
                    } else {
                        Tone::Surface
                    },
                    cx,
                )
                .px_2()
                .child(format!("{speed}×"))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                    let choices = [0.5_f32, 1., 2.];
                    let current = choices
                        .iter()
                        .position(|value| (s.speed - value).abs() < 0.01)
                        .unwrap_or(1);
                    let target = match e.keystroke.key.as_str() {
                        "left" => current.saturating_sub(1),
                        "right" => (current + 1).min(2),
                        _ => return,
                    };
                    s.set_speed(choices[target], cx);
                    cx.stop_propagation();
                }))
                .on_click(cx.listener(move |s, _, _, cx| s.set_speed(speed, cx))),
            );
        }
        root.child(controls)
    }
}

#[cfg(test)]
mod tests {
    use super::{LoopCall, LoopIteration, LoopStation, loop_pose, timeline};
    #[test]
    fn timeline_preserves_call_order_and_failed_verification() {
        let mut first = LoopIteration::new("logs", 3, false, "retry");
        first.calls.push(LoopCall::new("one", "read", "found"));
        first.calls.push(LoopCall::new("two", "inspect", "done"));
        let second = LoopIteration::new("fix", 4, true, "passed");
        let beats = timeline(&[first, second]);
        assert_eq!(beats.len(), 6);
        assert_eq!(
            (beats[0].iteration, beats[0].station),
            (0, LoopStation::Gather)
        );
        assert_eq!((beats[1].station, beats[1].call), (LoopStation::Act, 0));
        assert_eq!((beats[2].station, beats[2].call), (LoopStation::Act, 1));
        assert_eq!(
            (beats[3].iteration, beats[3].station),
            (0, LoopStation::Verify)
        );
        assert_eq!(
            (beats[4].iteration, beats[4].station),
            (1, LoopStation::Gather)
        );
        assert_eq!(
            (beats[5].iteration, beats[5].station),
            (1, LoopStation::Verify)
        );
    }
    #[test]
    fn paused_motion_keeps_the_same_timeline_position() {
        let (before_pause, moving_trail) = loop_pose(0.42, false, true, true);
        let (during_pause, paused_trail) = loop_pose(0.42, false, true, false);
        assert_eq!(before_pause, during_pause);
        assert!(before_pause > 0. && before_pause < 1.);
        assert!(moving_trail > 0.);
        assert_eq!(paused_trail, 0.);
        assert_eq!(loop_pose(0.42, true, true, true), (0., 0.));
    }
}
