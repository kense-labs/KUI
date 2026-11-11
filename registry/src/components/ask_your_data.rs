//! A host-driven question-to-query workflow with inspectable local results.
use crate::{
    components::input::{Input, InputStyle},
    primitives::{
        brand_mark::brand_mark,
        clipboard,
        control::UiLanguage,
        direct_touch::Contact,
        editor::{EditEvent, EditorScrollRegion},
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, standard_ease},
        field::{Field, FieldEvent},
        glyph::{Glyph, icon, spinner},
        nav::HorizontalScrollRegion,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, ops::Range, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;

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
#[derive(Clone, Copy, Default)]
pub enum AskDataLegend {
    #[default]
    Auto,
    Sum,
    Last,
    None,
}

/// Optional labels and display formatting supplied with a result.
#[derive(Clone, Default)]
pub struct AskDataPresentation {
    pub subtitle: Option<SharedString>,
    pub category_label: Option<SharedString>,
    pub view: Option<AskDataView>,
    pub legend: AskDataLegend,
    pub format_value: Option<fn(f64) -> String>,
}

impl AskDataPresentation {
    fn number(&self, value: f64) -> String {
        self.format_value.unwrap_or(number)(value)
    }
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
    dialect: Option<SharedString>,
    suggestions: Vec<SharedString>,
    run_question: SharedString,
    sql: String,
    sql_chunks: Vec<(Range<usize>, PhaseClock)>,
    status: AskDataStatus,
    active: Option<u64>,
    sequence: u64,
    scanned: usize,
    scanned_roll: Slide,
    total: Option<usize>,
    result: Option<Rc<AskDataResult>>,
    presentation: AskDataPresentation,
    error: SharedString,
    sql_open: bool,
    sql_preference: Option<bool>,
    keep_sql_open: bool,
    sql_reveal: Slide,
    reserved_sql_lines: usize,
    view: AskDataView,
    shown_view: AskDataView,
    view_fade: PhaseClock,
    view_fade_from: f32,
    view_focus: [FocusHandle; 3],
    stacked: bool,
    chart_focus: FocusHandle,
    chart_bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    selected: usize,
    inspecting: bool,
    page: usize,
    hovered_row: Option<usize>,
    plot: PhaseClock,
    busy_clock: PhaseClock,
    table_scroll: ScrollHandle,
    sql_scroll: ScrollHandle,
    copied: Option<bool>,
    copy_epoch: u64,
    action_focus: FocusHandle,
    _sub: Subscription,
    _escape: Subscription,
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
            .with_placeholder(
                lang.text("Ask a question about your data", "输入想查询的问题"),
                cx,
            )
        });
        let editor = question.read(cx).editor();
        let escape = cx.subscribe(&editor, |s: &mut Self, _, e: &EditEvent, cx| {
            if matches!(e, EditEvent::Escape) {
                s.stop(cx);
            }
        });
        let sub = cx.subscribe_in(
            &question,
            w,
            |s: &mut Self, _, event: &FieldEvent, w, cx| match event {
                FieldEvent::Submitted => {
                    if s.active.is_some() {
                        s.stop(cx);
                    } else {
                        s.run(w, cx);
                    }
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
            dialect: None,
            suggestions: Vec::new(),
            run_question: "".into(),
            sql: String::new(),
            sql_chunks: Vec::new(),
            status: AskDataStatus::Idle,
            active: None,
            sequence: 0,
            scanned: 0,
            scanned_roll: Slide::new(0.),
            total: None,
            result: None,
            presentation: AskDataPresentation::default(),
            error: "".into(),
            sql_open: false,
            sql_preference: None,
            keep_sql_open: false,
            sql_reveal: Slide::new(0.),
            reserved_sql_lines: 0,
            view: AskDataView::Bars,
            shown_view: AskDataView::Bars,
            view_fade: PhaseClock::default(),
            view_fade_from: 1.,
            view_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            stacked: true,
            chart_focus: cx.focus_handle().tab_stop(true),
            chart_bounds: Default::default(),
            contact: Default::default(),
            selected: 0,
            inspecting: false,
            page: 0,
            hovered_row: None,
            plot: PhaseClock::default(),
            busy_clock: PhaseClock::default(),
            table_scroll: ScrollHandle::new(),
            sql_scroll: ScrollHandle::new(),
            copied: None,
            copy_epoch: 0,
            action_focus: cx.focus_handle().tab_stop(true),
            _sub: sub,
            _escape: escape,
        }
    }
    pub fn question(&self) -> Entity<Input> {
        self.question.clone()
    }
    /// Dialect name and its mark in the generated query header, when available.
    pub fn with_dialect(mut self, dialect: impl Into<SharedString>) -> Self {
        self.dialect = Some(dialect.into());
        self
    }
    /// Optional prompts offered before the first query and after stopping or failing.
    pub fn with_suggestions(mut self, suggestions: Vec<SharedString>) -> Self {
        self.suggestions = suggestions;
        self
    }
    /// Keeps the SQL visible when execution starts unless the person has chosen otherwise.
    pub fn with_keep_sql_open(mut self, keep_open: bool) -> Self {
        self.keep_sql_open = keep_open;
        self
    }
    /// Show grouped columns instead of stacked totals for rates or averages.
    pub fn set_stacked(&mut self, stacked: bool, cx: &mut Context<Self>) {
        if self.stacked != stacked {
            self.stacked = stacked;
            cx.notify();
        }
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
        self.sql_chunks.clear();
        self.result = None;
        self.error = "".into();
        self.scanned = 0;
        self.scanned_roll.reset(0.);
        self.total = None;
        self.sql_open = false;
        self.sql_reveal.reset(0.);
        self.reserved_sql_lines = 0;
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
            self.status = AskDataStatus::Failed;
            self.error = text.into();
            cx.notify();
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
        self.sql_chunks.clear();
        self.status = AskDataStatus::WritingSql;
        self.busy_clock.restart(cx);
        self.error = "".into();
        self.scanned = 0;
        self.scanned_roll.reset(0.);
        self.total = None;
        self.result = None;
        self.reserved_sql_lines = 0;
        self.set_sql_open(self.sql_preference.unwrap_or(true), cx);
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.selected = 0;
        self.inspecting = false;
        self.page = 0;
        self.plot = PhaseClock::default();
        cx.emit(AskDataEvent::GenerateSql { id, question });
        cx.notify();
        true
    }
    /// Reserves the height of a known complete SQL response before streaming it.
    pub fn reserve_sql_lines(&mut self, id: u64, lines: usize, cx: &mut Context<Self>) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::WritingSql {
            return false;
        }
        self.reserved_sql_lines = lines;
        cx.notify();
        true
    }
    pub fn append_sql(&mut self, id: u64, chunk: &str, cx: &mut Context<Self>) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::WritingSql {
            return false;
        }
        let start = self.sql.len();
        self.sql.push_str(chunk);
        if !chunk.is_empty() && !disabled_motion(cx) {
            self.sql_chunks.retain(|(_, clock)| clock.age(cx) < 0.14);
            let mut clock = PhaseClock::default();
            clock.restart(cx);
            self.sql_chunks.push((start..self.sql.len(), clock));
        }
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
        self.set_sql_open(self.sql_preference.unwrap_or(self.keep_sql_open), cx);
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
        self.scanned_roll.duration(scanned as f32, 0.16, cx);
        self.total = total;
        cx.notify();
        true
    }
    pub fn finish_query(&mut self, id: u64, result: AskDataResult, cx: &mut Context<Self>) -> bool {
        self.finish_query_presented(id, result, AskDataPresentation::default(), cx)
    }

    pub fn finish_query_presented(
        &mut self,
        id: u64,
        result: AskDataResult,
        presentation: AskDataPresentation,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) || self.status != AskDataStatus::RunningQuery {
            return false;
        }
        self.active = None;
        if let Some(view) = presentation.view {
            self.view = view;
        }
        self.shown_view = self.view;
        self.view_fade = PhaseClock::default();
        self.view_fade_from = 1.;
        self.presentation = presentation;
        self.result = Some(Rc::new(result));
        self.status = AskDataStatus::Ready;
        self.selected = 0;
        self.page = 0;
        self.plot.restart(cx);
        cx.notify();
        true
    }
    fn set_sql_open(&mut self, open: bool, cx: &App) {
        self.sql_open = open;
        self.sql_reveal
            .duration(if open { 1. } else { 0. }, 0.30, cx);
    }
    fn toggle_sql(&mut self, cx: &mut Context<Self>) {
        if !self.sql.is_empty() {
            self.set_sql_open(!self.sql_open, cx);
            self.sql_preference = Some(self.sql_open);
            cx.notify();
        }
    }
    fn select_view(&mut self, view: AskDataView, cx: &mut Context<Self>) {
        if self.view != view {
            self.view_fade_from = self.view_opacity(self.view_fade.age(cx));
            self.view = view;
            if disabled_motion(cx) {
                self.shown_view = view;
            }
            self.view_fade.restart(cx);
            self.inspecting = false;
            cx.notify();
        }
    }
    fn view_opacity(&self, elapsed: f32) -> f32 {
        if self.shown_view != self.view {
            self.view_fade_from * (1. - (elapsed / 0.10).clamp(0., 1.))
        } else {
            let visible = (elapsed / 0.16).clamp(0., 1.);
            self.view_fade_from + (1. - self.view_fade_from) * visible
        }
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
        let f = ((x - b.left() - px(34.)) / (b.size.width - px(34.)).max(px(1.))).clamp(0., 0.9999);
        let next = ((f * count as f32) as usize).min(count - 1);
        if next != self.selected || !self.inspecting {
            self.selected = next;
            self.inspecting = true;
            cx.notify();
        }
    }
}
fn total_at(result: &AskDataResult, index: usize) -> f64 {
    result
        .series
        .iter()
        .filter_map(|series| series.values.get(index).filter(|v| v.is_finite()))
        .sum()
}

