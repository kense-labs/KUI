//! Illustrative scores: these are not measurements from a real model.
use gpui::{prelude::*, *};
use kui::{
    components::mixture_of_experts::{MixtureOfExperts, MoeExpert, MoeToken},
    primitives::{control::UiLanguage, family::Paint},
};
use std::time::Duration;
pub const IDS: &[&str] = &["mixture-of-experts"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    diagram: Entity<MixtureOfExperts>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let experts = vec![
            MoeExpert::new(
                "code",
                t("Code", "代码"),
                t("Identifiers and syntax", "标识符与语法"),
            ),
            MoeExpert::new(
                "numbers",
                t("Numbers", "数字"),
                t("Quantities and operators", "数量与运算符"),
            ),
            MoeExpert::new(
                "language",
                t("Language", "语言"),
                t("Natural language", "自然语言"),
            ),
            MoeExpert::new(
                "symbols",
                t("Symbols", "符号"),
                t("Punctuation and symbols", "标点与符号"),
            ),
        ];
        let tokens = vec![
            MoeToken::new("let", vec![0.65, 0.05, 0.2, 0.1]),
            MoeToken::new(" total", vec![0.45, 0.2, 0.3, 0.05]),
            MoeToken::new(" =", vec![0.25, 0.15, 0.05, 0.55]),
            MoeToken::new(" 42", vec![0.2, 0.6, 0.15, 0.05]),
            MoeToken::new(" +", vec![0.15, 0.35, 0.05, 0.45]),
            MoeToken::new(" cost", vec![0.3, 0.1, 0.55, 0.05]),
        ];
        let diagram = cx.new(|_| {
            MixtureOfExperts::new(experts, tokens)
                .with_capacity_factor(0.8)
                .with_parameters(120., 5., 12.)
                .with_simulated(true)
        });
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        diagram.update(cx, |d, cx| d.set_visible(visible, cx));
        let weak = diagram.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if weak
                    .update(cx, |d, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        d.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self { diagram }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .items_center()
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.diagram.clone())
    }
}
