//! Inspect how different parts of a conversation consume its context budget.
use crate::primitives::family::{Paint, PhaseClock, Slide, disabled_motion, standard_ease};
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
pub struct Band {
    pub label: String,
    pub tokens: u64,
    pub color: Hsla,
}
pub enum CapacityAction {
    Message,
    Attach,
    Compact,
}
const CELL_COUNT: usize = 200;
const CELL_DELAY: f32 = 0.006;
const CELL_FADE: f32 = 0.720;

struct Compaction {
    from: Vec<(Hsla, f32)>,
    first: usize,
    last: usize,
    clock: PhaseClock,
}
impl Compaction {
    fn paint(&self, index: usize, target: Hsla, elapsed: f32) -> (Hsla, f32) {
        let old = self.from[index];
        if index < self.first || index > self.last || old == (target, 1.) {
            return (target, 1.);
        }
        let t = (elapsed - (index - self.first) as f32 * CELL_DELAY) / CELL_FADE;
        if t <= 0. {
            return old;
        }
        if t < 0.55 {
            return (old.0, old.1 + (0.35 - old.1) * standard_ease(t / 0.55));
        }
        if t < 1. {
            let mix = standard_ease((t - 0.55) / 0.45);
            return (old.0.blend(target.opacity(mix)), 0.35 + 0.15 * mix);
        }
        (
            target,
            0.5 + 0.5 * standard_ease((t - 1.) * CELL_FADE / 0.14),
        )
    }
}

