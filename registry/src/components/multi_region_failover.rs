//! Host-controlled region failover state with explicit request/resolve behavior.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::TAU, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Recovering,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailoverPhase {
    Balanced,
    Degraded,
    Shifting,
    Secondary,
    Recovering,
    Rebalancing,
}
impl FailoverPhase {
    fn index(self) -> usize {
        match self {
            Self::Balanced => 0,
            Self::Degraded => 1,
            Self::Shifting => 2,
            Self::Secondary => 3,
            Self::Recovering => 4,
            Self::Rebalancing => 5,
        }
    }
    fn at(index: usize) -> Self {
        [
            Self::Balanced,
            Self::Degraded,
            Self::Shifting,
            Self::Secondary,
            Self::Recovering,
            Self::Rebalancing,
        ][index.min(5)]
    }
}
#[derive(Clone, Debug)]
pub struct FailoverSnapshot {
    pub phase: FailoverPhase,
    pub primary: SharedString,
    pub secondary: SharedString,
    pub primary_health: RegionHealth,
    pub secondary_health: RegionHealth,
    pub primary_share: f32,
    pub requests_per_second: f32,
    pub failed_checks: u8,
    pub passed_checks: u8,
}
impl FailoverSnapshot {
    pub fn new(primary: impl Into<SharedString>, secondary: impl Into<SharedString>) -> Self {
        Self {
            phase: FailoverPhase::Balanced,
            primary: primary.into(),
            secondary: secondary.into(),
            primary_health: RegionHealth::Healthy,
            secondary_health: RegionHealth::Healthy,
            primary_share: 0.5,
            requests_per_second: 0.,
            failed_checks: 0,
            passed_checks: 0,
        }
    }
    fn share(&self) -> f32 {
        if self.primary_share.is_finite() {
            self.primary_share.clamp(0., 1.)
        } else {
            0.
        }
    }
    fn rate(&self) -> f32 {
        if self.requests_per_second.is_finite() {
            self.requests_per_second.max(0.)
        } else {
            0.
        }
    }
}
#[derive(Clone, Debug)]
pub struct FailoverRequest {
    pub id: u64,
    pub phase: FailoverPhase,
}
pub struct MultiRegionFailover {
    snapshot: FailoverSnapshot,
    weight: Slide,
    visible: bool,
    motion: bool,
    clock: PhaseClock,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    next_id: u64,
    pending: Option<FailoverRequest>,
    failed: Option<(FailoverPhase, SharedString)>,
}
impl EventEmitter<FailoverRequest> for MultiRegionFailover {}
impl MultiRegionFailover {
    pub fn new(snapshot: FailoverSnapshot) -> Self {
        let share = snapshot.share();
        Self {
            snapshot,
            weight: Slide::new(share),
            visible: true,
            motion: true,
            clock: PhaseClock::default(),
            bounds: Default::default(),
            next_id: 0,
            pending: None,
            failed: None,
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_motion(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.motion != enabled {
            self.motion = enabled;
            self.clock.restart(cx);
            cx.notify();
        }
    }
    pub fn set_snapshot(&mut self, snapshot: FailoverSnapshot, cx: &mut Context<Self>) {
        self.weight.duration(snapshot.share(), 0.6, cx);
        self.snapshot = snapshot;
        cx.notify();
    }
    pub fn request_phase(&mut self, phase: FailoverPhase, cx: &mut Context<Self>) {
        if self.pending.is_some() || (self.snapshot.phase == phase && self.failed.is_none()) {
            return;
        }
        self.next_id = self.next_id.wrapping_add(1);
        let request = FailoverRequest {
            id: self.next_id,
            phase,
        };
        self.failed = None;
        self.pending = Some(request.clone());
        cx.emit(request);
        cx.notify();
    }
    pub fn resolve(
        &mut self,
        id: u64,
        result: Result<FailoverSnapshot, SharedString>,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self.pending.as_ref().filter(|r| r.id == id).cloned() else {
            return;
        };
        self.pending = None;
        match result {
            Ok(snapshot) => self.set_snapshot(snapshot, cx),
            Err(message) => {
                self.failed = Some((request.phase, message));
                cx.notify();
            }
        }
    }
}
fn health_label(health: RegionHealth, cx: &App) -> &'static str {
    match health {
        RegionHealth::Healthy => tr(cx, "Healthy", "正常"),
        RegionHealth::Degraded => tr(cx, "Degraded", "性能下降"),
        RegionHealth::Unhealthy => tr(cx, "Unavailable", "不可用"),
        RegionHealth::Recovering => tr(cx, "Recovering", "恢复中"),
    }
}
fn curve(a: Point<Pixels>, b: Point<Pixels>, t: f32) -> Point<Pixels> {
    let f = t * t * (3. - 2. * t);
    point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * f)
}
fn dot(center: Point<Pixels>, radius: f32, color: Hsla, w: &mut Window) {
    let mut path = PathBuilder::fill();
    for i in 0..=12 {
        let a = TAU * i as f32 / 12.;
        let q = point(
            center.x + px(a.cos() * radius),
            center.y + px(a.sin() * radius),
        );
        if i == 0 {
            path.move_to(q)
        } else {
            path.line_to(q)
        }
    }
    path.close();
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
fn cube(center: Point<Pixels>, width: f32, tint: Hsla, p: Paint, w: &mut Window) {
    let at = |x, y| center + point(px(x * width), px(y * width));
    let north = at(0., -0.25);
    let west = at(-0.5, 0.);
    let south = at(0., 0.25);
    let east = at(0.5, 0.);
    for (corners, color) in [
        (
            [west, south, at(0., 0.49), at(-0.5, 0.24)],
            p.tray.blend(tint.opacity(0.16)),
        ),
        (
            [south, east, at(0.5, 0.24), at(0., 0.49)],
            p.panel.blend(tint.opacity(0.23)),
        ),
        (
            [north, east, south, west],
            p.panel.blend(tint.opacity(0.08)),
        ),
    ] {
        for (mut path, ink) in [
            (PathBuilder::fill(), color),
            (
                PathBuilder::stroke(px(1.)),
                p.line.blend(tint.opacity(0.36)),
            ),
        ] {
            path.move_to(corners[0]);
            for pt in &corners[1..] {
                path.line_to(*pt);
            }
            path.close();
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        }
    }
    dot(center, 3.4, tint, w);
}
impl Render for MultiRegionFailover {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let weight = if self.visible && w.is_visible() {
            self.weight.get(w, cx)
        } else {
            self.weight.value(cx)
        }
        .clamp(0., 1.);
        let moving = self.visible && self.motion && !reduced && w.is_visible();
        let age = if moving {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
            }
            w.request_animation_frame();
            self.clock.age(cx)
        } else {
            0.
        };
        let primary = self.snapshot.primary.clone();
        let secondary = self.snapshot.secondary.clone();
        let rate = self.snapshot.rate();
        let phase = self.snapshot.phase.index();
        let labels = [
            tr(cx, "Balanced", "均衡"),
            tr(cx, "Degraded", "降级"),
            tr(cx, "Fail over", "故障切换"),
            tr(cx, "One region", "单区域"),
            tr(cx, "Recover", "恢复"),
            tr(cx, "Rebalance", "重新均衡"),
        ];
        let health = [self.snapshot.primary_health, self.snapshot.secondary_health];
        let h0 = health_label(health[0], cx);
        let h1 = health_label(health[1], cx);
        let diagram_label = format!(
            "{}: {} {:.0}% {}, {} {:.0}% {}. {} {} / 3. {} {} / 3.",
            labels[phase],
            primary,
            weight * 100.,
            h0,
            secondary,
            (1. - weight) * 100.,
            h1,
            tr(cx, "Failed checks", "失败检查"),
            self.snapshot.failed_checks.min(3),
            tr(cx, "Passed checks", "通过检查"),
            self.snapshot.passed_checks.min(3)
        );
        let bounds = self.bounds.clone();
        let stage_w = f32::from(bounds.get().size.width);
        let narrow = stage_w > 0. && stage_w < 640.;
        let compact = stage_w > 0. && stage_w < 440.;
        let stage_h = if narrow {
            (stage_w * 1.46).clamp(370., 790.)
        } else if stage_w > 1. {
            stage_w * 0.62
        } else {
            460.
        };
        let diagram = div()
            .id("failover-diagram")
            .role(Role::Image)
            .aria_label(diagram_label)
            .relative()
            .w_full()
            .h(px(stage_h))
            .child(
                canvas(
                    move |b, w, _| {
                        if bounds.get().size != b.size {
                            w.request_animation_frame();
                        }
                        bounds.set(b);
                    },
                    move |b, _, w, _| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let at = |x: f32, y: f32| {
                            point(b.origin.x + px(x * width), b.origin.y + px(y * height))
                        };
                        let (user, balancer, a, z) = if narrow {
                            (at(0.5, 0.17), at(0.5, 0.43), at(0.25, 0.76), at(0.75, 0.76))
                        } else {
                            (at(0.08, 0.5), at(0.36, 0.5), at(0.78, 0.25), at(0.78, 0.75))
                        };
                        for (start, end, color) in [
                            (user, balancer, p.bright),
                            (balancer, a, if weight > 0.01 { p.bright } else { p.line }),
                            (balancer, z, if weight < 0.99 { p.bright } else { p.line }),
                        ] {
                            let mut path = PathBuilder::stroke(px(1.5));
                            path.move_to(start);
                            for i in 1..=36 {
                                path.line_to(curve(start, end, i as f32 / 36.));
                            }
                            if let Ok(path) = path.build() {
                                w.paint_path(path, p.line.blend(color.opacity(0.45)));
                            }
                            if moving {
                                for i in 0..5 {
                                    let t = (age * 0.24 + i as f32 / 5.).fract();
                                    dot(curve(start, end, t), 2.5, color, w);
                                }
                            }
                        }
                        for (index, pos) in [user, balancer, a, z].into_iter().enumerate() {
                            let color = if index < 2 {
                                p.bright
                            } else {
                                match health[index - 2] {
                                    RegionHealth::Unhealthy => p.danger,
                                    RegionHealth::Degraded | RegionHealth::Recovering => p.faint,
                                    RegionHealth::Healthy => p.success,
                                }
                            };
                            let footprint = if index < 2 {
                                width * 0.23
                            } else {
                                width * 0.32
                            };
                            cube(pos, footprint.clamp(48., 140.), color, p, w);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(if narrow { 0.25 } else { 0.01 }))
                    .w(relative(if narrow { 0.50 } else { 0.14 }))
                    .text_center()
                    .top(relative(if narrow { 0.05 } else { 0.66 }))
                    .child(hint(tr(cx, "Users", "用户"), p)),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(if narrow { 0.25 } else { 0.22 }))
                    .w(relative(if narrow { 0.50 } else { 0.28 }))
                    .text_center()
                    .top(relative(if narrow { 0.54 } else { 0.66 }))
                    .child(hint(tr(cx, "Balancer", "负载均衡"), p)),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(if narrow { 0.01 } else { 0.56 }))
                    .w(relative(if narrow { 0.48 } else { 0.44 }))
                    .text_center()
                    .top(relative(if narrow { 0.89 } else { 0.44 }))
                    .child(hint(primary.clone(), p).truncate())
                    .child(hint(h0, p).truncate()),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(if narrow { 0.51 } else { 0.56 }))
                    .w(relative(if narrow { 0.48 } else { 0.44 }))
                    .text_center()
                    .top(relative(if narrow { 0.89 } else { 0.91 }))
                    .child(hint(secondary.clone(), p).truncate())
                    .child(hint(h1, p).truncate()),
            );
        let busy = self.pending.is_some();
        let steps = div()
            .id("failover-steps")
            .role(Role::Group)
            .tab_index(0)
            .flex()
            .flex_wrap()
            .gap_1()
            .aria_label(tr(cx, "Failover stages", "故障切换阶段"))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if s.pending.is_some() {
                    return;
                }
                let i = s.snapshot.phase.index();
                let next = match e.keystroke.key.as_str() {
                    "left" => i.saturating_sub(1),
                    "right" => (i + 1).min(5),
                    "home" => 0,
                    "end" => 5,
                    _ => return,
                };
                s.request_phase(FailoverPhase::at(next), cx);
                cx.stop_propagation();
            }))
            .gap(px(6.))
            .children(labels.iter().enumerate().map(|(i, label)| {
                let current = phase == i;
                face(
                    ("failover-step", i),
                    format!("{} {}", i + 1, label),
                    44.,
                    0.,
                    Tone::Clear,
                    cx,
                )
                .flex_1()
                .h(px(44.))
                .pt(px(10.))
                .flex_col()
                .items_start()
                .justify_start()
                .text_size(px(12.))
                .line_height(px(15.6))
                .text_color(if current { p.ink } else { p.faint })
                .hover(|s| s.text_color(p.ink))
                .aria_toggled(current.into())
                .when(busy, |v| v.opacity(0.5))
                .child(format!("{} {label}", i + 1))
                .on_click(cx.listener(move |s, _, _, cx| s.request_phase(FailoverPhase::at(i), cx)))
            }));
        let error = self.failed.clone();
        column()
            .w_full()
            .max_w(px(1080.))
            .gap_3()
            .p(px(16.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(title(tr(cx, "Multi-region failover", "多区域故障切换"), p))
            .child(hint(
                tr(cx, "Host-provided region status", "区域状态由宿主提供"),
                p,
            ))
            .child(diagram)
            .child(
                row()
                    .justify_between()
                    .gap_2()
                    .child(split_side(
                        &self.snapshot.primary,
                        weight * 100.,
                        rate * weight,
                        false,
                        compact,
                        p,
                    ))
                    .child(split_side(
                        &self.snapshot.secondary,
                        (1. - weight) * 100.,
                        rate * (1. - weight),
                        true,
                        compact,
                        p,
                    )),
            )
            .child(
                div()
                    .w(px(184.))
                    .max_w_full()
                    .h(px(4.))
                    .rounded(px(2.))
                    .bg(p.line)
                    .child(
                        div()
                            .h_full()
                            .w(relative(weight))
                            .rounded(px(2.))
                            .bg(p.bright),
                    ),
            )
            .child(hint(
                format!(
                    "{} {}/3 · {} {}/3",
                    tr(cx, "Failed checks", "失败检查"),
                    self.snapshot.failed_checks.min(3),
                    tr(cx, "Passed checks", "通过检查"),
                    self.snapshot.passed_checks.min(3)
                ),
                p,
            ))
            .child(steps)
            .when(!reduced, |v| {
                v.child(
                    face(
                        "failover-motion",
                        if self.motion {
                            tr(cx, "Pause flow", "暂停流动")
                        } else {
                            tr(cx, "Resume flow", "继续流动")
                        },
                        36.,
                        18.,
                        Tone::Clear,
                        cx,
                    )
                    .mt(px(2.))
                    .w(px(36.))
                    .px(px(0.))
                    .border_1()
                    .border_color(p.line)
                    .text_color(p.secondary)
                    .hover(|s| s.bg(p.tray).text_color(p.ink))
                    .aria_toggled(self.motion.into())
                    .child(if self.motion { "❚❚" } else { "▶" })
                    .on_click(cx.listener(|s, _, _, cx| s.set_motion(!s.motion, cx))),
                )
            })
            .when(busy, |v| {
                v.child(hint(
                    tr(cx, "Waiting for host confirmation…", "等待宿主确认…"),
                    p,
                ))
            })
            .when_some(error, |v, (phase, message)| {
                v.child(
                    row()
                        .items_center()
                        .gap_2()
                        .flex_wrap()
                        .child(div().text_color(p.danger).child(message))
                        .child(
                            face(
                                "failover-retry",
                                tr(cx, "Retry", "重试"),
                                36.,
                                crate::foundation::surface_radius(cx).into(),
                                Tone::Surface,
                                cx,
                            )
                            .px(px(10.))
                            .child(tr(cx, "Retry", "重试"))
                            .on_click(cx.listener(move |s, _, _, cx| s.request_phase(phase, cx))),
                        ),
                )
            })
    }
}
fn split_side(name: &str, percent: f32, rate: f32, end: bool, compact: bool, p: Paint) -> Div {
    let label = div()
        .min_w_0()
        .truncate()
        .text_size(px(14.))
        .line_height(px(20.))
        .text_color(p.secondary)
        .child(name.to_string());
    let metrics = row()
        .gap(px(6.))
        .when(end && !compact, |side| side.flex_row_reverse())
        .child(
            div()
                .min_w(px(32.))
                .text_size(px(14.))
                .line_height(px(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(format!("{percent:.0}%")),
        )
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(18.))
                .text_color(p.faint)
                .child(format!("{rate:.0}/s")),
        );
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .gap(px(if compact { 2. } else { 8. }))
        .when(compact, |side| {
            side.flex_col().when(end, |side| side.items_end())
        })
        .when(!compact, |side| {
            side.items_baseline()
                .when(end, |side| side.flex_row_reverse())
        })
        .child(label)
        .child(metrics)
}
#[cfg(test)]
mod tests {
    use super::{FailoverPhase, FailoverSnapshot};
    #[test]
    fn invalid_rates_and_weights_are_bounded() {
        let mut s = FailoverSnapshot::new("A", "B");
        s.primary_share = f32::NAN;
        s.requests_per_second = -1.;
        assert_eq!(s.share(), 0.);
        assert_eq!(s.rate(), 0.);
        s.primary_share = 1.5;
        assert_eq!(s.share(), 1.);
    }
    #[test]
    fn stage_indices_round_trip() {
        for i in 0..6 {
            assert_eq!(FailoverPhase::at(i).index(), i);
        }
    }
}
