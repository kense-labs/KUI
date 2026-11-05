//! A host-driven enrichment queue for an editable table column.
use crate::{
    components::{
        input::{Input, InputStyle},
        textarea::{Textarea, TextareaStyle},
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face},
        field::{Field, FieldEvent},
    },
};
use gpui::{prelude::*, *};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnrichRow {
    pub id: SharedString,
    pub name: SharedString,
    pub domain: SharedString,
}
impl EnrichRow {
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            domain: "".into(),
        }
    }
    pub fn with_domain(mut self, domain: impl Into<SharedString>) -> Self {
        self.domain = domain.into();
        self
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnrichSource {
    pub label: SharedString,
    pub href: SharedString,
}
impl EnrichSource {
    pub fn new(label: impl Into<SharedString>, href: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnrichResult {
    pub value: SharedString,
    pub source: Option<EnrichSource>,
}
impl EnrichResult {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            source: None,
        }
    }
    pub fn with_source(mut self, source: EnrichSource) -> Self {
        self.source = Some(source);
        self
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnrichStatus {
    #[default]
    Idle,
    Queued,
    Running,
    Done,
    Failed,
    Review,
}
#[derive(Clone, Debug)]
pub enum EnrichEvent {
    Requested {
        id: u64,
        row: EnrichRow,
        prompt: SharedString,
    },
    Cancelled(u64),
    Changed {
        row: SharedString,
        result: EnrichResult,
    },
    OpenSource {
        row: SharedString,
        source: EnrichSource,
    },
}
struct Cell {
    row: EnrichRow,
    input: Entity<Input>,
    status: EnrichStatus,
    request: Option<u64>,
    error: SharedString,
    source: Option<EnrichSource>,
    before: String,
    before_source: Option<EnrichSource>,
    filled: Option<SharedString>,
    programmatic: Option<SharedString>,
    pending: Option<EnrichResult>,
    clock: PhaseClock,
}
/// Controls scheduling and renders cell states; all enrichment work belongs to the host.
pub struct EnrichColumn {
    title: SharedString,
    prompt: Entity<Textarea>,
    cells: Vec<Cell>,
    concurrency: usize,
    request_seq: u64,
    batch_prompt: SharedString,
    editor_open: bool,
    hovered_row: Option<usize>,
    editor_height: Slide,
    _subs: Vec<Subscription>,
}
impl EventEmitter<EnrichEvent> for EnrichColumn {}
impl EnrichColumn {
    pub fn new(
        title: impl Into<SharedString>,
        rows: Vec<EnrichRow>,
        prompt: impl Into<String>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let language = UiLanguage::current(cx);
        let prompt = cx.new(|cx| {
            Field::<TextareaStyle>::new(
                "enrich-prompt",
                language.text("Instruction", "处理指令"),
                w,
                cx,
            )
            .with_value(prompt, w, cx)
        });
        let prompt_sub =
            cx.subscribe_in(&prompt, w, |s: &mut Self, _, event: &FieldEvent, _, cx| {
                if let FieldEvent::Changed(value) = event
                    && value != &s.batch_prompt
                    && s.cells
                        .iter()
                        .any(|c| matches!(c.status, EnrichStatus::Running | EnrichStatus::Queued))
                {
                    s.stop(cx);
                }
            });
        let mut subs = vec![prompt_sub];
        let mut cells: Vec<Cell> = Vec::new();
        let mut seen = HashSet::new();
        for row in rows {
            if !seen.insert(row.id.clone()) {
                continue;
            }
            let id = row.id.clone();
            let name = row.name.clone();
            let input = cx.new(|cx| Field::<InputStyle>::new(id.clone(), name, w, cx));
            let sub = cx.subscribe_in(
                &input,
                w,
                move |s: &mut Self, _, event: &FieldEvent, _, cx| {
                    let FieldEvent::Changed(value) = event else {
                        return;
                    };
                    let Some(i) = s.cells.iter().position(|c| c.row.id == id) else {
                        return;
                    };
                    if s.cells[i].programmatic.take().as_ref() == Some(value) {
                        return;
                    }
                    if let Some(request) = s.cells[i].request.take() {
                        cx.emit(EnrichEvent::Cancelled(request));
                    }
                    // Preserve a result that arrived while this edit was
                    // pending; the user can explicitly accept or reject it.
                    s.cells[i].status = if s.cells[i].pending.is_some() {
                        EnrichStatus::Review
                    } else {
                        EnrichStatus::Idle
                    };
                    s.cells[i].filled = None;
                    s.cells[i].source = None;
                    s.pump(cx);
                    cx.notify();
                },
            );
            cells.push(Cell {
                row,
                input,
                status: EnrichStatus::Idle,
                request: None,
                error: "".into(),
                source: None,
                before: String::new(),
                before_source: None,
                filled: None,
                programmatic: None,
                pending: None,
                clock: Default::default(),
            });
            subs.push(sub);
        }
        Self {
            title: title.into(),
            prompt,
            cells,
            concurrency: 2,
            request_seq: 0,
            batch_prompt: "".into(),
            editor_open: false,
            hovered_row: None,
            editor_height: Slide::snappy(0.),
            _subs: subs,
        }
    }
    pub fn with_concurrency(mut self, count: usize) -> Self {
        self.concurrency = count.clamp(1, 8);
        self
    }
    pub fn prompt(&self) -> Entity<Textarea> {
        self.prompt.clone()
    }
    pub fn field(&self, row: &str) -> Option<Entity<Input>> {
        self.cells
            .iter()
            .find(|c| c.row.id.as_ref() == row)
            .map(|c| c.input.clone())
    }
    pub fn status(&self, row: &str) -> Option<EnrichStatus> {
        self.cells
            .iter()
            .find(|c| c.row.id.as_ref() == row)
            .map(|c| c.status)
    }
    /// Update a row's host data; an in-flight request for the old data is cancelled.
    pub fn set_row(&mut self, row: EnrichRow, cx: &mut Context<Self>) -> bool {
        let Some(i) = self.cells.iter().position(|c| c.row.id == row.id) else {
            return false;
        };
        if self.cells[i].row == row {
            return true;
        }
        if let Some(request) = self.cells[i].request.take() {
            cx.emit(EnrichEvent::Cancelled(request));
        }
        // A value generated for the old row is no longer a result for the new
        // row. Preserve the visible value, but clear its old provenance.
        self.cells[i].status = EnrichStatus::Idle;
        self.cells[i].row = row;
        self.cells[i].pending = None;
        self.cells[i].filled = None;
        self.cells[i].source = None;
        self.pump(cx);
        cx.notify();
        true
    }
    /// Restore a persisted value without emitting a change or interrupting composition.
    pub fn set_cell(
        &mut self,
        id: &str,
        result: EnrichResult,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(i) = self.cells.iter().position(|c| c.row.id.as_ref() == id) else {
            return false;
        };
        let editor = self.cells[i].input.read(cx).editor();
        if editor.read(cx).is_composing() || editor.read(cx).focus.is_focused(w) {
            return false;
        }
        if let Some(request) = self.cells[i].request.take() {
            cx.emit(EnrichEvent::Cancelled(request));
        }
        let cell = &mut self.cells[i];
        cell.programmatic = (cell.input.read(cx).value(cx) != result.value.as_ref())
            .then_some(result.value.clone());
        cell.filled = None;
        cell.source = result.source;
        cell.pending = None;
        cell.status = EnrichStatus::Done;
        cell.input.update(cx, |input, cx| {
            input.set_value(result.value.to_string(), w, cx)
        });
        self.pump(cx);
        cx.notify();
        true
    }
    fn enqueue(&mut self, i: usize, cx: &mut Context<Self>) {
        self.cells[i].before = self.cells[i].input.read(cx).value(cx).to_owned();
        self.cells[i].before_source = self.cells[i].source.clone();
        self.cells[i].pending = None;
        self.cells[i].error = "".into();
        self.cells[i].status = EnrichStatus::Queued;
        self.cells[i].clock.restart(cx);
    }
    fn pump(&mut self, cx: &mut Context<Self>) {
        let prompt: SharedString = self.prompt.read(cx).value(cx).to_owned().into();
        if !self.prompt_ready(cx) || prompt != self.batch_prompt {
            self.stop(cx);
            return;
        }
        let mut slots = self.concurrency.saturating_sub(
            self.cells
                .iter()
                .filter(|c| c.status == EnrichStatus::Running)
                .count(),
        );
        for cell in &mut self.cells {
            if slots == 0 {
                break;
            }
            if cell.status != EnrichStatus::Queued {
                continue;
            }
            self.request_seq = self.request_seq.wrapping_add(1).max(1);
            let id = self.request_seq;
            cell.request = Some(id);
            cell.status = EnrichStatus::Running;
            cell.clock.restart(cx);
            slots -= 1;
            cx.emit(EnrichEvent::Requested {
                id,
                row: cell.row.clone(),
                prompt: prompt.clone(),
            });
        }
    }
    fn prompt_ready(&self, cx: &App) -> bool {
        let editor = self.prompt.read(cx).editor();
        !editor.read(cx).is_composing() && !self.prompt.read(cx).value(cx).trim().is_empty()
    }
    pub fn run_all(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.prompt_ready(cx) {
            return false;
        }
        self.stop(cx);
        self.batch_prompt = self.prompt.read(cx).value(cx).to_owned().into();
        for i in 0..self.cells.len() {
            self.enqueue(i, cx);
        }
        self.pump(cx);
        cx.notify();
        true
    }
    pub fn run_row(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        if !self.prompt_ready(cx) {
            return false;
        }
        let Some(i) = self.cells.iter().position(|c| c.row.id.as_ref() == id) else {
            return false;
        };
        let current_prompt: SharedString = self.prompt.read(cx).value(cx).to_owned().into();
        if self.batch_prompt != current_prompt {
            self.stop(cx);
            self.batch_prompt = current_prompt;
        }
        if matches!(
            self.cells[i].status,
            EnrichStatus::Queued | EnrichStatus::Running
        ) {
            return false;
        }
        self.enqueue(i, cx);
        self.pump(cx);
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        for cell in &mut self.cells {
            if let Some(id) = cell.request.take() {
                cx.emit(EnrichEvent::Cancelled(id));
            }
            if matches!(cell.status, EnrichStatus::Queued | EnrichStatus::Running) {
                cell.status = EnrichStatus::Idle;
                cell.clock.restart(cx);
            }
        }
        cx.notify();
    }
    /// Deliver a result only for the matching active request.
    pub fn finish(
        &mut self,
        request: u64,
        result: EnrichResult,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(i) = self.cells.iter().position(|c| c.request == Some(request)) else {
            return false;
        };
        if !self.prompt_ready(cx) || self.prompt.read(cx).value(cx) != self.batch_prompt.as_ref() {
            self.stop(cx);
            return false;
        }
        self.cells[i].request = None;
        let editor = self.cells[i].input.read(cx).editor();
        if editor.read(cx).is_composing()
            || editor.read(cx).focus.is_focused(w)
            || self.cells[i].input.read(cx).value(cx) != self.cells[i].before
        {
            self.cells[i].pending = Some(result);
            self.cells[i].status = EnrichStatus::Review;
        } else {
            self.write_result(i, result, w, cx);
        }
        self.cells[i].clock.restart(cx);
        self.pump(cx);
        cx.notify();
        true
    }
    fn write_result(&mut self, i: usize, result: EnrichResult, w: &Window, cx: &mut Context<Self>) {
        let cell = &mut self.cells[i];
        cell.programmatic = (cell.input.read(cx).value(cx) != result.value.as_ref())
            .then_some(result.value.clone());
        cell.filled = Some(result.value.clone());
        cell.source = result.source.clone();
        cell.pending = None;
        cell.status = EnrichStatus::Done;
        let value = result.value.to_string();
        cell.input
            .update(cx, |input, cx| input.set_value(value, w, cx));
        cx.emit(EnrichEvent::Changed {
            row: cell.row.id.clone(),
            result,
        });
    }
    pub fn fail(
        &mut self,
        request: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(i) = self.cells.iter().position(|c| c.request == Some(request)) else {
            return false;
        };
        if !self.prompt_ready(cx) || self.prompt.read(cx).value(cx) != self.batch_prompt.as_ref() {
            self.stop(cx);
            return false;
        }
        self.cells[i].request = None;
        self.cells[i].error = error.into();
        self.cells[i].status = EnrichStatus::Failed;
        self.cells[i].clock.restart(cx);
        self.pump(cx);
        cx.notify();
        true
    }
    pub fn review(&mut self, id: &str, accept: bool, w: &Window, cx: &mut Context<Self>) -> bool {
        let Some(i) = self.cells.iter().position(|c| c.row.id.as_ref() == id) else {
            return false;
        };
        let editor = self.cells[i].input.read(cx).editor();
        if editor.read(cx).is_composing() {
            return false;
        }
        let Some(result) = self.cells[i].pending.take() else {
            return false;
        };
        if accept {
            self.write_result(i, result, w, cx);
        } else {
            self.cells[i].status = EnrichStatus::Idle;
        }
        self.cells[i].clock.restart(cx);
        cx.notify();
        true
    }
    /// Reverts only unedited generated values; manual edits are never overwritten.
    pub fn undo(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.stop(cx);
        for cell in &mut self.cells {
            let editor = cell.input.read(cx).editor();
            if editor.read(cx).is_composing() {
                continue;
            }
            if cell
                .filled
                .as_deref()
                .is_some_and(|value| value == cell.input.read(cx).value(cx))
            {
                let old = cell.before.clone();
                cell.programmatic =
                    (cell.input.read(cx).value(cx) != old).then_some(old.clone().into());
                cell.input
                    .update(cx, |input, cx| input.set_value(old, w, cx));
                cell.source = cell.before_source.clone();
            }
            cell.filled = None;
            cell.pending = None;
            cell.status = EnrichStatus::Idle;
        }
        cx.notify();
    }
}
impl Render for EnrichColumn {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        self.editor_height
            .duration(self.editor_open as u8 as f32, 0.24, cx);
        let editor_h = self.editor_height.get(w, cx).clamp(0., 1.);
        let running = self
            .cells
            .iter()
            .any(|c| matches!(c.status, EnrichStatus::Running | EnrichStatus::Queued));
        let mut rows = div()
            .id("enrich-rows")
            .role(Role::RowGroup)
            .flex()
            .flex_col()
            .w_full();
        for (i, cell) in self.cells.iter().enumerate() {
            let fade = ease(cell.clock.time(0.16, w, cx) / 0.16);
            if cell.status == EnrichStatus::Running && !disabled_motion(cx) {
                w.request_animation_frame();
            }
            let status = match cell.status {
                EnrichStatus::Idle => t("Ready", "待处理"),
                EnrichStatus::Queued => t("Queued", "排队中"),
                EnrichStatus::Running => t("Working…", "处理中…"),
                EnrichStatus::Done => t("Done", "已完成"),
                EnrichStatus::Failed => t("Failed", "失败"),
                EnrichStatus::Review => t("Review", "待确认"),
            };
            let key = cell.row.id.clone();
            let label = format!(
                "{} {}",
                if cell.status == EnrichStatus::Failed {
                    t("Retry", "重试")
                } else {
                    t("Run", "运行")
                },
                cell.row.name
            );
            let mut value = div()
                .id(("enrich-cell", i))
                .role(Role::Cell)
                .min_w(px(180.))
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(cell.input.clone())
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(if cell.status == EnrichStatus::Failed {
                            p.danger
                        } else {
                            p.secondary
                        })
                        .opacity(fade)
                        .child(status),
                );
            if cell.status == EnrichStatus::Failed && !cell.error.is_empty() {
                value = value.child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.danger)
                        .child(cell.error.clone()),
                );
            }
            if let Some(pending) = &cell.pending {
                let keep = key.clone();
                let accept = key.clone();
                value = value
                    .child(div().text_size(px(12.)).child(format!(
                        "{}: {}",
                        t("Suggested", "建议值"),
                        pending.value
                    )))
                    .child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(
                                face(
                                    ("enrich-accept", i),
                                    t("Use suggestion", "使用建议"),
                                    32.,
                                    radius.into(),
                                    Tone::Clear,
                                    cx,
                                )
                                .px(px(8.))
                                .child(t("Use suggestion", "使用建议"))
                                .on_click(cx.listener(
                                    move |s, _, w, cx| {
                                        s.review(accept.as_ref(), true, w, cx);
                                    },
                                )),
                            )
                            .child(
                                face(
                                    ("enrich-keep", i),
                                    t("Keep mine", "保留当前值"),
                                    32.,
                                    radius.into(),
                                    Tone::Clear,
                                    cx,
                                )
                                .px(px(8.))
                                .child(t("Keep mine", "保留当前值"))
                                .on_click(cx.listener(
                                    move |s, _, w, cx| {
                                        s.review(keep.as_ref(), false, w, cx);
                                    },
                                )),
                            ),
                    );
            }
            if let Some(source) = &cell.source {
                let source = source.clone();
                let id = key.clone();
                value = value.child(
                    face(
                        ("enrich-source", i),
                        t("Open source", "打开来源"),
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .justify_start()
                    .max_w(px(168.))
                    .px(px(4.))
                    .py(px(4.))
                    .gap(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.secondary)
                    .hover(move |s| s.bg(p.tray).text_color(p.ink))
                    .child(format!("{} ↗", source.label))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.emit(EnrichEvent::OpenSource {
                            row: id.clone(),
                            source: source.clone(),
                        })
                    })),
                );
            }
            let button = if matches!(cell.status, EnrichStatus::Running | EnrichStatus::Queued) {
                None
            } else {
                Some(
                    face(
                        ("enrich-run", i),
                        label,
                        32.,
                        radius.into(),
                        Tone::Surface,
                        cx,
                    )
                    .px(px(8.))
                    .opacity(if self.hovered_row == Some(i) { 1. } else { 0. })
                    .hover(move |s| s.bg(p.tray).text_color(p.ink))
                    .child(if cell.status == EnrichStatus::Failed {
                        t("Retry", "重试")
                    } else {
                        t("Run", "运行")
                    })
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.run_row(key.as_ref(), cx);
                    })),
                )
            };
            rows = rows.child(
                div()
                    .id(("enrich-row", i))
                    .role(Role::Row)
                    .w_full()
                    .h(px(44.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .bg(match cell.status {
                        EnrichStatus::Running => p.bright.opacity(0.05),
                        EnrichStatus::Failed => p.danger.opacity(0.04),
                        _ => p.panel.opacity(0.),
                    })
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        let next = if *over { Some(i) } else { None };
                        if s.hovered_row != next && (s.hovered_row == Some(i) || *over) {
                            s.hovered_row = next;
                            cx.notify();
                        }
                    }))
                    .child(
                        div()
                            .id(("enrich-name", i))
                            .role(Role::RowHeader)
                            .h_full()
                            .pl(px(20.))
                            .pr(px(12.))
                            .w(px(150.))
                            .min_w(px(100.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(14.))
                            .text_color(p.ink)
                            .font_weight(FontWeight::MEDIUM)
                            .child(cell.row.name.clone())
                            .when(!cell.row.domain.is_empty(), |v| {
                                v.child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.secondary)
                                        .child(cell.row.domain.clone()),
                                )
                            }),
                    )
                    .child(value)
                    .when_some(button, |v, button| v.child(button)),
            );
        }
        let toggle = if self.editor_open {
            t("Close editor", "关闭编辑")
        } else {
            t("Edit instruction", "编辑指令")
        };
        let mut actions = div().flex().flex_wrap().gap(px(8.));
        actions = actions.child(
            face(
                "enrich-editor-toggle",
                toggle,
                32.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .px(px(8.))
            .child(toggle)
            .on_click(cx.listener(|s, _, _, cx| {
                s.editor_open = !s.editor_open;
                cx.notify();
            })),
        );
        if running {
            actions = actions.child(
                face(
                    "enrich-stop",
                    t("Stop", "停止"),
                    32.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(8.))
                .child(t("Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        } else {
            actions = actions.child(
                face(
                    "enrich-all",
                    t("Run all", "全部运行"),
                    32.,
                    radius.into(),
                    Tone::Primary,
                    cx,
                )
                .px(px(8.))
                .child(t("Run all", "全部运行"))
                .on_click(cx.listener(|s, _, _, cx| {
                    s.run_all(cx);
                })),
            );
        }
        actions = actions.child(
            face(
                "enrich-undo",
                t("Undo fill", "撤销填写"),
                32.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .px(px(8.))
            .child(t("Undo", "撤销"))
            .on_click(cx.listener(|s, _, w, cx| s.undo(w, cx))),
        );
        div()
            .id("enrich-column")
            .role(Role::Table)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(760.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .flex()
            .flex_col()
            .child(
                div()
                    .id("enrich-header")
                    .role(Role::Row)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .min_h(px(60.))
                    .pl(px(20.))
                    .pr(px(12.))
                    .py(px(12.))
                    .gap(px(12.))
                    .child(
                        div()
                            .id("enrich-title")
                            .role(Role::ColumnHeader)
                            .flex()
                            .items_baseline()
                            .gap(px(8.))
                            .text_size(px(16.))
                            .line_height(px(24.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.title.clone()),
                    )
                    .child(actions),
            )
            .when(editor_h > 0.001, |v| {
                v.child(
                    div()
                        .w_full()
                        .h(px(152. * editor_h))
                        .px(px(20.))
                        .pb(px(16.))
                        .overflow_hidden()
                        .child(self.prompt.clone()),
                )
            })
            .child(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::{EnrichColumn, EnrichResult, EnrichRow, EnrichStatus};
    use gpui::{EntityInputHandler, TestAppContext};
    fn rows() -> Vec<EnrichRow> {
        vec![
            EnrichRow::new("a", "A"),
            EnrichRow::new("b", "B"),
            EnrichRow::new("c", "C"),
        ]
    }
    #[test]
    fn concurrency_cancel_and_undo_preserve_manual_edits() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| EnrichColumn::new("Accounts", rows(), "Look up", w, cx));
        w.update(|_, cx| {
            control.update(cx, |s, cx| {
                assert!(s.run_all(cx));
                assert_eq!(
                    s.cells
                        .iter()
                        .filter(|c| c.status == EnrichStatus::Running)
                        .count(),
                    2
                );
                assert_eq!(
                    s.cells
                        .iter()
                        .filter(|c| c.status == EnrichStatus::Queued)
                        .count(),
                    1
                );
            })
        });
        let first = w.update(|_, cx| control.read(cx).cells[0].request.unwrap());
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                assert!(s.finish(first, EnrichResult::new("Generated"), w, cx));
                assert_eq!(s.cells[2].status, EnrichStatus::Running);
            })
        });
        w.update(|w, cx| {
            control
                .read(cx)
                .field("a")
                .unwrap()
                .update(cx, |field, cx| field.set_value("My edit", w, cx))
        });
        let pending = w.update(|_, cx| control.read(cx).cells[1].request.unwrap());
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.undo(w, cx);
                assert_eq!(s.field("a").unwrap().read(cx).value(cx), "My edit");
                assert!(!s.finish(pending, EnrichResult::new("Late"), w, cx));
            })
        });
    }
    #[test]
    fn pending_manual_and_prompt_edits_cannot_be_overwritten() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| EnrichColumn::new("Accounts", rows(), "Look up", w, cx));
        w.update(|_, cx| control.update(cx, |s, cx| assert!(s.run_all(cx))));
        let first = w.update(|_, cx| control.read(cx).cells[0].request.unwrap());
        let second = w.update(|_, cx| control.read(cx).cells[1].request.unwrap());
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.field("a")
                    .unwrap()
                    .update(cx, |f, cx| f.set_value("Manual", w, cx));
                assert!(s.finish(first, EnrichResult::new("Generated"), w, cx));
                assert_eq!(s.status("a"), Some(EnrichStatus::Review));
                assert_eq!(s.field("a").unwrap().read(cx).value(cx), "Manual");
                s.prompt()
                    .update(cx, |f, cx| f.set_value("New instruction", w, cx));
                assert!(!s.finish(second, EnrichResult::new("Old"), w, cx));
            })
        });
    }

    #[test]
    fn pending_manual_edit_keeps_a_simultaneous_result_for_review() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| EnrichColumn::new("Accounts", rows(), "Look up", w, cx));
        w.update(|_, cx| control.update(cx, |s, cx| assert!(s.run_all(cx))));
        let request = w.update(|_, cx| control.read(cx).cells[0].request.unwrap());
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.field("a")
                    .unwrap()
                    .update(cx, |f, cx| f.set_value("Manual", w, cx));
                assert!(s.finish(request, EnrichResult::new("Generated"), w, cx));
            })
        });
        w.update(|_, cx| {
            let s = control.read(cx);
            assert_eq!(s.status("a"), Some(EnrichStatus::Review));
            assert!(s.cells[0].pending.is_some());
            assert_eq!(s.field("a").unwrap().read(cx).value(cx), "Manual");
        });
    }

    #[test]
    fn changing_a_queued_row_does_not_run_old_work() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| EnrichColumn::new("Accounts", rows(), "Look up", w, cx));
        w.update(|_, cx| {
            control.update(cx, |s, cx| {
                assert!(s.run_all(cx));
                assert_eq!(s.status("c"), Some(EnrichStatus::Queued));
                assert!(s.set_row(EnrichRow::new("c", "New C"), cx));
                assert_eq!(s.status("c"), Some(EnrichStatus::Idle));
            })
        });
    }

    #[test]
    fn result_waits_while_pinyin_is_composing() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| EnrichColumn::new("Accounts", rows(), "Look up", w, cx));
        w.update(|_, cx| {
            control.update(cx, |s, cx| {
                s.run_all(cx);
            })
        });
        let request = w.update(|_, cx| control.read(cx).cells[0].request.unwrap());
        let editor = w.update(|_, cx| control.read(cx).field("a").unwrap().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                // Editing cancels an already running request; a late result is ignored.
                assert!(!s.finish(request, EnrichResult::new("Generated"), w, cx));
                assert_eq!(editor.read(cx).value(), "ni");
                assert!(editor.read(cx).is_composing());
            })
        });
    }
}
