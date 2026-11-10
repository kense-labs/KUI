//! Simulated document edits: no assistant is contacted and nothing is saved.
use gpui::{prelude::*, *};
use kui::{
    components::agent_cursor::{
        AgentCursor, AgentCursorEvent, AgentCursorStatus, DocBlock, DocBlockKind,
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, disabled_motion, face},
    },
};
use std::time::Duration;

pub const IDS: &[&str] = &["agent-cursor"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example {
    view: Entity<AgentCursor>,
    language: UiLanguage,
    fail_next: bool,
    failure_scenario: bool,
    playback: Option<Task<()>>,
    _subscription: Subscription,
    _autoplay: Task<()>,
}

fn draft(language: UiLanguage) -> Vec<DocBlock> {
    use DocBlockKind::{Heading, Paragraph, Title};
    let t = |en, zh| language.text(en, zh);
    vec![
        DocBlock::new(
            "title",
            Title,
            t("Workspace launch brief", "工作区发布计划"),
        ),
        DocBlock::new(
            "intro",
            Paragraph,
            t(
                "The next release brings a shared activity view to every Team workspace.",
                "下个版本将加入共享活动视图。",
            ),
        ),
        DocBlock::new("heading", Heading, t("Goals", "发布目标")),
        DocBlock::new(
            "body",
            Paragraph,
            t(
                "We want people to be able to find out what changed without needing to ask someone, since the last update led to a lot of questions and nobody was sure where to look.",
                "上次更新后有很多同事来问发生了什么，大家也不知道要到哪里找。我们希望这次大家可以自己看到有些什么变化，减少重复询问。",
            ),
        ),
        DocBlock::new(
            "next",
            Paragraph,
            t(
                "Start with five workpsaces and review the feedback on Friday.",
                "先在五个工区开放，周五检查反馈。",
            ),
        ),
    ]
}

async fn wait_visible(delay_ms: u64, view: &WeakEntity<AgentCursor>, cx: &mut AsyncApp) -> bool {
    let timer = cx.background_executor().clone();
    timer.timer(Duration::from_millis(delay_ms)).await;
    loop {
        match view.update(cx, |v, cx| {
            let visible = cx
                .try_global::<crate::StoryBridge>()
                .is_none_or(|bridge| (bridge.visible)());
            v.set_visible(visible, cx);
            v.is_visible()
        }) {
            Ok(true) => return true,
            Err(_) => return false,
            Ok(false) => timer.timer(Duration::from_millis(200)).await,
        }
    }
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|_| {
            AgentCursor::new(
                language.text("Sample document", "样例文档"),
                draft(language),
            )
            .with_meta(language.text("Maya Lee · Launch planning", "Maya Lee · 发布筹备"))
            .agent("Kense", rgb(0xc47148).into())
            .with_collaborator("Maya", rgb(0x8b70c7).into())
        });
        let weak = view.downgrade();
        let autoplay = cx.spawn(async move |_, cx| {
            if wait_visible(200, &weak, cx).await {
                let _ = weak.update(cx, |view, cx| {
                    if !disabled_motion(cx) && view.status() == AgentCursorStatus::Idle {
                        view.run(cx);
                    }
                });
            }
        });
        let subscription =
            cx.subscribe(&view, |s: &mut Self, view, event: &AgentCursorEvent, cx| {
                let AgentCursorEvent::Run { ticket } = event else {
                    if matches!(event, AgentCursorEvent::Stop { .. }) {
                        s.playback = None;
                    }
                    return;
                };
                let ticket = *ticket;
                let fail = std::mem::take(&mut s.fail_next);
                let language = s.language;
                let weak = view.downgrade();
                let reduced = disabled_motion(cx);
                s.playback = Some(cx.spawn(async move |_, cx| {
                    if !wait_visible(if reduced { 0 } else { 420 }, &weak, cx).await {
                        return;
                    }
                    if fail {
                        let _ = weak.update(cx, |view, cx| {
                            view.fail(
                                ticket,
                                language.text(
                                    "The connection dropped. Your draft is unchanged.",
                                    "连接已中断，原文已保留。可以重试。",
                                ),
                                cx,
                            );
                        });
                        return;
                    }
                    let mut blocks = draft(language);
                    let chunks: &[&str] = match language {
                        UiLanguage::Chinese => &[
                            "让团队随时查看",
                            "工作区的最新变化，",
                            "并在一个页面找到",
                            "需要跟进的事项。",
                        ],
                        _ => &[
                            "Make workspace changes easy",
                            " to find. A shared",
                            " activity view gives everyone",
                            " one place to see",
                            " updates and follow-up tasks.",
                        ],
                    };
                    if !reduced {
                        if !weak
                            .update(cx, |view, cx| {
                                view.set_activity_for(
                                    ticket,
                                    language.text("Reviewing the goals", "正在查看发布目标"),
                                    cx,
                                ) && view.set_target_for(ticket, "body", cx)
                            })
                            .unwrap_or(false)
                            || !wait_visible(320, &weak, cx).await
                            || !weak
                                .update(cx, |view, cx| view.select_target_for(ticket, "body", cx))
                                .unwrap_or(false)
                            || !wait_visible(420, &weak, cx).await
                        {
                            return;
                        }
                        let _ = weak.update(cx, |view, cx| {
                            view.set_activity_for(
                                ticket,
                                language.text("Tightening the goal statement", "正在精简发布目标"),
                                cx,
                            )
                        });
                        let mut text = String::new();
                        for chunk in chunks {
                            text.push_str(chunk);
                            blocks[3].text = text.clone().into();
                            if !weak
                                .update(cx, |view, cx| {
                                    view.set_blocks_for(ticket, blocks.clone(), cx)
                                })
                                .unwrap_or(false)
                                || !wait_visible(34, &weak, cx).await
                            {
                                return;
                            }
                        }
                    }
                    if !reduced && !wait_visible(180, &weak, cx).await {
                        return;
                    }
                    blocks[3].text = chunks.concat().into();
                    blocks.insert(
                        4,
                        DocBlock::new(
                            "rollout",
                            DocBlockKind::Heading,
                            language.text("Rollout", "发布节奏"),
                        ),
                    );
                    if !reduced
                        && (!weak
                            .update(cx, |view, cx| {
                                view.set_activity_for(
                                    ticket,
                                    language
                                        .text("Adding a rollout heading", "正在添加发布节奏标题"),
                                    cx,
                                ) && view.set_target_for(ticket, "rollout", cx)
                                    && view.set_blocks_for(ticket, blocks.clone(), cx)
                            })
                            .unwrap_or(false)
                            || !wait_visible(380, &weak, cx).await)
                    {
                        return;
                    }
                    if !reduced {
                        let typo = language.text("workpsaces", "工区");
                        if let Some(start) = blocks[5].text.find(typo)
                            && (!weak
                                .update(cx, |view, cx| {
                                    view.set_activity_for(
                                        ticket,
                                        language.text("Fixing a typo", "正在修正错字"),
                                        cx,
                                    ) && view.mark_correction_for(
                                        ticket,
                                        "next",
                                        start..start + typo.len(),
                                        cx,
                                    )
                                })
                                .unwrap_or(false)
                                || !wait_visible(520, &weak, cx).await)
                        {
                            return;
                        }
                    }
                    blocks[5].text = language
                        .text(
                            "Start with five workspaces and review the feedback on Friday.",
                            "先在五个工作区开放，周五检查反馈。",
                        )
                        .into();
                    if !reduced
                        && (!weak
                            .update(cx, |view, cx| {
                                view.set_blocks_for(ticket, blocks.clone(), cx)
                            })
                            .unwrap_or(false)
                            || !wait_visible(210, &weak, cx).await)
                    {
                        return;
                    }
                    let _ = weak.update(cx, |view, cx| view.finish(ticket, blocks, cx));
                }));
            });
        Self {
            view,
            language,
            fail_next: false,
            failure_scenario: false,
            playback: None,
            _subscription: subscription,
            _autoplay: autoplay,
        }
    }

    fn replay(&mut self, fail: bool, cx: &mut Context<Self>) {
        self.fail_next = fail;
        self.failure_scenario = fail;
        cx.notify();
        let language = self.language;
        self.view.update(cx, |view, cx| {
            view.stop(cx);
            view.undo(cx);
            view.set_blocks(draft(language), cx);
            view.run(cx);
        });
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let t = |en, zh| self.language.text(en, zh);
        div()
            .size_full()
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_start()
            .gap_3()
            .child(div().w_full().max_w(px(522.)).child(self.view.clone()))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(px(3.))
                    .p(px(4.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .child(
                        face(
                            "cursor-sim-success",
                            t("Edits land", "模拟成功"),
                            32.,
                            8.,
                            if self.failure_scenario {
                                Tone::Clear
                            } else {
                                Tone::Surface
                            },
                            cx,
                        )
                        .px_2()
                        .child(t("Edits land", "模拟成功"))
                        .on_click(cx.listener(|s, _, _, cx| s.replay(false, cx))),
                    )
                    .child(
                        face(
                            "cursor-sim-error",
                            t("Connection drops", "模拟断线"),
                            32.,
                            8.,
                            if self.failure_scenario {
                                Tone::Surface
                            } else {
                                Tone::Clear
                            },
                            cx,
                        )
                        .px_2()
                        .child(t("Connection drops", "模拟断线"))
                        .on_click(cx.listener(|s, _, _, cx| s.replay(true, cx))),
                    ),
            )
            .child(div().text_size(px(12.)).text_color(p.secondary).child(t(
                "Simulated edits — nothing is saved",
                "模拟修改，不会保存",
            )))
    }
}
