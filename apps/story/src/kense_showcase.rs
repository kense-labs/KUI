//! Interactive GPUI Studio showcases embedded on the Kense homepage workbench.

use gpui::{prelude::*, *};
use kui::{
    components::{
        button::Button,
        glass_segmented::{GlassSegmentChanged, GlassSegmentOption, GlassSegmented},
        switch::Switch,
    },
    foundation::Theme,
    primitives::{
        Size,
        control::UiLanguage,
        family::{Paint, Slide, disabled_motion},
    },
};
use std::time::Duration;

pub const IDS: &[&str] = &[
    "kense-gateway",
    "kense-sandbox",
    "kense-workbench",
    "kense-klang",
];

/// Instantiate a Kense homepage GPUI showcase scene when `id` matches [`IDS`].
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
    let inner: AnyView = cx.new(|cx| Scene::new(kind, w, cx)).into();
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
        let edge = |i| {
            div()
                .id(("kense-edge", i))
                .track_focus(&self.edges[i])
                .tab_stop(true)
                .size(px(0.))
        };
        let bridge = cx.try_global::<crate::StoryBridge>();
        let embedded = bridge.is_some();
        let min_height = bridge.map_or(508., |b| 508_f32.max((b.min_height)()));
        let content = div()
            .w_full()
            .min_h(px(min_height))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(if embedded { 22. } else { 18. }))
            .bg(paint.stage)
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
            .overflow_y_scroll()
            .font_family("KUI Preview Latin")
            .text_color(paint.ink)
            .text_size(px(14.))
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

struct Scene {
    kind: Kind,
    segmented: Entity<GlassSegmented>,
    toggle: Entity<Switch>,
    mode_index: usize,
    flag_on: bool,
    phase: usize,
    running: bool,
    blocked_alert: bool,
    awaiting_approval: bool,
    runs: u32,
    progress: Slide,
    _subs: Vec<Subscription>,
}

impl Scene {
    fn build_controls(
        kind: Kind,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<GlassSegmented>, Entity<Switch>, Vec<Subscription>) {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let options = match kind {
            Kind::Gateway => vec![
                GlassSegmentOption::new("llm", "NVIDIA H800"),
                GlassSegmentOption::new("mcp", "Ascend 910B"),
                GlassSegmentOption::new("a2a", "Apple M3 Ultra"),
            ],
            Kind::Sandbox => vec![
                GlassSegmentOption::new("sandbox", "Mac Studio"),
                GlassSegmentOption::new("microvm", "Linux Box"),
                GlassSegmentOption::new("pool", t("Cloud VM", "云端主机")),
            ],
            Kind::Workbench => vec![
                GlassSegmentOption::new("low", "Flash"),
                GlassSegmentOption::new("high", "V4"),
                GlassSegmentOption::new("max", "Reasoner"),
            ],
            Kind::Klang => vec![
                GlassSegmentOption::new("native", "Native C"),
                GlassSegmentOption::new("wasi", "wasm32-wasip1"),
                GlassSegmentOption::new("tinycc", "TinyCC JIT"),
            ],
        };
        let seg_label = match kind {
            Kind::Klang => t("Target", "编译目标"),
            _ => t("Target", "目标节点"),
        };
        let segmented = cx.new(|cx| GlassSegmented::new(seg_label, options, cx).with_small(true));
        let toggle_label = match kind {
            Kind::Gateway => t("dGPU memory quota", "开启单卡 dGPU 显存切分"),
            Kind::Sandbox => t("Keep session on close", "关闭窗口后后台常驻"),
            Kind::Workbench => t("Require single approval", "敏感命令需单次确认"),
            Kind::Klang => t("Prove checkfree bounds", "强制 checkfree 边界消除证明"),
        };
        let toggle =
            cx.new(|cx| Switch::new("kense-policy-switch", toggle_label, w, cx).with_checked(true));

        let seg_sub = cx.subscribe(&segmented, |this, _, ev: &GlassSegmentChanged, cx| {
            this.mode_index = match ev.0.as_ref() {
                "mcp" | "microvm" | "high" | "wasi" => 1,
                "a2a" | "pool" | "max" | "tinycc" => 2,
                _ => 0,
            };
            this.blocked_alert = false;
            cx.notify();
        });
        let sw_sub = cx.subscribe(&toggle, |this, _, checked: &bool, cx| {
            this.flag_on = *checked;
            cx.notify();
        });
        (segmented, toggle, vec![seg_sub, sw_sub])
    }

