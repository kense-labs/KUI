//! A hierarchical table with stable row identity, ancestor guides, tri-state selection and host-driven lazy loading.
use crate::primitives::{
    editor::EditorScrollRegion,
    family::{Paint, Slide, Tone, face},
    form_chrome::{check_stroke, tr},
    glyph::{Glyph, icon, spinner, transformed},
};
use gpui::{prelude::*, *};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeTableRow {
    pub id: SharedString,
    pub name: SharedString,
    pub cells: Vec<SharedString>,
    pub children: Vec<TreeTableRow>,
    pub lazy: bool,
}
impl TreeTableRow {
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            cells: Vec::new(),
            children: Vec::new(),
            lazy: false,
        }
    }
    pub fn with_cells(mut self, cells: Vec<SharedString>) -> Self {
        self.cells = cells;
        self
    }
    pub fn with_children(mut self, children: Vec<TreeTableRow>) -> Self {
        self.children = children;
        self
    }
    pub fn with_lazy_children(mut self) -> Self {
        self.lazy = true;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeTableEvent {
    LoadRequested(SharedString),
    Activated(SharedString),
    SelectionChanged(Vec<SharedString>),
    ExpandedChanged(Vec<SharedString>),
    SortChanged(Option<(usize, bool)>),
}
#[derive(Clone)]
struct Visible {
    row: TreeTableRow,
    depth: usize,
    parent: Option<SharedString>,
    amount: f32,
    position: usize,
    siblings: usize,
}
/// Renders a hierarchical table. Stable unique row IDs are supplied by the host.
pub struct TreeTable {
    label: SharedString,
    columns: Vec<SharedString>,
    rows: Vec<TreeTableRow>,
    expanded: HashSet<SharedString>,
    selected: HashSet<SharedString>,
    loading: HashSet<SharedString>,
    failed: HashSet<SharedString>,
    focused: Option<SharedString>,
    focus: FocusHandle,
    sort: Option<(usize, bool)>,
    selectable: bool,
    branch_motion: HashMap<SharedString, Slide>,
    viewport_width: Pixels,
    hide_below: Vec<f32>,
    list_state: ListState,
    list_ids: Vec<SharedString>,
}
impl EventEmitter<TreeTableEvent> for TreeTable {}
impl TreeTable {
    pub fn new(
        label: impl Into<SharedString>,
        columns: Vec<SharedString>,
        rows: Vec<TreeTableRow>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            label: label.into(),
            columns,
            rows,
            expanded: HashSet::new(),
            selected: HashSet::new(),
            loading: HashSet::new(),
            failed: HashSet::new(),
            focused: None,
            focus: cx.focus_handle().tab_stop(true),
            sort: None,
            selectable: true,
            branch_motion: HashMap::new(),
            viewport_width: px(0.),
            hide_below: Vec::new(),
            list_state: ListState::new(0, ListAlignment::Top, px(120.))
                .with_uniform_item_height(px(44.)),
            list_ids: Vec::new(),
        }
    }
    pub fn with_expanded(mut self, ids: &[&str]) -> Self {
        self.expanded = ids.iter().map(|id| SharedString::from(*id)).collect();
        self
    }
    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
    /// Minimum container width per column; the first column is always retained.
    pub fn with_hide_below(mut self, widths: Vec<f32>) -> Self {
        self.hide_below = widths;
        self
    }
    pub fn selected_ids(&self) -> Vec<SharedString> {
        let mut out: Vec<_> = self.selected.iter().cloned().collect();
        out.sort();
        out
    }
    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        if !self.selected.is_empty() {
            self.selected.clear();
            cx.emit(TreeTableEvent::SelectionChanged(Vec::new()));
            cx.notify();
        }
    }
    fn flatten(&self) -> Vec<Visible> {
        self.flatten_with_progress(None)
    }
    fn flatten_with_progress(&self, progress: Option<&HashMap<SharedString, f32>>) -> Vec<Visible> {
        #[allow(clippy::too_many_arguments)]
        fn visit(
            rows: &[TreeTableRow],
            depth: usize,
            parent: Option<SharedString>,
            sort: Option<(usize, bool)>,
            expanded: &HashSet<SharedString>,
            progress: Option<&HashMap<SharedString, f32>>,
            amount: f32,
            out: &mut Vec<Visible>,
        ) {
            let mut order: Vec<_> = rows.iter().collect();
            if let Some((column, ascending)) = sort {
                order.sort_by(|a, b| {
                    let left = if column == 0 {
                        a.name.to_lowercase()
                    } else {
                        a.cells
                            .get(column - 1)
                            .map_or_else(String::new, |x| x.to_lowercase())
                    };
                    let right = if column == 0 {
                        b.name.to_lowercase()
                    } else {
                        b.cells
                            .get(column - 1)
                            .map_or_else(String::new, |x| x.to_lowercase())
                    };
                    let num_l = left.trim_start_matches('$').replace(',', "").parse::<f64>();
                    let num_r = right
                        .trim_start_matches('$')
                        .replace(',', "")
                        .parse::<f64>();
                    let cmp = match (num_l, num_r) {
                        (Ok(x), Ok(y)) => x.total_cmp(&y),
                        _ => left.cmp(&right),
                    };
                    if ascending { cmp } else { cmp.reverse() }
                });
            }
            let siblings = order.len();
            for (position, row) in order.into_iter().enumerate() {
                out.push(Visible {
                    row: TreeTableRow {
                        id: row.id.clone(),
                        name: row.name.clone(),
                        cells: row.cells.clone(),
                        children: Vec::new(),
                        lazy: row.lazy || !row.children.is_empty(),
                    },
                    depth,
                    parent: parent.clone(),
                    amount,
                    position: position + 1,
                    siblings,
                });
                let branch = progress
                    .and_then(|p| p.get(&row.id))
                    .copied()
                    .unwrap_or(if expanded.contains(&row.id) { 1. } else { 0. });
                if branch > 0.001 {
                    visit(
                        &row.children,
                        depth + 1,
                        Some(row.id.clone()),
                        sort,
                        expanded,
                        progress,
                        amount * branch,
                        out,
                    );
                }
            }
        }
        let mut out = Vec::new();
        visit(
            &self.rows,
            0,
            None,
            self.sort,
            &self.expanded,
            progress,
            1.,
            &mut out,
        );
        out
    }
    fn row<'a>(rows: &'a [TreeTableRow], id: &str) -> Option<&'a TreeTableRow> {
        for row in rows {
            if row.id.as_ref() == id {
                return Some(row);
            }
            if let Some(child) = Self::row(&row.children, id) {
                return Some(child);
            }
        }
        None
    }
    fn row_mut<'a>(rows: &'a mut [TreeTableRow], id: &str) -> Option<&'a mut TreeTableRow> {
        for row in rows {
            if row.id.as_ref() == id {
                return Some(row);
            }
            if let Some(child) = Self::row_mut(&mut row.children, id) {
                return Some(child);
            }
        }
        None
    }
    fn path_to(rows: &[TreeTableRow], id: &str, path: &mut Vec<SharedString>) -> bool {
        for row in rows {
            path.push(row.id.clone());
            if row.id.as_ref() == id || Self::path_to(&row.children, id, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    fn leaves(row: &TreeTableRow, out: &mut Vec<SharedString>) {
        if row.children.is_empty() {
            out.push(row.id.clone())
        } else {
            for child in &row.children {
                Self::leaves(child, out)
            }
        }
    }
    fn state(&self, row: &TreeTableRow) -> (bool, bool) {
        let mut leaves = Vec::new();
        Self::leaves(row, &mut leaves);
        let n = leaves
            .iter()
            .filter(|id| self.selected.contains(*id))
            .count();
        (n > 0 && n == leaves.len(), n > 0 && n < leaves.len())
    }
    fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.selectable {
            return;
        }
        let Some(row) = Self::row(&self.rows, id) else {
            return;
        };
        let (all, _) = self.state(row);
        let mut leaves = Vec::new();
        Self::leaves(row, &mut leaves);
        for key in leaves {
            if all {
                self.selected.remove(&key);
            } else {
                self.selected.insert(key);
            }
        }
        let mut selected: Vec<_> = self.selected.iter().cloned().collect();
        selected.sort();
        cx.emit(TreeTableEvent::SelectionChanged(selected));
        cx.notify();
    }
    fn select_all(&mut self, cx: &mut Context<Self>) {
        if !self.selectable {
            return;
        }
        let mut leaves = Vec::new();
        for row in &self.rows {
            Self::leaves(row, &mut leaves);
        }
        let all = !leaves.is_empty() && leaves.iter().all(|id| self.selected.contains(id));
        for id in leaves {
            if all {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
        let mut selected: Vec<_> = self.selected.iter().cloned().collect();
        selected.sort();
        cx.emit(TreeTableEvent::SelectionChanged(selected));
        cx.notify();
    }
    fn expandable_ids(rows: &[TreeTableRow], out: &mut Vec<SharedString>) {
        for row in rows {
            if !row.children.is_empty() {
                out.push(row.id.clone());
                Self::expandable_ids(&row.children, out);
            }
        }
    }
    fn announce_expanded(&self, cx: &mut Context<Self>) {
        let mut ids: Vec<_> = self.expanded.iter().cloned().collect();
        ids.sort();
        cx.emit(TreeTableEvent::ExpandedChanged(ids));
    }
    fn toggle_all(&mut self, cx: &mut Context<Self>) {
        let mut ids = Vec::new();
        Self::expandable_ids(&self.rows, &mut ids);
        let close = !ids.is_empty() && ids.iter().all(|id| self.expanded.contains(id));
        for id in ids {
            if close {
                self.expanded.remove(&id);
            } else {
                self.expanded.insert(id.clone());
            }
            self.animate_branch(&id, !close, cx);
        }
        self.announce_expanded(cx);
        cx.notify();
    }
    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(row) = Self::row(&self.rows, id) else {
            return;
        };
        let was_open = self.expanded.contains(id);
        if row.lazy && row.children.is_empty() {
            if self.loading.contains(id) {
                return;
            }
            self.failed.remove(id);
            self.loading.insert(id.into());
            self.expanded.insert(id.into());
            self.animate_branch(id, true, cx);
            cx.emit(TreeTableEvent::LoadRequested(id.into()));
        } else if row.children.is_empty() {
            cx.emit(TreeTableEvent::Activated(id.into()));
        } else if self.expanded.remove(id) {
            self.animate_branch(id, false, cx);
        } else {
            self.expanded.insert(id.into());
            self.animate_branch(id, true, cx);
        }
        if was_open != self.expanded.contains(id) {
            self.announce_expanded(cx);
        }
        cx.notify();
    }
    fn animate_branch(&mut self, id: &str, open: bool, cx: &App) {
        let initial = if open { 0. } else { 1. };
        self.branch_motion
            .entry(id.into())
            .or_insert_with(|| Slide::snappy(initial))
            .duration(if open { 1. } else { 0. }, 0.4, cx);
    }
    /// Complete a lazy request. On error the row remains retryable.
    pub fn resolve_children(
        &mut self,
        id: &str,
        result: Result<Vec<TreeTableRow>, ()>,
        cx: &mut Context<Self>,
    ) {
        if !self.loading.remove(id) {
            return;
        }
        match result {
            Ok(children) => {
                let inherited = self.selected.remove(id);
                if let Some(row) = Self::row_mut(&mut self.rows, id) {
                    row.children = children;
                    row.lazy = false;
                    if inherited {
                        for child in &row.children {
                            let mut keys = Vec::new();
                            Self::leaves(child, &mut keys);
                            self.selected.extend(keys);
                        }
                    }
                }
                self.failed.remove(id);
            }
            Err(()) => {
                self.failed.insert(id.into());
            }
        }
        cx.notify();
    }
    fn sort_column(&mut self, column: usize, cx: &mut Context<Self>) {
        self.sort = match self.sort {
            Some((old, true)) if old == column => Some((column, false)),
            Some((old, false)) if old == column => None,
            _ => Some((column, true)),
        };
        cx.emit(TreeTableEvent::SortChanged(self.sort));
        cx.notify();
    }
    fn render_checkbox(checked: bool, mixed: bool, p: Paint) -> Div {
        let active = checked || mixed;
        div()
            .size(px(16.))
            .rounded(px(4.))
            .border_1()
            .border_color(if active { p.bright } else { p.faint })
            .bg(if active { p.bright } else { p.panel })
            .flex()
            .items_center()
            .justify_center()
            .child(check_stroke(
                if active { 1. } else { 0. },
                if mixed { 1. } else { 0. },
                12.,
                p.inverse,
            ))
    }
    fn render_row(
        &mut self,
        item: Visible,
        shown_columns: &[usize],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = Paint::get(cx);
        let row = item.row;
        let id = row.id.clone();
        let is_parent = !row.children.is_empty() || row.lazy;
        let open = self.expanded.contains(&id);
        let loading = self.loading.contains(&id);
        let failed = self.failed.contains(&id);
        let description = if failed {
            tr(
                cx,
                "Loading failed; activate to retry",
                "加载失败，激活后重试",
            )
        } else if loading {
            tr(cx, "Loading children", "正在加载子项")
        } else {
            ""
        };
        let (checked, mixed) =
            Self::row(&self.rows, &id).map_or((false, false), |original| self.state(original));
        let mut line = div()
            .id((ElementId::Name("tree-row".into()), id.clone()))
            .role(Role::Row)
            .aria_label(row.name.clone())
            .aria_description(description)
            .aria_selected(checked)
            .aria_level(item.depth + 1)
            .aria_position_in_set(item.position)
            .aria_size_of_set(item.siblings)
            .when(is_parent, |line| line.aria_expanded(open))
            .h(px(44. * item.amount))
            .opacity(item.amount)
            .overflow_hidden()
            .flex()
            .items_start()
            .border_b_1()
            .border_color(p.line)
            .when(checked, |e| e.bg(p.panel.blend(p.bright.opacity(0.10))))
            .when(!checked, |e| e.hover(move |s| s.bg(p.tray)))
            .when(checked, |e| {
                e.hover(move |s| s.bg(p.panel.blend(p.bright.opacity(0.16))))
            })
            .when(self.focused.as_ref() == Some(&id), |e| {
                e.bg(p.panel.blend(p.bright.opacity(0.16)))
                    .aria_active_descendant()
            });
        if self.selectable {
            let key = id.clone();
            let label: SharedString = format!("{} {}", tr(cx, "Select", "选择"), row.name).into();
            line = line.child(
                div()
                    .id((ElementId::Name("tree-check".into()), id.clone()))
                    .w(px(44.))
                    .h(px(44.))
                    .pl(px(6.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .role(Role::CheckBox)
                    .aria_label(label)
                    .aria_toggled(if mixed {
                        Toggled::Mixed
                    } else {
                        checked.into()
                    })
                    .cursor_pointer()
                    .child(Self::render_checkbox(checked, mixed, p))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.select(&key, cx);
                        cx.stop_propagation();
                    })),
            );
        }
        let indicator: AnyElement = if loading {
            spinner(0.25, 16., p.secondary).into_any_element()
        } else if failed {
            icon(Glyph::Alert, 14., p.danger).into_any_element()
        } else if is_parent {
            transformed(
                Glyph::Down,
                16.,
                if open { p.ink } else { p.faint },
                if open {
                    0.
                } else {
                    -std::f32::consts::FRAC_PI_2
                },
                1.,
                (0., 0.),
                1.,
            )
            .into_any_element()
        } else {
            div().size(px(16.)).into_any_element()
        };
        let mut tree_cell = div()
            .relative()
            .min_w(px(180.))
            .flex_1()
            .flex()
            .items_center()
            .h(px(44.))
            .gap(px(4.))
            .ml(px(10.))
            .pl(px(item.depth as f32 * 20.))
            .pr(px(12.));
        for d in 0..item.depth {
            tree_cell = tree_cell.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(d as f32 * 20. + 12.))
                    .w(px(1.))
                    .bg(p.line),
            );
        }
        tree_cell = tree_cell
            .child(
                div()
                    .size(px(24.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .when(is_parent, |d| {
                        d.hover(|s| s.bg(p.tray.blend(p.ink.opacity(0.06))))
                    })
                    .child(indicator),
            )
            .child(
                div()
                    .truncate()
                    .font_weight(if is_parent {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(row.name.clone()),
            );
        if failed {
            let retry_id = id.clone();
            let retry_label = tr(cx, "Retry", "重试");
            tree_cell = tree_cell.child(
                face(
                    (ElementId::Name("tree-retry".into()), id.clone()),
                    retry_label,
                    28.,
                    999.,
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .text_size(px(12.))
                .text_color(p.danger)
                .border_1()
                .border_color(p.danger.opacity(0.4))
                .child(retry_label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.toggle(&retry_id, cx);
                    cx.stop_propagation();
                })),
            );
        }
        line = line.child(tree_cell);
        for &i in shown_columns.iter().filter(|&&i| i > 0) {
            let cell_val = row.cells.get(i - 1).cloned().unwrap_or_default();
            line = line.child(
                div()
                    .min_w(px(100.))
                    .h(px(44.))
                    .px(px(12.))
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_end()
                    .truncate()
                    .text_color(p.secondary)
                    .child(cell_val),
            );
        }
        line = line
            .cursor_pointer()
            .on_click(cx.listener(move |s, _, w, cx| {
                s.focused = Some(id.clone());
                s.focus.focus(w, cx);
                s.toggle(&id, cx)
            }));
        line.into_any_element()
    }
    fn key(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let visible = self.flatten();
        if visible.is_empty() {
            return;
        }
        if e.keystroke.key == "a"
            && (e.keystroke.modifiers.platform || e.keystroke.modifiers.control)
            && self.selectable
        {
            self.select_all(cx);
            cx.stop_propagation();
            w.prevent_default();
            return;
        }
        let index = self
            .focused
            .as_ref()
            .and_then(|key| visible.iter().position(|r| r.row.id == *key))
            .unwrap_or(0);
        let current = &visible[index];
        let key = e.keystroke.key.as_str();
        let next = match key {
            "up" => Some(index.saturating_sub(1)),
            "down" => Some((index + 1).min(visible.len() - 1)),
            "home" => Some(0),
            "end" => Some(visible.len() - 1),
            "pageup" => Some(index.saturating_sub(10)),
            "pagedown" => Some((index + 10).min(visible.len() - 1)),
            "right" => {
                if (!current.row.children.is_empty() || current.row.lazy)
                    && !self.expanded.contains(&current.row.id)
                {
                    self.toggle(&current.row.id, cx);
                    None
                } else if index + 1 < visible.len()
                    && visible[index + 1].parent.as_ref() == Some(&current.row.id)
                {
                    Some(index + 1)
                } else {
                    None
                }
            }
            "left" => {
                if self.expanded.remove(&current.row.id) {
                    self.animate_branch(&current.row.id, false, cx);
                    self.announce_expanded(cx);
                    cx.notify();
                    None
                } else {
                    current
                        .parent
                        .as_ref()
                        .and_then(|p| visible.iter().position(|r| r.row.id == *p))
                }
            }
            "enter" => {
                self.toggle(&current.row.id, cx);
                None
            }
            "space" => {
                self.select(&current.row.id, cx);
                None
            }
            "*" => {
                let siblings: Vec<_> = visible
                    .iter()
                    .filter(|item| {
                        item.parent == current.parent
                            && item.row.lazy
                            && !self.expanded.contains(&item.row.id)
                    })
                    .map(|item| item.row.id.clone())
                    .collect();
                for id in siblings {
                    self.toggle(&id, cx);
                }
                None
            }
            _ => return,
        };
        if let Some(i) = next {
            self.list_state.scroll_to_reveal_item(i);
            self.focused = Some(visible[i].row.id.clone());
            cx.notify();
        }
        cx.stop_propagation();
        w.prevent_default();
    }
}
impl Render for TreeTable {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let width = if self.viewport_width > px(0.) {
            f32::from(self.viewport_width)
        } else {
            f32::from(w.viewport_size().width)
        };
        let shown_columns: Vec<usize> = (0..self.columns.len())
            .filter(|&i| {
                i == 0
                    || width
                        >= self.hide_below.get(i).copied().unwrap_or(if i == 1 {
                            420.
                        } else {
                            560.
                        })
            })
            .collect();
        let content_width = 180.
            + shown_columns.len().saturating_sub(1) as f32 * 100.
            + if self.selectable { 44. } else { 0. };
        let progress: HashMap<SharedString, f32> = self
            .branch_motion
            .iter()
            .map(|(id, slide)| (id.clone(), slide.get(w, cx).clamp(0., 1.)))
            .collect();
        let mut header = div()
            .flex()
            .h(px(40.))
            .items_center()
            .border_b_1()
            .border_color(p.line)
            .bg(p.tray.opacity(0.4))
            .text_size(px(12.))
            .text_color(p.secondary);
        if self.selectable {
            let mut leaves = Vec::new();
            for row in &self.rows {
                Self::leaves(row, &mut leaves);
            }
            let checked = !leaves.is_empty() && leaves.iter().all(|id| self.selected.contains(id));
            let mixed = !checked && leaves.iter().any(|id| self.selected.contains(id));
            let label = tr(cx, "Select all rows", "选择全部行");
            header = header.child(
                div()
                    .id("tree-select-all")
                    .w(px(44.))
                    .h_full()
                    .pl(px(6.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .role(Role::CheckBox)
                    .aria_label(label)
                    .aria_toggled(if mixed {
                        Toggled::Mixed
                    } else {
                        checked.into()
                    })
                    .tab_index(0)
                    .cursor_pointer()
                    .child(Self::render_checkbox(checked, mixed, p))
                    .on_click(cx.listener(|s, _, _, cx| s.select_all(cx)))
                    .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            s.select_all(cx);
                            cx.stop_propagation();
                            w.prevent_default();
                        }
                    })),
            );
        }
        let mut expandable = Vec::new();
        Self::expandable_ids(&self.rows, &mut expandable);
        let all_open = expandable.iter().all(|id| self.expanded.contains(id));
        for &i in &shown_columns {
            let col = &self.columns[i];
            let sorted = self.sort.filter(|(idx, _)| *idx == i);
            let arrow = match sorted {
                Some((_, true)) => " ↑",
                Some((_, false)) => " ↓",
                None => "",
            };
            let mut cell = div()
                .id(("tree-sort", i))
                .role(Role::ColumnHeader)
                .tab_index(0)
                .cursor_pointer()
                .h_full()
                .min_w(px(if i == 0 { 180. } else { 100. }))
                .flex_1()
                .flex()
                .items_center()
                .gap(px(2.))
                .px(px(12.))
                .when(i == 0, |d| d.pl(px(8.)))
                .when(i > 0, |d| d.justify_end())
                .text_size(px(12.))
                .text_color(if sorted.is_some() { p.ink } else { p.secondary });
            if i == 0 && !expandable.is_empty() {
                let label = if all_open {
                    tr(cx, "Collapse all", "全部收起")
                } else {
                    tr(cx, "Expand all", "全部展开")
                };
                cell = cell.child(
                    div()
                        .id("tree-expand-all")
                        .w(px(28.))
                        .h(px(28.))
                        .rounded(px(8.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .role(Role::Button)
                        .aria_label(label)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.tray))
                        .child(transformed(
                            Glyph::Down,
                            16.,
                            p.secondary,
                            if all_open {
                                0.
                            } else {
                                -std::f32::consts::FRAC_PI_2
                            },
                            1.,
                            (0., 0.),
                            1.,
                        ))
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.toggle_all(cx);
                            cx.stop_propagation();
                        })),
                );
            } else if i == 0 {
                cell = cell.child(div().w(px(24.)).mr(px(4.)).flex_none());
            }
            header = header.child(
                cell.child(
                    div()
                        .h(px(28.))
                        .ml(px(-8.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded(px(8.))
                        .hover(|s| s.bg(p.tray).text_color(p.ink))
                        .child(format!("{}{}", col, arrow)),
                )
                .on_click(cx.listener(move |s, _, _, cx| s.sort_column(i, cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        s.sort_column(i, cx);
                        cx.stop_propagation();
                        w.prevent_default();
                    }
                })),
            );
        }
        let visible = self.flatten_with_progress(Some(&progress));
        let ids: Vec<_> = visible.iter().map(|item| item.row.id.clone()).collect();
        if let Some(current) = self.focused.as_ref()
            && !ids.contains(current)
        {
            let mut path = Vec::new();
            if Self::path_to(&self.rows, current, &mut path) {
                self.focused = path.into_iter().rev().find(|id| ids.contains(id));
            } else {
                self.focused = ids.first().cloned();
            }
        }
        if ids != self.list_ids {
            let prefix = ids
                .iter()
                .zip(&self.list_ids)
                .take_while(|(a, b)| a == b)
                .count();
            let suffix = ids[prefix..]
                .iter()
                .rev()
                .zip(self.list_ids[prefix..].iter().rev())
                .take_while(|(a, b)| a == b)
                .count();
            self.list_state.splice(
                prefix..self.list_ids.len() - suffix,
                ids.len() - prefix - suffix,
            );
            self.list_ids = ids;
            let _ = self.list_state.clone().with_uniform_item_height(px(44.));
        }
        if let (Some(first), Some(last)) = (
            visible.iter().position(|item| item.amount < 0.999),
            visible.iter().rposition(|item| item.amount < 0.999),
        ) {
            self.list_state.remeasure_items(first..last + 1);
        }
        let body = if visible.is_empty() {
            div()
                .p(px(16.))
                .text_color(p.faint)
                .child(tr(cx, "Nothing to show", "暂无内容"))
                .into_any_element()
        } else {
            let height = (visible.len() as f32 * 44.).clamp(44., 350.);
            let this = cx.entity();
            let columns = shown_columns.clone();
            let scroll = self.list_state.clone();
            let list = list(self.list_state.clone(), move |index, _, cx| {
                this.update(cx, |table, cx| {
                    table.render_row(visible[index].clone(), &columns, cx)
                })
            })
            .h(px(height))
            .w_full();
            let monitor = canvas(
                |_, _, _| (),
                move |bounds, _, _, cx| {
                    if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                        let top = scroll.logical_scroll_top();
                        let before = top.item_ix > 0 || top.offset_in_item > px(0.1);
                        let after = scroll.is_scrolled_to_end().map(|end| !end).unwrap_or(false);
                        (host.0)(bounds, before, after);
                    }
                },
            )
            .absolute()
            .inset_0();
            div()
                .relative()
                .w_full()
                .h(px(height))
                .child(list)
                .child(monitor)
                .into_any_element()
        };
        let weak = cx.weak_entity();
        let previous_width = self.viewport_width;
        div()
            .id("tree-table")
            .relative()
            .role(Role::TreeGrid)
            .aria_label(self.label.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .on_key_down(cx.listener(Self::key))
            .w_full()
            .max_w_full()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if (bounds.size.width - previous_width).abs() > px(0.1) {
                            let _ = weak.update(cx, |table, cx| {
                                table.viewport_width = bounds.size.width;
                                cx.notify();
                            });
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .id("tree-table-scroll")
                    .overflow_x_scroll()
                    .child(div().min_w(px(content_width)).child(header).child(body)),
            )
    }
}
