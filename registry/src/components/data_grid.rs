//! Virtual, editable spreadsheet. Stable backing row indices survive view sorting.
use crate::primitives::{
    editor::{EditEvent, Editor, EditorScrollRegion, InputMode},
    family::{Paint, PhaseClock, Slide, disabled_motion},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::*,
};
use gpui::{prelude::*, *};
/// Optional browser save adapter installed by the preview host.
pub struct CsvDownload(pub fn(&str, &str));
impl Global for CsvDownload {}
pub struct GridExport {
    pub filename: String,
    pub csv: String,
}
impl EventEmitter<GridExport> for DataGrid {}
#[derive(Clone, Debug)]
pub struct CellEdited {
    pub row: usize,
    pub column: usize,
    pub value: SharedString,
}
#[derive(Clone, Copy, PartialEq)]
pub struct GridCell {
    pub row: usize,
    pub col: usize,
}
#[derive(Clone, Copy, PartialEq)]
enum GridPanel {
    Filters,
    Columns,
}
/// Virtual rows are laid out as independent roots by GPUI. A row must
/// receive exactly the same pixel tracks as its header, even while scrolled.
struct GridTracks {
    width: Pixels,
    gutter: Pixels,
    columns: Vec<Pixels>,
}
impl GridTracks {
    fn new(viewport: Pixels, count: usize, minimum: Pixels) -> Self {
        let gutter = px(32.);
        let space = f32::from((viewport - gutter).max(minimum * count as f32));
        let edge = |i: usize| (space * i as f32 / count.max(1) as f32).round();
        let columns = (0..count).map(|i| px(edge(i + 1) - edge(i))).collect();
        Self {
            width: gutter + px(space.round()),
            gutter,
            columns,
        }
    }
}
pub struct DataGrid {
    visible: Vec<usize>,
    filters: Vec<Entity<Editor>>,
    _filter_subs: Vec<Subscription>,
    panel: Option<GridPanel>,
    filter_anchor: TriggerBounds,
    columns_anchor: TriggerBounds,
    filter_focus: FocusHandle,
    columns_focus: FocusHandle,
    panel_focus: FocusHandle,
    title: SharedString,
    hscroll: ScrollHandle,
    viewport_width: Pixels,
    min_column_width: Pixels,
    headers: Vec<SharedString>,
    data: Vec<Vec<SharedString>>,
    order: Vec<usize>,
    // Only retain layout state for virtual rows that were on screen at sort time.
    moving: std::collections::BTreeMap<usize, Slide>,
    sorted: PhaseClock,
    edits: std::collections::BTreeMap<(usize, usize), PhaseClock>,
    anchor: GridCell,
    active: GridCell,
    editing: Option<(usize, usize)>,
    sort: Option<(usize, bool)>,
    selection_frame: Option<([Slide; 8], Pixels, usize)>,
    focus: FocusHandle,
    editor: Entity<Editor>,
    search: Entity<Editor>,
    _search_sub: Subscription,
    originals: std::collections::BTreeMap<(usize, usize), SharedString>,
    scroll: UniformListScrollHandle,
    _sub: Subscription,
    dragging: bool,
    fill: bool,
}
impl EventEmitter<CellEdited> for DataGrid {}
impl DataGrid {
    pub fn new(
        title: impl Into<SharedString>,
        headers: Vec<SharedString>,
        mut rows: Vec<Vec<SharedString>>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let cols = headers.len();
        for row in &mut rows {
            row.resize(cols, SharedString::default())
        }
        let cell = GridCell { row: 0, col: 0 };
        let editor =
            cx.new(|cx| Editor::new(tr(cx, "Edit cell", "编辑单元格"), InputMode::Single, w, cx));
        let sub = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, _, e: &EditEvent, w, cx| match e {
                EditEvent::Submitted => s.end_edit(true, w, cx),
                EditEvent::Escape => s.end_edit(false, w, cx),
                EditEvent::Blurred => s.commit(cx),
                _ => {}
            },
        );
        let search = cx.new(|cx| {
            let mut ed = Editor::new(tr(cx, "Search rows", "搜索行"), InputMode::Search, w, cx);
            ed.set_placeholder(tr(cx, "Search rows", "搜索行"));
            ed
        });
        let search_sub = cx.subscribe_in(
            &search,
            w,
            |s: &mut Self, _, event: &EditEvent, w, cx| match event {
                EditEvent::Changed => s.filter(cx),
                EditEvent::Escape => {
                    s.search.update(cx, |ed, cx| ed.set_value("", w, cx));
                    s.focus.focus(w, cx);
                    s.filter(cx)
                }
                EditEvent::Submitted => s.focus.focus(w, cx),
                _ => {}
            },
        );
        let mut filters = Vec::new();
        let mut filter_subs = Vec::new();
        for head in &headers {
            let field = cx.new(|cx| {
                let label = format!("{} {}", head, tr(cx, "filter", "筛选"));
                let mut e = Editor::new(label, InputMode::Single, w, cx);
                e.set_placeholder(tr(cx, "Filter value", "筛选值"));
                e
            });
            filter_subs.push(cx.subscribe_in(
                &field,
                w,
                |s: &mut Self, _, ev: &EditEvent, w, cx| match ev {
                    EditEvent::Changed => s.filter(cx),
                    EditEvent::Escape | EditEvent::Submitted => s.close_panel(w, cx),
                    _ => {}
                },
            ));
            filters.push(field);
        }
        Self {
            visible: (0..cols).collect(),
            filters,
            _filter_subs: filter_subs,
            panel: None,
            filter_anchor: Default::default(),
            columns_anchor: Default::default(),
            filter_focus: cx.focus_handle().tab_stop(true),
            columns_focus: cx.focus_handle().tab_stop(true),
            panel_focus: cx.focus_handle(),
            search,
            _search_sub: search_sub,
            originals: Default::default(),
            title: title.into(),
            hscroll: ScrollHandle::new(),
            viewport_width: px(0.),
            min_column_width: px(94.),
            headers,
            order: (0..rows.len()).collect(),
            moving: Default::default(),
            edits: Default::default(),
            sorted: Default::default(),
            data: rows,
            anchor: cell,
            active: cell,
            editing: None,
            sort: None,
            selection_frame: None,
            focus: cx.focus_handle().tab_stop(true),
            editor,
            scroll: UniformListScrollHandle::new(),
            _sub: sub,
            dragging: false,
            fill: false,
        }
    }
    fn filter(&mut self, cx: &mut Context<Self>) {
        self.selection_frame = None;
        self.moving.clear();
        self.sorted = Default::default();
        self.commit(cx);
        let q = self.search.read(cx).value().trim().to_lowercase();
        let words = q.split_whitespace().collect::<Vec<_>>();
        let column_filters = self
            .filters
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.read(cx).value().trim().to_lowercase()))
            .filter(|(_, q)| !q.is_empty())
            .collect::<Vec<_>>();
        self.order = (0..self.data.len())
            .filter(|r| {
                let row = &self.data[*r];
                column_filters
                    .iter()
                    .all(|(i, q)| Self::matches_filter(&row[*i], q))
                    && (words.is_empty() || {
                        let text = self
                            .visible
                            .iter()
                            .map(|c| row[*c].to_lowercase())
                            .collect::<Vec<_>>()
                            .join(" ");
                        words.iter().all(|w| text.contains(*w))
                    })
            })
            .collect();
        if let Some((column, desc)) = self.sort {
            self.order_rows(column, desc)
        }
        self.active = GridCell { row: 0, col: 0 };
        self.anchor = self.active;
        self.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(0.)));
        cx.notify()
    }
    fn numeric_value(text: &str) -> Option<f64> {
        let raw = text.trim().trim_start_matches('$').trim_end_matches('%');
        let value = if raw.contains(',') {
            raw.replace(',', "").parse::<f64>()
        } else {
            raw.parse::<f64>()
        };
        value.ok().filter(|n| n.is_finite())
    }

    /// Case-insensitive substring, =exact, !exclude or a numeric comparison.
    fn matches_filter(value: &str, query: &str) -> bool {
        for op in [">=", "<=", ">", "<"] {
            if let Some(arg) = query.strip_prefix(op) {
                return match (Self::numeric_value(value), Self::numeric_value(arg)) {
                    (Some(n), Some(v)) => match op {
                        ">=" => n >= v,
                        "<=" => n <= v,
                        ">" => n > v,
                        _ => n < v,
                    },
                    _ => false,
                };
            }
        }
        let value = value.to_lowercase();
        if let Some(q) = query.strip_prefix('=') {
            value == q
        } else if let Some(q) = query.strip_prefix('!') {
            !value.contains(q)
        } else {
            value.contains(query)
        }
    }
    /// CSV contains the visible columns and filtered rows in their current display order.
    pub fn csv(&self) -> String {
        use std::fmt::Write;
        fn record<'a>(out: &mut String, fields: impl Iterator<Item = &'a str>) {
            for (i, field) in fields.enumerate() {
                if i != 0 {
                    out.push(',')
                }
                if field.contains([',', '"', '\n', '\r']) {
                    let _ = write!(out, "\"{}\"", field.replace('"', "\"\""));
                } else {
                    out.push_str(field)
                }
            }
            out.push_str("\r\n");
        }
        let mut csv = String::new();
        record(
            &mut csv,
            self.visible.iter().map(|c| self.headers[*c].as_ref()),
        );
        for &r in &self.order {
            record(
                &mut csv,
                self.visible.iter().map(|c| self.data[r][*c].as_ref()),
            )
        }
        csv
    }
    fn export(&mut self, cx: &mut Context<Self>) {
        self.commit(cx);
        let stem = self
            .title
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
            .collect::<String>()
            .to_lowercase();
        let name = if stem.trim().is_empty() {
            "export"
        } else {
            stem.trim()
        };
        cx.emit(GridExport {
            filename: format!("{}.csv", name.replace(' ', "-")),
            csv: self.csv(),
        });
    }
    fn toggle_column(&mut self, col: usize, cx: &mut Context<Self>) {
        if let Some(pos) = self.visible.iter().position(|c| *c == col) {
            if self.visible.len() == 1 {
                return;
            }
            self.visible.remove(pos);
        } else {
            self.visible.push(col)
        }
        self.filter(cx)
    }
    fn move_column(&mut self, col: usize, left: bool, cx: &mut Context<Self>) {
        if let Some(i) = self.visible.iter().position(|c| *c == col) {
            let to = if left {
                i.saturating_sub(1)
            } else {
                (i + 1).min(self.visible.len() - 1)
            };
            self.visible.swap(i, to);
            self.anchor.col = to;
            self.active.col = to;
            cx.notify()
        }
    }
    fn close_panel(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        match self.panel.take() {
            Some(GridPanel::Filters) => self.filter_focus.focus(w, cx),
            Some(GridPanel::Columns) => self.columns_focus.focus(w, cx),
            None => return,
        }
        cx.notify()
    }
    fn toggle_panel(&mut self, panel: GridPanel, w: &mut Window, cx: &mut Context<Self>) {
        if self.panel == Some(panel) {
            self.close_panel(w, cx);
            return;
        }
        self.panel = Some(panel);
        if panel == GridPanel::Filters
            && let Some(f) = self.filters.first()
        {
            f.update(cx, |e, cx| e.focus(w, cx))
        } else {
            self.panel_focus.focus(w, cx)
        }
        cx.notify()
    }
    /// Minimum width of each data column before the grid scrolls horizontally.
    pub fn with_min_column_width(mut self, width: Pixels) -> Self {
        self.min_column_width = width.clamp(px(72.), px(360.));
        self
    }

    fn tools(&self, w: &Window, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let mut toolbar = row().gap_1().px_2().h(px(36.)).child(
            div()
                .min_w_0()
                .h(px(28.))
                .flex_1()
                .child(self.search.clone()),
        );
        let count = if self.order.len() == self.data.len() {
            self.data.len().to_string()
        } else {
            format!("{} / {}", self.order.len(), self.data.len())
        };
        if w.viewport_size().width >= px(410.) {
            toolbar = toolbar.child(
                label(
                    format!("{count} {}", tr(cx, "rows", "行")),
                    11.,
                    p.secondary,
                )
                .flex_none()
                .px_2(),
            );
        }
        let buttons = [
            (
                "grid-filter",
                tr(cx, "Filters", "筛选"),
                Glyph::Settings,
                Some(GridPanel::Filters),
            ),
            (
                "grid-columns",
                tr(cx, "Columns", "列"),
                Glyph::Menu,
                Some(GridPanel::Columns),
            ),
            (
                "grid-export",
                tr(cx, "Export", "导出"),
                Glyph::Download,
                None,
            ),
        ];
        for (id, name, g, kind) in buttons {
            let mut b = button(id, name, cx)
                .size(px(32.))
                .p_0()
                .relative()
                .child(icon(g, 16., p.secondary));
            if let Some(k) = kind {
                let (anchor, focus) = if k == GridPanel::Filters {
                    (&self.filter_anchor, &self.filter_focus)
                } else {
                    (&self.columns_anchor, &self.columns_focus)
                };
                b = b
                    .track_focus(focus)
                    .aria_expanded(self.panel == Some(k))
                    .child(anchor.measure());
            }
            toolbar = toolbar.child(b.on_click(cx.listener(move |s, _, w, cx| {
                if let Some(k) = kind {
                    s.toggle_panel(k, w, cx)
                } else {
                    s.export(cx)
                }
            })));
        }
        let Some(k) = self.panel else { return toolbar };
        let mut pop = panel(cx)
            .id("grid-panel")
            .track_focus(&self.panel_focus)
            .tab_stop(false)
            .role(Role::Dialog)
            .occlude()
            .w(fit(w, 260.))
            .max_h(w.viewport_size().height - px(24.))
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .shadow_lg()
            .on_mouse_down_out(cx.listener(|s, e: &MouseDownEvent, w, cx| {
                if !s.filter_anchor.get().contains(&e.position)
                    && !s.columns_anchor.get().contains(&e.position)
                {
                    s.close_panel(w, cx)
                }
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" {
                    s.close_panel(w, cx);
                    cx.stop_propagation()
                }
            }));
        if k == GridPanel::Filters {
            for (i, field) in self.filters.iter().enumerate() {
                pop = pop.child(
                    row()
                        .gap_1()
                        .child(
                            label(self.headers[i].clone(), 12., p.ink)
                                .w(px(78.))
                                .truncate(),
                        )
                        .child(
                            div()
                                .h(px(28.))
                                .flex_1()
                                .min_w_0()
                                .rounded(crate::foundation::surface_radius(cx))
                                .border_1()
                                .border_color(p.line)
                                .child(field.clone()),
                        ),
                )
            }
        } else {
            let columns = self
                .visible
                .iter()
                .copied()
                .chain((0..self.headers.len()).filter(|c| !self.visible.contains(c)));
            for c in columns {
                let visible = self.visible.contains(&c);
                let name = self.headers[c].clone();
                let toggle = format!("{} {name}", tr(cx, "Show column", "显示列"));
                let mut line = row().gap_1().child(
                    button(("show-col", c), toggle, cx)
                        .flex_1()
                        .justify_start()
                        .child(if visible {
                            icon(Glyph::Check, 12., p.ink).into_any_element()
                        } else {
                            div().size(px(12.)).into_any_element()
                        })
                        .child(name.clone())
                        .on_click(cx.listener(move |s, _, _, cx| s.toggle_column(c, cx))),
                );
                if visible {
                    for (left, g) in [(true, Glyph::Up), (false, Glyph::Down)] {
                        let verb = if left {
                            tr(cx, "Move left", "左移")
                        } else {
                            tr(cx, "Move right", "右移")
                        };
                        line = line.child(
                            button(
                                (if left { "move-left" } else { "move-right" }, c),
                                format!("{verb} {name}"),
                                cx,
                            )
                            .size(px(26.))
                            .p_0()
                            .child(icon(g, 12., p.secondary))
                            .on_click(cx.listener(move |s, _, _, cx| s.move_column(c, left, cx))),
                        )
                    }
                }
                pop = pop.child(line);
            }
        }
        let anchor = if k == GridPanel::Filters {
            &self.filter_anchor
        } else {
            &self.columns_anchor
        };
        let (pos, side) = anchor.place(
            w.viewport_size(),
            px(36. * self.headers.len() as f32 + 16.),
            px(4.),
        );
        toolbar.child(overlay(pop, pos, side))
    }
    pub fn reset(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.editing = None;
        for ((row, column), value) in std::mem::take(&mut self.originals) {
            self.data[row][column] = value.clone();
            cx.emit(CellEdited { row, column, value });
        }
        self.search.update(cx, |ed, cx| ed.set_value("", w, cx));
        self.sort = None;
        self.visible = (0..self.headers.len()).collect();
        for entry in &self.filters {
            entry.update(cx, |ed, cx| ed.set_value("", w, cx))
        }
        self.panel = None;
        self.filter(cx)
    }
    fn write(&mut self, row: usize, column: usize, value: SharedString, cx: &mut Context<Self>) {
        if self.data[row][column] == value {
            return;
        }
        self.originals
            .entry((row, column))
            .or_insert_with(|| self.data[row][column].clone());
        self.data[row][column] = value.clone();
        self.edits.entry((row, column)).or_default().restart(cx);
        cx.emit(CellEdited { row, column, value });
    }
    fn commit(&mut self, cx: &mut Context<Self>) {
        if let Some((r, c)) = self.editing.take() {
            let value: SharedString = self.editor.read(cx).value().to_owned().into();
            self.write(r, c, value, cx);
            cx.notify()
        }
    }
    fn end_edit(&mut self, save: bool, w: &mut Window, cx: &mut Context<Self>) {
        if save {
            self.commit(cx)
        } else {
            self.editing = None
        }
        self.focus.focus(w, cx);
        cx.notify()
    }
    fn begin_edit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(&r) = self.order.get(self.active.row) else {
            return;
        };
        let Some(&c) = self.visible.get(self.active.col) else {
            return;
        };
        let Some(v) = self.data[r].get(c) else { return };
        self.editing = Some((r, c));
        self.editor.update(cx, |e, cx| {
            e.set_value(v.to_string(), w, cx);
            e.focus(w, cx);
            e.select_all(w, cx)
        });
        cx.notify()
    }
    pub fn sort_by(&mut self, col: usize, cx: &mut Context<Self>) {
        if col >= self.headers.len() {
            return;
        }
        self.commit(cx);
        let desc = self.sort.is_some_and(|(c, d)| c == col && !d);
        let mut from = std::collections::BTreeMap::new();
        if !disabled_motion(cx) {
            let scroll = self.scroll.0.borrow().base_handle.clone();
            let first = ((-f32::from(scroll.offset().y) / 31.).floor() as usize).saturating_sub(1);
            let count = (f32::from(scroll.bounds().size.height) / 31.).ceil() as usize + 3;
            for (r, &key) in self.order.iter().enumerate().skip(first).take(count) {
                from.insert(
                    key,
                    self.moving
                        .remove(&key)
                        .unwrap_or_else(|| Slide::new(r as f32)),
                );
            }
            self.sorted.restart(cx);
        }
        self.order_rows(col, desc);
        for (r, key) in self.order.iter().enumerate() {
            if let Some(pos) = from.get_mut(key) {
                pos.duration(r as f32, 0.58, cx);
            }
        }
        self.moving = from;
        self.sort = Some((col, desc));
        cx.notify()
    }
    fn order_rows(&mut self, col: usize, desc: bool) {
        let data = &self.data;
        self.order.sort_by(|a, b| {
            let x = &data[*a][col];
            let y = &data[*b][col];
            let cmp = match (Self::numeric_value(x), Self::numeric_value(y)) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => x.cmp(y),
            };
            if desc { cmp.reverse() } else { cmp }
        });
    }
    fn selection_sum(&self) -> Option<f64> {
        let mut any = false;
        let mut total = 0.;
        for r in self.anchor.row.min(self.active.row)..=self.anchor.row.max(self.active.row) {
            let &data = self.order.get(r)?;
            for c in self.anchor.col.min(self.active.col)..=self.anchor.col.max(self.active.col) {
                let value = self.data[data].get(*self.visible.get(c)?)?;
                if let Some(n) = Self::numeric_value(value) {
                    total += n;
                    any = true
                }
            }
        }
        any.then_some(total)
    }
    fn finish_drag(&mut self, cx: &mut Context<Self>) {
        if self.fill && !self.order.is_empty() && !self.headers.is_empty() {
            let value =
                self.data[self.order[self.anchor.row]][self.visible[self.anchor.col]].clone();
            for r in self.anchor.row.min(self.active.row)..=self.anchor.row.max(self.active.row) {
                for c in self.anchor.col.min(self.active.col)..=self.anchor.col.max(self.active.col)
                {
                    let row = self.order[r];
                    self.write(row, self.visible[c], value.clone(), cx);
                }
            }
        }
        self.fill = false;
        self.dragging = false;
        cx.notify()
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(w)
            || self.editing.is_some()
            || self.order.is_empty()
            || self.headers.is_empty()
        {
            return;
        }
        let mut cell = self.active;
        match e.keystroke.key.as_str() {
            "enter" | "f2" => {
                self.begin_edit(w, cx);
                cx.stop_propagation();
                return;
            }
            "up" => cell.row = cell.row.saturating_sub(1),
            "down" => cell.row = (cell.row + 1).min(self.order.len() - 1),
            "left" => cell.col = cell.col.saturating_sub(1),
            "right" => cell.col = (cell.col + 1).min(self.visible.len() - 1),
            "home" => cell.col = 0,
            "end" => cell.col = self.visible.len() - 1,
            _ => return,
        }
        self.active = cell;
        if !e.keystroke.modifiers.shift {
            self.anchor = cell
        }
        self.scroll.scroll_to_item(cell.row, ScrollStrategy::Center);
        cx.stop_propagation();
        cx.notify()
    }
    fn line(&self, r: usize, tracks: &GridTracks, w: &Window, cx: &mut Context<Self>) -> Div {
        let p = Paint::get(cx);
        let data = self.order[r];
        let t = self.sorted.time(0.7, w, cx) / 0.7;
        let offset = if t >= 1. {
            0.
        } else {
            self.moving
                .get(&data)
                .map_or(0., |s| (s.get(w, cx) - r as f32) * 31.)
        };
        let mut line = row()
            .relative()
            .top(px(offset))
            .opacity(if self.moving.contains_key(&data) {
                1.
            } else {
                (t * 2.).min(1.)
            })
            .bg(p.panel)
            .w(tracks.width)
            .h(px(31.))
            .border_b_1()
            .border_color(p.line)
            .child(
                label((r + 1).to_string(), 11., p.faint)
                    .w(tracks.gutter)
                    .flex_none()
                    .px_1()
                    .id(("row-number", r))
                    .debug_selector(move || format!("grid-row-number-{r}")),
            );
        for (c, &column) in self.visible.iter().enumerate() {
            let v = &self.data[data][column];
            let cell = GridCell { row: r, col: c };
            let mut el = div()
                .id(("cell", (r * self.headers.len()) + c))
                .role(Role::GridCell)
                .debug_selector(move || format!("grid-cell-{r}-{c}"))
                .relative()
                .flex_none()
                .w(tracks.columns[c])
                .overflow_hidden()
                .h_full()
                .border_l_1()
                .border_color(p.line)
                .px_1()
                .flex()
                .items_center()
                .when_some(self.edits.get(&(data, column)), |d, edited| {
                    let strength = 1. - edited.time(0.9, w, cx) / 0.9;
                    if strength > 0. {
                        d.bg(p.success.opacity(strength * 0.20))
                    } else {
                        d
                    }
                });
            if self.editing == Some((data, column)) {
                el = el.child(self.editor.clone())
            } else {
                el = el
                    .child(label(v.clone(), 12., p.ink).truncate())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                            s.commit(cx);
                            s.active = cell;
                            if !e.modifiers.shift {
                                s.anchor = cell
                            }
                            s.dragging = true;
                            s.focus.focus(w, cx);
                            if e.click_count == 2 {
                                s.dragging = false;
                                s.begin_edit(w, cx)
                            }
                            cx.notify()
                        }),
                    )
                    .on_mouse_move(cx.listener(move |s, e: &MouseMoveEvent, _, cx| {
                        if s.dragging
                            && e.pressed_button == Some(MouseButton::Left)
                            && s.active != cell
                        {
                            s.active = cell;
                            cx.notify()
                        }
                    }));
                if self.anchor == cell && self.active == cell {
                    el = el.child(
                        div()
                            .id("fill")
                            .debug_selector(|| "grid-fill".into())
                            .absolute()
                            .right_0()
                            .bottom_0()
                            .size(px(7.))
                            .bg(p.secondary)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|s, _, _, cx| {
                                    s.dragging = true;
                                    s.fill = true;
                                    cx.stop_propagation()
                                }),
                            ),
                    )
                }
            }
            line = line.child(el)
        }
        line
    }
}
impl Render for DataGrid {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let viewport = if self.viewport_width > px(0.) {
            self.viewport_width
        } else {
            fit(w, 680.) - px(2.)
        };
        self.edits.retain(|_, phase| phase.age(cx) < 0.9);
        let tracks = GridTracks::new(viewport, self.visible.len(), self.min_column_width);
        let width = tracks.width;
        let gutter = tracks.gutter;
        // Coordinates are in scroll-content space. Scrolling adds the live
        // offset at paint time instead of retargeting the highlight behind it.
        let edge = |col: usize| {
            f32::from(tracks.gutter + tracks.columns.iter().take(col).copied().sum::<Pixels>())
        };
        let (a, b) = (self.anchor, self.active);
        let targets = [
            edge(a.col.min(b.col)),
            edge(a.col.max(b.col) + 1),
            31. * a.row.min(b.row) as f32,
            31. * (a.row.max(b.row) + 1) as f32,
            edge(b.col),
            edge(b.col + 1),
            31. * b.row as f32,
            31. * (b.row + 1) as f32,
        ];
        let count = self.visible.len();
        let (rect, last_width, last_count) =
            self.selection_frame
                .get_or_insert((targets.map(Slide::new), width, count));
        if *last_width != width || *last_count != count {
            *rect = targets.map(Slide::new);
            *last_width = width;
            *last_count = count;
        }
        let mut selection = [0.; 8];
        for (i, s) in rect.iter_mut().enumerate() {
            s.duration(targets[i], 0.19, cx);
            selection[i] = s.get(w, cx);
        }
        let has_cells = !self.order.is_empty() && !self.visible.is_empty();
        let mut head = row().w(width).h(px(32.)).bg(p.stage).child(
            div()
                .id("number-heading")
                .debug_selector(|| "grid-number-heading".into())
                .w(tracks.gutter)
                .flex_none(),
        );
        for (at, &i) in self.visible.iter().enumerate() {
            let c = &self.headers[i];
            head = head.child(
                button(("header", i), c.clone(), cx)
                    .flex_none()
                    .w(tracks.columns[at])
                    .overflow_hidden()
                    .px_1()
                    .justify_start()
                    .child(label(c.clone(), 12., p.secondary).truncate())
                    .on_click(cx.listener(move |s, _, _, cx| s.sort_by(i, cx))),
            )
        }
        let this = cx.entity();
        let list = uniform_list("rows", self.order.len(), move |range, w, cx| {
            this.update(cx, |s, cx| {
                range.map(|r| s.line(r, &tracks, w, cx)).collect::<Vec<_>>()
            })
        })
        .track_scroll(&self.scroll)
        .w(width)
        .h(px(310.));
        let sc = self.scroll.0.borrow().base_handle.clone();
        let monitor = canvas(
            |_, _, _| (),
            move |b, _, w, cx| {
                if has_cells {
                    let scroll_y = sc.offset().y;
                    w.with_content_mask(Some(ContentMask { bounds: b }), |w| {
                        for (idx, fill, stroke) in [
                            (0, p.ink.opacity(0.045), p.secondary.opacity(0.44)),
                            (4, p.ink.opacity(0.), p.secondary),
                        ] {
                            let x = b.left() + px(selection[idx]);
                            let y = b.top() + scroll_y + px(selection[idx + 2]);
                            let bounds = Bounds::new(
                                point(x, y),
                                size(
                                    px(selection[idx + 1] - selection[idx]),
                                    px(selection[idx + 3] - selection[idx + 2]),
                                ),
                            );
                            w.paint_quad(quad(
                                bounds,
                                px(0.),
                                fill,
                                px(1.),
                                stroke,
                                BorderStyle::Solid,
                            ));
                        }
                    });
                }
                if let Some(h) = cx.try_global::<EditorScrollRegion>() {
                    (h.0)(
                        b,
                        -sc.offset().y > px(0.1),
                        -sc.offset().y + px(0.1) < sc.max_offset().y,
                    )
                }
            },
        )
        .absolute()
        .inset_0();
        let weak = cx.weak_entity();
        let x_scroll = self.hscroll.clone();
        let y_scroll = self.scroll.0.borrow().base_handle.clone();
        let row_count = self.order.len();
        let frozen_gutter = canvas(
            |_, _, _| (),
            move |bounds, _, w, cx| {
                if x_scroll.offset().x >= px(-0.01) {
                    return;
                }
                let heading = Bounds::new(bounds.origin, size(gutter, px(32.)));
                w.paint_quad(fill(heading, p.stage));
                let body = Bounds::new(
                    point(bounds.left(), bounds.top() + px(32.)),
                    size(gutter, px(310.)),
                );
                let y = y_scroll.offset().y;
                let first = ((-f32::from(y)) / 31.).floor().max(0.) as usize;
                w.with_content_mask(Some(ContentMask { bounds: body }), |w| {
                    w.paint_quad(fill(body, p.panel));
                    for i in first..row_count.min(first + 12) {
                        let top = body.top() + y + px(i as f32 * 31.);
                        let number: SharedString = (i + 1).to_string().into();
                        let mut run = w.text_style().to_run(number.len());
                        run.color = p.faint;
                        let line = w.text_system().shape_line(number, px(11.), &[run], None);
                        let _ = line.paint(
                            point(body.left() + px(4.), top + px(7.)),
                            px(16.),
                            TextAlign::Left,
                            None,
                            w,
                            cx,
                        );
                        w.paint_quad(fill(
                            Bounds::new(point(body.left(), top + px(30.)), size(gutter, px(1.))),
                            p.line,
                        ));
                    }
                    w.paint_quad(fill(
                        Bounds::new(
                            point(body.right() - px(1.), body.top()),
                            size(px(1.), body.size.height),
                        ),
                        p.line,
                    ));
                });
            },
        )
        .absolute()
        .left_0()
        .top_0()
        .w(gutter)
        .h(px(342.));
        let previous_width = self.viewport_width;
        let count = if self.order.is_empty() || self.headers.is_empty() {
            0
        } else {
            (self.active.row.abs_diff(self.anchor.row) + 1)
                * (self.active.col.abs_diff(self.anchor.col) + 1)
        };
        let total = self
            .selection_sum()
            .map(|v| format!("   ·   {} {v}", tr(cx, "Sum", "合计")))
            .unwrap_or_default();
        panel(cx)
            .id("grid")
            .role(Role::Grid)
            .track_focus(&self.focus)
            .w(fit(w, 680.))
            .max_w_full()
            .on_key_down(cx.listener(|s, e, w, cx| s.key(e, w, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, _, cx| s.finish_drag(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, _, _, cx| s.finish_drag(cx)),
            )
            .child(
                row()
                    .p_2()
                    .justify_between()
                    .child(label(self.title.clone(), 15., p.ink))
                    .child(
                        button("reset", tr(cx, "Reset", "重置"), cx)
                            .h(px(28.))
                            .child(tr(cx, "Reset", "重置"))
                            .on_click(cx.listener(|s, _, w, cx| s.reset(w, cx))),
                    ),
            )
            .child(self.tools(w, cx))
            .child(
                div()
                    .relative()
                    .w_full()
                    .child(
                        horizontal_viewport(
                            "horizontal",
                            &self.hscroll,
                            column()
                                .w(width)
                                .flex_none()
                                .child(head)
                                .child(div().relative().w(width).child(list).child(monitor)),
                        )
                        .on_children_prepainted(move |bounds, _, cx| {
                            if let Some(view) = bounds.first()
                                && (view.size.width - previous_width).abs() > px(0.01)
                            {
                                let _ = weak.update(cx, |grid, cx| {
                                    grid.viewport_width = view.size.width;
                                    cx.notify()
                                });
                            }
                        }),
                    )
                    .child(frozen_gutter)
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .w(gutter)
                            .h(px(342.))
                            .id("grid-fixed-gutter")
                            .block_mouse_except_scroll(),
                    ),
            )
            .child(
                label(
                    format!(
                        "{} {}  ·  {} {}{total}",
                        tr(cx, "Row", "行"),
                        self.active.row + 1,
                        tr(cx, "Count", "计数"),
                        count
                    ),
                    12.,
                    p.secondary,
                )
                .px_2()
                .py_1(),
            )
    }
}
