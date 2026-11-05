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
            t("Review logs", "查看日志"),
            1200,
            false,
            t("Check needs another pass", "检查需要再次执行"),
        );
        one.calls.push(LoopCall::new(
            "Workspace",
            t("Inspect build", "检查构建"),
            t("A dependency is stale", "依赖需要更新"),
        ));
        let mut two = LoopIteration::new(
            t("Configuration", "读取配置"),
            800,
            true,
            t("Build is healthy", "构建正常"),
        );
        two.calls.push(LoopCall::new(
            "Workspace",
            t("Update and verify", "更新并验证"),
            t("Checks passed", "检查通过"),
        ));
        let view = cx.new(|_| {
            AgentLoop::new(
                t(
                    "Investigate a build and verify the fix.",
                    "排查构建问题并验证修复。",
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
