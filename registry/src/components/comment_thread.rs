//! Read, reply to and resolve a discussion in context.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    editor::{EditEvent, Editor, EditorScrollRegion, InputMode},
    family::Paint,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
#[derive(Clone)]
pub struct Comment {
    pub key: u64,
    pub parent: Option<u64>,
    pub author: String,
    pub text: String,
    pub age: String,
    pub likes: u32,
    pub mine: bool,
}
pub enum CommentEvent {
    Post { parent: Option<u64>, body: String },
    Edit(u64, String),
    Delete(u64),
    Like(u64),
    Resolve(bool),
}
pub struct CommentThread {
    id: SharedString,
    pub subject: String,
    pub comments: Vec<Comment>,
    pub resolved: bool,
    reply: Option<u64>,
    editing: Option<u64>,
    hidden: Vec<u64>,
    scroll: ScrollHandle,
    draft: Entity<Editor>,
    _sub: Subscription,
}
impl EventEmitter<CommentEvent> for CommentThread {}
impl CommentThread {
    pub fn new(
        id: impl Into<SharedString>,
        subject: impl Into<String>,
        comments: Vec<Comment>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = editor(
            tr(cx, "Write a comment…", "写一条评论…").into(),
            InputMode::Single,
            w,
            cx,
        );
        let sub = cx.subscribe_in(&draft, w, |s: &mut Self, _, ev: &EditEvent, w, cx| {
            if *ev == EditEvent::Submitted {
                s.send(w, cx)
            }
        });
        Self {
            id: id.into(),
            subject: subject.into(),
            comments,
            resolved: false,
            reply: None,
            editing: None,
            hidden: vec![],
            scroll: ScrollHandle::new(),
            draft,
            _sub: sub,
        }
    }
    fn send(&mut self, w: &Window, cx: &mut Context<Self>) {
        let body = self.draft.read(cx).value().trim().to_owned();
        if self.resolved || body.is_empty() {
            return;
        }
        cx.emit(if let Some(k) = self.editing.take() {
            CommentEvent::Edit(k, body)
        } else {
            CommentEvent::Post {
                parent: self.reply.take(),
                body,
            }
        });
        replace(&self.draft, "", w, cx);
        cx.notify()
    }
}
impl Render for CommentThread {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut list = column()
            .id("thread")
            .max_h(px(380.))
            .track_scroll(&self.scroll)
            .overflow_y_scroll();
        for (i, c) in self.comments.iter().enumerate() {
            let reply = Fold::get(
                ("comment-height", c.key as usize),
                !c.parent.is_some_and(|k| self.hidden.contains(&k)),
                74.,
                0.24,
                w,
                cx,
            );
            if !reply.mounted() {
                continue;
            }
            let k = c.key;
            list = list.child(
                reply.wrap(
                    8.,
                    column()
                        .gap(px(3.))
                        .when(c.parent.is_some(), |r| {
                            r.ml(px(22.)).pl(px(8.)).border_l_1().border_color(p.line)
                        })
                        .child(
                            row()
                                .child(avatar(&c.author, i, cx))
                                .child(title(c.author.clone()))
                                .child(note(c.age.clone(), cx)),
                        )
                        .child(div().child(c.text.clone()))
                        .child(
                            row()
                                .flex_wrap()
                                .child(btn(
                                    ("like", i),
                                    format!("♡ {}", c.likes),
                                    false,
                                    cx,
                                    move |_, _, cx| cx.emit(CommentEvent::Like(k)),
                                ))
                                .when(!self.resolved, |r| {
                                    r.child(btn(
                                        ("reply", i),
                                        tr(cx, "Reply", "回复"),
                                        false,
                                        cx,
                                        move |s, w, cx| {
                                            s.editing = None;
                                            s.reply = Some(k);
                                            s.draft.read(cx).focus.clone().focus(w, cx);
                                            cx.notify()
                                        },
                                    ))
                                })
                                .when(self.comments.iter().any(|t| t.parent == Some(k)), |r| {
                                    r.child(btn(
                                        ("replies", i),
                                        if self.hidden.contains(&k) {
                                            tr(cx, "Show replies", "展开回复")
                                        } else {
                                            tr(cx, "Hide replies", "收起回复")
                                        },
                                        false,
                                        cx,
                                        move |s, _, cx| {
                                            if s.hidden.contains(&k) {
                                                s.hidden.retain(|&t| t != k)
                                            } else {
                                                s.hidden.push(k)
                                            }
                                            cx.notify()
                                        },
                                    ))
                                })
                                .when(c.mine && !self.resolved, |r| {
                                    r.child(icon_btn(
                                        ("edit", i),
                                        tr(cx, "Edit", "编辑"),
                                        Glyph::Edit,
                                        cx,
                                        move |s, w, cx| {
                                            if let Some(c) = s.comments.iter().find(|c| c.key == k)
                                            {
                                                replace(&s.draft, c.text.clone(), w, cx);
                                                s.editing = Some(k);
                                                s.reply = None;
                                                s.draft.read(cx).focus.clone().focus(w, cx);
                                                cx.notify()
                                            }
                                        },
                                    ))
                                    .child(icon_btn(
                                        ("delete", i),
                                        tr(cx, "Delete", "删除"),
                                        Glyph::Trash,
                                        cx,
                                        move |_, _, cx| cx.emit(CommentEvent::Delete(k)),
                                    ))
                                }),
                        ),
                ),
            )
        }
        panel(&self.id, 522., cx)
            .child(
                row()
                    .child(title(self.subject.clone()).flex_1())
                    .child(badge(self.comments.len().to_string(), cx))
                    .child(btn(
                        "resolve",
                        if self.resolved {
                            tr(cx, "Reopen", "重新打开")
                        } else {
                            tr(cx, "Resolve", "标为已解决")
                        },
                        false,
                        cx,
                        |s, _, cx| {
                            s.resolved = !s.resolved;
                            cx.emit(CommentEvent::Resolve(s.resolved));
                            cx.notify()
                        },
                    )),
            )
            .child(divider(cx))
            .child(
                div().relative().child(list).child(
                    canvas(|_, _, _| (), {
                        let scroll = self.scroll.clone();
                        move |bounds, _, _, cx| {
                            if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                                let offset = -scroll.offset().y;
                                (host.0)(
                                    bounds,
                                    offset > px(0.1),
                                    offset + px(0.1) < scroll.max_offset().y,
                                );
                            }
                        }
                    })
                    .absolute()
                    .inset_0(),
                ),
            )
            .when(!self.resolved, |r| {
                r.when(self.reply.is_some() || self.editing.is_some(), |r| {
                    r.child(btn(
                        "cancel",
                        tr(cx, "Cancel reply or edit", "取消回复或编辑"),
                        false,
                        cx,
                        |s, w, cx| {
                            s.reply = None;
                            s.editing = None;
                            replace(&s.draft, "", w, cx);
                            cx.notify()
                        },
                    ))
                })
                .child(row().child(input(&self.draft, cx)).child(btn(
                    "send",
                    tr(cx, "Send", "发送"),
                    true,
                    cx,
                    |s, w, cx| s.send(w, cx),
                )))
            })
    }
}
