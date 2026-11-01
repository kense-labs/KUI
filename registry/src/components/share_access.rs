//! Invite collaborators and manage access to a shared resource.
use crate::primitives::ops_surface::*;
use crate::primitives::{
    clipboard,
    editor::{EditEvent, Editor, InputMode},
    glyph::Glyph,
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccessRole {
    Owner,
    Edit,
    Comment,
    View,
}
impl AccessRole {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Owner => tr(cx, "Owner", "所有者"),
            Self::Edit => tr(cx, "Can edit", "可编辑"),
            Self::Comment => tr(cx, "Can comment", "可评论"),
            Self::View => tr(cx, "Can view", "可查看"),
        }
    }
}
#[derive(Clone)]
pub struct Person {
    pub name: String,
    pub email: String,
    pub role: AccessRole,
}
pub enum AccessEvent {
    Invite(String),
    Role(String, AccessRole),
    Remove(String),
    Undo(String),
    LinkAccess(bool),
}
pub struct AccessEditor {
    id: SharedString,
    pub document: String,
    pub url: String,
    pub people: Vec<Person>,
    pub public_link: bool,
    pub open: bool,
    editor: Entity<Editor>,
    _sub: Subscription,
    menu: Option<usize>,
    removed: Option<(usize, Person)>,
    error: bool,
    copied: Option<bool>,
}
impl EventEmitter<AccessEvent> for AccessEditor {}
impl AccessEditor {
    pub fn new(
        id: impl Into<SharedString>,
        document: impl Into<String>,
        url: impl Into<String>,
        people: Vec<Person>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = editor(
            tr(cx, "Add people by email…", "输入邀请邮箱…").into(),
            InputMode::Single,
            w,
            cx,
        );
        let sub = cx.subscribe_in(&editor, w, |s: &mut Self, _, e: &EditEvent, w, cx| {
            if *e == EditEvent::Submitted {
                s.invite(w, cx)
            }
        });
        Self {
            id: id.into(),
            document: document.into(),
            url: url.into(),
            people,
            public_link: false,
            open: true,
            editor,
            _sub: sub,
            menu: None,
            removed: None,
            error: false,
            copied: None,
        }
    }
    fn invite(&mut self, w: &Window, cx: &mut Context<Self>) {
        let v = self.editor.read(cx).value().trim().to_lowercase();
        if !v
            .split_once('@')
            .is_some_and(|(a, b)| !a.is_empty() && b.contains('.') && !b.contains('@'))
            || v.chars().any(char::is_whitespace)
            || self.people.iter().any(|p| p.email.eq_ignore_ascii_case(&v))
        {
            self.error = true;
            cx.notify();
            return;
        }
        self.error = false;
        cx.emit(AccessEvent::Invite(v));
        replace(&self.editor, "", w, cx);
        cx.notify()
    }
}
impl Render for AccessEditor {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = panel(&self.id, 520., cx).child(
            row()
                .child(title(self.document.clone()).flex_1())
                .child(btn(
                    "share",
                    tr(cx, "Share", "分享"),
                    false,
                    cx,
                    |s, _, cx| {
                        s.open = !s.open;
                        cx.notify()
                    },
                )),
        );
        let body = Fold::get("share-panel", self.open, 390., 0.24, w, cx);
        if !body.mounted() {
            return root;
        }
        let mut r = column()
            .child(divider(cx))
            .child(title(format!(
                "{} “{}”",
                tr(cx, "Share", "分享"),
                self.document
            )))
            .child(row().child(input(&self.editor, cx)).child(btn(
                "invite",
                tr(cx, "Invite", "邀请"),
                true,
                cx,
                |s, w, cx| s.invite(w, cx),
            )))
            .when(self.error, |el| {
                el.child(note(
                    tr(
                        cx,
                        "Enter a new, valid email address.",
                        "请输入尚未加入的有效邮箱。",
                    ),
                    cx,
                ))
            })
            .child(note(
                format!(
                    "{} · {}",
                    tr(cx, "People with access", "拥有访问权限的成员"),
                    self.people.len()
                ),
                cx,
            ));
        for (i, p) in self.people.iter().enumerate() {
            r = r.child(
                row()
                    .flex_wrap()
                    .child(avatar(&p.name, i, cx))
                    .child(
                        column()
                            .flex_1()
                            .gap_0()
                            .child(title(p.name.clone()))
                            .child(note(p.email.clone(), cx).truncate()),
                    )
                    .child(if p.role == AccessRole::Owner {
                        badge(p.role.label(cx), cx).into_any_element()
                    } else {
                        btn(("role", i), p.role.label(cx), false, cx, move |s, _, cx| {
                            s.menu = if s.menu == Some(i) { None } else { Some(i) };
                            cx.notify()
                        })
                        .into_any_element()
                    }),
            );
            let menu = Fold::get(
                ("access-menu", i),
                self.menu == Some(i) && p.role != AccessRole::Owner,
                34.,
                0.2,
                w,
                cx,
            );
            if menu.mounted() {
                r = r.child(
                    menu.wrap(
                        8.,
                        row()
                            .flex_wrap()
                            .children(
                                [AccessRole::View, AccessRole::Comment, AccessRole::Edit]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(j, role)| {
                                        btn(
                                            ("set", j),
                                            role.label(cx),
                                            p.role == role,
                                            cx,
                                            move |s, _, cx| {
                                                let p = &mut s.people[i];
                                                p.role = role;
                                                s.menu = None;
                                                cx.emit(AccessEvent::Role(p.email.clone(), role));
                                                cx.notify()
                                            },
                                        )
                                    }),
                            )
                            .child(icon_btn(
                                "remove",
                                tr(cx, "Remove", "移除"),
                                Glyph::Trash,
                                cx,
                                move |s, _, cx| {
                                    let p = s.people.remove(i);
                                    cx.emit(AccessEvent::Remove(p.email.clone()));
                                    s.removed = Some((i, p));
                                    s.menu = None;
                                    cx.notify()
                                },
                            )),
                    ),
                )
            }
        }
        let content = r
            .child(divider(cx))
            .when(self.removed.is_some(), |el| {
                el.child(btn(
                    "undo",
                    tr(cx, "Undo removal", "撤销移除"),
                    false,
                    cx,
                    |s, _, cx| {
                        if let Some((i, p)) = s.removed.take() {
                            cx.emit(AccessEvent::Undo(p.email.clone()));
                            s.people.insert(i.min(s.people.len()), p);
                            cx.notify()
                        }
                    },
                ))
            })
            .child(
                row()
                    .flex_wrap()
                    .child(note(tr(cx, "General access", "链接权限"), cx))
                    .child(btn(
                        "access",
                        if self.public_link {
                            tr(cx, "Anyone with the link", "获得链接的任何人")
                        } else {
                            tr(cx, "Restricted", "访问受限")
                        },
                        false,
                        cx,
                        |s, _, cx| {
                            s.public_link = !s.public_link;
                            cx.emit(AccessEvent::LinkAccess(s.public_link));
                            cx.notify()
                        },
                    )),
            )
            .child(
                row()
                    .child(btn(
                        "copy",
                        match self.copied {
                            Some(true) => tr(cx, "Copied", "已复制"),
                            Some(false) => tr(cx, "Copy failed · Retry", "复制失败 · 重试"),
                            None => tr(cx, "Copy link", "复制链接"),
                        },
                        false,
                        cx,
                        |s, _, cx| {
                            let task = clipboard::write(&s.url, cx);
                            cx.spawn(async move |v, cx| {
                                let ok = task.await;
                                let _ = v.update(cx, |v, cx| {
                                    v.copied = Some(ok);
                                    cx.notify()
                                });
                            })
                            .detach()
                        },
                    ))
                    .child(btn(
                        "done",
                        tr(cx, "Done", "完成"),
                        true,
                        cx,
                        |s, _, cx| {
                            s.open = false;
                            cx.notify()
                        },
                    )),
            );
        root.child(body.wrap(12., content))
    }
}
