//! Local, deterministic host adapters. The installable controls never call model or workspace services.
use super::FamilyStory;
use gpui::{prelude::*, *};
use kui::{
    components::*,
    primitives::{
        control::UiLanguage,
        family::Paint,
        ops_surface::{btn, column, note, row},
    },
};
use std::{rc::Rc, time::Duration};
mod keys;
mod patch_demo;
mod reasoning;

type DemoAction = Rc<dyn Fn(&mut Window, &mut Context<Demo>)>;
struct Demo {
    view: AnyView,
    options: Vec<(&'static str, DemoAction)>,
    info: String,
    options_below: bool,
    segmented: bool,
    selected: usize,
    status_stage: bool,
    reasoning_stage: bool,
    showcase_cap: Option<f32>,
}
impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(w.viewport_size().width);
        let frame = self.showcase_cap.map_or(600., |cap| {
            (width - if width < 500. { 28. } else { 84. }).clamp(100., cap)
        });
        let mut r = column()
            .items_center()
            .w_full()
            .max_w(px(frame))
            .text_color(Paint::get(cx).ink);
        if self.reasoning_stage {
            r = r.gap(px(20.));
        }
        let p = Paint::get(cx);
        let controls = row()
            .flex_wrap()
            .when(self.segmented, |r| {
                r.p(px(3.))
                    .gap(px(2.))
                    .rounded(kui::Theme::global(cx).tokens.radius.md)
                    .border_1()
                    .border_color(p.line)
                    .bg(p.stage)
            })
            .children(self.options.iter().enumerate().map(|(i, (label, action))| {
                let f = action.clone();
                btn(("scenario", i), *label, false, cx, move |s, w, cx| {
                    s.selected = i;
                    f(w, cx);
                    cx.notify()
                })
                .when(self.reasoning_stage, |el| {
                    el.h(px(36.)).px(px(12.)).text_size(px(14.))
                })
                .when(self.segmented && self.selected == i, |el| {
                    el.bg(p.panel).border_1().border_color(p.line)
                })
            }));
        let view = column()
            .w_full()
            .items_center()
            .when(self.status_stage, |v| v.min_h(px(210.)).justify_center())
            .child(self.view.clone());
        if self.options.is_empty() {
            r = r.child(view)
        } else if self.options_below {
            r = r.child(view).child(controls)
        } else {
            r = r.child(controls).child(view)
        }
        r.when(!self.info.is_empty(), |r| {
            r.child(note(self.info.clone(), cx))
        })
    }
}
// Pass cumulative snapshots just as an actual streaming transport does. Use
// short character batches for scripts that do not separate words with spaces.
fn demo_chunks(value: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut words = 0;
    let mut chars = 0;
    for (i, c) in value.char_indices() {
        chars += 1;
        if c.is_whitespace() {
            words += 1
        }
        if words == 4 || chars == 18 {
            chunks.push(value[..i + c.len_utf8()].to_owned());
            chars = 0;
            words = 0
        }
    }
    if chunks.last().is_none_or(|s| s != value) {
        chunks.push(value.to_owned())
    }
    chunks
}
struct ToolTranscript {
    view: Entity<kui::components::tool_call::InvocationInspector>,
    language: UiLanguage,
}
impl Render for ToolTranscript {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = kui::primitives::family::Paint::get(cx);
        div()
            .w_full()
            .max_w(px(560.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .text_size(px(15.))
            .child(
                div().flex().justify_end().child(
                    div()
                        .max_w(relative(0.9))
                        .px_3()
                        .py_2()
                        .rounded(kui::foundation::surface_radius(cx))
                        .bg(p.hover)
                        .child(self.language.text(
                            "Is anything blocking the next release?",
                            "下次发布还有阻塞项吗？",
                        )),
                ),
            )
            .child(div().text_color(p.ink).child(self.language.text(
                "Let me check the open issues, project and database.",
                "我来检查待处理问题、项目和数据库。",
            )))
            .child(self.view.clone())
    }
}

fn later<T: 'static>(
    entity: &Entity<T>,
    ms: u64,
    f: impl FnOnce(&mut T, &mut Context<T>) + 'static,
    cx: &mut Context<Demo>,
) {
    let weak = entity.downgrade();
    cx.spawn(async move |_, cx| {
        cx.background_executor()
            .timer(Duration::from_millis(ms))
            .await;
        let _ = weak.update(cx, f);
    })
    .detach()
}
pub fn story(id: &str, w: &mut Window, cx: &mut Context<FamilyStory>) -> AnyView {
    cx.new(|cx| build(id, w, cx)).into()
}
fn build(id: &str, w: &mut Window, cx: &mut Context<Demo>) -> Demo {
    let l = UiLanguage::current(cx);
    let t = move |a: &'static str, b: &'static str| l.text(a, b);
    let mut options: Vec<(&'static str, DemoAction)> = vec![];
    macro_rules! option {
        ($label:expr,$body:expr) => {
            options.push(($label, Rc::new($body)))
        };
    }
    let view: AnyView = match id {
        "chat-thread" => {
            use chat_thread::*;
            let e = cx.new(|cx| {
                ChatThread::new(
                    id,
                    t("CSV export limit", "CSV 导出上限"),
                    vec![
                        Message {
                            author: "Maya".into(),
                            time: "16:12".into(),
                            text: t(
                                "Yesterday’s export stopped at 10,000 rows. Is that expected?",
                                "昨天的导出在一万行时停止了，这是预期的吗？",
                            )
                            .into(),
                            own: false,
                            attachment: None,
                            reactions: 0,
                        },
                        Message {
                            author: t("You", "你").into(),
                            time: "16:20".into(),
                            text: t(
                                "I’ll check the export job with our backend team.",
                                "我来和后端一起检查导出任务。",
                            )
                            .into(),
                            own: true,
                            attachment: None,
                            reactions: 0,
                        },
                        Message {
                            author: "Daniel".into(),
                            time: "09:18".into(),
                            text: t(
                                "Fixed. The export now streams every row.",
                                "已修复，现在会流式导出所有记录。",
                            )
                            .into(),
                            own: false,
                            attachment: Some("export-check.csv".into()),
                            reactions: 2,
                        },
                    ],
                    w,
                    cx,
                )
            });
            let mut reacted = std::collections::HashSet::new();
            cx.subscribe(&e, move |d: &mut Demo, e, ev: &ChatEvent, cx| match ev {
                ChatEvent::React(i) => e.update(cx, |s, cx| {
                    if let Some(m) = s.messages.get_mut(*i) {
                        if reacted.remove(i) {
                            m.reactions = m.reactions.saturating_sub(1)
                        } else {
                            reacted.insert(*i);
                            m.reactions += 1
                        }
                    }
                    cx.notify()
                }),
                ChatEvent::Send(text) => e.update(cx, |s, cx| {
                    s.messages.push(Message {
                        own: true,
                        author: t("You", "你").into(),
                        time: t("Now", "刚刚").into(),
                        text: text.clone(),
                        attachment: None,
                        reactions: 0,
                    });
                    cx.notify()
                }),
                ChatEvent::Attach => e.update(cx, |s, cx| {
                    s.messages.push(Message {
                        own: true,
                        author: t("You", "你").into(),
                        time: t("Now", "刚刚").into(),
                        text: t("Attached the sample export.", "已添加样例导出。").into(),
                        attachment: Some("revenue-sample.csv".into()),
                        reactions: 0,
                    });
                    cx.notify()
                }),
                ChatEvent::Download(i) => {
                    d.info = e.read(cx).messages[*i]
                        .attachment
                        .clone()
                        .unwrap_or_default();
                    cx.notify()
                }
            })
            .detach();
            e.into()
        }
        "starter-prompts" => {
            use starter_prompts::*;
            let prompts = [
                (
                    t("Write", "写作"),
                    "Gmail",
                    t("Reply to the launch thread", "回复发布邮件"),
                    t("Confirm the new launch date", "确认新的发布日期"),
                ),
                (
                    t("Code", "代码"),
                    "GitHub",
                    t("Review the latest PR", "评审最近的 PR"),
                    t(
                        "Find risky changes and missing tests",
                        "查找风险改动和遗漏测试",
                    ),
                ),
                (
                    t("Analyze", "分析"),
                    "Notion",
                    t("Find interview themes", "梳理访谈主题"),
                    t("Summarize the research notes", "总结研究笔记"),
                ),
                (
                    t("Plan", "计划"),
                    "Linear",
                    t("Plan the next sprint", "规划下个迭代"),
                    t("Fit the backlog into capacity", "按团队容量分配待办"),
                ),
                (
                    t("Write", "写作"),
                    "Google Meet",
                    t("Prepare meeting notes", "准备会议记录"),
                    t("Draft an agenda with decisions", "拟定包含决策事项的议程"),
                ),
            ]
            .into_iter()
            .map(|(g, a, n, d)| Prompt {
                group: g.into(),
                app: a.into(),
                title: n.into(),
                detail: d.into(),
                text: format!("{n}. {d}."),
            })
            .collect();
            let e = cx.new(|cx| {
                PromptShelf::new(
                    id,
                    t("What are we working on, Maya?", "Maya，今天想做些什么？"),
                    prompts,
                    w,
                    cx,
                )
                .with_model("Claude Opus 5.5")
            });
            cx.subscribe(&e, move |d: &mut Demo, _, ev: &PromptSent, cx| {
                d.info = format!("{} · {}", t("Sent", "已发送"), ev.0);
                cx.notify()
            })
            .detach();
            e.into()
        }
        "ai-translate" => {
            use ai_translate::*;
            let e = cx.new(|_| {
                let mut c = MessageTranslation::new(
                    id, "Chloé Nguyen",
                    "Bonjour à toute l'équipe ! La version 4.2 de notre app est prête pour la revue, avec un mode hors ligne. Les premiers essais montrent que la synchronisation prend trois secondes en moyenne. Pouvez-vous vérifier les écrans de paiement avant jeudi ? Merci pour ce sprint.",
                    ["English", "Deutsch", "Français", "Español", "日本語", "Português"].map(String::from).to_vec(),
                ).byline(t("Mobile lead, Lyon · Oct 27, 09:14", "移动端负责人，里昂 · 10 月 27 日 09:14"))
                    .portrait(Image::from_bytes(ImageFormat::Svg, include_bytes!("../../assets/portraits/emma.svg").to_vec()));
                c.detected = "French".into();
                c.credit = "Claude Haiku 5.5".into();
                c
            });
            let failure = Rc::new(std::cell::Cell::new(false));
            let next = failure.clone();
            option!(
                t("Fail next translation", "模拟下次翻译失败"),
                move |_, cx| {
                    next.set(true);
                    cx.notify()
                }
            );
            cx.subscribe(&e, move |_: &mut Demo, e, ev: &TranslationRequest, cx| {
                let n = ev.ticket;
                if failure.replace(false) {
                    later(
                        &e,
                        280,
                        move |s, cx| s.update_translation(n, Err(()), true, cx),
                        cx,
                    );
                    return;
                }
                let lines = match ev.language.as_str() {
                    "Deutsch" => [
                        "Hallo zusammen!",
                        "Version 4.2 unserer App ist mit einem Offline-Modus zur Prüfung bereit.",
                        "Erste Tests zeigen eine durchschnittliche Synchronisierung von drei Sekunden.",
                        "Könnt ihr die Zahlungsansicht bis Donnerstag prüfen?",
                        "Danke für diesen Sprint.",
                    ],
                    "Español" => [
                        "¡Hola equipo!",
                        "La versión 4.2 está lista para revisión, con un modo sin conexión.",
                        "Las primeras pruebas muestran una sincronización media de tres segundos.",
                        "¿Podéis revisar las pantallas de pago antes del jueves?",
                        "Gracias por este sprint.",
                    ],
                    "日本語" => [
                        "こんにちは。",
                        "オフラインモードを備えたバージョン4.2の審査準備ができました。",
                        "初回テストでは同期に平均3秒かかりました。",
                        "木曜日までに支払い画面を確認できますか？",
                        "今スプリントもありがとうございました。",
                    ],
                    "Português" => [
                        "Olá, equipe!",
                        "A versão 4.2 está pronta para revisão, com modo offline.",
                        "Os primeiros testes indicam três segundos de sincronização em média.",
                        "Podem rever os ecrãs de pagamento até quinta-feira?",
                        "Obrigado por este sprint.",
                    ],
                    _ => [
                        "Hello team!",
                        "Version 4.2 is ready for review, with an offline mode.",
                        "Early tests show that syncing takes three seconds on average.",
                        "Could you check the payment screens by Thursday?",
                        "Thanks for your work this sprint.",
                    ],
                };
                let full = lines.join(" ");
                let chunks = demo_chunks(&full);
                let last = chunks.len().saturating_sub(1);
                for (i, text) in chunks.into_iter().enumerate() {
                    later(
                        &e,
                        100 + i as u64 * 34,
                        move |s, cx| s.update_translation(n, Ok(text), i == last, cx),
                        cx,
                    )
                }
            })
            .detach();
            e.into()
        }
        "agent-timeline" => {
            use agent_timeline::*;
            let names = [
                t("Orchestrator", "编排器"),
                t("Checker", "检查器"),
                t("Researcher", "研究员"),
                t("Analyst", "分析师"),
                t("Writer", "撰写者"),
            ];
            let spans = [
                (
                    0,
                    0.,
                    2.4,
                    0,
                    None,
                    t("Scope the conversion drop", "界定转化率下降范围"),
                ),
                (
                    1,
                    2.4,
                    8.,
                    1,
                    Some(0),
                    t("Read checkout events", "读取结账事件"),
                ),
                (
                    2,
                    2.9,
                    13.,
                    1,
                    Some(0),
                    t("Compare release notes", "对比发布记录"),
                ),
                (
                    3,
                    8.,
                    18.2,
                    0,
                    Some(1),
                    t("Locate the slow step", "定位缓慢环节"),
                ),
                (
                    4,
                    18.2,
                    24.,
                    2,
                    Some(3),
                    t("Summarize the evidence", "汇总证据"),
                ),
                (0, 24., 26., 3, Some(4), t("Hand off report", "交付报告")),
            ]
            .into_iter()
            .map(|(lane, start, end, kind, parent, d)| Span {
                lane,
                start,
                end,
                kind,
                parent,
                detail: d.into(),
            })
            .collect();
            cx.new(|_| {
                RunLanes::new(
                    id,
                    t(
                        "Checkout conversion drop on Oct 14",
                        "10 月 14 日结账转化率下降",
                    ),
                    names.map(String::from).to_vec(),
                    spans,
                )
                .with_models(
                    ["Kense", "Kense Mini", "Kense Mini", "Kense", "Kense"]
                        .map(Into::into)
                        .to_vec(),
                )
                .with_totals(t("12.4k tokens · $0.08", "12.4k 词元 · $0.08"))
            })
            .into()
        }
        "context-window" => {
            use context_window::*;
            let bands = [
                (t("System prompt", "系统指令"), 4100, 0x782ba6),
                (t("Tools", "工具"), 32000, 0xe49a12),
                (t("Files and memory", "文件和记忆"), 100000, 0x089b86),
                (t("Summary", "摘要"), 0, 0x92b9dd),
                (t("Messages", "消息"), 291000, 0x287cc1),
            ]
            .into_iter()
            .map(|(n, tokens, c)| Band {
                label: n.into(),
                tokens,
                color: rgb(c).into(),
            })
            .collect();
            let e = cx.new(|_| {
                let mut s = CapacityBands::new(id, "Claude Opus 5.5", 1_000_000, bands);
                s.reserved = 64000;
                s
            });
            cx.subscribe(&e, |_: &mut Demo, e, a: &CapacityAction, cx| {
                e.update(cx, |s, cx| {
                    let before = s.used();
                    let mut compacted = matches!(a, CapacityAction::Compact);
                    match a {
                        CapacityAction::Message => s.bands[4].tokens += 21000,
                        CapacityAction::Attach => s.bands[2].tokens += 44000,
                        CapacityAction::Compact => {
                            s.bands[3].tokens += s.bands[4].tokens / 8;
                            s.bands[4].tokens = 0
                        }
                    }
                    if s.used() as f32 >= s.threshold * s.capacity as f32 {
                        compacted = true;
                        s.bands[3].tokens = (s.bands[3].tokens + s.bands[4].tokens) / 8;
                        s.bands[4].tokens = 0;
                        s.bands[2].tokens /= 2
                    }
                    if compacted {
                        s.did_compact(before, cx);
                    }
                    cx.notify()
                })
            })
            .detach();
            e.into()
        }
        "agent-status" => {
            use agent_status::*;
            let e = cx.new(|_| {
                WorkBadge::new(
                    id,
                    t("Coding agent", "代码助手"),
                    t("Ready", "就绪"),
                    vec![],
                )
            });
            let epoch = Rc::new(std::cell::Cell::new(0u64));
            for (research, label) in [
                (false, t("Coding agent", "代码助手")),
                (true, t("Research agent", "研究助手")),
            ] {
                let e = e.clone();
                let epoch = epoch.clone();
                option!(label, move |_, cx| {
                    let generation = epoch.get() + 1;
                    epoch.set(generation);
                    let steps = if research {
                        [
                            t("Search documents", "搜索文档"),
                            t("Check sources", "核查来源"),
                            t("Prepare report", "生成报告"),
                        ]
                    } else {
                        [
                            t("Inspect workspace", "检查工作区"),
                            t("Run checks", "执行检查"),
                            t("Summarize changes", "总结改动"),
                        ]
                    };
                    let run = e.update(cx, |v, cx| {
                        v.elapsed = 0;
                        v.agent = label.into();
                        v.activity = steps[0].into();
                        v.steps = steps
                            .iter()
                            .map(|n| Step {
                                label: (*n).into(),
                                result: String::new(),
                                complete: false,
                            })
                            .collect();
                        cx.notify();
                        v.activity.clone()
                    });
                    for i in 0..3 {
                        let epoch = epoch.clone();
                        let first = run.clone();
                        later(
                            &e,
                            700 * (i + 1) as u64,
                            move |v, cx| {
                                if generation != epoch.get()
                                    || v.steps.first().is_none_or(|s| s.label != first)
                                    || v.elapsed as usize != i
                                {
                                    return;
                                }
                                v.steps[i].complete = true;
                                v.steps[i].result = t("Done", "已完成").into();
                                v.elapsed += 1;
                                v.activity = steps
                                    .get(i + 1)
                                    .copied()
                                    .unwrap_or(t("Complete", "已完成"))
                                    .into();
                                cx.notify()
                            },
                            cx,
                        )
                    }
                })
            }
            e.into()
        }
        "tool-call" => {
            use tool_call::*;
            let bridge = cx.try_global::<crate::StoryBridge>().map(|b| b.visible);
            let e = cx.new(|_| {
                let view = InvocationInspector::new(
                    id,
                    vec![
                        Invocation {
                            name: "GitHub".into(),
                            action: t(
                                "Searched issues in kense-labs/KUI",
                                "搜索 kense-labs/KUI 的问题",
                            )
                            .into(),
                            input: "{\"label\": \"blocking-release\"}".into(),
                            output: t("4 results", "4 条结果").into(),
                            state: ToolState::Done,
                        },
                        Invocation {
                            name: "Linear".into(),
                            action: t("Read the release project", "查看发布项目").into(),
                            input: "{\"project\": \"KUI Release\"}".into(),
                            output: t("2 blocked", "2 项阻塞").into(),
                            state: ToolState::Done,
                        },
                        Invocation {
                            name: "PostgreSQL".into(),
                            action: t("Query pending migrations", "查询待处理迁移").into(),
                            input: "SELECT id, status FROM migrations WHERE status = 'pending';"
                                .into(),
                            output: t("The database connection timed out.", "数据库连接超时。")
                                .into(),
                            state: ToolState::Failed,
                        },
                    ],
                )
                .durations(vec!["1.2s".into(), "760ms".into(), "910ms".into()]);
                if let Some(visible) = bridge {
                    view.visibility(visible)
                } else {
                    view
                }
            });
            cx.subscribe(&e, move |_: &mut Demo, e, a: &ToolAction, cx| {
                let i = match a {
                    ToolAction::Approve(i) | ToolAction::Retry(i) => *i,
                    ToolAction::Deny(_) => return,
                };
                later(
                    &e,
                    720,
                    move |s, cx| {
                        if let Some(call) =
                            s.calls.get_mut(i).filter(|c| c.state == ToolState::Running)
                        {
                            call.state = ToolState::Done;
                            call.output = if i == 2 {
                                t("1 pending migration", "1 项待处理迁移")
                            } else {
                                t("Checklist created", "已创建检查清单")
                            }
                            .into();
                            cx.notify();
                        }
                    },
                    cx,
                )
            })
            .detach();
            let x = e.clone();
            option!(t("Simulate failure", "模拟失败"), move |_, cx| x
                .update(cx, |s, cx| {
                    s.calls[2].state = ToolState::Failed;
                    s.calls[2].output =
                        t("The database connection timed out.", "数据库连接超时。").into();
                    cx.notify();
                }));
            let x = e.clone();
            option!(t("Request approval", "请求批准"), move |_, cx| x
                .update(cx, |s, cx| {
                    s.calls[1].state = ToolState::Approval;
                    s.calls[1].action = t("Create a release checklist", "创建发布检查清单").into();
                    s.calls[1].output = t(
                        "This will create a new issue in Linear.",
                        "将在 Linear 创建一个问题。",
                    )
                    .into();
                    cx.notify();
                }));
            cx.new(|_| ToolTranscript {
                view: e,
                language: l,
            })
            .into()
        }
        "agent-plan" => {
            use agent_plan::*;
            let steps = [
                t("Map the public API routes", "梳理公开 API 路由"),
                t("Choose a sliding window rate limit", "确定滑动窗口限流规则"),
                t("Add expiring request counters", "添加会过期的请求计数器"),
                t(
                    "Return retry and rate limit headers",
                    "返回重试和限流响应头",
                ),
                t("Test bursts and window resets", "测试突发请求和窗口重置"),
                t(
                    "Document the limits in the API reference",
                    "在 API 文档记录限制",
                ),
            ]
            .map(String::from)
            .to_vec();
            let bridge = cx.try_global::<crate::StoryBridge>().map(|b| b.visible);
            let e = cx.new(|cx| {
                let view = TaskPlan::new(
                    id,
                    t("Add rate limiting to the public API", "为公开 API 添加限流"),
                    steps,
                    w,
                    cx,
                )
                .model("Claude Opus 5.5");
                if let Some(visible) = bridge {
                    view.visibility(visible)
                } else {
                    view
                }
            });
            let generation = Rc::new(std::cell::Cell::new(0u64));
            cx.subscribe(&e, move |d: &mut Demo, e, a: &PlanEvent, cx| match a {
                PlanEvent::Approve | PlanEvent::Pause(false) => {
                    let n = generation.get() + 1;
                    generation.set(n);
                    let first = e.read(cx).completed;
                    for i in first..e.read(cx).steps.len() {
                        let generation = generation.clone();
                        later(
                            &e,
                            680 * (i - first + 1) as u64,
                            move |s, cx| {
                                if generation.get() == n && s.approved {
                                    s.completed = i + 1;
                                    cx.notify();
                                }
                            },
                            cx,
                        )
                    }
                }
                PlanEvent::Pause(true) | PlanEvent::Restart => generation.set(generation.get() + 1),
                PlanEvent::Revise => {
                    d.info = t(
                        "Edit a step or add the missing work.",
                        "编辑步骤，或添加尚未列出的工作。",
                    )
                    .into();
                    cx.notify()
                }
                PlanEvent::Changed => {}
            })
            .detach();
            let x = e.clone();
            option!(
                t("Fail current step", "模拟当前步骤失败"),
                move |_, cx| x.update(cx, |s, cx| {
                    s.fail_current(t("The local check timed out.", "本地检查超时。"), cx);
                })
            );
            e.into()
        }
        "reasoning-block" => reasoning::demo(id, l, &mut options, w, cx),
        "source-cards" => {
            use source_cards::*;
            let sources = [
                (
                    "stripe.com",
                    t("Check the webhook signatures", "检查 Webhook 签名"),
                    "https://docs.stripe.com/webhooks/signature",
                    t(
                        "Use the unchanged raw request body to verify the signature.",
                        "使用未经转换的原始请求体验证签名。",
                    ),
                ),
                (
                    "mozilla.org",
                    "Request: text()",
                    "https://developer.mozilla.org/en-US/docs/Web/API/Request/text",
                    t(
                        "Read the request body as UTF-8 text.",
                        "将请求体读取为 UTF-8 文本。",
                    ),
                ),
                (
                    "github.com",
                    t(
                        "Verifying signatures in App Router",
                        "在 App Router 中验证签名",
                    ),
                    "https://github.com/stripe/stripe-node",
                    t(
                        "Check your route handler and signing secret.",
                        "检查路由处理程序和签名密钥。",
                    ),
                ),
                (
                    "vercel.com",
                    "Vercel Functions",
                    "https://vercel.com/docs/functions",
                    t(
                        "Read the incoming Request in the handler.",
                        "在处理程序中读取传入的 Request。",
                    ),
                ),
                (
                    "linear.app",
                    t("Troubleshoot a webhook", "排查 Webhook"),
                    "https://linear.app/docs/api-and-webhooks",
                    t(
                        "Inspect the delivery and signing details.",
                        "检查投递和签名详情。",
                    ),
                ),
                (
                    "w3.org",
                    "Web API",
                    "https://www.w3.org/",
                    t("Reference material for web requests.", "网页请求参考资料。"),
                ),
                (
                    "figma.com",
                    "Webhooks",
                    "https://developers.figma.com/docs/rest-api/webhooks/",
                    t(
                        "Compare webhook payload and delivery details.",
                        "对比 Webhook 请求和投递详情。",
                    ),
                ),
            ]
            .map(|(d, n, url, p)| Citation {
                domain: d.into(),
                title: n.into(),
                url: url.into(),
                passage: p.into(),
            })
            .to_vec();
            let e = cx.new(|_| {
                let mut shelf = CitationShelf::new(
                    id,
                    t(
                        "Why does my Stripe webhook fail signature verification in a Next.js route handler?",
                        "为什么 Stripe Webhook 在 Next.js 路由中无法通过签名验证？",
                    ),
                    sources.clone(),
                );
                shelf.state = SearchState::Searching;
                shelf
            });
            later(
                &e,
                1500,
                |s, cx| {
                    if matches!(s.state, SearchState::Searching) && !s.sources.is_empty() {
                        s.state = SearchState::Ready;
                        cx.notify();
                    }
                },
                cx,
            );
            for (mode, label) in [
                (0, t("Live", "实时")),
                (1, t("Failed", "失败")),
                (2, t("No results", "无结果")),
            ] {
                let e = e.clone();
                let data = sources.clone();
                option!(label, move |_, cx| {
                    e.update(cx, |s, cx| {
                        s.sources = if mode == 0 { data.clone() } else { vec![] };
                        s.state = match mode {
                            1 => SearchState::Failed,
                            2 => SearchState::Empty,
                            _ => SearchState::Ready,
                        };
                        cx.notify()
                    })
                })
            }
            cx.subscribe(&e, move |_: &mut Demo, e, a: &SourceEvent, cx| match a {
                SourceEvent::Open(url) => cx.open_url(url),
                SourceEvent::Retry => {
                    let data = sources.clone();
                    later(
                        &e,
                        350,
                        move |s, cx| {
                            if matches!(s.state, SearchState::Searching) && s.sources.is_empty() {
                                s.sources = data;
                                s.state = SearchState::Ready;
                                cx.notify();
                            }
                        },
                        cx,
                    )
                }
            })
            .detach();
            e.into()
        }
        "generation-stats" => {
            use generation_stats::*;
            let runs = [
                (
                    "Claude Sonnet 5.5",
                    420u32,
                    3640u32,
                    284u32,
                    1147u32,
                    3.,
                    15.,
                ),
                ("GPT-6.1 Sol", 550, 4590, 336, 1370, 4., 18.),
                ("Gemini 3.8 Flash", 260, 2960, 642, 1640, 0.45, 2.75),
                ("Mistral Large 4", 380, 3290, 244, 1024, 2., 7.),
            ];
            let visible: fn() -> bool = cx
                .try_global::<crate::StoryBridge>()
                .map_or((|| true) as fn() -> bool, |b| b.visible);
            let data = move |index: usize, elapsed: u32| {
                let (_, ttft, duration, target, input_tokens, input_rate, output_rate) =
                    runs[index];
                let stream = elapsed.saturating_sub(ttft) as f32 / (duration - ttft) as f32;
                let output_tokens = (target as f32 * stream.clamp(0., 1.)).round() as u32;
                let base_speed = target as f32 * 1000. / (duration - ttft) as f32;
                Metrics {
                    first_token_ms: if elapsed >= ttft { ttft } else { 0 },
                    duration_ms: elapsed.min(duration),
                    input_tokens,
                    output_tokens,
                    input_rate,
                    output_rate,
                    running: elapsed < duration,
                    samples: (0..elapsed.saturating_sub(ttft).div_ceil(160).min(48))
                        .map(|n| {
                            base_speed
                                * (1.
                                    + 0.08 * (n as f32 * 0.41).sin()
                                    + 0.02 * (n as f32 * 0.74).sin())
                        })
                        .collect(),
                }
            };
            let e = cx.new(|_| {
                GenerationReadout::new(id, runs[0].0, data(0, runs[0].2)).visibility(visible)
            });
            let selected = Rc::new(std::cell::Cell::new(0usize));
            let epoch = Rc::new(std::cell::Cell::new(0u64));
            let failed_mistral = Rc::new(std::cell::Cell::new(false));
            let play = {
                let weak = e.downgrade();
                let selection = selected.clone();
                move |index: usize, cycling: bool, cx: &mut Context<Demo>| {
                    let id = epoch.get() + 1;
                    epoch.set(id);
                    let epoch = epoch.clone();
                    let failure = failed_mistral.clone();
                    let weak = weak.clone();
                    let selection = selection.clone();
                    let reduce = kui::primitives::family::disabled_motion(cx);
                    let start =
                        move |view: &mut GenerationReadout,
                              index: usize,
                              cx: &mut Context<GenerationReadout>| {
                            let (model, _, duration, _, input, input_rate, _) = runs[index];
                            view.model = model.to_owned();
                            view.set_usage(
                                UsageBreakdown {
                                    cached_tokens: if index == 0 { 0 } else { input / 6 },
                                    reasoning_tokens: 0,
                                    cached_input_rate: input_rate * 0.1,
                                },
                                cx,
                            );
                            view.set_metrics(data(index, if reduce { duration } else { 0 }), cx);
                            view.set_state(
                                if reduce {
                                    GenerationState::Done
                                } else {
                                    GenerationState::Waiting
                                },
                                cx,
                            );
                        };
                    if weak.update(cx, |v, cx| start(v, index, cx)).is_err() || reduce {
                        return;
                    }
                    cx.spawn(async move |_, cx| {
                        let mut index = index;
                        'run: loop {
                            let (_, ttft, duration, _, _, _, _) = runs[index];
                            let mut time = 0;
                            while time < duration {
                                cx.background_executor()
                                    .timer(Duration::from_millis(if visible() { 80 } else { 320 }))
                                    .await;
                                if epoch.get() != id || weak.upgrade().is_none() {
                                    break 'run;
                                }
                                if !visible() {
                                    continue;
                                }
                                let previous = time;
                                time = (time + 80).min(duration);
                                let fail = index == 3 && time >= 1500 && !failure.replace(true);
                                if weak
                                    .update(cx, |view, cx| {
                                        view.set_metrics(data(index, time), cx);
                                        if fail {
                                            view.set_state(
                                                GenerationState::Failed(
                                                    t(
                                                        "The sample response timed out.",
                                                        "样例响应超时。",
                                                    )
                                                    .into(),
                                                ),
                                                cx,
                                            );
                                        } else if time == duration {
                                            view.set_state(GenerationState::Done, cx);
                                        } else if previous < ttft && time >= ttft {
                                            view.set_state(GenerationState::Streaming, cx);
                                        }
                                    })
                                    .is_err()
                                    || fail
                                {
                                    break 'run;
                                }
                            }
                            if !cycling {
                                break;
                            }
                            let mut dwell = 0;
                            while dwell < 1600 {
                                cx.background_executor()
                                    .timer(Duration::from_millis(160))
                                    .await;
                                if epoch.get() != id {
                                    break 'run;
                                }
                                match weak.update(cx, |v, cx| {
                                    (
                                        v.details_open(),
                                        kui::primitives::family::disabled_motion(cx),
                                    )
                                }) {
                                    Ok((false, false)) if visible() => dwell += 160,
                                    Ok((_, false)) => {}
                                    _ => break 'run,
                                }
                            }
                            index = (index + 1) % runs.len();
                            selection.set(index);
                            if weak.update(cx, |v, cx| start(v, index, cx)).is_err() {
                                break;
                            }
                        }
                    })
                    .detach();
                }
            };
            let replay = play.clone();
            let index = selected.clone();
            option!(
                t("Replay generation", "重播生成过程"),
                move |_, cx| replay(index.get(), false, cx)
            );
            let switch = play.clone();
            let index = selected.clone();
            option!(
                t("Next sample model", "下一个样例模型"),
                move |_, cx| {
                    index.set((index.get() + 1) % runs.len());
                    switch(index.get(), false, cx);
                }
            );
            play(0, true, cx);
            cx.subscribe(&e, move |_: &mut Demo, _, _: &RetryGeneration, cx| {
                play(selected.get(), true, cx)
            })
            .detach();
            e.into()
        }
        "comment-thread" => {
            use comment_thread::*;
            let comments = [
                (
                    1,
                    None,
                    "Jasmine",
                    t(
                        "Can the headline fit on one line on mobile?",
                        "标题能在移动端放进一行吗？",
                    ),
                    false,
                ),
                (
                    2,
                    Some(1),
                    "Emma",
                    t(
                        "Yes. The shorter version is in the draft.",
                        "可以，草稿里已有较短的版本。",
                    ),
                    true,
                ),
                (
                    3,
                    Some(1),
                    "Marcus",
                    t(
                        "That reads better at small sizes.",
                        "这个在小尺寸下更易读。",
                    ),
                    false,
                ),
                (
                    4,
                    None,
                    "Daniel",
                    t(
                        "I’ll update the social image after approval.",
                        "批准后我会更新分享图片。",
                    ),
                    false,
                ),
            ]
            .into_iter()
            .map(|(key, parent, a, b, mine)| Comment {
                key,
                parent,
                author: a.into(),
                text: b.into(),
                mine,
                age: "1h".into(),
                likes: if key == 1 { 2 } else { 0 },
            })
            .collect();
            let e = cx
                .new(|cx| CommentThread::new(id, t("Hero headline", "首页标题"), comments, w, cx));
            cx.subscribe(&e, move |_: &mut Demo, e, a: &CommentEvent, cx| {
                e.update(cx, |s, cx| {
                    match a {
                        CommentEvent::Post { parent, body } => {
                            let key = s.comments.iter().map(|c| c.key).max().unwrap_or(0) + 1;
                            s.comments.push(Comment {
                                key,
                                parent: *parent,
                                author: "Emma".into(),
                                text: body.clone(),
                                age: t("Now", "刚刚").into(),
                                likes: 0,
                                mine: true,
                            })
                        }
                        CommentEvent::Edit(k, b) => {
                            if let Some(c) = s.comments.iter_mut().find(|c| c.key == *k && c.mine) {
                                c.text = b.clone()
                            }
                        }
                        CommentEvent::Delete(k) => {
                            s.comments.retain(|c| c.key != *k && c.parent != Some(*k))
                        }
                        CommentEvent::Like(k) => {
                            if let Some(c) = s.comments.iter_mut().find(|c| c.key == *k) {
                                c.likes += 1
                            }
                        }
                        CommentEvent::Resolve(_) => {}
                    }
                    cx.notify()
                })
            })
            .detach();
            e.into()
        }
        "code-suggestion" => {
            let demo = cx.new(|cx| patch_demo::PatchDemo::new(w, cx));
            let suggestion = demo.clone();
            let empty = demo.clone();
            let error = demo.clone();
            option!(t("Suggestion", "建议"), move |_, cx| suggestion
                .update(cx, |s, cx| s.start(cx)));
            option!(t("No changes", "无改动"), move |_, cx| empty
                .update(cx, |s, cx| s.show_empty(cx)));
            option!(t("Error", "错误"), move |_, cx| error
                .update(cx, |s, cx| s.show_error(cx)));
            demo.into()
        }
        "model-card" => {
            use model_card::*;
            let models = [
                (
                    "Claude Opus 5.5",
                    "Anthropic",
                    1_000_000,
                    128000,
                    5.,
                    25.,
                    0.91,
                ),
                (
                    "Claude Sonnet 5.5",
                    "Anthropic",
                    1_000_000,
                    64000,
                    1.5,
                    6.,
                    0.84,
                ),
                (
                    "GPT-6.1 Sol",
                    "OpenAI",
                    1_000_000,
                    128000,
                    4.,
                    18.,
                    0.73,
                ),
                ("Gemini 4 Argon", "Google AI", 1_000_000, 64000, 2., 12., 0.71),
            ]
            .into_iter()
            .map(
                |(n, p, context, max_output, input_rate, output_rate, b)| Model {
                    name: n.into(),
                    provider: p.into(),
                    description: t(
                        "Illustrative model profile for reasoning, code, and document tasks. Prices and scores are sample data.",
                        "面向推理、代码和文档任务的演示档案。价格和评分均为样例数据。",
                    )
                    .into(),
                    features: vec![
                        t("Text", "文本").into(),
                        t("Image", "图像").into(),
                        t("Tools", "工具").into(),
                    ],
                    context,
                    max_output,
                    input_rate,
                    output_rate,
                    scores: vec![
                        (t("Code", "代码").into(), b),
                        (t("Reasoning", "推理").into(), b - 0.06),
                    ],
                },
            )
            .collect();
            cx.new(|_| ModelSheet::new(id, models)).into()
        }
        "provider-keys" => keys::demo(w, cx),
        "connector-setup" => {
            use connector_setup::*;
            let services = [
                ("Slack", t("Search channels and threads", "搜索频道与会话")),
                (
                    "GitHub",
                    t(
                        "Search repositories and pull requests",
                        "搜索仓库和拉取请求",
                    ),
                ),
                (
                    "Notion",
                    t("Search pages and databases", "搜索页面和数据库"),
                ),
                ("Google Drive", t("Search shared documents", "搜索共享文档")),
                ("Linear", t("Create and update issues", "创建和更新问题")),
                ("Jira", t("Track tickets and releases", "跟踪任务和发布")),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (n, d))| Service {
                id: n.into(),
                name: n.into(),
                description: d.into(),
                scopes: vec!["read:content".into(), "read:profile".into()],
                state: if [1, 3].contains(&i) {
                    ConnectionState::Connected
                } else {
                    ConnectionState::Offline
                },
            })
            .collect();
            let e = cx.new(|_| ConnectionWizard::new(id, services));
            let fail = Rc::new(std::cell::Cell::new(false));
            let next = fail.clone();
            option!(
                t("Fail next connection", "模拟连接失败"),
                move |_, _| next.set(true)
            );
            cx.subscribe(&e, move |_: &mut Demo, e, a: &ConnectionAction, cx| {
                if let ConnectionAction::Connect { service, ticket } = a {
                    let n = *ticket;
                    let failure = fail.replace(false);
                    for i in 1..=4 {
                        let id = service.clone();
                        later(
                            &e,
                            220 * i,
                            move |s, cx| {
                                let state = if failure {
                                    ConnectionState::Failed
                                } else if i == 4 {
                                    ConnectionState::Connected
                                } else {
                                    ConnectionState::Syncing(i as f32 / 4.)
                                };
                                s.progress(&id, n, state, cx)
                            },
                            cx,
                        )
                    }
                }
            })
            .detach();
            e.into()
        }
        "ai-command" => {
            use ai_command::*;
            let commands = [
                (
                    "dashboard",
                    t("Dashboard", "仪表盘"),
                    t("Pages", "页面"),
                    "G D",
                ),
                ("invoices", t("Invoices", "发票"), t("Pages", "页面"), "G I"),
                (
                    "customers",
                    t("Customers", "客户"),
                    t("Pages", "页面"),
                    "G C",
                ),
                (
                    "create",
                    t("Create invoice", "创建发票"),
                    t("Actions", "操作"),
                    "C",
                ),
                (
                    "invite",
                    t("Invite teammate", "邀请成员"),
                    t("Actions", "操作"),
                    "⌘ ⇧ I",
                ),
                (
                    "linear",
                    t("Search Linear issues", "搜索 Linear 问题"),
                    t("Connected apps", "已连接应用"),
                    "Linear",
                ),
                (
                    "notion",
                    t("Open the wiki", "打开团队文档"),
                    t("Connected apps", "已连接应用"),
                    "Notion",
                ),
            ]
            .map(|(id, n, g, h)| Command {
                id: id.into(),
                label: n.into(),
                section: g.into(),
                hint: h.into(),
            })
            .into_iter()
            .collect();
            let e =
                cx.new(|cx| CommandDesk::new(id, commands, w, cx).with_model("Claude Sonnet 5.5"));
            let fail = Rc::new(std::cell::Cell::new(false));
            let next = fail.clone();
            option!(t("Fail next request", "模拟请求失败"), move |_, _| {
                next.set(true)
            });
            cx.subscribe(&e, move |d: &mut Demo, e, a: &CommandEvent, cx| match a {
                CommandEvent::Ask { ticket, query } => {
                    let n = *ticket;
                    if fail.replace(false) {
                        later(&e, 300, move |s, cx| s.receive(n, Err(()), true, cx), cx);
                        return;
                    }
                    let answer = format!(
                        "{} “{}”.",
                        t("Sample workspace summary for", "工作区样例摘要"),
                        query
                    );
                    let chunks = demo_chunks(&answer);
                    let last = chunks.len().saturating_sub(1);
                    for (i, text) in chunks.into_iter().enumerate() {
                        later(
                            &e,
                            100 + i as u64 * 34,
                            move |s, cx| s.receive(n, Ok(text), i == last, cx),
                            cx,
                        )
                    }
                }
                CommandEvent::Open(id) => {
                    d.info = format!("{} · {}", t("Selected action", "已选操作"), id);
                    cx.notify()
                }
                CommandEvent::Cancel => {
                    d.info.clear();
                    cx.notify()
                }
            })
            .detach();
            e.into()
        }
        "share-access" => {
            use share_access::*;
            let people = [
                ("Jasmine Brooks", "jasmine", AccessRole::Owner),
                ("Nathan Cole", "nathan", AccessRole::Edit),
                ("Chloe Nguyen", "chloe", AccessRole::Comment),
                ("Daniel Kim", "daniel", AccessRole::View),
            ]
            .map(|(n, a, role)| Person {
                name: n.into(),
                email: format!("{a}@workspace.example"),
                role,
            })
            .into_iter()
            .collect();
            let e = cx.new(|cx| {
                AccessEditor::new(
                    id,
                    t("Q3 launch plan", "第三季度发布计划"),
                    "https://workspace.example/q3-launch",
                    people,
                    w,
                    cx,
                )
            });
            cx.subscribe(&e, |_: &mut Demo, e, a: &AccessEvent, cx| {
                if let AccessEvent::Invite(email) = a {
                    e.update(cx, |s, cx| {
                        s.people.push(Person {
                            name: email.split('@').next().unwrap_or(email).into(),
                            email: email.clone(),
                            role: AccessRole::View,
                        });
                        cx.notify()
                    })
                }
            })
            .detach();
            e.into()
        }
        "team-switcher" => {
            use team_switcher::*;
            let teams = [
                ("Linear", "Enterprise", 214, t("Admin", "管理员")),
                ("Vercel", "Pro", 38, t("Owner", "所有者")),
                ("Stripe", "Enterprise", 1240, t("Member", "成员")),
                ("Figma", "Organization", 96, t("Member", "成员")),
                ("Supabase", "Team", 52, t("Billing", "账务")),
                ("Raycast", "Pro", 24, t("Guest", "访客")),
                ("Design systems", "Team", 18, t("Member", "成员")),
                ("Platform", "Team", 32, t("Admin", "管理员")),
            ]
            .map(|(n, p, m, r)| Workspace {
                id: n.into(),
                name: n.into(),
                plan: p.into(),
                members: m,
                role: r.into(),
            })
            .into_iter()
            .collect();
            let e = cx.new(|cx| WorkspacePicker::new(id, teams, w, cx));
            cx.subscribe(&e, move |d: &mut Demo, e, a: &WorkspaceEvent, cx| {
                let (name, role, action) = match a {
                    WorkspaceEvent::Switch(id) => {
                        let name = e
                            .read(cx)
                            .teams
                            .iter()
                            .find(|team| team.id == *id)
                            .map_or(id.as_str(), |team| team.name.as_str());
                        d.info = format!("{} · {name}", t("Workspace", "工作区"));
                        cx.notify();
                        return;
                    }
                    WorkspaceEvent::Create(name) => (
                        name.clone(),
                        t("Owner", "所有者"),
                        t("Created workspace", "已创建工作区"),
                    ),
                    WorkspaceEvent::Join(_) => (
                        t("Invited workspace", "受邀工作区").to_owned(),
                        t("Member", "成员"),
                        t("Joined demo workspace", "已加入示例工作区"),
                    ),
                };
                e.update(cx, |s, cx| {
                    s.teams.push(Workspace {
                        id: format!("workspace-{}", s.teams.len()),
                        name: name.clone(),
                        plan: "Team".into(),
                        members: 1,
                        role: role.into(),
                    });
                    s.current = s.teams.len() - 1;
                    cx.notify();
                });
                d.info = format!("{action} · {name}");
                cx.notify();
            })
            .detach();
            e.into()
        }
        _ => unreachable!("unknown workspace story: {id}"),
    };
    Demo {
        view,
        options,
        info: String::new(),
        options_below: matches!(
            id,
            "ai-translate"
                | "agent-status"
                | "tool-call"
                | "agent-plan"
                | "generation-stats"
                | "connector-setup"
                | "ai-command"
        ),
        segmented: matches!(
            id,
            "reasoning-block" | "source-cards" | "code-suggestion" | "agent-status"
        ),
        selected: 0,
        status_stage: id == "agent-status",
        reasoning_stage: id == "reasoning-block",
        showcase_cap: match id {
            "ai-translate" => Some(620.),
            "tool-call" | "agent-plan" => Some(560.),
            "code-suggestion" => Some(720.),
            "generation-stats" => Some(760.),
            _ => None,
        },
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
