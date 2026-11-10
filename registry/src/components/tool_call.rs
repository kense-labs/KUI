//! Inspect a tool invocation, its arguments and its result.
use crate::primitives::{
    brand_mark::brand_mark,
    clipboard,
    family::{Paint, disabled_motion},
    glyph::{Glyph, icon, spinner},
    ops_surface::*,
};
use gpui::{prelude::*, *};
use web_time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    Running,
    Approval,
    Done,
    Failed,
    Denied,
}

pub struct Invocation {
    pub name: String,
    pub action: String,
    pub input: String,
    pub output: String,
    pub state: ToolState,
}

pub enum ToolAction {
    Approve(usize),
    Deny(usize),
    Retry(usize),
}

pub struct InvocationInspector {
    id: SharedString,
    pub calls: Vec<Invocation>,
    durations: Vec<SharedString>,
    group_open: bool,
    group_epoch: usize,
    compact: Option<bool>,
    open: Option<usize>,
    copied: Option<(usize, bool)>,
    visibility: fn() -> bool,
    clock: Instant,
}
impl EventEmitter<ToolAction> for InvocationInspector {}

impl InvocationInspector {
    pub fn new(id: impl Into<SharedString>, calls: Vec<Invocation>) -> Self {
        Self {
            id: id.into(),
            calls,
            durations: Vec::new(),
            group_open: true,
            group_epoch: 0,
            compact: None,
            open: None,
            copied: None,
            visibility: || true,
            clock: Instant::now(),
        }
    }

    /// Adds duration labels in call order.
    pub fn durations(mut self, durations: Vec<SharedString>) -> Self {
        self.durations = durations;
        self
    }

    /// Connects the visibility of an embedded preview to the running indicator.
    pub fn visibility(mut self, visible: fn() -> bool) -> Self {
        self.visibility = visible;
        self
    }

    fn request(&mut self, i: usize, action: ToolAction, cx: &mut Context<Self>) {
        let Some(call) = self.calls.get_mut(i) else {
            return;
        };
        match action {
            ToolAction::Approve(_) | ToolAction::Deny(_) if call.state != ToolState::Approval => {
                return;
            }
            ToolAction::Retry(_) if call.state != ToolState::Failed => return,
            _ => {}
        }
        call.state = if matches!(action, ToolAction::Deny(_)) {
            ToolState::Denied
        } else {
            ToolState::Running
        };
        cx.emit(action);
        cx.notify();
    }

    fn copy_input(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(call) = self.calls.get(i) else {
            return;
        };
        let request = clipboard::write(&call.input, cx);
        cx.spawn(async move |weak, cx| {
            let ok = request.await;
            let _ = weak.update(cx, |s, cx| {
                s.copied = Some((i, ok));
                cx.notify();
            });
        })
        .detach();
    }
}

fn tool_mark(name: &str, side: f32, cx: &App) -> AnyElement {
    brand_mark(name, side, cx)
        .unwrap_or_else(|| icon(Glyph::Settings, side, Paint::get(cx).secondary).into_any_element())
}

impl Render for InvocationInspector {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let compact = self.compact.unwrap_or(w.viewport_size().width < px(470.));
        let weak = cx.weak_entity();
        let running = self
            .calls
            .iter()
            .filter(|call| call.state == ToolState::Running)
            .count();
        let failed = self
            .calls
            .iter()
            .filter(|call| call.state == ToolState::Failed)
            .count();
        let count = self.calls.len();
        let group_name = if count == 1 {
            if running > 0 {
                tr(cx, "Using 1 tool", "正在使用 1 个工具").to_owned()
            } else {
                tr(cx, "Used 1 tool", "使用了 1 个工具").to_owned()
            }
        } else if running > 0 {
            format!(
                "{} {count} {}",
                tr(cx, "Using", "正在使用"),
                tr(cx, "tools", "个工具")
            )
        } else {
            format!(
                "{} {count} {}",
                tr(cx, "Used", "使用了"),
                tr(cx, "tools", "个工具")
            )
        };
        let motion = !disabled_motion(cx) && (self.visibility)() && w.is_visible();
        let turn = if running > 0 && motion {
            w.request_animation_frame();
            self.clock.elapsed().as_secs_f32() * 1.15
        } else {
            0.
        };
        let mut marks = row().gap(px(2.)).flex_none();
        let mut seen = Vec::new();
        for call in &self.calls {
            if seen.contains(&call.name.as_str()) {
                continue;
            }
            seen.push(&call.name);
            marks = marks.child(
                div()
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .child(tool_mark(&call.name, 12., cx)),
            );
            if seen.len() == 3 {
                break;
            }
        }

