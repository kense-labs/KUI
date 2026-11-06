//! Deferred batch removal with countdown ring, undo and explicit host commit acknowledgement.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    form_chrome::tr,
    glyph::{Glyph, countdown_ring, icon},
};
use gpui::{prelude::*, *};
use std::{collections::HashMap, time::Duration};
use web_time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoItem {
    pub key: SharedString,
    pub sender: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub meta: SharedString,
    pub unread: bool,
}
impl UndoItem {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            sender: "".into(),
            title: title.into(),
            description: "".into(),
            meta: "".into(),
            unread: false,
        }
    }
    pub fn with_sender(mut self, sender: impl Into<SharedString>) -> Self {
        self.sender = sender.into();
        self
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = meta.into();
        self
    }
    pub fn with_unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemovalKind {
    Archive,
    Delete,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoEntry {
    pub key: SharedString,
    pub kind: RemovalKind,
    pub original_index: usize,
}
/// The host must answer CommitRequested with resolve_commit; no irreversible operation occurs beforehand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoToastEvent {
    CommitRequested { batch: u64, entries: Vec<UndoEntry> },
    Undone(Vec<UndoEntry>),
}
/// Optimistically hides rows until a reversible batch is undone or confirmed by the host.
pub struct UndoToast {
    label: SharedString,
    items: Vec<UndoItem>,
    initial_items: Vec<UndoItem>,
    pending: Vec<UndoEntry>,
    duration: Duration,
    remaining: Duration,
    last: Instant,
    batch: u64,
    waiting: bool,
    hovered: bool,
    focused: bool,
    hidden: bool,
    error: bool,
    restored: usize,
    focus: FocusHandle,
    root_focus: FocusHandle,
    focus_subs: Vec<Subscription>,
    visibility_sub: Option<Subscription>,
    rows: HashMap<SharedString, Slide>,
    show: Slide,
    epoch: u64,
}
impl EventEmitter<UndoToastEvent> for UndoToast {}
impl UndoToast {
    pub fn new(
        label: impl Into<SharedString>,
        items: Vec<UndoItem>,
        cx: &mut Context<Self>,
    ) -> Self {
        let now = cx.background_executor().now();
        let rows = items
            .iter()
            .map(|item| (item.key.clone(), Slide::snappy(1.)))
            .collect();
        Self {
            label: label.into(),
            initial_items: items.clone(),
            items,
            pending: Vec::new(),
            duration: Duration::from_secs(6),
            remaining: Duration::ZERO,
            last: now,
            batch: 0,
            waiting: false,
            hovered: false,
            focused: false,
            hidden: false,
            error: false,
            restored: 0,
            focus: cx.focus_handle(),
            root_focus: cx.focus_handle(),
            focus_subs: Vec::new(),
            visibility_sub: None,
            rows,
            show: Slide::snappy(0.),
            epoch: 0,
        }
    }
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration.max(Duration::from_millis(100));
        self
    }
    pub fn pending(&self) -> &[UndoEntry] {
        &self.pending
    }
    pub fn is_depleted(&self) -> bool {
        self.items.len() < self.initial_items.len() || !self.pending.is_empty()
    }
    pub fn refill(&mut self, cx: &mut Context<Self>) {
        self.epoch = self.epoch.wrapping_add(1);
        self.waiting = false;
        self.error = false;
        self.restored = 0;
        self.pending.clear();
        self.items = self.initial_items.clone();
        for item in &self.items {
            self.rows
                .entry(item.key.clone())
                .or_insert_with(|| Slide::snappy(1.))
                .duration(1., 0.4, cx);
        }
        self.show.duration(0., 0.18, cx);
        cx.notify();
    }
    pub fn remove(&mut self, key: &str, kind: RemovalKind, cx: &mut Context<Self>) {
        if self.waiting || self.pending.iter().any(|e| e.key.as_ref() == key) {
            return;
        }
        let Some(index) = self.items.iter().position(|item| item.key.as_ref() == key) else {
            return;
        };
        self.pending.push(UndoEntry {
            key: self.items[index].key.clone(),
            kind,
            original_index: index,
        });
        self.error = false;
        self.restored = 0;
        self.remaining = self.duration;
        self.last = cx.background_executor().now();
        self.epoch = self.epoch.wrapping_add(1);
        self.show.duration(1., 0.42, cx);
        Self::schedule(self.epoch, cx);
        cx.notify();
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.waiting || self.pending.is_empty() {
            return;
        }
        let entries = std::mem::take(&mut self.pending);
        self.epoch = self.epoch.wrapping_add(1);
        self.restored = entries.len();
        self.show.duration(1., 0.18, cx);
        self.error = false;
        let epoch = self.epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1800))
                .await;
            let _ = this.update(cx, |s, cx| {
                if s.epoch == epoch && s.pending.is_empty() && !s.error {
                    s.restored = 0;
                    s.show.duration(0., 0.18, cx);
                    cx.notify();
                }
            });
        })
        .detach();
        cx.emit(UndoToastEvent::Undone(entries));
        cx.notify();
    }
    pub fn commit(&mut self, cx: &mut Context<Self>) {
        if self.waiting || self.pending.is_empty() {
            return;
        }
        self.waiting = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.batch = self.batch.wrapping_add(1);
        cx.emit(UndoToastEvent::CommitRequested {
            batch: self.batch,
            entries: self.pending.clone(),
        });
        cx.notify();
    }
    /// Ignore stale responses. Failure restores every original row without changing its order.
    pub fn resolve_commit(&mut self, batch: u64, success: bool, cx: &mut Context<Self>) {
        if !self.waiting || batch != self.batch {
            return;
        }
        self.waiting = false;
        if success {
            self.items
                .retain(|item| !self.pending.iter().any(|e| e.key == item.key));
            self.error = false;
            self.show.duration(0., 0.18, cx);
        } else {
            self.error = true;
            self.show.duration(1., 0.20, cx);
        }
        self.pending.clear();
        cx.notify();
    }
    fn schedule(epoch: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            let _ = this.update(cx, |s, cx| {
                if s.epoch != epoch || s.pending.is_empty() || s.waiting {
                    return;
                }
                let now = cx.background_executor().now();
                let delta = now.saturating_duration_since(s.last);
                s.last = now;
                if !s.hovered && !s.focused && !s.hidden {
                    s.remaining = s.remaining.saturating_sub(delta);
                }
                if s.remaining.is_zero() {
                    s.commit(cx)
                } else {
                    Self::schedule(epoch, cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }
}
impl Render for UndoToast {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subs.is_empty() {
            self.focus_subs
                .push(cx.on_focus_in(&self.focus, w, |s, _, _| s.focused = true));
            self.focus_subs
                .push(cx.on_focus_out(&self.focus, w, |s, _, _, _| s.focused = false));
        }
        if self.visibility_sub.is_none() {
            self.hidden = !w.is_visible();
            self.visibility_sub = Some(cx.observe_window_visibility(w, |s, visibility, _, cx| {
                s.hidden = !visibility.is_visible();
                s.last = cx.background_executor().now();
            }));
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let mut rows = div()
            .flex()
            .flex_col()
            .w_full()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .overflow_hidden();
        let mut any_visible = false;
        for (row_idx, item) in self.items.iter().enumerate() {
            let target = if self.pending.iter().any(|e| e.key == item.key) {
                0.
            } else {
                1.
            };
            let row = self
                .rows
                .entry(item.key.clone())
                .or_insert_with(|| Slide::snappy(1.));
            row.duration(target, 0.4, cx);
            let amount = row.get(w, cx).clamp(0., 1.);
            if amount <= 0.001 {
                continue;
            }
            any_visible = true;
            let key_archive = item.key.clone();
            let key_delete = item.key.clone();
            let mut title_line = div().flex().items_center().gap(px(8.)).min_w_0();
            if item.unread {
                title_line = title_line.child(div().size(px(6.)).rounded_full().bg(p.bright));
            }
            if !item.sender.is_empty() {
                title_line = title_line.child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(item.sender.clone()),
                );
            }
            title_line = title_line.child(
                div()
                    .truncate()
                    .text_color(if item.sender.is_empty() {
                        p.ink
                    } else {
                        p.secondary
                    })
                    .font_weight(if item.sender.is_empty() || item.unread {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(item.title.clone()),
            );
            let content = div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .min_w_0()
                .flex_1()
                .child(title_line)
                .when(!item.description.is_empty(), |c| {
                    c.child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(item.description.clone()),
                    )
                });
            let archive = tr(cx, "Archive", "归档");
            let delete = tr(cx, "Delete", "删除");
            rows = rows.child(
                div()
                    .h(px(60. * amount))
                    .overflow_hidden()
                    .opacity(amount)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(12.))
                    .py(px(10.))
                    .when(row_idx > 0, |r| r.border_t_1().border_color(p.line))
                    .hover(move |s| s.bg(p.tray))
                    .child(content)
                    .when(!item.meta.is_empty(), |r| {
                        r.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(item.meta.clone()),
                        )
                    })
                    .child(
                        face(
                            SharedString::from(format!("undo-archive-{}", key_archive)),
                            archive,
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .size(px(32.))
                        .rounded_full()
                        .p_0()
                        .hover(move |s| s.bg(p.ink.opacity(0.07)).text_color(p.ink))
                        .child(icon(Glyph::Archive, 14., p.secondary))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.remove(&key_archive, RemovalKind::Archive, cx)
                        })),
                    )
                    .child(
                        face(
                            SharedString::from(format!("undo-delete-{}", key_delete)),
                            delete,
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .size(px(32.))
                        .rounded_full()
                        .p_0()
                        .hover(move |s| s.bg(p.ink.opacity(0.07)).text_color(p.ink))
                        .child(icon(Glyph::Trash, 14., p.secondary))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.remove(&key_delete, RemovalKind::Delete, cx)
                        })),
                    ),
            );
        }
        if !any_visible {
            rows = rows.child(
                div()
                    .min_h(px(120.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .text_color(p.secondary)
                    .child(icon(Glyph::Check, 18., p.success))
                    .child(tr(cx, "All caught up", "已全部处理")),
            );
        }
        let visible = self.show.get(w, cx).clamp(0., 1.);
        let mut root = div()
            .id("undo-toast")
            .relative()
            .flex()
            .flex_col()
            .w(px(760.))
            .max_w_full()
            .text_color(p.ink)
            .track_focus(&self.root_focus)
            .role(Role::List)
            .aria_label(self.label.clone())
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "z"
                    && (e.keystroke.modifiers.platform || e.keystroke.modifiers.control)
                    && !e.keystroke.modifiers.shift
                    && !s.pending.is_empty()
                {
                    s.undo(cx);
                    cx.stop_propagation();
                    w.prevent_default();
                }
            }))
            .child(rows);
        if visible > 0. {
            let count = self.pending.len();
            let all_delete = !self.pending.is_empty()
                && self.pending.iter().all(|e| e.kind == RemovalKind::Delete);
            let action_word = if all_delete {
                tr(cx, "deleted", "已删除")
            } else {
                tr(cx, "archived", "已归档")
            };
            let text = if self.error {
                tr(
                    cx,
                    "Could not complete the action; items restored.",
                    "操作失败，项目已恢复。",
                )
                .to_owned()
            } else if self.waiting {
                tr(cx, "Completing…", "正在完成…").to_owned()
            } else if self.restored > 0 {
                format!("{} {}", self.restored, tr(cx, "restored", "项已恢复"))
            } else {
                format!(
                    "{count} {} {action_word} · {:.0}s",
                    if count == 1 {
                        tr(cx, "conversation", "条会话")
                    } else {
                        tr(cx, "conversations", "条会话")
                    },
                    self.remaining.as_secs_f32().ceil()
                )
            };
            let undo = tr(cx, "Undo", "撤销");
            let ring_frac =
                (self.remaining.as_secs_f32() / self.duration.as_secs_f32().max(0.1)).clamp(0., 1.);
            let mut left = div().flex().items_center().gap(px(12.));
            if self.restored > 0 {
                left = left.child(icon(Glyph::Check, 16., p.success));
            } else if !self.error && !self.waiting && !self.pending.is_empty() {
                left = left.child(countdown_ring(ring_frac, 24., p.bright));
            }
            left = left.child(div().text_color(p.ink).child(text.clone()));
            let mut toast = div()
                .id("undo-notice")
                .track_focus(&self.focus)
                .role(Role::Status)
                .aria_label(text)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .min_h(px(54.))
                .mt(px(12.))
                .w_full()
                .max_w(px(372.))
                .py(px(8.))
                .pl(px(12.))
                .pr(px(8.))
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .shadow_lg()
                .text_color(p.ink)
                .opacity(visible)
                .child(left)
                .on_hover(cx.listener(|s, h, _, _| s.hovered = *h));
            if !self.error && !self.waiting && !self.pending.is_empty() {
                toast = toast.child(
                    face("undo-action", undo, 36., radius.into(), Tone::Clear, cx)
                        .pl(px(14.))
                        .pr(px(12.))
                        .gap(px(8.))
                        .text_color(p.ink)
                        .bg(p.tray)
                        .hover(move |s| s.bg(p.tray.blend(p.ink.opacity(0.09))))
                        .child(undo)
                        .child(div().text_size(px(12.)).text_color(p.faint).child("⌘Z"))
                        .on_click(cx.listener(|s, _, _, cx| s.undo(cx))),
                );
            }
            root = root.child(toast);
        }
        root
    }
}
