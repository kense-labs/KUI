//! Replay supplied speculative-decoding rounds without inventing model results.
use crate::primitives::{
    family::{Paint, PhaseClock, Tone, disabled_motion, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
#[derive(Clone, Debug)]
pub struct DecodeRound {
    pub draft: Vec<SharedString>,
    pub accepted: usize,
    pub target: Option<SharedString>,
}
impl DecodeRound {
    pub fn new(draft: Vec<SharedString>, accepted: usize, target: Option<SharedString>) -> Self {
        Self {
            accepted: accepted.min(draft.len()),
            draft,
            target,
        }
    }
    fn accepted(&self) -> usize {
        self.accepted.min(self.draft.len())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeStatus {
    Ready,
    Loading,
    Failed,
}
#[derive(Clone, Debug)]
pub struct DecodeStats {
    pub passes: usize,
    pub drafted: usize,
    pub accepted: usize,
    pub tokens: usize,
    pub acceptance: f32,
    pub tokens_per_pass: f32,
    pub speedup: f32,
}
#[derive(Clone, Debug)]
pub struct DecodeCompleted(pub DecodeStats);
#[derive(Clone, Debug)]
pub struct DecodeRetry;
#[derive(Clone, Debug)]
pub struct DecodePlaybackChanged {
    pub playing: bool,
}
pub struct SpeculativeDecoding {
    rounds: Vec<DecodeRound>,
    status: DecodeStatus,
    error: SharedString,
    recorded: bool,
    allow_retry: bool,
    compare: bool,
    elapsed: f32,
    last_age: f32,
    clock: PhaseClock,
    visible: bool,
    playing: bool,
    autoplay: bool,
    started: bool,
    completed: bool,
    draft_cost: f32,
}
impl EventEmitter<DecodeCompleted> for SpeculativeDecoding {}
impl EventEmitter<DecodeRetry> for SpeculativeDecoding {}
impl EventEmitter<DecodePlaybackChanged> for SpeculativeDecoding {}
fn round_duration(round: &DecodeRound) -> f32 {
    let landing_slots = round.accepted() + usize::from(round.target.is_some());
    0.14 * round.draft.len() as f32 + 0.64 + 0.18 + 0.02 * landing_slots.min(16) as f32
}
fn ease(value: f32) -> f32 {
    let t = value.clamp(0., 1.);
    1. - (1. - t).powi(3)
}
fn total_duration(rounds: &[DecodeRound]) -> f32 {
    rounds.iter().map(round_duration).sum()
}
fn stats(rounds: &[DecodeRound], draft_cost: f32) -> DecodeStats {
    let passes = rounds.len();
    let drafted: usize = rounds.iter().map(|r| r.draft.len()).sum();
    let accepted: usize = rounds.iter().map(DecodeRound::accepted).sum();
    let tokens = accepted + rounds.iter().filter(|r| r.target.is_some()).count();
    let cost = passes as f32 + drafted as f32 * draft_cost.max(0.);
    DecodeStats {
        passes,
        drafted,
        accepted,
        tokens,
        acceptance: if drafted > 0 {
            accepted as f32 / drafted as f32
        } else {
            0.
        },
        tokens_per_pass: if passes > 0 {
            tokens as f32 / passes as f32
        } else {
            0.
        },
        speedup: if cost > 0. { tokens as f32 / cost } else { 0. },
    }
}
fn output(rounds: &[DecodeRound]) -> String {
    let mut result = String::new();
    for round in rounds {
        for token in round.draft.iter().take(round.accepted()) {
            result.push_str(token);
        }
        if let Some(target) = &round.target {
            result.push_str(target);
        }
    }
    result
}
impl SpeculativeDecoding {
    pub fn new(rounds: Vec<DecodeRound>) -> Self {
        Self {
            rounds,
            status: DecodeStatus::Ready,
            error: "".into(),
            recorded: false,
            allow_retry: false,
            compare: false,
            elapsed: 0.,
            last_age: 0.,
            clock: PhaseClock::default(),
            visible: true,
            playing: false,
            autoplay: true,
            started: false,
            completed: false,
            draft_cost: 0.1,
        }
    }
    pub fn with_recorded(mut self, recorded: bool) -> Self {
        self.recorded = recorded;
        self
    }
    pub fn with_autoplay(mut self, autoplay: bool) -> Self {
        self.autoplay = autoplay;
        self
    }
    pub fn with_retry(mut self, enabled: bool) -> Self {
        self.allow_retry = enabled;
        self
    }
    pub fn with_draft_cost(mut self, cost: f32) -> Self {
        if cost.is_finite() {
            self.draft_cost = cost.max(0.);
        }
        self
    }
    pub fn set_rounds(&mut self, rounds: Vec<DecodeRound>, cx: &mut Context<Self>) {
        self.rounds = rounds;
        self.elapsed = 0.;
        self.last_age = 0.;
        self.completed = false;
        self.clock.restart(cx);
        if self.rounds.is_empty() {
            self.playing = false;
        }
        cx.notify();
    }
    pub fn set_status(
        &mut self,
        status: DecodeStatus,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.status = status;
        self.error = error.into();
        if status != DecodeStatus::Ready {
            self.playing = false;
        }
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_age = 0.;
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.status != DecodeStatus::Ready || self.rounds.is_empty() {
            return;
        }
        if playing && self.elapsed >= total_duration(&self.rounds) {
            self.elapsed = 0.;
            self.completed = false;
        }
        if self.playing != playing {
            self.playing = playing;
            self.started = true;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.emit(DecodePlaybackChanged { playing });
            cx.notify();
        }
    }
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        self.elapsed = 0.;
        self.last_age = 0.;
        self.completed = false;
        self.started = true;
        self.playing = self.status == DecodeStatus::Ready && !self.rounds.is_empty();
        self.clock.restart(cx);
        cx.notify();
    }
    pub fn set_compare(&mut self, compare: bool, cx: &mut Context<Self>) {
        if self.compare != compare {
            self.compare = compare;
            cx.notify();
        }
    }
}
impl Render for SpeculativeDecoding {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let total = total_duration(&self.rounds);
        if self.autoplay
            && !self.started
            && self.visible
            && w.is_visible()
            && !reduced
            && self.status == DecodeStatus::Ready
            && !self.rounds.is_empty()
        {
            self.started = true;
            self.playing = true;
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let running = self.visible
            && w.is_visible()
            && self.playing
            && self.status == DecodeStatus::Ready
            && self.elapsed < total;
        if running {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
            }
            let age = self.clock.age(cx);
            self.elapsed = (self.elapsed + (age - self.last_age).clamp(0., 0.1)).min(total);
            self.last_age = age;
            if self.elapsed < total {
                w.request_animation_frame();
            } else if !self.completed {
                self.completed = true;
                self.playing = false;
                cx.emit(DecodeCompleted(stats(&self.rounds, self.draft_cost)));
            }
        } else {
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let mut remaining = self.elapsed;
        let mut current = None;
        let mut done = 0usize;
        for (i, round) in self.rounds.iter().enumerate() {
            let duration = round_duration(round);
            if remaining >= duration {
                remaining -= duration;
                done = i + 1;
            } else {
                current = Some((i, remaining));
                break;
            }
        }
        let visible_rounds = &self.rounds[..done];
        let written = output(visible_rounds);
        let spoken_output = if written.is_empty() {
            tr(cx, "No output yet", "尚无输出").to_string()
        } else {
            written.clone()
        };
        let text = output(&self.rounds);
        let current_round = current.and_then(|(i, age)| self.rounds.get(i).map(|r| (r, age)));
        let (draft, age) = current_round.map_or((&[][..], 0.), |(r, a)| (r.draft.as_slice(), a));
        let verify_progress =
            ((age - draft.len() as f32 * 0.14) / 0.64).clamp(0., 1.) * draft.len() as f32;
        let accepted = current_round.map_or(0, |(r, _)| r.accepted());
        let verified = if age > draft.len() as f32 * 0.14 {
            ((((age - draft.len() as f32 * 0.14) / 0.64) * draft.len() as f32).ceil() as usize)
                .min(draft.len())
        } else {
            0
        };
        let replacement = current_round.and_then(|(round, age)| {
            let reached = if round.draft.is_empty() {
                age >= 0.64
            } else if accepted < round.draft.len() {
                verified > accepted
            } else {
                verified == round.draft.len()
            };
            if reached { round.target.clone() } else { None }
        });
        let target_alpha = if reduced {
            1.
        } else if draft.is_empty() {
            ease((age - 0.64) / 0.14)
        } else {
            ease(verify_progress - accepted.min(draft.len().saturating_sub(1)) as f32)
        };
        let values = stats(visible_rounds, self.draft_cost);
        let chips = row()
            .flex_wrap()
            .gap_1()
            .children(draft.iter().enumerate().map(|(i, token)| {
                let entrance = if reduced {
                    f32::from(age >= i as f32 * 0.14)
                } else {
                    ease((age - i as f32 * 0.14) / 0.14)
                };
                let sweep = if reduced {
                    f32::from(verify_progress > i as f32)
                } else {
                    ease(verify_progress - i as f32)
                };
                let verdict = if i < verified {
                    if i < accepted {
                        "✓"
                    } else if i == accepted {
                        "×"
                    } else {
                        "–"
                    }
                } else {
                    ""
                };
                div()
                    .relative()
                    .left(px(if reduced { 0. } else { (1. - entrance) * -4. }))
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.hover)
                    .opacity(entrance)
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .bottom_0()
                            .w(relative(sweep))
                            .bg(p.bright.opacity(0.12)),
                    )
                    .child(
                        row()
                            .items_center()
                            .gap_1()
                            .child(div().relative().child(token.clone()).when(
                                i == accepted && i < verified,
                                |v| {
                                    v.child(
                                        div()
                                            .absolute()
                                            .left_0()
                                            .top(relative(0.5))
                                            .h(px(1.))
                                            .w(relative(sweep))
                                            .bg(p.danger),
                                    )
                                },
                            ))
                            .child(
                                div()
                                    .opacity(sweep)
                                    .text_color(if i == accepted && i < verified {
                                        p.danger
                                    } else {
                                        p.secondary
                                    })
                                    .child(verdict),
                            ),
                    )
            }));
        let token_slots = self
            .rounds
            .iter()
            .enumerate()
            .flat_map(|(round_index, round)| {
                let mut parts: Vec<(SharedString, bool)> = round
                    .draft
                    .iter()
                    .take(round.accepted())
                    .cloned()
                    .map(|s| (s, true))
                    .collect();
                if let Some(target) = round.target.clone() {
                    parts.push((target, false));
                }
                parts
                    .into_iter()
                    .enumerate()
                    .map(move |(slot, (s, from_draft))| (round_index, slot, s, from_draft))
            })
            .collect::<Vec<_>>();
        let output_line = div()
            .id("decoding-output")
            .role(Role::Label)
            .aria_label(spoken_output)
            .flex()
            .flex_wrap()
            .children(token_slots.iter().map(|(round, slot, token, from_draft)| {
                let landing = if *round < done {
                    1.
                } else if current.is_some_and(|(i, _)| i == *round) {
                    let start = draft.len() as f32 * 0.14 + 0.64 + (*slot).min(16) as f32 * 0.02;
                    if reduced {
                        f32::from(age >= start)
                    } else {
                        ease((age - start) / 0.18)
                    }
                } else {
                    0.
                };
                div()
                    .relative()
                    .top(px(if reduced { 0. } else { (1. - landing) * -4. }))
                    .text_color(if *from_draft { p.bright } else { p.ink })
                    .opacity(landing)
                    .child(token.clone())
            }));
        let baseline_count = if self.compare {
            ((self.elapsed / 0.64).floor() as usize).min(token_slots.len())
        } else {
            0
        };
        let baseline = div()
            .flex()
            .flex_wrap()
            .children(token_slots.iter().enumerate().map(|(i, (_, _, token, _))| {
                div()
                    .opacity(if i < baseline_count { 1. } else { 0. })
                    .child(token.clone())
            }));
        let ready = self.status == DecodeStatus::Ready;
        column()
            .w_full()
            .max_w(px(720.))
            .gap(px(20.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(title(tr(cx, "Speculative decoding", "推测解码"), p))
                    .child(hint(
                        if self.recorded {
                            tr(cx, "Recorded run", "回放记录")
                        } else {
                            tr(cx, "Illustrative data", "示意数据")
                        },
                        p,
                    )),
            )
            .child(hint(
                tr(
                    cx,
                    "A draft proposes tokens; the target verifies and writes the result.",
                    "草稿模型提出 token，目标模型验证后写入结果。",
                ),
                p,
            ))
            .child(
                column()
                    .gap_2()
                    .child(hint(tr(cx, "Draft model", "草稿模型"), p))
                    .child(chips),
            )
            .child(
                column()
                    .gap_1()
                    .child(hint(tr(cx, "Target model", "目标模型"), p))
                    .child(hint(
                        if verified > 0 {
                            tr(cx, "Checking guesses", "正在验证")
                        } else {
                            tr(cx, "Waiting", "等待")
                        },
                        p,
                    ))
                    .when_some(replacement, |v, token| {
                        v.child(
                            div()
                                .relative()
                                .left(px(if reduced {
                                    0.
                                } else {
                                    (1. - target_alpha) * -4.
                                }))
                                .opacity(target_alpha)
                                .text_color(p.bright)
                                .child(format!("✓ {token}")),
                        )
                    }),
            )
            .child(
                column()
                    .gap_1()
                    .child(hint(
                        format!(
                            "{} · {} {}",
                            tr(cx, "Output", "输出"),
                            done,
                            tr(cx, "passes", "次验证")
                        ),
                        p,
                    ))
                    .child(output_line),
            )
            .when(self.compare, |v| {
                v.child(
                    column()
                        .gap_1()
                        .child(hint(
                            tr(cx, "Target only (illustrative)", "仅目标模型（示意）"),
                            p,
                        ))
                        .child(baseline),
                )
            })
            .child(
                row().gap_3().flex_wrap().children(
                    [
                        (
                            tr(cx, "Estimated speedup", "估算加速"),
                            if values.passes > 0 {
                                format!("{:.2}×", values.speedup)
                            } else {
                                "—".into()
                            },
                        ),
                        (
                            tr(cx, "Drafts accepted", "草稿接受率"),
                            if values.drafted > 0 {
                                format!("{:.0}%", values.acceptance * 100.)
                            } else {
                                "—".into()
                            },
                        ),
                        (
                            tr(cx, "Tokens per pass", "每次验证 token"),
                            if values.passes > 0 {
                                format!("{:.2}", values.tokens_per_pass)
                            } else {
                                "—".into()
                            },
                        ),
                    ]
                    .into_iter()
                    .map(|(label, value)| {
                        column()
                            .min_w(px(120.))
                            .flex_1()
                            .child(hint(label, p))
                            .child(value)
                    }),
                ),
            )
            .child(
                row().gap_2().flex_wrap().children(
                    [
                        (
                            "decode-play",
                            if self.playing {
                                tr(cx, "Pause", "暂停")
                            } else {
                                tr(cx, "Play", "播放")
                            },
                            0usize,
                        ),
                        ("decode-restart", tr(cx, "Restart", "重新开始"), 1usize),
                        ("decode-compare", tr(cx, "Compare", "对照"), 2usize),
                    ]
                    .into_iter()
                    .map(|(id, label, action)| {
                        face(
                            id,
                            label,
                            36.,
                            crate::foundation::surface_radius(cx).into(),
                            if action == 2 && self.compare {
                                Tone::Primary
                            } else {
                                Tone::Surface
                            },
                            cx,
                        )
                        .px(px(10.))
                        .when(!ready, |v| v.opacity(0.5))
                        .when(action == 2, |v| v.aria_toggled(self.compare.into()))
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if s.status != DecodeStatus::Ready {
                                return;
                            }
                            match action {
                                0 => s.set_playing(!s.playing, cx),
                                1 => s.restart(cx),
                                _ => s.set_compare(!s.compare, cx),
                            }
                        }))
                    }),
                ),
            )
            .when(self.status == DecodeStatus::Loading, |v| {
                v.child(hint(tr(cx, "Loading run…", "正在加载运行记录…"), p))
            })
            .when(self.status == DecodeStatus::Failed, |v| {
                v.child(
                    row()
                        .gap_2()
                        .items_center()
                        .child(div().text_color(p.danger).child(self.error.clone()))
                        .when(self.allow_retry, |r| {
                            r.child(
                                face(
                                    "decode-retry",
                                    tr(cx, "Retry", "重试"),
                                    36.,
                                    crate::foundation::surface_radius(cx).into(),
                                    Tone::Surface,
                                    cx,
                                )
                                .px(px(10.))
                                .child(tr(cx, "Retry", "重试"))
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(DecodeRetry))),
                            )
                        }),
                )
            })
            .when(text.is_empty() && ready, |v| {
                v.child(hint(tr(cx, "No rounds supplied", "没有可回放的轮次"), p))
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{DecodeRound, output, stats};
    #[test]
    fn recorded_rounds_preserve_leading_spaces_and_costs() {
        let rounds = vec![
            DecodeRound::new(
                vec!["The".into(), " build".into(), " failed".into()],
                2,
                Some(" passed".into()),
            ),
            DecodeRound::new(vec![], 0, Some(".".into())),
        ];
        assert_eq!(output(&rounds), "The build passed.");
        let s = stats(&rounds, 0.1);
        assert_eq!((s.passes, s.drafted, s.accepted, s.tokens), (2, 3, 2, 4));
        assert!((s.speedup - 4. / 2.3).abs() < 0.0001);
    }
    #[test]
    fn accepted_never_exceeds_draft_count() {
        assert_eq!(DecodeRound::new(vec!["x".into()], 10, None).accepted(), 1);
    }
}
