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
        let mut events = LoopIteration::new(
            t("Checkout events", "结账事件"),
            1200,
            false,
            t("Issuer errors lack route IDs", "拒付日志缺少路由编号"),
        );
        events.detail = t("Recent payment failures", "最近的支付失败记录").into();
        events.calls.push(LoopCall::new(
            "Logs",
            t("Find declines", "查找拒付记录"),
            t("41 EU declines found", "找到 41 条欧洲拒付记录"),
        ));

        let mut routes = LoopIteration::new(
            t("Provider logs", "支付服务商日志"),
            6400,
            false,
            t("Alternate route needs a replay", "需重放备用路由请求"),
        );
        routes.detail = t("Issuer responses and routes", "发卡行响应及支付路由").into();
        routes.calls.push(LoopCall {
            failed: true,
            ..LoopCall::new(
                "Routes",
                t("Compare regions", "比较地区"),
                t("Provider lookup timed out", "服务商查询超时"),
            )
        });
        routes.calls.push(LoopCall::new(
            "Routes",
            t("Retry lookup", "重试查询"),
            t("Fallback route isolated", "已定位备用路由"),
        ));

        let mut replay = LoopIteration::new(
            t("Replay results", "请求重放结果"),
            9800,
            true,
            t("All sample payments pass", "样例支付全部通过"),
        );
        replay.detail = t("Replay before and after the fix", "修复前后的重放对比").into();
        replay.calls.push(LoopCall::new(
            "Tests",
            t("Replay checkout", "重放结账"),
            t("12 of 12 passed", "12 项检查全部通过"),
        ));
        replay.calls.push(LoopCall::new(
            "Notes",
            t("Summarize results", "整理结果"),
            t("Update drafted for the team", "已起草团队更新"),
        ));

        let visible = cx
            .try_global::<crate::StoryBridge>()
            .map(|bridge| bridge.visible);
        let view = cx.new(|_| {
            let agent = AgentLoop::new(
                t(
                    "Find why EU cards fail at checkout, verify the fix, and summarize it for the team.",
                    "查明欧洲银行卡结账失败的原因，验证修复并向团队汇报。",
                ),
                vec![events, routes, replay],
            )
            .title(t("Agent loop", "智能体循环"))
            .summary(t(
                "Isolated the fallback route and verified the fix against 12 sample payments.",
                "已定位备用路由，并用 12 笔样例支付验证了修复。",
            ))
            .autoplay(true);
            if let Some(visible) = visible { agent.visibility(visible) } else { agent }
        });
        Self { view, language }
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let width = f32::from(w.viewport_size().width);
        let gutter = ((width - 522.) / 2.).clamp(16., 52.);
        div()
            .w_full()
            .py_4()
            .px(px(gutter))
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(div().w_full().max_w(px(760.)).child(self.view.clone()))
            .child(
                div()
                    .text_center()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.language.text(
                        "Sample timeline only. No tools are called.",
                        "仅演示流程，不会调用真实工具。",
                    )),
            )
    }
}
