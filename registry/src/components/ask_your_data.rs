//! A host-driven question-to-query workflow with inspectable local results.
use crate::{
    components::input::{Input, InputStyle},
    primitives::{
        clipboard,
        control::UiLanguage,
        direct_touch::Contact,
        family::{Paint, PhaseClock, Tone, disabled_motion, face, standard_ease},
        field::{Field, FieldEvent},
        glyph::{Glyph, icon},
        nav::HorizontalScrollRegion,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub struct AskDataSeries {
    pub name: SharedString,
    pub values: Vec<f64>,
}
impl AskDataSeries {
    pub fn new(name: impl Into<SharedString>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AskDataResult {
    pub title: SharedString,
    pub categories: Vec<SharedString>,
    pub series: Vec<AskDataSeries>,
    pub row_count: usize,
    pub duration_ms: u64,
    pub insight: SharedString,
}
impl AskDataResult {
    pub fn new(
        title: impl Into<SharedString>,
        categories: Vec<SharedString>,
        series: Vec<AskDataSeries>,
    ) -> Self {
        Self {
            title: title.into(),
            categories,
            series,
            row_count: 0,
            duration_ms: 0,
            insight: "".into(),
        }
    }
    pub fn with_meta(mut self, row_count: usize, duration_ms: u64) -> Self {
        self.row_count = row_count;
        self.duration_ms = duration_ms;
        self
    }
    pub fn with_insight(mut self, insight: impl Into<SharedString>) -> Self {
        self.insight = insight.into();
        self
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AskDataStatus {
    #[default]
    Idle,
    WritingSql,
    RunningQuery,
    Ready,
    Stopped,
    Failed,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AskDataView {
    #[default]
    Bars,
    Lines,
    Table,
}
#[derive(Clone, Debug)]
pub enum AskDataEvent {
    GenerateSql { id: u64, question: SharedString },
    RunQuery { id: u64, sql: SharedString },
    Cancelled(u64),
}
/// The host performs SQL generation and query execution. This view performs neither itself.
pub struct AskYourData {
    question: Entity<Input>,
    source: SharedString,
    run_question: SharedString,
    sql: String,
    status: AskDataStatus,
    active: Option<u64>,
    sequence: u64,
    scanned: usize,
    total: Option<usize>,
    result: Option<AskDataResult>,
    error: SharedString,
    sql_open: bool,
    view: AskDataView,
    chart_focus: FocusHandle,
    chart_bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    selected: usize,
    page: usize,
    hovered_row: Option<usize>,
    plot: PhaseClock,
    table_scroll: ScrollHandle,
    sql_scroll: ScrollHandle,
    copied: Option<bool>,
    copy_epoch: u64,
    action_focus: FocusHandle,
    _sub: Subscription,
}
impl EventEmitter<AskDataEvent> for AskYourData {}
impl AskYourData {
    pub fn new(source: impl Into<SharedString>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let question = cx.new(|cx| {
            Field::<InputStyle>::new(
                "ask-data-question",
                lang.text("Ask a question", "输入问题"),
                w,
                cx,
            )
        });
        let sub = cx.subscribe_in(
            &question,
            w,
            |s: &mut Self, _, event: &FieldEvent, w, cx| match event {
                FieldEvent::Submitted => {
                    s.run(w, cx);
                }
                FieldEvent::Changed(value) if value != &s.run_question => {
                    s.invalidate_result(cx);
                }
                _ => {}
            },
        );
        Self {
            question,
            source: source.into(),
            run_question: "".into(),
            sql: String::new(),
            status: AskDataStatus::Idle,
            active: None,
            sequence: 0,
            scanned: 0,
            total: None,
            result: None,
            error: "".into(),
            sql_open: false,
            view: AskDataView::Bars,
            chart_focus: cx.focus_handle().tab_stop(true),
            chart_bounds: Default::default(),
            contact: Default::default(),
            selected: 0,
            page: 0,
            hovered_row: None,
            plot: PhaseClock::default(),
            table_scroll: ScrollHandle::new(),
            sql_scroll: ScrollHandle::new(),
            copied: None,
            copy_epoch: 0,
            action_focus: cx.focus_handle().tab_stop(true),
            _sub: sub,
        }
    }
    pub fn question(&self) -> Entity<Input> {
        self.question.clone()
    }
    pub fn status(&self) -> AskDataStatus {
        self.status
    }
    pub fn active_request(&self) -> Option<u64> {
        self.active
    }
    pub fn sql(&self) -> &str {
        &self.sql
    }
    fn next_id(&mut self) -> u64 {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.sequence
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.take() {
            cx.emit(AskDataEvent::Cancelled(id));
        }
    }
    fn invalidate_result(&mut self, cx: &mut Context<Self>) {
        let was_active = self.active.is_some();
        self.cancel(cx);
        self.status = if was_active {
            AskDataStatus::Stopped
        } else {
            AskDataStatus::Idle
        };
        self.run_question = "".into();
        self.sql.clear();
        self.result = None;
        self.error = "".into();
        self.scanned = 0;
        self.total = None;
        self.sql_open = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.selected = 0;
        self.page = 0;
        self.plot = PhaseClock::default();
        cx.notify();
    }
    /// Starts a host SQL-generation request; Enter during IME composition does not submit.
    pub fn run(&mut self, w: &Window, cx: &mut Context<Self>) -> bool {
        let editor = self.question.read(cx).editor();
        if editor.read(cx).is_composing() {
            return false;
        }
        let raw_question: SharedString = self.question.read(cx).value(cx).to_owned().into();
        let question: SharedString = raw_question.trim().to_owned().into();
        if question.is_empty() {
            let text = UiLanguage::current(cx).text("Enter a question first", "请先输入问题");
            self.question
                .update(cx, |input, cx| input.set_error(text, cx));
            return false;
        }
        let _ = w;
        self.question
            .update(cx, |input, cx| input.set_error("", cx));
        self.cancel(cx);
        let id = self.next_id();
        self.active = Some(id);
        self.run_question = raw_question;
        self.sql.clear();
        self.status = AskDataStatus::WritingSql;
        self.error = "".into();
        self.scanned = 0;
        self.total = None;
        self.result = None;
        self.sql_open = true;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.selected = 0;
        self.page = 0;
        self.plot = PhaseClock::default();
        cx.emit(AskDataEvent::GenerateSql { id, question });
        cx.notify();
        true
    }
    pub fn append_sql(&mut self, id: u64, chunk: &str, cx: &mut Context<Self>) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::WritingSql {
            return false;
        }
        self.sql.push_str(chunk);
        cx.notify();
        true
    }
    /// Completes SQL generation and asks the host to run it; the new query has its own request ID.
    pub fn finish_sql(&mut self, id: u64, cx: &mut Context<Self>) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::WritingSql {
            return false;
        }
        self.active = None;
        if self.sql.trim().is_empty() {
            self.status = AskDataStatus::Failed;
            self.error = UiLanguage::current(cx)
                .text("No SQL was returned.", "没有收到查询语句。")
                .into();
            cx.notify();
            return true;
        }
        let next = self.next_id();
        self.active = Some(next);
        self.status = AskDataStatus::RunningQuery;
        self.sql_open = false;
        cx.emit(AskDataEvent::RunQuery {
            id: next,
            sql: self.sql.clone().into(),
        });
        cx.notify();
        true
    }
    pub fn progress(
        &mut self,
        id: u64,
        scanned: usize,
        total: Option<usize>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::RunningQuery {
            return false;
        }
        self.scanned = scanned;
        self.total = total;
        cx.notify();
        true
    }
    pub fn finish_query(&mut self, id: u64, result: AskDataResult, cx: &mut Context<Self>) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::RunningQuery {
            return false;
        }
        self.active = None;
        self.result = Some(result);
        self.status = AskDataStatus::Ready;
        self.selected = 0;
        self.page = 0;
        self.plot.restart(cx);
        cx.notify();
        true
    }
    pub fn fail(
        &mut self,
        id: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) {
            return false;
        }
        self.active = None;
        self.status = AskDataStatus::Failed;
        self.error = message.into();
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.active.is_some() {
            self.cancel(cx);
            self.status = AskDataStatus::Stopped;
            cx.notify();
        }
    }
    fn copy_sql(&mut self, cx: &mut Context<Self>) {
        if self.sql.is_empty() {
            return;
        }
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        let epoch = self.copy_epoch;
        let task = clipboard::write(&self.sql, cx);
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
    }
    fn choose(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some(result) = &self.result else {
            return;
        };
        let count = result
            .categories
            .len()
            .saturating_sub(self.page * 120)
            .min(120);
        if count == 0 {
            return;
        }
        let b = self.chart_bounds.get();
        if b.size.width <= px(0.) {
            return;
        }
        let f = ((x - b.left()) / b.size.width).clamp(0., 0.9999);
        self.selected = ((f * count as f32) as usize).min(count - 1);
        cx.notify();
    }
}
fn chart_readout(result: &AskDataResult, index: usize) -> String {
    let mut text = result
        .categories
        .get(index)
        .map_or_else(String::new, ToString::to_string);
    for series in &result.series {
        if let Some(value) = series.values.get(index).filter(|v| v.is_finite()) {
            text.push_str(&format!(" · {}: {:.2}", series.name, value));
        }
    }
    text
}
impl Render for AskYourData {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let state = match self.status {
            AskDataStatus::Idle => t("Ready", "准备就绪"),
            AskDataStatus::WritingSql => t("Writing SQL", "正在生成查询"),
            AskDataStatus::RunningQuery => t("Running query", "正在查询"),
            AskDataStatus::Ready => t("Result ready", "结果已就绪"),
            AskDataStatus::Stopped => t("Stopped", "已停止"),
            AskDataStatus::Failed => t("Failed", "失败"),
        };
        let mut actions = div().flex().flex_wrap().gap(px(6.));
        if self.active.is_some() {
            actions = actions.child(
                face(
                    "ask-data-action",
                    t("Stop", "停止"),
                    36.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .track_focus(&self.action_focus)
                .px(px(10.))
                .child(t("Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        } else {
            let label = if matches!(self.status, AskDataStatus::Failed | AskDataStatus::Stopped) {
                t("Try again", "重试")
            } else {
                t("Ask", "查询")
            };
            actions = actions.child(
                face(
                    "ask-data-action",
                    label,
                    36.,
                    radius.into(),
                    Tone::Primary,
                    cx,
                )
                .track_focus(&self.action_focus)
                .px(px(10.))
                .child(label)
                .on_click(cx.listener(|s, _, w, cx| {
                    s.run(w, cx);
                })),
            );
        }
        let mut root = div()
            .id("ask-your-data")
            .role(Role::Group)
            .aria_label(t("Ask your data", "数据问答"))
            .w_full()
            .max_w(px(720.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(12.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .child(div().min_w(px(200.)).flex_1().child(self.question.clone()))
                    .child(actions),
            )
            .child(
                div()
                    .id("ask-data-status")
                    .role(Role::Status)
                    .px(px(16.))
                    .pb(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.faint)
                    .child(format!("{} · {}", self.source, state)),
            );
        if !self.sql.is_empty() {
            let button = face(
                "ask-data-sql",
                t("Generated SQL", "生成的查询"),
                36.,
                radius.into(),
                Tone::Surface,
                cx,
            )
            .w_full()
            .h(px(48.))
            .justify_between()
            .pl(px(16.))
            .pr(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .hover(|s| s.bg(p.tray.opacity(0.7)))
            .aria_expanded(self.sql_open)
            .child(t("Generated SQL", "生成的查询"))
            .child(icon(
                if self.sql_open {
                    Glyph::Up
                } else {
                    Glyph::Down
                },
                14.,
                p.secondary,
            ))
            .on_click(cx.listener(|s, _, _, cx| {
                s.sql_open = !s.sql_open;
                cx.notify();
            }));
            root = root.child(button);
            if self.sql_open {
                let label = match self.copied {
                    Some(true) => t("Copied", "已复制"),
                    Some(false) => t("Copy failed", "复制失败"),
                    None => t("Copy SQL", "复制查询"),
                };
                let scroll = self.sql_scroll.clone();
                let monitor = canvas(
                    |_, _, _| (),
                    move |bounds, _, _, cx| {
                        if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                            let left = -scroll.offset().x;
                            (host.0)(
                                bounds,
                                left > px(0.1),
                                left + px(0.1) < scroll.max_offset().x,
                            );
                        }
                    },
                )
                .absolute()
                .inset_0();
                let code = div()
                    .relative()
                    .w_full()
                    .child(
                        div()
                            .id("ask-data-sql-scroll")
                            .w_full()
                            .overflow_x_scroll()
                            .track_scroll(&self.sql_scroll)
                            .child(
                                div()
                                    .px(px(14.))
                                    .py(px(10.))
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .text_color(p.ink)
                                    .whitespace_nowrap()
                                    .child(self.sql.clone()),
                            ),
                    )
                    .child(monitor);
                root = root.child(
                    div()
                        .mx(px(12.))
                        .mb(px(12.))
                        .rounded(radius)
                        .bg(p.tray.opacity(0.6))
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(code)
                        .child(
                            face("ask-data-copy", label, 32., radius.into(), Tone::Clear, cx)
                                .px(px(8.))
                                .child(label)
                                .on_click(cx.listener(|s, _, _, cx| s.copy_sql(cx))),
                        ),
                );
            }
        }
        if self.status == AskDataStatus::RunningQuery {
            let count = if let Some(total) = self.total {
                format!("{} / {total}", self.scanned)
            } else {
                self.scanned.to_string()
            };
            root = root.child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(format!("{}: {count}", t("Rows scanned", "已扫描行数"))),
            );
        }
        if self.status == AskDataStatus::Failed {
            root = root.child(
                div()
                    .id("ask-data-error")
                    .role(Role::Alert)
                    .mx(px(16.))
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(radius)
                    .border_1()
                    .border_color(p.danger.opacity(0.28))
                    .bg(p.danger.opacity(0.05))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.danger)
                    .child(self.error.clone()),
            );
        }
        let Some(result) = self.result.clone() else {
            return root;
        };
        let choices = [
            (AskDataView::Bars, t("Bars", "柱状图")),
            (AskDataView::Lines, t("Lines", "折线图")),
            (AskDataView::Table, t("Table", "表格")),
        ];
        let mut switch = div().flex().flex_wrap().gap(px(4.));
        for (i, (view, label)) in choices.into_iter().enumerate() {
            switch = switch.child(
                face(
                    ("ask-data-view", i),
                    label,
                    32.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(12.))
                .text_size(px(12.))
                .line_height(px(16.))
                .aria_toggled((self.view == view).into())
                .when(self.view == view, |v| v.bg(p.tray).text_color(p.ink))
                .when(self.view != view, |v| {
                    v.text_color(p.secondary)
                        .hover(|s| s.bg(p.tray.opacity(0.7)).text_color(p.ink))
                })
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.view = view;
                    cx.notify();
                })),
            );
        }
        root = root.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .min_h(px(44.))
                .px(px(16.))
                .pt(px(16.))
                .border_t_1()
                .border_color(p.line)
                .gap(px(12.))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(result.title.clone()),
                )
                .child(switch),
        );
        let last_page = result.categories.len().saturating_sub(1) / 120;
        self.page = self.page.min(last_page);
        let offset = self.page * 120;
        let count = result.categories.len().saturating_sub(offset).min(120);
        self.selected = self.selected.min(count.saturating_sub(1));
        if self.view == AskDataView::Table {
            let mut table = div()
                .id("ask-data-table")
                .role(Role::Table)
                .aria_label(result.title.clone())
                .flex()
                .flex_col()
                .text_size(px(13.))
                .line_height(px(20.))
                .min_w(px((110. + result.series.len() as f32 * 100.).max(360.)));
            let mut header = div()
                .id("ask-data-head")
                .role(Role::Row)
                .flex()
                .text_color(p.faint)
                .border_b_1()
                .border_color(p.line)
                .child(
                    div()
                        .id("ask-data-category-head")
                        .role(Role::ColumnHeader)
                        .h(px(32.))
                        .px(px(12.))
                        .w(px(100.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(t("Category", "分类")),
                );
            for (j, series) in result.series.iter().enumerate() {
                header = header.child(
                    div()
                        .id(("ask-data-series-head", j))
                        .role(Role::ColumnHeader)
                        .h(px(32.))
                        .px(px(12.))
                        .w(px(90.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(series.name.clone()),
                );
            }
            table = table.child(header);
            for i in 0..count {
                let mut row = div()
                    .id(("ask-data-row", i))
                    .role(Role::Row)
                    .flex()
                    .border_b_1()
                    .border_color(p.line)
                    .bg(if self.hovered_row == Some(i) {
                        p.tray.opacity(0.6)
                    } else {
                        p.panel.opacity(0.)
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
                            .id(("ask-data-row-head", i))
                            .role(Role::RowHeader)
                            .h(px(32.))
                            .px(px(12.))
                            .w(px(100.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .text_color(p.secondary)
                            .child(result.categories[offset + i].clone()),
                    );
                for (j, series) in result.series.iter().enumerate() {
                    let value = series
                        .values
                        .get(offset + i)
                        .filter(|v| v.is_finite())
                        .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
                    row = row.child(
                        div()
                            .id((
                                "ask-data-cell",
                                i.saturating_mul(result.series.len()).saturating_add(j),
                            ))
                            .role(Role::Cell)
                            .h(px(32.))
                            .px(px(12.))
                            .w(px(90.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .child(value),
                    );
                }
                table = table.child(row);
            }
            let scroll = self.table_scroll.clone();
            let monitor = canvas(
                |_, _, _| (),
                move |bounds, _, _, cx| {
                    if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                        let left = -scroll.offset().x;
                        (host.0)(
                            bounds,
                            left > px(0.1),
                            left + px(0.1) < scroll.max_offset().x,
                        );
                    }
                },
            )
            .absolute()
            .inset_0();
            root = root.child(
                div()
                    .relative()
                    .w_full()
                    .child(
                        div()
                            .id("ask-data-table-scroll")
                            .w_full()
                            .overflow_x_scroll()
                            .track_scroll(&self.table_scroll)
                            .child(table),
                    )
                    .child(monitor),
            );
        } else {
            let age = if disabled_motion(cx) {
                f32::INFINITY
            } else {
                let age = self.plot.age(cx);
                let end = if self.view == AskDataView::Lines {
                    0.46 + 0.04 * result.series.len().saturating_sub(1) as f32
                } else {
                    let n = count.max(1) as f32;
                    let step = 0.016_f32.min(0.18 / n);
                    0.32 + step * count.saturating_sub(1) as f32
                };
                if age < end {
                    w.request_animation_frame();
                }
                age
            };
            let graph = result.clone();
            let kind = self.view;
            let bounds = self.chart_bounds.clone();
            let colors = [p.focus, p.success, p.secondary, p.danger];
            let plot = canvas(
                move |b, _, _| bounds.set(b),
                move |b, _, w, _| {
                    let count = graph.categories.len().saturating_sub(offset).min(120);
                    if count == 0 {
                        return;
                    }
                    let width: f32 = b.size.width.into();
                    let height: f32 = b.size.height.into();
                    let mut low = 0f64;
                    let mut high = 0f64;
                    for i in 0..count {
                        if kind == AskDataView::Bars {
                            let mut pos = 0.;
                            let mut neg = 0.;
                            for s in &graph.series {
                                let v = s
                                    .values
                                    .get(offset + i)
                                    .copied()
                                    .filter(|v| v.is_finite())
                                    .unwrap_or(0.);
                                if v >= 0. {
                                    pos += v;
                                } else {
                                    neg += v;
                                }
                            }
                            low = low.min(neg);
                            high = high.max(pos);
                        } else {
                            for s in &graph.series {
                                if let Some(v) = s.values.get(offset + i).filter(|v| v.is_finite())
                                {
                                    low = low.min(*v);
                                    high = high.max(*v);
                                }
                            }
                        }
                    }
                    if !(high - low).is_finite() {
                        return;
                    }
                    if (high - low).abs() < f64::EPSILON {
                        high = 1.;
                        low = 0.;
                    }
                    let y = |v: f64| b.bottom() - px(((v - low) / (high - low)) as f32 * height);
                    let pitch = width / count as f32;
                    if kind == AskDataView::Bars {
                        for i in 0..count {
                            let mut pos = 0.;
                            let mut neg = 0.;
                            let step = 0.016_f32.min(0.18 / count.max(1) as f32);
                            let grow = standard_ease(((age - i as f32 * step) / 0.32).clamp(0., 1.));
                            for (j, s) in graph.series.iter().enumerate() {
                                let v = s
                                    .values
                                    .get(offset + i)
                                    .copied()
                                    .filter(|v| v.is_finite())
                                    .unwrap_or(0.)
                                    * f64::from(grow);
                                let (a, z) = if v >= 0. {
                                    let a = pos;
                                    pos += v;
                                    (a, pos)
                                } else {
                                    let a = neg;
                                    neg += v;
                                    (a, neg)
                                };
                                let ya = y(a);
                                let yz = y(z);
                                let h = (ya - yz).abs();
                                if h > px(0.1) {
                                    w.paint_quad(fill(
                                        Bounds::new(
                                            point(
                                                b.left() + px(i as f32 * pitch + pitch * 0.16),
                                                ya.min(yz),
                                            ),
                                            size(px((pitch * 0.68).max(1.)), h),
                                        ),
                                        colors[j % colors.len()],
                                    ));
                                }
                            }
                        }
                    } else {
                        for (j, s) in graph.series.iter().enumerate() {
                            let mut path = PathBuilder::stroke(px(2.));
                            let mut started = false;
                            let drawn =
                                standard_ease(((age - 0.04 * j as f32) / 0.46).clamp(0., 1.));
                            for i in 0..count {
                                if i as f32 > (count.saturating_sub(1) as f32) * drawn {
                                    break;
                                }
                                if let Some(v) = s.values.get(offset + i).filter(|v| v.is_finite())
                                {
                                    let pt = point(b.left() + px((i as f32 + 0.5) * pitch), y(*v));
                                    if started {
                                        path.line_to(pt);
                                    } else {
                                        path.move_to(pt);
                                        started = true;
                                    }
                                } else {
                                    started = false;
                                }
                            }
                            if let Ok(path) = path.build() {
                                w.paint_path(path, colors[j % colors.len()]);
                            }
                        }
                    }
                },
            )
            .size_full();
            let readout = if count == 0 {
                t("No data", "暂无数据").to_owned()
            } else {
                chart_readout(&result, offset + self.selected)
            };
            root = root
                .child(
                    div()
                        .id("ask-data-chart")
                        .role(Role::Group)
                        .aria_label(result.title.clone())
                        .aria_value(readout.clone())
                        .track_focus(&self.chart_focus)
                        .tab_index(0)
                        .w_full()
                        .h(px(220.))
                        .relative()
                        .child(plot)
                        .child(self.contact.listen(cx, |s, e, w, cx| {
                            if matches!(e.phase, TouchPhase::Started | TouchPhase::Moved) {
                                if e.phase == TouchPhase::Started {
                                    s.chart_focus.focus(w, cx);
                                }
                                s.choose(e.position.x, cx);
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|s, e: &MouseDownEvent, w, cx| {
                                s.chart_focus.focus(w, cx);
                                s.choose(e.position.x, cx);
                            }),
                        )
                        .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                            if e.pressed_button == Some(MouseButton::Left) {
                                s.choose(e.position.x, cx);
                            }
                        }))
                        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                            let count = s.result.as_ref().map_or(0, |r| {
                                r.categories.len().saturating_sub(s.page * 120).min(120)
                            });
                            if count == 0 {
                                return;
                            }
                            s.selected = match e.keystroke.key.as_str() {
                                "left" => s.selected.saturating_sub(1),
                                "right" => (s.selected + 1).min(count - 1),
                                "home" => 0,
                                "end" => count - 1,
                                _ => return,
                            };
                            cx.notify();
                            w.prevent_default();
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .id("ask-data-readout")
                        .role(Role::Status)
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(readout),
                );
            let palette = [p.focus, p.success, p.secondary, p.danger];
            let mut legend = div().px(px(16.)).flex().flex_wrap().gap(px(16.));
            for (i, series) in result.series.iter().enumerate() {
                legend = legend.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .size(px(8.))
                                .rounded(px(2.5))
                                .bg(palette[i % palette.len()]),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.secondary)
                                .child(series.name.clone()),
                        ),
                );
            }
            root = root.child(legend);
        }
        if last_page > 0 {
            let mut pages = div().flex().flex_wrap().items_center().gap(px(6.)).child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(format!(
                        "{} {}–{} / {}",
                        t("Showing", "显示"),
                        offset + 1,
                        offset + count,
                        result.categories.len()
                    )),
            );
            if self.page > 0 {
                pages = pages.child(
                    face(
                        "ask-data-previous",
                        t("Previous", "上一页"),
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(t("Previous", "上一页"))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.page = s.page.saturating_sub(1);
                        s.selected = 0;
                        cx.notify();
                    })),
                );
            }
            if self.page < last_page {
                pages = pages.child(
                    face(
                        "ask-data-next",
                        t("Next", "下一页"),
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(t("Next", "下一页"))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.page = (s.page + 1).min(last_page);
                        s.selected = 0;
                        cx.notify();
                    })),
                );
            }
            root = root.child(pages);
        }
        if !result.insight.is_empty() {
            root = root.child(
                div()
                    .min_h(px(40.))
                    .px(px(16.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .child(result.insight),
            );
        }
        root.child(
            div()
                .px(px(16.))
                .pb(px(16.))
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.faint)
                .child(format!("{} · {} ms", result.row_count, result.duration_ms)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{AskDataResult, AskDataSeries, AskDataStatus, AskYourData};
    use gpui::{EntityInputHandler, TestAppContext};
    #[test]
    fn stages_have_distinct_ids_and_cancelled_results_are_rejected() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| AskYourData::new("Analytics", w, cx));
        w.update(|w, cx| {
            control
                .read(cx)
                .question()
                .update(cx, |field, cx| field.set_value("  Signups?  ", w, cx));
            control.update(cx, |view, cx| {
                assert!(view.run(w, cx));
            });
        });
        let generation = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|_, cx| {
            control.update(cx, |view, cx| {
                assert!(view.append_sql(generation, "SELECT 1", cx));
                assert!(view.finish_sql(generation, cx));
                let query = view.active_request().unwrap();
                assert_ne!(generation, query);
                assert!(!view.append_sql(generation, " late", cx));
                assert!(view.progress(query, 1, Some(1), cx));
                assert!(view.finish_query(
                    query,
                    AskDataResult::new(
                        "Count",
                        vec!["Today".into()],
                        vec![AskDataSeries::new("Users", vec![1.])]
                    ),
                    cx
                ));
                assert_eq!(view.status(), AskDataStatus::Ready);
                assert!(!view.fail(query, "late", cx));
            })
        });
        w.update(|w, cx| {
            control.update(cx, |view, cx| {
                assert!(view.run(w, cx));
            })
        });
        let id = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|w, cx| {
            control
                .read(cx)
                .question()
                .update(cx, |field, cx| field.set_value("Changed question", w, cx))
        });
        w.update(|_, cx| {
            control.update(cx, |view, cx| {
                assert_eq!(view.status(), AskDataStatus::Stopped);
                assert!(!view.append_sql(id, "stale", cx));
            })
        });
    }
    #[test]
    fn editing_a_completed_question_clears_its_sql_and_result() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| AskYourData::new("Analytics", w, cx));
        w.update(|w, cx| {
            control
                .read(cx)
                .question()
                .update(cx, |field, cx| field.set_value("First", w, cx))
        });
        w.update(|w, cx| control.update(cx, |view, cx| assert!(view.run(w, cx))));
        let generation = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|_, cx| {
            control.update(cx, |view, cx| {
                assert!(view.append_sql(generation, "SELECT 1", cx));
                assert!(view.finish_sql(generation, cx));
                let query = view.active_request().unwrap();
                assert!(view.finish_query(
                    query,
                    AskDataResult::new("Old", vec!["A".into()], vec![]),
                    cx
                ));
                assert!(view.result.is_some());
            })
        });
        w.update(|w, cx| {
            control
                .read(cx)
                .question()
                .update(cx, |field, cx| field.set_value("Second", w, cx))
        });
        w.update(|_, cx| {
            control.update(cx, |view, cx| {
                assert_eq!(view.status(), AskDataStatus::Idle);
                assert!(view.sql().is_empty());
                assert!(view.result.is_none());
                assert!(!view.append_sql(generation, "late", cx));
            })
        });
    }
    #[test]
    fn composing_question_cannot_start_a_query() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| AskYourData::new("Analytics", w, cx));
        let editor = w.update(|_, cx| control.read(cx).question().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|w, cx| {
            control.update(cx, |view, cx| {
                assert!(!view.run(w, cx));
                assert_eq!(view.active_request(), None);
                assert_eq!(editor.read(cx).value(), "ni");
            })
        });
    }
}