    fn new(kind: Kind, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let (segmented, toggle, subs) = Self::build_controls(kind, w, cx);
        Self {
            kind,
            segmented,
            toggle,
            mode_index: 0,
            flag_on: true,
            phase: 3,
            running: false,
            blocked_alert: false,
            awaiting_approval: false,
            runs: 1,
            progress: Slide::new(1.),
            _subs: subs,
        }
    }

    fn trigger_run(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        self.blocked_alert = false;
        self.awaiting_approval = false;
        self.runs = self.runs.wrapping_add(1);
        if disabled_motion(cx) {
            self.phase = if self.kind == Kind::Workbench { 2 } else { 3 };
            self.awaiting_approval = self.kind == Kind::Workbench;
            self.progress
                .reset(if self.awaiting_approval { 0.75 } else { 1. });
            cx.notify();
            return;
        }
        self.running = true;
        self.phase = 0;
        self.progress.reset(0.25);
        cx.notify();

        let pause_for_approval = self.kind == Kind::Workbench;
        cx.spawn(async move |weak, cx| {
            for next in 1..=3 {
                cx.background_executor()
                    .timer(Duration::from_millis(270))
                    .await;
                let stop = weak
                    .update(cx, |this, cx| {
                        if pause_for_approval && next == 3 {
                            this.running = false;
                            this.awaiting_approval = true;
                            this.phase = 2;
                            this.progress.duration(0.76, 0.24, cx);
                            cx.notify();
                            return true;
                        }
                        this.phase = next;
                        this.progress.duration((next as f32 + 1.) / 4., 0.24, cx);
                        if next == 3 {
                            this.running = false;
                        }
                        cx.notify();
                        false
                    })
                    .unwrap_or(true);
                if stop {
                    break;
                }
            }
        })
        .detach();
    }

    fn secondary_action(&mut self, cx: &mut Context<Self>) {
        match self.kind {
            Kind::Sandbox | Kind::Klang => {
                self.blocked_alert = true;
                self.running = false;
                self.phase = 2;
                self.progress.duration(0.75, 0.2, cx);
                cx.notify();
            }
            Kind::Workbench if self.awaiting_approval => {
                self.awaiting_approval = false;
                self.phase = 3;
                self.progress.duration(1., 0.22, cx);
                cx.notify();
            }
            _ => {
                self.blocked_alert = false;
                self.phase = (self.phase + 1) % 4;
                self.progress
                    .duration((self.phase as f32 + 1.) / 4., 0.22, cx);
                cx.notify();
            }
        }
    }

    fn step_titles(&self, lang: UiLanguage) -> [&'static str; 4] {
        let zh = lang == UiLanguage::Chinese;
        match self.kind {
            Kind::Gateway => [
                if zh { "显存估算" } else { "VRAM math" },
                if zh { "异构选卡" } else { "Pick GPU" },
                if zh { "启动实例" } else { "Start instance" },
                if zh { "/v1 就绪" } else { "/v1 ready" },
            ],
            Kind::Sandbox => [
                if zh { "SSH 连通" } else { "SSH link" },
                if zh { "分配环境" } else { "Clean box" },
                if zh { "运行任务" } else { "Execute" },
                if zh { "后台保活" } else { "Keep alive" },
            ],
            Kind::Workbench => [
                if zh { "选择工作区" } else { "Workspace" },
                if zh { "生成方案" } else { "Plan" },
                if zh { "单次确认" } else { "Approval" },
                if zh { "保存历史" } else { "Saved" },
            ],
            Kind::Klang => [
                if zh { "解析 .k" } else { "Parse .k" },
                if zh { "生命周期" } else { "Lifetimes" },
                if zh { "边界证明" } else { "Bounds BCE" },
                if zh { "产物就绪" } else { "Emitted" },
            ],
        }
    }

