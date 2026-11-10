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
    data.attendees = ["Maya", "Alex", "Lin", "Noah", "Sam", "Jo"]
        .into_iter()
        .map(Into::into)
        .collect();
    let point = |id, en, zh, speaker, time, quote_en, quote_zh| {
        MeetingPoint::new(id, t(en, zh)).quote(MeetingQuote::new(
            speaker,
            time,
            t(quote_en, quote_zh),
        ))
    };
    data.summary = vec![
        point(
            "scope",
            "Annual billing is ready for a staged release. Monthly plans stay unchanged.",
            "年度账单已准备好分批发布，月度方案保持原样。",
            "Maya",
            "04:18",
            "Start with annual billing. We don't need to touch monthly plans.",
            "先发布年度账单，月度方案不用动。",
        ),
        point(
            "pilot",
            "Start with five pilot workspaces and check invoice totals before widening access.",
            "先向五个工作区开放试用，核对账单金额后再扩大范围。",
            "Lin",
            "09:52",
            "Five pilot workspaces will give us enough invoices to verify.",
            "五个试用工作区就能提供足够的账单来核对。",
        ),
        point(
            "support",
            "Support needs a short note explaining credits and invoice dates.",
            "客服需要一份简短说明，解释抵扣额度和开票日期。",
            "Jo",
            "15:06",
            "People are going to ask how credits appear on their first invoice.",
            "用户可能会问抵扣额度如何显示在首张账单上。",
        ),
        point(
            "watch",
            "Review the first day of payment failures and decide whether to expand on Thursday.",
            "观察首日的付款失败情况，再决定周四是否扩大开放。",
            "Noah",
            "28:10",
            "Let's look at a full day of failures before we expand.",
            "扩大范围前，先看一整天的付款失败情况。",
        ),
    ];
    data.decisions = vec![
        point(
            "rollout",
            "Release to pilot workspaces first.",
            "优先向试用工作区发布。",
            "Maya",
            "31:05",
            "Pilot first, then open it up when the invoices check out.",
            "先试用，账单核对无误后再开放。",
        ),
        point(
            "threshold",
            "Pause rollout if the payment failure rate exceeds 2%.",
            "付款失败率超过 2% 时暂停发布。",
            "Lin",
            "33:14",
            "Two percent is our cutoff.",
            "以百分之二为暂停阈值。",
        ),
        point(
            "faq",
            "Publish the billing FAQ alongside the release.",
            "随发布一并上线账单常见问题。",
            "Jo",
            "36:24",
            "The FAQ should go live when the pilot starts.",
            "试用开始时就要上线常见问题。",
        ),
    ];
    data.actions = [
        (
            point(
                "checklist",
                "Prepare and share the rollout checklist.",
                "准备并分享发布检查清单。",
                "Alex",
                "18:32",
                "I'll send a checklist before end of day.",
                "我今天结束前发一份检查清单。",
            ),
            "Alex",
            t("Today", "今天"),
            true,
        ),
        (
            point(
                "invoices",
                "Compare the first five annual invoices against the ledger.",
                "将首批五张年度账单与台账逐一核对。",
                "Lin",
                "26:18",
                "I'll match those invoices against the ledger.",
                "我会把这些账单与台账核对。",
            ),
            "Lin",
            t("Tomorrow", "明天"),
            false,
        ),
        (
            point(
                "draft",
                "Draft the support FAQ for credit balances and invoice dates.",
                "起草抵扣余额与开票日期的客服说明。",
                "Jo",
                "36:24",
                "I'll cover credits and billing dates in the FAQ.",
                "我会在常见问题里说明抵扣和开票日期。",
            ),
            "Jo",
            t("Wednesday", "周三"),
            false,
        ),
        (
            point(
                "dashboard",
                "Add a payment-failure alert to the rollout dashboard.",
                "在发布仪表板中添加付款失败告警。",
                "Noah",
                "39:40",
                "I'll wire the alert into the rollout dashboard.",
                "我会把告警接到发布仪表板上。",
            ),
            "Noah",
            t("Thursday", "周四"),
            false,
        ),
    ]
    .into_iter()
    .map(|(point, owner, due, done)| {
        let mut action = MeetingAction::new(point, owner);
        action.due = due.into();
        action.done = done;
        action
    })
    .collect();
    data
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|_| MeetingSummary::new(sample(language)).with_transcript("Kense", 4182));
        let sub = cx.subscribe(
            &view,
            |s: &mut Self, view, event: &MeetingSummaryEvent, cx| match event {
                MeetingSummaryEvent::Send { ticket, .. } => {
                    let fail = !s.failed;
                    s.failed = true;
                    let ticket = *ticket;
                    let view = view.downgrade();
                    let error = s.language.text("Sample failure", "样例失败");
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(380))
                            .await;
                        let _ = view.update(cx, |v, cx| {
                            v.finish_send(
                                ticket,
                                if fail {
                                    Err(error.into())
                                } else {
                                    Ok("TASK-12".into())
                                },
                                cx,
                            );
                        });
                    })
                    .detach();
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
            .child(div().w_full().max_w(px(522.)).child(self.view.clone()))
    }
}
