//! Stable text or numeric sorting with row identities preserved.
use crate::primitives::{
    family::{Paint, Slide},
    glyph::{Glyph, icon, transformed},
    nav::*,
};
use gpui::{prelude::*, *};
pub struct TableColumn {
    pub name: SharedString,
    pub numeric: bool,
}
impl TableColumn {
    pub fn new(name: impl Into<SharedString>, numeric: bool) -> Self {
        Self {
            name: name.into(),
            numeric,
        }
    }
}
pub struct TableSorted {
    pub column: usize,
    pub descending: bool,
}
pub struct TableSelection(pub Vec<usize>);
impl EventEmitter<TableSelection> for SortableTable {}
pub struct SortableTable {
    hscroll: ScrollHandle,
    selected: std::collections::BTreeSet<usize>,
    positions: Vec<Slide>,
    arrows: Vec<Slide>,
    title: SharedString,
    columns: Vec<TableColumn>,
    rows: Vec<Vec<SharedString>>,
    order: Vec<usize>,
    sort: Option<(usize, bool)>,
}
impl EventEmitter<TableSorted> for SortableTable {}
impl SortableTable {
    pub fn new(
        title: impl Into<SharedString>,
        columns: Vec<TableColumn>,
        rows: Vec<Vec<SharedString>>,
    ) -> Self {
        Self {
            title: title.into(),
            arrows: vec![Slide::new(0.); columns.len()],
            columns,
            order: (0..rows.len()).collect(),
            positions: (0..rows.len()).map(|i| Slide::new(i as f32)).collect(),
            selected: Default::default(),
            hscroll: ScrollHandle::new(),
            rows,
            sort: None,
        }
    }
    fn check(
        &self,
        id: impl Into<ElementId>,
        name: &'static str,
        checked: bool,
        cx: &App,
    ) -> Stateful<Div> {
        let p = Paint::get(cx);
        button(id, name, cx).size(px(32.)).p_0().child(
            row()
                .size(px(15.))
                .justify_center()
                .rounded(crate::foundation::surface_radius(cx))
                .border_1()
                .border_color(p.line)
                .when(checked, |b| {
                    b.bg(p.bright).child(icon(Glyph::Check, 11., p.inverse))
                }),
        )
    }
    pub fn sort(&mut self, column: usize, descending: bool, cx: &mut Context<Self>) {
        let Some(col) = self.columns.get(column) else {
            return;
        };
        let rows = &self.rows;
        self.order.sort_by(|a, b| {
            let lhs = rows[*a].get(column).map(SharedString::as_ref).unwrap_or("");
            let rhs = rows[*b].get(column).map(SharedString::as_ref).unwrap_or("");
            let cmp = if col.numeric {
                match (lhs.parse::<f64>(), rhs.parse::<f64>()) {
                    (Ok(x), Ok(y)) => x.total_cmp(&y),
                    _ => lhs.cmp(rhs),
                }
            } else {
                lhs.cmp(rhs)
            };
            if descending { cmp.reverse() } else { cmp }
        });
        for (i, r) in self.order.iter().enumerate() {
            self.positions[*r].duration(i as f32, 0.28, cx)
        }
        self.arrows[column].duration(if descending { 1. } else { 0. }, 0.18, cx);
        self.sort = Some((column, descending));
        cx.emit(TableSorted { column, descending });
        cx.notify()
    }
}
impl Render for SortableTable {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let width = fit(w, 680.);
        let all = !self.rows.is_empty() && self.selected.len() == self.rows.len();
        let mut head = row().bg(p.stage).child(
            self.check("all", tr(cx, "Select all rows", "选择所有行"), all, cx)
                .on_click(cx.listener(|s, _, _, cx| {
                    if s.selected.len() == s.rows.len() {
                        s.selected.clear()
                    } else {
                        s.selected.extend(0..s.rows.len())
                    }
                    cx.emit(TableSelection(s.selected.iter().copied().collect()));
                    cx.notify()
                })),
        );

        for (i, c) in self.columns.iter().enumerate() {
            head = head.child(
                button(("heading", i), c.name.clone(), cx)
                    .flex_1()
                    .min_w(px(80.))
                    .justify_start()
                    .child(c.name.clone())
                    .when(self.sort.is_some_and(|(k, _)| k == i), |b| {
                        b.child(transformed(
                            Glyph::Up,
                            12.,
                            p.faint,
                            self.arrows[i].get(w, cx) * std::f32::consts::PI,
                            1.,
                            (0., 0.),
                            1.,
                        ))
                    })
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.sort(i, s.sort.is_some_and(|(col, desc)| col == i && !desc), cx)
                    })),
            )
        }
        let mut table = column()
            .min_w(px(self.columns.len() as f32 * 100.))
            .child(head);
        for (at, &index) in self.order.iter().enumerate() {
            let offset = 40. * (self.positions[index].get(w, cx) - at as f32);
            let selected = self.selected.contains(&index);
            let check = self
                .check(
                    ("select", index),
                    tr(cx, "Select row", "选择行"),
                    selected,
                    cx,
                )
                .on_click(cx.listener(move |s, _, _, cx| {
                    if !s.selected.remove(&index) {
                        s.selected.insert(index);
                    }
                    cx.emit(TableSelection(s.selected.iter().copied().collect()));
                    cx.notify()
                }));
            let mut line = row()
                .relative()
                .top(px(offset))
                .h(px(40.))
                .bg(if selected { p.stage } else { p.panel })
                .border_t_1()
                .border_color(p.line)
                .child(check);
            for i in 0..self.columns.len() {
                line = line.child(
                    label(
                        self.rows[index].get(i).cloned().unwrap_or_default(),
                        12.,
                        p.ink,
                    )
                    .px_2()
                    .truncate()
                    .flex_1()
                    .min_w(px(80.)),
                )
            }
            table = table.child(line)
        }
        panel(cx)
            .w(width)
            .child(label(self.title.clone(), 15., p.ink).px_3().py_2())
            .child(horizontal_viewport("table-scroll", &self.hscroll, table))
    }
}
