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

pub const IDS: &[&str] = &["kense-gateway", "kense-sandbox", "kense-workbench"];

/// Instantiate a Kense homepage GPUI showcase scene when `id` matches [`IDS`].
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    if !IDS.contains(&id) {
        return None;
    }
    crate::ensure_story_fonts(cx);
    let kind = match id {
        "kense-sandbox" => Kind::Sandbox,
        "kense-workbench" => Kind::Workbench,
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
                GlassSegmentOption::new("llm", "LLM /v1"),
                GlassSegmentOption::new("mcp", "MCP Tool"),
                GlassSegmentOption::new("a2a", "A2A Agent"),
            ],
            Kind::Sandbox => vec![
                GlassSegmentOption::new("sandbox", "Sandbox"),
                GlassSegmentOption::new("microvm", "MicroVM"),
                GlassSegmentOption::new("pool", t("Prewarmed", "预热池")),
            ],
            Kind::Workbench => vec![
                GlassSegmentOption::new("low", "Flash"),
                GlassSegmentOption::new("high", "V4"),
                GlassSegmentOption::new("max", "Max"),
            ],
        };
        let segmented = cx.new(|cx| {
            GlassSegmented::new(t("Execution route", "执行路由"), options, cx).with_small(true)
        });
        let toggle_label = match kind {
            Kind::Gateway => t("Vault egress injection", "Vault 密文引用出网"),
            Kind::Sandbox => t("Fail-closed boundary", "失败闭合内核边界"),
            Kind::Workbench => t("Closed KCL schema", "KCL 闭合 Schema 准入"),
        };
        let toggle =
            cx.new(|cx| Switch::new("kense-policy-switch", toggle_label, w, cx).with_checked(true));

        let seg_sub = cx.subscribe(&segmented, |this, _, ev: &GlassSegmentChanged, cx| {
            this.mode_index = match ev.0.as_ref() {
                "mcp" | "microvm" | "high" => 1,
                "a2a" | "pool" | "max" => 2,
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
            Kind::Sandbox => {
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
                if zh { "主 Key 验签" } else { "Verify key" },
                if zh { "换发 SVID" } else { "Mint SVID" },
                if zh { "零凭据出网" } else { "Zero-secret" },
                if zh { "哈希锚定" } else { "Anchored" },
            ],
            Kind::Sandbox => [
                if zh { "内核探测" } else { "OS probe" },
                if zh { "隔离挂载" } else { "Isolation" },
                if zh { "有界工作区" } else { "Workspace" },
                if zh { "回收补池" } else { "Replenish" },
            ],
            Kind::Workbench => [
                if zh { "Schema 准入" } else { "KCL schema" },
                if zh { "同源渲染" } else { "GPUI view" },
                if zh { "单次授权" } else { "Approval" },
                if zh { "状态归档" } else { "Persisted" },
            ],
        }
    }

    fn detail_rows(&self, lang: UiLanguage) -> [(&'static str, String); 3] {
        let zh = lang == UiLanguage::Chinese;
        let digest = (self.runs * 19 + 0x4a) & 0xff;
        match self.kind {
            Kind::Gateway => {
                let aud = ["llm/v1", "mcp/stdio", "a2a/mesh"][self.mode_index];
                let egress = if self.flag_on {
                    if zh {
                        "secret_ref@v4 · 沙箱明文=0".into()
                    } else {
                        "secret_ref@v4 · 0 plaintext".into()
                    }
                } else if zh {
                    "出网侧隔离".into()
                } else {
                    "isolated egress".into()
                };
                [
                    (
                        if zh {
                            "换发短时 SVID"
                        } else {
                            "Scoped SVID"
                        },
                        format!("RFC 8693 · {aud} · 600s"),
                    ),
                    (
                        if zh {
                            "零凭据执行与注入"
                        } else {
                            "Vault egress"
                        },
                        egress,
                    ),
                    (
                        if zh {
                            "计量与哈希锚定"
                        } else {
                            "Audit chain"
                        },
                        format!("1 event · sha256:4e8a{digest:02x}"),
                    ),
                ]
            }
            Kind::Sandbox => {
                let engine = match (zh, self.mode_index) {
                    (true, 0) => "内置 Sandbox · 28ms 启动 / 6ms 执行",
                    (true, 1) => "libkrun MicroVM · HVF / KVM",
                    (true, _) => "预热池极速挂载 · 19.3ms RTT",
                    (false, 0) => "In-tree Sandbox · 28ms start / 6ms exec",
                    (false, 1) => "libkrun MicroVM · HVF / KVM",
                    (false, _) => "Prewarmed pool · 19.3ms RTT",
                };
                let boundary = if self.blocked_alert {
                    if zh {
                        "~/.ssh 与外网套接字 -> 内核拒绝 (EPERM)"
                    } else {
                        "~/.ssh & outbound socket -> denied (EPERM)"
                    }
                } else if self.flag_on {
                    "Seatbelt / seccomp · cap ≤ 100 KiB"
                } else {
                    "probe required"
                };
                [
                    (
                        if zh {
                            "隔离运行时"
                        } else {
                            "Isolation tier"
                        },
                        engine.into(),
                    ),
                    (
                        if zh {
                            "有界工作区与阻断"
                        } else {
                            "Process boundary"
                        },
                        boundary.into(),
                    ),
                    (
                        if zh {
                            "退出回收与补池"
                        } else {
                            "Lifecycle"
                        },
                        if zh {
                            format!("后代进程树已完整回收 · #{:02}", self.runs)
                        } else {
                            format!("descendant tree reaped · #{:02}", self.runs)
                        },
                    ),
                ]
            }
            Kind::Workbench => {
                let model = ["deepseek-flash", "deepseek-v4", "deepseek-reasoner"][self.mode_index];
                let acp = if self.awaiting_approval {
                    if zh {
                        "请求单次授权：ksandbox exec -- cargo test".into()
                    } else {
                        "awaiting approval: ksandbox exec -- cargo test".into()
                    }
                } else {
                    format!("stdio json-rpc · {model}")
                };
                [
                    (
                        if zh {
                            "闭合 Schema 准入"
                        } else {
                            "KCL admission"
                        },
                        format!("sha256:7c21{digest:02x} · ≤ 1 MiB"),
                    ),
                    (
                        if zh {
                            "桌面与 Web 同源渲染"
                        } else {
                            "ACP session"
                        },
                        acp,
                    ),
                    (
                        if zh {
                            "授权执行与归档"
                        } else {
                            "State archive"
                        },
                        if zh {
                            "会话轨迹已归档至 DSH_HOME · 支持冷恢复".into()
                        } else {
                            "persisted to DSH_HOME · cold resume ready".into()
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
                    "Dispatch gateway request · zero-secret egress",
                    "发起网关请求 · 沙箱零明文出网",
                ),
                t("Unified Gateway · Trust Chain", "统一网关 · 单一信任链"),
                "urn:kense:gateway:v1",
            ),
            Kind::Sandbox => (
                t(
                    "Run workload inside isolated runtime",
                    "运行隔离负载 · 预热池极速挂载",
                ),
                t("KBox · Isolation Runtime", "KBox & KSandbox · 隔离运行时"),
                "urn:kense:kbox:microvm",
            ),
            Kind::Workbench => (
                t(
                    "ksandbox exec -- cargo test",
                    "请求单次授权：ksandbox exec -- cargo test",
                ),
                t("KWork · KUI · KCL Studio", "KWork · KUI · KCL 同源工作台"),
                "urn:kense:kwork:acp",
            ),
        };

        let status_badge = if self.blocked_alert {
            (t("Blocked", "越界已阻断"), p.danger)
        } else if self.awaiting_approval {
            (t("Approval", "等待单次授权"), p.bright)
        } else if self.running {
            (t("Running", "链路运行中"), p.bright)
        } else {
            (t("Verified", "已验证"), p.success)
        };

        let primary_label = match self.kind {
            Kind::Gateway => t("Dispatch request", "发起网关请求"),
            Kind::Sandbox => t("Run workload", "运行隔离负载"),
            Kind::Workbench => t("Send ACP prompt", "发送 ACP 会话"),
        };
        let secondary_label = match self.kind {
            Kind::Sandbox => t("Simulate escape", "模拟越界访问"),
            Kind::Workbench if self.awaiting_approval => t("Approve tool", "批准工具执行"),
            _ => t("Step stage", "单步切换阶段"),
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
