//! Choose and run an assistant command against a selection.
use crate::primitives::brand_mark::brand_mark;
use crate::primitives::ops_surface::*;
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::Paint,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
pub struct Command {
    pub id: String,
    pub label: String,
    pub section: String,
    pub hint: String,
}
pub enum CommandEvent {
    Open(String),
    Ask { ticket: u64, query: String },
    Cancel,
}
pub struct CommandDesk {
    id: SharedString,
    pub commands: Vec<Command>,
    ai: bool,
    edit: Entity<Editor>,
    _sub: Subscription,
    cursor: usize,
    ticket: u64,
    answer: String,
    waiting: bool,
    failed: bool,
    model: Option<SharedString>,
}
impl EventEmitter<CommandEvent> for CommandDesk {}
impl CommandDesk {
    pub fn new(
        id: impl Into<SharedString>,
        commands: Vec<Command>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let edit = editor(
            tr(cx, "Search or ask a question…", "搜索或提问…").into(),
            InputMode::Combobox,
            w,
            cx,
        );
        let sub = cx.subscribe_in(&edit, w, |s: &mut Self, _, e: &EditEvent, _, cx| {
            match e {
                EditEvent::Submitted => s.submit(cx),
                EditEvent::Navigate(d) => {
                    let n = s.filtered(cx).len();
                    s.cursor = if *d {
                        (s.cursor + 1).min(n.saturating_sub(1))
                    } else {
                        s.cursor.saturating_sub(1)
                    }
                }
                EditEvent::Changed => s.cursor = 0,
                EditEvent::Escape => {
                    if s.ai {
                        s.mode(false, cx);
                    } else {
                        s.ticket += 1;
                        s.waiting = false;
                        cx.emit(CommandEvent::Cancel)
                    }
                }
                _ => {}
            }
            cx.notify()
        });
        Self {
            id: id.into(),
            commands,
            ai: false,
            edit,
            _sub: sub,
            cursor: 0,
            ticket: 0,
            answer: String::new(),
            waiting: false,
            failed: false,
            model: None,
        }
    }
    /// Display the model used to answer questions.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    fn filtered(&self, cx: &App) -> Vec<usize> {
        let q = self.edit.read(cx).value().to_lowercase();
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.label.to_lowercase().contains(&q) || c.section.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.ai {
            let q = self.edit.read(cx).value().trim().to_string();
            if !q.is_empty() && !self.waiting {
                self.ticket += 1;
                self.waiting = true;
                self.failed = false;
                self.answer.clear();
                cx.emit(CommandEvent::Ask {
                    ticket: self.ticket,
                    query: q,
                })
            }
        } else if let Some(i) = self.filtered(cx).get(self.cursor) {
            cx.emit(CommandEvent::Open(self.commands[*i].id.clone()))
        }
        cx.notify()
    }
    pub fn receive(
        &mut self,
        ticket: u64,
        result: Result<String, ()>,
        done: bool,
        cx: &mut Context<Self>,
    ) {
        if ticket != self.ticket || !self.waiting {
            return;
        }
        match result {
            Ok(s) => self.answer = s,
            Err(()) => self.failed = true,
        }
        self.waiting = !done && !self.failed;
        cx.notify()
    }
    fn mode(&mut self, ai: bool, cx: &mut Context<Self>) {
        if self.ai != ai {
            if self.waiting {
                cx.emit(CommandEvent::Cancel);
            }
            self.ai = ai;
            self.ticket += 1;
            self.waiting = false;
            cx.notify()
        }
    }
}
impl Render for CommandDesk {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = panel(&self.id, 522., cx)
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "tab"
                    && !e.keystroke.modifiers.shift
                    && s.edit.read(cx).focus.is_focused(w)
                {
                    s.mode(!s.ai, cx);
                    if s.ai && !s.edit.read(cx).value().trim().is_empty() {
                        s.submit(cx);
                    }
                    w.prevent_default();
                    cx.stop_propagation()
                }
            }))
            .child(
                row()
                    .child(icon(Glyph::Search, 17., p.secondary))
                    .child(div().flex_1().min_w_0().child(input(&self.edit, cx)))
                    .flex_wrap()
                    .child(btn(
                        "search",
                        tr(cx, "Search", "搜索"),
                        !self.ai,
                        cx,
                        |s, _, cx| s.mode(false, cx),
                    ))
                    .child(btn(
                        "ai",
                        tr(cx, "Ask AI", "向 AI 提问"),
                        self.ai,
                        cx,
                        |s, _, cx| s.mode(true, cx),
                    )),
            )
            .child(divider(cx));
        let amount = animated_value("command-mode", f32::from(self.ai), 0.16, w, cx);
        let mut panes = div().relative().w_full().h(px(296.)).overflow_hidden();
        if self.ai || amount > 0.01 {
            let status = if self.failed {
                tr(cx, "Request failed", "请求失败")
            } else if self.waiting {
                tr(cx, "Generating…", "生成中…")
            } else {
                tr(cx, "Ask about your workspace", "询问工作区相关问题")
            };
            let answer = column()
                .absolute()
                .inset_0()
                .top(px(2. * (1. - amount)))
                .opacity(amount)
                .child(note(status, cx))
                .child(
                    StyledText::new(self.answer.clone()).with_highlights(stream_highlights(
                        "answer-chunks",
                        &self.answer,
                        w,
                        cx,
                    )),
                )
                .child(btn(
                    "ask",
                    if self.failed {
                        tr(cx, "Retry", "重试")
                    } else {
                        tr(cx, "Ask", "提问")
                    },
                    true,
                    cx,
                    |s, _, cx| s.submit(cx),
                ));
            panes = panes.child(answer);
        }
        if !self.ai || amount < 0.99 {
            let mut rows = column()
                .id("commands")
                .max_h(px(300.))
                .overflow_y_scroll()
                .gap_0();
            let mut section = "";
            let items = self.filtered(cx);
            if items.is_empty() {
                rows = rows.child(note(tr(cx, "No commands found", "未找到命令"), cx))
            }
            for (pos, i) in items.into_iter().enumerate() {
                let c = &self.commands[i];
                if c.section != section {
                    section = &c.section;
                    rows = rows.child(note(c.section.clone(), cx).mt(px(5.)))
                }
                let id = c.id.clone();
                rows = rows.child(
                    item(("cmd", i), c.label.clone(), cx, move |s, _, cx| {
                        s.cursor = pos;
                        cx.emit(CommandEvent::Open(id.clone()));
                        cx.notify()
                    })
                    .when(self.cursor == pos, |e| e.bg(p.stage))
                    .child(div().flex_1().child(c.label.clone()))
                    .child(note(c.hint.clone(), cx)),
                )
            }
            panes = panes.child(div().absolute().inset_0().opacity(1. - amount).child(rows))
        }
        r.child(panes).child(
            row()
                .justify_between()
                .flex_wrap()
                .child(
                    row()
                        .gap(px(6.))
                        .when_some(self.model.clone(), |row, model| {
                            row.when_some(brand_mark(&model, 14., cx), |row, mark| row.child(mark))
                                .child(note(model, cx))
                        }),
                )
                .child(note(
                    tr(
                        cx,
                        "↑↓ Navigate · ↵ Open · Tab Ask AI",
                        "↑↓ 选择 · ↵ 打开 · Tab 切换 AI",
                    ),
                    cx,
                )),
        )
    }
}