pub struct CapacityBands {
    id: SharedString,
    pub model: String,
    pub capacity: u64,
    pub reserved: u64,
    pub threshold: f32,
    pub bands: Vec<Band>,
    selected: Option<usize>,
    hovered: Option<usize>,
    values: Vec<(u64, Slide)>,
    painted_cells: Vec<(Hsla, f32)>,
    wave: Option<Compaction>,
    start_wave: bool,
    freed: u64,
}
impl EventEmitter<CapacityAction> for CapacityBands {}
impl CapacityBands {
    pub fn new(
        id: impl Into<SharedString>,
        model: impl Into<String>,
        capacity: u64,
        bands: Vec<Band>,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            capacity,
            reserved: 0,
            threshold: 0.8,
            values: bands
                .iter()
                .map(|b| (b.tokens, Slide::new(b.tokens as f32)))
                .collect(),
            painted_cells: Vec::with_capacity(CELL_COUNT),
            wave: None,
            start_wave: false,
            freed: 0,
            hovered: None,
            bands,
            selected: None,
        }
    }
    /// Call after the host commits a smaller allocation. Failed or empty compactions
    /// leave the previous presentation alone.
    pub fn did_compact(&mut self, previous_used: u64, cx: &mut Context<Self>) {
        let freed = previous_used.saturating_sub(self.used());
        if freed > 0 {
            self.freed = freed;
            self.start_wave = true;
            cx.notify();
        }
    }

    pub fn used(&self) -> u64 {
        self.bands
            .iter()
            .fold(0u64, |n, b| n.saturating_add(b.tokens))
    }
}
impl Render for CapacityBands {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let limit = self.capacity.max(1);
        if self.values.len() != self.bands.len() {
            self.values = self
                .bands
                .iter()
                .map(|b| (b.tokens, Slide::new(b.tokens as f32)))
                .collect()
        }
        let displayed: Vec<u64> = self
            .bands
            .iter()
            .zip(&mut self.values)
            .map(|(b, (target, curve))| {
                if *target != b.tokens {
                    *target = b.tokens;
                    curve.duration(b.tokens as f32, 0.36, cx)
                }
                curve.get(w, cx).round() as u64
            })
            .collect();
        let used = displayed.iter().copied().fold(0u64, u64::saturating_add);
        let highlighted = self.hovered.or(self.selected);

        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        let cell = |index: usize, boundaries: &[u64]| {
            let pos = index as f64 / CELL_COUNT as f64 * limit as f64;
            let mut end = 0u64;
            let which = boundaries.iter().position(|tokens| {
                end = end.saturating_add(*tokens);
                pos < end as f64
            });
            let color = which.map(|i| self.bands[i].color).unwrap_or_else(|| {
                if pos >= limit.saturating_sub(self.reserved) as f64 {
                    p.faint
                } else {
                    p.stage
                }
            });
            (which, color)
        };
        let target: Vec<_> = self.bands.iter().map(|b| b.tokens).collect();
        if std::mem::take(&mut self.start_wave)
            && self.painted_cells.len() == CELL_COUNT
            && !disabled_motion(cx)
        {
            let mut changed =
                (0..CELL_COUNT).filter(|&i| self.painted_cells[i] != (cell(i, &target).1, 1.));
            if let Some(first) = changed.next() {
                let last = changed.next_back().unwrap_or(first);
                let mut clock = PhaseClock::default();
                clock.restart(cx);
                self.wave = Some(Compaction {
                    from: self.painted_cells.clone(),
                    first,
                    last,
                    clock,
                });
            }
        }
        let elapsed = self.wave.as_ref().map_or(0., |s| {
            let end = CELL_FADE + (s.last - s.first) as f32 * CELL_DELAY + 0.14;
            s.clock.time(end, w, cx)
        });
        if self
            .wave
            .as_ref()
            .is_some_and(|s| elapsed >= CELL_FADE + (s.last - s.first) as f32 * CELL_DELAY + 0.14)
        {
            self.wave = None;
        }
        self.painted_cells.clear();
        let mut grid = column().gap(px(2.));
        for r in 0..10 {
            let mut line = row().gap(px(2.));
            for c in 0..20 {
                let i = r * 20 + c;
                let (which, base) = cell(
                    i,
                    if self.wave.is_some() {
                        &target
                    } else {
                        &displayed
                    },
                );
                let (color, alpha) = self
                    .wave
                    .as_ref()
                    .map_or((base, 1.), |wave| wave.paint(i, base, elapsed));
                self.painted_cells.push((color, alpha));
                let highlight = if highlighted.is_none() || highlighted == which {
                    1.
                } else {
                    0.25
                };
                line = line.child(
                    div()
                        .flex_1()
                        .h(px(9.))
                        .rounded(radius)
                        .bg(color)
                        .opacity(alpha * highlight),
                );
            }
            grid = grid.child(line);
            if (r + 1) as f32 == (10. * self.threshold).round() {
                grid = grid.child(divider(cx))
            }
        }
        panel(&self.id, 480., cx)
            .child(
                row()
                    .justify_between()
                    .child(
                        column()
                            .child(
                                title(if used >= 1000 {
                                    format!("{:.0}K", used as f64 / 1000.)
                                } else {
                                    used.to_string()
                                })
                                .text_size(px(26.)),
                            )
                            .child(note(
                                format!(
                                    "{} {}K {}",
                                    tr(cx, "of", "总量"),
                                    limit / 1000,
                                    tr(cx, "tokens", "词元")
                                ),
                                cx,
                            ))
                            .child(note(
                                format!(
                                    "{:.0}% · {} {}",
                                    100. * used as f64 / limit as f64,
                                    limit.saturating_sub(used).saturating_sub(self.reserved),
                                    tr(cx, "free", "可用")
                                ),
                                cx,
                            )),
                    )
                    .child(badge(self.model.clone(), cx)),
            )
            .child(grid)
            .child({
                let label = if self.freed > 0 {
                    format!(
                        "{} · {} {}",
                        tr(cx, "Context compacted", "上下文已压缩"),
                        self.freed,
                        tr(cx, "tokens freed", "词元已释放")
                    )
                } else {
                    format!(
                        "{} {}  ·  {} {:.0}%",
                        tr(cx, "Tokens per cell", "每格词元"),
                        limit / 200,
                        tr(cx, "Compacts at", "压缩阈值"),
                        self.threshold * 100.
                    )
                };
                div()
                    .id("capacity-status")
                    .role(Role::Status)
                    .aria_label(label.clone())
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(p.secondary)
                    .child(swap_label("capacity-status-label", label, w, cx))
            })
            .child(
                column()
                    .gap_0()
                    .children(self.bands.iter().enumerate().map(|(i, b)| {
                        item(("category", i), b.label.clone(), cx, move |s, _, cx| {
                            s.selected = if s.selected == Some(i) { None } else { Some(i) };
                            cx.notify()
                        })
                        .on_hover(cx.listener(move |s, over, _, cx| {
                            if *over {
                                s.hovered = Some(i)
                            } else if s.hovered == Some(i) {
                                s.hovered = None
                            }
                            cx.notify()
                        }))
                        .child(div().size(px(9.)).rounded(radius).bg(b.color))
                        .child(div().flex_1().child(b.label.clone()))
                        .child(note(format!("{}", b.tokens), cx))
                    })),
            )
            .child(
                row()
                    .flex_wrap()
                    .child(btn(
                        "message",
                        tr(cx, "Send message", "发送消息"),
                        false,
                        cx,
                        |_, _, cx| cx.emit(CapacityAction::Message),
                    ))
                    .child(btn(
                        "file",
                        tr(cx, "Attach file", "附加文件"),
                        false,
                        cx,
                        |_, _, cx| cx.emit(CapacityAction::Attach),
                    ))
                    .child(btn(
                        "compact",
                        tr(cx, "Compact now", "立即压缩"),
                        false,
                        cx,
                        |_, _, cx| cx.emit(CapacityAction::Compact),
                    )),
            )
    }
}
