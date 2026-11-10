//! A host-driven workflow preview and editor. No actions are executed internally.
use crate::primitives::{
    control::activate,
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, PhaseClock, Tone, disabled_motion, ease, face},
    glyph::{Glyph, icon},
    presentation::{column, hint, row, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

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
    rail: PhaseClock,
    result: PhaseClock,
    latest_result: Option<usize>,
    result_height: Rc<Cell<Pixels>>,
    completed: bool,
    loading: bool,
    query: Option<(Entity<Editor>, Subscription)>,
    menu_index: usize,
    menu_focus: Option<FocusHandle>,
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
            rail: PhaseClock::default(),
            result: PhaseClock::default(),
            latest_result: None,
            result_height: Rc::default(),
            completed: false,
            loading: false,
            query: None,
            menu_index: 0,
            menu_focus: None,
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
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if loading != self.loading {
            if loading {
                self.stop(cx);
                self.menu_open = false;
            }
            self.loading = loading;
            cx.notify();
        }
    }
    pub fn set_steps(&mut self, steps: Vec<WorkflowStep>, cx: &mut Context<Self>) {
        self.stop(cx);
        self.steps = steps;
        self.completed = false;
        self.latest_result = None;
        self.selected_path = self.paths.first().map(|path| path.id.clone());
        self.menu_open = false;
        cx.notify();
    }
    fn active(&self, step: &WorkflowStep) -> bool {
        step.path.is_none() || step.path == self.selected_path
    }
    pub fn start(&mut self, cx: &mut Context<Self>) {
        if self.running || self.steps.is_empty() || self.loading {
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
        self.completed = false;
        self.latest_result = None;
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
            self.completed = true;
            cx.emit(WorkflowEvent::Completed { success: true });
            cx.notify();
            return;
        };
        self.serial = self.serial.wrapping_add(1);
        self.pending = Some((self.serial, index));
        self.rail.restart(cx);
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
                self.latest_result = Some(index);
                self.result_height.set(px(0.));
                self.result.restart(cx);
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
            if step.path.is_some() {
                self.selected_path = step.path.clone();
            }
            step.status = WorkflowStatus::Idle;
            step.error = None;
            self.attempt += 1;
            self.running = true;
            self.advance(cx);
        }
    }
    fn remove(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.running
            || self
                .steps
                .get(index)
                .is_none_or(|step| step.kind == WorkflowKind::Trigger)
        {
            return;
        }
        self.latest_result = None;
        self.completed = false;
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
        if (step.kind == WorkflowKind::Trigger) != self.steps.is_empty() {
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        step.id = format!("{}-{}", step.id, self.serial).into();
        if self.steps.iter().any(|s| s.kind == WorkflowKind::Branch) {
            step.path = self.selected_path.clone();
        }
        self.steps.push(step);
        self.completed = false;
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

fn app_mark(step: &WorkflowStep) -> (Glyph, Hsla) {
    let (mark, color) = match step.app.as_ref() {
        "Gmail" => (Glyph::Comment, 0xd25046),
        "Slack" => (Glyph::Comment, 0x9966b1),
        "Linear" => (Glyph::Slash, 0x7774cb),
        _ => match step.kind {
            WorkflowKind::Trigger => (Glyph::Comment, 0xb96043),
            WorkflowKind::Ai => (Glyph::Bulb, 0x8e69bd),
            WorkflowKind::Branch => (Glyph::Down, 0x318a80),
            WorkflowKind::Action => (Glyph::Arrow, 0x5d80b9),
        },
    };
    (mark, rgb(color).into())
}

fn elapsed(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1000.)
    }
}

impl AiWorkflow {
    fn choices(&self, cx: &App) -> Vec<usize> {
        let query = self
            .query
            .as_ref()
            .map(|(editor, _)| editor.read(cx).value())
            .unwrap_or("");
        let query = query.trim().to_lowercase();
        self.catalog
            .iter()
            .enumerate()
            .filter_map(|(index, step)| {
                let available = (step.kind == WorkflowKind::Trigger) == self.steps.is_empty();
                (available
                    && (query.is_empty()
                        || [&step.title, &step.app, &step.detail]
                            .iter()
                            .any(|part| part.to_lowercase().contains(&query))))
                .then_some(index)
            })
            .collect()
    }