fn chart_readout(
    result: &AskDataResult,
    index: usize,
    presentation: &AskDataPresentation,
    total_label: Option<&str>,
) -> String {
    let mut text = result
        .categories
        .get(index)
        .map_or_else(String::new, ToString::to_string);
    for series in &result.series {
        if let Some(value) = series.values.get(index).filter(|v| v.is_finite()) {
            text.push_str(&format!(
                " · {}: {}",
                series.name,
                presentation.number(*value)
            ));
        }
    }
    if let Some(label) = total_label {
        text.push_str(&format!(
            " · {label}: {}",
            presentation.number(total_at(result, index))
        ));
    }
    text
}

fn number(value: f64) -> String {
    if (value.round() - value).abs() < 0.005 {
        let rounded = format!("{value:.0}");
        let (sign, digits) = rounded
            .strip_prefix('-')
            .map_or(("", rounded.as_str()), |v| ("-", v));
        let mut text = String::from(sign);
        for (index, digit) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                text.push(',');
            }
            text.push(digit);
        }
        text
    } else {
        format!("{value:.2}")
    }
}

fn sparkle(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let path = |points: &[(f32, f32)]| {
                let mut stroke = PathBuilder::stroke(px(1.1));
                stroke.move_to(b.origin + point(px(points[0].0), px(points[0].1)));
                for &(x, y) in &points[1..] {
                    stroke.line_to(b.origin + point(px(x), px(y)));
                }
                stroke.build()
            };
            for points in [
                &[
                    (5., 3.),
                    (6.1, 6.),
                    (9., 7.2),
                    (6.1, 8.4),
                    (5., 11.5),
                    (3.8, 8.4),
                    (1., 7.2),
                    (3.8, 6.),
                    (5., 3.),
                ][..],
                &[(10.9, 1.5), (10.9, 5.8)][..],
                &[(8.8, 3.6), (13., 3.6)][..],
            ] {
                if let Ok(path) = path(points) {
                    w.paint_path(path, ink);
                }
            }
        },
    )
    .size(px(14.))
}

