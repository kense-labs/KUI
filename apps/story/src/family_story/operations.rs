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
}
impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut r = column()
            .items_center()
            .w_full()
            .max_w(px(600.))
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
                    "Calendar",
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
            let e=cx.new(|_|{let mut c=MessageTranslation::new(id,"Chloé Nguyen","Bonjour à toute l’équipe ! La nouvelle version est prête. Pouvez-vous vérifier les écrans de paiement avant jeudi ?",["English","Deutsch","Español","日本語","Português"].map(String::from).to_vec());c.detected="Français".into();c.credit=t("Sample translation · Local demo","样例译文 · 本地演示").into();c});
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
                        "Die neue Version ist fertig.",
                        "Bitte prüft die Zahlungsansicht bis Donnerstag.",
                    ],
                    "Español" => [
                        "¡Hola equipo!",
                        "La nueva versión está lista.",
                        "Revisad las pantallas de pago antes del jueves.",
                    ],
                    "日本語" => [
                        "こんにちは。",
                        "新しいバージョンが完成しました。",
                        "木曜日までに支払い画面を確認してください。",
                    ],
                    "Português" => [
                        "Olá, equipe!",
                        "A nova versão está pronta.",
                        "Revisem as telas de pagamento até quinta-feira.",
                    ],
                    _ => [
                        "Hello team!",
                        "The new release is ready.",
                        "Please review the payment screens by Thursday.",
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
                    t("Checkout conversion drop", "结账转化率下降"),
                    names.map(String::from).to_vec(),
                    spans,
                )
            })
            .into()
        }
        "context-window" => {
            use context_window::*;
            let bands = [
                (t("System prompt", "系统指令"), 4100, 0x6383ac),
                (t("Tools", "工具"), 32000, 0x8383b2),
                (t("Files and memory", "文件和记忆"), 100000, 0xb29366),
                (t("Summary", "摘要"), 0, 0x619f88),
                (t("Messages", "消息"), 291000, 0x628995),
            ]
            .into_iter()
            .map(|(n, tokens, c)| Band {
                label: n.into(),
                tokens,
                color: rgb(c).into(),
            })
            .collect();
            let e = cx.new(|_| {
                let mut s =
                    CapacityBands::new(id, t("Assistant · 1M", "助理 · 1M"), 1_000_000, bands);
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
            let e = cx.new(|_| {
                InvocationInspector::new(
                    id,
                    vec![
                        Invocation {
                            name: "GitHub".into(),
                            action: t("Search release blockers", "搜索发布阻塞项").into(),
                            input: "{\"label\": \"blocking-release\"}".into(),
                            output: t("Two open issues", "两个待处理问题").into(),
                            state: ToolState::Done,
                        },
                        Invocation {
                            name: "Linear".into(),
                            action: t("Create a release checklist", "创建发布检查清单").into(),
                            input: "{\"team\": \"platform\"}".into(),
                            output: String::new(),
                            state: ToolState::Approval,
                        },
                    ],
                )
            });
            cx.subscribe(&e, move |_: &mut Demo, e, a: &ToolAction, cx| {
                let i = match a {
                    ToolAction::Approve(i) | ToolAction::Retry(i) => *i,
                    ToolAction::Deny(_) => return,
                };
                later(
                    &e,
                    400,
                    move |s, cx| {
                        s.calls[i].state = ToolState::Done;
                        s.calls[i].output =
                            t("Checklist created · PLAT-104", "已创建检查清单 · PLAT-104").into();
                        cx.notify()
                    },
                    cx,
                )
            })
            .detach();
            let x = e.clone();
            option!(t("Simulate failure", "模拟失败"), move |_, cx| {
                x.update(cx, |s, cx| {
                    s.calls[0].state = ToolState::Failed;
                    s.calls[0].output = t("Connection timed out", "连接超时").into();
                    cx.notify()
                })
            });
            e.into()
        }
        "agent-plan" => {
            use agent_plan::*;
            let steps = [
                t("Read the API routes", "检查 API 路由"),
                t("Choose a rate limit", "确定速率限制"),
                t("Add request counters", "添加请求计数器"),
                t("Return retry headers", "返回重试头"),
                t("Test the limiter", "测试限流器"),
                t("Document limits", "编写限流说明"),
            ]
            .map(String::from)
            .to_vec();
            let e = cx.new(|cx| {
                TaskPlan::new(
                    id,
                    t("Add rate limiting to the public API", "为公开 API 添加限流"),
                    steps,
                    w,
                    cx,
                )
            });
            cx.subscribe(&e, move |d: &mut Demo, e, a: &PlanEvent, cx| match a {
                PlanEvent::Approve => {
                    for i in 0..e.read(cx).steps.len() {
                        later(
                            &e,
                            400 * (i + 1) as u64,
                            move |s, cx| {
                                s.completed = i + 1;
                                cx.notify()
                            },
                            cx,
                        )
                    }
                }
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
            e.into()
        }
        "reasoning-block" => reasoning::demo(id, l, &mut options, w, cx),
        "source-cards" => {
            use source_cards::*;
            let sources = [
                (
                    "docs.rs",
                    "gpui",
                    "https://docs.rs/gpui/",
                    t(
                        "API reference and trait implementations.",
                        "接口参考和 trait 实现。",
                    ),
                ),
                (
                    "rust-lang.org",
                    "The Rust Book",
                    "https://doc.rust-lang.org/book/",
                    t("Ownership and borrowing examples.", "所有权和借用示例。"),
                ),
                (
                    "developer.mozilla.org",
                    "WebAssembly",
                    "https://developer.mozilla.org/en-US/docs/WebAssembly",
                    t("Browser module lifecycle.", "浏览器模块生命周期。"),
                ),
                (
                    "w3.org",
                    "ARIA",
                    "https://www.w3.org/WAI/ARIA/apg/",
                    t("Keyboard interaction patterns.", "键盘交互模式。"),
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
                CitationShelf::new(
                    id,
                    t(
                        "How does a Rust UI run in the browser?",
                        "Rust 界面如何在浏览器中运行？",
                    ),
                    sources.clone(),
                )
            });
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
                            s.sources = data;
                            s.state = SearchState::Ready;
                            cx.notify()
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
            let data = |n: u32| Metrics {
                first_token_ms: 420,
                duration_ms: 420 + n * 12,
                input_tokens: 1187,
                output_tokens: n,
                input_rate: 3.,
                output_rate: 15.,
                running: n < 284,
                samples: (0..24).map(|j| 65. + ((j * 17 + n) % 27) as f32).collect(),
            };
            let e = cx.new(|_| GenerationReadout::new(id, t("Assistant", "助手"), data(284)));
            let x = e.clone();
            let epoch = Rc::new(std::cell::Cell::new(0u64));
            option!(
                t("Replay generation", "重播生成过程"),
                move |_, cx| {
                    let generation = epoch.get() + 1;
                    epoch.set(generation);
                    for i in 0..=10 {
                        let epoch = epoch.clone();
                        later(
                            &x,
                            i * 160,
                            move |s, cx| {
                                if generation == epoch.get() {
                                    s.set_metrics(data((i * 284 / 10) as u32), cx)
                                }
                            },
                            cx,
                        )
                    }
                }
            );
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
            use code_suggestion::*;
            let hunks = || {
                vec![
                    Hunk {
                        line: 4,
                        before: vec!["let response = send(request).await?;".into()],
                        after: vec![
                            "let response = retry(3, || send(request.clone())).await?;".into(),
                        ],
                        decision: Decision::Pending,
                    },
                    Hunk {
                        line: 9,
                        before: vec!["return response.body();".into()],
                        after: vec!["response.ensure_success()?.body()".into()],
                        decision: Decision::Pending,
                    },
                ]
            };
            let e = cx.new(|cx| {
                let mut s = PatchReview::new(id, "invoice.rs", hunks(), cx);
                s.summary = t("Add retry with backoff", "添加退避重试").into();
                s
            });
            for (mode, label) in [
                (0, t("Suggestion", "建议")),
                (1, t("No changes", "无改动")),
                (2, t("Error", "错误")),
            ] {
                let x = e.clone();
                option!(label, move |_, cx| x.update(cx, |s, cx| {
                    s.hunks = if mode == 0 { hunks() } else { vec![] };
                    s.summary = match mode {
                        1 => t("No changes needed", "无需修改"),
                        2 => t(
                            "Request failed. Select Suggestion to retry.",
                            "请求失败，选择「建议」可重试。",
                        ),
                        _ => t("Add retry with backoff", "添加退避重试"),
                    }
                    .into();
                    cx.notify()
                }))
            }
            e.into()
        }
        "model-card" => {
            use model_card::*;
            let models = [
                (
                    t("Long context", "长上下文配置"),
                    t("Sample profile", "示例配置"),
                    1_000_000,
                    128000,
                    5.,
                    25.,
                    0.91,
                ),
                (
                    t("Low latency", "低延迟配置"),
                    t("Sample profile", "示例配置"),
                    256000,
                    64000,
                    1.5,
                    6.,
                    0.84,
                ),
                (
                    t("Compact", "轻量配置"),
                    t("Sample profile", "示例配置"),
                    128000,
                    32000,
                    0.4,
                    1.6,
                    0.73,
                ),
            ]
            .into_iter()
            .map(
                |(n, p, context, max_output, input_rate, output_rate, b)| Model {
                    name: n.into(),
                    provider: p.into(),
                    description: t(
                        "A sample model profile for workspace reasoning and document tasks.",
                        "工作区推理与文档任务的样例模型档案。",
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
            let e = cx.new(|cx| CommandDesk::new(id, commands, w, cx));
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
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
