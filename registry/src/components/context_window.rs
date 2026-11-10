//! Inspect how different parts of a conversation consume its context budget.
use crate::primitives::brand_mark::brand_mark;
use crate::primitives::family::{Paint, PhaseClock, Slide, disabled_motion, standard_ease};
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
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
    grid_width: Rc<Cell<Pixels>>,
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
            grid_width: Default::default(),
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
        let cell_radius = px(2.);
        let cell = |index: usize, boundaries: &[u64]| {
            let pos = index as f64 / CELL_COUNT as f64 * limit as f64;
            let mut end = 0u64;
            let which = boundaries.iter().position(|tokens| {
                end = end.saturating_add(*tokens);
                pos < end as f64
            });
            let color = which.map(|i| self.bands[i].color).unwrap_or_else(|| {
                if pos >= limit.saturating_sub(self.reserved) as f64 {
                    p.panel
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
        let grid_width = self.grid_width.clone();
        let measured = grid_width.get();
        let fallback = (w.viewport_size().width - px(64.)).clamp(px(160.), px(488.));
        let width = if measured > px(0.) {
            measured
        } else {
            fallback
        };
        let side = ((width - px(38.)) / 20.).max(px(1.));
        let threshold_row = (10. * self.threshold).round() as usize;
        let threshold_visible = (1..=10).contains(&threshold_row);
        let threshold_gap = px(if threshold_visible { 6. } else { 0. });
        let mut cells = Vec::with_capacity(CELL_COUNT);
        for i in 0..CELL_COUNT {
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
            let reserved = which.is_none()
                && i as f64 / CELL_COUNT as f64 * limit as f64
                    >= limit.saturating_sub(self.reserved) as f64;
            cells.push((
                color.opacity(alpha * highlight),
                reserved,
                alpha * highlight,
            ));
        }
        let grid = div()
            .id("capacity-grid")
            .debug_selector(|| "capacity-grid".into())
            .w_full()
            .h(side * 10. + px(18.) + threshold_gap)
            .child(
                canvas(
                    move |bounds, w, _| {
                        if (grid_width.replace(bounds.size.width) - bounds.size.width).abs()
                            > px(0.5)
                        {
                            w.request_animation_frame();
                        }
                    },
                    move |bounds, _, w, _| {
                        let stride = side + px(2.);
                        for (i, (fill_color, reserved, alpha)) in cells.into_iter().enumerate() {
                            let row = i / 20;
                            let col = i % 20;
                            let offset = if threshold_visible && row >= threshold_row {
                                threshold_gap
                            } else {
                                px(0.)
                            };
                            let origin = bounds.origin
                                + point(stride * col as f32, stride * row as f32 + offset);
                            w.paint_quad(quad(
                                Bounds::new(origin, size(side, side)),
                                Corners::all(cell_radius),
                                fill_color,
                                Edges::all(if reserved { px(1.) } else { px(0.) }),
                                p.line.opacity(alpha),
                                BorderStyle::Solid,
                            ));
                        }
                        if threshold_visible {
                            let y = bounds.top() + stride * threshold_row as f32 + px(1.);
                            w.paint_quad(fill(
                                Bounds::new(
                                    point(bounds.left(), y),
                                    size(bounds.size.width, px(1.)),
                                ),
                                p.line,
                            ));
                        }
                    },
                )
                .size_full(),
            );
        let readout = |tokens: u64, cx: &App| {
            let amount = if tokens >= 1_000 {
                let scaled = format!("{:.1}", tokens as f64 / 1_000.);
                format!("{}K", scaled.trim_end_matches(".0"))
            } else {
                tokens.to_string()
            };
            row()
                .gap(px(12.))
                .child(note(amount, cx))
                .child(div().w(px(38.)).text_right().child(note(
                    format!("{:.1}%", 100. * tokens as f64 / limit as f64),
                    cx,
                )))
        };
        let reserve = self.reserved.min(limit);
        let free = limit.saturating_sub(used).saturating_sub(reserve);
        panel(&self.id, 522., cx)
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
                    .child(
                        row()
                            .gap_1()
                            .when_some(brand_mark(&self.model, 14., cx), |row, mark| {
                                row.child(mark)
                            })
                            .child(badge(self.model.clone(), cx)),
                    ),
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
                        .child(div().size(px(9.)).rounded(cell_radius).bg(b.color))
                        .child(div().flex_1().child(b.label.clone()))
                        .child(readout(b.tokens, cx))
                    }))
                    .children(
                        [
                            (tr(cx, "Free", "可用"), free, false),
                            (tr(cx, "Reserved for output", "预留输出"), reserve, true),
                        ]
                        .map(|(label, tokens, outline)| {
                            row()
                                .min_h(px(30.))
                                .gap(px(8.))
                                .child(
                                    div()
                                        .size(px(9.))
                                        .rounded(cell_radius)
                                        .bg(p.stage)
                                        .when(outline, |cell| cell.border_1().border_color(p.line)),
                                )
                                .child(div().flex_1().child(label))
                                .child(readout(tokens, cx))
                        }),
                    ),
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
