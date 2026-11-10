//! Interactive examples of additional selection controls.
use gpui::{prelude::*, *};
use kui::{
    components::{
        announcement_bar::{Announcement, AnnouncementBar},
        ask_your_data::{AskDataEvent, AskDataResult, AskDataSeries, AskDataStatus, AskYourData},
        billing_toggle::BillingToggle,
        cookie_consent::{ConsentCategory, CookieConsent},
        countdown::Countdown,
        elastic_slider::ElasticSlider,
        enrich_column::{EnrichColumn, EnrichEvent, EnrichResult, EnrichRow, EnrichSource},
        glass_menu::{GlassMenu, GlassMenuGroup, GlassMenuItem, GlassMenuSelected},
        glass_segmented::{GlassSegmentChanged, GlassSegmentOption, GlassSegmented},
        mega_menu_morph::{
            MegaMenuFeature, MegaMenuGroup, MegaMenuLink, MegaMenuMorph, MegaMenuNavigate,
            MegaMenuSection,
        },
        money_input::MoneyInput,
        morph_select::{MorphOption, MorphSelect},
        pii_redaction::{PiiEvent, PiiKind, PiiRedaction, PiiSpan},
        presence_stack::{
            PresenceAlign, PresencePerson, PresenceSelected, PresenceStack, PresenceStatus,
        },
        range_histogram_slider::RangeHistogramSlider,
        reply_draft::{
            ReplyDraft, ReplyDraftEvent, ReplyHint, ReplyLength, ReplyMessage, ReplyTone,
        },
        ruler_slider::RulerSlider,
        scrub_input::ScrubInput,
        signature_pad::{SignaturePad, SignaturePngExported},
        smart_form_fill::{SmartField, SmartFillEvent, SmartFormFill, SmartSuggestion},
        ticket_router::{Ticket, TicketDecision, TicketLane, TicketRouter, TicketRouterEvent},
        tldr_toggle::{TldrArticle, TldrEvent, TldrMode, TldrParagraph, TldrPoint, TldrToggle},
        toggle_group::{ToggleGroup, ToggleMode, ToggleOption},
        tree_table::{TreeTable, TreeTableEvent, TreeTableRow},
        undo_toast::{UndoItem, UndoToast, UndoToastEvent},
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
    },
};

pub const IDS: &[&str] = &[
    "rating",
    "billing-toggle",
    "toggle-group",
    "radio-cards",
    "money-input",
    "countdown",
    "elastic-slider",
    "range-histogram-slider",
    "announcement-bar",
    "cookie-consent",
    "ruler-slider",
    "signature-pad",
    "scrub-input",
    "morph-select",
    "undo-toast",
    "tree-table",
    "mega-menu-morph",
    "glass-segmented",
    "glass-menu",
    "presence-stack",
    "tldr-toggle",
    "smart-form-fill",
    "enrich-column",
    "ticket-router",
    "reply-draft",
    "ask-your-data",
    "pii-redaction",
];

