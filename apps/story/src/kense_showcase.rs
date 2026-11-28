//! Real interactive GPUI product interfaces for KStack, KStudio, KWork, and K on the Kense homepage.

mod klang;
mod kstack;
mod kstudio;
mod kwork;

use gpui::{prelude::*, *};
use kui::{
    foundation::{Theme, ThemeMode},
    primitives::{control::UiLanguage, family::Paint},
};

pub const IDS: &[&str] = &[
    "kense-gateway",
    "kense-sandbox",
    "kense-workbench",
    "kense-klang",
];

/// Instantiate a Kense homepage GPUI product interface when `id` matches [`IDS`].
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    if !IDS.contains(&id) {
        return None;
    }
    crate::ensure_story_fonts(cx);
    let inner: AnyView = match id {
        "kense-sandbox" => cx.new(kstudio::KStudioApp::new).into(),
        "kense-workbench" => cx.new(kwork::KWorkApp::new).into(),
        "kense-klang" => cx.new(klang::KlangIdeApp::new).into(),
        _ => cx.new(kstack::KStackConsole::new).into(),
    };
    Some(cx.new(|cx| HostShell::new(inner, cx)).into())
}

pub(super) fn tr<'a>(zh: bool, zh_s: &'a str, en_s: &'a str) -> &'a str {
    if zh { zh_s } else { en_s }
}

pub(super) fn is_zh(cx: &App) -> bool {
    UiLanguage::current(cx) == UiLanguage::Chinese
}

pub(super) fn is_dark(cx: &App) -> bool {
    Theme::global(cx).mode == ThemeMode::Dark
}