    fn detail_rows(&self, lang: UiLanguage) -> [(&'static str, String); 3] {
        let zh = lang == UiLanguage::Chinese;
        let _digest = (self.runs * 19 + 0x4a) & 0xff;
        match self.kind {
            Kind::Gateway => {
                let (spec, engine) = match self.mode_index {
                    0 => (
                        "weights 41.2 GB + kv 14.8 GB / 80 GB",
                        "vLLM · NIXL · node-04",
                    ),
                    1 => (
                        "weights 38.4 GB + kv 12.0 GB / 64 GB",
                        "vLLM-Ascend · Mooncake",
                    ),
                    _ => (
                        "weights 22.6 GB + kv 8.4 GB / 192 GB",
                        "mlx-engine · studio-m3",
                    ),
                };
                let dgpu = if self.flag_on {
                    if zh {
                        "dGPU 配额已启用 · 单卡多模型并存"
                    } else {
                        "dGPU quota active · multi-model card"
                    }
                } else if zh {
                    "整卡独占分配"
                } else {
                    "dedicated full card"
                };
                [
                    (
                        if zh {
                            "启动前显存预算"
                        } else {
                            "VRAM estimate"
                        },
                        spec.into(),
                    ),
                    (
                        if zh {
                            "选中节点与实例"
                        } else {
                            "Selected GPU"
                        },
                        engine.into(),
                    ),
                    (
                        if zh {
                            "显存共享与网关"
                        } else {
                            "Gateway & quota"
                        },
                        format!("{dgpu} · /v1 #{:02}", self.runs),
                    ),
                ]
            }
            Kind::Sandbox => {
                let host = match (zh, self.mode_index) {
                    (true, 0) => "desk-studio.local · 24 核 / 空闲 142 GB",
                    (true, 1) => "build-box-02 · Ubuntu 24.04 · MicroVM",
                    (true, _) => "cn-hz-worker-07 · Tailscale 直连",
                    (false, 0) => "desk-studio.local · 24 cores / 142 GB free",
                    (false, 1) => "build-box-02 · Ubuntu 24.04 · MicroVM",
                    (false, _) => "cn-hz-worker-07 · via Tailscale",
                };
                let boundary = if self.blocked_alert {
                    if zh {
                        "读取宿主机 ~/.ssh -> 已被沙箱阻断 (EPERM)"
                    } else {
                        "read host ~/.ssh -> blocked by sandbox (EPERM)"
                    }
                } else if self.flag_on {
                    if zh {
                        "KBox / KSandbox 隔离中 · 关窗后会话继续运行"
                    } else {
                        "isolated box · keeps running when window closes"
                    }
                } else if zh {
                    "窗口关闭时自动结束会话"
                } else {
                    "ends when window closes"
                };
                [
                    (
                        if zh {
                            "目标机器状态"
                        } else {
                            "Target machine"
                        },
                        host.into(),
                    ),
                    (
                        if zh {
                            "运行边界与保活"
                        } else {
                            "Isolation & tray"
                        },
                        boundary.into(),
                    ),
                    (
                        if zh {
                            "已托管任务"
                        } else {
                            "Active session"
                        },
                        if zh {
                            format!("claude-code + 后台终端运行中 · #{:02}", self.runs)
                        } else {
                            format!("claude-code + persistent pty · #{:02}", self.runs)
                        },
                    ),
                ]
            }
            Kind::Workbench => {
                let model = ["deepseek-flash", "deepseek-v4", "deepseek-reasoner"][self.mode_index];
                let step = if self.awaiting_approval {
                    if zh {
                        "等待你确认执行：cargo test --workspace".into()
                    } else {
                        "awaiting approval: cargo test --workspace".into()
                    }
                } else {
                    format!("{model} · tools ready")
                };
                [
                    (
                        if zh {
                            "当前项目工作区"
                        } else {
                            "Workspace"
                        },
                        "~/Projects/billing-service".into(),
                    ),
                    (
                        if zh {
                            "模型与工具状态"
                        } else {
                            "Model & tools"
                        },
                        step,
                    ),
                    (
                        if zh {
                            "历史与定时任务"
                        } else {
                            "History & schedule"
                        },
                        if zh {
                            format!("本地历史已保存 · 每日 09:00 巡检就绪 (#{:02})", self.runs)
                        } else {
                            format!("saved locally · 09:00 daily run armed (#{:02})", self.runs)
                        },
                    ),
                ]
            }
            Kind::Klang => {
                let target = match (zh, self.mode_index) {
                    (true, 0) => "k build -O3 · C 后端 + 工作线程队列",
                    (true, 1) => "k build --target wasm32-wasip1 · Wasmtime",
                    (true, _) => "k run gateway.k · TinyCC 进程内直跑",
                    (false, 0) => "k build -O3 · C backend + worker queues",
                    (false, 1) => "k build --target wasm32-wasip1 · Wasmtime",
                    (false, _) => "k run gateway.k · in-process TinyCC",
                };
                let check = if self.blocked_alert {
                    if zh {
                        "返回局部数据栈引用 -> 编译期拒绝 (lifetime escape)"
                    } else {
                        "return local stack ref -> rejected (lifetime escape)"
                    }
                } else if self.flag_on {
                    if zh {
                        "数据栈作用域回收 · checkfree 零越界检查"
                    } else {
                        "scope-exit data stack · checkfree zero bounds checks"
                    }
                } else if zh {
                    "数据栈作用域回收 · 保留动态边界检查"
                } else {
                    "scope-exit data stack · runtime bounds checks kept"
                };
                [
                    (
                        if zh {
                            "编译目标与后端"
                        } else {
                            "Target & backend"
                        },
                        target.into(),
                    ),
                    (
                        if zh {
                            "生命周期与边界"
                        } else {
                            "Lifetimes & BCE"
                        },
                        check.into(),
                    ),
                    (
                        if zh {
                            "Cordis 服务锁"
                        } else {
                            "Cordis lock"
                        },
                        if zh {
                            format!("cordis.lock 已验证 · onnx-gateway #{:02}", self.runs)
                        } else {
                            format!("cordis.lock verified · onnx-gateway #{:02}", self.runs)
                        },
                    ),
                ]
            }
        }
    }
}

impl Render for Scene {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let narrow = w.viewport_size().width < px(480.);
        let bar = self.progress.get(w, cx).clamp(0., 1.);