struct InputExamples {
    views: Vec<AnyView>,
    width: f32,
    padded: bool,
}
struct SmartFillExample {
    control: Entity<SmartFormFill>,
    fail_next: bool,
    note: SharedString,
    _subscription: Subscription,
}
impl SmartFillExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let fields = vec![
            SmartField::new("name", l.text("Full name", "全名")),
            SmartField::new("role", l.text("Job title", "职位")),
            SmartField::new("company", l.text("Company", "公司")),
            SmartField::new("email", l.text("Email", "邮箱")),
            SmartField::new("phone", l.text("Phone", "电话")),
            SmartField::new("address", l.text("Address", "地址")),
        ];
        let control = cx.new(|cx| SmartFormFill::new(fields, w, cx));
        let source = control.read(cx).source();
        source.update(cx, |field, cx| field.set_value("Name: Maya Okafor\nRole: Head of Partnerships\nCompany: Stripe\nEmail: maya@example.com\nPhone: +1 415 555 0100\nAddress: San Francisco, CA", w, cx));
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &SmartFillEvent, w, cx| match event {
                SmartFillEvent::Requested { id, text } => {
                    let id = *id;
                    let text = text.to_string();
                    let entity = entity.clone();
                    let fail = s.fail_next;
                    s.fail_next = false;
                    let lang = UiLanguage::current(cx);
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(600))
                            .await;
                        if fail {
                            let _ = cx.update(|_, cx| {
                                entity.update(cx, |form, cx| {
                                    form.fail(
                                        id,
                                        lang.text(
                                            "Sample extraction failed. Try again.",
                                            "样例提取失败，请重试。",
                                        ),
                                        cx,
                                    );
                                })
                            });
                            return;
                        }
                        for (key, prefix) in [
                            ("name", "Name:"),
                            ("email", "Email:"),
                            ("company", "Company:"),
                            ("role", "Role:"),
                            ("phone", "Phone:"),
                            ("address", "Address:"),
                        ] {
                            let Some(line) = text
                                .lines()
                                .find(|line| line.trim_start().starts_with(prefix))
                            else {
                                continue;
                            };
                            let value = line.trim_start().strip_prefix(prefix).unwrap_or("").trim();
                            if value.is_empty() {
                                continue;
                            }
                            let confidence = if key == "role" { 0.72 } else { 0.96 };
                            let suggestion =
                                SmartSuggestion::new(key, value.to_owned(), confidence)
                                    .with_source(value.to_owned());
                            let applied = cx
                                .update(|w, cx| {
                                    entity.update(cx, |form, cx| form.apply(id, suggestion, w, cx))
                                })
                                .unwrap_or(false);
                            if !applied {
                                return;
                            }
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(140))
                                .await;
                        }
                        let _ = cx.update(|_, cx| {
                            entity.update(cx, |form, cx| {
                                form.finish(id, cx);
                            })
                        });
                    })
                    .detach();
                }
                SmartFillEvent::Submitted(_) => {
                    s.note = UiLanguage::current(cx)
                        .text("Example submitted; no data saved", "示例已提交，未保存数据")
                        .into();
                    cx.notify();
                }
                SmartFillEvent::Cancelled(_) => {}
            },
        );
        control.update(cx, |control, cx| {
            control.start(cx);
        });
        Self {
            control,
            fail_next: false,
            note: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for SmartFillExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let failure = if self.fail_next {
            l.text("Next request will fail", "下次请求将失败")
        } else {
            l.text("Simulate a failed request", "模拟一次请求失败")
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(self.control.clone())
            .child(div().text_size(px(12.)).child(l.text(
                "Sample parser runs locally; no text is sent.",
                "样例解析仅在本地运行，不会发送文本。",
            )))
            .child(
                div()
                    .id("smart-example-fail")
                    .role(Role::Button)
                    .tab_index(0)
                    .cursor_pointer()
                    .text_size(px(12.))
                    .child(failure)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.fail_next = !s.fail_next;
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.fail_next = !s.fail_next;
                            cx.notify();
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    })),
            )
            .child(div().text_size(px(12.)).child(self.note.clone()))
    }
}
#[derive(Clone, Copy, Default)]
enum AskSample {
    #[default]
    Signups,
    Revenue,
    Churn,
}
impl AskSample {
    fn questions(l: UiLanguage) -> [&'static str; 3] {
        [
            l.text(
                "Weekly signups by plan since August",
                "八月以来每周各套餐的注册量",
            ),
            l.text(
                "Monthly recurring revenue by region in 2026",
                "2026 年各地区每月经常性收入",
            ),
            l.text(
                "Monthly churn rate by plan since May",
                "五月以来各套餐的每月流失率",
            ),
        ]
    }
    fn from_question(q: &str) -> Option<Self> {
        let q = q.to_lowercase();
        if q.contains("revenue") || q.contains("收入") {
            Some(Self::Revenue)
        } else if q.contains("churn") || q.contains("流失") {
            Some(Self::Churn)
        } else if q.contains("signup") || q.contains("注册") {
            Some(Self::Signups)
        } else {
            None
        }
    }
    fn sql(self) -> &'static str {
        match self {
            Self::Signups => {
                "-- Weekly signups for completed weeks
SELECT
  date_trunc('week', u.created_at)::date AS week,
  p.name AS plan,
  count(*) AS signups
FROM sample_users u
JOIN sample_subscriptions s ON u.id = s.user_id
JOIN sample_plans p ON p.id = s.plan_id
WHERE u.created_at >= DATE '2026-08-03'
  AND u.created_at < DATE '2026-10-26'
GROUP BY week, plan
ORDER BY week, plan;"
            }
            Self::Revenue => {
                "-- Monthly recurring revenue by region
SELECT
  date_trunc('month', recorded_at)::date AS month,
  region,
  sum(monthly_amount) AS recurring_revenue
FROM sample_revenue
WHERE recorded_at >= DATE '2026-01-01'
GROUP BY month, region
ORDER BY month, region;"
            }
            Self::Churn => {
                "-- Monthly churn rate by plan
SELECT
  date_trunc('month', recorded_at)::date AS month,
  plan,
  avg(churn_pct) AS churn_rate
FROM sample_churn
WHERE recorded_at >= DATE '2026-05-01'
GROUP BY month, plan
ORDER BY month, plan;"
            }
        }
    }
    fn result(self, l: UiLanguage) -> AskDataResult {
        let t = |en, zh| l.text(en, zh);
        let (title, months, series, insight) = match self {
            Self::Signups => (
                t("Weekly signups by plan", "每周各套餐的注册量"),
                vec![
                    t("Aug 3", "8月3日"),
                    t("Aug 10", "8月10日"),
                    t("Aug 17", "8月17日"),
                    t("Aug 24", "8月24日"),
                    t("Aug 31", "8月31日"),
                    t("Sep 7", "9月7日"),
                    t("Sep 14", "9月14日"),
                    t("Sep 21", "9月21日"),
                    t("Sep 28", "9月28日"),
                    t("Oct 5", "10月5日"),
                    t("Oct 12", "10月12日"),
                    t("Oct 19", "10月19日"),
                ],
                vec![
                    AskDataSeries::new(
                        t("Free", "免费"),
                        vec![59., 61., 62., 59., 48., 55., 60., 59., 60., 58., 60., 60.],
                    ),
                    AskDataSeries::new(
                        "Pro",
                        vec![24., 26., 28., 29., 27., 31., 33., 34., 34., 35., 37., 40.],
                    ),
                    AskDataSeries::new(
                        t("Team", "团队"),
                        vec![9., 10., 11., 11., 12., 13., 13., 14., 16., 17., 18., 21.],
                    ),
                    AskDataSeries::new(
                        t("Enterprise", "企业"),
                        vec![2., 2., 3., 2., 3., 3., 3., 3., 4., 4., 5., 6.],
                    ),
                ],
                t(
                    "Weekly Pro signups rose from 24 to 40. Team rose from 9 to 21, while Free stayed close to 60 after the late-August dip.",
                    "Pro 每周注册量从 24 增至 40，团队套餐从 9 增至 21。免费套餐在八月底回落后回到 60 左右。",
                ),
            ),
            Self::Revenue => (
                t("Recurring revenue by region", "各地区经常性收入"),
                vec![
                    t("Jan", "一月"),
                    t("Feb", "二月"),
                    t("Mar", "三月"),
                    t("Apr", "四月"),
                    t("May", "五月"),
                    t("Jun", "六月"),
                ],
                vec![
                    AskDataSeries::new(t("Americas", "美洲"), vec![69., 73., 84., 88., 92., 97.]),
                    AskDataSeries::new(t("Europe", "欧洲"), vec![42., 44., 51., 58., 61., 67.]),
                    AskDataSeries::new(
                        t("Asia Pacific", "亚太"),
                        vec![30., 36., 38., 43., 46., 54.],
                    ),
                ],
                t(
                    "Europe added 25 units and Asia Pacific added 24. The Americas stayed the largest region through June.",
                    "欧洲增加 25，亚太增加 24。截至六月，美洲仍是收入最高的地区。",
                ),
            ),
            Self::Churn => (
                t("Monthly churn rate by plan", "各套餐每月流失率"),
                vec![
                    t("May", "五月"),
                    t("Jun", "六月"),
                    t("Jul", "七月"),
                    t("Aug", "八月"),
                    t("Sep", "九月"),
                    t("Oct", "十月"),
                ],
                vec![
                    AskDataSeries::new(t("Free", "免费"), vec![6.2, 6.0, 5.8, 5.5, 5.2, 5.0]),
                    AskDataSeries::new("Pro", vec![4.7, 4.2, 4.1, 4.0, 3.7, 3.4]),
                    AskDataSeries::new(t("Team", "团队"), vec![3.2, 3.1, 3.0, 3.0, 2.7, 2.6]),
                ],
                t(
                    "Free, Pro and Team all improved. Pro fell from 4.7% to 3.4%.",
                    "免费、Pro 和团队套餐的流失率均下降。Pro 从 4.7% 降到 3.4%。",
                ),
            ),
        };
        let count = months.len() * series.len();
        AskDataResult::new(title, months.into_iter().map(Into::into).collect(), series)
            .with_meta(count, 214)
            .with_insight(insight)
    }
}
struct AskDataExample {
    control: Entity<AskYourData>,
    sample: AskSample,
    fail_next: bool,
    _subscription: Subscription,
}
impl AskDataExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let control = cx.new(|cx| AskYourData::new("analytics", w, cx).with_dialect("PostgreSQL"));
        let input = control.read(cx).question();
        input.update(cx, |field, cx| {
            field.set_value(AskSample::questions(l)[0], w, cx)
        });
        let subscription = cx.subscribe_in(
            &control, w,
            |s: &mut Self, entity, event: &AskDataEvent, _, cx| {
                let language = UiLanguage::current(cx);
                match event {
                    AskDataEvent::GenerateSql { id, question } => {
                        let Some(sample) = AskSample::from_question(question) else {
                            entity.update(cx, |view, cx| {
                                view.fail(*id, language.text("Try one of the suggested questions to explore the sample data.", "可以选一个建议问题来查看样例数据。"), cx);
                            });
                            return;
                        };
                        s.sample = sample;
                        let id = *id;
                        let entity = entity.clone();
                        cx.spawn(async move |_, cx| {
                            let sql = sample.sql();
                            let mut cursor = 0;
                            let mut ends = sql.match_indices(char::is_whitespace)
                                .filter(|(i, _)| *i > 0 && !sql.as_bytes()[i - 1].is_ascii_whitespace())
                                .map(|(i, separator)| i + separator.len()).skip(3).step_by(4)
                                .chain(std::iter::once(sql.len()));
                            for end in &mut ends {
                                let accepted = entity.update(cx, |view, cx| view.append_sql(id, &sql[cursor..end], cx));
                                if !accepted { return; }
                                cursor = end;
                                cx.background_executor().timer(std::time::Duration::from_millis(34)).await;
                            }
                            entity.update(cx, |view, cx| view.finish_sql(id, cx));
                        }).detach();
                    }
                    AskDataEvent::RunQuery { id, .. } => {
                        let id = *id;
                        let fail = std::mem::take(&mut s.fail_next);
                        let sample = s.sample;
                        let entity = entity.clone();
                        cx.spawn(async move |_, cx| {
                            let result = sample.result(language);
                            for step in 1..=4 {
                                let accepted = entity.update(cx, |view, cx| view.progress(id, result.row_count * step / 4, Some(result.row_count), cx));
                                if !accepted { return; }
                                cx.background_executor().timer(std::time::Duration::from_millis(170)).await;
                            }
                            entity.update(cx, |view, cx| {
                                if fail {
                                    view.fail(id, language.text("Sample query failed. Try again.", "样例查询失败，请重试。"), cx)
                                } else {
                                    let finished = view.finish_query(id, result, cx);
                                    if finished {
                                        view.set_stacked(!matches!(sample, AskSample::Churn), cx);
                                    }
                                    finished
                                }
                            });
                        }).detach();
                    }
                    AskDataEvent::Cancelled(_) => {}
                }
            },
        );
        control.update(cx, |control, cx| {
            control.run(w, cx);
        });
        Self {
            control,
            sample: AskSample::default(),
            fail_next: false,
            _subscription: subscription,
        }
    }
}
impl Render for AskDataExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let show_suggestions = matches!(
            self.control.read(cx).status(),
            AskDataStatus::Idle | AskDataStatus::Failed | AskDataStatus::Stopped
        );
        let radius = f32::from(kui::foundation::surface_radius(cx));
        let failure = if self.fail_next {
            l.text("Next sample query will fail", "下次样例查询将失败")
        } else {
            l.text("Simulate a failed query", "模拟一次查询失败")
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.))
            .child(self.control.clone())
            .when(show_suggestions, |root| {
                root.child(
                    div()
                        .w_full()
                        .max_w(px(522.))
                        .flex()
                        .flex_wrap()
                        .gap(px(6.))
                        .children(AskSample::questions(l).into_iter().enumerate().map(
                            |(i, question)| {
                                face(
                                    ("ask-example-suggestion", i),
                                    question,
                                    32.,
                                    radius,
                                    Tone::Surface,
                                    cx,
                                )
                                .px(px(9.))
                                .text_size(px(12.))
                                .child(div().min_w_0().truncate().child(question))
                                .on_click(cx.listener(
                                    move |s, _, w, cx| {
                                        let field = s.control.read(cx).question();
                                        field.update(cx, |field, cx| {
                                            field.set_value(question, w, cx)
                                        });
                                        s.control.update(cx, |view, cx| {
                                            view.run(w, cx);
                                        });
                                    },
                                ))
                            },
                        )),
                )
            })
            .child(div().text_size(px(12.)).child(l.text(
                "Example only: fixed SQL and sample data; no database is queried.",
                "仅为示例：使用固定查询和样例数据，不会连接数据库。",
            )))
            .child(
                div()
                    .id("ask-example-fail")
                    .role(Role::Button)
                    .tab_index(0)
                    .cursor_pointer()
                    .text_size(px(12.))
                    .child(failure)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.fail_next = !s.fail_next;
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.fail_next = !s.fail_next;
                            cx.notify();
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    })),
            )
    }
}

