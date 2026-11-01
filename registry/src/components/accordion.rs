//! Collapsible sections with measured-height animation.
use crate::primitives::{
    family::Paint,
    glyph::{Glyph, transformed},
    nav::*,
};
use gpui::{prelude::*, *};
pub struct AccordionItem {
    pub title: SharedString,
    pub body: SharedString,
}
impl AccordionItem {
    pub fn new(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}
pub struct Accordion {
    items: Vec<(AccordionItem, Fold)>,
    multiple: bool,
    focus: Vec<FocusHandle>,
}
pub struct AccordionChanged {
    pub index: usize,
    pub expanded: bool,
}
impl EventEmitter<AccordionChanged> for Accordion {}
impl Accordion {
    pub fn new(items: Vec<AccordionItem>, initial: Option<usize>, cx: &App) -> Self {
        let focus = items
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        Self {
            items: items
                .into_iter()
                .enumerate()
                .map(|(i, v)| (v, Fold::new(initial == Some(i), 70.)))
                .collect(),
            multiple: false,
            focus,
        }
    }
    pub fn multiple(mut self, allow: bool) -> Self {
        self.multiple = allow;
        self
    }
    fn toggle(&mut self, i: usize, cx: &mut Context<Self>) {
        let expanded = !self.items[i].1.open;
        if !self.multiple {
            for (_, fold) in &mut self.items {
                fold.set(false, cx)
            }
        }
        self.items[i].1.set(expanded, cx);
        cx.emit(AccordionChanged { index: i, expanded });
        cx.notify()
    }
}
impl Render for Accordion {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut body = column().w(fit(w, 460.)).text_color(p.ink);
        let n = self.items.len();
        for (i, (item, fold)) in self.items.iter_mut().enumerate() {
            let target = self.focus[i].clone();
            body = body
                .child(
                    button(("heading", i), item.title.clone(), cx)
                        .track_focus(&target)
                        .h(px(50.))
                        .px_4()
                        .justify_between()
                        .aria_expanded(fold.open)
                        .child(item.title.clone())
                        .child(transformed(
                            Glyph::Down,
                            16.,
                            p.secondary,
                            fold.fraction(w, cx) * std::f32::consts::PI,
                            1.,
                            (0., 0.),
                            1.,
                        ))
                        .on_click(cx.listener(move |s, _, _, cx| s.toggle(i, cx)))
                        .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                            let j = match e.keystroke.key.as_str() {
                                "up" => (i + n - 1) % n,
                                "down" => (i + 1) % n,
                                "home" => 0,
                                "end" => n - 1,
                                _ => return,
                            };
                            s.focus[j].focus(w, cx);
                            cx.stop_propagation()
                        })),
                )
                .child(fold.render(
                    label(item.body.clone(), 14., p.secondary).px_4().pb_4(),
                    w,
                    cx,
                ));
            if i + 1 < n {
                body = body.child(div().h(px(1.)).bg(p.line))
            }
        }
        body
    }
}