        let (intent_prompt, header_title, subtitle) = match self.kind {
            Kind::Gateway => (
                t(
                    "Deploy Qwen3-72B · estimate VRAM & pick GPU",
                    "部署 Qwen3-72B · 估算显存并自动选卡",
                ),
                t(
                    "KStack · GPU Scheduler & MaaS",
                    "KStack · 异构选卡与模型服务",
                ),
                "kstack /v1/chat/completions",
            ),
            Kind::Sandbox => (
                t(
                    "Hand issue #142 to agent on Linux box",
                    "把 Issue #142 交给远端机器上的 Agent 处理",
                ),
                t(
                    "KStudio · Machine & Agent Hub",
                    "KStudio · 多机与 Agent 控制台",
                ),
                "ssh + kbox microvm",
            ),
            Kind::Workbench => (
                t(
                    "Run workspace test suite & summarize diff",
                    "运行当前工作区测试并汇总改动",
                ),
                t(
                    "KWork · Desktop Agent Workbench",
                    "KWork · 智能体桌面工作台",
                ),
                "kwork + kui gpui",
            ),
            Kind::Klang => (
                t(
                    "k build gateway.k · check lifetimes & Cordis lock",
                    "k build gateway.k · 检查引用生命周期与 Cordis 锁",
                ),
                t(
                    "K · Systems Compiler & Cordis",
                    "K 语言 · 系统编译器与 Cordis 微内核",
                ),
                "k v0.1 · .k -> C / WASI / TinyCC",
            ),
        };

        let status_badge = if self.blocked_alert {
            (
                match self.kind {
                    Kind::Klang => t("Rejected", "编译期拒绝"),
                    _ => t("Blocked", "越界已阻断"),
                },
                p.danger,
            )
        } else if self.awaiting_approval {
            (t("Approval", "等待确认"), p.bright)
        } else if self.running {
            (t("Running", "执行中"), p.bright)
        } else {
            (t("Ready", "运行正常"), p.success)
        };

        let primary_label = match self.kind {
            Kind::Gateway => t("Deploy model", "一键部署模型"),
            Kind::Sandbox => t("Dispatch to machine", "派发到该机器"),
            Kind::Workbench => t("Run task", "发送任务"),
            Kind::Klang => t("Compile & check", "编译并检查"),
        };
        let secondary_label = match self.kind {
            Kind::Sandbox => t("Test path block", "模拟敏感目录阻断"),
            Kind::Klang => t("Test ref escape", "模拟悬垂引用检查"),
            Kind::Workbench if self.awaiting_approval => t("Approve once", "批准本次执行"),
            _ => t("Next step", "切换步骤"),
        };

        let run_entity = cx.entity();
        let primary_btn = Button::new("kense-primary-run")
            .primary()
            .with_size(Size::Sm)
            .loading(self.running)
            .label(primary_label)
            .on_click(move |_, _, cx| {
                run_entity.update(cx, |this, cx| this.trigger_run(cx));
            })
            .into_themed(cx);

        let sec_entity = cx.entity();
        let secondary_btn = Button::new("kense-secondary-act")
            .secondary()
            .with_size(Size::Sm)
            .disabled(self.running)
            .label(secondary_label)
            .on_click(move |_, _, cx| {
                sec_entity.update(cx, |this, cx| this.secondary_action(cx));
            })
            .into_themed(cx);

