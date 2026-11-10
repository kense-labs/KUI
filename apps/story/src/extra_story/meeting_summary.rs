//! Sample meeting notes; sends and regeneration are simulated.
use gpui::{prelude::*, *};
use kui::{
    components::meeting_summary::{
        MeetingAction, MeetingData, MeetingPoint, MeetingQuote, MeetingStatus, MeetingSummary,
        MeetingSummaryEvent,
    },
    primitives::{
        control::UiLanguage,
        family::Paint,
        ops_surface::{btn, row},
    },
};
use std::time::Duration;
pub const IDS: &[&str] = &["meeting-summary"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    view: Entity<MeetingSummary>,
    language: UiLanguage,
    failed: bool,
    scenario: usize,
    generation: u64,
    _sub: Subscription,
}
fn sample(language: UiLanguage) -> MeetingData {
    let t = |en, zh| language.text(en, zh);
    let mut data = MeetingData::new(t("Annual billing launch review", "年度账单上线评审"));
    data.meta = t(
        "Video call · Tue, Oct 27 · 10:00–10:42 · 42 min",
        "视频会议 · 10 月 27 日 · 10:00–10:42 · 42 分钟",
    )
    .into();
    data.attendees = vec!["Maya".into(), "Alex".into(), "Lin".into()];
    data.summary = vec![
        MeetingPoint::new(
            "summary",
            t(
                "The team reviewed the release plan.",
                "团队讨论了发布计划。",
            ),
        )
        .quote(MeetingQuote::new(
            "Maya",
            "04:18",
            t(
                "Let's review the release schedule first.",
                "先看一下发布排期。",
            ),
        )),
    ];
    data.decisions = vec![MeetingPoint::new(
        "decision",
        t("Ship the smaller scope this week.", "本周先发布较小范围。"),
    )];
    data.actions = vec![MeetingAction::new(
        MeetingPoint::new(
            "action",
            t("Prepare the release checklist.", "准备发布检查清单。"),
        )
        .quote(MeetingQuote::new(
            "Alex",
            "18:32",
            t("I can prepare a checklist.", "我来准备检查清单。"),
        )),
        "Alex",
    )];
    data
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|_| MeetingSummary::new(sample(language)));
        let sub = cx.subscribe(
            &view,
            |s: &mut Self, view, event: &MeetingSummaryEvent, cx| match event {
                MeetingSummaryEvent::Send { ticket, .. } => {
                    let fail = !s.failed;
                    s.failed = true;
                    view.update(cx, |v, cx| {
                        v.finish_send(
                            *ticket,
                            if fail {
                                Err(s.language.text("Sample failure", "样例失败").into())
                            } else {
                                Ok("TASK-12".into())
                            },
                            cx,
                        )
                    });
                }
                MeetingSummaryEvent::Regenerate => {
                    s.show(0, cx);
                }
                _ => {}
            },
        );
        let mut this = Self {
            view,
            language,
            failed: false,
            scenario: 0,
            generation: 0,
            _sub: sub,
        };
        this.show(0, cx);
        this
    }
    fn show(&mut self, scenario: usize, cx: &mut Context<Self>) {
        self.scenario = scenario;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let language = self.language;
        self.view.update(cx, |v, cx| match scenario {
            1 => v.set_status(
                MeetingStatus::Error,
                language.text(
                    "Couldn't summarize the transcript. Try again.",
                    "无法整理转录记录，请重试。",
                ),
                cx,
            ),
            2 => v.set_status(MeetingStatus::NoTranscript, "", cx),
            _ => v.set_status(MeetingStatus::Summarizing, "", cx),
        });
        if scenario == 0 {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(1100))
                    .await;
                let _ = this.update(cx, |s, cx| {
                    if generation == s.generation {
                        s.view.update(cx, |v, cx| v.set_data(sample(language), cx));
                    }
                });
            })
            .detach();
        }
        cx.notify();
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
            .justify_start()
            .gap_3()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.language.text(
                        "Sample notes — actions are simulated",
                        "样例纪要，发送操作为模拟",
                    )),
            )
            .child(
                row().justify_center().flex_wrap().children(
                    [
                        ("Live", "正常"),
                        ("Error", "错误"),
                        ("No transcript", "无转录"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(i, (en, zh))| {
                        btn(
                            ("meeting-scenario", i),
                            self.language.text(en, zh),
                            self.scenario == i,
                            cx,
                            move |s, _, cx| s.show(i, cx),
                        )
                    }),
                ),
            )
            .child(div().w_full().max_w(px(640.)).child(self.view.clone()))
    }
}
