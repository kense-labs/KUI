//! Illustrative thought candidates and scores; no model is queried.
use gpui::{prelude::*, *};
use kui::{
    components::reasoning_tree::{ReasoningTree, Thought},
    primitives::{control::UiLanguage, family::Paint},
};
pub const IDS: &[&str] = &["reasoning-tree"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    view: Entity<ReasoningTree>,
    language: UiLanguage,
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let thoughts = vec![
            Thought::new("small", t("Ship a small release", "先发布小版本"))
                .score(0.86)
                .children(vec![
                    Thought::new("scope", t("Reduce scope", "缩小范围")).score(0.72),
                    Thought::new("flags", t("Use a feature flag", "使用功能开关")).score(0.93),
                ]),
            Thought::new("delay", t("Delay the release", "延后发布"))
                .score(0.75)
                .children(vec![
                    Thought::new("review", t("Review dependencies", "检查依赖")).score(0.81),
                ]),
            Thought::new("parallel", t("Parallel work", "并行推进")).score(0.62),
        ];
        let view = cx.new(|cx| {
            ReasoningTree::new(
                t("How can the release ship by Friday?", "怎样在周五前发布？"),
                thoughts,
                cx,
            )
        });
        view.update(cx, |tree, cx| {
            tree.step(cx);
            tree.set_playing(true, cx);
        });
        Self { view, language }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.language.text(
                        "Illustrative scores — no model is called",
                        "分数仅作演示，不调用模型",
                    )),
            )
            .child(div().w_full().max_w(px(760.)).child(self.view.clone()))
    }
}
