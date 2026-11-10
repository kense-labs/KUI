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
                "punctuation",
                t("Punctuation", "标点"),
                t("Punctuation and delimiters", "标点与分隔符"),
            ),
            MoeExpert::new(
                "names",
                t("Names", "名称"),
                t("Names and entities", "名称与实体"),
            ),
            MoeExpert::new(
                "grammar",
                t("Grammar", "语法"),
                t("Sentence structure", "句子结构"),
            ),
            MoeExpert::new(
                "verbs",
                t("Verbs", "动词"),
                t("Actions and verbs", "动作与动词"),
            ),
            MoeExpert::new(
                "dates",
                t("Dates", "日期"),
                t("Dates and times", "日期与时间"),
            ),
            MoeExpert::new(
                "symbols",
                t("Symbols", "符号"),
                t("Operators and symbols", "运算符与符号"),
            ),
        ];
        let tokens = [
            ("Maya", 3, 5),
            (" paid", 5, 4),
            (" $", 7, 1),
            ("1", 1, 7),
            (",", 2, 1),
            ("240", 1, 7),
            (" on", 4, 6),
            (" Oct", 6, 3),
            (" 21", 6, 1),
            (",", 2, 4),
            (" then", 4, 5),
            (" ran", 5, 4),
            (" npm", 0, 3),
            (" test", 0, 5),
            (".", 2, 4),
        ]
        .into_iter()
        .map(|(token, first, second)| {
            if token == "Maya" {
                return MoeToken::new(token, vec![0.04, 0.07, 0.06, 0.46, 0.08, 0.21, 0.03, 0.05]);
            }
            let mut scores = vec![0.02; 8];
            scores[first] = 0.67;
            scores[second] = 0.21;
            MoeToken::new(token, scores)
        })
        .collect();
        let diagram = cx.new(|_| {
            MixtureOfExperts::new(experts, tokens)
                .with_capacity_factor(1.)
                .with_parameters(671., 5., 16.)
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
