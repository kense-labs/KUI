//! A host-driven workflow preview and editor. No actions are executed internally.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    presentation::{column, hint, row, tr},
};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowKind {
    Trigger,
    Ai,
    Branch,
    Action,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowStatus {
    Idle,
    Running,
    Done,
    Failed,
    Skipped,
}
#[derive(Clone, Debug)]
pub struct WorkflowStep {
    pub id: SharedString,
    pub title: SharedString,
    pub app: SharedString,
    pub detail: SharedString,
    pub kind: WorkflowKind,
    /// Empty for top-level rows; otherwise identifies the containing branch path.
    pub path: Option<SharedString>,
    pub status: WorkflowStatus,
    pub output: Option<SharedString>,
    pub error: Option<SharedString>,
    pub duration_ms: Option<u64>,
}
impl WorkflowStep {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        app: impl Into<SharedString>,
        kind: WorkflowKind,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            app: app.into(),
            detail: "".into(),
            kind,
            path: None,
            status: WorkflowStatus::Idle,
            output: None,
            error: None,
            duration_ms: None,
        }
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
    pub fn with_path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = Some(path.into());
        self
    }
}
#[derive(Clone, Debug)]
pub struct WorkflowPath {
    pub id: SharedString,
    pub label: SharedString,
}
impl WorkflowPath {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct WorkflowResult {
    pub output: SharedString,
    pub duration_ms: Option<u64>,
    /// Required for a branch: must reference one of the supplied paths.
    pub path: Option<SharedString>,
}
impl WorkflowResult {
    pub fn new(output: impl Into<SharedString>) -> Self {
        Self {
            output: output.into(),
            duration_ms: None,
            path: None,
        }
    }
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
    pub fn with_path(mut self, id: impl Into<SharedString>) -> Self {
        self.path = Some(id.into());
        self
    }
}
#[derive(Clone, Debug)]
pub enum WorkflowEvent {
    RunStep {
        request_id: u64,
        step: Box<WorkflowStep>,
        previous: Vec<(SharedString, SharedString)>,
        attempt: u32,
    },
    Cancel {
        request_id: u64,
    },
    Changed {
        steps: Vec<WorkflowStep>,
    },
    Completed {
        success: bool,
    },
}
pub struct AiWorkflow {
    title: SharedString,
    description: SharedString,
    steps: Vec<WorkflowStep>,
    paths: Vec<WorkflowPath>,
    catalog: Vec<WorkflowStep>,
    selected_path: Option<SharedString>,
    running: bool,
    pending: Option<(u64, usize)>,
    serial: u64,
    attempt: u32,
    menu_open: bool,
    simulated: bool,
    visible: bool,
    rail: Slide,
    path_focus: Option<FocusHandle>,
}
impl EventEmitter<WorkflowEvent> for AiWorkflow {}
impl AiWorkflow {
    pub fn new(
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        steps: Vec<WorkflowStep>,
        paths: Vec<WorkflowPath>,
    ) -> Self {
        let selected_path = paths.first().map(|p| p.id.clone());
        Self {
            title: title.into(),
            description: description.into(),
            steps,
            paths,
            catalog: Vec::new(),
            selected_path,
            running: false,
            pending: None,
            serial: 0,
            attempt: 0,
            menu_open: false,
            simulated: false,
            visible: true,
            rail: Slide::new(0.),
            path_focus: None,
        }
    }
    pub fn with_catalog(mut self, catalog: Vec<WorkflowStep>) -> Self {
        self.catalog = catalog;
        self
    }
    pub fn with_simulated(mut self, simulated: bool) -> Self {
        self.simulated = simulated;
        self
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_steps(&mut self, steps: Vec<WorkflowStep>, cx: &mut Context<Self>) {
        self.stop(cx);
        self.steps = steps;
        cx.notify();
    }
    fn active(&self, step: &WorkflowStep) -> bool {
        step.path.is_none() || step.path == self.selected_path
    }
    pub fn start(&mut self, cx: &mut Context<Self>) {
        if self.running || self.steps.is_empty() {
            return;
        }
        self.menu_open = false;
        for step in &mut self.steps {
            step.status = WorkflowStatus::Idle;
            step.output = None;
            step.error = None;
            step.duration_ms = None;
        }
        self.running = true;
        self.attempt = 0;
        self.advance(cx);
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.running = false;
        if let Some((id, index)) = self.pending.take() {
            if let Some(step) = self.steps.get_mut(index) {
                step.status = WorkflowStatus::Idle;
            }
            cx.emit(WorkflowEvent::Cancel { request_id: id });
        }
        cx.notify();
    }
    fn advance(&mut self, cx: &mut Context<Self>) {
        if !self.running || self.pending.is_some() {
            return;
        }
        let next = self
            .steps
            .iter()
            .position(|s| self.active(s) && s.status == WorkflowStatus::Idle);
        let Some(index) = next else {
            self.running = false;
            for step in &mut self.steps {
                if step.status == WorkflowStatus::Idle {
                    step.status = WorkflowStatus::Skipped;
                }
            }
            cx.emit(WorkflowEvent::Completed { success: true });
            cx.notify();
            return;
        };
        self.serial = self.serial.wrapping_add(1);
        self.pending = Some((self.serial, index));
        self.rail.reset(0.);
        self.rail.duration(1., 0.2, cx);
        self.steps[index].status = WorkflowStatus::Running;
        let previous = self
            .steps
            .iter()
            .filter_map(|s| s.output.clone().map(|v| (s.id.clone(), v)))
            .collect();
        cx.emit(WorkflowEvent::RunStep {
            request_id: self.serial,
            step: Box::new(self.steps[index].clone()),
            previous,
            attempt: self.attempt + 1,
        });
        cx.notify();
    }
    pub fn resolve(
        &mut self,
        id: u64,
        result: Result<WorkflowResult, SharedString>,
        cx: &mut Context<Self>,
    ) {
        let Some((_, index)) = self.pending.filter(|(pending, _)| *pending == id) else {
            return;
        };
        self.pending = None;
        match result {
            Ok(result) => {
                if self.steps[index].kind == WorkflowKind::Branch {
                    let valid = result
                        .path
                        .as_ref()
                        .is_some_and(|path| self.paths.iter().any(|p| &p.id == path));
                    if !valid {
                        self.fail(
                            index,
                            tr(cx, "Unknown workflow path", "工作流分支无效").into(),
                            cx,
                        );
                        return;
                    }
                    self.selected_path = result.path;
                }
                let step = &mut self.steps[index];
                step.status = WorkflowStatus::Done;
                step.output = Some(result.output);
                step.duration_ms = result.duration_ms;
                step.error = None;
                self.attempt = 0;
                self.advance(cx);
            }
            Err(error) => self.fail(index, error, cx),
        }
        cx.notify();
    }
    fn fail(&mut self, index: usize, error: SharedString, cx: &mut Context<Self>) {
        self.running = false;
        self.steps[index].status = WorkflowStatus::Failed;
        self.steps[index].error = Some(error);
        cx.emit(WorkflowEvent::Completed { success: false });
        cx.notify();
    }
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        if let Some(step) = self
            .steps
            .iter_mut()
            .find(|s| s.status == WorkflowStatus::Failed)
        {
            step.status = WorkflowStatus::Idle;
            step.error = None;
            self.attempt += 1;
            self.running = true;
            self.advance(cx);
        }
    }
    fn remove(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.running || index >= self.steps.len() {
            return;
        }
        let branch = self.steps[index].kind == WorkflowKind::Branch;
        self.steps.remove(index);
        if branch {
            self.steps.retain(|step| step.path.is_none());
        }
        cx.emit(WorkflowEvent::Changed {
            steps: self.steps.clone(),
        });
        cx.notify();
    }
    fn add(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        let Some(mut step) = self.catalog.get(index).cloned() else {
            return;
        };
        self.serial = self.serial.wrapping_add(1);
        step.id = format!("{}-{}", step.id, self.serial).into();
        if self.steps.iter().any(|s| s.kind == WorkflowKind::Branch) {
            step.path = self.selected_path.clone();
        }
        self.steps.push(step);
        self.menu_open = false;
        cx.emit(WorkflowEvent::Changed {
            steps: self.steps.clone(),
        });
        cx.notify();
    }
}
fn kind(kind: WorkflowKind, cx: &App) -> &'static str {
    match kind {
        WorkflowKind::Trigger => tr(cx, "Trigger", "触发器"),
        WorkflowKind::Ai => tr(cx, "AI step", "AI 步骤"),
        WorkflowKind::Branch => tr(cx, "Branch", "分支"),
        WorkflowKind::Action => tr(cx, "Action", "操作"),
    }
}
fn state(status: WorkflowStatus, cx: &App) -> &'static str {
    match status {
        WorkflowStatus::Idle => tr(cx, "Ready", "待执行"),
        WorkflowStatus::Running => tr(cx, "Running", "运行中"),
        WorkflowStatus::Done => tr(cx, "Done", "完成"),
        WorkflowStatus::Failed => tr(cx, "Failed", "失败"),
        WorkflowStatus::Skipped => tr(cx, "Skipped", "已跳过"),
    }
}
impl Render for AiWorkflow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let progress = if self.visible && w.is_visible() {
            self.rail.get(w, cx)
        } else {
            self.rail.value(cx)
        }
        .clamp(0., 1.);
        let focus = self
            .path_focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let mut list = column()
            .id("workflow-steps")
            .w_full()
            .gap(px(16.))
            .py(px(12.))
            .px(px(8.))
            .role(Role::List);
        for (index, step) in self.steps.iter().enumerate() {
            if !self.active(step) {
                continue;
            }
            let accent = match step.status {
                WorkflowStatus::Done => p.success,
                WorkflowStatus::Failed => p.danger,
                WorkflowStatus::Running => p.bright,
                _ => p.secondary,
            };
            let mark = match step.status {
                WorkflowStatus::Done => "✓",
                WorkflowStatus::Failed => "!",
                WorkflowStatus::Running => "•",
                WorkflowStatus::Skipped => "–",
                WorkflowStatus::Idle => "·",
            };
            let step_label = format!("{} · {} · {}", step.title, step.app, state(step.status, cx));
            let has_next = self.steps.iter().skip(index + 1).any(|s| self.active(s));
            let mut entry = column()
                .id(SharedString::from(format!("workflow-entry-{}", step.id)))
                .w_full()
                .relative()
                .role(Role::ListItem)
                .aria_label(step_label)
                .gap_1();
            if has_next {
                let filled = match step.status {
                    WorkflowStatus::Done => 1.,
                    WorkflowStatus::Running => progress,
                    _ => 0.,
                };
                entry = entry.child(
                    div()
                        .absolute()
                        .left(px(15.5))
                        .top(px(16.))
                        .bottom(px(-4.))
                        .w(px(1.))
                        .bg(p.line)
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .h(relative(filled))
                                .bg(p.bright.opacity(0.8)),
                        )
                        .when(
                            step.status == WorkflowStatus::Running && progress < 0.995,
                            |rail| {
                                rail.child(
                                    div()
                                        .absolute()
                                        .left(px(-2.))
                                        .top(relative(progress))
                                        .size(px(5.))
                                        .rounded(px(2.5))
                                        .bg(p.bright),
                                )
                            },
                        ),
                );
            }
            let status = state(step.status, cx);
            let content = row()
                .w_full()
                .h(px(32.))
                .items_center()
                .gap(px(10.))
                .pl(px(8.))
                .pr(px(4.))
                .rounded(crate::foundation::surface_radius(cx))
                .child(
                    div()
                        .relative()
                        .w(px(16.))
                        .bg(p.panel)
                        .text_center()
                        .text_color(accent)
                        .child(mark),
                )
                .child(div().min_w(px(0.)).flex_1().child(step.title.clone()))
                .child(hint(step.app.clone(), p))
                .child(
                    div()
                        .w(px(72.))
                        .text_right()
                        .text_sm()
                        .text_color(accent)
                        .child(status),
                )
                .when(!self.running, |v| {
                    v.child(
                        face(
                            ("workflow-remove", index),
                            format!("{} {}", tr(cx, "Remove", "移除"), step.title),
                            30.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(8.))
                        .child("×")
                        .on_click(cx.listener(move |s, _, _, cx| s.remove(index, cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, _, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.remove(index, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    )
                });
            entry = entry.child(content);
            if !step.detail.is_empty() {
                entry = entry.child(div().pl(px(34.)).child(hint(step.detail.clone(), p)));
            }
            if let Some(output) = step.output.clone() {
                entry = entry.child(
                    div()
                        .pl(px(34.))
                        .text_sm()
                        .text_color(p.secondary)
                        .child(output),
                );
            }
            if let Some(error) = step.error.clone() {
                entry = entry.child(
                    row()
                        .pl(px(34.))
                        .gap(px(12.))
                        .items_center()
                        .child(div().text_sm().text_color(p.danger).child(error))
                        .child(
                            face(
                                "workflow-retry",
                                tr(cx, "Retry", "重试"),
                                24.,
                                8.,
                                Tone::Surface,
                                cx,
                            )
                            .px(px(8.))
                            .text_size(px(12.))
                            .child(tr(cx, "Retry", "重试"))
                            .on_click(cx.listener(|s, _, _, cx| s.retry(cx)))
                            .on_key_down(cx.listener(
                                |s, e: &KeyDownEvent, _, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        && !e.is_held
                                    {
                                        s.retry(cx);
                                        cx.stop_propagation();
                                    }
                                },
                            )),
                        ),
                );
            }
            list = list.child(entry);
            if step.kind == WorkflowKind::Branch && !self.paths.is_empty() {
                let mut tabs = row().relative().pl(px(34.)).gap(px(2.)).flex_wrap().child(
                    div()
                        .absolute()
                        .left(px(15.5))
                        .top(px(-4.))
                        .bottom(px(-4.))
                        .w(px(1.))
                        .bg(p.line),
                );
                for (path_index, path) in self.paths.iter().enumerate() {
                    let id = path.id.clone();
                    let key_id = id.clone();
                    let selected = self.selected_path.as_ref() == Some(&id);
                    tabs = tabs.child(
                        face(
                            ("workflow-path", path_index),
                            path.label.clone(),
                            24.,
                            6.,
                            if selected {
                                Tone::Primary
                            } else {
                                Tone::Surface
                            },
                            cx,
                        )
                        .px(px(8.))
                        .text_size(px(12.))
                        .role(Role::Tab)
                        .aria_selected(selected)
                        .tab_index(if selected { 0 } else { -1 })
                        .when(selected, |v| v.track_focus(&focus))
                        .child(path.label.clone())
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if !s.running {
                                s.selected_path = Some(id.clone());
                                cx.notify();
                            }
                        }))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                if s.running || e.is_held {
                                    return;
                                }
                                let count = s.paths.len();
                                if count == 0 {
                                    return;
                                }
                                let target = match e.keystroke.key.as_str() {
                                    "enter" | "space" => Some(path_index),
                                    "left" | "up" => Some((path_index + count - 1) % count),
                                    "right" | "down" => Some((path_index + 1) % count),
                                    "home" => Some(0),
                                    "end" => Some(count - 1),
                                    _ => None,
                                };
                                if let Some(i) = target {
                                    s.selected_path = s
                                        .paths
                                        .get(i)
                                        .map(|path| path.id.clone())
                                        .or(Some(key_id.clone()));
                                    if let Some(focus) = &s.path_focus {
                                        focus.focus(w, cx);
                                    }
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    );
                }
                list = list.child(tabs);
            }
        }
        let action_label = if self.running {
            tr(cx, "Stop", "停止")
        } else {
            tr(cx, "Test run", "测试运行")
        };
        let mut body = column()
            .w_full()
            .max_w(px(640.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .w_full()
                    .gap(px(12.))
                    .items_center()
                    .pt(px(12.))
                    .pr(px(12.))
                    .pb(px(12.))
                    .pl(px(16.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(
                        column()
                            .flex_1()
                            .min_w(px(0.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .child(self.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.secondary)
                                    .child(self.description.clone()),
                            ),
                    )
                    .child(
                        face("workflow-run", action_label, 28., 10., Tone::Primary, cx)
                            .min_w(px(96.))
                            .px(px(10.))
                            .text_size(px(13.))
                            .child(action_label)
                            .on_click(cx.listener(
                                |s, _, _, cx| if s.running { s.stop(cx) } else { s.start(cx) },
                            ))
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    if s.running {
                                        s.stop(cx)
                                    } else {
                                        s.start(cx)
                                    }
                                    cx.stop_propagation();
                                }
                            })),
                    ),
            )
            .child(list);
        if self.steps.is_empty() {
            body = body.child(hint(
                tr(cx, "Add a trigger to start building", "添加触发器开始创建"),
                p,
            ));
        }
        if !self.catalog.is_empty() {
            body = body.child(
                face(
                    "workflow-add",
                    tr(cx, "Add step", "添加步骤"),
                    28.,
                    8.,
                    Tone::Surface,
                    cx,
                )
                .pl(px(8.))
                .pr(px(10.))
                .text_size(px(13.))
                .child(tr(cx, "Add step", "添加步骤"))
                .on_click(cx.listener(|s, _, _, cx| {
                    if !s.running {
                        s.menu_open = !s.menu_open;
                        cx.notify();
                    }
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                        && !e.is_held
                        && !s.running
                    {
                        s.menu_open = !s.menu_open;
                        cx.notify();
                        cx.stop_propagation();
                    } else if e.keystroke.key == "escape" && s.menu_open {
                        s.menu_open = false;
                        cx.notify();
                        cx.stop_propagation();
                    }
                })),
            );
            if self.menu_open {
                let mut options = column()
                    .w(px(300.))
                    .gap(px(0.))
                    .p(px(4.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.tray)
                    .border_1()
                    .border_color(p.line);
                for (index, item) in self.catalog.iter().enumerate() {
                    options = options.child(
                        face(
                            ("workflow-option", index),
                            item.title.clone(),
                            32.,
                            10.,
                            Tone::Surface,
                            cx,
                        )
                        .px(px(8.))
                        .text_size(px(13.))
                        .child(format!("{} · {}", item.title, kind(item.kind, cx)))
                        .on_click(cx.listener(move |s, _, _, cx| s.add(index, cx)))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, _, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.add(index, cx);
                                    cx.stop_propagation();
                                } else if e.keystroke.key == "escape" {
                                    s.menu_open = false;
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    );
                }
                body = body.child(options);
            }
        }
        body.child(hint(
            if self.simulated {
                tr(
                    cx,
                    "Simulated run · nothing is sent",
                    "模拟运行 · 不会发送任何内容",
                )
            } else {
                tr(cx, "Actions are provided by the host", "操作由宿主提供")
            },
            p,
        ))
    }
}