        // Clean 4-step horizontal pill strip (zero uppercase 01..04 serials)
        let steps = self.step_titles(lang);
        let mut step_strip = div()
            .w_full()
            .rounded(px(12.))
            .bg(p.stage.opacity(0.75))
            .p(px(5.))
            .grid()
            .gap(px(4.))
            .when(narrow, |d| d.grid_cols(2))
            .when(!narrow, |d| d.grid_cols(4));

        for (idx, label) in steps.into_iter().enumerate() {
            let active = idx == self.phase;
            let passed = idx <= self.phase;
            let alert = self.blocked_alert && idx == 2;
            let dot = if alert {
                p.danger
            } else if passed {
                status_badge.1
            } else {
                p.faint.opacity(0.35)
            };
            let step_entity = cx.entity();
            step_strip = step_strip.child(
                div()
                    .id(("kense-step-pill", idx))
                    .cursor_pointer()
                    .rounded(px(9.))
                    .px(px(8.))
                    .py(px(6.))
                    .bg(if active {
                        p.panel
                    } else {
                        gpui::transparent_black()
                    })
                    .when(active, |d| d.border_1().border_color(p.line.opacity(0.65)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .on_click(move |_, _, cx| {
                        step_entity.update(cx, |this, cx| {
                            if !this.running {
                                this.blocked_alert = false;
                                this.phase = idx;
                                this.progress.duration((idx as f32 + 1.) / 4., 0.2, cx);
                                cx.notify();
                            }
                        });
                    })
                    .child(div().size(px(6.)).rounded(px(999.)).bg(dot))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(if active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if alert {
                                p.danger
                            } else if active {
                                p.ink
                            } else {
                                p.secondary
                            })
                            .truncate()
                            .child(label),
                    ),
            );
        }

        // Serene Apple-Wallet / Alipay-Sheet hairline key-value rows
        let rows = self.detail_rows(lang);
        let mut spec_list = div()
            .w_full()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(p.line.opacity(0.5));

        for (idx, (k_label, v_text)) in rows.into_iter().enumerate() {
            let alert_row = self.blocked_alert && idx == 1;
            let wait_row = self.awaiting_approval && idx == 1;
            spec_list = spec_list.child(
                div()
                    .py(px(11.))
                    .border_b_1()
                    .border_color(p.line.opacity(0.5))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.secondary)
                            .child(k_label),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_family("Fira Code")
                            .text_color(if alert_row {
                                p.danger
                            } else if wait_row {
                                p.bright
                            } else {
                                p.ink
                            })
                            .child(v_text),
                    ),
            );
        }

        let sheet = div()
            .w_full()
            .rounded(px(18.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line.opacity(0.65))
            .p(px(if narrow { 16. } else { 22. }))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(11.))
                            .child(
                                div()
                                    .size(px(34.))
                                    .rounded(px(10.))
                                    .bg(status_badge.1.opacity(0.12))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div().size(px(10.)).rounded(px(999.)).bg(status_badge.1),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.))
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(header_title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .font_family("Fira Code")
                                            .text_color(p.faint)
                                            .child(subtitle),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(4.))
                            .rounded(px(999.))
                            .bg(status_badge.1.opacity(0.12))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(status_badge.1)
                            .child(status_badge.0),
                    ),
            )
            .child(self.segmented.clone())
            .child(step_strip)
            .child(
                div()
                    .w_full()
                    .h(px(2.))
                    .rounded(px(999.))
                    .bg(p.stage)
                    .overflow_hidden()
                    .child(div().h_full().rounded(px(999.)).w(relative(bar)).bg(
                        if self.blocked_alert {
                            p.danger
                        } else {
                            status_badge.1
                        },
                    )),
            )
            .child(spec_list)
            .child(
                div()
                    .pt(px(2.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(self.toggle.clone())
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .child(secondary_btn)
                            .child(primary_btn),
                    ),
            );

        // Conversational intent bubble above the native verification sheet
        div()
            .w_full()
            .max_w(px(560.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div().w_full().flex().justify_end().child(
                    div()
                        .px(px(14.))
                        .py(px(7.))
                        .rounded(px(999.))
                        .bg(p.bright)
                        .text_color(p.inverse)
                        .text_size(px(12.5))
                        .child(intent_prompt),
                ),
            )
            .child(sheet)
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
