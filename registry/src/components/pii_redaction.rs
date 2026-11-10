//! Review and edit host-supplied redaction ranges before copying a masked preview.
use crate::{
    components::textarea::{Textarea, TextareaStyle},
    primitives::{
        brand_mark::brand_mark,
        clipboard,
        control::UiLanguage,
        family::{Paint, Tone, face},
        field::{Field, FieldEvent},
        form_chrome::check_stroke,
    },
};
use gpui::{prelude::*, *};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PiiKind {
    Name,
    Email,
    Phone,
    Card,
    Other,
}
impl PiiKind {
    fn token(self) -> &'static str {
        match self {
            Self::Name => "NAME",
            Self::Email => "EMAIL",
            Self::Phone => "PHONE",
            Self::Card => "CARD",
            Self::Other => "OTHER",
        }
    }
    fn label(self, lang: UiLanguage) -> &'static str {
        match self {
            Self::Name => lang.text("Name", "姓名"),
            Self::Email => lang.text("Email", "邮箱"),
            Self::Phone => lang.text("Phone", "电话"),
            Self::Card => lang.text("Card", "卡号"),
            Self::Other => lang.text("Other", "其他"),
        }
    }
}
/// A half-open UTF-8 byte range in the exact text submitted to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiiSpan {
    pub start: usize,
    pub end: usize,
    pub kind: PiiKind,
}
impl PiiSpan {
    pub fn new(start: usize, end: usize, kind: PiiKind) -> Self {
        Self { start, end, kind }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PiiStatus {
    #[default]
    Idle,
    Scanning,
    Ready,
    Stopped,
    Failed,
}
#[derive(Clone, Debug)]
pub enum PiiEvent {
    ScanRequested { id: u64, text: SharedString },
    Cancelled(u64),
}
fn valid_spans(text: &str, spans: &mut [PiiSpan]) -> bool {
    spans.sort_by_key(|span| (span.start, span.end));
    let boundaries: Vec<usize> = text
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut end = 0;
    for span in spans {
        if span.start < end
            || span.start >= span.end
            || span.end > text.len()
            || boundaries.binary_search(&span.start).is_err()
            || boundaries.binary_search(&span.end).is_err()
        {
            return false;
        }
        end = span.end;
    }
    true
}
fn redaction_tokens(text: &str, spans: &[PiiSpan]) -> Option<Vec<(PiiSpan, String)>> {
    let mut sorted = spans.to_vec();
    if !valid_spans(text, &mut sorted) {
        return None;
    }
    let mut counts: HashMap<PiiKind, usize> = HashMap::new();
    let mut tokens: HashMap<(PiiKind, &str), String> = HashMap::new();
    Some(
        sorted
            .into_iter()
            .map(|span| {
                let original = &text[span.start..span.end];
                let token = tokens.entry((span.kind, original)).or_insert_with(|| {
                    let count = counts.entry(span.kind).or_insert(0);
                    loop {
                        *count += 1;
                        let candidate = format!("[{}_{}]", span.kind.token(), count);
                        if !text.contains(&candidate) {
                            break candidate;
                        }
                    }
                });
                (span, token.clone())
            })
            .collect(),
    )
}

fn mask_text(text: &str, spans: &[PiiSpan]) -> Option<String> {
    let mut output = String::new();
    let mut previous = 0;
    for (span, token) in redaction_tokens(text, spans)? {
        output.push_str(&text[previous..span.start]);
        output.push_str(&token);
        previous = span.end;
    }
    output.push_str(&text[previous..]);
    Some(output)
}
/// Does not detect, transmit or send any content. The host supplies every scan result.
pub struct PiiRedaction {
    source: Entity<Textarea>,
    spans: Vec<PiiSpan>,
    painted: Rc<RefCell<Vec<PiiSpan>>>,
    snapshot: SharedString,
    model: Option<SharedString>,
    status: PiiStatus,
    active: Option<u64>,
    sequence: u64,
    reviewed: bool,
    selected_kind: PiiKind,
    history: Vec<Vec<PiiSpan>>,
    error: SharedString,
    copied: Option<bool>,
    copy_epoch: u64,
    action_focus: FocusHandle,
    _sub: Subscription,
}
impl EventEmitter<PiiEvent> for PiiRedaction {}
impl PiiRedaction {
    pub fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let painted: Rc<RefCell<Vec<PiiSpan>>> = Rc::new(RefCell::new(Vec::new()));
        let marks = painted.clone();
        let source = cx.new(|cx| {
            Field::<TextareaStyle>::new("pii-source", l.text("Message", "原始文本"), w, cx)
        });
        source.read(cx).editor().update(cx, |editor, _| {
            editor.set_run_styler(Rc::new(move |text, base| {
                let mut spans = marks.borrow().clone();
                if !valid_spans(text, &mut spans) {
                    return vec![base.clone()];
                }
                let mut runs = Vec::new();
                let mut at = 0;
                for span in spans {
                    if span.start > at {
                        let mut run = base.clone();
                        run.len = span.start - at;
                        runs.push(run);
                    }
                    let mut marked = base.clone();
                    marked.len = span.end - span.start;
                    marked.background_color = Some(base.color.opacity(0.18));
                    runs.push(marked);
                    at = span.end;
                }
                if at < text.len() || runs.is_empty() {
                    let mut run = base.clone();
                    run.len = text.len() - at;
                    runs.push(run);
                }
                runs
            }));
        });
        let sub = cx.subscribe_in(&source, w, |s: &mut Self, _, event: &FieldEvent, _, cx| {
            if let FieldEvent::Changed(value) = event
                && value != &s.snapshot
            {
                s.invalidate(cx);
            }
        });
        Self {
            source,
            spans: Vec::new(),
            painted,
            snapshot: "".into(),
            model: None,
            status: PiiStatus::Idle,
            active: None,
            sequence: 0,
            reviewed: false,
            selected_kind: PiiKind::Other,
            history: Vec::new(),
            error: "".into(),
            copied: None,
            copy_epoch: 0,
            action_focus: cx.focus_handle().tab_stop(true),
            _sub: sub,
        }
    }
    /// Name the host-selected scan model in the status row.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    pub fn source(&self) -> Entity<Textarea> {
        self.source.clone()
    }
    pub fn status(&self) -> PiiStatus {
        self.status
    }
    pub fn spans(&self) -> &[PiiSpan] {
        &self.spans
    }
    pub fn active_request(&self) -> Option<u64> {
        self.active
    }
    fn source_text(&self, cx: &App) -> String {
        self.source.read(cx).value(cx).to_owned()
    }
    fn composing(&self, cx: &App) -> bool {
        self.source.read(cx).editor().read(cx).is_composing()
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.take() {
            cx.emit(PiiEvent::Cancelled(id));
        }
    }
    fn repaint_source(&mut self, cx: &mut Context<Self>) {
        *self.painted.borrow_mut() = self.spans.clone();
        self.source
            .read(cx)
            .editor()
            .update(cx, |_, cx| cx.notify());
    }
    fn invalidate(&mut self, cx: &mut Context<Self>) {
        self.cancel(cx);
        self.spans.clear();
        self.history.clear();
        self.snapshot = "".into();
        self.status = PiiStatus::Idle;
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.error = "".into();
        self.repaint_source(cx);
        cx.notify();
    }
    /// Explicitly asks the host to scan the current text. It is never scanned or transmitted automatically.
    pub fn scan(&mut self, cx: &mut Context<Self>) -> bool {
        if self.composing(cx) {
            return false;
        }
        let text: SharedString = self.source_text(cx).into();
        if text.trim().is_empty() {
            self.error = UiLanguage::current(cx)
                .text("Enter text first", "请先输入文本")
                .into();
            cx.notify();
            return false;
        }
        self.cancel(cx);
        if self.snapshot != text {
            self.spans.clear();
            self.history.clear();
        }
        self.snapshot = text.clone();
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.error = "".into();
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.active = Some(self.sequence);
        self.status = PiiStatus::Scanning;
        self.repaint_source(cx);
        cx.emit(PiiEvent::ScanRequested {
            id: self.sequence,
            text,
        });
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.active.is_some() {
            self.cancel(cx);
            self.status = PiiStatus::Stopped;
            self.reviewed = false;
            cx.notify();
        }
    }
    /// Reject the complete result if any range is overlapping, invalid or splits a Unicode grapheme.
    pub fn finish_scan(
        &mut self,
        id: u64,
        mut spans: Vec<PiiSpan>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) || self.status != PiiStatus::Scanning {
            return false;
        }
        if self.composing(cx) || self.source_text(cx) != self.snapshot.as_ref() {
            self.invalidate(cx);
            return false;
        }
        self.active = None;
        if !valid_spans(self.snapshot.as_ref(), &mut spans) {
            self.status = PiiStatus::Failed;
            self.reviewed = false;
            self.error = UiLanguage::current(cx)
                .text(
                    "Invalid or overlapping ranges; nothing was applied.",
                    "识别范围无效或重叠，未应用结果。",
                )
                .into();
            cx.notify();
            return true;
        }
        self.history.push(self.spans.clone());
        self.spans = spans;
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.repaint_source(cx);
        cx.notify();
        true
    }
    pub fn fail_scan(
        &mut self,
        id: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) {
            return false;
        }
        if self.composing(cx) || self.source_text(cx) != self.snapshot.as_ref() {
            self.invalidate(cx);
            return false;
        }
        self.active = None;
        self.status = PiiStatus::Failed;
        self.reviewed = false;
        self.error = message.into();
        cx.notify();
        true
    }
    /// Add a user-selected source range. It must not overlap an existing range.
    pub fn add_range(&mut self, span: PiiSpan, cx: &mut Context<Self>) -> bool {
        if self.composing(cx) {
            return false;
        }
        let current: SharedString = self.source_text(cx).into();
        if current.is_empty() {
            return false;
        }
        if self.snapshot != current {
            // A Changed event may not have reached us yet. Invalidate approval
            // before validating the selection, including on the failure path.
            self.invalidate(cx);
            self.snapshot = current;
        }
        let mut next = self.spans.clone();
        next.push(span);
        if !valid_spans(self.snapshot.as_ref(), &mut next) {
            self.error = UiLanguage::current(cx)
                .text(
                    "Select a non-overlapping text range.",
                    "请选择不重叠的有效文本范围。",
                )
                .into();
            cx.notify();
            return false;
        }
        self.cancel(cx);
        self.history.push(self.spans.clone());
        self.spans = next;
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.error = "".into();
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.repaint_source(cx);
        cx.notify();
        true
    }
    pub fn add_selection(&mut self, kind: PiiKind, cx: &mut Context<Self>) -> bool {
        if self.composing(cx) {
            return false;
        }
        let range = self.source.read(cx).editor().read(cx).selection();
        self.add_range(PiiSpan::new(range.start, range.end, kind), cx)
    }
    pub fn remove_range(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if self.source_text(cx) != self.snapshot.as_ref()
            || index >= self.spans.len()
            || self.composing(cx)
        {
            return false;
        }
        self.cancel(cx);
        self.history.push(self.spans.clone());
        self.spans.remove(index);
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.repaint_source(cx);
        cx.notify();
        true
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) -> bool {
        if self.source_text(cx) != self.snapshot.as_ref() || self.composing(cx) {
            return false;
        }
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.cancel(cx);
        self.spans = previous;
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.repaint_source(cx);
        cx.notify();
        true
    }
    /// A preview is available only for the exact text against which ranges were checked.
    pub fn preview(&self, cx: &App) -> Option<String> {
        if self.status != PiiStatus::Ready
            || self.composing(cx)
            || self.source_text(cx) != self.snapshot.as_ref()
        {
            return None;
        }
        mask_text(self.snapshot.as_ref(), &self.spans)
    }
    pub fn set_reviewed(&mut self, reviewed: bool, cx: &mut Context<Self>) -> bool {
        if self.preview(cx).is_none() {
            return false;
        }
        self.reviewed = reviewed;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        cx.notify();
        true
    }
    /// Copy is enabled only after the reader explicitly reviews the current ranges.
    pub fn copy_preview(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.reviewed {
            return false;
        }
        let Some(text) = self.preview(cx) else {
            return false;
        };
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        let epoch = self.copy_epoch;
        let task = clipboard::write(&text, cx);
        cx.spawn(async move |this, cx| {
            let ok = task.await;
            let _ = this.update(cx, |s, cx| {
                if s.copy_epoch == epoch {
                    s.copied = Some(ok);
                    cx.notify();
                }
            });
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1800))
                .await;
            let _ = this.update(cx, |s, cx| {
                if s.copy_epoch == epoch {
                    s.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
        true
    }
}
impl Render for PiiRedaction {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let status = match self.status {
            PiiStatus::Idle => t("Awaiting scan or manual selection", "等待识别或手动选择"),
            PiiStatus::Scanning => t("Scanning", "正在识别"),
            PiiStatus::Ready => t("Review all ranges before copying", "复制前请核对全部范围"),
            PiiStatus::Stopped => t("Stopped", "已停止"),
            PiiStatus::Failed => t("Scan failed", "识别失败"),
        };
        let mut actions = div().flex().flex_wrap().gap(px(8.));
        if self.active.is_some() {
            actions = actions.child(
                face(
                    "pii-action",
                    t("Stop", "停止"),
                    32.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .track_focus(&self.action_focus)
                .px(px(14.))
                .child(t("Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        } else {
            let label = if matches!(self.status, PiiStatus::Failed | PiiStatus::Stopped) {
                t("Try again", "重试")
            } else {
                t("Scan", "识别")
            };
            actions = actions.child(
                face("pii-action", label, 32., radius.into(), Tone::Primary, cx)
                    .track_focus(&self.action_focus)
                    .px(px(14.))
                    .child(label)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.scan(cx);
                    })),
            );
        }
        let mut kinds = div().flex().flex_wrap().gap(px(8.)).mt(px(4.));
        for (i, kind) in [
            PiiKind::Name,
            PiiKind::Email,
            PiiKind::Phone,
            PiiKind::Card,
            PiiKind::Other,
        ]
        .into_iter()
        .enumerate()
        {
            kinds = kinds.child(
                face(
                    ("pii-kind", i),
                    kind.label(l),
                    28.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(2.))
                .text_size(px(14.))
                .line_height(px(20.))
                .aria_toggled((self.selected_kind == kind).into())
                .when(self.selected_kind == kind, |v| {
                    v.text_color(p.ink).font_weight(FontWeight::MEDIUM)
                })
                .when(self.selected_kind != kind, |v| {
                    v.text_color(p.secondary).hover(|s| s.text_color(p.ink))
                })
                .child(kind.label(l))
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.selected_kind = kind;
                    cx.notify();
                })),
            );
        }
        let mut list = div()
            .id("pii-ranges")
            .role(Role::List)
            .aria_label(t("Redaction ranges", "脱敏范围"))
            .flex()
            .flex_wrap()
            .gap(px(8.));
        for (i, (span, token)) in redaction_tokens(&self.snapshot, &self.spans)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            let label = format!("{} · {token}", span.kind.label(l));
            let remove = format!("{} {}", t("Remove", "移除"), label);
            list = list.child(
                div()
                    .id(("pii-range", i))
                    .role(Role::ListItem)
                    .flex()
                    .items_center()
                    .justify_between()
                    .min_w(px(140.))
                    .flex_1()
                    .p(px(6.))
                    .rounded(radius)
                    .bg(p.tray)
                    .min_h(px(28.))
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(label),
                    )
                    .child(
                        face(
                            ("pii-remove", i),
                            remove,
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(8.))
                        .child(t("Remove", "移除"))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.remove_range(i, cx);
                        })),
                    ),
            );
        }
        let mut root = div()
            .id("pii-redaction")
            .role(Role::Group)
            .aria_label(t("Review redaction", "核对脱敏范围"))
            .w_full()
            .max_w(px(640.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(self.source.clone())
            .child(kinds)
            .child(
                face(
                    "pii-add",
                    t("Mask selected text", "脱敏所选文本"),
                    36.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .child(t("Mask selected text", "脱敏所选文本"))
                .on_click(cx.listener(|s, _, _, cx| {
                    s.add_selection(s.selected_kind, cx);
                })),
            )
            .child(list)
            .child(
                face(
                    "pii-undo",
                    t("Undo range change", "撤销范围修改"),
                    32.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(8.))
                .child(t("Undo", "撤销"))
                .on_click(cx.listener(|s, _, _, cx| {
                    s.undo(cx);
                })),
            )
            .child(
                div()
                    .id("pii-status")
                    .role(Role::Status)
                    .min_h(px(32.))
                    .pt(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.secondary)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .children(
                                self.model
                                    .as_ref()
                                    .and_then(|model| brand_mark(model, 14., cx)),
                            )
                            .child(status),
                    )
                    .child(actions),
            );
        if !self.error.is_empty() {
            root = root.child(
                div()
                    .id("pii-error")
                    .role(Role::Alert)
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.danger)
                    .child(self.error.clone()),
            );
        }
        if let Some(preview) = self.preview(cx) {
            root = root
                .child(
                    div()
                        .min_h(px(90.))
                        .rounded(radius)
                        .bg(p.tray)
                        .px(px(14.))
                        .py(px(12.))
                        .text_size(px(14.))
                        .line_height(px(22.))
                        .text_color(p.ink)
                        .child(
                            div()
                                .mb(px(6.))
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .child(t("Masked preview", "脱敏预览")),
                        )
                        .child(preview),
                )
                .child(
                    face(
                        "pii-reviewed",
                        t("I reviewed these ranges", "我已核对脱敏范围"),
                        36.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .gap(px(8.))
                    .aria_toggled(self.reviewed.into())
                    .child(
                        div()
                            .size(px(16.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if self.reviewed { p.bright } else { p.faint })
                            .bg(if self.reviewed { p.bright } else { p.panel })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(check_stroke(
                                if self.reviewed { 1. } else { 0. },
                                0.,
                                12.,
                                p.inverse,
                            )),
                    )
                    .child(if self.reviewed {
                        t("Reviewed", "已核对")
                    } else {
                        t("I reviewed these ranges", "我已核对脱敏范围")
                    })
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.set_reviewed(!s.reviewed, cx);
                    })),
                );
            let label = match self.copied {
                Some(true) => t("Copied", "已复制"),
                Some(false) => t("Copy failed", "复制失败"),
                None => t("Copy masked text", "复制脱敏文本"),
            };
            let button = face("pii-copy", label, 36., radius.into(), Tone::Surface, cx)
                .px(px(10.))
                .child(label);
            root = root.child(if self.reviewed {
                button.on_click(cx.listener(|s, _, _, cx| {
                    s.copy_preview(cx);
                }))
            } else {
                button.opacity(0.45).tab_index(-1)
            });
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{PiiKind, PiiRedaction, PiiSpan, mask_text};
    use gpui::TestAppContext;

    fn span(text: &str, needle: &str, kind: PiiKind) -> PiiSpan {
        let start = text.find(needle).unwrap();
        PiiSpan::new(start, start + needle.len(), kind)
    }

    #[test]
    fn replaces_unicode_ranges_and_reuses_non_colliding_tokens() {
        let text = "[EMAIL_1] 张三 lin@example.com 和 lin@example.com";
        let name = span(text, "张三", PiiKind::Name);
        let first = span(text, "lin@example.com", PiiKind::Email);
        let last_start = text.rfind("lin@example.com").unwrap();
        let last = PiiSpan::new(
            last_start,
            last_start + "lin@example.com".len(),
            PiiKind::Email,
        );
        assert_eq!(
            mask_text(text, &[last, name, first]).unwrap(),
            "[EMAIL_1] [NAME_1] [EMAIL_2] 和 [EMAIL_2]"
        );
    }

    #[test]
    fn rejects_overlapping_or_split_grapheme_ranges() {
        let text = "e\u{301}张三";
        assert!(mask_text(text, &[PiiSpan::new(0, 1, PiiKind::Other)]).is_none());
        assert!(mask_text(text, &[PiiSpan::new(1, 2, PiiKind::Other)]).is_none());
        assert!(mask_text(text, &[PiiSpan::new(0, text.len() + 1, PiiKind::Other)]).is_none());
        let name = span(text, "张三", PiiKind::Name);
        assert!(mask_text(text, &[name.clone(), name]).is_none());
    }

    #[test]
    fn rejects_stale_results_and_invalidates_review_after_edit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(PiiRedaction::new);
        w.update(|w, cx| {
            let source = view.read(cx).source();
            source.update(cx, |field, cx| field.set_value("Name: 张三", w, cx));
        });
        w.update(|_, cx| view.update(cx, |s, cx| assert!(s.scan(cx))));
        let id = w.update(|_, cx| view.read(cx).active_request().unwrap());
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                let name = span("Name: 张三", "张三", PiiKind::Name);
                assert!(s.finish_scan(id, vec![name], cx));
                assert_eq!(s.preview(cx).as_deref(), Some("Name: [NAME_1]"));
                assert!(s.set_reviewed(true, cx));
            })
        });
        w.update(|w, cx| {
            let source = view.read(cx).source();
            source.update(cx, |field, cx| field.set_value("Name: 李四", w, cx));
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.preview(cx).is_none());
                assert!(!s.copy_preview(cx));
                assert!(!s.finish_scan(id, vec![], cx));
            })
        });
    }

    #[test]
    fn invalid_manual_selection_never_keeps_prior_approval_for_changed_text() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(PiiRedaction::new);
        w.update(|w, cx| {
            view.read(cx)
                .source()
                .update(cx, |field, cx| field.set_value("张三", w, cx))
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.add_range(PiiSpan::new(0, "张三".len(), PiiKind::Name), cx));
                assert!(s.set_reviewed(true, cx));
            })
        });
        w.update(|w, cx| {
            view.read(cx)
                .source()
                .update(cx, |field, cx| field.set_value("李四", w, cx));
            view.update(cx, |s, cx| {
                assert!(!s.add_range(PiiSpan::new(0, 0, PiiKind::Name), cx));
                assert!(s.preview(cx).is_none());
                assert!(!s.copy_preview(cx));
            });
        });
    }

    #[test]
    fn blocks_scanning_during_ime_composition() {
        use gpui::EntityInputHandler;
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(PiiRedaction::new);
        let editor = w.update(|_, cx| view.read(cx).source().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(!s.scan(cx));
                assert!(!s.add_selection(PiiKind::Name, cx));
                assert!(s.preview(cx).is_none());
            })
        });
    }
}
