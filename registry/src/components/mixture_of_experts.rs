//! An illustrative, host-supplied sparse expert routing diagram.
use crate::primitives::{
    family::{disabled_motion, face, Paint, PhaseClock, Tone},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug)]
pub struct MoeExpert {
    pub id: SharedString,
    pub name: SharedString,
    pub handles: SharedString,
}
impl MoeExpert {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        handles: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            handles: handles.into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct MoeToken {
    pub text: SharedString,
    pub scores: Vec<f32>,
}
impl MoeToken {
    pub fn new(text: impl Into<SharedString>, scores: Vec<f32>) -> Self {
        Self {
            text: text.into(),
            scores,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MoeChoice {
    pub expert: usize,
    pub weight: f32,
    pub dropped: bool,
}
#[derive(Clone, Debug)]
pub struct MoeRoute {
    pub token: usize,
    pub choices: Vec<MoeChoice>,
}
#[derive(Clone, Debug)]
pub enum MoeEvent {
    Route(MoeRoute),
    TopKChanged(usize),
    PlaybackChanged(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pin {
    Token(usize),
    Expert(usize),
}
pub struct MixtureOfExperts {
    experts: Vec<MoeExpert>,
    tokens: Vec<MoeToken>,
    routes: Vec<MoeRoute>,
    loads: Vec<usize>,
    capacity: Option<usize>,
    top_k: usize,
    capacity_factor: f32,
    index: usize,
    elapsed: f32,
    last_age: f32,
    clock: PhaseClock,
    visible: bool,
    paused: bool,
    pin: Option<Pin>,
    simulated: bool,
    total_parameters: Option<f64>,
    shared_parameters: f64,
    expert_parameters: f64,
    token_focus: Vec<FocusHandle>,
    expert_focus: Vec<FocusHandle>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl EventEmitter<MoeEvent> for MixtureOfExperts {}
impl MixtureOfExperts {
    pub fn new(experts: Vec<MoeExpert>, tokens: Vec<MoeToken>) -> Self {
        let mut result = Self {
            experts,
            tokens,
            routes: vec![],
            loads: vec![],
            capacity: None,
            top_k: 2,
            capacity_factor: 1.,
            index: 0,
            elapsed: 0.,
            last_age: 0.,
            clock: PhaseClock::default(),
            visible: true,
            paused: false,
            pin: None,
            simulated: false,
            total_parameters: None,
            shared_parameters: 0.,
            expert_parameters: 0.,
            token_focus: vec![],
            expert_focus: vec![],
            bounds: Default::default(),
        };
        result.recompute();
        result
    }
    pub fn with_simulated(mut self, simulated: bool) -> Self {
        self.simulated = simulated;
        self
    }
    pub fn with_parameters(mut self, total: f64, shared: f64, per_expert: f64) -> Self {
        if total.is_finite()
            && total >= 0.
            && shared.is_finite()
            && shared >= 0.
            && per_expert.is_finite()
            && per_expert >= 0.
        {
            self.total_parameters = Some(total);
            self.shared_parameters = shared;
            self.expert_parameters = per_expert;
        }
        self
    }
    pub fn with_capacity_factor(mut self, factor: f32) -> Self {
        self.capacity_factor = if factor.is_nan() || factor < 0. {
            1.
        } else {
            factor
        };
        self.recompute();
        self
    }
    pub fn set_data(
        &mut self,
        experts: Vec<MoeExpert>,
        tokens: Vec<MoeToken>,
        cx: &mut Context<Self>,
    ) {
        self.experts = experts;
        self.tokens = tokens;
        self.pin = None;
        self.reset(cx);
        self.recompute();
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.notify();
        }
    }
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.emit(MoeEvent::PlaybackChanged(paused));
            cx.notify();
        }
    }
    pub fn set_top_k(&mut self, k: usize, cx: &mut Context<Self>) {
        let k = k.clamp(1, 2);
        if self.top_k != k {
            self.top_k = k;
            self.pin = None;
            self.reset(cx);
            self.recompute();
            cx.emit(MoeEvent::TopKChanged(k));
            cx.notify();
        }
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.index = 0;
        self.elapsed = 0.;
        self.last_age = 0.;
        self.clock.restart(cx);
    }
    fn recompute(&mut self) {
        let n = self.experts.len();
        self.capacity = if n == 0 || self.capacity_factor.is_infinite() {
            None
        } else {
            Some(
                ((self.tokens.len() as f32 * self.top_k as f32 / n as f32) * self.capacity_factor)
                    .ceil() as usize,
            )
        };
        self.loads = vec![0; n];
        self.routes.clear();
        for (index, token) in self.tokens.iter().enumerate() {
            let mut scores: Vec<(usize, f32)> = (0..n)
                .filter_map(|i| {
                    token
                        .scores
                        .get(i)
                        .copied()
                        .filter(|v| v.is_finite() && *v > 0.)
                        .map(|v| (i, v))
                })
                .collect();
            scores.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            scores.truncate(self.top_k.min(n));
            let sum: f64 = scores.iter().map(|(_, v)| *v as f64).sum();
            let choices = scores
                .into_iter()
                .map(|(expert, value)| {
                    let dropped = self
                        .capacity
                        .is_some_and(|limit| self.loads[expert] >= limit);
                    if !dropped {
                        self.loads[expert] += 1;
                    }
                    MoeChoice {
                        expert,
                        weight: if sum > 0. {
                            (value as f64 / sum) as f32
                        } else {
                            0.
                        },
                        dropped,
                    }
                })
                .collect();
            self.routes.push(MoeRoute {
                token: index,
                choices,
            });
        }
    }
    fn pin(&mut self, pin: Pin, cx: &mut Context<Self>) {
        self.pin = if self.pin == Some(pin) {
            None
        } else {
            Some(pin)
        };
        self.clock.restart(cx);
        self.last_age = 0.;
        cx.notify();
    }
    fn move_pin(
        &mut self,
        expert: bool,
        index: usize,
        key: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let n = if expert {
            self.experts.len()
        } else {
            self.tokens.len()
        };
        if n == 0 {
            return;
        }
        let next = match key {
            "left" | "up" => Some((index + n - 1) % n),
            "right" | "down" => Some((index + 1) % n),
            "home" => Some(0),
            "end" => Some(n - 1),
            "escape" => {
                self.pin = None;
                self.clock.restart(cx);
                self.last_age = 0.;
                cx.notify();
                cx.stop_propagation();
                return;
            }
            _ => None,
        };
        if let Some(i) = next {
            self.pin = Some(if expert {
                Pin::Expert(i)
            } else {
                Pin::Token(i)
            });
            self.clock.restart(cx);
            self.last_age = 0.;
            let focus = if expert {
                self.expert_focus.get(i)
            } else {
                self.token_focus.get(i)
            };
            if let Some(focus) = focus {
                focus.focus(w, cx);
            }
            cx.notify();
            cx.stop_propagation();
        }
    }
}
fn expert_position(index: usize, count: usize, narrow: bool) -> (f32, f32) {
    if narrow {
        let cols = count.clamp(1, 4);
        let row = index / cols;
        let rows = count.max(1).div_ceil(cols);
        let col = index % cols;
        let in_row = (count - row * cols).min(cols).max(1);
        (
            (col + 1) as f32 / (in_row + 1) as f32,
            0.44 + (row as f32 + 0.5) * 0.34 / rows as f32,
        )
    } else {
        (
            0.65,
            0.10 + (index as f32 + 0.5) * 0.80 / count.max(1) as f32,
        )
    }
}
fn curve_point(a: Point<Pixels>, b: Point<Pixels>, t: f32, vertical: bool) -> Point<Pixels> {
    let t = t.clamp(0., 1.);
    let eased = t * t * (3. - 2. * t);
    if vertical {
        point(a.x + (b.x - a.x) * eased, a.y + (b.y - a.y) * t)
    } else {
        point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * eased)
    }
}
fn paint_curve(a: Point<Pixels>, b: Point<Pixels>, vertical: bool, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::stroke(px(1.));
    path.move_to(a);
    for index in 1..=24 {
        path.line_to(curve_point(a, b, index as f32 / 24., vertical));
    }
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
fn paint_packet(pos: Point<Pixels>, color: Hsla, w: &mut Window) {
    w.paint_quad(
        fill(
            Bounds {
                origin: point(pos.x - px(3.), pos.y - px(3.)),
                size: size(px(6.), px(6.)),
            },
            color,
        )
        .corner_radii(px(3.)),
    );
}
impl Render for MixtureOfExperts {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while self.token_focus.len() < self.tokens.len() {
            self.token_focus.push(cx.focus_handle().tab_stop(true));
        }
        while self.expert_focus.len() < self.experts.len() {
            self.expert_focus.push(cx.focus_handle().tab_stop(true));
        }
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let count = self.tokens.len();
        let playing = self.visible
            && w.is_visible()
            && !self.paused
            && self.pin.is_none()
            && !reduced
            && count > 0;
        if playing {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
                self.last_age = 0.;
            }
            let age = self.clock.age(cx);
            if age.is_finite() {
                self.elapsed += (age - self.last_age).clamp(0., 0.1);
                self.last_age = age;
            }
            if self.elapsed >= 2.2 {
                self.elapsed -= 2.2;
                self.index = (self.index + 1) % count;
                if let Some(route) = self.routes.get(self.index).cloned() {
                    cx.emit(MoeEvent::Route(route));
                }
            }
            w.request_animation_frame();
        } else {
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let index = match self.pin {
            Some(Pin::Token(i)) => i.min(count.saturating_sub(1)),
            _ => self.index.min(count.saturating_sub(1)),
        };
        let route = self.routes.get(index);
        let token = self.tokens.get(index);
        let mut tokens = row().gap_1().flex_wrap().pt(px(16.)).border_t_1().border_color(p.line);
        for (i, token) in self.tokens.iter().enumerate() {
            let selected = index == i;
            let done = i < index && self.pin.is_none();
            let pinned = self.pin == Some(Pin::Token(i));
            let desc = self.routes.get(i).map_or_else(String::new, |route| {
                route
                    .choices
                    .iter()
                    .filter_map(|c| {
                        self.experts.get(c.expert).map(|e| {
                            format!(
                                "{} {:.0}%{}",
                                e.name,
                                c.weight * 100.,
                                if c.dropped {
                                    tr(cx, " full", " 已满")
                                } else {
                                    ""
                                }
                            )
                        })
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            tokens = tokens.child(
                face(
                    ("moe-token", i),
                    format!("{}: {}", token.text, desc),
                    28.,
                    8.,
                    Tone::Clear,
                    cx,
                )
                .h(px(28.))
                .px(px(8.))
                .rounded(px(8.))
                .border_1()
                .border_color(if pinned || selected {
                    if pinned {
                        p.ink
                    } else {
                        p.faint.blend(p.ink.opacity(0.40))
                    }
                } else {
                    gpui::transparent_black()
                })
                .bg(if done && !pinned && !selected {
                    p.tray
                } else if pinned || selected {
                    p.panel
                } else {
                    gpui::transparent_black()
                })
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(if pinned || selected || done { p.ink } else { p.faint })
                .when(!pinned && !selected && !done, |token| {
                    token.hover(|s| s.border_color(p.faint).text_color(p.ink))
                })
                .track_focus(&self.token_focus[i])
                .aria_toggled(pinned.into())
                .child(token.text.clone())
                .on_click(cx.listener(move |s, _, _, cx| s.pin(Pin::Token(i), cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.pin(Pin::Token(i), cx);
                        cx.stop_propagation();
                    } else {
                        s.move_pin(false, i, e.keystroke.key.as_str(), w, cx);
                    }
                })),
            );
        }
        let detail = match self.pin {
            Some(Pin::Expert(i)) => self
                .experts
                .get(i)
                .map(|e| e.handles.clone())
                .unwrap_or_default(),
            _ => token
                .map(|t| t.text.clone())
                .unwrap_or_else(|| tr(cx, "No tokens", "暂无 token").into()),
        };
        let token_text = token.map(|t| t.text.clone()).unwrap_or_else(|| "—".into());
        let width = f32::from(self.bounds.get().size.width);
        let narrow = width > 0. && width < 568.;
        let cycle = if narrow { 1.8 } else { 2.2 };
        let phase = if reduced {
            1.
        } else {
            (self.elapsed / cycle).clamp(0., 1.)
        };
        let gate = if narrow { 0. } else { 0.5 / cycle };
        let expert_at = if narrow { 0.6 / cycle } else { 1.16 / cycle };
        let out_at = if narrow { 1.1 / cycle } else { 1.86 / cycle };
        let expert_count = self.experts.len();
        let tiles = expert_count.max(1) as f32;
        let stage_height = if narrow {
            286.
        } else {
            42. + 28. * tiles + (tiles - 1.) * 4.
        };
        let bounds = self.bounds.clone();
        let selected_routes = route.map_or_else(Vec::new, |r| r.choices.clone());
        let mut stage = div()
            .id("moe-stage")
            .relative()
            .w_full()
            .h(px(stage_height));
        stage = stage.child(
            canvas(
                move |b, w, _| {
                    if bounds.get().size != b.size {
                        w.request_animation_frame();
                    }
                    bounds.set(b);
                },
                move |b, _, w, _| {
                    let at = |xy: (f32, f32)| {
                        point(
                            b.origin.x + b.size.width * xy.0,
                            b.origin.y + b.size.height * xy.1,
                        )
                    };
                    let (input, router, output) = if narrow {
                        ((0.5, 0.08), (0.5, 0.26), (0.5, 0.92))
                    } else {
                        ((0.08, 0.5), (0.28, 0.5), (0.93, 0.5))
                    };
                    let (input, router, output) = (at(input), at(router), at(output));
                    paint_curve(input, router, narrow, p.line, w);
                    for i in 0..expert_count {
                        let expert = at(expert_position(i, expert_count, narrow));
                        let choice = selected_routes.iter().find(|choice| choice.expert == i);
                        let color = match choice {
                            Some(choice) if choice.dropped => p.danger.opacity(0.7),
                            Some(_) => p.bright.opacity(0.8),
                            None => p.line,
                        };
                        paint_curve(router, expert, narrow, color, w);
                        paint_curve(
                            expert,
                            output,
                            narrow,
                            color.opacity(if choice.is_some() { 0.8 } else { 0.45 }),
                            w,
                        );
                    }
                    if !reduced && phase < out_at {
                        if phase < gate {
                            paint_packet(
                                curve_point(input, router, phase / gate, narrow),
                                p.bright,
                                w,
                            );
                        } else {
                            for choice in selected_routes.iter().filter(|choice| !choice.dropped) {
                                if choice.expert >= expert_count {
                                    continue;
                                }
                                let expert =
                                    at(expert_position(choice.expert, expert_count, narrow));
                                let packet = if phase < expert_at {
                                    curve_point(
                                        router,
                                        expert,
                                        (phase - gate) / (expert_at - gate),
                                        narrow,
                                    )
                                } else {
                                    curve_point(
                                        expert,
                                        output,
                                        (phase - expert_at) / (out_at - expert_at),
                                        narrow,
                                    )
                                };
                                paint_packet(packet, p.bright, w);
                            }
                        }
                    }
                },
            )
            .absolute()
            .inset_0(),
        );
        let (input_pos, router_pos, output_pos) = if narrow {
            ((0.5, 0.08), (0.5, 0.26), (0.5, 0.92))
        } else {
            ((0.08, 0.5), (0.28, 0.5), (0.93, 0.5))
        };
        for (pos, label) in [
            (input_pos, token_text),
            (router_pos, tr(cx, "Router", "路由器").into()),
            (output_pos, tr(cx, "Output", "输出").into()),
        ] {
            let chip_radius = if label.as_ref() == "Router" || label.as_ref() == "路由器" {
                8.
            } else {
                9.
            };
            stage = stage.child(
                div()
                    .absolute()
                    .left(relative(pos.0))
                    .top(relative(pos.1))
                    .ml(px(-34.))
                    .mt(px(-14.))
                    .w(px(68.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(px(chip_radius))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.tray)
                    .text_sm()
                    .child(label),
            );
        }
        for (i, expert) in self.experts.iter().enumerate() {
            let choice =
                route.and_then(|r| r.choices.iter().enumerate().find(|(_, v)| v.expert == i));
            let (choice_text, weight, dropped) = match choice {
                Some((rank, c)) => (
                    if rank == 0 {
                        tr(cx, "First", "首选")
                    } else {
                        tr(cx, "Second", "次选")
                    },
                    c.weight,
                    c.dropped,
                ),
                None => (tr(cx, "Not selected", "未选"), 0., false),
            };
            let caption = format!(
                "{} · {} · {} {}{}",
                expert.name,
                choice_text,
                self.loads.get(i).copied().unwrap_or(0),
                tr(cx, "routes", "条路由"),
                if dropped {
                    tr(cx, " · full", " · 已满")
                } else {
                    ""
                }
            );
            let pinned = self.pin == Some(Pin::Expert(i));
            let rank = choice.as_ref().map(|(rank, _)| *rank);
            let rank_color = match rank {
                Some(0) => p.bright,
                Some(_) => p.success,
                None => p.line,
            };
            let name: SharedString = if narrow {
                expert
                    .name
                    .chars()
                    .next()
                    .map_or_else(|| "?".into(), |c| c.to_string().into())
            } else {
                expert.name.clone()
            };
            let bar = div()
                .w_full()
                .h(px(2.))
                .bg(p.faint.opacity(0.28))
                .child(
                    div()
                        .h_full()
                        .w(relative(weight.clamp(0., 1.)))
                        .bg(if dropped {
                            p.danger.opacity(0.7)
                        } else if rank.is_some() {
                            rank_color.opacity(0.7)
                        } else {
                            p.secondary.opacity(0.55)
                        }),
                );
            let pos = expert_position(i, expert_count, narrow);
            let tile_width = if narrow { 54. } else { 94. };
            stage = stage.child(
                face(
                    ("moe-expert", i),
                    caption,
                    28.,
                    if narrow { 9. } else { 10. },
                    Tone::Clear,
                    cx,
                )
                .absolute()
                .left(relative(pos.0))
                .top(relative(pos.1))
                .ml(px(-tile_width / 2.))
                .mt(px(-14.))
                .w(px(tile_width))
                .px(px(if narrow { 0. } else { 10. }))
                .gap(px(if narrow { 6. } else { 8. }))
                .border_1()
                .border_color(if dropped {
                    p.line.blend(p.danger.opacity(0.55))
                } else if rank.is_some() {
                    p.line.blend(rank_color.opacity(0.55))
                } else if pinned {
                    p.faint
                } else {
                    p.line
                })
                .when(dropped, |tile| tile.border_dashed())
                .bg(if pinned {
                    p.tray
                } else if rank.is_some() {
                    p.panel.blend(rank_color.opacity(0.09))
                } else {
                    p.panel
                })
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(p.ink)
                .when(!pinned && rank.is_none() && !dropped, |tile| {
                    tile.hover(|s| s.border_color(p.faint))
                })
                .track_focus(&self.expert_focus[i])
                .aria_toggled(pinned.into())
                .child(column().w_full().items_center().child(name).child(bar))
                .on_click(cx.listener(move |s, _, _, cx| s.pin(Pin::Expert(i), cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.pin(Pin::Expert(i), cx);
                        cx.stop_propagation();
                    } else {
                        s.move_pin(true, i, e.keystroke.key.as_str(), w, cx);
                    }
                })),
            );
        }
        let route_description = route.map_or_else(
            || tr(cx, "No route", "暂无路由").to_string(),
            |r| {
                r.choices
                    .iter()
                    .enumerate()
                    .filter_map(|(rank, choice)| {
                        self.experts.get(choice.expert).map(|expert| {
                            format!(
                                "{} {} {:.0}%{}",
                                if rank == 0 {
                                    tr(cx, "First", "首选")
                                } else {
                                    tr(cx, "Second", "次选")
                                },
                                expert.name,
                                choice.weight * 100.,
                                if choice.dropped {
                                    tr(cx, " · full", " · 已满")
                                } else {
                                    ""
                                }
                            )
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("   ")
            },
        );
        let active = self.total_parameters.map(|total| {
            format!(
                "{:.1}B / {total:.1}B {}",
                self.shared_parameters + self.top_k as f64 * self.expert_parameters,
                tr(cx, "parameters active per token", "参数每 token 激活")
            )
        });
        column()
            .id("moe-card")
            .role(Role::Group)
            .aria_label(tr(cx, "Mixture of experts routing", "专家混合路由"))
            .w_full()
            .max_w(px(760.))
            .gap(px(20.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .w_full()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(title(tr(cx, "Expert routing", "专家路由"), p))
                    .when_some(active, |v, a| v.child(hint(a, p))),
            )
            .child(row().gap_2().flex_wrap().children((1..=2).map(|k| {
                face(
                    ("moe-k", k),
                    format!("Top {k}"),
                    32.,
                    crate::foundation::surface_radius(cx).into(),
                    if self.top_k == k {
                        Tone::Primary
                    } else {
                        Tone::Surface
                    },
                    cx,
                )
                .px(px(10.))
                .aria_toggled((self.top_k == k).into())
                .child(format!("Top {k}"))
                .on_click(cx.listener(move |s, _, _, cx| s.set_top_k(k, cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                    let next = match e.keystroke.key.as_str() {
                        "enter" | "space" => Some(k),
                        "left" | "home" => Some(1),
                        "right" | "end" => Some(2),
                        _ => None,
                    };
                    if let Some(next) = next {
                        s.set_top_k(next, cx);
                        cx.stop_propagation();
                    }
                }))
            })))
            .child(tokens)
            .child(stage)
            .child(hint(route_description, p))
            .child(hint(detail, p))
            .child(
                face(
                    "moe-play",
                    if self.paused {
                        tr(cx, "Resume", "继续")
                    } else {
                        tr(cx, "Pause", "暂停")
                    },
                    32.,
                    16.,
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .child(if self.paused {
                    tr(cx, "Resume", "继续")
                } else {
                    tr(cx, "Pause", "暂停")
                })
                .on_click(cx.listener(|s, _, _, cx| s.set_paused(!s.paused, cx)))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.set_paused(!s.paused, cx);
                        cx.stop_propagation();
                    }
                })),
            )
            .when(self.simulated, |v| {
                v.child(hint(
                    tr(
                        cx,
                        "Illustrative scores and parameter counts",
                        "模拟分数与参数量",
                    ),
                    p,
                ))
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{MixtureOfExperts, MoeExpert, MoeToken};
    #[test]
    fn deterministic_ties_and_capacity() {
        let experts = vec![MoeExpert::new("a", "A", ""), MoeExpert::new("b", "B", "")];
        let tokens = vec![
            MoeToken::new("x", vec![1., 1.]),
            MoeToken::new("y", vec![1., 1.]),
        ];
        let m = MixtureOfExperts::new(experts, tokens).with_capacity_factor(0.5);
        assert_eq!(m.routes[0].choices[0].expert, 0);
        assert!(!m.routes[0].choices[0].dropped);
        assert!(m.routes[1].choices[0].dropped);
    }
}
