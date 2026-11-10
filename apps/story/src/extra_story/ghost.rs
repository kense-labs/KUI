//! Simulated inline suggestions; no network requests are made.
use gpui::{prelude::*, *};
use kui::{
    components::ghost_text::{GhostText, GhostTextEvent},
    primitives::{
        control::UiLanguage,
        family::{Paint, disabled_motion},
    },
};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, OnceLock},
    time::Duration,
};

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
fn github_mark(dark: bool) -> Arc<Image> {
    static LIGHT: OnceLock<Arc<Image>> = OnceLock::new();
    static DARK: OnceLock<Arc<Image>> = OnceLock::new();
    let mark = if dark { &DARK } else { &LIGHT };
    mark.get_or_init(|| {
        let svg = include_str!("../../assets/orbit-logos/github.svg");
        let bytes = if dark {
            svg.replace("#181717", "#f1f1f1").into_bytes()
        } else {
            svg.as_bytes().to_vec()
        };
        Arc::new(Image::from_bytes(ImageFormat::Svg, bytes))
    })
    .clone()
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|cx| {
            GhostText::new(language.text("Description", "说明"), w, cx)
                .plain()
                .min_height(186.)
                .model("Claude Haiku 5.5")
                .placeholder(language.text("Write a description…", "输入说明…"), cx)
        });
        view.update(cx, |view, cx| {
            view.set_value(
                language.text(
                    "Customers who upgraded mid-cycle could be charged twice.\n\nThis change keeps the original subscription reference",
                    "在账期中途升级的客户可能被重复扣款。\n\n此改动保留原订阅记录",
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
                        " when the billing request is retried.",
                        "，便于重试账单请求。",
                    );
                    view.update(cx, |view, cx| view.push_chunk(*id, text, true, cx));
                }
                _ => {}
            },
        );
        let weak = view.downgrade();
        cx.spawn(async move |_, cx| {
            let suffix = language.text(
                " when a billing request is retried.",
                "，便于重试账单请求。",
            );
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
                            .child(
                                img(github_mark(
                                    kui::foundation::Theme::global(cx).mode
                                        == kui::foundation::ThemeMode::Dark,
                                ))
                                .size(px(16.))
                                .flex_none(),
                            )
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
                                            .child(language.text(
                                                "Fix double charge on mid-cycle upgrades",
                                                "修复账期中途升级时的重复扣款",
                                            )),
                                    )
                                    .child(div().text_size(px(12.)).text_color(p.secondary).child(
                                        language.text(
                                            "Daniel Kim wants to merge a sample billing change",
                                            "Daniel Kim 请求合并一项示例账单改动",
                                        ),
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
