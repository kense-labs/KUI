//! Start a new conversation from a short list of suggested questions.
use crate::primitives::editor::{EditEvent, Editor, InputMode};
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct Prompt {
    pub group: String,
    pub app: String,
    pub title: String,
    pub detail: String,
    pub text: String,
}
pub struct PromptShelf {
    id: SharedString,
    pub greeting: String,
    pub prompts: Vec<Prompt>,
    filter: Option<String>,
    offset: usize,
    draft: Entity<Editor>,
    _sub: Subscription,
}
pub struct PromptSent(pub String);
impl EventEmitter<PromptSent> for PromptShelf {}
impl PromptShelf {
    pub fn new(
        id: impl Into<SharedString>,
        greeting: impl Into<String>,
        prompts: Vec<Prompt>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = editor(
            tr(cx, "Write your own prompt", "输入你的问题").into(),
            InputMode::Single,
            w,
            cx,
        );
        let sub = cx.subscribe_in(&draft, w, |s: &mut Self, _, e: &EditEvent, w, cx| {
            if *e == EditEvent::Submitted {
                s.send(w, cx)
            }
        });
        Self {
            id: id.into(),
            greeting: greeting.into(),
            prompts,
            filter: None,
            offset: 0,
            draft,
            _sub: sub,
        }
    }
    fn send(&mut self, w: &Window, cx: &mut Context<Self>) {
        let text = self.draft.read(cx).value().trim().to_owned();
        if !text.is_empty() {
            cx.emit(PromptSent(text));
            replace(&self.draft, "", w, cx)
        }
    }
}
impl Render for PromptShelf {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut groups = vec![None];
        for p in &self.prompts {
            let g = Some(p.group.clone());
            if !groups.contains(&g) {
                groups.push(g)
            }
        }
        let items: Vec<_> = self
            .prompts
            .iter()
            .filter(|p| self.filter.as_ref().is_none_or(|g| g == &p.group))
            .collect();
        let available = (f32::from(w.viewport_size().width) - 100.).clamp(280., 680.);
        let cols = usize::from(available > 560.) + 1;
        let ordered: Vec<_> = (0..items.len().min(4))
            .map(|i| items[(i + self.offset) % items.len()])
            .collect();
        let mut grid = column();
        for (r, chunk) in ordered.chunks(cols).enumerate() {
            let mut line = row().items_stretch();
            for (i, p) in chunk.iter().enumerate() {
                let text = p.text.clone();
                let delay = (r * cols + i) as f32 * 0.035;
                let card = item(
                    ("prompt", r * cols + i),
                    p.title.clone(),
                    cx,
                    move |s, w, cx| {
                        replace(&s.draft, text.clone(), w, cx);
                        s.draft.read(cx).focus.clone().focus(w, cx)
                    },
                )
                .flex_1()
                .flex_col()
                .items_start()
                .p(px(12.))
                .border_1()
                .border_color(crate::primitives::family::Paint::get(cx).line)
                .child(
                    row()
                        .child(badge(p.group.clone(), cx))
                        .child(swap_label_after(
                            ("prompt-app", r * cols + i),
                            p.app.clone(),
                            delay,
                            w,
                            cx,
                        )),
                )
                .child(
                    swap_label_after(
                        ("prompt-title", r * cols + i),
                        p.title.clone(),
                        delay,
                        w,
                        cx,
                    )
                    .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    swap_label_after(
                        ("prompt-copy", r * cols + i),
                        p.detail.clone(),
                        delay,
                        w,
                        cx,
                    )
                    .text_size(px(12.))
                    .text_color(crate::primitives::family::Paint::get(cx).secondary),
                );
                line = line.child(card);
            }
            grid = grid.child(line)
        }
        panel(&self.id, available, cx)
            .child(title(self.greeting.clone()).text_size(px(21.)))
            .child(note(
                tr(
                    cx,
                    "Choose a starter or write your own.",
                    "选一个起点，或自己输入问题。",
                ),
                cx,
            ))
            .child(
                row()
                    .flex_wrap()
                    .children(groups.into_iter().enumerate().map(|(i, g)| {
                        btn(
                            ("group", i),
                            g.clone().unwrap_or_else(|| tr(cx, "All", "全部").into()),
                            self.filter == g,
                            cx,
                            move |s, _, cx| {
                                if s.filter == g {
                                    return;
                                }
                                s.filter = g.clone();
                                s.offset = 0;
                                cx.notify()
                            },
                        )
                    }))
                    .child(btn(
                        "shuffle",
                        tr(cx, "Shuffle", "换一组"),
                        false,
                        cx,
                        |s, _, cx| {
                            s.offset = s.offset.wrapping_add(1);
                            cx.notify()
                        },
                    )),
            )
            .child(grid)
            .child(row().child(input(&self.draft, cx)).child(btn(
                "submit",
                tr(cx, "Send", "发送"),
                true,
                cx,
                |s, w, cx| s.send(w, cx),
            )))
    }
}