        let head = item(
            "tools",
            tr(cx, "Tool calls", "工具调用"),
            cx,
            |s, _, cx| {
                s.group_open = !s.group_open;
                if s.group_open {
                    s.group_epoch = s.group_epoch.wrapping_add(1);
                }
                cx.notify();
            },
        )
        .w_full()
        .h(px(45.))
        .px(px(14.))
        .py_0()
        .gap_2()
        .border_1()
        .border_color(transparent_black())
        .aria_expanded(self.group_open)
        .child(marks)
        .child(
            swap_label("tools-used", group_name, w, cx)
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(13.)),
        )
        .when(failed > 0, |v| {
            v.child(
                note(format!("{failed} {}", tr(cx, "failed", "项失败")), cx).text_color(p.danger),
            )
        })
        .child(icon(
            if self.group_open {
                Glyph::Up
            } else {
                Glyph::Down
            },
            13.,
            p.secondary,
        ));
        let mut root = panel(&self.id, 560., cx).p_0().gap_0().child(
            div()
                .on_children_prepainted(move |bounds, _, cx| {
                    if let Some(head) = bounds.first() {
                        let narrow = head.size.width + px(2.) < px(440.);
                        if narrow != compact {
                            let _ = weak.update(cx, |s, cx| {
                                s.compact = Some(narrow);
                                cx.notify();
                            });
                        }
                    }
                })
                .child(head),
        );
        let group = Fold::get("tool-rows-fold", self.group_open, 125., 0.26, w, cx);
        if !group.mounted() {
            return root;
        }
        let mut rows = column().gap_0().border_t_1().border_color(p.line).p_1();
        for (i, c) in self.calls.iter().enumerate() {
            let (state, glyph, ink) = match c.state {
                ToolState::Running => (tr(cx, "Running", "运行中"), Glyph::Clock, p.secondary),
                ToolState::Approval => (
                    tr(cx, "Approval required", "等待批准"),
                    Glyph::Lock,
                    p.secondary,
                ),
                ToolState::Done => (tr(cx, "Done", "完成"), Glyph::Check, p.success),
                ToolState::Failed => (tr(cx, "Failed", "失败"), Glyph::Alert, p.danger),
                ToolState::Denied => (tr(cx, "Denied", "已拒绝"), Glyph::Close, p.secondary),
            };
            let open = self.open == Some(i);
            let duration = self.durations.get(i).map_or("", SharedString::as_ref);
            let meta = if c.state == ToolState::Done && !compact && !c.output.is_empty() {
                if duration.is_empty() {
                    c.output.clone()
                } else {
                    format!("{} · {duration}", c.output)
                }
            } else if !duration.is_empty() && c.state == ToolState::Done {
                duration.to_owned()
            } else {
                String::new()
            };
            let status = if c.state == ToolState::Running {
                spinner(turn, 14., ink).into_any_element()
            } else {
                icon(glyph, 14., ink).into_any_element()
            };
            let mut action_label = swap_label(("call-action", i), c.action.clone(), w, cx)
                .debug_selector(move || format!("tool-title-{i}"))
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(13.));
            if c.state == ToolState::Running {
                action_label =
                    action_label.text_color(p.ink.blend(p.secondary.opacity(if motion {
                        0.26 + (turn * std::f32::consts::TAU).sin() * 0.18
                    } else {
                        0.32
                    })));
            }
            let toggle = item(("call", i), c.action.clone(), cx, move |s, _, cx| {
                s.open = if s.open == Some(i) { None } else { Some(i) };
                s.copied = None;
                cx.notify();
            })
            .w_full()
            .h(px(40.))
            .py_0()
            .px(px(10.))
            .gap(px(9.))
            .aria_label(format!("{}: {} · {state}", c.name, c.action))
            .aria_expanded(open)
            .child(
                div()
                    .size(px(16.))
                    .flex_none()
                    .debug_selector(move || format!("tool-name-{i}"))
                    .child(tool_mark(&c.name, 16., cx)),
            )
            .child(action_label)
            .when(!meta.is_empty(), |v| {
                v.child(note(meta, cx).max_w(px(106.)).truncate())
            })
            .child(status)
            .child(icon(
                if open { Glyph::Up } else { Glyph::Down },
                12.,
                p.secondary,
            ));
            let mut section = column().gap_0().child(toggle);
            if matches!(
                c.state,
                ToolState::Approval | ToolState::Failed | ToolState::Denied
            ) {
                let reason = if c.output.is_empty() {
                    state.to_owned()
                } else {
                    c.output.clone()
                };
                let mut actions = row().gap(px(6.));
                if c.state == ToolState::Approval {
                    actions = actions
                        .child(
                            btn(
                                ("deny", i),
                                tr(cx, "Deny", "拒绝"),
                                false,
                                cx,
                                move |s, _, cx| s.request(i, ToolAction::Deny(i), cx),
                            )
                            .aria_label(format!("{}: {}", tr(cx, "Deny", "拒绝"), c.action))
                            .border_1()
                            .border_color(p.line),
                        )
                        .child(
                            btn(
                                ("allow", i),
                                tr(cx, "Allow", "批准"),
                                true,
                                cx,
                                move |s, _, cx| s.request(i, ToolAction::Approve(i), cx),
                            )
                            .aria_label(format!(
                                "{}: {}",
                                tr(cx, "Allow", "批准"),
                                c.action
                            )),
                        );
                } else if c.state == ToolState::Failed {
                    actions = actions.child(
                        btn(
                            ("retry", i),
                            tr(cx, "Retry", "重试"),
                            false,
                            cx,
                            move |s, _, cx| s.request(i, ToolAction::Retry(i), cx),
                        )
                        .aria_label(format!("{}: {}", tr(cx, "Retry", "重试"), c.action))
                        .border_1()
                        .border_color(p.line),
                    );
                }
                section = section.child(
                    row()
                        .when(compact, |v| v.flex_col().items_start())
                        .px(px(10.))
                        .pl(px(35.))
                        .pb_2()
                        .child(
                            note(reason, cx)
                                .flex_1()
                                .debug_selector(move || format!("tool-reason-{i}")),
                        )
                        .child(actions),
                );
            }
            let detail = Fold::get(("tool-detail", i), open, 125., 0.26, w, cx);
            if detail.mounted() {
                let copy_label = match self.copied {
                    Some((selected, true)) if selected == i => tr(cx, "Copied", "已复制"),
                    Some((selected, false)) if selected == i => tr(cx, "Copy failed", "复制失败"),
                    _ => tr(cx, "Copy input", "复制输入"),
                };
                section = section.child(
                    detail.wrap(
                        0.,
                        column()
                            .pl(px(if compact { 10. } else { 35. }))
                            .pr(px(10.))
                            .pb_3()
                            .child(
                                row()
                                    .justify_between()
                                    .child(title(tr(cx, "Input", "输入")))
                                    .child(btn(
                                        ("copy-tool-input", i),
                                        copy_label,
                                        false,
                                        cx,
                                        move |s, _, cx| s.copy_input(i, cx),
                                    )),
                            )
                            .child(
                                div()
                                    .id(("tool-input-code", i))
                                    .px_2()
                                    .py_2()
                                    .rounded(crate::foundation::surface_radius(cx))
                                    .bg(p.stage)
                                    .overflow_x_scroll()
                                    .whitespace_nowrap()
                                    .child(note(c.input.clone(), cx)),
                            )
                            .child(title(tr(cx, "Output", "输出")))
                            .child(note(
                                if c.output.is_empty() {
                                    "—".to_owned()
                                } else {
                                    c.output.clone()
                                },
                                cx,
                            )),
                    ),
                );
            }
            rows = rows.child(arrive(
                section,
                (
                    "tool-row",
                    self.group_epoch.wrapping_mul(131).wrapping_add(i),
                ),
                (i.min(12) * 20) as u64,
                6.,
                cx,
            ));
        }
        root = root.child(group.wrap(0., rows));
        root
    }
}