struct PiiExample {
    control: Entity<PiiRedaction>,
    fail_next: bool,
    _subscription: Subscription,
}
impl PiiExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let control = cx.new(|cx| PiiRedaction::new(w, cx).with_model("Claude Sonnet 5.5"));
        let source = control.read(cx).source();
        source.update(cx, |field, cx| {
            field.set_value(lang.text(
                "Hi, this is Sofia Ramirez. I was charged twice for my order on card 4242 4242 4242 4242. Please contact me at sofia@example.com or +1 415 555 0100.",
                "你好，我是 Sofia Ramirez。我的银行卡 4242 4242 4242 4242 被重复扣款。请联系 sofia@example.com 或 +1 415 555 0100。"
            ), w, cx)
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &PiiEvent, _, cx| match event {
                PiiEvent::ScanRequested { id, text } => {
                    let id = *id;
                    let text = text.to_string();
                    let entity = entity.clone();
                    let fail = s.fail_next;
                    s.fail_next = false;
                    let lang = UiLanguage::current(cx);
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(600))
                            .await;
                        let mut spans = Vec::new();
                        for (needle, kind) in [
                            ("Sofia Ramirez", PiiKind::Name),
                            ("sofia@example.com", PiiKind::Email),
                            ("+1 415 555 0100", PiiKind::Phone),
                            ("4242 4242 4242 4242", PiiKind::Card),
                        ] {
                            for (start, _) in text.match_indices(needle) {
                                spans.push(PiiSpan::new(start, start + needle.len(), kind));
                            }
                        }
                        entity.update(cx, |control, cx| {
                            if fail {
                                control.fail_scan(
                                    id,
                                    lang.text(
                                        "Sample scan failed. Try again.",
                                        "样例识别失败，请重试。",
                                    ),
                                    cx,
                                );
                            } else {
                                control.finish_scan(id, spans, cx);
                            }
                        });
                    })
                    .detach();
                }
                PiiEvent::Cancelled(_) => {}
            },
        );
        control.update(cx, |control, cx| {
            control.scan(cx);
        });
        Self {
            control,
            fail_next: false,
            _subscription: subscription,
        }
    }
}
impl Render for PiiExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let failure = if self.fail_next {
            l.text("Next sample scan will fail", "下次样例识别将失败")
        } else {
            l.text("Simulate a failed scan", "模拟一次识别失败")
        };
        div().w_full().flex().flex_col().gap(px(10.))
            .child(self.control.clone())
            .child(div().text_size(px(12.)).child(l.text(
                "Simulated demo: matches only the four fixed example values. No text is transmitted. Do not use this demo as a real detector.",
                "模拟演示：只匹配四个固定样例，不会传输文本；不能作为真实识别工具。",
            )))
            .child(div().id("pii-example-fail").role(Role::Button).tab_index(0).cursor_pointer().text_size(px(12.))
                .child(failure)
                .on_click(cx.listener(|s, _, _, cx| { s.fail_next = !s.fail_next; cx.notify(); }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        s.fail_next = !s.fail_next; cx.notify(); w.prevent_default(); cx.stop_propagation();
                    }
                })))
    }
}

struct ReplyExample {
    control: Entity<ReplyDraft>,
    fail_next: bool,
    note: SharedString,
    _subscription: Subscription,
}
impl ReplyExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let message = ReplyMessage::new(
            "Diana Foster",
            l.text("Renewal call next week", "下周的续约会议"),
            l.text(
                "Hi Ryan,\n\nThanks for the update. Before we sign the renewal, could we talk through the new seats and single sign-on? Does Thursday afternoon work for you?\n\nBest,\nDiana",
                "你好 Ryan，\n\n谢谢你的更新。签订续约之前，我们能否讨论新增席位和单点登录？周四下午方便吗？\n\nDiana",
            ),
        );
        let hints = vec![
            ReplyHint::new("yes", l.text("Thursday works", "周四可以")),
            ReplyHint::new("later", l.text("Suggest Friday", "建议周五")),
            ReplyHint::new("details", l.text("Ask about SSO", "询问单点登录")),
        ];
        let control = cx.new(|cx| {
            ReplyDraft::new(message, hints, w, cx)
                .with_sender_image(crate::family_story::collections::portrait_image(0))
                .with_model("Claude Sonnet 5.5")
        });
        let subscription = cx.subscribe_in(&control, w, |s: &mut Self, entity, event: &ReplyDraftEvent, w, cx| {
            match event {
                ReplyDraftEvent::Requested(request) => {
                    let request = request.clone(); let entity = entity.clone(); let fail = s.fail_next; s.fail_next = false;
                    let language = UiLanguage::current(cx);
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor().timer(std::time::Duration::from_millis(550)).await;
                        if fail {
                            let _ = cx.update(|_, cx| entity.update(cx, |draft, cx| { draft.fail(request.id, language.text("Sample draft failed. Try again.", "样例草稿生成失败，请重试。"), cx); }));
                            return;
                        }
                        let kind = request.hint.as_ref().map(|hint| hint.id.as_ref()).unwrap_or("yes");
                        let sentence = match (language, kind) {
                            (UiLanguage::Chinese, "later") => "周四我有其他安排，周五方便吗？",
                            (UiLanguage::Chinese, "details") => "能否先告诉我你们的单点登录服务商？",
                            (UiLanguage::Chinese, _) => "周四下午合适。我会提前准备新增席位和单点登录的资料。",
                            (_, "later") => "I have a conflict Thursday. Would Friday work for you?",
                            (_, "details") => "Could you share which identity provider you use for single sign-on?",
                            _ => "Thursday afternoon works. I can review the new seats and single sign-on options before we meet.",
                        };
                        let greeting = match (language, request.tone) {
                            (UiLanguage::Chinese, ReplyTone::Formal) => "您好，\n\n",
                            (UiLanguage::Chinese, ReplyTone::Warm) => "你好，很高兴收到你的消息。\n\n",
                            (UiLanguage::Chinese, _) => "你好，\n\n",
                            (_, ReplyTone::Formal) => "Dear Diana,\n\n",
                            (_, ReplyTone::Warm) => "Hi Diana, great to hear from you.\n\n",
                            _ => "Hi Diana,\n\n",
                        };
                        let extra = match (language, request.length) {
                            (_, ReplyLength::Short) => "",
                            (UiLanguage::Chinese, ReplyLength::Long) => "如果还有需要提前讨论的内容，欢迎随时告诉我。",
                            (UiLanguage::Chinese, _) => "期待后续沟通。",
                            (_, ReplyLength::Long) => " Please let me know if there is anything else we should discuss beforehand.",
                            _ => " Looking forward to it.",
                        };
                        let closing = if language == UiLanguage::Chinese { "\n\n谢谢！" } else { "\n\nThank you!" };
                        let chunks = vec![greeting, sentence, extra, closing];
                        for chunk in chunks {
                            let accepted = cx.update(|_, cx| entity.update(cx, |draft, cx| draft.append(request.id, chunk, cx))).unwrap_or(false);
                            if !accepted { return; }
                            cx.background_executor().timer(std::time::Duration::from_millis(180)).await;
                        }
                        let _ = cx.update(|w, cx| entity.update(cx, |draft, cx| { draft.finish(request.id, w, cx); }));
                    }).detach();
                }
                ReplyDraftEvent::Cancelled(_) => {}
                ReplyDraftEvent::Insert(_) => { s.note = UiLanguage::current(cx).text("Sample inserted; no message sent.", "已触发样例插入，未发送消息。").into(); cx.notify(); }
            }
        });
        control.update(cx, |control, cx| {
            control.generate(cx);
        });
        Self {
            control,
            fail_next: false,
            note: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for ReplyExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let failure = if self.fail_next {
            l.text("Next request will fail", "下次请求将失败")
        } else {
            l.text("Simulate a failed request", "模拟一次请求失败")
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(self.control.clone())
            .child(div().text_size(px(12.)).child(l.text(
                "Sample text is generated locally. No message is sent.",
                "样例文本在本地生成，不会发送消息。",
            )))
            .child(
                div()
                    .id("reply-example-fail")
                    .role(Role::Button)
                    .tab_index(0)
                    .cursor_pointer()
                    .text_size(px(12.))
                    .child(failure)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.fail_next = !s.fail_next;
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.fail_next = !s.fail_next;
                            cx.notify();
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    })),
            )
            .child(div().text_size(px(12.)).child(self.note.clone()))
    }
}

