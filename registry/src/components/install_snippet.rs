//! Host-driven onboarding snippets. This component never executes commands or creates keys.
use crate::primitives::{
    clipboard,
    control::UiLanguage,
    family::{Paint, Tone, face},
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallCommand {
    pub manager: SharedString,
    pub text: SharedString,
}
impl InstallCommand {
    pub fn new(manager: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            manager: manager.into(),
            text: text.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallStep {
    pub id: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub filename: SharedString,
    pub snippets: Vec<InstallCommand>,
    /// Enables a host-owned, cancellable key generation request for this step.
    pub generate_key: bool,
}
impl InstallStep {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        snippets: Vec<InstallCommand>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: "".into(),
            filename: "".into(),
            snippets,
            generate_key: false,
        }
    }
    pub fn description(mut self, value: impl Into<SharedString>) -> Self {
        self.description = value.into();
        self
    }
    pub fn filename(mut self, value: impl Into<SharedString>) -> Self {
        self.filename = value.into();
        self
    }
    pub fn generate_key(mut self, yes: bool) -> Self {
        self.generate_key = yes;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallFramework {
    pub id: SharedString,
    pub label: SharedString,
    pub steps: Vec<InstallStep>,
}
impl InstallFramework {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        steps: Vec<InstallStep>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            steps,
        }
    }
}
#[derive(Clone, Debug)]
pub enum InstallSnippetEvent {
    KeyRequested {
        id: u64,
        framework: SharedString,
        step: SharedString,
    },
    KeyCancelled(u64),
    CopyFinished {
        framework: SharedString,
        step: SharedString,
        success: bool,
    },
    ConfirmCancelled(u64),
    ConfirmRequested {
        id: u64,
        framework: SharedString,
        step: SharedString,
    },
}
/// Frameworks and snippets are supplied by the host. Tokens `{{API_KEY}}` are filled only after explicit key generation.
pub struct InstallSnippet {
    frameworks: Vec<InstallFramework>,
    selected: usize,
    manager: usize,
    framework_focus: Vec<FocusHandle>,
    manager_focus: Vec<FocusHandle>,
    sequence: u64,
    active_key: Option<(u64, usize, usize)>,
    active_confirm: Option<(u64, usize, usize)>,
    done: Vec<(usize, usize)>,
    key: Option<SharedString>,
    error: SharedString,
    copied: Option<(usize, bool)>,
    copy_epoch: u64,
}
impl EventEmitter<InstallSnippetEvent> for InstallSnippet {}
impl InstallSnippet {
    pub fn new(frameworks: Vec<InstallFramework>, cx: &mut Context<Self>) -> Self {
        let framework_focus = frameworks
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        let manager_count = frameworks
            .iter()
            .flat_map(|f| &f.steps)
            .map(|s| s.snippets.len())
            .max()
            .unwrap_or(0);
        let manager_focus = (0..manager_count)
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
        Self {
            frameworks,
            selected: 0,
            manager: 0,
            framework_focus,
            manager_focus,
            sequence: 0,
            active_key: None,
            active_confirm: None,
            done: vec![],
            key: None,
            error: "".into(),
            copied: None,
            copy_epoch: 0,
        }
    }
    pub fn active_key(&self) -> Option<u64> {
        self.active_key.map(|v| v.0)
    }
    fn next_id(&mut self) -> u64 {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.sequence
    }
    pub fn select_framework(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.frameworks.len() || index == self.selected {
            return false;
        }
        self.cancel_key(cx);
        self.cancel_confirmation(cx);
        self.selected = index;
        self.manager = 0;
        self.error = "".into();
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        cx.notify();
        true
    }
    pub fn select_manager(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        let count = self
            .frameworks
            .get(self.selected)
            .and_then(|f| f.steps.first())
            .map_or(0, |s| s.snippets.len());
        if index >= count || index == self.manager {
            return false;
        }
        self.manager = index;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        cx.notify();
        true
    }
    fn snippet(&self, step: usize) -> Option<String> {
        let step = self.frameworks.get(self.selected)?.steps.get(step)?;
        let snippet = step
            .snippets
            .get(self.manager)
            .or_else(|| step.snippets.first())?;
        Some(match &self.key {
            Some(key) => snippet.text.replace("{{API_KEY}}", key),
            None => snippet.text.to_string(),
        })
    }
    pub fn request_key(&mut self, step: usize, cx: &mut Context<Self>) -> bool {
        let Some(item) = self
            .frameworks
            .get(self.selected)
            .and_then(|f| f.steps.get(step))
        else {
            return false;
        };
        if !item.generate_key || self.active_key.is_some() {
            return false;
        }
        let framework = self.frameworks[self.selected].id.clone();
        let step_id = item.id.clone();
        let id = self.next_id();
        self.active_key = Some((id, self.selected, step));
        self.error = "".into();
        cx.emit(InstallSnippetEvent::KeyRequested {
            id,
            framework,
            step: step_id,
        });
        cx.notify();
        true
    }
    pub fn cancel_key(&mut self, cx: &mut Context<Self>) {
        if let Some((id, _, _)) = self.active_key.take() {
            cx.emit(InstallSnippetEvent::KeyCancelled(id));
            cx.notify();
        }
    }
    pub fn finish_key(
        &mut self,
        id: u64,
        value: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self
            .active_key
            .is_some_and(|(active, framework, _)| active == id && framework == self.selected)
        {
            return false;
        }
        let value = value.into();
        if value.is_empty() {
            return self.fail_key(id, "Empty key", cx);
        }
        self.active_key = None;
        self.key = Some(value);
        self.error = "".into();
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        cx.notify();
        true
    }
    pub fn fail_key(
        &mut self,
        id: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.active_key.is_some_and(|(active, _, _)| active == id) {
            return false;
        }
        self.active_key = None;
        self.error = message.into();
        cx.notify();
        true
    }
    /// Request host confirmation; the host must call `confirm_step` with the matching ID.
    pub fn request_confirmation(&mut self, step: usize, cx: &mut Context<Self>) -> bool {
        let Some(item) = self
            .frameworks
            .get(self.selected)
            .and_then(|f| f.steps.get(step))
        else {
            return false;
        };
        if self.active_confirm.is_some() || self.done.contains(&(self.selected, step)) {
            return false;
        }
        let framework = self.frameworks[self.selected].id.clone();
        let step_id = item.id.clone();
        let id = self.next_id();
        self.active_confirm = Some((id, self.selected, step));
        self.error = "".into();
        cx.emit(InstallSnippetEvent::ConfirmRequested {
            id,
            framework,
            step: step_id,
        });
        cx.notify();
        true
    }
    pub fn cancel_confirmation(&mut self, cx: &mut Context<Self>) {
        if let Some((id, _, _)) = self.active_confirm.take() {
            cx.emit(InstallSnippetEvent::ConfirmCancelled(id));
            cx.notify();
        }
    }
    pub fn confirm_step(
        &mut self,
        id: u64,
        success: bool,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((active, framework, step)) = self.active_confirm else {
            return false;
        };
        if active != id {
            return false;
        }
        self.active_confirm = None;
        if success {
            if !self.done.contains(&(framework, step)) {
                self.done.push((framework, step));
            }
        } else {
            self.error = message.into();
        }
        cx.notify();
        true
    }
    pub fn copy_step(&mut self, step: usize, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.snippet(step) else {
            return false;
        };
        let framework = self.frameworks[self.selected].id.clone();
        let step_id = self.frameworks[self.selected].steps[step].id.clone();
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        let epoch = self.copy_epoch;
        let task = clipboard::write(&text, cx);
        cx.spawn(async move |this, cx| {
            let success = task.await;
            let _ = this.update(cx, |this, cx| {
                cx.emit(InstallSnippetEvent::CopyFinished {
                    framework,
                    step: step_id,
                    success,
                });
                if this.copy_epoch == epoch {
                    this.copied = Some((step, success));
                    cx.notify();
                }
            });
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1800))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.copy_epoch == epoch {
                    this.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
        true
    }
}

impl Render for InstallSnippet {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let t = |en: &'static str, zh: &'static str| lang.text(en, zh);
        let radius = crate::foundation::surface_radius(cx);
        let mut tabs = div()
            .id("install-frameworks")
            .role(Role::TabList)
            .aria_label(t("Framework", "框架"))
            .flex()
            .flex_wrap()
            .gap(px(4.));
        let framework_count = self.frameworks.len();
        for (i, item) in self.frameworks.iter().enumerate() {
            let label = item.label.clone();
            tabs = tabs.child(
                face(
                    ("install-framework", i),
                    label.clone(),
                    34.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .role(Role::Tab)
                .tab_index(if i == self.selected { 0 } else { -1 })
                .track_focus(&self.framework_focus[i])
                .aria_selected(i == self.selected)
                .when(i == self.selected, |v| v.bg(p.hover))
                .px(px(10.))
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.select_framework(i, cx);
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    let next = match e.keystroke.key.as_str() {
                        "left" => (i + framework_count - 1) % framework_count,
                        "right" => (i + 1) % framework_count,
                        "home" => 0,
                        "end" => framework_count - 1,
                        _ => return,
                    };
                    s.select_framework(next, cx);
                    s.framework_focus[next].focus(w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                })),
            );
        }
        let mut content = div()
            .id("install-steps")
            .role(Role::TabPanel)
            .w_full()
            .flex()
            .flex_col()
            .gap(px(16.));
        if let Some(framework) = self.frameworks.get(self.selected) {
            let steps = framework.steps.clone();
            let managers = steps
                .first()
                .map(|s| s.snippets.clone())
                .unwrap_or_default();
            let manager_count = managers.len();
            let mut manager_tabs = div()
                .id("install-managers")
                .role(Role::TabList)
                .aria_label(t("Package manager", "包管理器"))
                .flex()
                .flex_wrap()
                .gap(px(4.));
            if manager_count > 1 {
                for (i, command) in managers.iter().enumerate() {
                    let label = command.manager.clone();
                    manager_tabs = manager_tabs.child(
                        face(
                            ("install-manager", i),
                            label.clone(),
                            30.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .role(Role::Tab)
                        .tab_index(if i == self.manager { 0 } else { -1 })
                        .track_focus(&self.manager_focus[i])
                        .aria_selected(i == self.manager)
                        .when(i == self.manager, |v| v.bg(p.hover))
                        .px(px(8.))
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.select_manager(i, cx);
                        }))
                        .on_key_down(cx.listener(
                            move |s, e: &KeyDownEvent, w, cx| {
                                let next = match e.keystroke.key.as_str() {
                                    "left" => (i + manager_count - 1) % manager_count,
                                    "right" => (i + 1) % manager_count,
                                    "home" => 0,
                                    "end" => manager_count - 1,
                                    _ => return,
                                };
                                s.select_manager(next, cx);
                                s.manager_focus[next].focus(w, cx);
                                w.prevent_default();
                                cx.stop_propagation();
                            },
                        )),
                    );
                }
                content = content.child(manager_tabs);
            }
            let mut step_list = div()
                .id("install-list")
                .role(Role::List)
                .flex()
                .flex_col()
                .gap(px(16.));
            for (i, step) in steps.iter().enumerate() {
                let done = self.done.contains(&(self.selected, i));
                let marker = if done {
                    "✓".to_string()
                } else {
                    (i + 1).to_string()
                };
                let confirm_pending = self
                    .active_confirm
                    .is_some_and(|(_, f, index)| f == self.selected && index == i);
                let mut actions = div().flex().flex_wrap().gap(px(6.));
                let copied = match self.copied {
                    Some((index, true)) if index == i => t("Copied", "已复制"),
                    Some((index, false)) if index == i => t("Copy failed; retry", "复制失败，重试"),
                    _ => t("Copy", "复制"),
                };
                actions = actions.child(
                    face(
                        ("install-copy", i),
                        copied,
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(copied)
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.copy_step(i, cx);
                    })),
                );
                if step.generate_key {
                    let key_pending = self
                        .active_key
                        .is_some_and(|(_, f, index)| f == self.selected && index == i);
                    let label = if key_pending {
                        t("Cancel", "取消")
                    } else if self.key.is_some() {
                        t("Generate another key", "重新生成密钥")
                    } else {
                        t("Generate key", "生成密钥")
                    };
                    actions = actions.child(
                        face(
                            ("install-key", i),
                            label,
                            32.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(8.))
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if key_pending {
                                s.cancel_key(cx);
                            } else {
                                s.request_key(i, cx);
                            }
                        })),
                    );
                }
                if !done {
                    let label = if confirm_pending {
                        t("Cancel confirmation", "取消确认")
                    } else {
                        t("Confirm step", "确认步骤")
                    };
                    let button = face(
                        ("install-confirm", i),
                        label,
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(label);
                    actions = actions.child(if confirm_pending {
                        button.on_click(cx.listener(|s, _, _, cx| s.cancel_confirmation(cx)))
                    } else if self.active_confirm.is_some() {
                        button.opacity(0.5).tab_index(-1)
                    } else {
                        button.on_click(cx.listener(move |s, _, _, cx| {
                            s.request_confirmation(i, cx);
                        }))
                    });
                }
                let snippet = self.snippet(i).unwrap_or_default();
                let item = div()
                    .id(("install-step", i))
                    .role(Role::ListItem)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .size(px(28.))
                                    .rounded(radius)
                                    .bg(p.hover)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(marker),
                            )
                            .child(div().text_color(p.ink).child(step.title.clone()))
                            .when(done, |v| {
                                v.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .child(t("Done", "已完成")),
                                )
                            }),
                    )
                    .when(!step.description.is_empty(), |v| {
                        v.child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.muted)
                                .child(step.description.clone()),
                        )
                    })
                    .when(!step.filename.is_empty(), |v| {
                        v.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(step.filename.clone()),
                        )
                    })
                    .child(
                        div().w_full().rounded(radius).bg(p.tray).p(px(10.)).child(
                            div()
                                .id(("install-code", i))
                                .w_full()
                                .overflow_x_scroll()
                                .child(div().whitespace_nowrap().text_size(px(13.)).child(snippet)),
                        ),
                    )
                    .child(actions);
                step_list = step_list.child(item);
            }
            content = content.child(step_list);
        }
        div()
            .id("install-snippet")
            .role(Role::Group)
            .aria_label(t("Quick start", "快速开始"))
            .w_full()
            .max_w(px(640.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(p.ink)
                    .child(t("Get started", "快速开始")),
            )
            .child(tabs)
            .child(content)
            .when(!self.error.is_empty(), |v| {
                v.child(
                    div()
                        .id("install-error")
                        .role(Role::Alert)
                        .text_color(p.danger)
                        .child(self.error.clone()),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn examples() -> Vec<InstallFramework> {
        vec![
            InstallFramework::new(
                "first",
                "First",
                vec![
                    InstallStep::new(
                        "setup",
                        "Set up",
                        vec![InstallCommand::new("cargo", "key={{API_KEY}}")],
                    )
                    .generate_key(true),
                ],
            ),
            InstallFramework::new(
                "second",
                "Second",
                vec![InstallStep::new(
                    "setup",
                    "Set up",
                    vec![InstallCommand::new("cargo", "next")],
                )],
            ),
        ]
    }

    #[test]
    fn ignores_late_host_results_and_supports_retry() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| InstallSnippet::new(examples(), cx));
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.request_key(0, cx));
                let id = s.active_key().unwrap();
                s.cancel_key(cx);
                assert!(!s.finish_key(id, "stale", cx));
                assert!(s.request_key(0, cx));
                let id = s.active_key().unwrap();
                assert!(s.fail_key(id, "failed", cx));
                assert!(s.request_key(0, cx));
                let id = s.active_key().unwrap();
                assert!(s.finish_key(id, "sample", cx));
                assert_eq!(s.snippet(0).as_deref(), Some("key=sample"));
                assert!(s.request_confirmation(0, cx));
                let confirm = s.active_confirm.unwrap().0;
                s.cancel_confirmation(cx);
                assert!(!s.confirm_step(confirm, true, "", cx));
                assert!(s.request_confirmation(0, cx));
                let confirm = s.active_confirm.unwrap().0;
                assert!(s.confirm_step(confirm, false, "failed", cx));
                assert!(s.done.is_empty());
                assert!(s.request_confirmation(0, cx));
                let confirm = s.active_confirm.unwrap().0;
                assert!(s.confirm_step(confirm, true, "", cx));
                assert_eq!(s.done, vec![(0, 0)]);
            })
        });
    }
}
