//! Browse references associated with an answer and inspect a source.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    brand_mark::brand_mark,
    family::Paint,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct Citation {
    pub domain: String,
    pub title: String,
    pub passage: String,
    pub url: String,
}

fn site_mark(domain: &str, cx: &App) -> Option<AnyElement> {
    let provider = match domain.trim_start_matches("www.") {
        "stripe.com" => "Stripe",
        "github.com" => "GitHub",
        "vercel.com" => "Vercel",
        "linear.app" => "Linear",
        "figma.com" => "Figma",
        _ => return None,
    };
    brand_mark(provider, 15., cx)
}
pub enum SearchState {
    Searching,
    Ready,
    Failed,
    Empty,
}
pub enum SourceEvent {
    Open(String),
    Retry,
}
pub struct CitationShelf {
    id: SharedString,
    pub query: String,
    pub sources: Vec<Citation>,
    pub state: SearchState,
    all: bool,
    selected: Option<usize>,
    hover: Option<usize>,
    intent: Option<usize>,
    previous: Vec<Citation>,
}
impl EventEmitter<SourceEvent> for CitationShelf {}
impl CitationShelf {
    pub fn new(
        id: impl Into<SharedString>,
        query: impl Into<String>,
        sources: Vec<Citation>,
    ) -> Self {
        Self {
            id: id.into(),
            query: query.into(),
            sources,
            state: SearchState::Ready,
            all: false,
            selected: None,
            hover: None,
            intent: None,
            previous: vec![],
        }
    }
    pub fn cite(&mut self, i: usize, cx: &mut Context<Self>) {
        if i < self.sources.len() {
            self.selected = Some(i);
            self.all |= i >= 3;
            cx.notify()
        }
    }
}
impl Render for CitationShelf {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        if !self.sources.is_empty() {
            self.previous.clone_from(&self.sources)
        }
        let grid_fold = Fold::get("source-grid", !self.sources.is_empty(), 180., 0.21, w, cx);
        if !grid_fold.mounted() {
            self.previous.clear()
        }
        let shown = if self.sources.is_empty() {
            &self.previous
        } else {
            &self.sources
        };
        let current = self.hover.or(self.selected);
        let n = if self.all { shown.len() } else { 3 };
        let state = match self.state {
            SearchState::Searching => tr(cx, "Searching", "搜索中"),
            SearchState::Ready => tr(cx, "Sources", "引用来源"),
            SearchState::Failed => tr(cx, "Search failed", "搜索失败"),
            SearchState::Empty => tr(cx, "No results", "无搜索结果"),
        };
        let mut cards: Vec<AnyElement> = vec![];
        for (i, c) in shown.iter().enumerate().take(n) {
            cards.push(
                item(("citation", i), c.title.clone(), cx, move |s, _, cx| {
                    s.cite(i, cx)
                })
                .on_hover(cx.listener(move |s, over, _, cx| {
                    if *over {
                        s.intent = Some(i);
                        if s.hover.is_some() {
                            s.hover = Some(i)
                        } else {
                            cx.spawn(async move |weak, cx| {
                                cx.background_executor()
                                    .timer(std::time::Duration::from_millis(320))
                                    .await;
                                let _ = weak.update(cx, |s, cx| {
                                    if s.intent == Some(i) {
                                        s.hover = Some(i);
                                        cx.notify()
                                    }
                                });
                            })
                            .detach();
                        }
                    } else if s.intent == Some(i) {
                        s.intent = None;
                        s.hover = None
                    }
                    cx.notify()
                }))
                .flex_1()
                .min_w_0()
                .min_h(px(84.))
                .flex_col()
                .items_start()
                .p(px(12.))
                .border_1()
                .border_color(if current == Some(i) { p.ink } else { p.line })
                .child(
                    row()
                        .w_full()
                        .child(site_mark(&c.domain, cx).unwrap_or_else(|| {
                            avatar(&c.domain, i, cx).size(px(20.)).into_any_element()
                        }))
                        .child(note(c.domain.clone(), cx).flex_1().truncate())
                        .child(badge((i + 1).to_string(), cx)),
                )
                .child(div().child(c.title.clone()))
                .into_any_element(),
            );
        }
        if shown.len() > 3 {
            cards.push(
                item(
                    "more",
                    tr(cx, "More sources", "更多来源"),
                    cx,
                    |s, _, cx| {
                        s.all = !s.all;
                        cx.notify()
                    },
                )
                .flex_1()
                .min_w_0()
                .min_h(px(84.))
                .p(px(12.))
                .flex_col()
                .items_start()
                .border_1()
                .border_color(p.line)
                .when(!self.all, |card| {
                    card.child(row().gap(px(4.)).children(
                        shown.iter().enumerate().skip(3).take(3).map(|(i, source)| {
                            site_mark(&source.domain, cx).unwrap_or_else(|| {
                                avatar(&source.domain, i, cx)
                                    .size(px(16.))
                                    .into_any_element()
                            })
                        }),
                    ))
                })
                .child(title(if self.all {
                    tr(cx, "Show less", "收起").to_string()
                } else {
                    format!("+{} {}", shown.len() - 3, tr(cx, "more", "个来源"))
                }))
                .when(!self.all, |card| {
                    card.child(
                        note(
                            shown
                                .iter()
                                .skip(3)
                                .map(|source| source.domain.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                            cx,
                        )
                        .w_full()
                        .truncate(),
                    )
                })
                .into_any_element(),
            )
        }
        let mut grid = column();
        let mut line = row().items_stretch();
        let cols = if w.viewport_size().width < px(380.) {
            1
        } else {
            2
        };
        let total = cards.len();
        for (i, card) in cards.into_iter().enumerate() {
            line = line.child(card);
            if (i + 1) % cols == 0 || i + 1 == total {
                grid = grid.child(line);
                line = row().items_stretch()
            }
        }
        panel(&self.id, 522., cx)
            .p(px(24.))
            .child(title(self.query.clone()).text_size(px(16.)))
            .child(
                row()
                    .min_w_0()
                    .gap(px(6.))
                    .when(matches!(self.state, SearchState::Searching), |row| {
                        row.child(icon(Glyph::Search, 14., p.secondary))
                    })
                    .child(note(state, cx))
                    .child(badge(self.sources.len().to_string(), cx))
                    .when(
                        matches!(self.state, SearchState::Searching) && !self.sources.is_empty(),
                        |row| row.child(note(self.query.clone(), cx).flex_1().min_w_0().truncate()),
                    ),
            )
            .when(grid_fold.mounted(), |v| v.child(grid_fold.wrap(12., grid)))
            .when_some(current.and_then(|i| self.sources.get(i)), |v, c| {
                let url = c.url.clone();
                v.child(
                    column()
                        .p(px(10.))
                        .bg(p.stage)
                        .rounded(radius)
                        .child(title(c.title.clone()))
                        .child(note(c.passage.clone(), cx))
                        .child(btn(
                            "open",
                            tr(cx, "Open source", "打开来源"),
                            false,
                            cx,
                            move |_, _, cx| cx.emit(SourceEvent::Open(url.clone())),
                        )),
                )
            })
            .when(matches!(self.state, SearchState::Failed), |r| {
                r.child(btn(
                    "retry",
                    tr(cx, "Retry", "重试"),
                    false,
                    cx,
                    |s, _, cx| {
                        s.state = SearchState::Searching;
                        cx.emit(SourceEvent::Retry);
                        cx.notify()
                    },
                ))
            })
    }
}
