//! Read a conversation and append a new message in sequence.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::Paint,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct Message {
    pub author: String,
    pub time: String,
    pub text: String,
    pub own: bool,
    pub attachment: Option<String>,
    pub reactions: u32,
}
pub enum ChatEvent {
    Send(String),
    Attach,
    Download(usize),
    React(usize),
}
pub struct ChatThread {
    id: SharedString,
    pub topic: String,
    pub messages: Vec<Message>,
    pub online: bool,
    pub typing: Option<String>,
    scroll: ScrollHandle,
    previous_len: usize,
    composer: Entity<Editor>,
    _sub: Subscription,
}
impl EventEmitter<ChatEvent> for ChatThread {}
impl ChatThread {
    pub fn new(
        id: impl Into<SharedString>,
        topic: impl Into<String>,
        messages: Vec<Message>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let composer = editor(
            tr(cx, "Write a message…", "输入消息…").into(),
            InputMode::Single,
            w,
            cx,
        );
        let sub = cx.subscribe_in(&composer, w, |v: &mut Self, _, ev: &EditEvent, w, cx| {
            if *ev == EditEvent::Submitted {
                v.send(w, cx)
            }
        });
        Self {
            id: id.into(),
            topic: topic.into(),
            previous_len: messages.len(),
            scroll: ScrollHandle::new(),
            messages,
            online: true,
            typing: None,
            composer,
            _sub: sub,
        }
    }
    fn send(&mut self, w: &Window, cx: &mut Context<Self>) {
        let s = self.composer.read(cx).value().trim().to_owned();
        if self.online && !s.is_empty() {
            replace(&self.composer, "", w, cx);
            cx.emit(ChatEvent::Send(s))
        }
    }
}
impl Render for ChatThread {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        let mut names = vec![];
        for m in &self.messages {
            if !m.own && !names.contains(&m.author) {
                names.push(m.author.clone())
            }
        }
        if self.messages.len() != self.previous_len {
            if self.messages.last().is_some_and(|m| m.own)
                || self.scroll.max_offset().y + self.scroll.offset().y < px(24.)
            {
                self.scroll.scroll_to_bottom();
            }
            self.previous_len = self.messages.len();
        }
        let log = column()
            .id("messages")
            .track_scroll(&self.scroll)
            .max_h(px(300.))
            .overflow_y_scroll()
            .gap(px(12.))
            .children(self.messages.iter().enumerate().map(|(i, m)| {
                let same = i > 0 && self.messages[i - 1].author == m.author;
                let bubble = column()
                    .gap(px(3.))
                    .when(m.own, |c| c.items_end())
                    .when(!same, |c| {
                        c.child(
                            row()
                                .child(title(m.author.clone()))
                                .child(note(m.time.clone(), cx)),
                        )
                    })
                    .child(
                        div()
                            .max_w(relative(0.9))
                            .py(px(7.))
                            .px(px(10.))
                            .rounded(radius)
                            .bg(if m.own { p.bright } else { p.stage })
                            .text_color(if m.own { p.inverse } else { p.ink })
                            .child(m.text.clone()),
                    )
                    .when_some(m.attachment.clone(), |r, a| {
                        r.child(btn(("file", i), a, false, cx, move |_, _, cx| {
                            cx.emit(ChatEvent::Download(i))
                        }))
                    })
                    .child(
                        item(
                            ("like", i),
                            format!("♡  {}", m.reactions),
                            cx,
                            move |_, _, cx| cx.emit(ChatEvent::React(i)),
                        )
                        .h(px(28.))
                        .child(crate::primitives::glyph::icon(
                            Glyph::Heart,
                            13.,
                            p.secondary,
                        ))
                        .child(swap_label(
                            ("reaction-count", i),
                            m.reactions.to_string(),
                            w,
                            cx,
                        )),
                    );
                arrive(bubble, ("message-arrived", i), 0, 4., cx)
            }));
        panel(&self.id, 522., cx)
            .child(
                row()
                    .justify_between()
                    .child(
                        column()
                            .gap_0()
                            .child(title(self.topic.clone()))
                            .child(note(names.join(", "), cx)),
                    )
                    .child(btn(
                        "presence",
                        if self.online {
                            tr(cx, "Go offline", "离线")
                        } else {
                            tr(cx, "Go online", "上线")
                        },
                        false,
                        cx,
                        |s, _, cx| {
                            s.online = !s.online;
                            cx.notify()
                        },
                    )),
            )
            .child(divider(cx))
            .child(log)
            .when_some(self.typing.clone(), |r, n| r.child(note(n, cx)))
            .child(divider(cx))
            .child(
                row()
                    .child(icon_btn(
                        "attach",
                        tr(cx, "Attach file", "附加文件"),
                        Glyph::Plus,
                        cx,
                        |_, _, cx| cx.emit(ChatEvent::Attach),
                    ))
                    .child(input(&self.composer, cx))
                    .child(icon_btn(
                        "send",
                        tr(cx, "Send", "发送"),
                        Glyph::Send,
                        cx,
                        |s, w, cx| s.send(w, cx),
                    )),
            )
    }
}
