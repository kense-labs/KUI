//! Authentic interactive GPUI product interfaces for KWork, KStack, KStudio, and K on the Kense homepage.

use gpui::{prelude::*, *};
use kui::{
    components::{button::Button, switch::Switch},
    foundation::{Theme, ThemeMode},
    primitives::{Size, control::UiLanguage, family::Paint},
};

pub const IDS: &[&str] = &[
    "kense-gateway",
    "kense-sandbox",
    "kense-workbench",
    "kense-klang",
];

/// Instantiate a Kense homepage GPUI product interface when `id` matches [`IDS`].
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    if !IDS.contains(&id) {
        return None;
    }
    crate::ensure_story_fonts(cx);
    let kind = match id {
        "kense-sandbox" => Kind::Sandbox,
        "kense-workbench" => Kind::Workbench,
        "kense-klang" => Kind::Klang,
        _ => Kind::Gateway,
    };
    let inner: AnyView = cx.new(|cx| ProductWindow::new(kind, w, cx)).into();
    Some(cx.new(|cx| HostShell::new(inner, cx)).into())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Gateway,
    Sandbox,
    Workbench,
    Klang,
}

struct HostShell {
    child: AnyView,
    focus: FocusHandle,
    edges: [FocusHandle; 2],
}

impl HostShell {
    fn new(child: AnyView, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        Self {
            child,
            focus: cx.focus_handle(),
            edges: [cx.focus_handle(), cx.focus_handle()],
        }
    }
}

impl Render for HostShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let outer_bg = if dark { rgb(0x0c0c0e) } else { rgb(0xf6f7f9) };
        let edge = |i| {
            div()
                .id(("kense-edge", i))
                .track_focus(&self.edges[i])
                .tab_stop(true)
                .size(px(0.))
        };
        let bridge = cx.try_global::<crate::StoryBridge>();
        let embedded = bridge.is_some();
        let min_height = bridge.map_or(572., |b| 572_f32.max((b.min_height)()));
        let content = div()
            .w_full()
            .h(px(min_height))
            .flex_none()
            .flex()
            .flex_col()
            .bg(outer_bg)
            .when(embedded, |e| e.child(edge(0)))
            .child(self.child.clone())
            .when(embedded, |e| e.child(edge(1)));

        div()
            .on_children_prepainted(|bounds, _, cx| {
                if let (Some(bounds), Some(bridge)) =
                    (bounds.first(), cx.try_global::<crate::StoryBridge>())
                {
                    (bridge.frame)(bounds.size.height.into(), Theme::global(cx).mode);
                }
            })
            .id("kense-gpui-showcase")
            .track_focus(&self.focus)
            .tab_stop(false)
            .tab_group()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .font_family("KUI Preview Latin")
            .text_color(paint.ink)
            .text_size(px(13.))
            .when(
                cfg!(target_family = "wasm") && UiLanguage::current(cx) == UiLanguage::Chinese,
                |e| e.font_family("KUI Story Sans"),
            )
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "tab" {
                    let back = e.keystroke.modifiers.shift;
                    let entering = v.focus.is_focused(w);
                    let step = |w: &mut Window, cx: &mut Context<Self>| {
                        if back {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        }
                    };
                    step(w, cx);
                    if entering && v.edges[usize::from(back)].is_focused(w) {
                        step(w, cx);
                    }
                    if v.edges[usize::from(!back)].is_focused(w) {
                        v.focus.focus(w, cx);
                        if let Some(b) = cx.try_global::<crate::StoryBridge>() {
                            (b.leave)(back);
                        }
                    }
                    cx.stop_propagation();
                }
            }))
            .child(content)
    }
}

#[derive(Clone, Copy)]
struct Chrome {
    sidebar_bg: Rgba,
    stage_bg: Rgba,
    inspector_bg: Rgba,
    card_bg: Rgba,
    elevated_bg: Rgba,
    active_row: Rgba,
    border: Rgba,
    ink: Rgba,
    secondary: Rgba,
    muted: Rgba,
    green: Rgba,
    green_bg: Rgba,
    red: Rgba,
    red_bg: Rgba,
    blue: Rgba,
    amber: Rgba,
}

impl Chrome {
    fn get(cx: &App) -> Self {
        if Theme::global(cx).mode == ThemeMode::Dark {
            Self {
                sidebar_bg: rgb(0x161618),
                stage_bg: rgb(0x0c0c0e),
                inspector_bg: rgb(0x101013),
                card_bg: rgb(0x151519),
                elevated_bg: rgb(0x1d1d22),
                active_row: rgb(0x232329),
                border: rgba(0xffffff14),
                ink: rgb(0xf4f4f6),
                secondary: rgb(0xa1a1aa),
                muted: rgb(0x71717a),
                green: rgb(0x34d399),
                green_bg: rgba(0x10b98122),
                red: rgb(0xf87171),
                red_bg: rgba(0xef444422),
                blue: rgb(0x3b82f6),
                amber: rgb(0xfbbf24),
            }
        } else {
            Self {
                sidebar_bg: rgb(0xf3f4f6),
                stage_bg: rgb(0xffffff),
                inspector_bg: rgb(0xf9fafb),
                card_bg: rgb(0xffffff),
                elevated_bg: rgb(0xf1f3f6),
                active_row: rgb(0xe7e9ef),
                border: rgba(0x00000014),
                ink: rgb(0x111113),
                secondary: rgb(0x52525b),
                muted: rgb(0x71717a),
                green: rgb(0x059669),
                green_bg: rgba(0x10b9811f),
                red: rgb(0xdc2626),
                red_bg: rgba(0xef44441a),
                blue: rgb(0x2563eb),
                amber: rgb(0xd97706),
            }
        }
    }
}

struct ProductWindow {
    kind: Kind,
    toggle: Entity<Switch>,
    primary_index: usize,
    sub_index: usize,
    nav_index: usize,
    flag_on: bool,
    alert_mode: bool,
    dictating: bool,
    draft_seeded: bool,
    sent_count: u32,
    _sub: Subscription,
}

