//! Authentic interactive KStudio Multi-Machine GPUI Hub (`50-main-overview` & `62-entity-session`).

use super::{is_dark, is_zh, tr, traffic_lights};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum StudioView {
    Overview,
    Session(usize),
    RuntimeBoxes,
    Runners,
    AiUsage,
}

pub(super) struct KStudioApp {
    pub(super) view: StudioView,
    pub(super) alerts_resolved: [bool; 3],
    pub(super) session_logs: [Vec<SharedString>; 3],
    pub(super) session_agent: [&'static str; 3],
    pub(super) box_paused: [bool; 3],
}

struct MachineInfo {
    name: &'static str,
    os_zh: &'static str,
    os_en: &'static str,
    cpu: f32,
    mem: f32,
    disk_zh: &'static str,
    disk_en: &'static str,
    session_idx: Option<usize>,
    runners_zh: &'static str,
    runners_en: &'static str,
}

const MACHINES: [MachineInfo; 6] = [
    MachineInfo {
        name: "Forge",
        os_zh: "macOS 26 · arm64",
        os_en: "macOS 26 · arm64",
        cpu: 0.37,
        mem: 0.41,
        disk_zh: "剩余 1.2 TB",
        disk_en: "1.2 TB free",
        session_idx: Some(0),
        runners_zh: "3 个 Runner · 2 个忙碌",
        runners_en: "3 runners · 2 busy",
    },
    MachineInfo {
        name: "sandbox",
        os_zh: "Ubuntu 24.04 · Forge 上的容器",
        os_en: "Ubuntu 24.04 · Container on Forge",
        cpu: 0.08,
        mem: 0.14,
        disk_zh: "剩余 1.2 TB",
        disk_en: "1.2 TB free",
        session_idx: None,
        runners_zh: "kbox 隔离沙箱就绪",
        runners_en: "kbox container ready",
    },
    MachineInfo {
        name: "Orbit",
        os_zh: "macOS 26 · arm64",
        os_en: "macOS 26 · arm64",
        cpu: 0.33,
        mem: 0.52,
        disk_zh: "剩余 203 GB",
        disk_en: "203 GB free",
        session_idx: Some(1),
        runners_zh: "2 个 Runner · 1 个忙碌",
        runners_en: "2 runners · 1 busy",
    },
    MachineInfo {
        name: "Beacon",
        os_zh: "Ubuntu 24.04 · x64",
        os_en: "Ubuntu 24.04 · x64",
        cpu: 0.29,
        mem: 0.22,
        disk_zh: "剩余 1.6 TB",
        disk_en: "1.6 TB free",
        session_idx: Some(2),
        runners_zh: "1 个 Runner · 1 个忙碌",
        runners_en: "1 runner · 1 busy",
    },
    MachineInfo {
        name: "win-test",
        os_zh: "Windows 11 · Beacon 上的虚拟机",
        os_en: "Windows 11 · VM on Beacon",
        cpu: 0.04,
        mem: 0.38,
        disk_zh: "剩余 38 GB",
        disk_en: "38 GB free",
        session_idx: None,
        runners_zh: "暂无运行项 · 点击启动",
        runners_en: "Idle · click to launch agent",
    },
    MachineInfo {
        name: "Pebble",
        os_zh: "Fedora Linux 42 · x64",
        os_en: "Fedora Linux 42 · x64",
        cpu: 0.02,
        mem: 0.37,
        disk_zh: "剩余 512 GB",
        disk_en: "512 GB free",
        session_idx: None,
        runners_zh: "1 个 Runner · 空闲",
        runners_en: "1 runner · Idle",
    },
];

impl KStudioApp {
    pub(super) fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            view: StudioView::Overview,
            alerts_resolved: [false, false, false],
            session_agent: ["Claude Code", "Codex", "OpenCode"],
            session_logs: [
                vec![
                    "> 修复偶发失败的结账测试".into(),
                    "● 读取(e2e/checkout.spec.ts)  |- 已读取 84 行".into(),
                    "● 更新(src/cart/applyCoupon.ts)  |- 新增 3 行，删除 2 行（税前计算折扣）"
                        .into(),
                    "● Bash(npm test -- e2e/checkout.spec.ts)  |- ✓ 12 项通过 (4.1 秒)".into(),
                ],
                vec![
                    "> 为 iOS 结账页接入 Apple Pay 沙箱回执校验".into(),
                    "● 读取(ios/CheckoutViewModel.swift)  |- 已读取 142 行".into(),
                    "! 等待你确认：执行 xcodebuild test -scheme OrbitApp (KSandbox 已隔离 ~/.ssh)"
                        .into(),
                ],
                vec![
                    "> 为网关补充速率限制响应头 X-RateLimit-Remaining".into(),
                    "● 更新(internal/gateway/ratelimit.go)  |- 新增 19 行".into(),
                    "● Bash(go test ./internal/gateway/...)  |- ✓ PASS (1.8s)".into(),
                ],
            ],
            box_paused: [false, true, false],
        }
    }

    fn breadcrumb(&self, zh: bool) -> String {
        match self.view {
            StudioView::Overview => tr(zh, "概览", "Overview").into(),
            StudioView::Session(0) => {
                tr(zh, "会话 › Checkout fix", "Sessions › Checkout fix").into()
            }
            StudioView::Session(1) => {
                tr(zh, "会话 › Codex · ios-app", "Sessions › Codex · ios-app").into()
            }
            StudioView::Session(_) => {
                tr(zh, "会话 › OpenCode · api", "Sessions › OpenCode · api").into()
            }
            StudioView::RuntimeBoxes => {
                tr(zh, "运行时容器 › kbox MicroVMs", "Runtime › kbox MicroVMs").into()
            }
            StudioView::Runners => {
                tr(zh, "工作负载 › Runner 任务", "Workloads › Runner Jobs").into()
            }
            StudioView::AiUsage => tr(zh, "总览 › AI 用量", "Overview › AI Usage").into(),
        }
    }

    fn render_titlebar(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = if dark { rgb(0x0d0d10) } else { rgb(0xf1f3f6) };
        let border_c = if dark { rgb(0x1f1f24) } else { rgb(0xe2e5ec) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x111827) };
        let muted = if dark { rgb(0x8e8e98) } else { rgb(0x6b7280) };

        div()
            .h(px(34.))
            .w_full()
            .flex_none()
            .bg(bg)
            .border_b_1()
            .border_color(border_c)
            .px(px(12.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(traffic_lights())
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(tr(zh, "开发", "Workspace")),
                    )
                    .child(div().text_color(muted).text_size(px(12.)).child("←  →"))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(ink)
                            .child(self.breadcrumb(zh)),
                    ),
            )
            .child(
                div()
                    .id("kst-title-new")
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(border_c)
                    .text_size(px(11.))
                    .text_color(muted)
                    .hover(|s| s.text_color(ink))
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.view = StudioView::Session(0);
                        cx.notify();
                    }))
                    .child(tr(zh, "+ 新建终端会话", "+ New Session")),
            )
    }

    fn render_activity_bar(&self, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = if dark { rgb(0x09090b) } else { rgb(0xeaecef) };
        let border_c = if dark { rgb(0x1c1c21) } else { rgb(0xdadce2) };
        let act_bg = if dark { rgb(0x232329) } else { rgb(0xffffff) };
        let ink = if dark { rgb(0xf4f4f5) } else { rgb(0x111827) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x6b7280) };

        let items = [
            (
                ">_",
                matches!(
                    self.view,
                    StudioView::Overview | StudioView::Session(_) | StudioView::AiUsage
                ),
                StudioView::Overview,
            ),
            (
                "[ ]",
                matches!(self.view, StudioView::RuntimeBoxes),
                StudioView::RuntimeBoxes,
            ),
            (
                "±",
                matches!(self.view, StudioView::Runners),
                StudioView::Runners,
            ),
        ];

        let mut rail = div()
            .w(px(42.))
            .h_full()
            .flex_none()
            .bg(bg)
            .border_r_1()
            .border_color(border_c)
            .py(px(10.))
            .flex()
            .flex_col()
            .items_center()
            .justify_between();

        let mut top = div().flex().flex_col().items_center().gap(px(8.)).child(
            div()
                .w(px(26.))
                .h(px(26.))
                .rounded(px(6.))
                .bg(ink)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .font_weight(FontWeight::BOLD)
                .text_color(bg)
                .child("K"),
        );

        for (idx, (glyph, active, target)) in items.into_iter().enumerate() {
            top = top.child(
                div()
                    .id(("kst-act", idx))
                    .cursor_pointer()
                    .w(px(30.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .bg(if active { act_bg } else { rgba(0x00000000) })
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_family("Fira Code")
                    .text_size(px(11.))
                    .text_color(if active { ink } else { muted })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.view = target;
                        cx.notify();
                    }))
                    .child(glyph),
            );
        }

        rail = rail.child(top).child(
            div()
                .text_size(px(10.5))
                .font_family("Fira Code")
                .text_color(muted)
                .child("v0.5"),
        );
        rail
    }

    fn render_sidebar(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = if dark { rgb(0x111114) } else { rgb(0xf7f8fa) };
        let border_c = if dark { rgb(0x1e1e24) } else { rgb(0xe2e5ec) };
        let act_bg = if dark { rgb(0x232329) } else { rgb(0xe6e8ee) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x111827) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x4b5563) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x9ca3af) };

        let nav_row = |id_idx: usize,
                       label: &str,
                       badge: Option<&str>,
                       active: bool,
                       target: StudioView,
                       cx: &mut Context<Self>| {
            let mut r = div()
                .id(("kst-sb-nav", id_idx))
                .cursor_pointer()
                .h(px(26.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(if active { act_bg } else { rgba(0x00000000) })
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(11.5))
                .text_color(if active { ink } else { sub })
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.view = target;
                    cx.notify();
                }))
                .child(label.to_owned());
            if let Some(b) = badge {
                r = r.child(
                    div()
                        .px(px(5.))
                        .rounded_full()
                        .bg(if dark { rgb(0x27272a) } else { rgb(0xe5e7eb) })
                        .text_size(px(10.))
                        .text_color(muted)
                        .child(b.to_owned()),
                );
            }
            r
        };

        div()
            .w(px(196.))
            .h_full()
            .flex_none()
            .bg(bg)
            .border_r_1()
            .border_color(border_c)
            .p(px(8.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .px(px(8.))
                            .text_size(px(10.))
                            .text_color(muted)
                            .child(tr(zh, "总览", "Overview")),
                    )
                    .child(nav_row(
                        0,
                        tr(zh, "概览", "Overview"),
                        None,
                        self.view == StudioView::Overview,
                        StudioView::Overview,
                        cx,
                    ))
                    .child(nav_row(
                        1,
                        tr(zh, "AI 用量", "AI Usage"),
                        None,
                        self.view == StudioView::AiUsage,
                        StudioView::AiUsage,
                        cx,
                    ))
                    .child(
                        div()
                            .mt(px(4.))
                            .px(px(8.))
                            .text_size(px(10.))
                            .text_color(muted)
                            .child(tr(zh, "工作负载", "Workloads")),
                    )
                    .child(nav_row(
                        2,
                        tr(zh, "Runner 任务", "Runner Jobs"),
                        Some("6"),
                        self.view == StudioView::Runners,
                        StudioView::Runners,
                        cx,
                    ))
                    .child(nav_row(
                        3,
                        tr(zh, "kbox 微虚拟机", "kbox MicroVMs"),
                        Some("3"),
                        self.view == StudioView::RuntimeBoxes,
                        StudioView::RuntimeBoxes,
                        cx,
                    )),
            )
            .child(
                div()
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .text_color(muted)
                    .child(tr(zh, "搜索机器与会话", "Search"))
                    .child("/"),
            )
            // Authentic Machine Tree (`机器 9`: Forge -> Checkout fix / Orbit -> Codex · ios-app / Beacon -> OpenCode · api)
            .child(
                div()
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
                            .text_color(muted)
                            .child(tr(zh, "机器 9", "Machines 9"))
                            .child("+"),
                    )
                    // Forge
                    .child(
                        div()
                            .px(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0x22c55e)))
                            .child("Forge")
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(muted)
                                    .child("macOS"),
                            ),
                    )
                    .child(
                        div()
                            .id("kst-tree-s0")
                            .debug_selector(|| "kst-tree-s0".into())
                            .cursor_pointer()
                            .ml(px(10.))
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(5.))
                            .bg(if self.view == StudioView::Session(0) {
                                act_bg
                            } else {
                                rgba(0x00000000)
                            })
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(ink)
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.view = StudioView::Session(0);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(div().text_color(rgb(0xf97316)).child("*"))
                                    .child("Checkout fix"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0xf97316))
                                    .child("run"),
                            ),
                    )
                    // Orbit
                    .child(
                        div()
                            .mt(px(2.))
                            .px(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0x22c55e)))
                            .child("Orbit")
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(muted)
                                    .child("arm64"),
                            ),
                    )
                    .child(
                        div()
                            .id("kst-tree-s1")
                            .cursor_pointer()
                            .ml(px(10.))
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(5.))
                            .bg(if self.view == StudioView::Session(1) {
                                act_bg
                            } else {
                                rgba(0x00000000)
                            })
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(ink)
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.view = StudioView::Session(1);
                                cx.notify();
                            }))
                            .child("● Codex · ios-app")
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0xf59e0b))),
                    )
                    // Beacon
                    .child(
                        div()
                            .mt(px(2.))
                            .px(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0x22c55e)))
                            .child("Beacon")
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(muted)
                                    .child("Ubuntu"),
                            ),
                    )
                    .child(
                        div()
                            .id("kst-tree-s2")
                            .cursor_pointer()
                            .ml(px(10.))
                            .px(px(8.))
                            .py(px(3.))
                            .rounded(px(5.))
                            .bg(if self.view == StudioView::Session(2) {
                                act_bg
                            } else {
                                rgba(0x00000000)
                            })
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(ink)
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.view = StudioView::Session(2);
                                cx.notify();
                            }))
                            .child(">_ OpenCode · api")
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x22c55e))
                                    .child("run"),
                            ),
                    ),
            )
    }

    fn render_overview(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let card_bg = if dark { rgb(0x131317) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x222228) } else { rgb(0xe2e5ec) };
        let ink = if dark { rgb(0xfafafa) } else { rgb(0x09090b) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x52525b) };
        let muted = if dark { rgb(0x71717a) } else { rgb(0x9ca3af) };

        let alerts = [
            (
                tr(zh, "forge-03 已离线", "forge-03 is offline"),
                tr(
                    zh,
                    "forge-03 已安装但未运行。",
                    "Runner installed on Forge but stopped.",
                ),
                tr(zh, "启动 Runner", "Start runner"),
            ),
            (
                tr(
                    zh,
                    "“train (small)”找不到可用的 Runner",
                    "“train (small)” waiting for gpu runner",
                ),
                tr(
                    zh,
                    "已在 acme/models 排队 6 分钟 · 缺少 gpu 标签。",
                    "Queued 6m in acme/models · needs gpu label.",
                ),
                tr(zh, "分配 Beacon", "Assign Beacon"),
            ),
            (
                tr(zh, "Vault 的磁盘空间不足", "Vault is low on disk"),
                tr(
                    zh,
                    "总容量 4.0 TB，剩余 41 GB。",
                    "41 GB free of 4.0 TB (_work cache).",
                ),
                tr(zh, "清理缓存", "Prune _work"),
            ),
        ];

        let mut alert_box = div()
            .rounded(px(8.))
            .bg(card_bg)
            .border_1()
            .border_color(border_c)
            .flex()
            .flex_col();

        for (idx, (t, d, btn_lbl)) in alerts.iter().enumerate() {
            let resolved = self.alerts_resolved[idx];
            alert_box = alert_box.child(
                div()
                    .px(px(12.))
                    .py(px(6.))
                    .when(idx > 0, |e| e.border_t_1().border_color(border_c))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(if resolved {
                                        rgb(0x22c55e)
                                    } else {
                                        rgb(0xf59e0b)
                                    })
                                    .child(if resolved { "✓" } else { "!" }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if resolved { muted } else { ink })
                                    .child(*t),
                            )
                            .child(div().text_size(px(11.)).text_color(muted).child(*d)),
                    )
                    .child(
                        div()
                            .id(("kst-alert-btn", idx))
                            .cursor_pointer()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded(px(5.))
                            .border_1()
                            .border_color(border_c)
                            .text_size(px(10.5))
                            .text_color(if resolved { rgb(0x22c55e) } else { sub })
                            .on_click(cx.listener(move |v, _, _, cx| {
                                v.alerts_resolved[idx] = !v.alerts_resolved[idx];
                                cx.notify();
                            }))
                            .child(if resolved {
                                tr(zh, "已处理", "Resolved")
                            } else {
                                *btn_lbl
                            }),
                    ),
            );
        }

        // 2x3 Machine Cards Grid (exact 50-main-overview.png!)
        let mut grid = div().flex().flex_wrap().gap(px(8.));
        for (m_idx, m) in MACHINES.iter().enumerate() {
            let target_session = m.session_idx.unwrap_or(0);
            let session_line: AnyElement = match m.session_idx {
                Some(0) => div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .child(div().text_color(ink).child("* Checkout fix"))
                    .child(
                        div()
                            .text_color(sub)
                            .child(tr(zh, "正在工作...", "Working...")),
                    )
                    .into_any_element(),
                Some(1) => div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .child(div().text_color(ink).child("● Codex · ios-app"))
                    .child(div().text_color(rgb(0xf59e0b)).child(tr(
                        zh,
                        "等待你处理",
                        "Waiting for you",
                    )))
                    .into_any_element(),
                Some(_) => div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .child(div().text_color(ink).child(">_ OpenCode · api"))
                    .child(
                        div()
                            .text_color(sub)
                            .child(tr(zh, "正在工作...", "Working...")),
                    )
                    .into_any_element(),
                None => div()
                    .text_size(px(11.))
                    .text_color(muted)
                    .child(tr(
                        zh,
                        "暂无运行项。点击启动智能体。",
                        "Nothing running. Click to start agent.",
                    ))
                    .into_any_element(),
            };

            let mini_bar = |lbl: &str, pct: f32| {
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_size(px(9.5))
                            .text_color(muted)
                            .child(lbl.to_owned())
                            .child(format!("{}%", (pct * 100.0) as u32)),
                    )
                    .child(
                        div()
                            .h(px(3.))
                            .w_full()
                            .rounded_full()
                            .bg(border_c)
                            .child(div().h_full().w(relative(pct)).rounded_full().bg(sub)),
                    )
            };

            grid = grid.child(
                div()
                    .id(("kst-mcard", m_idx))
                    .cursor_pointer()
                    .w(px(252.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(border_c)
                    .hover(|s| s.border_color(rgb(0x52525b)))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.view = if m_idx == 1 {
                            StudioView::RuntimeBoxes
                        } else {
                            StudioView::Session(target_session)
                        };
                        cx.notify();
                    }))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(ink)
                                            .child(m.name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(muted)
                                            .child(tr(zh, m.os_zh, m.os_en)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(0x22c55e))
                                    .child(tr(zh, "● 在线", "● online")),
                            ),
                    )
                    .child(session_line)
                    .child(div().text_size(px(10.5)).text_color(muted).child(tr(
                        zh,
                        m.runners_zh,
                        m.runners_en,
                    )))
                    .child(
                        div()
                            .pt(px(4.))
                            .border_t_1()
                            .border_color(border_c)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(mini_bar("CPU", m.cpu))
                            .child(mini_bar(tr(zh, "内存", "RAM"), m.mem))
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(muted)
                                    .child(tr(zh, m.disk_zh, m.disk_en)),
                            ),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(ink)
                                    .child(tr(zh, "早上好", "Good afternoon")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(sub)
                                    .child(tr(
                                        zh,
                                        "9 台机器 · 8 台在线 · 3 个智能体正在工作 · 1 项等待你处理 · 点击任意机器或左侧会话进入终端",
                                        "9 machines · 8 online · 3 agents working · 1 waiting for you · click any machine or session",
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .id("kst-hero-new")
                            .cursor_pointer()
                            .px(px(11.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .bg(ink)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if dark { rgb(0x09090b) } else { rgb(0xffffff) })
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.view = StudioView::Session(0);
                                cx.notify();
                            }))
                            .child(tr(zh, "新建会话", "New session")),
                    ),
            )
            .child(alert_box)
            .child(grid)
    }

    // Authentic `62-entity-session.png` live terminal view!
    fn render_session(
        &self,
        idx: usize,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let border_c = if dark { rgb(0x232329) } else { rgb(0xe2e5ec) };
        let term_bg = if dark { rgb(0x0b0b0d) } else { rgb(0xffffff) };
        let ink = if dark { rgb(0xf4f4f6) } else { rgb(0x111827) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x4b5563) };

        let (title, machine, cwd) = match idx {
            0 => ("Checkout fix", "Forge", "/Users/ci/code/checkout"),
            1 => ("Codex · ios-app", "Orbit", "/Users/dev/ios-app"),
            _ => ("OpenCode · api", "Beacon", "/srv/kense/gateway"),
        };
        let agent = self.session_agent[idx];

        let mut lines = div().flex().flex_col().gap(px(6.));
        for ln in &self.session_logs[idx] {
            let s = ln.as_ref();
            let color = if s.starts_with('!') {
                rgb(0xf59e0b)
            } else if s.contains('✓') {
                rgb(0x22c55e)
            } else if s.starts_with('>') {
                ink
            } else {
                sub
            };
            lines = lines.child(
                div()
                    .font_family("Fira Code")
                    .text_size(px(11.5))
                    .text_color(color)
                    .child(ln.clone()),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            // Header (`Checkout fix · Claude Code · Forge`) + `[交接]` `[返回概览]`
            .child(
                div()
                    .pb(px(8.))
                    .border_b_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(28.))
                                    .h(px(28.))
                                    .rounded(px(7.))
                                    .bg(rgba(0xf9731624))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xf97316))
                                    .child("*"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(ink)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .font_family("Fira Code")
                                            .text_size(px(11.))
                                            .text_color(sub)
                                            .child(format!("{agent} · {machine} · {cwd}")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .id("kst-handoff")
                                    .debug_selector(|| "kst-handoff".into())
                                    .cursor_pointer()
                                    .px(px(9.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(border_c)
                                    .text_size(px(11.))
                                    .text_color(ink)
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        let next = if v.session_agent[idx] == "Claude Code" {
                                            "Codex"
                                        } else {
                                            "Claude Code"
                                        };
                                        v.session_agent[idx] = next;
                                        v.session_logs[idx].push(
                                            format!("✓ 会话上下文已无损交接至 {next} (保留工作目录与终端现场)").into(),
                                        );
                                        cx.notify();
                                    }))
                                    .child(tr(zh, "<-> 跨智能体交接", "<-> Handoff Agent")),
                            )
                            .child(
                                div()
                                    .id("kst-back-overview")
                                    .cursor_pointer()
                                    .px(px(9.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .bg(if dark { rgb(0x232329) } else { rgb(0xe5e7eb) })
                                    .text_size(px(11.))
                                    .text_color(ink)
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.view = StudioView::Overview;
                                        cx.notify();
                                    }))
                                    .child(tr(zh, "← 返回机器概览", "← Overview")),
                            ),
                    ),
            )
            // Orange Claude Code / Agent welcome box (`62-entity-session.png`)
            .child(
                div()
                    .w(px(420.))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgba(0xf9731688))
                    .bg(term_bg)
                    .font_family("Fira Code")
                    .text_size(px(11.5))
                    .child(
                        div()
                            .text_color(ink)
                            .child(format!("* {} {agent}!", tr(zh, "欢迎使用", "Welcome to"))),
                    )
                    .child(div().text_color(sub).child(format!("  cwd: {cwd}"))),
            )
            .child(lines)
            // Interactive terminal command bar
            .child(
                div()
                    .mt(px(4.))
                    .p(px(9.))
                    .rounded(px(6.))
                    .bg(term_bg)
                    .border_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .font_family("Fira Code")
                            .text_size(px(11.5))
                            .text_color(rgb(0xf97316))
                            .child(tr(zh, "* 正在等待指令（点击右侧按钮直接向远端 PTY 发送）：", "* Ready on remote PTY — click to execute:")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .id("kst-cmd-git")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(border_c)
                                    .font_family("Fira Code")
                                    .text_size(px(11.))
                                    .text_color(ink)
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        v.session_logs[idx].push(
                                            "● Bash(git diff --stat)  |- 2 files changed, 22 insertions(+), 2 deletions(-)".into(),
                                        );
                                        cx.notify();
                                    }))
                                    .child("git diff --stat"),
                            )
                            .child(
                                div()
                                    .id("kst-cmd-approve")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded(px(5.))
                                    .bg(rgb(0x16a34a))
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        v.session_logs[idx].push(
                                            "✓ 已放行远端测试套件 · 38/38 specs passed (KSandbox 零越权访问)".into(),
                                        );
                                        cx.notify();
                                    }))
                                    .child(tr(zh, "✓ 批准并跑测试", "✓ Approve & Run")),
                            ),
                    ),
            )
    }

    fn render_boxes_or_aux(
        &self,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let card_bg = if dark { rgb(0x131317) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x222228) } else { rgb(0xe2e5ec) };
        let ink = if dark { rgb(0xfafafa) } else { rgb(0x09090b) };
        let sub = if dark { rgb(0xa1a1aa) } else { rgb(0x52525b) };

        let boxes = [
            (
                "box-checkout-pr48",
                "Forge · libkrun MicroVM · 4 vCPU · 8 GiB",
                "19ms warm start",
            ),
            (
                "box-cuda-eval",
                "Beacon · OCI + KSandbox · deny-all net",
                "Idle auto-paused",
            ),
            (
                "box-docs-audit",
                "Orbit · libkrun MicroVM · 2 vCPU · 4 GiB",
                "Running Claude Code",
            ),
        ];

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(ink)
                    .child(match self.view {
                        StudioView::RuntimeBoxes => tr(
                            zh,
                            "kbox 轻量微虚拟机与容器 (Runtime Container)",
                            "kbox MicroVM Boxes & Containers",
                        ),
                        StudioView::Runners => tr(
                            zh,
                            "自建 GitHub Runner 队列 (8 台在线)",
                            "Self-Hosted GitHub Runners (8 Online)",
                        ),
                        _ => tr(
                            zh,
                            "跨 18 种编程智能体令牌与缓存统计",
                            "18-Agent Token & Prompt-Cache Telemetry",
                        ),
                    }),
            )
            .children(boxes.iter().enumerate().map(|(i, (name, spec, state))| {
                let paused = self.box_paused[i];
                div()
                    .p(px(11.))
                    .rounded(px(8.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(ink)
                                    .child(*name),
                            )
                            .child(div().text_size(px(11.)).text_color(sub).child(*spec)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if paused { rgb(0xf59e0b) } else { rgb(0x22c55e) })
                                    .child(if paused {
                                        tr(zh, "● 已休眠快照", "● Paused")
                                    } else {
                                        *state
                                    }),
                            )
                            .child(
                                div()
                                    .id(("kst-box-toggle", i))
                                    .cursor_pointer()
                                    .px(px(9.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(border_c)
                                    .text_size(px(11.))
                                    .text_color(ink)
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        v.box_paused[i] = !v.box_paused[i];
                                        cx.notify();
                                    }))
                                    .child(if paused {
                                        tr(zh, "19ms 唤醒", "Resume 19ms")
                                    } else {
                                        tr(zh, "暂停快照", "Pause Box")
                                    }),
                            ),
                    )
            }))
    }
}

impl Render for KStudioApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let zh = is_zh(cx);
        let dark = is_dark(cx);
        let stage_bg = if dark { rgb(0x09090b) } else { rgb(0xffffff) };

        let stage: AnyElement = match self.view {
            StudioView::Overview => self.render_overview(zh, dark, cx).into_any_element(),
            StudioView::Session(idx) => self.render_session(idx, zh, dark, cx).into_any_element(),
            _ => self.render_boxes_or_aux(zh, dark, cx).into_any_element(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(stage_bg)
            .child(self.render_titlebar(zh, dark, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.render_activity_bar(dark, cx))
                    .child(self.render_sidebar(zh, dark, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .id("kst-stage-scroll")
                            .overflow_y_scroll()
                            .p(px(14.))
                            .child(stage),
                    ),
            )
    }
}
