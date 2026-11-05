//! A host-controlled collection of transient notifications.
use crate::{
    components::toast::{Toast, ToastEvent},
    primitives::family::{Paint, Slide},
};
use gpui::{prelude::*, *};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(pub u64);
#[derive(Clone, Debug)]
pub struct StackToastEvent {
    pub id: ToastId,
    pub event: ToastEvent,
}
struct Entry {
    id: ToastId,
    toast: Entity<Toast>,
    _subscription: Subscription,
}
pub struct ToastStack {
    entries: Vec<Entry>,
    next: u64,
    limit: usize,
    label: SharedString,
    expanded: Slide,
    is_expanded: bool,
    hovered: bool,
    focused: bool,
    focus: Option<FocusHandle>,
    focus_subs: Vec<Subscription>,
}
impl EventEmitter<StackToastEvent> for ToastStack {}
impl ToastStack {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            entries: Vec::new(),
            next: 0,
            limit: 12,
            label: label.into(),
            expanded: Slide::new(0.),
            is_expanded: false,
            hovered: false,
            focused: false,
            focus: None,
            focus_subs: Vec::new(),
        }
    }
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }
    pub fn push(
        &mut self,
        message: impl Into<SharedString>,
        action: Option<SharedString>,
        timeout: Duration,
        cx: &mut Context<Self>,
    ) -> ToastId {
        self.next = self.next.wrapping_add(1);
        let id = ToastId(self.next);
        let toast = cx.new(|_| Toast::new());
        let subscription = cx.subscribe(&toast, move |s, _, event: &ToastEvent, cx| {
            cx.emit(StackToastEvent { id, event: *event });
            s.entries.retain(|entry| entry.id != id);
            cx.notify();
        });
        if self.entries.len() >= self.limit {
            self.entries.remove(0);
        }
        toast.update(cx, |toast, cx| {
            toast.set_stack_paused(self.is_expanded, cx);
            toast.show(message, action, timeout, cx);
        });
        self.entries.push(Entry {
            id,
            toast,
            _subscription: subscription,
        });
        cx.notify();
        id
    }
    pub fn update(
        &mut self,
        id: ToastId,
        message: impl Into<SharedString>,
        action: Option<SharedString>,
        timeout: Duration,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(entry) = self.entries.iter().find(|entry| entry.id == id) {
            entry
                .toast
                .update(cx, |toast, cx| toast.show(message, action, timeout, cx));
            true
        } else {
            false
        }
    }
    pub fn dismiss(&mut self, id: ToastId, cx: &mut Context<Self>) -> bool {
        let previous = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        if previous != self.entries.len() {
            cx.emit(StackToastEvent {
                id,
                event: ToastEvent::Dismissed,
            });
            cx.notify();
            true
        } else {
            false
        }
    }
    fn sync_expanded(&mut self, cx: &mut Context<Self>) {
        let expanded = self.hovered || self.focused;
        if self.is_expanded == expanded {
            return;
        }
        self.is_expanded = expanded;
        self.expanded
            .duration(if expanded { 1. } else { 0. }, 0.42, cx);
        for entry in &self.entries {
            entry
                .toast
                .update(cx, |toast, cx| toast.set_stack_paused(expanded, cx));
        }
        cx.notify();
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
impl Render for ToastStack {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus.is_none() {
            let focus = cx.focus_handle().tab_stop(true);
            self.focus_subs.push(cx.on_focus_in(&focus, w, |s, _, cx| {
                s.focused = true;
                s.sync_expanded(cx);
            }));
            self.focus_subs
                .push(cx.on_focus_out(&focus, w, |s, _, _, cx| {
                    s.focused = false;
                    s.sync_expanded(cx);
                }));
            self.focus = Some(focus);
        }
        let focus = self.focus.as_ref().unwrap();
        let p = Paint::get(cx);
        let n = self.entries.len().min(3);
        let expanded = self.expanded.get(w, cx).clamp(0., 1.);
        let height = stack_span(n, expanded);
        let mut stack = div()
            .id("toast-stack")
            .role(Role::Region)
            .aria_label(self.label.clone())
            .track_focus(focus)
            .relative()
            .w(px(360.))
            .max_w_full()
            .h(px(height))
            .text_color(p.ink)
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                s.hovered = *over;
                s.sync_expanded(cx);
            }))
            .on_key_down(cx.listener(|s, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape"
                    && let Some(id) = s.entries.last().map(|entry| entry.id)
                {
                    s.dismiss(id, cx);
                }
            }));
        for depth in (0..n).rev() {
            let entry = &self.entries[self.entries.len() - depth - 1];
            entry.toast.update(cx, |toast, _| {
                toast.set_stack_chrome(depth == 0 || expanded > 0.5);
            });
            let tuck = 1. - expanded;
            let scale = 1. - 0.05 * depth as f32 * tuck;
            let bottom = depth as f32 * (14. * tuck + (76. + 12.) * expanded);
            let inset = (1. - scale) * 0.5;
            stack = stack.child(
                div()
                    .absolute()
                    .bottom(px(bottom))
                    .left(relative(inset))
                    .right(relative(inset))
                    .child(entry.toast.clone()),
            );
        }
        stack
    }
}

fn stack_span(count: usize, expanded: f32) -> f32 {
    if count == 0 {
        return 0.;
    }
    let n = count as f32;
    let collapsed = 76. + 14. * (n - 1.);
    let open = 76. * n + 12. * (n - 1.);
    collapsed + (open - collapsed) * expanded
}

#[cfg(test)]
mod tests {
    use super::stack_span;

    #[test]
    fn collapsed_stack_peeks_and_open_stack_gaps() {
        assert_eq!(stack_span(0, 0.), 0.);
        assert!((stack_span(3, 0.) - 104.).abs() < 0.001);
        assert!((stack_span(3, 1.) - 252.).abs() < 0.001);
    }
}