fn insight_text(text: &SharedString, elapsed: f32) -> StyledText {
    let visible_words = ((elapsed / 0.034).max(0.) as usize).saturating_mul(4);
    let mut words = text
        .split_word_bound_indices()
        .filter(|(_, word)| !word.trim().is_empty());
    let hidden = words.nth(visible_words).map(|(start, _)| start);
    StyledText::new(text.clone()).with_highlights(hidden.into_iter().map(|start| {
        (
            start..text.len(),
            HighlightStyle {
                fade_out: Some(1.),
                ..Default::default()
            },
        )
    }))
}

fn scroll_monitor(scroll: ScrollHandle) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, _, cx| {
            let offset = scroll.offset();
            let max = scroll.max_offset();
            if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                (host.0)(bounds, -offset.x > px(0.1), -offset.x + px(0.1) < max.x);
            }
            if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                (host.0)(bounds, -offset.y > px(0.1), -offset.y + px(0.1) < max.y);
            }
        },
    )
    .absolute()
    .inset_0()
}

fn database_mark(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let c = bounds.center();
            let mut path = PathBuilder::stroke(px(1.2));
            let oval = |path: &mut PathBuilder, y: f32, start: usize, end: usize| {
                for i in start..=end {
                    let angle = i as f32 * std::f32::consts::TAU / 24.;
                    let point = c + point(px(5.5 * angle.cos()), px(y + 2.1 * angle.sin()));
                    if i == start {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
            };
            oval(&mut path, -4., 0, 24);
            for y in [0., 4.] {
                oval(&mut path, y, 0, 12);
            }
            for x in [-5.5, 5.5] {
                path.move_to(c + point(px(x), px(-4.)));
                path.line_to(c + point(px(x), px(4.)));
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(16.))
    .flex_none()
}

fn sql_text(line: &str, p: Paint, fades: &[(Range<usize>, f32)]) -> StyledText {
    let mut highlights = Vec::new();
    let dark = p.ink.to_rgb().r > 0.5;
    let keyword = rgb(if dark { 0xcb9cf0 } else { 0x7950a4 });
    let literal = rgb(if dark { 0x86d6b9 } else { 0x237b65 });
    let mut words = line.char_indices().peekable();
    while let Some((start, ch)) = words.next() {
        let (end, color) = if line[start..].starts_with("--") {
            highlights.push((
                start..line.len(),
                HighlightStyle {
                    color: Some(p.faint),
                    ..Default::default()
                },
            ));
            break;
        } else if ch == '\'' {
            let mut end = line.len();
            for (i, ch) in words.by_ref() {
                if ch == '\'' {
                    end = i + 1;
                    break;
                }
            }
            (end, Some(literal))
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            while words
                .peek()
                .is_some_and(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_')
            {
                words.next();
            }
            let end = words.peek().map_or(line.len(), |(i, _)| *i);
            let kind = line[start..end].to_ascii_uppercase();
            let color = match kind.as_str() {
                "SELECT" | "AS" | "FROM" | "WHERE" | "AND" | "BY" | "GROUP" | "ORDER" | "JOIN"
                | "ON" | "DATE" => Some(keyword),
                "COUNT" | "SUM" | "AVG" | "DATE_TRUNC" => {
                    Some(rgb(if dark { 0x88bafa } else { 0x387ba9 }))
                }
                _ if ch.is_ascii_digit() => Some(literal),
                _ => None,
            };
            (end, color)
        } else {
            continue;
        };
        if let Some(color) = color {
            highlights.push((
                start..end,
                HighlightStyle {
                    color: Some(color.into()),
                    ..Default::default()
                },
            ));
        }
    }
    if fades.is_empty() {
        return StyledText::new(SharedString::from(line.to_owned())).with_highlights(highlights);
    }
    let mut boundaries = vec![0, line.len()];
    for range in highlights
        .iter()
        .map(|(range, _)| range)
        .chain(fades.iter().map(|(range, _)| range))
    {
        boundaries.extend([range.start, range.end]);
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    let runs = boundaries.windows(2).map(|pair| {
        let start = pair[0];
        let mut style = highlights
            .iter()
            .find(|(range, _)| range.contains(&start))
            .map_or(HighlightStyle::default(), |(_, style)| *style);
        style.fade_out = fades
            .iter()
            .find(|(range, _)| range.contains(&start))
            .map(|(_, fade)| *fade);
        (start..pair[1], style)
    });
    StyledText::new(SharedString::from(line.to_owned())).with_highlights(runs)
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
        let busy = self.active.is_some();
        let reduced = disabled_motion(cx);
        let busy_age = if busy && !reduced {
            w.request_animation_frame();
            self.busy_clock.age(cx)
        } else {
            0.
        };
        let scanned = if reduced {
            self.scanned
        } else {
            self.scanned_roll.value(cx).round() as usize
        };
        let live = self.result.as_ref().map_or_else(
            || format!("{} · {}", self.source, state),
            |result| {
                format!(
                    "{} · {} {} · {}ms. {}",
                    result.title,
                    number(result.row_count as f64),
                    t("rows", "行"),
                    result.duration_ms,
                    result.insight
                )
            },
        );
        let action_label = if busy {
            t("Stop", "停止")
        } else {
            t("Ask", "查询")
        };
        let action = face(
            "ask-data-action",
            action_label,
            32.,
            radius.into(),
            Tone::Primary,
            cx,
        )
        .track_focus(&self.action_focus)
        .size(px(32.))
        .child(if busy {
            div()
                .size(px(10.))
                .rounded(px(2.))
                .bg(p.inverse)
                .into_any_element()
        } else {
            icon(Glyph::ArrowUp, 16., p.inverse).into_any_element()
        })
        .on_click(cx.listener(|s, _, w, cx| {
            if s.active.is_some() {
                s.stop(cx);
            } else {
                s.run(w, cx);
            }
        }));
        let editor = self.question.read(cx).editor();
        let focused = self.question.read(cx).focus_handle(cx).is_focused(w);
        let mut root = div()
            .id("ask-your-data")
            .role(Role::Group)
            .aria_label(t("Ask your data", "数据问答"))
            .w_full()
            .max_w(px(522.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .flex()
            .flex_col()
            .child(
                div().p(px(12.)).child(
                    div()
                        .h(px(44.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .pl(px(12.))
                        .pr(px(5.))
                        .rounded(radius)
                        .border_1()
                        .border_color(if focused { p.focus } else { p.line })
                        .child(database_mark(p.secondary))
                        .child(div().h(px(20.)).w(px(1.)).flex_none().bg(p.line))
                        .child(div().h_full().min_w_0().flex_1().child(editor))
                        .child(action),
                ),
            )
            .child(
                div()
                    .id("ask-data-status")
                    .role(Role::Status)
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .child(live),
            );
        let detail = self.result.as_ref().map_or_else(
            || self.source.to_string(),
            |r| format!("{} {}", number(r.row_count as f64), t("rows", "行")),
        );
        let timing = self.result.as_ref().map(|r| {
            if r.duration_ms < 1000 {
                format!("{}ms", r.duration_ms)
            } else {
                format!("{:.1}s", r.duration_ms as f64 / 1000.)
            }
        });
        let query_label = format!(
            "{}, {} · {detail}{}",
            t("Generated SQL", "生成的查询"),
            self.dialect.as_deref().unwrap_or(self.source.as_ref()),
            timing.map_or_else(String::new, |ms| format!(" · {ms}")),
        );
        let query = face(
            "ask-data-sql",
            query_label,
            36.,
            radius.into(),
            Tone::Clear,
            cx,
        )
        .flex_1()
        .min_w_0()
        .gap(px(9.))
        .justify_start()
        .when_some(self.dialect.as_ref(), |row, dialect| {
            row.children(brand_mark(dialect, 16., cx))
        })
        .child(
            div()
                .flex_none()
                .font_weight(FontWeight::MEDIUM)
                .child(t("Generated SQL", "生成的查询")),
        )
        .child(div().min_w_0().truncate().text_color(p.faint).child(detail))
        .aria_expanded(self.sql_open && !self.sql.is_empty())
        .tab_stop(!self.sql.is_empty())
        .on_click(cx.listener(|s, _, _, cx| s.toggle_sql(cx)));
        let copy_label = match self.copied {
            Some(true) => t("Copied", "已复制"),
            Some(false) => t("Copy failed", "复制失败"),
            None => t("Copy SQL", "复制查询"),
        };
        let copy = face(
            "ask-data-copy",
            copy_label,
            30.,
            radius.into(),
            Tone::Clear,
            cx,
        )
        .size(px(30.))
        .tab_stop(!self.sql.is_empty())
        .opacity(if self.sql.is_empty() { 0.4 } else { 1. })
        .child(icon(
            match self.copied {
                Some(true) => Glyph::Check,
                Some(false) => Glyph::Alert,
                None => Glyph::Copy,
            },
            16.,
            p.secondary,
        ))
        .on_click(cx.listener(|s, _, _, cx| s.copy_sql(cx)));
        let chevron = face(
            "ask-data-sql-chevron",
            t("Toggle SQL", "展开或收起查询"),
            28.,
            radius.into(),
            Tone::Clear,
            cx,
        )
        .size(px(28.))
        .tab_stop(false)
        .child(icon(
            if self.sql_open && !self.sql.is_empty() {
                Glyph::Up
            } else {
                Glyph::Down
            },
            14.,
            p.secondary,
        ))
        .on_click(cx.listener(|s, _, _, cx| s.toggle_sql(cx)));
        root = root.child(
            div()
                .flex()
                .items_center()
                .gap(px(3.))
                .px(px(12.))
                .h(px(48.))
                .border_t_1()
                .border_color(p.line)
                .child(query)
                .when(self.status != AskDataStatus::Idle, |row| {
                    let (glyph, color) = match self.status {
                        AskDataStatus::Ready => (Glyph::Check, p.success),
                        AskDataStatus::Stopped => (Glyph::Pause, p.secondary),
                        AskDataStatus::Failed => (Glyph::Alert, p.danger),
                        _ => (Glyph::Spinner, p.faint),
                    };
                    if busy && !reduced {
                        row.child(spinner(busy_age, 14., color))
                    } else {
                        row.child(icon(if busy { Glyph::Clock } else { glyph }, 14., color))
                    }
                })
                .child(copy)
                .child(chevron),
        );
        let sql_fraction = self.sql_reveal.get(w, cx).clamp(0., 1.);
        if sql_fraction > 0.001 {
            let monitor = scroll_monitor(self.sql_scroll.clone());
            let fades = if reduced {
                Vec::new()
            } else {
                self.sql_chunks.retain(|(_, clock)| clock.age(cx) < 0.14);
                if !self.sql_chunks.is_empty() && !busy {
                    w.request_animation_frame();
                }
                self.sql_chunks
                    .iter()
                    .map(|(range, clock)| (range, (1. - clock.age(cx) / 0.14).clamp(0., 1.)))
                    .collect::<Vec<_>>()
            };
            let lines = self
                .sql
                .split('\n')
                .scan(0, |start, line| {
                    let origin = *start;
                    *start += line.len() + 1;
                    Some((line, origin))
                })
                .enumerate()
                .map(|(i, (line, origin))| {
                    let line_fades = fades
                        .iter()
                        .filter_map(|(range, fade)| {
                            let start = range.start.max(origin);
                            let end = range.end.min(origin + line.len());
                            if start < end {
                                Some((start - origin..end - origin, *fade))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .h(px(19.))
                        .flex_none()
                        .child(
                            div()
                                .w(px(22.))
                                .flex_none()
                                .text_right()
                                .text_color(p.faint)
                                .child((i + 1).to_string()),
                        )
                        .child(div().flex_none().child(sql_text(line, p, &line_fades)))
                })
                .collect::<Vec<_>>();
            let line_count = self.sql.split('\n').count().max(self.reserved_sql_lines);
            let height = (22. + 19. * line_count as f32).min(288.);
            root = root.child(
                div()
                    .h(px((height + 12.) * sql_fraction))
                    .overflow_hidden()
                    .child(
                        div()
                            .relative()
                            .mx(px(12.))
                            .h(px(height))
                            .overflow_hidden()
                            .rounded(radius)
                            .bg(p.stage)
                            .border_1()
                            .border_color(p.line)
                            .child(
                                div()
                                    .id("ask-data-sql-scroll")
                                    .w_full()
                                    .h_full()
                                    .overflow_x_scroll()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.sql_scroll)
                                    .child(
                                        div()
                                            .px(px(9.))
                                            .py(px(10.))
                                            .text_size(px(12.))
                                            .line_height(px(19.))
                                            .font_family("Fira Code")
                                            .text_color(p.ink)
                                            .whitespace_nowrap()
                                            .children(lines),
                                    ),
                            )
                            .child(monitor),
                    ),
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
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .justify_between()
                    .gap(px(8.))
                    .child(self.error.clone())
                    .child(
                        face(
                            "ask-data-retry",
                            t("Try again", "重试"),
                            28.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(8.))
                        .child(t("Try again", "重试"))
                        .on_click(cx.listener(|s, _, w, cx| {
                            if s.run(w, cx) {
                                s.action_focus.focus(w, cx);
                            }
                        })),
                    ),
            );
        }
        let Some(result) = self.result.clone() else {
            if self.active.is_some() {
                root = root.child(
                    div()
                        .id("ask-data-chart-loading")
                        .aria_label(t("Chart pending", "等待图表"))
                        .border_t_1()
                        .border_color(p.line)
                        .px(px(16.))
                        .pt(px(14.))
                        .pb(px(24.))
                        .flex()
                        .flex_col()
                        .gap(px(15.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(if reduced {
                                    icon(Glyph::Clock, 15., p.secondary).into_any_element()
                                } else {
                                    spinner(busy_age, 15., p.secondary).into_any_element()
                                })
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div().text_size(px(14.)).text_color(p.ink).child(state),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.faint)
                                                .font_features(FontFeatures(
                                                    vec![("tnum".into(), 1)].into(),
                                                ))
                                                .child(
                                                    if self.status == AskDataStatus::WritingSql {
                                                        t(
                                                            "Reading the schema",
                                                            "正在读取数据库结构",
                                                        )
                                                        .to_owned()
                                                    } else {
                                                        match self.total {
                                                            Some(total) => format!(
                                                                "{} / {total} {}",
                                                                number(scanned as f64),
                                                                t("rows", "行")
                                                            ),
                                                            None => format!(
                                                                "{} {}",
                                                                number(scanned as f64),
                                                                t("rows scanned", "行已扫描")
                                                            ),
                                                        }
                                                    },
                                                ),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .h(px(216.))
                                .pt(px(22.))
                                .pl(px(26.))
                                .flex()
                                .items_end()
                                .gap(px(6.))
                                .border_b_1()
                                .border_color(p.line)
                                .children(
                                    [
                                        0.40, 0.54, 0.48, 0.62, 0.44, 0.67, 0.72, 0.63, 0.81, 0.71,
                                        0.9, 0.84,
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, height)| {
                                        let opacity = if reduced {
                                            1.
                                        } else {
                                            0.67 + 0.33 * (busy_age * 2.4 - i as f32 * 0.22).cos()
                                        };
                                        div()
                                            .flex_1()
                                            .h(relative(height))
                                            .rounded_t(px(4.))
                                            .bg(p.stage.opacity(opacity))
                                    }),
                                ),
                        )
                        .child(div().h(px(7.)).w(relative(0.65)).bg(p.stage).rounded_full()),
                );
            }
            if !busy && !self.suggestions.is_empty() {
                root =
                    root.child(
                        div()
                            .px(px(12.))
                            .py(px(16.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t("Ask in plain words", "用自然语言提问")),
                            )
                            .child(div().text_size(px(13.)).text_color(p.secondary).child(
                                match l {
                                    UiLanguage::Chinese => {
                                        format!("根据 {} 的只读查询绘制答案。", self.source)
                                    }
                                    _ => {
                                        format!("Answers use a read-only query on {}.", self.source)
                                    }
                                },
                            ))
                            .children(self.suggestions.iter().cloned().enumerate().map(
                                |(i, question)| {
                                    face(
                                        ("ask-data-suggestion", i),
                                        question.clone(),
                                        32.,
                                        radius.into(),
                                        Tone::Surface,
                                        cx,
                                    )
                                    .min_w_0()
                                    .justify_start()
                                    .px(px(8.))
                                    .border_1()
                                    .border_color(p.line)
                                    .text_size(px(13.))
                                    .child(div().min_w_0().truncate().child(question.clone()))
                                    .on_click(cx.listener(
                                        move |s, _, w, cx| {
                                            s.question.update(cx, |field, cx| {
                                                field.set_value(question.clone(), w, cx)
                                            });
                                            if s.run(w, cx) {
                                                s.action_focus.focus(w, cx);
                                            }
                                        },
                                    ))
                                },
                            )),
                    );
            }
            return root;
        };
        let colors: [Hsla; 4] = if p.ink.to_rgb().r > 0.5 {
            [0x53a9ef, 0xb783e7, 0xf0ad46, 0x35c2a7]
        } else {
            [0x197ac2, 0x7027a7, 0xe69414, 0x099786]
        }
        .map(|v| rgb(v).into());
        let view_elapsed = self.view_fade.age(cx);
        if self.shown_view != self.view && (view_elapsed >= 0.10 || disabled_motion(cx)) {
            self.shown_view = self.view;
            self.view_fade_from = 0.;
            self.view_fade.restart(cx);
        }
        let view_opacity = if disabled_motion(cx) {
            1.
        } else {
            let elapsed = self.view_fade.age(cx);
            if elapsed
                < if self.shown_view == self.view {
                    0.16
                } else {
                    0.10
                }
            {
                w.request_animation_frame();
            }
            self.view_opacity(elapsed)
        };
        let choices = [
            (AskDataView::Bars, t("Bar", "柱状图")),
            (AskDataView::Lines, t("Line", "折线图")),
            (AskDataView::Table, t("Table", "表格")),
        ];
        let mut switch = div()
            .w_full()
            .flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(radius)
            .bg(p.stage)
            .border_1()
            .border_color(p.line.opacity(0.5));
        for (i, (view, label)) in choices.into_iter().enumerate() {
            switch = switch.child(
                face(
                    ("ask-data-view", i),
                    label,
                    34.,
                    f32::from(radius).min(4.),
                    Tone::Clear,
                    cx,
                )
                .track_focus(&self.view_focus[i])
                .px(px(12.))
                .text_size(px(14.))
                .line_height(px(20.))
                .border_1()
                .border_color(transparent_black())
                .aria_toggled((self.view == view).into())
                .when(self.view == view, |v| {
                    v.bg(p.panel)
                        .border_color(p.line.opacity(0.65))
                        .text_color(p.ink)
                })
                .when(self.view != view, |v| {
                    v.text_color(p.secondary)
                        .hover(|s| s.bg(p.tray.opacity(0.7)).text_color(p.ink))
                })
                .child(label)
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    let next = match e.keystroke.key.as_str() {
                        "left" => (i + 2) % 3,
                        "right" => (i + 1) % 3,
                        "home" => 0,
                        "end" => 2,
                        "enter" | "space" if !e.is_held => i,
                        _ => return,
                    };
                    s.select_view(choices[next].0, cx);
                    s.view_focus[next].focus(w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }))
                .on_click(cx.listener(move |s, _, _, cx| s.select_view(view, cx))),
            );
        }
        let subtitle = self.presentation.subtitle.clone().or_else(|| {
            result
                .categories
                .first()
                .zip(result.categories.last())
                .map(|(first, last)| {
                    format!(
                        "{first} – {last} · {} {}",
                        result.row_count,
                        t("rows", "行")
                    )
                    .into()
                })
        });
        root = root.child(
            div()
                .flex()
                .flex_col()
                .px(px(12.))
                .pt(px(16.))
                .border_t_1()
                .border_color(p.line)
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(result.title.clone()),
                        )
                        .when_some(subtitle, |header, subtitle| {
                            header.child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.faint)
                                    .child(subtitle),
                            )
                        }),
                )
                .child(switch),
        );
        let last_page = result.categories.len().saturating_sub(1) / 120;
        self.page = self.page.min(last_page);
        let offset = self.page * 120;
        let count = result.categories.len().saturating_sub(offset).min(120);
        self.selected = self.selected.min(count.saturating_sub(1));
        let mut legend = div()
            .px(px(12.))
            .pt(px(12.))
            .pb(px(10.))
            .flex()
            .flex_wrap()
            .gap_x(px(16.))
            .gap_y(px(4.));
        for (i, series) in result.series.iter().enumerate() {
            let mut values = series.values.iter().copied().filter(|v| v.is_finite());
            let value = match self.presentation.legend {
                AskDataLegend::Auto if self.stacked => Some(values.sum()),
                AskDataLegend::Sum => Some(values.sum()),
                AskDataLegend::Last => values.next_back(),
                _ => None,
            };
            legend = legend.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .size(px(8.))
                            .rounded(px(2.5))
                            .bg(colors[i % colors.len()]),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.secondary)
                            .child(series.name.clone()),
                    )
                    .when_some(value, |item, value| {
                        item.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.ink)
                                .child(self.presentation.number(value)),
                        )
                    }),
            );
        }
        root = root.child(legend);
        let mut visual = div().w_full().flex().flex_col().opacity(view_opacity);
        if self.shown_view == AskDataView::Table {
            let mut table = div()
                .id("ask-data-table")
                .role(Role::Table)
                .aria_label(result.title.clone())
                .flex()
                .flex_col()
                .text_size(px(13.))
                .line_height(px(20.))
                .min_w(px((80.
                    + result.series.len() as f32 * 80.
                    + if self.stacked { 80. } else { 0. })
                .max(320.)));
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
                        .w(px(80.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(
                            self.presentation
                                .category_label
                                .clone()
                                .unwrap_or_else(|| t("Category", "分类").into()),
                        ),
                );
            for (j, series) in result.series.iter().enumerate() {
                header = header.child(
                    div()
                        .id(("ask-data-series-head", j))
                        .role(Role::ColumnHeader)
                        .h(px(32.))
                        .px(px(12.))
                        .min_w(px(80.))
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_end()
                        .child(series.name.clone()),
                );
            }
            if self.stacked {
                header = header.child(
                    div()
                        .id("ask-data-total-head")
                        .role(Role::ColumnHeader)
                        .flex()
                        .items_center()
                        .justify_end()
                        .min_w(px(80.))
                        .flex_1()
                        .px(px(12.))
                        .child(t("Total", "合计")),
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
                            .w(px(80.))
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
                        .map_or_else(|| "—".to_owned(), |v| self.presentation.number(*v));
                    row = row.child(
                        div()
                            .id((
                                "ask-data-cell",
                                i.saturating_mul(result.series.len()).saturating_add(j),
                            ))
                            .role(Role::Cell)
                            .h(px(32.))
                            .px(px(12.))
                            .min_w(px(80.))
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_end()
                            .child(value),
                    );
                }
                if self.stacked {
                    row = row.child(
                        div()
                            .id(("ask-data-total", i))
                            .role(Role::Cell)
                            .flex()
                            .items_center()
                            .justify_end()
                            .min_w(px(80.))
                            .flex_1()
                            .px(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.presentation.number(total_at(&result, offset + i))),
                    );
                }
                table = table.child(row);
            }
            let monitor = scroll_monitor(self.table_scroll.clone());
            visual = visual.child(
                div()
                    .relative()
                    .h(px(216.))
                    .mx(px(12.))
                    .rounded(radius)
                    .overflow_hidden()
                    .border_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .id("ask-data-table-scroll")
                            .w_full()
                            .h_full()
                            .overflow_x_scroll()
                            .overflow_y_scroll()
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
                let end = if self.shown_view == AskDataView::Lines {
                    0.46
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
            let kind = self.shown_view;
            let stacked = self.stacked;
            let value_format = self.presentation.format_value.unwrap_or(number);
            let highlighted = self.inspecting.then_some(self.selected);
            let bounds = self.chart_bounds.clone();
            let plot = canvas(
                move |b, _, _| bounds.set(b),
                move |b, _, w, cx| {
                    let count = graph.categories.len().saturating_sub(offset).min(120);
                    if count == 0 {
                        return;
                    }
                    let width = f32::from(b.size.width - px(34.)).max(1.);
                    let height = f32::from(b.size.height - px(27.)).max(1.);
                    let plot_left = b.left() + px(34.);
                    let plot_bottom = b.top() + px(height);
                    let mut low = 0f64;
                    let mut high = 0f64;
                    for i in 0..count {
                        if kind == AskDataView::Bars && stacked {
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
                    let step_scale = ((high - low) / 3.).log10().floor();
                    let unit = 10_f64.powf(step_scale).max(1e-10);
                    let step = [1., 2., 5., 10.]
                        .into_iter()
                        .map(|n| n * unit)
                        .find(|step| *step >= (high - low) / 3.)
                        .unwrap_or(unit * 10.);
                    let max_tick = (high / step).ceil() * step;
                    let min_tick = (low / step).floor() * step;
                    let y = |v: f64| {
                        plot_bottom
                            - px(((v - min_tick) / (max_tick - min_tick).max(step)) as f32 * height)
                    };
                    let pitch = width / count as f32;
                    let paint_label = |text: SharedString,
                                       x: Pixels,
                                       y: Pixels,
                                       align: TextAlign,
                                       w: &mut Window,
                                       cx: &mut App| {
                        let mut run = w.text_style().to_run(text.len());
                        run.color = p.faint;
                        let line = w.text_system().shape_line(text, px(10.), &[run], None);
                        let x = match align {
                            TextAlign::Right => x - line.width,
                            TextAlign::Center => x - line.width / 2.,
                            _ => x,
                        };
                        let _ = line.paint(point(x, y), px(14.), TextAlign::Left, None, w, cx);
                    };
                    for i in 0..=((max_tick - min_tick) / step).round().clamp(0., 8.) as usize {
                        let value = min_tick + i as f64 * step;
                        let row_y = y(value);
                        w.paint_quad(fill(
                            Bounds::new(point(plot_left, row_y), size(px(width), px(1.))),
                            p.line.opacity(0.65),
                        ));
                        paint_label(
                            value_format(value).into(),
                            plot_left - px(6.),
                            row_y - px(7.),
                            TextAlign::Right,
                            w,
                            cx,
                        );
                    }
                    if let Some(index) = highlighted.filter(|index| *index < count) {
                        let x = plot_left + px((index as f32 + 0.5) * pitch);
                        w.paint_quad(fill(
                            Bounds::new(point(x, b.top()), size(px(1.), px(height))),
                            p.faint.opacity(0.35),
                        ));
                    }
                    let every = (46. / pitch).ceil().max(1.) as usize;
                    for i in (0..count).step_by(every) {
                        paint_label(
                            graph.categories[offset + i].clone(),
                            plot_left + px((i as f32 + 0.5) * pitch),
                            plot_bottom + px(5.),
                            TextAlign::Center,
                            w,
                            cx,
                        );
                    }
                    if kind == AskDataView::Bars {
                        for i in 0..count {
                            let mut pos = 0.;
                            let mut neg = 0.;
                            let step = 0.016_f32.min(0.18 / count.max(1) as f32);
                            let grow =
                                standard_ease(((age - i as f32 * step) / 0.32).clamp(0., 1.));
                            let last_positive = graph.series.iter().rposition(|series| {
                                series
                                    .values
                                    .get(offset + i)
                                    .is_some_and(|v| v.is_finite() && *v > 0.)
                            });
                            let last_negative = graph.series.iter().rposition(|series| {
                                series
                                    .values
                                    .get(offset + i)
                                    .is_some_and(|v| v.is_finite() && *v < 0.)
                            });
                            for (j, s) in graph.series.iter().enumerate() {
                                let v = s
                                    .values
                                    .get(offset + i)
                                    .copied()
                                    .filter(|v| v.is_finite())
                                    .unwrap_or(0.)
                                    * f64::from(grow);
                                let (a, z) = if !stacked {
                                    (0., v)
                                } else if v >= 0. {
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
                                    let group = pitch * 0.68;
                                    let column_width = if stacked {
                                        group
                                    } else {
                                        group / graph.series.len() as f32
                                    };
                                    let start = plot_left
                                        + px(i as f32 * pitch
                                            + pitch * 0.16
                                            + if stacked { 0. } else { j as f32 * column_width });
                                    let cap = !stacked
                                        || Some(j)
                                            == if v >= 0. {
                                                last_positive
                                            } else {
                                                last_negative
                                            };
                                    let r = if cap { px(3.).min(h / 2.) } else { px(0.) };
                                    let inset = if stacked { px(0.) } else { px(0.6) };
                                    let seam = if a == 0. {
                                        px(0.)
                                    } else {
                                        px(0.85).min(h / 3.)
                                    };
                                    w.paint_quad(
                                        fill(
                                            Bounds::new(
                                                point(
                                                    start + inset,
                                                    ya.min(yz) + if v < 0. { seam } else { px(0.) },
                                                ),
                                                size(
                                                    (px(column_width) - inset * 2.).max(px(0.5)),
                                                    h - seam,
                                                ),
                                            ),
                                            colors[j % colors.len()],
                                        )
                                        .corner_radii(
                                            Corners {
                                                top_left: if v >= 0. { r } else { px(0.) },
                                                top_right: if v >= 0. { r } else { px(0.) },
                                                bottom_left: if v < 0. { r } else { px(0.) },
                                                bottom_right: if v < 0. { r } else { px(0.) },
                                            },
                                        ),
                                    );
                                }
                            }
                        }
                    } else {
                        for (j, s) in graph.series.iter().enumerate() {
                            let mut path = PathBuilder::stroke(px(2.));
                            let mut previous: Option<Point<Pixels>> = None;
                            let drawn = standard_ease((age / 0.46).clamp(0., 1.));
                            let end = count.saturating_sub(1) as f32 * drawn;
                            for i in 0..count {
                                if i as f32 > end.ceil() {
                                    break;
                                }
                                if let Some(v) = s.values.get(offset + i).filter(|v| v.is_finite())
                                {
                                    let pt = point(plot_left + px((i as f32 + 0.5) * pitch), y(*v));
                                    if let Some(last) = previous {
                                        let progress = (end - (i - 1) as f32).clamp(0., 1.);
                                        path.line_to(last + (pt - last) * progress);
                                    } else {
                                        path.move_to(pt);
                                    }
                                    previous = Some(pt);
                                } else {
                                    previous = None;
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
            let inspecting = self.inspecting;
            let readout = if count == 0 {
                t("No data", "暂无数据").to_owned()
            } else {
                chart_readout(
                    &result,
                    offset + self.selected,
                    &self.presentation,
                    self.stacked.then(|| t("Total", "合计")),
                )
            };
            visual = visual
                .child(
                    div()
                        .id("ask-data-chart")
                        .role(Role::Group)
                        .aria_label(result.title.clone())
                        .aria_value(readout.clone())
                        .track_focus(&self.chart_focus)
                        .tab_index(0)
                        .h(px(216.))
                        .mx(px(12.))
                        .relative()
                        .child(plot)
                        .when(inspecting && count > 0, |chart| {
                            let pos = (self.selected as f32 + 0.5) / count as f32;
                            chart.child(
                                div()
                                    .absolute()
                                    .top(px(10.))
                                    .when(pos < 0.5, |tip| tip.right(px(6.)))
                                    .when(pos >= 0.5, |tip| tip.left(px(34.)))
                                    .max_w(relative(0.7))
                                    .px(px(10.))
                                    .py(px(7.))
                                    .rounded(radius)
                                    .bg(p.panel)
                                    .border_1()
                                    .border_color(p.line)
                                    .text_size(px(12.))
                                    .child(readout.clone()),
                            )
                        })
                        .on_hover(cx.listener(|s, over: &bool, _, cx| {
                            if s.inspecting != *over {
                                s.inspecting = *over;
                                cx.notify();
                            }
                        }))
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
                            s.choose(e.position.x, cx);
                        }))
                        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                            let count = s.result.as_ref().map_or(0, |r| {
                                r.categories.len().saturating_sub(s.page * 120).min(120)
                            });
                            if count == 0 {
                                return;
                            }
                            if e.keystroke.key == "escape" {
                                s.inspecting = false;
                                cx.notify();
                                return;
                            }
                            s.selected = match e.keystroke.key.as_str() {
                                "left" => s.selected.saturating_sub(1),
                                "right" => (s.selected + 1).min(count - 1),
                                "home" => 0,
                                "end" => count - 1,
                                _ => return,
                            };
                            s.inspecting = true;
                            cx.notify();
                            w.prevent_default();
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .id("ask-data-readout")
                        .absolute()
                        .size(px(1.))
                        .overflow_hidden()
                        .role(Role::Status)
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(readout),
                );
        }
        root = root.child(visual);
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
            let words = result
                .insight
                .split_word_bounds()
                .filter(|word| !word.trim().is_empty())
                .count();
            let end = 0.20 + words.div_ceil(4) as f32 * 0.034;
            let age = self.plot.time(end, w, cx);
            let elapsed = if age >= end {
                f32::INFINITY
            } else {
                age - 0.20
            };
            root = root.child(
                div()
                    .min_h(px(40.))
                    .mt(px(8.))
                    .px(px(12.))
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .child(div().flex_none().mt(px(3.)).child(sparkle(p.secondary)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(insight_text(&result.insight, elapsed)),
                    ),
            );
        }
        root.pb(px(12.))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AskDataPresentation, AskDataResult, AskDataSeries, AskDataStatus, AskDataView, AskYourData,
        chart_readout,
    };
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
                assert!(view.reserve_sql_lines(generation, 12, cx));
                assert!(view.append_sql(generation, "SELECT 1", cx));
                assert!(view.finish_sql(generation, cx));
                let query = view.active_request().unwrap();
                assert_ne!(generation, query);
                assert!(!view.reserve_sql_lines(generation, 20, cx));
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
    fn a_result_can_supply_view_and_shared_numeric_readouts() {
        let result = AskDataResult::new(
            "Revenue",
            vec!["Jan".into()],
            vec![
                AskDataSeries::new("EU", vec![55.]),
                AskDataSeries::new("US", vec![15.]),
            ],
        );
        let presentation = AskDataPresentation {
            view: Some(AskDataView::Lines),
            format_value: Some(|v| format!("${v:.0}k")),
            ..Default::default()
        };
        assert_eq!(
            chart_readout(&result, 0, &presentation, Some("Total")),
            "Jan · EU: $55k · US: $15k · Total: $70k",
        );
        assert_eq!(
            chart_readout(&result, 0, &presentation, None),
            "Jan · EU: $55k · US: $15k"
        );

        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, window) = app
            .add_window_view(|w, cx| AskYourData::new("analytics", w, cx).with_keep_sql_open(true));
        window.update(|w, cx| {
            control
                .read(cx)
                .question()
                .update(cx, |field, cx| field.set_value("Revenue", w, cx));
            control.update(cx, |view, cx| {
                assert!(view.run(w, cx));
                let id = view.active_request().unwrap();
                assert!(view.append_sql(id, "SELECT 1", cx));
                assert!(view.finish_sql(id, cx));
                assert!(view.sql_open);
                let id = view.active_request().unwrap();
                assert!(view.finish_query_presented(id, result, presentation, cx));
                assert_eq!(view.shown_view, AskDataView::Lines);
                view.toggle_sql(cx);
                assert!(view.run(w, cx));
                assert!(!view.sql_open);
            });
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
