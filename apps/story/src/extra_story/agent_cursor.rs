//! Simulated document edits: no assistant is contacted and nothing is saved.
use gpui::{prelude::*, *};
use kui::{
    components::agent_cursor::{
        AgentCursor, AgentCursorEvent, AgentCursorStatus, DocBlock, DocBlockKind,
    },
    primitives::{control::UiLanguage, family::Paint},
};
use std::time::Duration;
pub const IDS: &[&str] = &["agent-cursor"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    view: Entity<AgentCursor>,
    language: UiLanguage,
    failed: bool,
    _sub: Subscription,
}
fn blocks(language: UiLanguage, edited: bool) -> Vec<DocBlock> {
    let t = |en, zh| language.text(en, zh);
    vec![
        DocBlock::new("title", DocBlockKind::Title, t("Release notes", "发布说明")),
        DocBlock::new("heading", DocBlockKind::Heading, t("Plan", "计划")),
        DocBlock::new(
            "body",
            DocBlockKind::Paragraph,
            if edited {
                t(
                    "Begin with a small release, check the results, and then expand to everyone.",
                    "先小范围发布，检查结果后再逐步开放给所有用户。",
                )
            } else {
                t(
                    "Release to a few people first and see if it works before telling everybody.",
                    "先给少量用户发布，确认可用后再通知所有人。",
                )
            },
        ),
        DocBlock::new(
            "next",
            DocBlockKind::Paragraph,
            t(
                "The team will review progress on Friday.",
                "团队将在周五检查进度。",
            ),
        ),
    ]
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|_| {
            AgentCursor::new(
                language.text("Sample document", "样例文档"),
                blocks(language, false),
            )
        });
        let initial = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        view.update(cx, |v, cx| v.set_visible(initial, cx));
        let weak = view.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                if weak
                    .update(cx, |v, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        v.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let sub = cx.subscribe(&view, |s: &mut Self, view, event: &AgentCursorEvent, cx| {
            if let AgentCursorEvent::Run { ticket } = event {
                let ticket = *ticket;
                let fail = !s.failed;
                s.failed = true;
                let language = s.language;
                let weak = view.downgrade();
                cx.spawn(async move |_, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(300))
                        .await;
                    if weak
                        .update(cx, |v, cx| {
                            v.set_status_for(ticket, AgentCursorStatus::Editing, cx);
                            v.set_target_for(ticket, "body", cx);
                        })
                        .is_err()
                    {
                        return;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(700))
                        .await;
                    if weak
                        .update(cx, |v, cx| {
                            v.set_blocks_for(ticket, blocks(language, true), cx);
                        })
                        .is_err()
                    {
                        return;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(700))
                        .await;
                    let _ = weak.update(cx, |v, cx| {
                        if fail {
                            v.fail(
                                ticket,
                                language
                                    .text("Sample run failed; retry.", "样例修改失败，请重试。"),
                                cx,
                            );
                        } else {
                            v.finish(ticket, blocks(language, true), cx);
                        }
                    });
                })
                .detach();
            }
        });
        Self {
            view,
            language,
            failed: false,
            _sub: sub,
        }
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
                div().text_size(px(12.)).text_color(p.secondary).child(
                    self.language
                        .text("Simulated edits — nothing is saved", "模拟修改，不会保存"),
                ),
            )
            .child(div().w_full().max_w(px(680.)).child(self.view.clone()))
    }
}
