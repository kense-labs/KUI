//! Simulated inline suggestions; no network requests are made.
use gpui::{prelude::*, *};
use kui::{
    components::ghost_text::{GhostText, GhostTextEvent},
    primitives::{
        control::UiLanguage,
        family::{Paint, disabled_motion},
        glyph::{Glyph, icon},
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};

pub const IDS: &[&str] = &["ghost-text"];
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}
struct Example {
    view: Entity<GhostText>,
    language: UiLanguage,
    _subscription: Subscription,
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|cx| {
            GhostText::new(language.text("Description", "说明"), w, cx)
                .plain()
                .min_height(186.)
                .model("Kense")
                .placeholder(language.text("Write a description…", "输入说明…"), cx)
        });
        view.update(cx, |view, cx| {
            view.set_value(
                language.text(
                    "Workspace changes now stay in sync across devices.\n\nThe update keeps local edits intact",
                    "工作区的更改现在会在设备之间同步。\n\n更新会保留本地编辑内容",
                ),
                w,
                cx,
            );
        });
        let active = Rc::new(Cell::new(true));
        let user_active = active.clone();
        let sub = cx.subscribe_in(
            &view,
            w,
            move |s: &mut Self, view, event: &GhostTextEvent, _, cx| match event {
                GhostTextEvent::PreviewStopped => user_active.set(false),
                GhostTextEvent::Request { id, .. } => {
                    let text = s.language.text(
                        " while keeping everything up to date.",
                        "，同时保持所有内容为最新。 ",
                    );
                    view.update(cx, |view, cx| view.push_chunk(*id, text, true, cx));
                }
                _ => {}
            },
        );
        let weak = view.downgrade();
        cx.spawn(async move |_, cx| {
            let suffix = language.text(" while syncing the latest changes.", "，并同步最新更改。");
            let length = suffix.chars().count();
            let mut step = 0usize;
            let mut last_count = None;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(160))
                    .await;
                if !active.get() {
                    break;
                }
                let alive = weak.update(cx, |view, cx| {
                    let visible = cx
                        .try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)());
                    if !visible {
                        return true;
                    }
                    let reduced = disabled_motion(cx);
                    let count = if reduced {
                        length
                    } else {
                        (step * 3).min(length)
                    };
                    if last_count != Some(count) {
                        let text: String = suffix.chars().take(count).collect();
                        if !view.preview_suggestion(text, cx) {
                            return false;
                        }
                        last_count = Some(count);
                    }
                    if reduced {
                        return false;
                    }
                    step = (step + 1) % (length.div_ceil(3) + 16);
                    true
                });
                if !matches!(alive, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
        Self {
            view,
            language,
            _subscription: sub,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let language = self.language;
        div()
            .size_full()
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(520.))
                    .overflow_hidden()
                    .rounded(kui::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .px(px(16.))
                            .py(px(14.))
                            .border_b_1()
                            .border_color(p.line)
                            .child(icon(Glyph::FileText, 16., p.ink))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(
                                                language
                                                    .text("Keep work in sync", "保持工作区同步"),
                                            ),
                                    )
                                    .child(div().text_size(px(12.)).text_color(p.secondary).child(
                                        language.text("Kense · sample change", "Kense · 示例变更"),
                                    )),
                            ),
                    )
                    .child(div().p(px(16.)).child(self.view.clone()))
                    .child(
                        div()
                            .px(px(16.))
                            .pb(px(12.))
                            .text_size(px(11.))
                            .text_color(p.secondary)
                            .child(language.text(
                                "Sample suggestions · nothing is sent",
                                "样例建议 · 不会发送数据",
                            )),
                    ),
            )
    }
}