struct TicketExample {
    control: Entity<TicketRouter>,
    fail_classification: bool,
    fail_route: bool,
    next: usize,
    note: SharedString,
    _subscription: Subscription,
}
impl TicketExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let lanes = vec![
            TicketLane::new("billing", l.text("Billing", "账务")),
            TicketLane::new("support", l.text("Engineering", "工程")),
            TicketLane::new("product", l.text("Success", "客户成功")),
        ];
        let tickets = vec![
            Ticket::new(
                "invoice",
                l.text("Charged twice for the October invoice", "十月账单被重复扣款"),
                l.text("Hi team, our ending 4417 was billed twice for the growth plan. Could you check the duplicate charge?", "你好，我们尾号 4417 的卡被重复扣费，能帮忙检查这笔账单吗？"),
            ).with_sender("Priya Raman").with_channel("Gmail"),
            Ticket::new(
                "login",
                l.text("Cannot sign in", "无法登录"),
                l.text("Our SAML login loops back to the sign-in screen after authentication.", "验证后会跳回登录界面。"),
            ).with_sender("Daniel Kim").with_channel("Gmail"),
            Ticket::new(
                "feature",
                l.text("Feature request", "功能建议"),
                l.text("Can we export all changes as a CSV for the team report?", "可以导出 CSV 供团队报告使用吗？"),
            ).with_sender("Avery Chen").with_channel("Gmail"),
        ];
        let control = cx.new(|_| {
            TicketRouter::new(l.text("Ticket routing", "工单分流"), lanes, tickets)
                .with_concurrency(2)
                .with_model("Claude Haiku 5.5")
        });
        let weak = control.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;
                if weak
                    .update(cx, |router, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        router.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &TicketRouterEvent, _, cx| {
                let l = UiLanguage::current(cx);
                match event {
                    TicketRouterEvent::ClassifyRequested { request, ticket } => {
                        let request = *request;
                        let ticket = ticket.clone();
                        let entity = entity.clone();
                        let fail = ticket.id.as_ref() == "login" && s.fail_classification;
                        if fail {
                            s.fail_classification = false;
                        }
                        cx.spawn(async move |_, cx| {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(550))
                                .await;
                            entity.update(cx, |router, cx| {
                                if fail {
                                    router.failed(
                                        request,
                                        l.text("Example classification failed", "样例分类失败"),
                                        cx,
                                    );
                                } else {
                                    let lane = match ticket.id.as_ref() {
                                        "invoice" => "billing",
                                        "feature" => "product",
                                        _ => "support",
                                    };
                                    let confidence = if ticket.id.as_ref() == "feature" {
                                        0.63
                                    } else {
                                        0.92
                                    };
                                    router.classified(
                                        request,
                                        TicketDecision::new(lane, confidence)
                                            .with_intent(l.text("Question", "咨询"))
                                            .with_sentiment(l.text("Neutral", "中性"))
                                            .with_assignee("Alex"),
                                        cx,
                                    );
                                }
                            });
                        })
                        .detach();
                    }
                    TicketRouterEvent::RouteRequested {
                        request, ticket, ..
                    } => {
                        let request = *request;
                        let entity = entity.clone();
                        let fail = ticket.id.as_ref() == "invoice" && s.fail_route;
                        if fail {
                            s.fail_route = false;
                        }
                        cx.spawn(async move |_, cx| {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(350))
                                .await;
                            entity.update(cx, |router, cx| {
                                let result = if fail {
                                    Err(l.text("Example assignment failed", "样例分配失败").into())
                                } else {
                                    Ok(())
                                };
                                router.routed(request, result, cx);
                            });
                        })
                        .detach();
                    }
                    TicketRouterEvent::Cancelled(_) => {
                        s.note = l
                            .text("Pending sample request cancelled", "已取消样例请求")
                            .into();
                        cx.notify();
                    }
                }
            },
        );
        Self {
            control,
            fail_classification: true,
            fail_route: true,
            next: 1,
            note: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for TicketExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        div().w_full().flex().flex_col().gap(px(10.)).child(self.control.clone())
            .child(div().text_size(px(12.)).child(l.text("Sample only: no ticket is sent or assigned. Two requests fail once to demonstrate retry.", "仅为样例：不会发送或分配工单。两项请求首次失败以演示重试。")))
            .child(div().id("ticket-add-example").role(Role::Button).tab_index(0).cursor_pointer().text_size(px(12.))
                .child(l.text("Add sample ticket", "添加样例工单"))
                .on_click(cx.listener(|s, _, _, cx| {
                    let number = s.next; s.next += 1;
                    let l = UiLanguage::current(cx);
                    let ticket = Ticket::new(format!("sample-{number}"), format!("{} {number}", l.text("Sample ticket", "样例工单")), l.text("A sample request", "一条样例请求"));
                    s.control.update(cx, |router, cx| router.set_ticket(ticket, cx));
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        let number = s.next; s.next += 1; let l = UiLanguage::current(cx);
                        let ticket = Ticket::new(format!("sample-{number}"), format!("{} {number}", l.text("Sample ticket", "样例工单")), l.text("A sample request", "一条样例请求"));
                        s.control.update(cx, |router, cx| router.set_ticket(ticket, cx)); w.prevent_default(); cx.stop_propagation();
                    }
                })))
            .child(div().text_size(px(12.)).child(self.note.clone()))
    }
}

struct EnrichExample {
    control: Entity<EnrichColumn>,
    fail_once: bool,
    note: SharedString,
    _subscription: Subscription,
}
impl EnrichExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let rows = vec![
            EnrichRow::new("linear", "Linear"),
            EnrichRow::new("vercel", "Vercel"),
            EnrichRow::new("notion", "Notion"),
            EnrichRow::new("figma", "Figma"),
            EnrichRow::new("stripe", "Stripe"),
            EnrichRow::new("supabase", "Supabase"),
            EnrichRow::new("retool", "Retool"),
        ];
        let control = cx.new(|cx| {
            EnrichColumn::new(
                language.text("Company", "公司"),
                rows,
                language.text(
                    "Find each company's team size and latest funding round",
                    "查找每家公司的团队规模与最新一轮融资",
                ),
                w,
                cx,
            )
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &EnrichEvent, w, cx| match event {
                EnrichEvent::Requested { id, row, .. } => {
                    let id = *id;
                    let row = row.clone();
                    let entity = entity.clone();
                    let fail = row.id.as_ref() == "vercel" && s.fail_once;
                    if fail {
                        s.fail_once = false;
                    }
                    let l = UiLanguage::current(cx);
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(500))
                            .await;
                        let _ = cx.update(|w, cx| {
                            entity.update(cx, |column, cx| {
                                if fail {
                                    column.fail(
                                        id,
                                        l.text(
                                            "Sample lookup failed; retry.",
                                            "样例查询失败，请重试。",
                                        ),
                                        cx,
                                    );
                                } else {
                                    let value = match row.id.as_ref() {
                                        "linear" => {
                                            l.text("180 people · Series C", "180 人 · C 轮")
                                        }
                                        "vercel" => {
                                            l.text("820 people · Series F", "820 人 · F 轮")
                                        }
                                        "notion" => {
                                            l.text("1,100 people · Series C", "1,100 人 · C 轮")
                                        }
                                        "figma" => l.text(
                                            "1,400 people · Public company",
                                            "1,400 人 · 上市公司",
                                        ),
                                        "stripe" => {
                                            l.text("8,000 people · Series I", "8,000 人 · I 轮")
                                        }
                                        "supabase" => {
                                            l.text("180 people · Series D", "180 人 · D 轮")
                                        }
                                        _ => l.text("360 people · Series C", "360 人 · C 轮"),
                                    };
                                    column.finish(
                                        id,
                                        EnrichResult::new(value).with_source(EnrichSource::new(
                                            l.text("Sample source", "样例来源"),
                                            "https://example.com/",
                                        )),
                                        w,
                                        cx,
                                    );
                                }
                            })
                        });
                    })
                    .detach();
                }
                EnrichEvent::OpenSource { .. } => {
                    s.note = UiLanguage::current(cx)
                        .text(
                            "Sample source selected; no page was opened.",
                            "已选择样例来源，未打开网页。",
                        )
                        .into();
                    cx.notify();
                }
                EnrichEvent::Changed { .. } | EnrichEvent::Cancelled(_) => {}
            },
        );
        Self {
            control,
            fail_once: true,
            note: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for EnrichExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(self.control.clone())
            .child(div().text_size(px(12.)).child(l.text(
                "Illustrative results. The first Vercel lookup fails so you can try again.",
                "结果为演示数据。Vercel 首次查询模拟失败，可测试重试。",
            )))
            .child(div().text_size(px(12.)).child(self.note.clone()))
    }
}