pub(super) fn traffic_lights() -> Div {
    let dot = |hex: u32| div().w(px(10.)).h(px(10.)).rounded_full().bg(rgb(hex));
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(dot(0xff5f57))
        .child(dot(0xfebc2e))
        .child(dot(0x28c840))
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
        let dark = is_dark(cx);
        let outer_bg = if dark { rgb(0x0a0a0c) } else { rgb(0xf4f6f9) };
        let edge = |i| {
            div()
                .id(("kense-edge", i))
                .track_focus(&self.edges[i])
                .tab_stop(true)
                .size(px(0.))
        };
        let bridge = cx.try_global::<crate::StoryBridge>();
        let embedded = bridge.is_some();
        let min_height = bridge.map_or(628., |b| 628_f32.max((b.min_height)()));
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
            .text_size(px(12.5))
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

#[cfg(test)]
mod tests {
    use super::{UiLanguage, klang, kstack, kstudio, kwork};
    use gpui::{Modifiers, TestAppContext, px, size};

    #[test]
    fn kstack_deploy_and_overview_click_interactions_fit_viewport() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            crate::ensure_story_fonts(cx);
            cx.set_global(UiLanguage::Chinese);
        });
        let (view, window) = app.add_window_view(|_, cx| kstack::KStackConsole::new(cx));
        window.simulate_resize(size(px(1108.), px(628.)));
        window.run_until_parked();

        let launch_bounds = window.debug_bounds("ks-launch-btn").expect("ks-launch-btn");
        assert!(
            launch_bounds.bottom() <= px(628.) && launch_bounds.right() <= px(1108.),
            "KStack launch button clipped outside 1108x628: {launch_bounds:?}"
        );

        // Click "创建推理部署" -> adds deployment and transitions to Deployments list!
        window.simulate_click(launch_bounds.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            let s = view.read(cx);
            assert_eq!(s.page, kstack::NavPage::Deployments);
            assert_eq!(s.deployments.len(), 4);
        });

        // Click left sidebar "概览" -> transitions to Overview dashboard!
        let ov_bounds = window
            .debug_bounds("kstack-nav-0-0")
            .expect("kstack-nav-0-0");
        window.simulate_click(ov_bounds.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).page, kstack::NavPage::Overview);
        });
    }

    #[test]
    fn kstudio_overview_session_and_agent_handoff_click_interactions() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            crate::ensure_story_fonts(cx);
            cx.set_global(UiLanguage::Chinese);
        });
        let (view, window) = app.add_window_view(|_, cx| kstudio::KStudioApp::new(cx));
        window.simulate_resize(size(px(1108.), px(628.)));
        window.run_until_parked();

        // Click Forge's "* Checkout fix" session in left machine tree -> opens Session(0)!
        let tree_s0 = window.debug_bounds("kst-tree-s0").expect("kst-tree-s0");
        assert!(tree_s0.bottom() <= px(628.));
        window.simulate_click(tree_s0.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).view, kstudio::StudioView::Session(0));
            assert_eq!(view.read(cx).session_agent[0], "Claude Code");
        });

        // Click "<-> 跨智能体交接" -> hands session over to Codex!
        let handoff = window.debug_bounds("kst-handoff").expect("kst-handoff");
        assert!(handoff.bottom() <= px(628.) && handoff.right() <= px(1108.));
        window.simulate_click(handoff.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).session_agent[0], "Codex");
            assert_eq!(view.read(cx).session_logs[0].len(), 5);
        });
    }

    #[test]
    fn kwork_three_pane_workbench_send_and_kantor_sheet_interactions() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            crate::ensure_story_fonts(cx);
            cx.set_global(UiLanguage::English);
        });
        let (view, window) = app.add_window_view(|_, cx| kwork::KWorkApp::new(cx));
        window.simulate_resize(size(px(1108.), px(628.)));
        window.run_until_parked();

        // Verify composer send button and right inspector tabs fit cleanly inside 1108x628!
        let send_bounds = window.debug_bounds("kw-send-btn").expect("kw-send-btn");
        let sheet_tab = window.debug_bounds("kw-rtab-sheet").expect("kw-rtab-sheet");
        assert!(
            send_bounds.bottom() <= px(628.) && sheet_tab.right() <= px(1108.),
            "KWork controls clipped: send={send_bounds:?}, sheet_tab={sheet_tab:?}"
        );

        // Click blue send button -> appends live follow-up turn!
        window.simulate_click(send_bounds.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).follow_ups[0].len(), 1);
        });

        // Click Notes.xlsx tab -> switches right inspector to Kantor Office spreadsheet!
        window.simulate_click(sheet_tab.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).right_tab, kwork::RightTab::OfficeSheet);
        });
    }

    #[test]
    fn klang_vscode_ide_jit_and_e0312_quickfix_interactions() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            crate::ensure_story_fonts(cx);
            cx.set_global(UiLanguage::Chinese);
        });
        let (view, window) = app.add_window_view(|_, cx| klang::KlangIdeApp::new(cx));
        window.simulate_resize(size(px(1108.), px(628.)));
        window.run_until_parked();

        let jit_btn = window.debug_bounds("kl-lens-jit").expect("kl-lens-jit");
        let e0312_btn = window.debug_bounds("kl-lens-e0312").expect("kl-lens-e0312");
        assert!(jit_btn.bottom() <= px(628.) && e0312_btn.right() <= px(1108.));

        // Click "▶ Run with JIT" -> increments run_count & appends TinyCC JIT output!
        window.simulate_click(jit_btn.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).run_count, 2);
        });

        // Click "! 注入 E0312 越出测试" -> switches bottom panel to PROBLEMS (1)!
        window.simulate_click(e0312_btn.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(view.read(cx).inject_e0312);
            assert_eq!(view.read(cx).bottom_tab, klang::BottomTab::Problems);
        });

        // Click "一键修复 Quick Fix" inside PROBLEMS -> resolves E0312!
        let qf_btn = window
            .debug_bounds("kl-quickfix-e0312")
            .expect("kl-quickfix-e0312");
        assert!(qf_btn.bottom() <= px(628.));
        window.simulate_click(qf_btn.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(!view.read(cx).inject_e0312);
        });
    }
}