    fn toggle_menu(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.running || self.loading {
            return;
        }
        if self.menu_open {
            self.close_menu(w, cx);
            return;
        }
        let (editor, _) = self.query.get_or_insert_with(|| {
            let editor = cx.new(|cx| {
                let label = tr(cx, "Search steps…", "搜索步骤…");
                let mut editor = Editor::new(label, InputMode::Combobox, w, cx);
                editor.set_placeholder(label);
                editor
            });
            let sub = cx.subscribe_in(&editor, w, |s: &mut Self, _, event: &EditEvent, w, cx| {
                if !s.menu_open {
                    return;
                }
                match event {
                    EditEvent::Changed => s.menu_index = 0,
                    EditEvent::Navigate(forward) => {
                        let count = s.choices(cx).len();
                        if count > 0 {
                            s.menu_index = if *forward {
                                (s.menu_index + 1) % count
                            } else {
                                (s.menu_index + count - 1) % count
                            };
                        }
                    }
                    EditEvent::Submitted => {
                        if let Some(&index) = s.choices(cx).get(s.menu_index) {
                            s.add(index, cx);
                            s.close_menu(w, cx);
                        }
                    }
                    EditEvent::Escape => s.close_menu(w, cx),
                    _ => {}
                }
                cx.notify();
            });
            (editor, sub)
        });
        editor.update(cx, |field, cx| {
            field.set_value("", w, cx);
            field.focus(w, cx);
        });
        self.menu_index = 0;
        self.menu_open = true;
        cx.notify();
    }

    fn close_menu(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.menu_open = false;
        if let Some(focus) = &self.menu_focus {
            focus.focus(w, cx);
        }
        cx.notify();
    }

    fn paths_view(&self, focus: &FocusHandle, cx: &Context<Self>) -> Div {
        let mut tabs = row()
            .pl(px(34.))
            .pr(px(8.))
            .pb(px(3.))
            .gap(px(4.))
            .flex_wrap();
        for (index, path) in self.paths.iter().enumerate() {
            let id = path.id.clone();
            let selected = self.selected_path.as_ref() == Some(&id);
            tabs = tabs.child(
                face(
                    ("workflow-path", index),
                    path.label.clone(),
                    23.,
                    5.,
                    if selected { Tone::Surface } else { Tone::Clear },
                    cx,
                )
                .px(px(7.))
                .text_size(px(11.))
                .role(Role::Tab)
                .aria_selected(selected)
                .when(selected, |button| button.bg(Paint::get(cx).hover))
                .when(self.running, |button| button.tab_index(-1).cursor_default())
                .tab_index(if selected { 0 } else { -1 })
                .when(selected, |node| node.track_focus(focus))
                .child(path.label.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    if !s.running {
                        s.selected_path = Some(id.clone());
                        cx.notify();
                    }
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if s.running || e.is_held {
                        return;
                    }
                    let count = s.paths.len();
                    let next = match e.keystroke.key.as_str() {
                        "enter" | "space" => index,
                        "left" | "up" => (index + count - 1) % count,
                        "right" | "down" => (index + 1) % count,
                        "home" => 0,
                        "end" => count - 1,
                        _ => return,
                    };
                    s.selected_path = Some(s.paths[next].id.clone());
                    if let Some(focus) = &s.path_focus {
                        focus.focus(w, cx);
                    }
                    cx.notify();
                    cx.stop_propagation();
                })),
            );
        }
        tabs
    }

    fn catalog_view(&self, empty: bool, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let indices = if empty {
            self.catalog
                .iter()
                .enumerate()
                .filter_map(|(i, step)| (step.kind == WorkflowKind::Trigger).then_some(i))
                .collect()
        } else {
            self.choices(cx)
        };
        column()
            .w_full()
            .gap(px(3.))
            .p(px(6.))
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .when(!empty, |list| {
                list.children(self.query.as_ref().map(|(editor, _)| {
                    row()
                        .items_center()
                        .h(px(34.))
                        .gap(px(8.))
                        .px(px(7.))
                        .border_b_1()
                        .border_color(p.line)
                        .child(div().size(px(14.)).flex_none().child(icon(
                            Glyph::Search,
                            14.,
                            p.faint,
                        )))
                        .child(div().flex_1().h_full().min_w_0().child(editor.clone()))
                }))
            })
            .when(indices.is_empty(), |list| {
                list.child(
                    div()
                        .px_2()
                        .py_1()
                        .child(hint(tr(cx, "No matching steps", "没有匹配的步骤"), p)),
                )
            })
            .children(indices.into_iter().enumerate().map(|(row_index, index)| {
                let step = &self.catalog[index];
                let (mark, color) = app_mark(step);
                activate(
                    face(
                        ("workflow-option", index),
                        step.title.clone(),
                        34.,
                        5.,
                        if !empty && self.menu_index == row_index {
                            Tone::Surface
                        } else {
                            Tone::Clear
                        },
                        cx,
                    )
                    .w_full()
                    .px_2()
                    .justify_start()
                    .gap_2()
                    .text_size(px(13.))
                    .child(
                        div()
                            .size(px(16.))
                            .flex_none()
                            .child(icon(mark, 16., color)),
                    )
                    .child(div().flex_1().truncate().child(step.title.clone()))
                    .child(hint(kind(step.kind, cx), p)),
                    cx,
                    move |s, w, cx| {
                        s.add(index, cx);
                        s.close_menu(w, cx);
                    },
                )
            }))
    }
}