struct TldrExample {
    control: Entity<TldrToggle>,
    fail_next: bool,
    _subscription: Subscription,
}
impl TldrExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let article = TldrArticle::new(l.text("Making workspace sync faster", "让工作区同步更快"), vec![
            TldrParagraph::new("context", l.text("Large workspaces are busiest at the start of the day, when many clients reconnect and request recent changes.", "工作日开始时，许多客户端重新连接并请求近期改动，大型工作区最为繁忙。")),
            TldrParagraph::new("small", l.text("Loading the most important changes first keeps the interface responsive while older items arrive in the background.", "优先加载关键改动，旧内容在后台到达，界面可保持响应。")),
            TldrParagraph::new("review", l.text("We measured first paint and time to catch up independently, so each change could be evaluated.", "我们分别测量首帧时间和完成同步的时间，便于评估每项改动。")),
        ]).with_source("Linear");
        let control = cx.new(|cx| TldrToggle::new(article, cx).with_model("Claude Haiku 5.5"));
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &TldrEvent, _, cx| {
                if let TldrEvent::Requested(request) = event {
                    let request = request.clone();
                    let entity = entity.clone();
                    let fail = s.fail_next;
                    s.fail_next = false;
                    let language = UiLanguage::current(cx);
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(700))
                            .await;
                        if fail {
                            entity.update(cx, |control, cx| {
                                control.fail(
                                    request.id,
                                    language.text(
                                        "Example request failed. Try again.",
                                        "示例请求失败，请重试。",
                                    ),
                                    cx,
                                );
                            });
                            return;
                        }
                        if request.mode == TldrMode::Summary {
                            let chunks = if language == UiLanguage::Chinese {
                                vec![
                                    "大型工作区在重新连接时优先加载关键改动，",
                                    "旧内容在后台加载，",
                                    "首帧与完整同步时间分开测量。",
                                ]
                            } else {
                                vec![
                                    "Load the most important workspace changes first, ",
                                    "let older content catch up in the background, ",
                                    "and measure first paint separately from total sync time.",
                                ]
                            };
                            for chunk in chunks {
                                let accepted = entity.update(cx, |control, cx| {
                                    control.append_text(request.id, chunk, cx)
                                });
                                if !accepted {
                                    return;
                                }
                                cx.background_executor()
                                    .timer(std::time::Duration::from_millis(180))
                                    .await;
                            }
                        } else {
                            let points = if language == UiLanguage::Chinese {
                                vec![
                                    ("优先加载关键改动。", "context"),
                                    ("旧内容在后台同步。", "small"),
                                    ("分开测量首帧与完整同步。", "review"),
                                ]
                            } else {
                                vec![
                                    ("Load the most important changes first.", "context"),
                                    ("Sync older content in the background.", "small"),
                                    ("Measure first paint and full sync separately.", "review"),
                                ]
                            };
                            for (text, source) in points.into_iter().take(request.point_count) {
                                let accepted = entity.update(cx, |control, cx| {
                                    control.append_point(
                                        request.id,
                                        TldrPoint::new(text, source),
                                        cx,
                                    )
                                });
                                if !accepted {
                                    return;
                                }
                                cx.background_executor()
                                    .timer(std::time::Duration::from_millis(180))
                                    .await;
                            }
                        }
                        entity.update(cx, |control, cx| {
                            control.finish(request.id, cx);
                        });
                    })
                    .detach();
                }
            },
        );
        control.update(cx, |control, cx| {
            control.select_mode(TldrMode::Summary, cx);
        });
        Self {
            control,
            fail_next: false,
            _subscription: subscription,
        }
    }
}
impl Render for TldrExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let name = if self.fail_next {
            l.text("Next request will fail", "下次请求将失败")
        } else {
            l.text("Simulate a failed request", "模拟一次请求失败")
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(self.control.clone())
            .child(div().text_size(px(12.)).child(l.text(
                "Sample summary: no content is sent.",
                "样例摘要：不会发送文章内容。",
            )))
            .child(
                div()
                    .id("tldr-fail-next")
                    .role(Role::Button)
                    .tab_index(0)
                    .cursor_pointer()
                    .text_size(px(12.))
                    .child(name)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.fail_next = !s.fail_next;
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.fail_next = !s.fail_next;
                            cx.notify();
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                    })),
            )
    }
}
struct PresenceExample {
    control: Entity<PresenceStack>,
    people: Vec<PresencePerson>,
    next: usize,
    note: SharedString,
    _subscription: Subscription,
}
impl PresenceExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let people = vec![
            PresencePerson::new("me", "Alex Chen")
                .with_image(crate::family_story::collections::portrait_image(2)),
            PresencePerson::new("em", "Emma Lin")
                .with_image(crate::family_story::collections::portrait_image(0))
                .with_status(PresenceStatus::Typing),
            PresencePerson::new("sam", "Sam Park")
                .with_image(crate::family_story::collections::portrait_image(1))
                .with_status(PresenceStatus::Idle),
            PresencePerson::new("lee", "Lee Kim")
                .with_image(crate::family_story::collections::portrait_image(3)),
            PresencePerson::new("mia", "Mia Evans"),
            PresencePerson::new("kai", "Kai Liu"),
        ];
        let control = cx.new(|cx| {
            PresenceStack::new("presence-example", people.clone(), cx)
                .with_self_id("me", cx)
                .with_align(PresenceAlign::End)
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, _, event: &PresenceSelected, _, cx| {
                s.note = format!(
                    "{}: {}",
                    UiLanguage::current(cx).text("Selected", "已选择"),
                    event.0
                )
                .into();
                cx.notify();
            },
        );
        Self {
            control,
            people,
            next: 1,
            note: "".into(),
            _subscription: subscription,
        }
    }
    fn sync(&mut self, cx: &mut Context<Self>) {
        let people = self.people.clone();
        self.control
            .update(cx, |control, cx| control.set_people(people, cx));
        cx.notify();
    }
}
impl Render for PresenceExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let p = kui::primitives::family::Paint::get(cx);
        let button = |id: &'static str, text: &'static str| {
            div()
                .id(id)
                .debug_selector(move || text.to_string())
                .px(px(10.))
                .h(px(32.))
                .rounded(px(8.))
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(p.ink)
                .cursor_pointer()
                .child(text)
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .w_full()
                    .min_h(px(104.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .p(px(16.))
                    .pb(px(36.))
                    .rounded(kui::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(l.text("Project plan", "项目计划")),
                            )
                            .child(div().text_size(px(13.)).text_color(p.secondary).child(
                                match l {
                                    UiLanguage::Chinese => format!(
                                        "你和另外 {} 人",
                                        self.people.len().saturating_sub(1)
                                    ),
                                    _ => format!(
                                        "You and {} others",
                                        self.people.len().saturating_sub(1)
                                    ),
                                },
                            )),
                    )
                    .child(self.control.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .child(
                        button("presence-add", l.text("Someone joins", "有人加入")).on_click(
                            cx.listener(|s, _, _, cx| {
                                let id = format!("guest-{}", s.next);
                                let name = format!(
                                    "{} {}",
                                    UiLanguage::current(cx).text("Guest", "访客"),
                                    s.next
                                );
                                s.next += 1;
                                s.people.push(PresencePerson::new(id, name));
                                s.sync(cx);
                            }),
                        ),
                    )
                    .child(
                        button("presence-remove", l.text("Someone leaves", "有人离开")).on_click(
                            cx.listener(|s, _, _, cx| {
                                if s.people.len() > 1 {
                                    s.people.pop();
                                    s.sync(cx);
                                }
                            }),
                        ),
                    )
                    .child(
                        button("presence-status", l.text("Change status", "切换状态")).on_click(
                            cx.listener(|s, _, _, cx| {
                                if let Some(person) = s.people.get_mut(1) {
                                    person.status = match person.status {
                                        PresenceStatus::Active => PresenceStatus::Typing,
                                        PresenceStatus::Typing => PresenceStatus::Idle,
                                        PresenceStatus::Idle => PresenceStatus::Active,
                                    };
                                    s.sync(cx);
                                }
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.note.clone()),
            )
    }
}
struct GlassExample {
    control: Entity<GlassSegmented>,
    images: [ImageSource; 4],
    current: usize,
    previous: usize,
    transition: kui::primitives::family::PhaseClock,
    _subscription: Subscription,
}
impl GlassExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let control = cx.new(|cx| {
            GlassSegmented::new(
                language.text("Photo", "照片"),
                vec![
                    GlassSegmentOption::new("alfama", language.text("Alfama", "老城")),
                    GlassSegmentOption::new("tram", language.text("Tram", "电车")),
                    GlassSegmentOption::new("bridge", language.text("Bridge", "大桥")),
                    GlassSegmentOption::new("coast", language.text("Coast", "海岸")),
                ],
                cx,
            )
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, _, event: &GlassSegmentChanged, _, cx| {
                let next = match event.0.as_ref() {
                    "alfama" => 0,
                    "tram" => 1,
                    "bridge" => 2,
                    "coast" => 3,
                    _ => return,
                };
                if next != s.current {
                    s.previous = s.current;
                    s.current = next;
                    s.transition.restart(cx);
                    cx.notify();
                }
            },
        );
        let image = |bytes: &'static [u8]| -> ImageSource {
            std::sync::Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())).into()
        };
        Self {
            control,
            images: [
                image(include_bytes!("../../assets/places/alfama.jpg")),
                image(include_bytes!("../../assets/places/tram.jpg")),
                image(include_bytes!("../../assets/places/bridge.jpg")),
                image(include_bytes!("../../assets/places/coast.jpg")),
            ],
            current: 0,
            previous: 0,
            transition: Default::default(),
            _subscription: subscription,
        }
    }
}
struct GlassMenuExample {
    control: Entity<GlassMenu>,
    image: ImageSource,
    note: SharedString,
    _subscription: Subscription,
}
impl GlassMenuExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en: &'static str, zh: &'static str| language.text(en, zh);
        let groups = vec![
            GlassMenuGroup::new(vec![
                GlassMenuItem::new("copy", t("Copy", "复制")),
                GlassMenuItem::new("move", t("Move to", "移动到")).with_children(vec![
                    GlassMenuItem::new("projects", t("Projects", "项目")),
                    GlassMenuItem::new("archive", t("Archive", "归档")),
                ]),
                GlassMenuItem::new("favorite", t("Favorite", "收藏"))
                    .with_checked(false)
                    .with_keep_open(true),
            ]),
            GlassMenuGroup::new(vec![
                GlassMenuItem::new("delete", t("Delete", "删除")).with_danger(true),
            ]),
        ];
        let control =
            cx.new(|cx| GlassMenu::new("glass-actions", t("Pool house", "泳池住宅"), groups, cx));
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, control, event: &GlassMenuSelected, _, cx| {
                if let Some(checked) = event.checked {
                    control.update(cx, |menu, cx| {
                        menu.set_checked(event.id.as_ref(), checked, cx)
                    });
                }
                s.note = format!(
                    "{}: {}",
                    UiLanguage::current(cx).text("Selected", "已选择"),
                    event.id
                )
                .into();
                cx.notify();
            },
        );
        Self {
            control,
            image: std::sync::Arc::new(Image::from_bytes(
                ImageFormat::Jpeg,
                include_bytes!("../../assets/places/pool-house.jpg").to_vec(),
            ))
            .into(),
            note: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for GlassMenuExample {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let radius = kui::foundation::surface_radius(cx);
        div()
            .relative()
            .w_full()
            .h(px(if w.viewport_size().width < px(460.) {
                388.
            } else {
                556.
            }))
            .rounded(radius)
            .p(px(16.))
            .child(
                img(self.image.clone())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .rounded(radius)
                    .object_fit(ObjectFit::Cover),
            )
            .flex()
            .flex_col()
            .justify_between()
            .items_start()
            .child(self.control.clone())
            .child(
                div()
                    .text_color(white())
                    .text_size(px(13.))
                    .child(self.note.clone()),
            )
    }
}
impl Render for GlassExample {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let alpha = kui::primitives::family::ease(self.transition.time(0.25, w, cx) / 0.25);
        let language = UiLanguage::current(cx);
        let title = [
            language.text("Alfama at sunset", "日落下的老城"),
            language.text("Tram through Graça", "驶过山城的电车"),
            language.text("Across the Tagus", "跨越塔霍河"),
            language.text("Atlantic coast", "大西洋海岸"),
        ][self.current];
        let radius = kui::foundation::surface_radius(cx);
        div()
            .relative()
            .w_full()
            .h(px(if w.viewport_size().width < px(460.) {
                370.
            } else {
                514.
            }))
            .rounded(radius)
            .overflow_hidden()
            .child(
                img(self.images[self.previous].clone())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .rounded(radius)
                    .object_fit(ObjectFit::Cover),
            )
            .child(
                img(self.images[self.current].clone())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .rounded(radius)
                    .object_fit(ObjectFit::Cover)
                    .opacity(alpha),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .rounded(radius)
                            .bg(black().opacity(0.34))
                            .px(px(10.))
                            .py(px(7.))
                            .self_start()
                            .text_color(white())
                            .text_size(px(14.))
                            .child(title),
                    )
                    .child(self.control.clone()),
            )
    }
}

