//! Illustrative timeline; no agent or tool calls are made.
use gpui::{prelude::*, *};
use kui::{
    components::agent_loop::{AgentLoop, LoopCall, LoopIteration},
    primitives::{control::UiLanguage, family::Paint},
};
pub const IDS: &[&str] = &["agent-loop"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    view: Entity<AgentLoop>,
    language: UiLanguage,
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let mut one = LoopIteration::new(
            t("Checkout events", "结账事件"),
            1200,
            false,
            t("Check needs another pass", "检查需要再次执行"),
        );
        one.calls.push(LoopCall::new(
            "Workspace",
            t("Check decline codes", "检查拒付代码"),
            t("EU issuer declines increased", "欧洲发卡行拒付上升"),
        ));
        let mut two = LoopIteration::new(
            t("Payment provider logs", "支付服务商日志"),
            800,
            true,
            t("Verified against payment events", "已与支付事件核对"),
        );
        two.calls.push(LoopCall::new(
            "Workspace",
            t("Compare card routes", "对比银行卡路由"),
            t("Checks passed", "检查通过"),
        ));
        let view = cx.new(|_| {
            AgentLoop::new(
                t(
                    "Find out why EU cards are declined at checkout, then update the team.",
                    "查明欧洲银行卡在结账时被拒的原因，再向团队汇报。",
                ),
                vec![one, two],
            )
            .title(t("Agent loop", "智能体循环"))
            .summary(t("The sample run completed.", "样例流程已完成。"))
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
                        "Illustration only — pause during movement to inspect it",
                        "演示流程；移动中可暂停查看，不会调用真实工具",
                    )),
            )
            .child(div().w_full().max_w(px(760.)).child(self.view.clone()))
    }
}
