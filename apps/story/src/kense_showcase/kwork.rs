//! Authentic interactive KWork (`Kwrok`) Desktop AI Workbench (`kwrok-workbench.png`).

use super::{is_dark, is_zh, tr};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RightTab {
    Diff,
    OfficeSheet,
    Terminal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum DiffSubTab {
    Changes,
    History,
    PullRequest,
    Issues,
}

#[derive(Clone)]
pub(super) struct FollowUpTurn {
    prompt: SharedString,
    agent: &'static str,
    reply: SharedString,
}

pub(super) struct KWorkApp {
    pub(super) chat_idx: usize,
    pub(super) right_open: bool,
    pub(super) right_tab: RightTab,
    pub(super) diff_sub: DiffSubTab,
    pub(super) diff_collapsed: bool,
    pub(super) hunk_staged: bool,
    pub(super) runtime_idx: usize,
    pub(super) preset_idx: usize,
    pub(super) whisper_active: bool,
    pub(super) draft_override: Option<SharedString>,
    pub(super) inbox_banner: bool,
    pub(super) follow_ups: [Vec<FollowUpTurn>; 3],
    pub(super) selected_row: usize,
    pub(super) sheet_ai_applied: bool,
    pub(super) term_logs: Vec<SharedString>,
}

const RUNTIMES: [&str; 3] = ["Codex", "Claude Code", "DeepSeek"];

struct PresetPrompt {
    label_zh: &'static str,
    label_en: &'static str,
    prompt_zh: &'static str,
    prompt_en: &'static str,
    reply_zh: &'static str,
    reply_en: &'static str,
}

const PRESETS: [PresetPrompt; 3] = [
    PresetPrompt {
        label_zh: "运行回归测试",
        label_en: "Run cargo test",
        prompt_zh: "运行 reading_time 单元与边界回归测试，确认 0 词与 221 词全部通过。",
        prompt_en: "Run the reading_time unit and boundary tests to confirm 0 and 221 words pass.",
        reply_zh: "已在工作区执行 `cargo test reading_time`：4 项断言全部通过（耗时 0.18s），无需额外修改。",
        reply_en: "Ran `cargo test reading_time` in workspace: all 4 assertions passed in 0.18s with zero warnings.",
    },
    PresetPrompt {
        label_zh: "支持中文字数统计",
        label_en: "Support CJK count",
        prompt_zh: "如果笔记包含中文字符，按每分钟 300 字与英文 220 词混合向上取整。",
        prompt_en: "If a field note mixes CJK characters, blend 300 CJK chars/min with 220 Latin words/min.",
        reply_zh: "已在 `src/reading_time.rs` 补充 CJK 字符分支：`cjk.div_ceil(300) + words.div_ceil(220)`，最短仍保持 1 分钟。",
        reply_en: "Added CJK character weighting in `src/reading_time.rs`: blends `cjk / 300` + `words / 220` with `.max(1)` floor.",
    },
    PresetPrompt {
        label_zh: "同步到 Notes.xlsx",
        label_en: "Sync Notes.xlsx",
        prompt_zh: "把最新阅读时长公式同步进右侧 Kantor 表格 Notes.xlsx 的 D 列。",
        prompt_en: "Sync the updated reading-time formula into column D of Notes.xlsx in Kantor Office.",
        reply_zh: "已通过 Kantor 协同引擎更新 `Notes.xlsx` D2:D6 公式为 `=MAX(1, CEILING(C2/220, 1))`，可在右侧表格查看。",
        reply_en: "Updated `Notes.xlsx` D2:D6 to `=MAX(1, CEILING(C2/220, 1))` via Kantor Office engine.",
    },
];

struct SheetRow {
    id: &'static str,
    title_zh: &'static str,
    title_en: &'static str,
    words: u32,
    old_min: u32,
    new_min: u32,
}

const SHEET_ROWS: [SheetRow; 5] = [
    SheetRow {
        id: "FN-01",
        title_zh: "山谷气象站速记",
        title_en: "Glacier station quick note",
        words: 0,
        old_min: 0,
        new_min: 1,
    },
    SheetRow {
        id: "FN-02",
        title_zh: "高原站点采样记录",
        title_en: "Alpine tundra sample log",
        words: 148,
        old_min: 0,
        new_min: 1,
    },
    SheetRow {
        id: "FN-03",
        title_zh: "气压计漂移校准",
        title_en: "Barometer drift check",
        words: 220,
        old_min: 1,
        new_min: 1,
    },
    SheetRow {
        id: "FN-04",
        title_zh: "河谷水文周报",
        title_en: "Valley hydrology digest",
        words: 221,
        old_min: 1,
        new_min: 2,
    },
    SheetRow {
        id: "FN-05",
        title_zh: "年度设备巡检清单",
        title_en: "Winter rig inspection",
        words: 690,
        old_min: 3,
        new_min: 4,
    },
];

impl KWorkApp {
    pub(super) fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            chat_idx: 0,
            right_open: true,
            right_tab: RightTab::Diff,
            diff_sub: DiffSubTab::Changes,
            diff_collapsed: false,
            hunk_staged: false,
            runtime_idx: 0,
            preset_idx: 0,
            whisper_active: false,
            draft_override: None,
            inbox_banner: false,
            follow_ups: [Vec::new(), Vec::new(), Vec::new()],
            selected_row: 0,
            sheet_ai_applied: true,
            term_logs: vec![
                "$ cargo test -p field_notes reading_time".into(),
                "running 2 tests".into(),
                "test reading_time::tests::counts_partial_minutes ... ok".into(),
                "test reading_time::tests::empty_note_is_one_minute ... ok".into(),
                "test result: ok. 2 passed; 0 failed; finished in 0.14s".into(),
            ],
        }
    }

    fn current_draft(&self, zh: bool) -> SharedString {
        if let Some(custom) = &self.draft_override {
            return custom.clone();
        }
        let p = &PRESETS[self.preset_idx];
        tr(zh, p.prompt_zh, p.prompt_en).into()
    }

    fn send_current(&mut self, zh: bool) {
        let prompt = self.current_draft(zh);
        let agent = RUNTIMES[self.runtime_idx];
        let reply: SharedString = if self.whisper_active {
            self.whisper_active = false;
            tr(
                zh,
                "已根据离线 Whisper 语音指令复核 `src/reading_time.rs` 并在本地沙箱跑通测试，PR 摘要已就绪。",
                "Processed offline Whisper voice note: verified `src/reading_time.rs` and prepared PR summary.",
            )
            .into()
        } else if self.draft_override.is_some() {
            tr(
                zh,
                "已完成 `src/reading_time.rs` (+13 -1) 变更审查：`div_ceil(220).max(1)` 无整数越界风险，建议直接合并。",
                "Reviewed `src/reading_time.rs` (+13 -1): `div_ceil(220).max(1)` is overflow-safe and covered by unit tests.",
            )
            .into()
        } else {
            let p = &PRESETS[self.preset_idx];
            if self.preset_idx == 2 {
                self.right_open = true;
                self.right_tab = RightTab::OfficeSheet;
                self.sheet_ai_applied = true;
            }
            tr(zh, p.reply_zh, p.reply_en).into()
        };
        self.draft_override = None;
        self.preset_idx = (self.preset_idx + 1) % PRESETS.len();
        self.follow_ups[self.chat_idx].push(FollowUpTurn {
            prompt,
            agent,
            reply,
        });
    }

    fn render_sidebar(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let side_bg = if dark { rgb(0x1c1c1f) } else { rgb(0xf3f4f6) };
        let border_c = if dark { rgb(0x28292e) } else { rgb(0xe2e5ec) };
        let active_bg = if dark { rgb(0x2b2c31) } else { rgb(0xe2e6ee) };
        let ink = if dark { rgb(0xf3f4f6) } else { rgb(0x111827) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x4b5563) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x9ca3af) };

        let nav_row = |id: &'static str,
                       icon: &'static str,
                       label: &'static str,
                       active: bool,
                       badge: Option<&'static str>| {
            div()
                .id(id)
                .cursor_pointer()
                .px(px(8.))
                .py(px(5.))
                .rounded(px(7.))
                .bg(if active { active_bg } else { rgba(0x00000000) })
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(11.5))
                .text_color(if active { ink } else { sub })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .w(px(14.))
                                .font_family("Fira Code")
                                .text_size(px(10.5))
                                .text_color(if active { ink } else { muted })
                                .child(icon),
                        )
                        .child(label),
                )
                .when_some(badge, |d, b| {
                    d.child(
                        div()
                            .px(px(5.))
                            .py(px(1.))
                            .rounded_full()
                            .bg(if dark { rgb(0x2d3240) } else { rgb(0xdbeafe) })
                            .text_size(px(9.5))
                            .text_color(if dark { rgb(0x93c5fd) } else { rgb(0x1d4ed8) })
                            .child(b),
                    )
                })
        };

        let chats: [(&str, &str, Option<&str>); 3] = [
            (tr(zh, "Reading time", "Reading time"), "●", None),
            (tr(zh, "Field guide", "Field guide"), "•", Some("27m")),
            (tr(zh, "Release notes", "Release notes"), "•", Some("1h")),
        ];

        let mut chat_list = div().flex().flex_col().gap(px(2.));
        for (idx, (title, _dot, age)) in chats.iter().enumerate() {
            let active = self.chat_idx == idx;
            chat_list = chat_list.child(
                div()
                    .id(("kw-chat-item", idx))
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(7.))
                    .bg(if active { active_bg } else { rgba(0x00000000) })
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.5))
                    .text_color(if active { ink } else { sub })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.chat_idx = idx;
                        if idx == 1 {
                            v.right_open = true;
                            v.right_tab = RightTab::OfficeSheet;
                        } else if idx == 0 {
                            v.right_open = true;
                            v.right_tab = RightTab::Diff;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(
                                div()
                                    .w(px(14.))
                                    .h(px(14.))
                                    .rounded_full()
                                    .bg(if active {
                                        rgb(0x2f5bea)
                                    } else if dark {
                                        rgb(0x2a2b32)
                                    } else {
                                        rgb(0xd1d5db)
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .font_family("Fira Code")
                                    .text_size(px(8.5))
                                    .text_color(rgb(0xffffff))
                                    .child(">_"),
                            )
                            .child(*title),
                    )
                    .child(if active {
                        div()
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(ink)
                            .into_any_element()
                    } else if let Some(t) = age {
                        div()
                            .text_size(px(10.))
                            .text_color(muted)
                            .child(*t)
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }

        div()
            .w(px(176.))
            .flex_none()
            .h_full()
            .bg(side_bg)
            .border_r_1()
            .border_color(border_c)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        // Window / navigation header
                        div()
                            .h(px(36.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(super::traffic_lights())
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .font_family("Fira Code")
                                    .text_size(px(10.5))
                                    .text_color(muted)
                                    .child("<-")
                                    .child("->"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(4.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                nav_row(
                                    "kw-nav-search",
                                    "?",
                                    tr(zh, "搜索", "Search"),
                                    false,
                                    None,
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.inbox_banner = false;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                nav_row(
                                    "kw-nav-new",
                                    "+",
                                    tr(zh, "新建对话", "New chat"),
                                    false,
                                    None,
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.follow_ups[v.chat_idx].clear();
                                        v.draft_override = None;
                                        v.inbox_banner = false;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                nav_row(
                                    "kw-nav-inbox",
                                    "[]",
                                    tr(zh, "收件箱", "Inbox"),
                                    self.inbox_banner,
                                    Some("2"),
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.inbox_banner = !v.inbox_banner;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                nav_row(
                                    "kw-nav-docs",
                                    "=",
                                    tr(zh, "文档 · Kantor", "Documents"),
                                    self.right_open && self.right_tab == RightTab::OfficeSheet,
                                    None,
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.right_open = true;
                                        v.right_tab = RightTab::OfficeSheet;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                nav_row(
                                    "kw-nav-git",
                                    "+-",
                                    tr(zh, "Git 变更", "Git changes"),
                                    self.right_open && self.right_tab == RightTab::Diff,
                                    Some("1"),
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.right_open = true;
                                        v.right_tab = RightTab::Diff;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                nav_row(
                                    "kw-nav-term",
                                    ">_",
                                    tr(zh, "工作区终端", "Terminal"),
                                    self.right_open && self.right_tab == RightTab::Terminal,
                                    None,
                                )
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        v.right_open = true;
                                        v.right_tab = RightTab::Terminal;
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(div().mx(px(10.)).my(px(6.)).h(px(1.)).bg(border_c))
                    .child(
                        div()
                            .px(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .px(px(6.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(tr(zh, "对话", "Chats"))
                                    .child(
                                        div()
                                            .font_family("Fira Code")
                                            .text_size(px(9.5))
                                            .child("Field Notes"),
                                    ),
                            )
                            .child(chat_list),
                    ),
            )
            .child(
                div()
                    .id("kw-settings-btn")
                    .cursor_pointer()
                    .h(px(36.))
                    .px(px(14.))
                    .border_t_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.5))
                    .text_color(sub)
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.runtime_idx = (v.runtime_idx + 1) % RUNTIMES.len();
                        cx.notify();
                    }))
                    .child(tr(zh, "设置", "Settings"))
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(9.5))
                            .text_color(muted)
                            .child(RUNTIMES[self.runtime_idx]),
                    ),
            )
    }

    fn render_chat_stage(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let stage_bg = if dark { rgb(0x0f0f11) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x232429) } else { rgb(0xe5e7eb) };
        let bubble_bg = if dark { rgb(0x1e1f24) } else { rgb(0xf1f5f9) };
        let bubble_bd = if dark { rgb(0x2d2e36) } else { rgb(0xdbe2ea) };
        let code_bg = if dark { rgb(0x1f2229) } else { rgb(0xeef2f7) };
        let composer_bg = if dark { rgb(0x16171b) } else { rgb(0xf8fafc) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let sub = if dark { rgb(0xb4b6be) } else { rgb(0x334155) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x64748b) };

        let chat_title = match self.chat_idx {
            0 => "Reading time",
            1 => "Field guide",
            _ => "Release notes",
        };

        let user_prompt = match self.chat_idx {
            0 => tr(
                zh,
                "把阅读时长向上取整到下一分钟。哪怕最短的现场笔记，也至少显示 1 分钟阅读。",
                "Round reading time up to the next minute. Even the shortest field note should show a one-minute read.",
            ),
            1 => tr(
                zh,
                "把现场笔记字数与阅读时长同步进 Notes.xlsx，并检查 0 词笔记的显示。",
                "Sync the field-note word counts into Notes.xlsx and verify zero-word notes show 1 min.",
            ),
            _ => tr(
                zh,
                "根据 main 分支最近提交生成 KWork v0.4 桌面工作台发布说明。",
                "Draft the KWork v0.4 desktop workbench release notes from recent commits on main.",
            ),
        };

        let mut thread = div()
            .flex_1()
            .min_h_0()
            .id("kw-chat-scroll")
            .overflow_y_scroll()
            .px(px(22.))
            .py(px(14.))
            .flex()
            .flex_col()
            .gap(px(14.));

        if self.inbox_banner {
            thread = thread.child(
                div()
                    .p(px(10.))
                    .rounded(px(9.))
                    .bg(if dark { rgb(0x151924) } else { rgb(0xeff6ff) })
                    .border_1()
                    .border_color(if dark { rgb(0x263554) } else { rgb(0xbfdbfe) })
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .child(div().text_color(ink).child(tr(
                        zh,
                        "收件箱 (2)：#48 Reading time 补丁待审查 · Notes.xlsx 公式已同步",
                        "Inbox (2): PR #48 Reading time ready for review · Notes.xlsx synced",
                    )))
                    .child(
                        div()
                            .id("kw-inbox-open-pr")
                            .cursor_pointer()
                            .px(px(7.))
                            .py(px(2.))
                            .rounded(px(5.))
                            .bg(rgb(0x2f5bea))
                            .text_color(rgb(0xffffff))
                            .text_size(px(10.))
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.right_open = true;
                                v.right_tab = RightTab::Diff;
                                v.diff_sub = DiffSubTab::PullRequest;
                                v.inbox_banner = false;
                                cx.notify();
                            }))
                            .child(tr(zh, "查看 PR", "Open PR")),
                    ),
            );
        }

        // Right-aligned user bubble (exact kwrok-workbench.png!)
        thread = thread.child(
            div().flex().justify_end().child(
                div()
                    .max_w(px(340.))
                    .px(px(13.))
                    .py(px(9.))
                    .rounded(px(12.))
                    .bg(bubble_bg)
                    .border_1()
                    .border_color(bubble_bd)
                    .text_size(px(12.))
                    .line_height(rems(1.42))
                    .text_color(ink)
                    .child(user_prompt),
            ),
        );

        // Agent primary turn
        let agent_turn = if self.chat_idx == 0 {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(muted)
                        .child(tr(zh, "用时 12 秒", "Took 12s")),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(12.))
                        .line_height(rems(1.45))
                        .text_color(ink)
                        .child(tr(
                            zh,
                            "已更新阅读时长估算逻辑：",
                            "Updated the reading-time estimate in",
                        ))
                        .child(
                            div()
                                .id("kw-pill-file")
                                .cursor_pointer()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(4.))
                                .bg(code_bg)
                                .border_1()
                                .border_color(border_c)
                                .font_family("Fira Code")
                                .text_size(px(11.))
                                .text_color(if dark { rgb(0x93c5fd) } else { rgb(0x1d4ed8) })
                                .on_click(cx.listener(|v, _, _, cx| {
                                    v.right_open = true;
                                    v.right_tab = RightTab::Diff;
                                    v.diff_sub = DiffSubTab::Changes;
                                    v.diff_collapsed = false;
                                    cx.notify();
                                }))
                                .child("reading_time.rs"),
                        )
                        .child(tr(
                            zh,
                            "— 短笔记固定至少 1 分钟，长文按 220 词/分钟向上取整。",
                            ". Short notes stay at one minute, and longer notes round up at 220 words/min.",
                        )),
                )
                .child(
                    div()
                        .pt(px(2.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink)
                        .child(tr(zh, "边界条件验证", "Boundary checks")),
                )
                .child(
                    div()
                        .pl(px(8.))
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .font_family("Fira Code")
                        .text_size(px(11.))
                        .text_color(sub)
                        .child(tr(zh, "• 空笔记 -> 1 min", "• Empty note -> 1 min"))
                        .child(tr(zh, "• 220 词 -> 1 min", "• 220 words -> 1 min"))
                        .child(tr(zh, "• 221 词 -> 2 min", "• 221 words -> 2 min")),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(sub)
                        .child(tr(
                            zh,
                            "新增回归测试已全部通过。",
                            "The new regression test passes.",
                        )),
                )
        } else if self.chat_idx == 1 {
            div()
                .flex()
                .flex_col()
                .gap(px(7.))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(muted)
                        .child(tr(zh, "用时 9 秒 · Kantor Office 协同", "Took 9s · Kantor Office")),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(rems(1.45))
                        .text_color(ink)
                        .child(tr(
                            zh,
                            "已把 5 条现场采样笔记写入右侧 Notes.xlsx，并将 0 词与 148 词行的阅读时长修正为 1 min。",
                            "Synced 5 field notes into Notes.xlsx on the right; 0-word and 148-word rows now evaluate to 1 min.",
                        )),
                )
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(7.))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(muted)
                        .child(tr(zh, "用时 7 秒 · Git 历史汇总", "Took 7s · Git changelog")),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(rems(1.45))
                        .text_color(ink)
                        .child(tr(
                            zh,
                            "已整理 v0.4 变更摘要：统一多智能体会话、右侧 Git Diff 逐行审查、Kantor 原生表格与 Whisper 离线听写。",
                            "Prepared v0.4 changelog: unified multi-agent threads, inline Git Diff review, Kantor spreadsheets, and offline Whisper dictation.",
                        )),
                )
        };
        thread = thread.child(agent_turn);

        // Appended live follow-up turns!
        for (f_idx, turn) in self.follow_ups[self.chat_idx].iter().enumerate() {
            thread = thread
                .child(
                    div().flex().justify_end().child(
                        div()
                            .id(("kw-fup-q", f_idx))
                            .max_w(px(340.))
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(12.))
                            .bg(bubble_bg)
                            .border_1()
                            .border_color(bubble_bd)
                            .text_size(px(11.5))
                            .text_color(ink)
                            .child(turn.prompt.clone()),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(div().text_size(px(10.)).text_color(muted).child(format!(
                            "{} · {}",
                            turn.agent,
                            tr(zh, "刚刚", "just now")
                        )))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .line_height(rems(1.42))
                                .text_color(ink)
                                .child(turn.reply.clone()),
                        ),
                );
        }

        // Quick prompt selector pills
        let mut quick_row = div().flex().flex_wrap().items_center().gap(px(5.));
        for (p_idx, preset) in PRESETS.iter().enumerate() {
            let active = self.draft_override.is_none() && self.preset_idx == p_idx;
            quick_row = quick_row.child(
                div()
                    .id(("kw-preset-chip", p_idx))
                    .cursor_pointer()
                    .px(px(7.))
                    .py(px(2.))
                    .rounded_full()
                    .bg(if active {
                        if dark { rgb(0x252834) } else { rgb(0xe0e7ff) }
                    } else {
                        rgba(0x00000000)
                    })
                    .border_1()
                    .border_color(if active { rgb(0x2f5bea) } else { border_c })
                    .text_size(px(10.))
                    .text_color(if active { ink } else { muted })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.preset_idx = p_idx;
                        v.draft_override = None;
                        v.whisper_active = false;
                        cx.notify();
                    }))
                    .child(tr(zh, preset.label_zh, preset.label_en)),
            );
        }

        let draft_text = self.current_draft(zh);

        div()
            .flex_1()
            .min_w(px(260.))
            .h_full()
            .bg(stage_bg)
            .flex()
            .flex_col()
            .justify_between()
            // Top breadcrumb bar
            .child(
                div()
                    .h(px(36.))
                    .px(px(16.))
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .child(div().text_color(muted).child("Field Notes"))
                            .child(div().text_color(muted).child("/"))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(ink)
                                    .child(chat_title),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .id("kw-toggle-inspector")
                                    .cursor_pointer()
                                    .px(px(7.))
                                    .py(px(2.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(border_c)
                                    .font_family("Fira Code")
                                    .text_size(px(10.))
                                    .text_color(sub)
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.right_open = !v.right_open;
                                        cx.notify();
                                    }))
                                    .child(if self.right_open {
                                        tr(zh, "[|] 收起分栏", "[|] Hide pane")
                                    } else {
                                        tr(zh, "[|] 展开分栏", "[|] Split pane")
                                    }),
                            ),
                    ),
            )
            // Conversation thread
            .child(thread)
            // Bottom floating rounded-2xl composer (exact kwrok-workbench.png!)
            .child(
                div()
                    .px(px(16.))
                    .pb(px(12.))
                    .pt(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(quick_row)
                    .child(
                        div()
                            .p(px(11.))
                            .rounded(px(14.))
                            .bg(composer_bg)
                            .border_1()
                            .border_color(if self.whisper_active {
                                rgb(0x2f5bea)
                            } else {
                                bubble_bd
                            })
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .line_height(rems(1.38))
                                    .text_color(ink)
                                    .child(draft_text),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .id("kw-whisper-btn")
                                                    .cursor_pointer()
                                                    .px(px(7.))
                                                    .py(px(2.))
                                                    .rounded(px(6.))
                                                    .bg(if self.whisper_active {
                                                        rgb(0x2f5bea)
                                                    } else if dark {
                                                        rgb(0x22232a)
                                                    } else {
                                                        rgb(0xe2e8f0)
                                                    })
                                                    .text_size(px(10.))
                                                    .text_color(if self.whisper_active {
                                                        rgb(0xffffff)
                                                    } else {
                                                        sub
                                                    })
                                                    .on_click(cx.listener(move |v, _, _, cx| {
                                                        v.whisper_active = !v.whisper_active;
                                                        if v.whisper_active {
                                                            v.draft_override = Some(
                                                                tr(
                                                                    zh,
                                                                    "[Whisper 离线听写] 帮我检查 reading_time.rs 改动并生成 PR 描述。",
                                                                    "[Offline Whisper] Double-check reading_time.rs and summarize the PR.",
                                                                )
                                                                .into(),
                                                            );
                                                        } else {
                                                            v.draft_override = None;
                                                        }
                                                        cx.notify();
                                                    }))
                                                    .child(if self.whisper_active {
                                                        tr(zh, "● 听写中", "● Dictating")
                                                    } else {
                                                        tr(zh, "Whisper 听写", "Whisper")
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.5))
                                                    .text_color(muted)
                                                    .child("Context · reading_time.rs"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(
                                                div()
                                                    .id("kw-runtime-pill")
                                                    .cursor_pointer()
                                                    .px(px(7.))
                                                    .py(px(2.))
                                                    .rounded_full()
                                                    .bg(if dark {
                                                        rgb(0x23242b)
                                                    } else {
                                                        rgb(0xe2e8f0)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(5.))
                                                    .text_size(px(10.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(ink)
                                                    .on_click(cx.listener(|v, _, _, cx| {
                                                        v.runtime_idx =
                                                            (v.runtime_idx + 1) % RUNTIMES.len();
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .w(px(6.))
                                                            .h(px(6.))
                                                            .rounded_full()
                                                            .bg(rgb(0x22c55e)),
                                                    )
                                                    .child(RUNTIMES[self.runtime_idx]),
                                            )
                                            .child(
                                                div()
                                                    .id("kw-send-btn")
                                                    .debug_selector(|| "kw-send-btn".into())
                                                    .cursor_pointer()
                                                    .w(px(24.))
                                                    .h(px(24.))
                                                    .rounded_full()
                                                    .bg(rgb(0x2f5bea))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .font_family("Fira Code")
                                                    .text_size(px(11.))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0xffffff))
                                                    .on_click(cx.listener(move |v, _, _, cx| {
                                                        v.send_current(zh);
                                                        cx.notify();
                                                    }))
                                                    .child("↑"),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn render_right_inspector(
        &self,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pane_bg = if dark { rgb(0x0b0b0d) } else { rgb(0xf8fafc) };
        let card_bg = if dark { rgb(0x121317) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x22242a) } else { rgb(0xe2e8f0) };
        let tab_on = if dark { rgb(0x1a1b20) } else { rgb(0xffffff) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x475569) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x94a3b8) };

        let body: AnyElement = match self.right_tab {
            RightTab::Diff => self.render_diff_pane(zh, dark, cx).into_any_element(),
            RightTab::OfficeSheet => self.render_office_pane(zh, dark, cx).into_any_element(),
            RightTab::Terminal => self.render_terminal_pane(zh, dark, cx).into_any_element(),
        };

        let top_tab = |id: &'static str, tab: RightTab, label: &'static str| {
            let active = self.right_tab == tab;
            div()
                .id(id)
                .debug_selector(move || id.into())
                .cursor_pointer()
                .px(px(9.))
                .py(px(4.))
                .rounded(px(6.))
                .bg(if active { tab_on } else { rgba(0x00000000) })
                .border_1()
                .border_color(if active { border_c } else { rgba(0x00000000) })
                .text_size(px(11.))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active { ink } else { muted })
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.right_tab = tab;
                    cx.notify();
                }))
                .child(label)
        };

        div()
            .w(px(348.))
            .flex_none()
            .h_full()
            .bg(pane_bg)
            .border_l_1()
            .border_color(border_c)
            .flex()
            .flex_col()
            .child(
                // Inspector top tab strip
                div()
                    .h(px(36.))
                    .px(px(10.))
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(top_tab("kw-rtab-diff", RightTab::Diff, "+- Diff"))
                            .child(top_tab(
                                "kw-rtab-sheet",
                                RightTab::OfficeSheet,
                                "Notes.xlsx",
                            ))
                            .child(top_tab(
                                "kw-rtab-term",
                                RightTab::Terminal,
                                tr(zh, ">_ 终端", ">_ Term"),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_family("Fira Code")
                            .text_color(sub)
                            .child(if self.hunk_staged { "staged" } else { "git" }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .id("kw-right-scroll")
                    .overflow_y_scroll()
                    .p(px(10.))
                    .bg(card_bg.opacity(if dark { 0.0 } else { 0.4 }))
                    .child(body),
            )
    }

    fn render_diff_pane(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let card_bg = if dark { rgb(0x111216) } else { rgb(0xffffff) };
        let capsule_bg = if dark { rgb(0x16171c) } else { rgb(0xeef2f6) };
        let pill_on = if dark { rgb(0x262830) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x23252c) } else { rgb(0xdbe2ea) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x475569) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x94a3b8) };

        let sub_btn = |id: &'static str, tab: DiffSubTab, label: &'static str| {
            let active = self.diff_sub == tab;
            div()
                .id(id)
                .cursor_pointer()
                .px(px(8.))
                .py(px(3.))
                .rounded(px(6.))
                .bg(if active { pill_on } else { rgba(0x00000000) })
                .text_size(px(10.5))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active { ink } else { muted })
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.diff_sub = tab;
                    cx.notify();
                }))
                .child(label)
        };

        let diff_lines: [(&str, &str, &str, i32); 14] = [
            ("5", "-", "    words / WORDS_PER_MINUTE", -1),
            ("5", "+", "    words.div_ceil(WORDS_PER_MINUTE).max(1)", 1),
            ("6", "+", "}", 1),
            ("7", "+", "", 1),
            ("8", "+", "#[cfg(test)]", 1),
            ("9", "+", "mod tests {", 1),
            ("10", "+", "    use super::minutes;", 1),
            ("11", "+", "", 1),
            ("12", "+", "    #[test]", 1),
            ("13", "+", "    fn counts_partial_minutes() {", 1),
            ("14", "+", "        assert_eq!(minutes(0), 1);", 1),
            ("15", "+", "        assert_eq!(minutes(220), 1);", 1),
            ("16", "+", "        assert_eq!(minutes(221), 2);", 1),
            ("17", "+", "    }", 1),
        ];

        let mut code_box = div()
            .rounded(px(7.))
            .border_1()
            .border_color(border_c)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(9.))
                    .py(px(5.))
                    .bg(if dark { rgb(0x16181e) } else { rgb(0xf1f5f9) })
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .font_family("Fira Code")
                    .text_size(px(10.5))
                    .child(div().text_color(ink).child("src/reading_time.rs"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(div().text_color(rgb(0x22c55e)).child("+13"))
                            .child(div().text_color(rgb(0xef4444)).child("-1")),
                    ),
            );

        for (ln, sign, code, kind) in diff_lines {
            let row_bg = if kind < 0 {
                if dark {
                    rgba(0xef444424)
                } else {
                    rgba(0xef44441c)
                }
            } else if dark {
                rgba(0x22c55e1e)
            } else {
                rgba(0x22c55e18)
            };
            let sign_c = if kind < 0 {
                rgb(0xef4444)
            } else {
                rgb(0x22c55e)
            };
            code_box = code_box.child(
                div()
                    .px(px(8.))
                    .py(px(1.5))
                    .bg(row_bg)
                    .flex()
                    .items_center()
                    .font_family("Fira Code")
                    .text_size(px(10.))
                    .child(div().w(px(20.)).flex_none().text_color(muted).child(ln))
                    .child(div().w(px(12.)).flex_none().text_color(sign_c).child(sign))
                    .child(div().text_color(ink).child(code)),
            );
        }

        let sub_content: AnyElement = match self.diff_sub {
            DiffSubTab::Changes => div()
                .p(px(8.))
                .rounded(px(9.))
                .bg(card_bg)
                .border_1()
                .border_color(border_c)
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .id("kw-diff-file-hdr")
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_between()
                        .on_click(cx.listener(|v, _, _, cx| {
                            v.diff_collapsed = !v.diff_collapsed;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(11.))
                                .child(
                                    div()
                                        .font_family("Fira Code")
                                        .text_color(muted)
                                        .child(if self.diff_collapsed { ">" } else { "v" }),
                                )
                                .child(
                                    div()
                                        .px(px(4.))
                                        .rounded(px(3.))
                                        .bg(rgba(0xf59e0b26))
                                        .font_family("Fira Code")
                                        .text_size(px(9.5))
                                        .text_color(rgb(0xf59e0b))
                                        .child("~"),
                                )
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(ink)
                                        .child("reading_time.rs"),
                                )
                                .child(div().text_color(muted).child("src")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .font_family("Fira Code")
                                .text_size(px(10.))
                                .child(div().text_color(rgb(0x22c55e)).child("+13"))
                                .child(div().text_color(rgb(0xef4444)).child("-1")),
                        ),
                )
                .when(!self.diff_collapsed, |d| d.child(code_box))
                .into_any_element(),
            DiffSubTab::History => div()
                .p(px(10.))
                .rounded(px(9.))
                .bg(card_bg)
                .border_1()
                .border_color(border_c)
                .flex()
                .flex_col()
                .gap(px(7.))
                .font_family("Fira Code")
                .text_size(px(10.5))
                .child(
                    div()
                        .text_color(ink)
                        .child("a84d2c1  fix(notes): ceiling reading_time floor 1m"),
                )
                .child(
                    div()
                        .text_color(sub)
                        .child("593af07  feat(kantor): live xlsx formula sync"),
                )
                .child(
                    div()
                        .text_color(sub)
                        .child("11c08e4  feat(voice): offline whisper.cpp ptt"),
                )
                .into_any_element(),
            DiffSubTab::PullRequest => div()
                .p(px(10.))
                .rounded(px(9.))
                .bg(card_bg)
                .border_1()
                .border_color(border_c)
                .flex()
                .flex_col()
                .gap(px(6.))
                .text_size(px(11.))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink)
                        .child("PR #48 · Round reading time up to 1 min"),
                )
                .child(div().text_color(rgb(0x22c55e)).child(tr(
                    zh,
                    "✓ CI 2/2 通过 · 可直接合并到 main",
                    "✓ CI 2/2 green · Ready to merge into main",
                )))
                .into_any_element(),
            DiffSubTab::Issues => div()
                .p(px(10.))
                .rounded(px(9.))
                .bg(card_bg)
                .border_1()
                .border_color(border_c)
                .flex()
                .flex_col()
                .gap(px(6.))
                .text_size(px(11.))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink)
                        .child("#19 Short field notes display 0 min read"),
                )
                .child(div().text_color(sub).child(tr(
                    zh,
                    "关联当前分支修复，合并后自动关闭。",
                    "Linked to current diff; closes automatically on merge.",
                )))
                .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            // Changes / History / PR / Issues + main branch row
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .p(px(2.))
                            .rounded(px(7.))
                            .bg(capsule_bg)
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(sub_btn("kw-dsub-chg", DiffSubTab::Changes, "Changes 1"))
                            .child(sub_btn("kw-dsub-his", DiffSubTab::History, "History"))
                            .child(sub_btn("kw-dsub-pr", DiffSubTab::PullRequest, "PR"))
                            .child(sub_btn("kw-dsub-iss", DiffSubTab::Issues, "Issues")),
                    )
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(10.5))
                            .text_color(sub)
                            .child("main v"),
                    ),
            )
            // Stage & Draft review toolbar (exact kwrok-workbench.png!)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("kw-stage-btn")
                                    .cursor_pointer()
                                    .px(px(7.))
                                    .py(px(2.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(if self.hunk_staged {
                                        rgb(0x22c55e)
                                    } else {
                                        border_c
                                    })
                                    .text_size(px(10.5))
                                    .text_color(if self.hunk_staged {
                                        rgb(0x22c55e)
                                    } else {
                                        sub
                                    })
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.hunk_staged = !v.hunk_staged;
                                        cx.notify();
                                    }))
                                    .child(if self.hunk_staged {
                                        tr(zh, "✓ 已暂存", "✓ Staged")
                                    } else {
                                        tr(zh, "✓ 暂存补丁", "✓ Stage")
                                    }),
                            )
                            .child(
                                div()
                                    .id("kw-draft-review")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(5.))
                                    .bg(if dark { rgb(0x1b1d24) } else { rgb(0xe2e8f0) })
                                    .text_size(px(10.5))
                                    .text_color(ink)
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        v.draft_override = Some(
                                            tr(
                                                zh,
                                                "请审查 src/reading_time.rs (+13 -1) 的边界测试与整数取整逻辑。",
                                                "Draft a concise code review for src/reading_time.rs (+13 -1).",
                                            )
                                            .into(),
                                        );
                                        cx.notify();
                                    }))
                                    .child(tr(zh, "Draft review · 起草审查", "Draft review")),
                            ),
                    ),
            )
            .child(sub_content)
    }

    fn render_office_pane(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let card_bg = if dark { rgb(0x111318) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x23262f) } else { rgb(0xdbe2ea) };
        let hdr_bg = if dark { rgb(0x181b22) } else { rgb(0xf1f5f9) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x0f172a) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x475569) };

        let row_num = self.selected_row + 2;
        let formula = if self.sheet_ai_applied {
            format!("=MAX(1, CEILING(C{row_num}/220, 1))")
        } else {
            format!("=INT(C{row_num}/220)")
        };

        let mut grid = div()
            .rounded(px(7.))
            .border_1()
            .border_color(border_c)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(8.))
                    .py(px(5.))
                    .bg(hdr_bg)
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .font_family("Fira Code")
                    .text_size(px(10.))
                    .text_color(sub)
                    .child(div().w(px(48.)).child("A · ID"))
                    .child(
                        div()
                            .flex_1()
                            .child(tr(zh, "B · 现场笔记", "B · Field Note")),
                    )
                    .child(div().w(px(52.)).child("C · Words"))
                    .child(div().w(px(56.)).child("D · Min")),
            );

        for (idx, r) in SHEET_ROWS.iter().enumerate() {
            let sel = self.selected_row == idx;
            let mins = if self.sheet_ai_applied {
                r.new_min
            } else {
                r.old_min
            };
            grid = grid.child(
                div()
                    .id(("kw-sheet-row", idx))
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(5.))
                    .bg(if sel {
                        if dark { rgb(0x1c2538) } else { rgb(0xdbeafe) }
                    } else {
                        card_bg
                    })
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .text_size(px(10.5))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.selected_row = idx;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(48.))
                            .font_family("Fira Code")
                            .text_size(px(10.))
                            .text_color(sub)
                            .child(r.id),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(ink)
                            .child(tr(zh, r.title_zh, r.title_en)),
                    )
                    .child(
                        div()
                            .w(px(52.))
                            .font_family("Fira Code")
                            .text_color(sub)
                            .child(format!("{}", r.words)),
                    )
                    .child(
                        div()
                            .w(px(56.))
                            .font_family("Fira Code")
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if mins == 0 {
                                rgb(0xef4444)
                            } else {
                                rgb(0x22c55e)
                            })
                            .child(format!("{mins} min")),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(tr(
                                zh,
                                "Kantor Office · Notes.xlsx",
                                "Kantor Office · Notes.xlsx",
                            )),
                    )
                    .child(
                        div()
                            .id("kw-sheet-toggle-ai")
                            .cursor_pointer()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded(px(5.))
                            .bg(rgb(0x2f5bea))
                            .text_size(px(10.))
                            .text_color(rgb(0xffffff))
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.sheet_ai_applied = !v.sheet_ai_applied;
                                cx.notify();
                            }))
                            .child(if self.sheet_ai_applied {
                                tr(zh, "对比旧公式", "Compare old formula")
                            } else {
                                tr(zh, "应用 AI 取整公式", "Apply AI formula")
                            }),
                    ),
            )
            .child(
                div()
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(6.))
                    .bg(hdr_bg)
                    .border_1()
                    .border_color(border_c)
                    .font_family("Fira Code")
                    .text_size(px(10.))
                    .text_color(ink)
                    .child(format!("D{row_num}  fx  {formula}")),
            )
            .child(grid)
    }

    fn render_terminal_pane(
        &self,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let bg = if dark { rgb(0x090a0d) } else { rgb(0x0f172a) };
        let border_c = if dark { rgb(0x23262f) } else { rgb(0x334155) };

        let mut log_col = div()
            .p(px(10.))
            .rounded(px(8.))
            .bg(bg)
            .border_1()
            .border_color(border_c)
            .flex()
            .flex_col()
            .gap(px(4.))
            .font_family("Fira Code")
            .text_size(px(10.));

        for line in &self.term_logs {
            let color = if line.starts_with('$') {
                rgb(0x93c5fd)
            } else if line.contains("ok") {
                rgb(0x4ade80)
            } else {
                rgb(0xe2e8f0)
            };
            log_col = log_col.child(div().text_color(color).child(line.clone()));
        }

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .id("kw-term-run-test")
                            .cursor_pointer()
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(5.))
                            .bg(rgb(0x2f5bea))
                            .font_family("Fira Code")
                            .text_size(px(10.))
                            .text_color(rgb(0xffffff))
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.term_logs.push("$ cargo test --quiet".into());
                                v.term_logs
                                    .push("test result: ok. 14 passed; 0 failed (0.16s)".into());
                                cx.notify();
                            }))
                            .child(tr(zh, "运行 cargo test", "Run cargo test")),
                    )
                    .child(
                        div()
                            .id("kw-term-git-stat")
                            .cursor_pointer()
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(border_c)
                            .font_family("Fira Code")
                            .text_size(px(10.))
                            .text_color(rgb(0x94a3b8))
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.term_logs.push("$ git status -sb".into());
                                v.term_logs
                                    .push("## main...origin/main · M src/reading_time.rs".into());
                                cx.notify();
                            }))
                            .child("git status"),
                    ),
            )
            .child(log_col)
    }
}

impl Render for KWorkApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let zh = is_zh(cx);
        let dark = is_dark(cx);
        let border_c = if dark { rgb(0x24252a) } else { rgb(0xd8dee9) };

        div()
            .size_full()
            .rounded(px(10.))
            .border_1()
            .border_color(border_c)
            .overflow_hidden()
            .flex()
            .child(self.render_sidebar(zh, dark, cx))
            .child(self.render_chat_stage(zh, dark, cx))
            .when(self.right_open, |d| {
                d.child(self.render_right_inspector(zh, dark, cx))
            })
    }
}