impl Render for AiWorkflow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let visible = self.visible && w.is_visible();
        let motion = visible && !disabled_motion(cx);
        let pulse = if self.running && motion {
            self.rail.time(0.24, w, cx) / 0.24
        } else {
            1.
        };
        let reveal = if motion {
            ease(self.result.time(0.24, w, cx) / 0.24)
        } else {
            1.
        };
        let path_focus = self
            .path_focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let menu_focus = self
            .menu_focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let compact = w.viewport_size().width < px(510.);
        let row_height = if compact { 44. } else { 35. };
        let mut list = column()
            .id("workflow-steps")
            .w_full()
            .px(px(8.))
            .py(px(7.))
            .role(Role::List);
        let mut steps = self
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| self.active(step))
            .peekable();
        while let Some((index, step)) = steps.next() {
            let status = state(step.status, cx);
            let (mark, color) = app_mark(step);
            let mut entry = column()
                .id((ElementId::from("workflow-entry"), step.id.clone()))
                .role(Role::ListItem)
                .aria_label(format!("{} · {} · {status}", step.title, step.app))
                .relative();
            if let Some((_, next)) = steps.peek() {
                let receiving = next.status == WorkflowStatus::Running;
                let fill = if receiving {
                    pulse
                } else if next.status == WorkflowStatus::Done {
                    1.
                } else {
                    0.
                };
                entry = entry.child(
                    div()
                        .absolute()
                        .left(px(16.5))
                        .top(px(row_height / 2.))
                        .bottom(px(-row_height / 2.))
                        .w(px(1.))
                        .bg(p.line)
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .top_0()
                                .w_full()
                                .h(relative(fill))
                                .bg(p.faint.opacity(0.74)),
                        )
                        .when(receiving && motion && pulse < 1., |rail| {
                            rail.child(
                                div()
                                    .absolute()
                                    .left(px(-2.))
                                    .top(relative(pulse))
                                    .mt(px(-2.))
                                    .size(px(5.))
                                    .rounded_full()
                                    .bg(p.bright),
                            )
                        }),
                );
            }
            let detail = div()
                .min_w_0()
                .truncate()
                .text_size(px(11.))
                .text_color(p.faint)
                .child(step.detail.clone());
            let title = div()
                .truncate()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(step.title.clone());
            let label = if compact {
                column()
                    .flex_1()
                    .min_w_0()
                    .child(title)
                    .when(!step.detail.is_empty(), |node| node.child(detail))
            } else {
                row()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(7.))
                    .child(title.flex_none())
                    .when(!step.detail.is_empty(), |node| node.child(detail))
            };
            let status_node = match step.status {
                WorkflowStatus::Done => row()
                    .gap(px(4.))
                    .items_center()
                    .child(icon(Glyph::Check, 12., p.success))
                    .child(
                        step.duration_ms
                            .map(elapsed)
                            .unwrap_or_else(|| status.into()),
                    ),
                WorkflowStatus::Failed => row()
                    .gap(px(4.))
                    .items_center()
                    .child(icon(Glyph::Alert, 12., p.danger))
                    .child(status),
                WorkflowStatus::Running => row()
                    .gap(px(4.))
                    .items_center()
                    .child(div().size(px(5.)).rounded_full().bg(p.bright))
                    .child(status),
                _ => row(),
            };
            entry = entry.child(
                row()
                    .h(px(row_height))
                    .px(px(8.))
                    .gap(px(10.))
                    .items_center()
                    .rounded(radius)
                    .hover(|v| v.bg(p.hover.opacity(0.32)))
                    .child(
                        div()
                            .relative()
                            .size(px(18.))
                            .flex_none()
                            .rounded(px(4.))
                            .bg(p.panel)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(mark, 16., color)),
                    )
                    .child(label)
                    .child(
                        status_node
                            .w(px(if compact { 57. } else { 70. }))
                            .flex_none()
                            .justify_end()
                            .text_size(px(11.))
                            .text_color(p.secondary),
                    )
                    .child(div().size(px(20.)).flex_none().when(
                        !self.running && step.kind != WorkflowKind::Trigger,
                        |slot| {
                            slot.child(activate(
                                face(
                                    (ElementId::from("workflow-remove"), step.id.clone()),
                                    format!("{} {}", tr(cx, "Remove", "移除"), step.title),
                                    20.,
                                    4.,
                                    Tone::Clear,
                                    cx,
                                )
                                .w(px(20.))
                                .child(icon(
                                    Glyph::Close,
                                    11.,
                                    p.faint,
                                )),
                                cx,
                                move |s, _, cx| s.remove(index, cx),
                            ))
                        },
                    )),
            );
            if let Some(output) = &step.output
                && !output.is_empty()
            {
                let animating = self.latest_result == Some(index) && reveal < 1.;
                let content = column()
                    .pb(px(10.))
                    .pt(px(2.))
                    .gap(px(2.))
                    .flex_none()
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .children(output.lines().enumerate().map(|(i, line)| {
                        div()
                            .text_color(if i == 0 { p.ink } else { p.secondary })
                            .when(i == 0, |line| line.font_weight(FontWeight::MEDIUM))
                            .child(line.to_owned())
                    }));
                entry = entry.child(
                    column()
                        .ml(px(36.))
                        .mr(px(8.))
                        .when(animating, |container| {
                            let height = self.result_height.clone();
                            container
                                .opacity(reveal)
                                .max_h(height.get() * reveal)
                                .overflow_hidden()
                                .on_children_prepainted(move |bounds, w, _| {
                                    if let Some(bounds) = bounds.first()
                                        && bounds.size.height != height.get()
                                    {
                                        height.set(bounds.size.height);
                                        w.request_animation_frame();
                                    }
                                })
                        })
                        .child(content),
                );
            }
            if let Some(error) = &step.error {
                entry = entry.child(
                    row()
                        .ml(px(36.))
                        .mr(px(8.))
                        .mb(px(7.))
                        .gap(px(6.))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.))
                                .text_color(p.danger)
                                .child(error.clone()),
                        )
                        .child(activate(
                            face(
                                "workflow-retry",
                                tr(cx, "Retry", "重试"),
                                23.,
                                5.,
                                Tone::Surface,
                                cx,
                            )
                            .flex_none()
                            .px_2()
                            .text_size(px(12.))
                            .child(tr(cx, "Retry", "重试")),
                            cx,
                            |s, _, cx| s.retry(cx),
                        )),
                );
            }
            if step.kind == WorkflowKind::Branch && !self.paths.is_empty() {
                entry = entry.child(self.paths_view(&path_focus, cx));
            }
            list = list.child(entry);
        }
        let (label, mark) = if self.running {
            (tr(cx, "Stop", "停止"), Glyph::Pause)
        } else if self.completed {
            (tr(cx, "Run again", "重新运行"), Glyph::Redo)
        } else {
            (tr(cx, "Test run", "测试运行"), Glyph::Play)
        };
        let total = self
            .steps
            .iter()
            .filter(|step| self.active(step))
            .filter_map(|step| step.duration_ms)
            .sum();
        let footer = if self.completed {
            format!(
                "{} · {}",
                tr(cx, "Run complete", "运行完成"),
                elapsed(total)
            )
        } else if self.running {
            tr(cx, "Test run in progress", "正在测试运行").into()
        } else if self
            .steps
            .iter()
            .any(|step| self.active(step) && step.status == WorkflowStatus::Failed)
        {
            tr(cx, "Run failed", "运行失败").into()
        } else if self.simulated {
            tr(cx, "Simulated run", "模拟运行").into()
        } else {
            format!(
                "{} {}",
                self.steps.iter().filter(|step| self.active(step)).count(),
                tr(cx, "steps", "个步骤")
            )
        };
        let active_path = self
            .paths
            .iter()
            .find(|path| Some(&path.id) == self.selected_path.as_ref());
        let add_label = if self
            .steps
            .iter()
            .any(|step| step.kind == WorkflowKind::Branch)
        {
            active_path
                .map(|path| format!("{} {}", tr(cx, "Add to", "添加到"), path.label))
                .unwrap_or_else(|| tr(cx, "Add step", "添加步骤").into())
        } else {
            tr(cx, "Add step", "添加步骤").into()
        };
        let run_disabled = self.steps.is_empty() || self.loading;
        let button = face("workflow-run", label, 28., 6., Tone::Primary, cx)
            .px(px(9.))
            .gap(px(5.))
            .flex_none()
            .text_size(px(12.))
            .when(run_disabled, |button| button.tab_index(-1).cursor_default())
            .opacity(if run_disabled { 0.42 } else { 1. })
            .child(icon(mark, 12., p.inverse))
            .child(label);
        let button = if run_disabled {
            button
        } else {
            activate(
                button,
                cx,
                |s, _, cx| if s.running { s.stop(cx) } else { s.start(cx) },
            )
        };
        let ready = !self.loading && !self.steps.is_empty();
        column()
            .w_full()
            .max_w(px(640.))
            .rounded(radius)
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .w_full()
                    .min_h(px(55.))
                    .gap(px(8.))
                    .items_center()
                    .py(px(9.))
                    .pl(px(16.))
                    .pr(px(12.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(
                        column()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(px(16.))
                                    .text_color(p.secondary)
                                    .child(self.description.clone()),
                            ),
                    )
                    .child(button),
            )
            .when(ready, |body| body.child(list))
            .when(self.loading, |body| {
                body.child(
                    column()
                        .id("workflow-loading")
                        .role(Role::Status)
                        .aria_label(tr(cx, "Loading workflow", "正在加载工作流"))
                        .px(px(16.))
                        .py(px(10.))
                        .children([0.52, 0.66, 0.43, 0.59, 0.48].into_iter().map(|width| {
                            row()
                                .h(px(row_height))
                                .items_center()
                                .gap(px(9.))
                                .child(div().size(px(16.)).rounded(px(4.)).bg(p.line))
                                .child(
                                    div()
                                        .w(relative(width))
                                        .h(px(7.))
                                        .rounded(px(3.))
                                        .bg(p.line),
                                )
                        })),
                )
            })
            .when(!self.loading && self.steps.is_empty(), |body| {
                body.child(
                    column()
                        .px(px(16.))
                        .py(px(12.))
                        .gap(px(8.))
                        .child(hint(tr(cx, "Choose a trigger", "选择触发器"), p))
                        .child(self.catalog_view(true, cx)),
                )
            })
            .child(
                row()
                    .relative()
                    .min_h(px(44.))
                    .px(px(12.))
                    .py(px(5.))
                    .border_t_1()
                    .border_color(p.line)
                    .justify_between()
                    .items_center()
                    .gap(px(4.))
                    .flex_wrap()
                    .child(div().text_size(px(11.)).text_color(p.faint).child(footer))
                    .when(ready && !self.catalog.is_empty(), |row| {
                        row.child(activate(
                            face("workflow-add", add_label.clone(), 27., 5., Tone::Clear, cx)
                                .track_focus(&menu_focus)
                                .aria_expanded(self.menu_open)
                                .when(self.running, |button| button.tab_index(-1).cursor_default())
                                .px(px(7.))
                                .gap(px(4.))
                                .text_size(px(12.))
                                .child(icon(Glyph::Plus, 12., p.secondary))
                                .child(add_label),
                            cx,
                            |s, w, cx| s.toggle_menu(w, cx),
                        ))
                    })
                    .when(self.menu_open, |row| {
                        row.child(
                            div()
                                .absolute()
                                .right(px(8.))
                                .bottom(px(42.))
                                .w(relative(0.94))
                                .max_w(px(300.))
                                .shadow_md()
                                .child(self.catalog_view(false, cx)),
                        )
                    }),
            )
    }
}