struct MegaExample {
    menu: Entity<MegaMenuMorph>,
    note: SharedString,
    _subscription: Subscription,
}
impl MegaExample {
    fn new(sections: Vec<MegaMenuSection>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let menu = cx.new(|cx| {
            let brand = |bytes: &[u8]| {
                std::sync::Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())).into()
            };
            MegaMenuMorph::new("Kense", sections, cx)
                .with_brand_images(
                    brand(include_bytes!(
                        "../../../../website/public/assets/logo-light.png"
                    )),
                    brand(include_bytes!(
                        "../../../../website/public/assets/logo-dark.png"
                    )),
                )
                .with_compact_below(px(640.))
                .with_primary("start", lang.text("Start now", "开始使用"))
        });
        let subscription = cx.subscribe_in(
            &menu,
            w,
            |s: &mut Self, _, event: &MegaMenuNavigate, _, cx| {
                let lang = UiLanguage::current(cx);
                s.note = format!(
                    "{}: {}",
                    lang.text("Selected destination", "已选择目的地"),
                    event.0
                )
                .into();
                cx.notify();
            },
        );
        Self {
            menu,
            note: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for MegaExample {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lang = UiLanguage::current(cx);
        let note = if self.note.is_empty() {
            lang.text(
                "Choose a destination from the navigation",
                "从导航中选择一个目的地",
            )
            .into()
        } else {
            self.note.clone()
        };
        let p = Paint::get(cx);
        let narrow = w.viewport_size().width < px(400.);
        div()
            .relative()
            .w_full()
            .min_h(px(if narrow { 472. } else { 520. }))
            .rounded(kui::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .child(
                div()
                    .absolute()
                    .top(px(108.))
                    .left(px(if narrow { 20. } else { 28. }))
                    .right(px(if narrow { 20. } else { 28. }))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .max_w(px(380.))
                            .text_size(px(if narrow { 26. } else { 31. }))
                            .line_height(px(if narrow { 31. } else { 36. }))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(
                                lang.text(
                                    "Build native interfaces with KUI",
                                    "用 KUI 构建原生界面",
                                ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(p.secondary)
                            .child(lang.text(
                                "One toolkit for responsive desktop and web applications.",
                                "一套工具，构建适应不同屏幕的桌面与网页应用。",
                            )),
                    )
                    .child(
                        div()
                            .id("mega-demo-destination")
                            .role(Role::Status)
                            .text_size(px(13.))
                            .text_color(p.faint)
                            .child(note),
                    ),
            )
            .child(self.menu.clone())
    }
}
struct TreeExample {
    control: Entity<TreeTable>,
    fail_once: bool,
    _subscription: Subscription,
}
impl TreeExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let rows = vec![
            TreeTableRow::new("platform", language.text("Platform", "平台"))
                .with_cells(vec!["48".into(), "$4,450".into()])
                .with_children(vec![
                    TreeTableRow::new("api", "API").with_cells(vec!["16".into(), "$2,150".into()]),
                    TreeTableRow::new("auth", language.text("Authentication", "身份验证"))
                        .with_cells(vec!["8".into(), "$1,180".into()]),
                    TreeTableRow::new("database", language.text("Database", "数据库"))
                        .with_cells(vec!["6".into(), "$420".into()]),
                    TreeTableRow::new("storage", language.text("Storage", "存储"))
                        .with_cells(vec!["4".into(), "$210".into()]),
                    TreeTableRow::new("metrics", language.text("Metrics", "指标"))
                        .with_cells(vec!["3".into(), "$180".into()]),
                    TreeTableRow::new("preview", language.text("Preview", "预览"))
                        .with_cells(vec!["2".into(), "$120".into()]),
                    TreeTableRow::new("search", language.text("Search", "搜索"))
                        .with_cells(vec!["2".into(), "$110".into()]),
                    TreeTableRow::new("cache", language.text("Cache", "缓存"))
                        .with_cells(vec!["2".into(), "$80".into()]),
                ]),
            TreeTableRow::new("growth", language.text("Growth", "增长"))
                .with_cells(vec!["7".into(), "$600".into()])
                .with_lazy_children(),
            TreeTableRow::new("design", language.text("Design", "设计"))
                .with_cells(vec!["4".into(), "$320".into()]),
        ];
        let columns = vec![
            language.text("Name", "名称").into(),
            language.text("Resources", "资源").into(),
            language.text("Monthly", "每月").into(),
        ];
        let control = cx.new(|cx| {
            TreeTable::new(language.text("Resources", "资源"), columns, rows, cx)
                .with_expanded(&["platform"])
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &TreeTableEvent, _, cx| {
                if let TreeTableEvent::LoadRequested(id) = event {
                    let (id, fail, entity) = (id.clone(), s.fail_once, entity.clone());
                    s.fail_once = false;
                    let child_name = UiLanguage::current(cx).text("Campaign analytics", "活动分析");
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(300))
                            .await;
                        entity.update(cx, |tree, cx| {
                            let result = if fail {
                                Err(())
                            } else {
                                Ok(vec![
                                    TreeTableRow::new("campaign", child_name)
                                        .with_cells(vec!["7".into(), "$600".into()]),
                                ])
                            };
                            tree.resolve_children(&id, result, cx);
                        });
                    })
                    .detach();
                }
            },
        );
        Self {
            control,
            fail_once: true,
            _subscription: subscription,
        }
    }
}
impl Render for TreeExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(self.control.clone())
            .child(div().text_size(px(12.)).child(UiLanguage::current(cx).text(
                "The first Growth load fails; click it again to retry.",
                "首次加载增长项目会失败，再点击可重试。",
            )))
    }
}
struct UndoExample {
    control: Entity<UndoToast>,
    fail_next: bool,
    _subscription: Subscription,
}
impl UndoExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en: &'static str, zh: &'static str| language.text(en, zh);
        let items = vec![
            UndoItem::new("one", t("Design systems deep dive", "设计系统深入探讨"))
                .with_sender("Emma Collins")
                .with_description(t(
                    "Shared the slides and recording from Thursday.",
                    "已分享周四的演示文稿与录像。",
                ))
                .with_meta("2m")
                .with_unread(true),
            UndoItem::new("two", t("Q4 roadmap sign-off", "Q4 路线图确认"))
                .with_sender("Marcus Johnson")
                .with_description(t(
                    "Added the timeline for mobile offline sync.",
                    "补充了移动端离线同步的时间表。",
                ))
                .with_meta("18m")
                .with_unread(true),
            UndoItem::new(
                "three",
                t("Checkout button stalls on Safari", "Safari 结账按钮卡顿"),
            )
            .with_sender("Sofia Rossi")
            .with_description(t(
                "Reproduced on iOS 17.4, patch is in review.",
                "已在 iOS 17.4 复现，补丁正在审核。",
            ))
            .with_meta("1h"),
            UndoItem::new("four", t("Updated invoice for October", "十月更新账单"))
                .with_sender("Daniel Kim")
                .with_description(t(
                    "Attached the PDF with the new VAT number.",
                    "已附上包含新税号的 PDF。",
                ))
                .with_meta("Yesterday"),
            UndoItem::new("five", t("Agreement signed", "协议已签署"))
                .with_sender("Olivia Bennett")
                .with_description(t(
                    "Both copies are ready in the project folder.",
                    "两份文件都已放在项目文件夹。",
                ))
                .with_meta(t("Sun", "周日")),
            UndoItem::new("six", t("Access to the design workspace", "设计工作区权限"))
                .with_sender("Nathan Cole")
                .with_description(t(
                    "Your read access is ready for the next review.",
                    "下次评审需要的读取权限已开通。",
                ))
                .with_meta(t("Sat", "周六")),
        ];
        let avatars = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                (
                    item.key.clone(),
                    crate::family_story::collections::portrait_image(i).into(),
                )
            })
            .collect();
        let control = cx.new(|cx| {
            UndoToast::new(language.text("Inbox", "收件箱"), items, cx).with_avatars(avatars)
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, entity, event: &UndoToastEvent, _, cx| {
                if let UndoToastEvent::CommitRequested { batch, .. } = event {
                    let (batch, succeed, entity) = (*batch, !s.fail_next, entity.clone());
                    s.fail_next = false;
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(300))
                            .await;
                        entity.update(cx, |control, cx| control.resolve_commit(batch, succeed, cx));
                    })
                    .detach();
                }
            },
        );
        Self {
            control,
            fail_next: false,
            _subscription: subscription,
        }
    }
}
impl Render for UndoExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = UiLanguage::current(cx);
        let p = Paint::get(cx);
        let radius = kui::foundation::surface_radius(cx);
        let depleted = self.control.read(cx).is_depleted();
        let label = if self.fail_next {
            language.text("Next commit will fail", "下次提交将失败")
        } else {
            language.text("Simulate a failed commit", "模拟一次提交失败")
        };
        div()
            .w_full()
            .min_h(px(512.))
            .p(px(16.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(10.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(language.text("Inbox", "收件箱")),
                    )
                    .when(depleted, |h| {
                        let refill = language.text("Refill inbox", "重置收件箱");
                        h.child(
                            face("undo-refill", refill, 30., radius.into(), Tone::Clear, cx)
                                .px(px(8.))
                                .gap(px(5.))
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .child(icon(Glyph::Undo, 13., p.secondary))
                                .child(refill)
                                .on_click(cx.listener(|s, _, _, cx| {
                                    s.control.update(cx, |c, cx| c.refill(cx));
                                })),
                        )
                    }),
            )
            .child(self.control.clone())
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(language.text(
                        "Archive a few in a row, then undo with the button or ⌘Z",
                        "连续归档几条，然后点击撤销或按 ⌘Z",
                    ))
                    .child(
                        div()
                            .id("undo-fail-next")
                            .role(Role::Button)
                            .tab_index(0)
                            .cursor_pointer()
                            .text_color(p.secondary)
                            .child(label)
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.fail_next = !s.fail_next;
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    s.fail_next = !s.fail_next;
                                    cx.notify();
                                    cx.stop_propagation();
                                    w.prevent_default();
                                }
                            })),
                    ),
            )
    }
}
struct SignatureExample {
    pad: Entity<SignaturePad>,
    status: SharedString,
    _subscription: Subscription,
}
impl SignatureExample {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let pad = cx.new(|cx| SignaturePad::new(language.text("Sign here", "请在这里签名"), cx));
        let subscription = cx.subscribe_in(
            &pad,
            w,
            |s: &mut Self, _, event: &SignaturePngExported, _, cx| {
                let language = UiLanguage::current(cx);
                s.status = match &event.0 {
                    Ok(data) => format!(
                        "{} · {} KB",
                        language.text(
                            "PNG prepared; host can save the bytes",
                            "PNG 已生成；宿主可保存文件"
                        ),
                        data.len().div_ceil(1024)
                    )
                    .into(),
                    Err(_) => language
                        .text(
                            "PNG export failed; your signature is unchanged",
                            "PNG 导出失败，笔迹仍保留",
                        )
                        .into(),
                };
                cx.notify();
            },
        );
        Self {
            pad,
            status: "".into(),
            _subscription: subscription,
        }
    }
}
impl Render for SignatureExample {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(self.pad.clone())
            .child(div().text_size(px(12.)).child(self.status.clone()))
    }
}
impl Render for InputExamples {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut stack = div()
            .flex()
            .flex_col()
            .gap(px(28.))
            .w(px(self.width))
            .max_w_full()
            .px(px(16.));
        for view in &self.views {
            stack = stack.child(view.clone());
        }
        div()
            .size_full()
            .flex()
            .justify_center()
            .when(self.padded, |d| {
                d.py(px(if w.viewport_size().width < px(460.) {
                    24.
                } else {
                    54.
                }))
            })
            .when(self.width > 700., |d| d.items_start().pt(px(70.)))
            .when(self.width <= 700., |d| d.items_center())
            .child(stack)
    }
}
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    let l = UiLanguage::current(cx);
    let t = |en: &'static str, zh: &'static str| l.text(en, zh);
    let mut views: Vec<AnyView> = Vec::new();
    match id {
        "ask-your-data" => views.push(cx.new(|cx| AskDataExample::new(w, cx)).into()),
        "pii-redaction" => views.push(cx.new(|cx| PiiExample::new(w, cx)).into()),
        "reply-draft" => views.push(cx.new(|cx| ReplyExample::new(w, cx)).into()),
        "enrich-column" => views.push(cx.new(|cx| EnrichExample::new(w, cx)).into()),
        "ticket-router" => views.push(cx.new(|cx| TicketExample::new(w, cx)).into()),
        "smart-form-fill" => views.push(cx.new(|cx| SmartFillExample::new(w, cx)).into()),
        "tldr-toggle" => views.push(cx.new(|cx| TldrExample::new(w, cx)).into()),
        "presence-stack" => views.push(cx.new(|cx| PresenceExample::new(w, cx)).into()),
        "glass-menu" => views.push(cx.new(|cx| GlassMenuExample::new(w, cx)).into()),
        "glass-segmented" => views.push(cx.new(|cx| GlassExample::new(w, cx)).into()),
        "mega-menu-morph" => {
            let products = MegaMenuSection::new("products", t("Products", "产品"))
                .with_groups(vec![
                    MegaMenuGroup::new(
                        t("Build", "开发"),
                        vec![
                            MegaMenuLink::new("components", t("Components", "组件"))
                                .with_description(t(
                                    "Explore the interface library",
                                    "探索界面组件库",
                                )),
                            MegaMenuLink::new("themes", t("Themes", "主题")).with_description(t(
                                "Adapt colors and appearance",
                                "调整颜色和外观",
                            )),
                            MegaMenuLink::new("install", t("Installation", "安装"))
                                .with_description(t(
                                    "Add KUI to a Rust project",
                                    "将 KUI 加入 Rust 项目",
                                )),
                        ],
                    ),
                    MegaMenuGroup::new(
                        t("Targets", "运行平台"),
                        vec![
                            MegaMenuLink::new("desktop", t("Desktop", "桌面"))
                                .with_description(t("Build native interfaces", "构建原生界面")),
                            MegaMenuLink::new("web", t("Web", "网页"))
                                .with_description(t("Run KUI in the browser", "在浏览器运行 KUI")),
                            MegaMenuLink::new("accessibility", t("Accessibility", "无障碍"))
                                .with_description(t(
                                    "Support keyboard and focus",
                                    "支持键盘和焦点",
                                )),
                        ],
                    ),
                ])
                .with_feature(
                    MegaMenuFeature::new(
                        "updates",
                        t("What's new", "最新动态"),
                        t("Read updates", "查看更新"),
                    )
                    .with_description(t("See the latest improvements", "了解最新功能与改进")),
                );
            let resources =
                MegaMenuSection::new("resources", t("Resources", "资源")).with_groups(vec![
                    MegaMenuGroup::new(
                        t("Learn", "学习"),
                        vec![
                            MegaMenuLink::new("docs", t("Documentation", "文档")).with_description(
                                t("Start building with KUI", "开始使用 KUI 开发"),
                            ),
                            MegaMenuLink::new("examples", t("Examples", "示例"))
                                .with_description(t("See common patterns", "查看常用交互")),
                        ],
                    ),
                ]);
            let sections = vec![
                products,
                resources,
                MegaMenuSection::new("about", t("About", "关于")),
            ];
            views.push(cx.new(|cx| MegaExample::new(sections, w, cx)).into());
        }
        "tree-table" => views.push(cx.new(|cx| TreeExample::new(w, cx)).into()),
        "undo-toast" => views.push(cx.new(|cx| UndoExample::new(w, cx)).into()),
        "morph-select" => {
            let options = vec![
                MorphOption::new("backlog", t("Backlog", "待处理")).with_group(t("Open", "开放")),
                MorphOption::new("todo", t("To do", "待办")).with_group(t("Open", "开放")),
                MorphOption::new("progress", t("In progress", "进行中"))
                    .with_group(t("Open", "开放")),
                MorphOption::new("review", t("In review", "审核中")).with_group(t("Open", "开放")),
                MorphOption::new("done", t("Done", "已完成")).with_group(t("Closed", "已关闭")),
                MorphOption::new("cancelled", t("Cancelled", "已取消"))
                    .with_group(t("Closed", "已关闭")),
            ];
            views.push(
                cx.new(|cx| {
                    MorphSelect::new(t("Status", "状态"), options, w, cx)
                        .with_value("progress")
                        .with_searchable(true)
                })
                .into(),
            );
        }

        "scrub-input" => {
            views.push(
                cx.new(|cx| {
                    ScrubInput::new(t("Width", "宽度"), 0.0..=1000.0, 1.0, 176.0, "px", w, cx)
                        .with_units(vec!["px".into(), "%".into()])
                })
                .into(),
            );
            views.push(
                cx.new(|cx| {
                    ScrubInput::new(t("Rotation", "旋转"), 0.0..=360.0, 1.0, 45.0, "°", w, cx)
                        .with_wrap(true)
                })
                .into(),
            );
        }
        "signature-pad" => views.push(cx.new(|cx| SignatureExample::new(w, cx)).into()),
        "ruler-slider" => {
            views.push(
                cx.new(|cx| {
                    RulerSlider::new(t("Height", "身高"), 120.0..=220.0, 1.0, 172.0, cx)
                        .with_unit("cm")
                })
                .into(),
            );
            views.push(
                cx.new(|cx| {
                    RulerSlider::new(t("Weight", "体重"), 35.0..=140.0, 0.5, 68.5, cx)
                        .with_unit("kg")
                })
                .into(),
            );
        }
        "cookie-consent" => {
            let categories = vec![
                ConsentCategory::new("essential", t("Essential", "必要"))
                    .with_required(true)
                    .with_description(t("Needed for basic site functions", "网站基础功能所需")),
                ConsentCategory::new("analytics", t("Analytics", "分析"))
                    .with_description(t("Help improve the experience", "帮助改善使用体验")),
                ConsentCategory::new("personalization", t("Personalization", "个性化")),
            ];
            views.push(cx.new(|cx| CookieConsent::new(categories, w, cx)).into());
        }
        "announcement-bar" => {
            let messages = vec![
                Announcement::new(
                    "launch",
                    t("Meet the new KUI components", "发现全新的 KUI 组件"),
                )
                .with_action(t("Learn more", "了解更多")),
            ];
            views.push(cx.new(|cx| AnnouncementBar::new(messages, cx)).into());
        }
        "range-histogram-slider" => {
            let data: Vec<f64> = (0..480)
                .map(|i| {
                    let a = ((i * 37 % 149) as f64) / 149.;
                    let b = ((i * 83 % 157) as f64) / 157.;
                    ((a + b) * 210.).round()
                })
                .collect();
            views.push(
                cx.new(|cx| {
                    RangeHistogramSlider::new(
                        t("Price per night", "每晚价格"),
                        data,
                        0.0..=500.0,
                        5.0,
                        (80.0, 260.0),
                        32,
                        w,
                        cx,
                    )
                })
                .into(),
            );
        }
        "elastic-slider" => views.push(
            cx.new(|cx| {
                ElasticSlider::new(t("Volume", "音量"), 0.0..=100.0, 1.0, 50.0, "%", w, cx)
            })
            .into(),
        ),
        "countdown" => views.push(
            cx.new(|cx| {
                Countdown::new(
                    t("Launch", "发布倒计时"),
                    std::time::Duration::from_secs(95),
                    cx,
                )
                .with_complete_label(t("Live now", "现已发布"))
            })
            .into(),
        ),
        "money-input" => views.push(
            cx.new(|cx| {
                MoneyInput::new(t("Amount", "金额"), 12500, 0..=1000000, w, cx)
                    .with_quick_add(vec![1000, 5000])
            })
            .into(),
        ),
        "billing-toggle" => views.push(
            cx.new(|cx| {
                BillingToggle::new(t("Billing period", "计费周期"), cx)
                    .with_labels(t("Monthly", "按月"), t("Yearly", "按年"))
                    .with_savings(t("Save 20%", "节省 20%"))
            })
            .into(),
        ),
        "toggle-group" => {
            let options = vec![
                ToggleOption::new("left", t("Left", "左对齐")),
                ToggleOption::new("center", t("Center", "居中")),
                ToggleOption::new("right", t("Right", "右对齐")),
            ];
            views.push(
                cx.new(|cx| {
                    ToggleGroup::new(t("Alignment", "对齐方式"), options, ToggleMode::Single, cx)
                        .with_selected(&["left"])
                })
                .into(),
            );
            let options = vec![
                ToggleOption::new("bold", t("Bold", "粗体")),
                ToggleOption::new("italic", t("Italic", "斜体")),
                ToggleOption::new("underline", t("Underline", "下划线")),
            ];
            views.push(
                cx.new(|cx| {
                    ToggleGroup::new(t("Formatting", "格式"), options, ToggleMode::Multiple, cx)
                        .with_selected(&["bold"])
                })
                .into(),
            );
        }
        _ => return None,
    }
    let width = if id == "mega-menu-morph" {
        680.
    } else if matches!(
        id,
        "announcement-bar"
            | "presence-stack"
            | "undo-toast"
            | "glass-menu"
            | "glass-segmented"
            | "ticket-router"
            | "ask-your-data"
    ) {
        554.
    } else if id == "tree-table"
        || id == "tldr-toggle"
        || id == "smart-form-fill"
        || id == "enrich-column"
        || id == "reply-draft"
        || id == "pii-redaction"
    {
        680.
    } else {
        390.
    };
    let padded = matches!(
        id,
        "glass-menu" | "glass-segmented" | "ticket-router" | "ask-your-data"
    );
    Some(
        cx.new(|_| InputExamples {
            views,
            width,
            padded,
        })
        .into(),
    )
}

