//! Inspect a tool invocation, its arguments and its result.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    family::{Paint, measure},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
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
    group_open: bool,
    compact: Option<bool>,
    open: Option<usize>,
}
impl EventEmitter<ToolAction> for InvocationInspector {}
impl InvocationInspector {
    pub fn new(id: impl Into<SharedString>, calls: Vec<Invocation>) -> Self {
        Self {
            id: id.into(),
            calls,
            group_open: true,
            compact: None,
            open: None,
        }
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
        cx.notify()
    }
}
impl Render for InvocationInspector {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let compact = self.compact.unwrap_or(w.viewport_size().width < px(440.));
        let weak = cx.weak_entity();
        let tag_width = self
            .calls
            .iter()
            .map(|c| measure(&c.name.clone().into(), 11., w).ceil() + 14.)
            .fold(44_f32, f32::max)
            .min(84.);
        let content_left = 12. + tag_width + 10.;
        let mut root = panel(&self.id, 522., cx).p(px(4.)).gap_0().child(
            div()
                .on_children_prepainted(move |bounds, _, cx| {
                    if let Some(head) = bounds.first() {
                        let narrow = head.size.width + px(10.) < px(440.);
                        if narrow != compact {
                            let _ = weak.update(cx, |s, cx| {
                                s.compact = Some(narrow);
                                cx.notify()
                            });
                        }
                    }
                })
                .child(
                    item(
                        "tools",
                        tr(cx, "Tool calls", "工具调用"),
                        cx,
                        |v, _, cx| {
                            v.group_open = !v.group_open;
                            cx.notify()
                        },
                    )
                    .w_full()
                    .border_1()
                    .border_color(transparent_black())
                    .aria_expanded(self.group_open)
                    .h(px(40.))
                    .px(px(11.))
                    .py_0()
                    .gap(px(10.))
                    .child(div().w(px(tag_width)).flex_none().child(icon(
                        Glyph::Settings,
                        16.,
                        p.secondary,
                    )))
                    .child(div().flex_1().min_w_0().child(format!(
                        "{} · {}",
                        tr(cx, "Tool calls", "工具调用"),
                        self.calls.len()
                    )))
                    .child(icon(
                        if self.group_open {
                            Glyph::Up
                        } else {
                            Glyph::Down
                        },
                        14.,
                        p.secondary,
                    )),
                ),
        );
        let group = Fold::get("tool-rows-fold", self.group_open, 100., 0.24, w, cx);
        if group.mounted() {
            let mut rows = column()
                .gap_0()
                .border_t_1()
                .border_color(p.line)
                .py(px(4.));
            for (i, c) in self.calls.iter().enumerate() {
                let state = match c.state {
                    ToolState::Running => tr(cx, "Running", "运行中"),
                    ToolState::Approval => tr(cx, "Approval required", "等待批准"),
                    ToolState::Done => tr(cx, "Complete", "完成"),
                    ToolState::Failed => tr(cx, "Failed", "失败"),
                    ToolState::Denied => tr(cx, "Denied", "已拒绝"),
                };
                let open = self.open == Some(i);
                let has_action = matches!(c.state, ToolState::Approval | ToolState::Failed);
                let mut section = column()
                    .gap_0()
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(if has_action {
                        p.line
                    } else {
                        transparent_black()
                    });
                let status = swap_label(("tool-status", i), state, w, cx)
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(p.secondary);
                let action_title = div()
                    .id(("tool-title", i))
                    .debug_selector(move || format!("tool-title-{i}"))
                    .min_w_0()
                    .truncate()
                    .child(c.action.clone());
                let content = column()
                    .id(("tool-summary", i))
                    .debug_selector(move || format!("tool-summary-{i}"))
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(action_title);
                let (content, trailing_status) = if compact || has_action {
                    (content.child(status), None)
                } else {
                    (content, Some(status))
                };
                let tag = div()
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.stage)
                    .text_size(px(11.))
                    .px(px(6.))
                    .id(("tool-name", i))
                    .debug_selector(move || format!("tool-name-{i}"))
                    .w(px(tag_width))
                    .h(px(24.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().min_w_0().truncate().child(c.name.clone()));
                let mut toggle = item(("call", i), c.action.clone(), cx, move |s, _, cx| {
                    s.open = if s.open == Some(i) { None } else { Some(i) };
                    cx.notify()
                })
                .aria_label(format!("{} · {} · {}", c.name, c.action, state))
                .aria_expanded(open)
                .flex_1()
                .min_w_0()
                .min_h(px(38.))
                .items_center()
                .border_1()
                .border_color(transparent_black())
                .px(px(10.))
                .gap(px(10.))
                .child(tag)
                .child(content);
                if let Some(status) = trailing_status {
                    toggle = toggle.child(status);
                }
                let mut heading = row().items_center().gap_0().w_full().child(toggle);
                let mut actions = row().gap(px(6.)).flex_none();
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
                            .px(px(12.))
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
                            .px(px(12.)),
                        );
                } else if c.state == ToolState::Failed {
                    actions = actions.child(btn(
                        ("retry", i),
                        tr(cx, "Retry", "重试"),
                        false,
                        cx,
                        move |s, _, cx| s.request(i, ToolAction::Retry(i), cx),
                    ));
                }
                if has_action && compact {
                    section = section
                        .child(heading)
                        .child(actions.ml(px(content_left - 1.)).mb(px(6.)));
                } else {
                    if has_action {
                        heading = heading.child(actions.mr(px(11.)));
                    }
                    section = section.child(heading);
                }
                let detail = Fold::get(("tool-detail", i), open, 112., 0.24, w, cx);
                if detail.mounted() {
                    section = section.child(
                        detail.wrap(
                            0.,
                            column()
                                .pl(px(if compact { 11. } else { content_left - 1. }))
                                .pr(px(11.))
                                .pb(px(8.))
                                .child(title(tr(cx, "Input", "输入")))
                                .child(note(c.input.clone(), cx))
                                .child(title(tr(cx, "Output", "输出")))
                                .child(note(c.output.clone(), cx)),
                        ),
                    );
                }
                rows = rows.child(section);
            }
            root = root.child(group.wrap(0., rows));
        }
        root
    }
}
