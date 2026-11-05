//! Host-driven extraction into real editable fields, with explicit review and undo.
use crate::{
    components::{
        input::{Input, InputStyle},
        textarea::{Textarea, TextareaStyle},
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Tone, ease, face},
        field::{Field, FieldEvent},
    },
};
use gpui::{prelude::*, *};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartField {
    pub id: SharedString,
    pub label: SharedString,
    pub initial: SharedString,
}
impl SmartField {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            initial: "".into(),
        }
    }
    pub fn with_value(mut self, value: impl Into<SharedString>) -> Self {
        self.initial = value.into();
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SmartSuggestion {
    pub field: SharedString,
    pub value: SharedString,
    pub confidence: f32,
    pub source: SharedString,
    pub reason: SharedString,
}
impl SmartSuggestion {
    pub fn new(
        field: impl Into<SharedString>,
        value: impl Into<SharedString>,
        confidence: f32,
    ) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            confidence: if confidence.is_finite() {
                confidence.clamp(0., 1.)
            } else {
                0.
            },
            source: "".into(),
            reason: "".into(),
        }
    }
    pub fn with_source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = source.into();
        self
    }
    pub fn with_reason(mut self, reason: impl Into<SharedString>) -> Self {
        self.reason = reason.into();
        self
    }
}
#[derive(Clone, Debug)]
pub enum SmartFillEvent {
    Requested { id: u64, text: SharedString },
    Cancelled(u64),
    Submitted(Vec<(SharedString, SharedString)>),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SmartFillStatus {
    #[default]
    Idle,
    Reading,
    Ready,
    Stopped,
    Failed,
    Submitted,
}
struct FieldState {
    spec: SmartField,
    input: Entity<Input>,
    revision: u64,
    start_revision: u64,
    before: String,
    filled: Option<SharedString>,
    programmatic: Option<SharedString>,
    pending: Option<SmartSuggestion>,
    confidence: Option<f32>,
    source: SharedString,
    needs_review: bool,
    flash: PhaseClock,
}
/// Extraction is supplied by the host. No source text leaves this component by itself.
pub struct SmartFormFill {
    source: Entity<Textarea>,
    fields: Vec<FieldState>,
    threshold: f32,
    status: SmartFillStatus,
    error: SharedString,
    request: Option<u64>,
    request_seq: u64,
    request_source: SharedString,
    highlight: Rc<RefCell<Option<SharedString>>>,
    _subs: Vec<Subscription>,
}
impl EventEmitter<SmartFillEvent> for SmartFormFill {}
impl SmartFormFill {
    pub fn new(specs: Vec<SmartField>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let highlight: Rc<RefCell<Option<SharedString>>> = Rc::new(RefCell::new(None));
        let painted = highlight.clone();
        let source = cx.new(|cx| {
            Field::<TextareaStyle>::new(
                "smart-source",
                language.text("Source text", "来源文本"),
                w,
                cx,
            )
        });
        source.read(cx).editor().update(cx, |editor, _| {
            editor.set_run_styler(Rc::new(move |text, base| {
                let query = painted.borrow();
                let Some(needle) = query.as_deref().filter(|s| !s.is_empty()) else {
                    return vec![base.clone()];
                };
                let Some(start) = text.find(needle) else {
                    return vec![base.clone()];
                };
                let end = start + needle.len();
                let mut runs = Vec::new();
                if start > 0 {
                    let mut run = base.clone();
                    run.len = start;
                    runs.push(run);
                }
                let mut selected = base.clone();
                selected.len = end - start;
                selected.background_color = Some(base.color.opacity(0.24));
                runs.push(selected);
                if end < text.len() {
                    let mut run = base.clone();
                    run.len = text.len() - end;
                    runs.push(run);
                }
                runs
            }));
        });
        let mut fields = Vec::new();
        let mut subs = Vec::new();
        let source_sub =
            cx.subscribe_in(&source, w, |s: &mut Self, _, event: &FieldEvent, _, cx| {
                if let FieldEvent::Changed(value) = event
                    && !s.request_source.is_empty()
                    && value != &s.request_source
                {
                    s.invalidate_source(cx);
                }
            });
        subs.push(source_sub);
        for spec in specs {
            if fields.iter().any(|f: &FieldState| f.spec.id == spec.id) {
                continue;
            }
            let id = spec.id.clone();
            let focus_id = id.clone();
            let label = spec.label.clone();
            let initial = spec.initial.to_string();
            let input = cx.new(|cx| {
                Field::<InputStyle>::new(id.clone(), label, w, cx).with_value(initial, w, cx)
            });
            let sub = cx.subscribe_in(
                &input,
                w,
                move |s: &mut Self, _, event: &FieldEvent, _, cx| {
                    let FieldEvent::Changed(value) = event else {
                        return;
                    };
                    let Some(field) = s.fields.iter_mut().find(|f| f.spec.id == id) else {
                        return;
                    };
                    if field.programmatic.take().as_ref() == Some(value) {
                        return;
                    }
                    field.revision = field.revision.wrapping_add(1);
                    field.filled = None;
                    // An edit may have happened just before an async suggestion
                    // arrived. Keep that suggestion for explicit review.
                    field.needs_review = field.pending.is_some();
                    field.confidence = None;
                    let previous_source = std::mem::take(&mut field.source);
                    let lang = UiLanguage::current(cx);
                    let edited = lang.text("Edited", "已编辑");
                    let review = field.needs_review;
                    field.input.update(cx, |input, cx| {
                        input.set_error(
                            if review {
                                lang.text("Review suggested value", "请核对建议值")
                            } else {
                                ""
                            },
                            cx,
                        );
                        input.set_description(edited, cx);
                    });
                    let source_was_highlighted =
                        s.highlight.borrow().as_ref() == Some(&previous_source);
                    if !previous_source.is_empty() && source_was_highlighted {
                        *s.highlight.borrow_mut() = None;
                        s.source.read(cx).editor().update(cx, |_, cx| cx.notify());
                    }
                    cx.notify();
                },
            );
            let observer = cx.observe_in(&input, w, move |s: &mut Self, input, w, cx| {
                let Some(i) = s.fields.iter().position(|f| f.spec.id == focus_id) else {
                    return;
                };
                let focused = input.read(cx).focus_handle(cx).is_focused(w);
                if focused {
                    s.highlight_field(i, true, cx);
                }
            });
            fields.push(FieldState {
                spec,
                input,
                revision: 0,
                start_revision: 0,
                before: String::new(),
                filled: None,
                programmatic: None,
                pending: None,
                confidence: None,
                source: "".into(),
                needs_review: false,
                flash: Default::default(),
            });
            subs.push(sub);
            subs.push(observer);
        }
        Self {
            source,
            fields,
            threshold: 0.8,
            status: SmartFillStatus::Idle,
            error: "".into(),
            request: None,
            request_seq: 0,
            request_source: "".into(),
            highlight,
            _subs: subs,
        }
    }
    pub fn with_review_threshold(mut self, threshold: f32) -> Self {
        if threshold.is_finite() {
            self.threshold = threshold.clamp(0., 1.);
        }
        self
    }
    pub fn source(&self) -> Entity<Textarea> {
        self.source.clone()
    }
    pub fn field(&self, id: &str) -> Option<Entity<Input>> {
        self.fields
            .iter()
            .find(|f| f.spec.id.as_ref() == id)
            .map(|f| f.input.clone())
    }
    pub fn status(&self) -> SmartFillStatus {
        self.status
    }
    pub fn active_request(&self) -> Option<u64> {
        self.request
    }
    pub fn values(&self, cx: &App) -> Vec<(SharedString, SharedString)> {
        self.fields
            .iter()
            .map(|f| {
                (
                    f.spec.id.clone(),
                    f.input.read(cx).value(cx).to_owned().into(),
                )
            })
            .collect()
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.request.take() {
            cx.emit(SmartFillEvent::Cancelled(id));
        }
    }
    fn source_matches_request(&self, cx: &App) -> bool {
        !self.source.read(cx).editor().read(cx).is_composing()
            && self.source.read(cx).value(cx) == self.request_source.as_ref()
    }
    fn invalidate_source(&mut self, cx: &mut Context<Self>) {
        let was_running = self.request.is_some();
        self.cancel(cx);
        self.request_source = "".into();
        self.status = if was_running {
            SmartFillStatus::Stopped
        } else {
            SmartFillStatus::Idle
        };
        self.error = "".into();
        for field in &mut self.fields {
            field.pending = None;
            field.needs_review = false;
            field.confidence = None;
            field.source = "".into();
            field.input.update(cx, |input, cx| {
                input.set_error("", cx);
                input.set_description("", cx);
            });
        }
        *self.highlight.borrow_mut() = None;
        self.source
            .read(cx)
            .editor()
            .update(cx, |_, cx| cx.notify());
        cx.notify();
    }
    /// Starts a new host request; edits to the source automatically cancel it.
    pub fn start(&mut self, cx: &mut Context<Self>) -> bool {
        if self.source.read(cx).editor().read(cx).is_composing() {
            return false;
        }
        let text: SharedString = self.source.read(cx).value(cx).to_owned().into();
        if text.trim().is_empty() {
            let label = UiLanguage::current(cx).text("Enter source text first", "请先输入来源文本");
            self.source
                .update(cx, |input, cx| input.set_error(label, cx));
            return false;
        }
        self.source.update(cx, |input, cx| input.set_error("", cx));
        self.cancel(cx);
        self.request_seq = self.request_seq.wrapping_add(1).max(1);
        self.request = Some(self.request_seq);
        self.request_source = text.clone();
        for field in &mut self.fields {
            field.before = field.input.read(cx).value(cx).to_owned();
            field.start_revision = field.revision;
            field.pending = None;
        }
        self.error = "".into();
        self.status = SmartFillStatus::Reading;
        cx.emit(SmartFillEvent::Requested {
            id: self.request_seq,
            text,
        });
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.request.is_some() {
            self.cancel(cx);
            self.status = SmartFillStatus::Stopped;
            cx.notify();
        }
    }
    /// Applies one host result, or queues it for review when the user is editing the field.
    pub fn apply(
        &mut self,
        request: u64,
        suggestion: SmartSuggestion,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.request != Some(request) {
            return false;
        }
        if !self.source_matches_request(cx) {
            self.invalidate_source(cx);
            return false;
        }
        let Some(i) = self
            .fields
            .iter()
            .position(|f| f.spec.id == suggestion.field)
        else {
            return false;
        };
        let field = &self.fields[i];
        let editor = field.input.read(cx).editor();
        if editor.read(cx).is_composing()
            || editor.read(cx).focus.is_focused(w)
            || field.revision != field.start_revision
            || (field.input.read(cx).value(cx) != field.before
                && field.filled.as_deref() != Some(field.input.read(cx).value(cx)))
        {
            let field = &mut self.fields[i];
            field.pending = Some(suggestion);
            field.needs_review = true;
            field.input.update(cx, |input, cx| {
                input.set_error(
                    UiLanguage::current(cx).text("Review suggested value", "请核对建议值"),
                    cx,
                )
            });
        } else {
            self.write_suggestion(i, suggestion, w, cx);
        }
        cx.notify();
        true
    }
    fn write_suggestion(
        &mut self,
        i: usize,
        suggestion: SmartSuggestion,
        w: &Window,
        cx: &mut Context<Self>,
    ) {
        let l = UiLanguage::current(cx);
        let field = &mut self.fields[i];
        let confidence = if suggestion.confidence.is_finite() {
            suggestion.confidence.clamp(0., 1.)
        } else {
            0.
        };
        field.programmatic = (field.input.read(cx).value(cx) != suggestion.value.as_ref())
            .then_some(suggestion.value.clone());
        field.filled = Some(suggestion.value.clone());
        field.source = suggestion.source.clone();
        field.confidence = Some(confidence);
        field.pending = None;
        field.needs_review = confidence < self.threshold;
        field.flash.restart(cx);
        let description = if suggestion.reason.is_empty() {
            format!(
                "{} {:.0}%",
                l.text("Confidence", "可信度"),
                confidence * 100.
            )
        } else {
            format!(
                "{} {:.0}% · {}",
                l.text("Confidence", "可信度"),
                confidence * 100.,
                suggestion.reason
            )
        };
        let value = suggestion.value.to_string();
        let review = field.needs_review;
        field.input.update(cx, |input, cx| {
            input.set_value(value, w, cx);
            input.set_description(description, cx);
            input.set_error(
                if review {
                    l.text("Review this value", "请核对该值")
                } else {
                    ""
                },
                cx,
            );
        });
        *self.highlight.borrow_mut() = (!field.source.is_empty()).then_some(field.source.clone());
        self.source
            .read(cx)
            .editor()
            .update(cx, |_, cx| cx.notify());
    }
    pub fn finish(&mut self, request: u64, cx: &mut Context<Self>) -> bool {
        if self.request != Some(request) {
            return false;
        }
        if !self.source_matches_request(cx) {
            self.invalidate_source(cx);
            return false;
        }
        self.request = None;
        self.status = SmartFillStatus::Ready;
        cx.notify();
        true
    }
    pub fn fail(
        &mut self,
        request: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.request != Some(request) {
            return false;
        }
        if !self.source_matches_request(cx) {
            self.invalidate_source(cx);
            return false;
        }
        self.request = None;
        self.error = message.into();
        self.status = SmartFillStatus::Failed;
        cx.notify();
        true
    }
    pub fn review(
        &mut self,
        id: &str,
        accept_suggestion: bool,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.request_source.is_empty() && !self.source_matches_request(cx) {
            self.invalidate_source(cx);
            return false;
        }
        let Some(i) = self.fields.iter().position(|f| f.spec.id.as_ref() == id) else {
            return false;
        };
        let editor = self.fields[i].input.read(cx).editor();
        if editor.read(cx).is_composing() {
            return false;
        }
        if accept_suggestion && let Some(suggestion) = self.fields[i].pending.take() {
            self.write_suggestion(i, suggestion, w, cx);
        }
        self.fields[i].pending = None;
        self.fields[i].needs_review = false;
        self.fields[i]
            .input
            .update(cx, |input, cx| input.set_error("", cx));
        cx.notify();
        true
    }
    /// Restores values only while they still match the automatic fill, preserving later edits.
    pub fn undo(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.stop(cx);
        for field in &mut self.fields {
            let editor = field.input.read(cx).editor();
            if editor.read(cx).is_composing() {
                continue;
            }
            if field
                .filled
                .as_deref()
                .is_some_and(|filled| filled == field.input.read(cx).value(cx))
            {
                let value = field.before.clone();
                field.programmatic =
                    (field.input.read(cx).value(cx) != value).then_some(value.clone().into());
                field.input.update(cx, |input, cx| {
                    input.set_value(value, w, cx);
                    input.set_error("", cx);
                    input.set_description("", cx);
                });
            }
            field.filled = None;
            field.pending = None;
            field.needs_review = false;
            field.confidence = None;
            field.source = "".into();
        }
        *self.highlight.borrow_mut() = None;
        self.status = SmartFillStatus::Idle;
        cx.notify();
    }
    pub fn submit(&mut self, w: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.source.read(cx).editor().read(cx).is_composing() {
            return false;
        }
        if !self.request_source.is_empty() && !self.source_matches_request(cx) {
            self.invalidate_source(cx);
            return false;
        }
        if self.request.is_some() {
            return false;
        }
        if let Some(field) = self
            .fields
            .iter()
            .find(|f| f.needs_review || f.pending.is_some())
        {
            field.input.focus_handle(cx).focus(w, cx);
            return false;
        }
        if self
            .fields
            .iter()
            .any(|f| f.input.read(cx).editor().read(cx).is_composing())
        {
            return false;
        }
        cx.emit(SmartFillEvent::Submitted(self.values(cx)));
        self.status = SmartFillStatus::Submitted;
        cx.notify();
        true
    }
    fn highlight_field(&mut self, i: usize, active: bool, cx: &mut Context<Self>) {
        let source = self
            .fields
            .get(i)
            .map(|f| f.source.clone())
            .filter(|s| !s.is_empty());
        *self.highlight.borrow_mut() = if active { source } else { None };
        self.source
            .read(cx)
            .editor()
            .update(cx, |_, cx| cx.notify());
        cx.notify();
    }
}
impl Render for SmartFormFill {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let mut fields = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .min_w(px(240.))
            .flex_1();
        for i in 0..self.fields.len() {
            let field = &self.fields[i];
            let age = field.flash.time(0.9, w, cx);
            let tint = if age < 0.9 {
                (1. - ease(age / 0.9)) * 0.14
            } else {
                0.
            };
            let id = field.spec.id.clone();
            let pending = field.pending.is_some();
            let review = field.needs_review;
            let mut row = div()
                .id(("smart-field", i))
                .rounded(radius)
                .bg(p.focus.opacity(tint))
                .on_hover(cx.listener(move |s, over: &bool, _, cx| s.highlight_field(i, *over, cx)))
                .child(field.input.clone());
            if review {
                if let Some(suggestion) = &field.pending {
                    row = row.child(div().text_size(px(12.)).text_color(p.secondary).child(
                        format!("{}: {}", t("Suggested", "建议值"), suggestion.value),
                    ));
                    let keep_id = id.clone();
                    row = row.child(
                        face(
                            ("smart-keep", i),
                            t("Keep mine", "保留当前值"),
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(8.))
                        .child(t("Keep mine", "保留当前值"))
                        .on_click(cx.listener(move |s, _, w, cx| {
                            s.review(keep_id.as_ref(), false, w, cx);
                        })),
                    );
                }
                let label = if pending {
                    t("Use suggestion", "使用建议")
                } else {
                    t("Mark checked", "确认无误")
                };
                row = row.child(
                    face(
                        ("smart-review", i),
                        label,
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(label)
                    .on_click(cx.listener(move |s, _, w, cx| {
                        s.review(id.as_ref(), true, w, cx);
                    })),
                );
            }
            fields = fields.child(row);
        }
        let status = match self.status {
            SmartFillStatus::Idle => t("Ready", "准备就绪"),
            SmartFillStatus::Reading => t("Reading", "正在读取"),
            SmartFillStatus::Ready => t("Review results", "请核对结果"),
            SmartFillStatus::Stopped => t("Stopped", "已停止"),
            SmartFillStatus::Failed => t("Failed", "失败"),
            SmartFillStatus::Submitted => t("Submitted", "已提交"),
        };
        let mut actions = div().flex().flex_wrap().gap(px(8.));
        if self.request.is_some() {
            actions = actions.child(
                face(
                    "smart-stop",
                    t("Stop", "停止"),
                    36.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .child(t("Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        } else {
            let label = if matches!(
                self.status,
                SmartFillStatus::Failed | SmartFillStatus::Stopped
            ) {
                t("Try again", "重试")
            } else {
                t("Fill from text", "从文本填写")
            };
            actions = actions.child(
                face("smart-run", label, 36., radius.into(), Tone::Surface, cx)
                    .px(px(10.))
                    .child(label)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.start(cx);
                    })),
            );
        }
        actions = actions
            .child(
                face(
                    "smart-undo",
                    t("Undo fill", "撤销填写"),
                    36.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(10.))
                .child(t("Undo", "撤销"))
                .on_click(cx.listener(|s, _, w, cx| s.undo(w, cx))),
            )
            .child(
                face(
                    "smart-save",
                    t("Save", "保存"),
                    36.,
                    radius.into(),
                    Tone::Primary,
                    cx,
                )
                .px(px(10.))
                .child(t("Save", "保存"))
                .on_click(cx.listener(|s, _, w, cx| {
                    s.submit(w, cx);
                })),
            );
        div()
            .id("smart-form-fill")
            .role(Role::Group)
            .aria_label(t("Smart form fill", "智能表单填写"))
            .w_full()
            .max_w(px(860.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .p(px(20.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(20.))
                    .child(div().min_w(px(240.)).flex_1().child(self.source.clone()))
                    .child(fields),
            )
            .when(self.status == SmartFillStatus::Failed, |v| {
                v.child(
                    div()
                        .id("smart-error")
                        .role(Role::Alert)
                        .text_color(p.danger)
                        .child(self.error.clone()),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_between()
                    .items_center()
                    .gap(px(12.))
                    .pt(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .min_h(px(36.))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .text_color(p.secondary)
                            .child(status),
                    )
                    .child(actions),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{SmartField, SmartFormFill, SmartSuggestion};
    use gpui::{EntityInputHandler, TestAppContext};
    #[test]
    fn stale_results_review_and_undo_preserve_user_edits() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            SmartFormFill::new(
                vec![
                    SmartField::new("name", "Name"),
                    SmartField::new("role", "Role"),
                ],
                w,
                cx,
            )
        });
        w.update(|w, cx| {
            let source = control.read(cx).source();
            source.update(cx, |field, cx| {
                field.set_value("Name: Alex\nRole: Designer", w, cx)
            });
            control.update(cx, |form, cx| {
                assert!(form.start(cx));
            });
        });
        let id = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|w, cx| {
            control.update(cx, |form, cx| {
                assert!(form.apply(
                    id,
                    SmartSuggestion::new("name", "Alex", 0.96).with_source("Alex"),
                    w,
                    cx
                ));
                assert!(form.apply(id, SmartSuggestion::new("role", "Designer", 0.5), w, cx));
                assert!(form.finish(id, cx));
                assert!(!form.submit(w, cx));
                assert!(form.review("role", true, w, cx));
                assert!(form.submit(w, cx));
            })
        });
        w.update(|_, cx| {
            control.update(cx, |form, cx| {
                assert_eq!(form.fields[0].confidence, Some(0.96));
                assert_eq!(form.fields[0].source.as_ref(), "Alex");
                form.highlight_field(0, true, cx);
            })
        });
        w.update(|w, cx| {
            let field = control.read(cx).field("name").unwrap();
            field.update(cx, |field, cx| field.set_value("Manually edited", w, cx));
        });
        w.update(|w, cx| {
            control.update(cx, |form, cx| {
                assert!(form.fields[0].confidence.is_none());
                assert!(form.fields[0].source.is_empty());
                assert!(form.highlight.borrow().is_none());
                form.undo(w, cx);
                assert_eq!(
                    form.field("name").unwrap().read(cx).value(cx),
                    "Manually edited"
                );
                assert!(!form.apply(id, SmartSuggestion::new("name", "Late", 1.), w, cx));
            })
        });
    }
    #[test]
    fn pending_source_and_field_edits_are_protected_before_events_arrive() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            SmartFormFill::new(vec![SmartField::new("name", "Name")], w, cx)
        });
        w.update(|w, cx| {
            control
                .read(cx)
                .source()
                .update(cx, |f, cx| f.set_value("Name: Alex", w, cx))
        });
        w.update(|_, cx| control.update(cx, |s, cx| assert!(s.start(cx))));
        let id = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.field("name")
                    .unwrap()
                    .update(cx, |f, cx| f.set_value("Manual", w, cx));
                assert!(s.apply(id, SmartSuggestion::new("name", "Alex", 1.), w, cx));
                assert!(s.fields[0].pending.is_some());
                assert_eq!(s.fields[0].input.read(cx).value(cx), "Manual");
                s.source()
                    .update(cx, |f, cx| f.set_value("Name: Sam", w, cx));
                assert!(!s.finish(id, cx));
                assert!(s.fields[0].pending.is_none());
                assert_eq!(s.fields[0].input.read(cx).value(cx), "Manual");
            })
        });
    }

    #[test]
    fn pending_field_change_keeps_a_simultaneous_suggestion_for_review() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            SmartFormFill::new(vec![SmartField::new("name", "Name")], w, cx)
        });
        w.update(|w, cx| {
            control
                .read(cx)
                .source()
                .update(cx, |f, cx| f.set_value("Name: Alex", w, cx))
        });
        w.update(|_, cx| control.update(cx, |s, cx| assert!(s.start(cx))));
        let id = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.field("name")
                    .unwrap()
                    .update(cx, |f, cx| f.set_value("Manual", w, cx));
                assert!(s.apply(id, SmartSuggestion::new("name", "Alex", 1.), w, cx));
            })
        });
        w.update(|_, cx| {
            let s = control.read(cx);
            assert!(s.fields[0].pending.is_some());
            assert!(s.fields[0].needs_review);
            assert_eq!(s.fields[0].input.read(cx).value(cx), "Manual");
        });
    }

    #[test]
    fn source_composition_cannot_start_extraction() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            SmartFormFill::new(vec![SmartField::new("name", "Name")], w, cx)
        });
        let editor = w.update(|_, cx| control.read(cx).source().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |e, cx| {
                e.focus(w, cx);
                e.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|_, cx| control.update(cx, |s, cx| assert!(!s.start(cx))));
    }

    #[test]
    fn composing_field_is_not_overwritten_by_a_result() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            SmartFormFill::new(vec![SmartField::new("name", "Name")], w, cx)
        });
        w.update(|w, cx| {
            control
                .read(cx)
                .source()
                .update(cx, |field, cx| field.set_value("Name: Alex", w, cx));
            control.update(cx, |form, cx| {
                form.start(cx);
            });
        });
        let id = w.update(|_, cx| control.read(cx).active_request().unwrap());
        let editor = w.update(|_, cx| control.read(cx).field("name").unwrap().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|w, cx| {
            control.update(cx, |form, cx| {
                assert!(form.apply(id, SmartSuggestion::new("name", "Alex", 0.99), w, cx));
                assert!(form.fields[0].pending.is_some());
                assert_eq!(editor.read(cx).value(), "ni");
                assert!(editor.read(cx).is_composing());
                assert!(!form.review("name", true, w, cx));
            })
        });
    }
}