impl ProductWindow {
    fn new(kind: Kind, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let sw_label = match kind {
            Kind::Gateway => lang.text("dGPU slice sharing", "单卡 dGPU 切分"),
            Kind::Sandbox => lang.text("Keep alive in tray", "关窗后台常驻"),
            Kind::Workbench => lang.text("Auto-run offline tests", "提交前自动跑单测"),
            Kind::Klang => lang.text("Prove checkfree BCE", "边界检查消除"),
        };
        let toggle =
            cx.new(|cx| Switch::new("kense-prod-switch", sw_label, w, cx).with_checked(true));
        let sub = cx.subscribe(&toggle, |this, _, checked: &bool, cx| {
            this.flag_on = *checked;
            cx.notify();
        });
        Self {
            kind,
            toggle,
            primary_index: 0,
            sub_index: 0,
            nav_index: match kind {
                Kind::Workbench => 3,
                Kind::Gateway => 1,
                _ => 0,
            },
            flag_on: true,
            alert_mode: false,
            dictating: false,
            draft_seeded: false,
            sent_count: 1,
            _sub: sub,
        }
    }

    fn render_window_dots(c: Chrome, title: &str) -> Div {
        div()
            .h(px(38.))
            .px(px(14.))
            .border_b_1()
            .border_color(c.border)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(9.)).rounded_full().bg(c.border))
                    .child(div().size(px(9.)).rounded_full().bg(c.border))
                    .child(div().size(px(9.)).rounded_full().bg(c.border)),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.secondary)
                    .truncate()
                    .child(title.to_owned()),
            )
    }

    /// 1:1 authentic `Kwrok` (`KWork`) 3-pane native desktop workbench (`kwrok-workbench.png`).
    fn render_kwork(&mut self, c: Chrome, zh: bool, compact: bool, cx: &mut Context<Self>) -> Div {
        let t = |en: &'static str, cn: &'static str| if zh { cn } else { en };
        let chats = [
            (
                t("Reading time", "阅读时长估算"),
                "•",
                "src/reading_time.rs",
                "+13",
                "-1",
            ),
            (
                t("Field guide", "离线听写接入"),
                "27m",
                "crates/kwrok_dictation/src/lib.rs",
                "+18",
                "-3",
            ),
            (
                t("Release notes", "发布说明校验"),
                "1h",
                "scripts/macos-kwrok-app.sh",
                "+9",
                "-2",
            ),
        ];
        let active_chat = chats[self.primary_index % chats.len()];
        let models = ["Codex", "DeepSeek V4", "Sonnet 4.5"];
        let active_model = models[self.sub_index % models.len()];

        let nav_items = [
            ("+  ", t("New chat", "新建对话")),
            ("•  ", t("Inbox", "收件箱")),
            ("▸  ", t("Documents", "项目文档")),
            ("±  ", t("Git changes", "Git 变更")),
        ];

        // Left sidebar
        let mut nav_col = div().flex().flex_col().gap(px(2.)).px(px(10.)).pt(px(10.));
        nav_col = nav_col.child(
            div()
                .mb(px(6.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(7.))
                .bg(c.elevated_bg)
                .border_1()
                .border_color(c.border)
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(12.))
                .text_color(c.muted)
                .child(t("Search", "搜索会话与文件"))
                .child(div().text_size(px(10.5)).child("⌘K")),
        );
        for (idx, (glyph, label)) in nav_items.into_iter().enumerate() {
            let selected = self.nav_index == idx;
            let ent = cx.entity();
            nav_col = nav_col.child(
                div()
                    .id(("kw-nav", idx))
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .bg(if selected {
                        c.active_row
                    } else {
                        rgba(0x00000000)
                    })
                    .text_size(px(12.5))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { c.ink } else { c.secondary })
                    .on_click(move |_, _, cx| {
                        ent.update(cx, |this, cx| {
                            this.nav_index = idx;
                            cx.notify();
                        });
                    })
                    .child(format!("{glyph}{label}")),
            );
        }

        let mut chat_list = div()
            .mt(px(14.))
            .px(px(10.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .px(px(8.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.muted)
                    .child(t("Chats", "最近会话")),
            );
        for (idx, (name, age, _, _, _)) in chats.into_iter().enumerate() {
            let selected = self.primary_index == idx;
            let ent = cx.entity();
            chat_list = chat_list.child(
                div()
                    .id(("kw-chat", idx))
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .bg(if selected {
                        c.active_row
                    } else {
                        rgba(0x00000000)
                    })
                    .flex()
                    .items_center()
                    .justify_between()
                    .on_click(move |_, _, cx| {
                        ent.update(cx, |this, cx| {
                            this.primary_index = idx;
                            this.draft_seeded = false;
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(if selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if selected { c.ink } else { c.secondary })
                            .truncate()
                            .child(name),
                    )
                    .child(div().text_size(px(11.)).text_color(c.muted).child(age)),
            );
        }

        let sidebar = div()
            .w(px(192.))
            .h_full()
            .flex_none()
            .bg(c.sidebar_bg)
            .border_r_1()
            .border_color(c.border)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(Self::render_window_dots(c, "Kwrok"))
                    .child(nav_col)
                    .child(chat_list),
            )
            .child(
                div()
                    .p(px(10.))
                    .border_t_1()
                    .border_color(c.border)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(self.toggle.clone())
                    .child(
                        div()
                            .px(px(4.))
                            .text_size(px(11.5))
                            .text_color(c.muted)
                            .child(t(
                                "•  Settings · Whisper Base",
                                "•  设置 · Whisper 离线语音",
                            )),
                    ),
            );

        // Center conversation pane
        let (prompt_msg, reply_summary, b1, b2, b3) = match self.primary_index % 3 {
            0 => (
                t(
                    "Round reading time up to the next minute. Even the shortest field note should show a one-minute read.",
                    "把阅读时长向上取整到下一分钟，哪怕最短的笔记也至少显示 1 分钟阅读时长。",
                ),
                t(
                    "Updated the reading-time estimate in src/reading_time.rs. Short notes stay at 1 min, and longer notes round up at 220 wpm.",
                    "已更新 src/reading_time.rs 中的阅读时长估算：短笔记固定至少 1 分钟，长文按 220 词/分钟向上取整。",
                ),
                t("• Empty note → 1 min", "• 空白或短笔记 → 1 分钟"),
                t("• 220 words → 1 min", "• 220 词边界 → 1 分钟"),
                t("• 221 words → 2 min", "• 221 词进位 → 2 分钟"),
            ),
            1 => (
                t(
                    "Wire local Whisper Base dictation into the composer mic button without network calls.",
                    "把本地 Whisper Base 离线语音识别接到输入框麦克风按钮上，全程不发外网请求。",
                ),
                t(
                    "Connected crates/kwrok_dictation to the composer draft buffer with automatic zh/en language detection on Apple Silicon CPU.",
                    "已将 crates/kwrok_dictation 接入输入框草稿区，在本地 CPU 上自动识别中英文并实时写入草稿。",
                ),
                t("• 0 network requests", "• 全程离线零外网请求"),
                t("• PCM 16kHz ring buffer", "• 16kHz 环形音频缓冲"),
                t("• Draft editable before send", "• 识别结果可编辑后再发送"),
            ),
            _ => (
                t(
                    "Verify DMG notarization and SHA-256 checksum output in macos-kwrok-app.sh.",
                    "检查 macos-kwrok-app.sh 打包脚本中的签名校验与 SHA-256 输出。",
                ),
                t(
                    "Added strict codesign --verify --deep and deterministic SHA-256 manifest generation before packaging Kwrok.app.",
                    "已在生成 Kwrok.app 后加入严格签名验证与确定性 SHA-256 校验清单输出。",
                ),
                t("• Universal bundle check", "• 校验 App Bundle 结构"),
                t("• Strict codesign verify", "• 强制验证开发者签名"),
                t("• Emit SHA256SUMS.txt", "• 自动写入 SHA-256 清单"),
            ),
        };

        let composer_text = if self.dictating {
            t(
                "● Listening offline (Whisper Base): Which tests can run without network?",
                "● 离线听写中（Whisper Base）：这里面哪些测试可以完全离线运行？",
            )
        } else if self.draft_seeded {
            t(
                "Review the staged diff in main and check edge cases before commit.",
                "请复查当前暂存区与 main 的代码差异，检查是否还有遗漏的边界情况。",
            )
        } else {
            t("Ask a follow-up…", "继续追问，或输入 / 调出规划与目标指令…")
        };

        let mic_ent = cx.entity();
        let model_ent = cx.entity();
        let send_ent = cx.entity();

        let center = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(c.stage_bg)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(38.))
                            .px(px(18.))
                            .border_b_1()
                            .border_color(c.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.5))
                                    .child(
                                        div().text_color(c.muted).child("Field Notes  /"),
                                    )
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(c.ink)
                                            .child(active_chat.0),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_family("Fira Code")
                                    .text_color(c.muted)
                                    .child(format!("#{:02}", self.sent_count)),
                            ),
                    )
                    .child(
                        div()
                            .p(px(20.))
                            .flex()
                            .flex_col()
                            .gap(px(14.))
                            .child(
                                div().w_full().flex().justify_end().child(
                                    div()
                                        .max_w(px(380.))
                                        .px(px(14.))
                                        .py(px(10.))
                                        .rounded(px(12.))
                                        .bg(c.elevated_bg)
                                        .border_1()
                                        .border_color(c.border)
                                        .text_size(px(12.5))
                                        .line_height(rems(1.45))
                                        .text_color(c.ink)
                                        .child(prompt_msg),
                                ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(11.5))
                                    .text_color(c.muted)
                                    .child(t("Took 12s", "思考与修改耗时 12s"))
                                    .child(
                                        div()
                                            .px(px(7.))
                                            .py(px(2.))
                                            .rounded(px(5.))
                                            .bg(c.elevated_bg)
                                            .font_family("Fira Code")
                                            .text_size(px(11.))
                                            .text_color(c.ink)
                                            .child(active_chat.2),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(rems(1.5))
                                    .text_color(c.ink)
                                    .child(reply_summary),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(c.ink)
                                            .child(t("Boundary checks", "边界验证结果")),
                                    )
                                    .child(div().text_size(px(12.5)).text_color(c.secondary).child(b1))
                                    .child(div().text_size(px(12.5)).text_color(c.secondary).child(b2))
                                    .child(div().text_size(px(12.5)).text_color(c.secondary).child(b3)),
                            )
                            .when(self.sent_count > 1, |d| {
                                d.child(
                                    div()
                                        .px(px(11.))
                                        .py(px(7.))
                                        .rounded(px(8.))
                                        .bg(c.green_bg)
                                        .text_size(px(12.))
                                        .text_color(c.green)
                                        .child(t(
                                            "✓ cargo test --offline passed (14/14 unit tests green)",
                                            "✓ 已执行离线回归测试：14/14 项单元测试全部通过",
                                        )),
                                )
                            }),
                    ),
            )
            .child(
                div().p(px(14.)).child(
                    div()
                        .rounded(px(14.))
                        .bg(c.card_bg)
                        .border_1()
                        .border_color(c.border)
                        .p(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(if self.dictating || self.draft_seeded {
                                    c.ink
                                } else {
                                    c.muted
                                })
                                .child(composer_text),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .id("kw-mic")
                                                .cursor_pointer()
                                                .px(px(8.))
                                                .py(px(4.))
                                                .rounded(px(6.))
                                                .bg(if self.dictating {
                                                    c.red_bg
                                                } else {
                                                    c.elevated_bg
                                                })
                                                .text_size(px(11.5))
                                                .text_color(if self.dictating {
                                                    c.red
                                                } else {
                                                    c.secondary
                                                })
                                                .on_click(move |_, _, cx| {
                                                    mic_ent.update(cx, |this, cx| {
                                                        this.dictating = !this.dictating;
                                                        cx.notify();
                                                    });
                                                })
                                                .child(if self.dictating {
                                                    t("● Whisper", "● 听写中")
                                                } else {
                                                    t("• Dictate", "• 离线听写")
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(c.muted)
                                                .child("Context"),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .id("kw-model")
                                                .cursor_pointer()
                                                .px(px(8.))
                                                .py(px(4.))
                                                .rounded(px(6.))
                                                .bg(c.elevated_bg)
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(c.ink)
                                                .on_click(move |_, _, cx| {
                                                    model_ent.update(cx, |this, cx| {
                                                        this.sub_index =
                                                            (this.sub_index + 1) % 3;
                                                        cx.notify();
                                                    });
                                                })
                                                .child(format!("{active_model} ▾")),
                                        )
                                        .child(
                                            div()
                                                .id("kw-send")
                                                .cursor_pointer()
                                                .size(px(26.))
                                                .rounded_full()
                                                .bg(c.blue)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(0xffffff))
                                                .on_click(move |_, _, cx| {
                                                    send_ent.update(cx, |this, cx| {
                                                        this.sent_count =
                                                            this.sent_count.wrapping_add(1);
                                                        this.dictating = false;
                                                        this.draft_seeded = false;
                                                        cx.notify();
                                                    });
                                                })
                                                .child("↑"),
                                        ),
                                ),
                        ),
                ),
            );

        // Right Git Diff pane
        let draft_ent = cx.entity();
        let diff_lines: [(&str, &str, u8); 9] = match self.primary_index % 3 {
            0 => [
                (" 5 -", "  words / WORDS_PER_MINUTE", 1),
                (" 5 +", "  words.div_ceil(WORDS_PER_MINUTE).max(1)", 2),
                (" 6 +", "}", 2),
                (" 8 +", "#[cfg(test)]", 2),
                (" 9 +", "mod tests {", 2),
                ("12 +", "  #[test]", 2),
                ("13 +", "  fn counts_partial_minutes() {", 2),
                ("14 +", "    assert_eq!(minutes(0), 1);", 2),
                ("16 +", "    assert_eq!(minutes(221), 2);", 2),
            ],
            1 => [
                ("41 -", "  let text = cloud_transcribe(pcm).await?;", 1),
                ("41 +", "  let text = whisper_cpu.decode_pcm16(&pcm)?;", 2),
                ("42 +", "  composer.insert_draft(&text, cx);", 2),
                ("43  ", "  cx.notify();", 0),
                ("44 +", "  telemetry::record_offline_ms(elapsed);", 2),
                ("45  ", "  Ok(())", 0),
                ("46  ", "}", 0),
                ("48 +", "#[test] fn offline_zh_en_mix() { ... }", 2),
                ("49  ", "", 0),
            ],
            _ => [
                ("18 -", "codesign --force \"$APP_BUNDLE\"", 1),
                (
                    "18 +",
                    "codesign --force --options runtime \"$APP_BUNDLE\"",
                    2,
                ),
                (
                    "19 +",
                    "codesign --verify --deep --strict \"$APP_BUNDLE\"",
                    2,
                ),
                (
                    "20 +",
                    "shasum -a 256 \"$DMG\" > \"$OUT/SHA256SUMS.txt\"",
                    2,
                ),
                ("21  ", "echo \"Packaged Kwrok.app\"", 0),
                ("22  ", "", 0),
                ("23  ", "", 0),
                ("24  ", "", 0),
                ("25  ", "", 0),
            ],
        };

        let mut code_box = div()
            .rounded(px(8.))
            .bg(c.stage_bg)
            .border_1()
            .border_color(c.border)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .border_b_1()
                    .border_color(c.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .font_family("Fira Code")
                    .text_size(px(11.5))
                    .child(div().text_color(c.ink).child(active_chat.2))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().text_color(c.green).child(active_chat.3))
                            .child(div().text_color(c.red).child(active_chat.4)),
                    ),
            );

        for (ln, code, kind_flag) in diff_lines {
            if ln.trim().is_empty() && code.is_empty() {
                continue;
            }
            let row_bg = match kind_flag {
                1 => c.red_bg,
                2 => c.green_bg,
                _ => rgba(0x00000000),
            };
            let row_fg = match kind_flag {
                1 => c.red,
                2 => c.green,
                _ => c.secondary,
            };
            code_box = code_box.child(
                div()
                    .px(px(8.))
                    .py(px(3.))
                    .bg(row_bg)
                    .font_family("Fira Code")
                    .text_size(px(11.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(px(30.)).flex_none().text_color(c.muted).child(ln))
                    .child(div().min_w_0().truncate().text_color(row_fg).child(code)),
            );
        }

        let diff_pane = div()
            .w(px(320.))
            .h_full()
            .flex_none()
            .bg(c.inspector_bg)
            .border_l_1()
            .border_color(c.border)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(38.))
                    .px(px(12.))
                    .border_b_1()
                    .border_color(c.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .px(px(9.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .bg(c.elevated_bg)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.ink)
                            .child("±  Diff  ×"),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_family("Fira Code")
                            .text_color(c.secondary)
                            .child("git · main"),
                    ),
            )
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(c.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded(px(999.))
                                    .bg(c.active_row)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(c.ink)
                                    .child(t("Changes 1", "变更 1")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(c.muted)
                                    .child(t("History · PR", "历史 · PR")),
                            ),
                    )
                    .child(
                        div()
                            .id("kw-draft-review")
                            .cursor_pointer()
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(11.))
                            .text_color(c.ink)
                            .on_click(move |_, _, cx| {
                                draft_ent.update(cx, |this, cx| {
                                    this.draft_seeded = true;
                                    this.dictating = false;
                                    cx.notify();
                                });
                            })
                            .child(t("Draft review", "起草审查请求")),
                    ),
            )
            .child(div().p(px(12.)).child(code_box));

        div()
            .size_full()
            .flex()
            .when(!compact, |d| d.child(sidebar))
            .child(center)
            .when(!compact, |d| d.child(diff_pane))
    }

    /// Real `KStack` heterogeneous GPU cluster & `/v1` model serving console.
    fn render_kstack(&mut self, c: Chrome, zh: bool, compact: bool, cx: &mut Context<Self>) -> Div {
        let t = |en: &'static str, cn: &'static str| if zh { cn } else { en };
        let models = [
            (
                "Qwen3-72B-FP8",
                "41.2 GB",
                "14.8 GB",
                "58.4 / 80 GB",
                0.73_f32,
            ),
            (
                "DeepSeek-R1-Distill",
                "34.0 GB",
                "11.2 GB",
                "47.6 / 64 GB",
                0.74_f32,
            ),
            (
                "Llama-4-Scout-17B",
                "19.4 GB",
                "8.6 GB",
                "30.2 / 192 GB",
                0.16_f32,
            ),
        ];
        let m = models[self.sub_index % models.len()];

        let nodes = [
            (
                "node-hz-h800-04",
                "NVIDIA H800 · 80 GB",
                "vLLM · NVLink",
                t("Matched · 76 GB free", "已匹配 · 空闲 76 GB"),
                true,
            ),
            (
                "node-bj-910b-02",
                "Ascend 910B · 64 GB",
                "vLLM-Ascend · Mooncake",
                t("Eligible · 61 GB free", "可调度 · 空闲 61 GB"),
                true,
            ),
            (
                "studio-m3-ultra",
                "Apple M3 Ultra · 192 GB",
                "mlx-engine · Unified",
                t("Eligible · 164 GB free", "可调度 · 空闲 164 GB"),
                true,
            ),
            (
                "node-sh-a10-09",
                "NVIDIA A10 · 24 GB",
                "PCIe Gen4",
                t("Skipped: 24 GB < 58.4 GB", "已排除：显存不足 24 < 58.4 GB"),
                false,
            ),
        ];

        let sidebar = div()
            .w(px(176.))
            .h_full()
            .flex_none()
            .bg(c.sidebar_bg)
            .border_r_1()
            .border_color(c.border)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(Self::render_window_dots(c, "KStack · 40 MB"))
                    .child(
                        div().p(px(10.)).flex().flex_col().gap(px(3.)).children(
                            [
                                t("Cluster Overview", "集群总览"),
                                t("Deployments /v1", "模型部署 /v1"),
                                t("GPU Nodes (1,003)", "算力节点 (1,003)"),
                                t("dGPU Quotas", "dGPU 显存切分"),
                                t("Benchmarks", "吞吐压测与计费"),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(i, lbl)| {
                                let active = i == 1;
                                div()
                                    .px(px(10.))
                                    .py(px(6.))
                                    .rounded(px(7.))
                                    .bg(if active {
                                        c.active_row
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .text_size(px(12.5))
                                    .font_weight(if active {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if active { c.ink } else { c.secondary })
                                    .child(lbl)
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .p(px(12.))
                    .border_t_1()
                    .border_color(c.border)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(self.toggle.clone())
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_family("Fira Code")
                            .text_color(c.green)
                            .child(t("● 7 regions · 1,003 GPUs", "● 7 地域 · 1,003 卡在线")),
                    ),
            );

        let mut model_tabs = div().flex().flex_wrap().items_center().gap(px(6.));
        for (idx, (name, _, _, _, _)) in models.into_iter().enumerate() {
            let active = self.sub_index % models.len() == idx;
            let ent = cx.entity();
            model_tabs = model_tabs.child(
                div()
                    .id(("ks-model", idx))
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(5.))
                    .rounded(px(7.))
                    .bg(if active { c.active_row } else { c.card_bg })
                    .border_1()
                    .border_color(c.border)
                    .font_family("Fira Code")
                    .text_size(px(11.5))
                    .text_color(if active { c.ink } else { c.secondary })
                    .on_click(move |_, _, cx| {
                        ent.update(cx, |this, cx| {
                            this.sub_index = idx;
                            this.primary_index = idx.min(2);
                            cx.notify();
                        });
                    })
                    .child(name),
            );
        }

        let mut node_rows = div()
            .rounded(px(10.))
            .bg(c.card_bg)
            .border_1()
            .border_color(c.border)
            .overflow_hidden()
            .flex()
            .flex_col();

        for (idx, (host, chip, engine, reason, ok)) in nodes.into_iter().enumerate() {
            let selected = self.primary_index == idx;
            let ent = cx.entity();
            node_rows = node_rows.child(
                div()
                    .id(("ks-node", idx))
                    .cursor_pointer()
                    .px(px(12.))
                    .py(px(9.))
                    .when(idx > 0, |d| d.border_t_1().border_color(c.border))
                    .bg(if selected {
                        c.active_row
                    } else {
                        rgba(0x00000000)
                    })
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .on_click(move |_, _, cx| {
                        if ok {
                            ent.update(cx, |this, cx| {
                                this.primary_index = idx;
                                cx.notify();
                            });
                        }
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(div().size(px(7.)).rounded_full().bg(if !ok {
                                        c.muted
                                    } else if selected {
                                        c.green
                                    } else {
                                        c.blue
                                    }))
                                    .child(
                                        div()
                                            .font_family("Fira Code")
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if ok { c.ink } else { c.muted })
                                            .child(host),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(c.secondary)
                                            .child(chip),
                                    ),
                            )
                            .child(
                                div()
                                    .pl(px(15.))
                                    .font_family("Fira Code")
                                    .text_size(px(11.))
                                    .text_color(c.muted)
                                    .child(engine),
                            ),
                    )
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(6.))
                            .bg(if !ok {
                                c.red_bg
                            } else if selected {
                                c.green_bg
                            } else {
                                c.elevated_bg
                            })
                            .text_size(px(11.))
                            .text_color(if !ok {
                                c.red
                            } else if selected {
                                c.green
                            } else {
                                c.secondary
                            })
                            .child(reason),
                    ),
            );
        }

        let deploy_ent = cx.entity();
        let deploy_btn = Button::new("ks-deploy-btn")
            .primary()
            .with_size(Size::Sm)
            .label(t("Deploy /v1 replica", "拉起 /v1 推理实例"))
            .on_click(move |_, _, cx| {
                deploy_ent.update(cx, |this, cx| {
                    this.sent_count = this.sent_count.wrapping_add(1);
                    cx.notify();
                });
            })
            .into_themed(cx);

        let center = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(c.stage_bg)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(38.))
                    .px(px(18.))
                    .border_b_1()
                    .border_color(c.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(12.5))
                            .text_color(c.ink)
                            .child(t(
                                "VRAM Planner & Heterogeneous Placement",
                                "启动前显存预算与异构选卡",
                            )),
                    )
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(11.))
                            .text_color(c.green)
                            .child(format!("POST /v1/chat/completions · #{}", self.sent_count)),
                    ),
            )
            .child(
                div()
                    .p(px(18.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(model_tabs)
                            .child(deploy_btn),
                    )
                    .child(
                        div()
                            .rounded(px(10.))
                            .bg(c.card_bg)
                            .border_1()
                            .border_color(c.border)
                            .p(px(12.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(12.))
                                    .child(div().text_color(c.secondary).child(format!(
                                        "{} {} + KV {} · {}",
                                        t("Weights", "权重"),
                                        m.1,
                                        m.2,
                                        if self.flag_on {
                                            t("dGPU quota enabled", "dGPU 显存配额已开启")
                                        } else {
                                            t("Exclusive full card", "整卡独占模式")
                                        }
                                    )))
                                    .child(
                                        div()
                                            .font_family("Fira Code")
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(c.ink)
                                            .child(m.3),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(6.))
                                    .rounded_full()
                                    .bg(c.elevated_bg)
                                    .overflow_hidden()
                                    .child(
                                        div().h_full().w(relative(m.4)).rounded_full().bg(c.green),
                                    ),
                            ),
                    )
                    .child(node_rows),
            );

        let inspector = div()
            .w(px(236.))
            .h_full()
            .flex_none()
            .bg(c.inspector_bg)
            .border_l_1()
            .border_color(c.border)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(38.))
                    .px(px(14.))
                    .border_b_1()
                    .border_color(c.border)
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.ink)
                    .child(t("/v1 Live Gateway", "/v1 网关实时指标")),
            )
            .child(
                div()
                    .p(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .children(
                        [
                            (t("TTFT p50", "首字延迟 TTFT"), "164 ms"),
                            (t("Throughput", "解码吞吐"), "148.6 tok/s"),
                            (t("Verified Specs", "内置验证规格"), "392 / 1,235"),
                        ]
                        .into_iter()
                        .map(|(k, v)| {
                            div()
                                .p(px(10.))
                                .rounded(px(8.))
                                .bg(c.card_bg)
                                .border_1()
                                .border_color(c.border)
                                .flex()
                                .flex_col()
                                .gap(px(3.))
                                .child(div().text_size(px(11.)).text_color(c.muted).child(k))
                                .child(
                                    div()
                                        .font_family("Fira Code")
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(c.ink)
                                        .child(v),
                                )
                        }),
                    )
                    .child(
                        div()
                            .p(px(10.))
                            .rounded(px(8.))
                            .bg(c.stage_bg)
                            .border_1()
                            .border_color(c.border)
                            .font_family("Fira Code")
                            .text_size(px(10.5))
                            .line_height(rems(1.45))
                            .text_color(c.secondary)
                            .child(format!(
                                "curl $KSTACK/v1/models\n→ {} ready\n→ replica #{:02} healthy",
                                m.0, self.sent_count
                            )),
                    ),
            );

        div()
            .size_full()
            .flex()
            .when(!compact, |d| d.child(sidebar))
            .child(center)
            .when(!compact, |d| d.child(inspector))
    }

    /// Real `KStudio` multi-machine SSH, persistent PTY & isolated Agent Hub.
    fn render_kstudio(
        &mut self,
        c: Chrome,
        zh: bool,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let t = |en: &'static str, cn: &'static str| if zh { cn } else { en };
        let hosts = [
            (
                "desk-studio.local",
                "M3 Ultra · 24c · 142 GB free",
                "macOS Seatbelt",
            ),
            (
                "build-box-02",
                "Ubuntu 24.04 · KBox MicroVM",
                "libkrun KVM · 14ms",
            ),
            (
                "cn-hz-gpu-07",
                "Tailscale · H800 × 4",
                "Bubblewrap + seccomp",
            ),
        ];
        let active_host = hosts[self.primary_index % hosts.len()];

        let mut host_col = div().p(px(10.)).flex().flex_col().gap(px(4.)).child(
            div()
                .px(px(8.))
                .pb(px(4.))
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.muted)
                .child(t("Machines (SSH / Tailscale)", "已连接机器")),
        );
        for (idx, (name, spec, _)) in hosts.into_iter().enumerate() {
            let selected = self.primary_index % hosts.len() == idx;
            let ent = cx.entity();
            host_col = host_col.child(
                div()
                    .id(("kst-host", idx))
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(8.))
                    .bg(if selected {
                        c.active_row
                    } else {
                        rgba(0x00000000)
                    })
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .on_click(move |_, _, cx| {
                        ent.update(cx, |this, cx| {
                            this.primary_index = idx;
                            this.alert_mode = false;
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().size(px(6.)).rounded_full().bg(c.green))
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(c.ink)
                                    .truncate()
                                    .child(name),
                            ),
                    )
                    .child(
                        div()
                            .pl(px(12.))
                            .text_size(px(11.))
                            .text_color(c.muted)
                            .truncate()
                            .child(spec),
                    ),
            );
        }

        let sidebar = div()
            .w(px(196.))
            .h_full()
            .flex_none()
            .bg(c.sidebar_bg)
            .border_r_1()
            .border_color(c.border)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(Self::render_window_dots(c, "KStudio"))
                    .child(host_col)
                    .child(
                        div()
                            .px(px(18.))
                            .pt(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(c.muted)
                                    .child(t("18 Agent CLIs", "托管编程 Agent (18)")),
                            )
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(11.5))
                                    .text_color(c.ink)
                                    .child("● claude-code  #142"),
                            )
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(11.5))
                                    .text_color(c.secondary)
                                    .child("● codex        review"),
                            )
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(11.5))
                                    .text_color(c.secondary)
                                    .child("• gemini-cli   idle"),
                            ),
                    ),
            )
            .child(
                div()
                    .p(px(12.))
                    .border_t_1()
                    .border_color(c.border)
                    .child(self.toggle.clone()),
            );

        let block_ent = cx.entity();
        let run_ent = cx.entity();
        let test_block_btn = Button::new("kst-block-btn")
            .secondary()
            .with_size(Size::Sm)
            .label(t("Test ~/.ssh block", "模拟读取宿主机 ~/.ssh"))
            .on_click(move |_, _, cx| {
                block_ent.update(cx, |this, cx| {
                    this.alert_mode = !this.alert_mode;
                    cx.notify();
                });
            })
            .into_themed(cx);
        let dispatch_btn = Button::new("kst-dispatch-btn")
            .primary()
            .with_size(Size::Sm)
            .label(t("Dispatch issue #142", "派发 Issue #142"))
            .on_click(move |_, _, cx| {
                run_ent.update(cx, |this, cx| {
                    this.alert_mode = false;
                    this.sent_count = this.sent_count.wrapping_add(1);
                    cx.notify();
                });
            })
            .into_themed(cx);

        let center = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(c.stage_bg)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(38.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(c.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .font_family("Fira Code")
                                    .text_size(px(11.5))
                                    .child(
                                        div()
                                            .px(px(8.))
                                            .py(px(3.))
                                            .rounded(px(5.))
                                            .bg(c.active_row)
                                            .text_color(c.ink)
                                            .child(format!("ssh://{}", active_host.0)),
                                    )
                                    .child(
                                        div()
                                            .text_color(c.muted)
                                            .child(t("pty · Linux Desktop · Files", "常驻终端 · 远程桌面 · 跨机传文件")),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(c.green)
                                    .child(active_host.2),
                            ),
                    )
                    .child(
                        div()
                            .p(px(18.))
                            .font_family("Fira Code")
                            .text_size(px(12.))
                            .line_height(rems(1.6))
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_color(c.muted)
                                    .child(format!("$ kstudio attach {} --box warm-pool", active_host.0)),
                            )
                            .child(
                                div()
                                    .text_color(c.green)
                                    .child("✓ Mounted isolated workspace /work/repo in 14ms"),
                            )
                            .child(
                                div()
                                    .text_color(c.ink)
                                    .child("$ claude-code \"Fix race in connection pool (#142)\""),
                            )
                            .child(
                                div()
                                    .text_color(c.secondary)
                                    .child(format!(
                                        "→ Running cargo test --workspace (session #{:02}, survives window close)",
                                        self.sent_count
                                    )),
                            )
                            .when(self.alert_mode, |d| {
                                d.child(
                                    div()
                                        .mt(px(4.))
                                        .px(px(10.))
                                        .py(px(7.))
                                        .rounded(px(7.))
                                        .bg(c.red_bg)
                                        .text_color(c.red)
                                        .child(t(
                                            "× KSandbox blocked open(\"/Users/host/.ssh/id_ed25519\") -> EPERM",
                                            "× KSandbox 沙箱已拦截宿主机敏感路径读取 (~/.ssh/id_ed25519) -> EPERM",
                                        )),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .px(px(18.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(c.border)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c.secondary)
                            .child(if self.flag_on {
                                t("Tray daemon active · SSH mux persistent", "菜单栏守护已启用 · 关窗不断连")
                            } else {
                                t("Close session with window", "关闭窗口时结束远程会话")
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(test_block_btn)
                            .child(dispatch_btn),
                    ),
            );

        let inspector = div()
            .w(px(224.))
            .h_full()
            .flex_none()
            .bg(c.inspector_bg)
            .border_l_1()
            .border_color(c.border)
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.ink)
                    .child(t("Isolation & Tunnels", "沙箱边界与端口转发")),
            )
            .child(
                div()
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(c.card_bg)
                    .border_1()
                    .border_color(c.border)
                    .font_family("Fira Code")
                    .text_size(px(11.))
                    .line_height(rems(1.5))
                    .text_color(c.secondary)
                    .child("rw: /work/repo\ndeny: ~/.ssh, ~/.aws\nnet: allowlist only"),
            )
            .child(
                div()
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(c.card_bg)
                    .border_1()
                    .border_color(c.border)
                    .font_family("Fira Code")
                    .text_size(px(11.))
                    .line_height(rems(1.5))
                    .text_color(c.ink)
                    .child(":8080 → 127.0.0.1:8080\n:5901 → Linux VNC live"),
            );

        div()
            .size_full()
            .flex()
            .when(!compact, |d| d.child(sidebar))
            .child(center)
            .when(!compact, |d| d.child(inspector))
    }

    /// Real `K` systems compiler (`k`), lifetime escape checker & Cordis lock studio.
    fn render_klang(&mut self, c: Chrome, zh: bool, compact: bool, cx: &mut Context<Self>) -> Div {
        let t = |en: &'static str, cn: &'static str| if zh { cn } else { en };
        let targets = [
            ("Native C (-O3)", "build/onnx-gateway (1.8 MB)"),
            ("wasm32-wasip1", "build/gateway.wasm (412 KB)"),
            ("TinyCC JIT", "in-process · 6 ms cold start"),
        ];
        let active_target = targets[self.sub_index % targets.len()];

        let mut target_col = div().p(px(10.)).flex().flex_col().gap(px(4.)).child(
            div()
                .px(px(8.))
                .pb(px(4.))
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.muted)
                .child(t("Execution Target", "编译与执行目标")),
        );
        for (idx, (name, out)) in targets.into_iter().enumerate() {
            let selected = self.sub_index % targets.len() == idx;
            let ent = cx.entity();
            target_col = target_col.child(
                div()
                    .id(("kl-target", idx))
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(8.))
                    .bg(if selected {
                        c.active_row
                    } else {
                        rgba(0x00000000)
                    })
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .on_click(move |_, _, cx| {
                        ent.update(cx, |this, cx| {
                            this.sub_index = idx;
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.ink)
                            .child(name),
                    )
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(10.5))
                            .text_color(c.muted)
                            .child(out),
                    ),
            );
        }

        let sidebar = div()
            .w(px(184.))
            .h_full()
            .flex_none()
            .bg(c.sidebar_bg)
            .border_r_1()
            .border_color(c.border)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(Self::render_window_dots(c, "k · Cordis"))
                    .child(target_col),
            )
            .child(
                div()
                    .p(px(12.))
                    .border_t_1()
                    .border_color(c.border)
                    .child(self.toggle.clone()),
            );

        let err_ent = cx.entity();
        let build_ent = cx.entity();
        let escape_btn = Button::new("kl-escape-btn")
            .secondary()
            .with_size(Size::Sm)
            .label(if self.alert_mode {
                t("Fix lifetime escape", "修复悬垂引用")
            } else {
                t("Test ref escape", "注入局部引用越界")
            })
            .on_click(move |_, _, cx| {
                err_ent.update(cx, |this, cx| {
                    this.alert_mode = !this.alert_mode;
                    cx.notify();
                });
            })
            .into_themed(cx);
        let build_btn = Button::new("kl-build-btn")
            .primary()
            .with_size(Size::Sm)
            .label(t("k check && k build", "编译并验证"))
            .on_click(move |_, _, cx| {
                build_ent.update(cx, |this, cx| {
                    this.alert_mode = false;
                    this.sent_count = this.sent_count.wrapping_add(1);
                    cx.notify();
                });
            })
            .into_themed(cx);

        let return_line = if self.alert_mode {
            ("08", "  return &scratch[0]; // × escapes scope!", true)
        } else {
            ("08", "  return out.commit(total); // ✓ safe value", false)
        };
        let src_lines = [
            ("01", "module gateway imports cordis::http;", false),
            ("02", "", false),
            (
                "03",
                "fn route_batch(req: &Request, out: &mut Reply) -> Status {",
                false,
            ),
            ("04", "  datastack scratch: [Token; 256];", false),
            ("05", "  let n = scratch.decode_from(req.body());", false),
            ("06", "  var total: u64 = 0;", false),
            (
                "07",
                "  for i in 0..n { total += scratch[i].weight; } // checkfree",
                false,
            ),
            return_line,
            ("09", "} // scratch reclaimed at scope exit (0 GC)", false),
        ];

        let mut editor = div()
            .p(px(14.))
            .font_family("Fira Code")
            .text_size(px(11.5))
            .flex()
            .flex_col()
            .gap(px(3.));
        for (ln, code, is_err) in src_lines {
            editor = editor.child(
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .bg(if is_err { c.red_bg } else { rgba(0x00000000) })
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(div().w(px(22.)).flex_none().text_color(c.muted).child(ln))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(if is_err { c.red } else { c.ink })
                            .child(code),
                    ),
            );
        }

        let center = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(c.stage_bg)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(38.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(c.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .font_family("Fira Code")
                            .text_size(px(12.))
                            .child(div().text_color(c.ink).child("src/gateway.k"))
                            .child(div().text_color(c.muted).child(active_target.0)),
                    )
                    .child(editor),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(c.border)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(11.5))
                            .text_color(if self.alert_mode { c.red } else { c.green })
                            .child(if self.alert_mode {
                                "error[E0312]: reference to datastack `scratch` escapes fn"
                            } else {
                                "✓ 0 lifetime errors · 0 GC pauses · cordis.lock verified"
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(escape_btn)
                            .child(build_btn),
                    ),
            );

        let inspector = div()
            .w(px(236.))
            .h_full()
            .flex_none()
            .bg(c.inspector_bg)
            .border_l_1()
            .border_color(c.border)
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.ink)
                    .child(t("Compiler Verification", "编译期静态证明")),
            )
            .children(
                [
                    (
                        t("Reference Lifetimes", "引用生命周期"),
                        if self.alert_mode {
                            ("REJECTED", c.red)
                        } else {
                            ("VERIFIED", c.green)
                        },
                    ),
                    (
                        t("Bounds Checks (BCE)", "越界检查消除"),
                        if self.flag_on {
                            ("14/14 ELIDED", c.green)
                        } else {
                            ("DYNAMIC KEPT", c.amber)
                        },
                    ),
                    (
                        t("Cordis Service Lock", "Cordis 服务锁"),
                        ("LOCKED", c.green),
                    ),
                ]
                .into_iter()
                .map(|(label, (val, col))| {
                    div()
                        .p(px(10.))
                        .rounded(px(8.))
                        .bg(c.card_bg)
                        .border_1()
                        .border_color(c.border)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(c.secondary)
                                .child(label),
                        )
                        .child(
                            div()
                                .font_family("Fira Code")
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(col)
                                .child(val),
                        )
                }),
            )
            .child(
                div()
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(c.stage_bg)
                    .border_1()
                    .border_color(c.border)
                    .font_family("Fira Code")
                    .text_size(px(10.5))
                    .line_height(rems(1.45))
                    .text_color(c.secondary)
                    .child(format!(
                        "target: {}\nout: {}\nbuild #{:02} ok",
                        active_target.0, active_target.1, self.sent_count
                    )),
            );

        div()
            .size_full()
            .flex()
            .when(!compact, |d| d.child(sidebar))
            .child(center)
            .when(!compact, |d| d.child(inspector))
    }
}

impl Render for ProductWindow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Chrome::get(cx);
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let compact = w.viewport_size().width < px(760.);

        let inner = match self.kind {
            Kind::Workbench => self.render_kwork(c, zh, compact, cx),
            Kind::Gateway => self.render_kstack(c, zh, compact, cx),
            Kind::Sandbox => self.render_kstudio(c, zh, compact, cx),
            Kind::Klang => self.render_klang(c, zh, compact, cx),
        };

        div()
            .size_full()
            .bg(c.stage_bg)
            .overflow_hidden()
            .child(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::IDS;
    use gpui::prelude::*;

    #[test]
    fn kense_showcase_scenes_mount_and_step() {
        let mut cx = gpui::TestAppContext::single();
        cx.update(kui::init);
        let (_, cx) =
            cx.add_window_view(|w, cx| crate::StoryHost::new(Some("kense-gateway"), w, cx));
        cx.update(|w, cx| {
            for id in IDS {
                let host = cx.new(|cx| crate::StoryHost::new(Some(id), w, cx));
                host.update(cx, |h, cx| {
                    assert!(h.show_story(id, w, cx));
                });
            }
        });
    }
}
