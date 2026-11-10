//! Simulated support handoff and sends. No service is contacted.
use gpui::{prelude::*, *};
use kui::{
    components::agent_handoff::{
        AgentHandoff, AgentHandoffEvent, HandoffAuthor, HandoffMessage, HandoffStage,
        HandoffSummary,
    },
    primitives::{
        control::UiLanguage,
        family::Paint,
        ops_surface::{btn, row},
    },
};
use std::time::Duration;
pub const IDS: &[&str] = &["agent-handoff"];
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Preview::new(w, cx)).into())
}
struct Preview {
    example: Entity<Example>,
    unavailable: bool,
}

impl Preview {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            example: cx.new(|cx| Example::new(false, w, cx)),
            unavailable: false,
        }
    }
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        div()
            .size_full()
            .min_h(px(684.))
            .p_4()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(p.stage)
            .text_color(p.ink)
            .child(
                row().justify_center().flex_wrap().children(
                    [
                        (false, lang.text("Person joins", "人工接入")),
                        (true, lang.text("No one available", "暂无客服")),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(i, (unavailable, label))| {
                        btn(
                            ("handoff-scenario", i),
                            label,
                            self.unavailable == unavailable,
                            cx,
                            move |v, w, cx| {
                                v.unavailable = unavailable;
                                v.example = cx.new(|cx| Example::new(unavailable, w, cx));
                                cx.notify();
                            },
                        )
                    }),
                ),
            )
            .child(self.example.clone())
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(lang.text(
                        "Simulated handoff — no messages are sent",
                        "模拟转接，不会发送消息",
                    )),
            )
    }
}

struct Example {
    view: Entity<AgentHandoff>,
    language: UiLanguage,
    unavailable: bool,
    _sub: Subscription,
}
impl Example {
    fn new(unavailable: bool, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let mut summary = HandoffSummary::new(t(
            "A customer needs help with an order.",
            "客户需要订单方面的帮助。",
        ));
        summary.fields = vec![(
            t("Request", "请求").into(),
            t("Talk to support", "联系支持").into(),
        )];
        let view = cx.new(|cx| {
            AgentHandoff::new(
                t("Order NW-48213", "订单 NW-48213"),
                t("Order delivery", "订单配送"),
                summary,
                w,
                cx,
            )
            .subtitle(t("Kense support", "Kense 客服"))
            .estimate(t("Typical wait: 2 min", "预计等待：2 分钟"))
        });
        view.update(cx, |v, cx| {
            v.set_messages(
                vec![
                    HandoffMessage::new(
                        "one",
                        HandoffAuthor::Customer,
                        t("Can you help with my order?", "可以帮我处理订单吗？"),
                    ),
                    HandoffMessage::new(
                        "two",
                        HandoffAuthor::Assistant,
                        t(
                            "I can share the details with a person.",
                            "我可以把情况转交给人工客服。",
                        ),
                    ),
                ],
                cx,
            );
        });
        let sub = cx.subscribe_in(
            &view,
            w,
            |s: &mut Self, view, event: &AgentHandoffEvent, w, cx| match event {
                AgentHandoffEvent::Request { ticket } => {
                    let ticket = *ticket;
                    let fail = s.unavailable;
                    let language = s.language;
                    let weak = view.downgrade();
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(1200))
                            .await;
                        let _ = weak.update(cx, |v, cx| {
                            if fail {
                                v.fail_transfer(
                                    ticket,
                                    language.text(
                                        "Everyone is busy right now. You can keep chatting or retry later.",
                                        "客服都在忙。可以继续聊天，或稍后重试。",
                                    ),
                                    cx,
                                );
                            } else if v.connect(ticket, language.text("Maya", "小林"), cx) {
                                v.append_message(
                                    HandoffMessage::new(
                                        "agent-greeting",
                                        HandoffAuthor::Agent,
                                        language.text(
                                            "I have the details. How can I help?",
                                            "我已收到情况，有什么可以帮你？",
                                        ),
                                    )
                                    .after_handoff(),
                                    cx,
                                );
                            }
                        });
                    })
                    .detach();
                }
                AgentHandoffEvent::Send { ticket, text } => {
                    let ticket = *ticket;
                    let text = text.clone();
                    view.update(cx, |v, cx| {
                        let after = v.stage() != HandoffStage::Assistant;
                        if v.finish_send(ticket, Ok(()), w, cx) {
                            let mut message = HandoffMessage::new(
                                format!("message-{ticket}"), HandoffAuthor::Customer, text,
                            );
                            if after { message = message.after_handoff(); }
                            v.append_message(message, cx);
                        }
                    });
                }
                AgentHandoffEvent::Cancel { .. } | AgentHandoffEvent::CancelSend { .. } => {}
            },
        );
        Self {
            view,
            language,
            unavailable,
            _sub: sub,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w_full().max_w(px(560.)).child(self.view.clone())
    }
}