#[cfg(test)]
mod glass_menu_tests {
    use super::{GlassExample, GlassMenuExample, PresenceExample};
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use std::time::Duration;
    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }
    #[test]
    fn menu_keyboard_submenu_select_and_focus_restore() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (view, w) = app.add_window_view(GlassMenuExample::new);
        let trigger = w.debug_bounds("Pool house").expect("trigger").center();
        w.simulate_click(trigger, Modifiers::default());
        tick(w, 600);
        assert!(w.debug_bounds("Move to").is_some());
        w.simulate_keystrokes("down down right");
        tick(w, 300);
        assert!(w.debug_bounds("Projects").is_some());
        w.simulate_keystrokes("left");
        tick(w, 300);
        assert!(w.debug_bounds("Move to").is_some());
        w.simulate_keystrokes("down right down enter");
        tick(w, 500);
        assert!(w.update(|_, cx| view.read(cx).note.contains("projects")));
        assert!(w.debug_bounds("Projects").is_none());
        w.simulate_keystrokes("down");
        tick(w, 300);
        assert!(w.debug_bounds("Move to").is_some());
        w.simulate_keystrokes("escape");
        tick(w, 500);
        assert!(w.debug_bounds("Move to").is_none());
    }

    #[test]
    fn segmented_selection_changes_the_displayed_photo() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (view, w) = app.add_window_view(GlassExample::new);
        assert_eq!(w.update(|_, cx| view.read(cx).current), 0);
        let photo = w.debug_bounds("Tram").expect("second photo").center();
        w.simulate_click(photo, Modifiers::default());
        tick(w, 300);
        assert_eq!(w.update(|_, cx| view.read(cx).current), 1);
        assert_eq!(w.update(|_, cx| view.read(cx).previous), 0);
    }

    #[test]
    fn presence_overflow_roster_keyboard_selection_and_host_updates() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (view, w) = app.add_window_view(PresenceExample::new);
        let add = w.debug_bounds("Someone joins").expect("join").center();
        w.simulate_click(add, Modifiers::default());
        tick(w, 400);
        assert_eq!(w.update(|_, cx| view.read(cx).people.len()), 7);
        let stack = w.debug_bounds("Presence stack").expect("stack").center();
        w.simulate_click(stack, Modifiers::default());
        tick(w, 400);
        assert!(w.debug_bounds("Alex Chen (you) · Active now").is_some());
        w.simulate_keystrokes("end enter");
        tick(w, 400);
        assert!(w.update(|_, cx| view.read(cx).note.contains("guest-1")));
        w.simulate_keystrokes("down");
        tick(w, 400);
        assert!(w.debug_bounds("Alex Chen (you) · Active now").is_some());
        w.simulate_keystrokes("escape");
        tick(w, 400);
        assert!(w.debug_bounds("Alex Chen (you) · Active now").is_none());
    }
}
