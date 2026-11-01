//! Token entry with composition-safe input and explicit two-step Backspace removal.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide, measure},
    form_chrome::*,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

struct ChipMotion {
    label: SharedString,
    presence: Slide,
    position: Option<[Slide; 2]>,
}

impl ChipMotion {
    fn new(label: SharedString, present: bool) -> Self {
        Self {
            label,
            presence: Slide::snappy(present as u8 as f32),
            position: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TagsChanged(pub Vec<SharedString>);
pub struct TagInput {
    label: SharedString,
    values: Vec<SharedString>,
    draft: Entity<Editor>,
    armed: bool,
    visible: Vec<ChipMotion>,
    slots: Rc<RefCell<HashMap<SharedString, Point<Pixels>>>>,
    error: SharedString,
    max: usize,
    _sub: Subscription,
}
impl EventEmitter<TagsChanged> for TagInput {}
impl TagInput {
    pub fn new(label: impl Into<SharedString>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let label = label.into();
        let draft = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Single, w, cx);
            e.set_placeholder(tr(cx, "Add a tag…", "添加标签…"));
            e
        });
        let sub = cx.subscribe_in(&draft, w, |s: &mut Self, e, ev: &EditEvent, w, cx| {
            if *ev == EditEvent::Submitted {
                s.add(w, cx)
            } else if *ev == EditEvent::Changed {
                let v = e.read(cx).value();
                if v.ends_with(',') || v.ends_with('，') {
                    s.add(w, cx)
                }
                s.armed = false
            }
            cx.notify()
        });
        Self {
            label,
            values: vec![],
            draft,
            armed: false,
            visible: vec![],
            slots: Default::default(),
            error: "".into(),
            max: 20,
            _sub: sub,
        }
    }
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.values = tags.iter().map(|s| s.to_string().into()).collect();
        self.visible = self
            .values
            .iter()
            .cloned()
            .map(|v| ChipMotion::new(v, true))
            .collect();
        self
    }
    pub fn tags(&self) -> &[SharedString] {
        &self.values
    }
    fn add(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let raw = self.draft.read(cx).value().to_owned();
        let mut changed = false;
        self.error = "".into();
        for val in raw
            .split([',', '，'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if self.values.iter().any(|v| v.eq_ignore_ascii_case(val)) {
                self.error = tr(cx, "Tag already exists", "标签已存在").into()
            } else if self.values.len() >= self.max {
                self.error = tr(cx, "Tag limit reached", "已达到标签上限").into();
                break;
            } else {
                self.values.push(val.to_owned().into());
                changed = true
            }
        }
        if changed {
            self.draft.update(cx, |e, cx| e.set_value("", w, cx));
            cx.emit(TagsChanged(self.values.clone()))
        }
        cx.notify()
    }
    fn remove(&mut self, i: usize, cx: &mut Context<Self>) {
        if i < self.values.len() {
            self.values.remove(i);
            self.armed = false;
            cx.emit(TagsChanged(self.values.clone()));
            cx.notify()
        }
    }
}
impl Render for TagInput {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut line = row()
            .relative()
            .flex_wrap()
            .gap(px(4.))
            .min_h(px(44.))
            .p(px(5.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .child(div().absolute().inset_0());
        // GPUI wraps the slots. The painted chips are positioned independently so a
        // wrap can travel from its old row without ever painting at the new row first.
        for tag in &self.values {
            if !self.visible.iter().any(|chip| &chip.label == tag) {
                self.visible.push(ChipMotion::new(tag.clone(), false));
            }
            line = line.child(
                div()
                    .flex_none()
                    .w(px(34. + measure(tag, 14., w)))
                    .h(px(24.)),
            );
        }
        let slot_names = self.values.clone();
        let slots = self.slots.clone();
        line = line
            .child(
                div()
                    .h(px(30.))
                    .min_w(px(80.))
                    .flex_1()
                    .child(self.draft.clone()),
            )
            .on_children_prepainted(move |children, w, _| {
                let Some(anchor) = children.first() else {
                    return;
                };
                let next = slot_names
                    .iter()
                    .zip(children.iter().skip(1))
                    .map(|(tag, b)| (tag.clone(), b.origin - anchor.origin))
                    .collect();
                if *slots.borrow() != next {
                    *slots.borrow_mut() = next;
                    w.request_animation_frame();
                }
            });
        for chip in &mut self.visible {
            chip.presence
                .duration(self.values.contains(&chip.label) as u8 as f32, 0.40, cx);
        }
        self.visible
            .retain(|chip| self.values.contains(&chip.label) || chip.presence.value(cx) > 0.001);
        for chip in &mut self.visible {
            let t = &chip.label;
            let selected = self.values.iter().position(|v| v == t);
            let i = selected.unwrap_or(usize::MAX);
            let amount = chip.presence.get(w, cx).clamp(0., 1.);
            if let Some(point) = self.slots.borrow().get(t) {
                let dest = [f32::from(point.x), f32::from(point.y)];
                let position = chip.position.get_or_insert_with(|| dest.map(Slide::snappy));
                for (axis, target) in position.iter_mut().zip(dest) {
                    axis.duration(target, 0.40, cx);
                }
            }
            let Some(position) = &chip.position else {
                continue;
            };
            let tag = t.clone();
            let width = 34. + measure(t, 14., w);
            line = line.child(
                row()
                    .debug_selector(|| format!("Tag {t}"))
                    .absolute()
                    .left(px(position[0].get(w, cx)))
                    .top(px(position[1].get(w, cx)))
                    .w(px(width * (0.9 + 0.1 * amount)))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .opacity(amount)
                    .h(px(24. * (0.9 + amount * 0.1)))
                    .rounded(crate::foundation::surface_radius(cx))
                    .pl(px(8.))
                    .gap(px(1.))
                    .bg(
                        if self.armed
                            && selected.is_some()
                            && i == self.values.len().saturating_sub(1)
                        {
                            p.bright
                        } else {
                            p.hover
                        },
                    )
                    .text_color(
                        if self.armed
                            && selected.is_some()
                            && i == self.values.len().saturating_sub(1)
                        {
                            p.inverse
                        } else {
                            p.ink
                        },
                    )
                    .child(t.clone())
                    .child(
                        glyph(
                            ElementId::Name(format!("tag-remove-{tag}").into()),
                            format!("{} {t}", tr(cx, "Remove", "移除")),
                            Glyph::Close,
                            cx,
                        )
                        .size(px(23.))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if let Some(i) = s.values.iter().position(|v| v == &tag) {
                                s.remove(i, cx)
                            }
                        })),
                    ),
            )
        }
        section(420., cx)
            .id("tag-input")
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "backspace"
                    && s.draft.read(cx).value().is_empty()
                    && !s.values.is_empty()
                {
                    if s.armed {
                        s.remove(s.values.len() - 1, cx)
                    } else {
                        s.armed = true;
                        cx.notify()
                    }
                    w.prevent_default();
                    cx.stop_propagation()
                }
            }))
            .child(self.label.clone())
            .child(line)
            .child(hint(
                if self.error.is_empty() {
                    tr(
                        cx,
                        "Enter adds a tag. Backspace twice removes the last tag.",
                        "回车添加标签，连按两次退格删除最后一个标签。",
                    )
                    .into()
                } else {
                    self.error.clone()
                },
                cx,
            ))
    }
}
