//! Host-driven onboarding snippets. This component never executes commands or creates keys.
use crate::primitives::{
    clipboard,
    control::UiLanguage,
    family::{Paint, Slide, Tone, face},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

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
        value: Option<SharedString>,
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
    title: Option<SharedString>,
    description: SharedString,
    version: Option<SharedString>,
    key_placeholder: SharedString,
    provided_key: bool,
    confirm_steps: bool,
    selected: usize,
    manager: usize,
    framework_focus: Vec<FocusHandle>,
    framework_marker: Slide,
    tab_width: Rc<Cell<Pixels>>,
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
            title: None,
            description: "".into(),
            version: None,
            key_placeholder: "YOUR_API_KEY".into(),
            provided_key: false,
            confirm_steps: false,
            selected: 0,
            manager: 0,
            framework_focus,
            framework_marker: Slide::new(0.),
            tab_width: Default::default(),
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
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
    pub fn with_version(mut self, version: impl Into<SharedString>) -> Self {
        self.version = Some(version.into());
        self
    }
    pub fn with_key_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.key_placeholder = placeholder.into();
        self
    }
    /// Display a key already supplied by the host and hide key generation.
    pub fn with_api_key(mut self, key: impl Into<SharedString>) -> Self {
        let key = key.into();
        self.provided_key = !key.is_empty();
        self.key = self.provided_key.then_some(key);
        self
    }
    pub fn with_step_confirmation(mut self, enabled: bool) -> Self {
        self.confirm_steps = enabled;
        self
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
        self.framework_marker.duration(index as f32, 0.24, cx);
        let manager_count = self.frameworks[index]
            .steps
            .first()
            .map_or(0, |s| s.snippets.len());
        self.manager = self.manager.min(manager_count.saturating_sub(1));
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
        Some(snippet.text.replace(
            "{{API_KEY}}",
            self.key.as_ref().unwrap_or(&self.key_placeholder),
        ))
    }
    pub fn request_key(&mut self, step: usize, cx: &mut Context<Self>) -> bool {
        let Some(item) = self
            .frameworks
            .get(self.selected)
            .and_then(|f| f.steps.get(step))
        else {
            return false;
        };
        if !item.generate_key || self.active_key.is_some() || self.provided_key {
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
        let Some((active, framework, step)) = self.active_key else {
            return false;
        };
        if active != id || framework != self.selected {
            return false;
        }
        let value = value.into();
        if value.is_empty() {
            return self.fail_key(id, "Empty key", cx);
        }
        self.active_key = None;
        self.key = Some(value);
        if !self.confirm_steps && !self.done.contains(&(framework, step)) {
            self.done.push((framework, step));
        }
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
        let selection = self.selected;
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
                    value: success.then(|| text.into()),
                });
                if success && !this.confirm_steps && !this.done.contains(&(selection, step)) {
                    this.done.push((selection, step));
                    cx.notify();
                }
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let t = |en: &'static str, zh: &'static str| lang.text(en, zh);
        let radius = crate::foundation::surface_radius(cx);
        let framework_count = self.frameworks.len();
        let width = f32::from(self.tab_width.get());
        let compact = width > 0. && width < 348.;
        let cells = framework_count.max(1);
        let cell_width = ((width - 6. - 2. * (cells - 1) as f32) / cells as f32).max(0.);
        let marker = self.framework_marker.get(window, cx);
        let tab_width = self.tab_width.clone();
        let mut tabs = div()
            .id("install-frameworks")
            .role(Role::TabList)
            .aria_label(t("Framework", "框架"))
            .relative()
            .flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(radius)
            .bg(p.stage)
            .child(
                canvas(
                    move |b, w, _| {
                        if tab_width.replace(b.size.width) != b.size.width {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .when(cell_width > 0., |v| {
                v.child(
                    div()
                        .absolute()
                        .top(px(3.))
                        .left(px(3. + marker * (cell_width + 2.)))
                        .w(px(cell_width))
                        .h(px(34.))
                        .rounded(radius)
                        .bg(p.panel)
                        .border_1()
                        .border_color(p.line),
                )
            });
        for (i, item) in self.frameworks.iter().enumerate() {
            let label = item.label.clone();
            let mark = match item.id.as_ref() {
                "desktop" => Some(Glyph::Monitor),
                "web" => Some(Glyph::Globe),
                "library" => Some(Glyph::Book),
                "cli" => Some(Glyph::Arrow),
                _ => None,
            };
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
                .track_focus(&self.framework_focus[i].clone().tab_stop(i == self.selected))
                .aria_selected(i == self.selected)
                .when(cell_width <= 0. && i == self.selected, |v| v.bg(p.panel))
                .flex_1()
                .min_w_0()
                .text_ellipsis()
                .px(px(4.))
                .gap(px(6.))
                .when_some(mark.filter(|_| !compact), |v, mark| {
                    v.child(icon(mark, 14., p.secondary))
                })
                .child(label)
                .on_click(cx.listener(move |s, _, w, cx| {
                    s.select_framework(i, cx);
                    s.framework_focus[i].focus(w, cx);
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
                        .track_focus(&self.manager_focus[i].clone().tab_stop(i == self.manager))
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
            }
            let mut manager_tabs = (manager_count > 1).then_some(manager_tabs);
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
                let mut key_action = None;
                let copied = match self.copied {
                    Some((index, true)) if index == i => t("Copied", "已复制"),
                    Some((index, false)) if index == i => t("Copy failed; retry", "复制失败，重试"),
                    _ => t("Copy", "复制"),
                };
                let copy = face(
                    ("install-copy", i),
                    copied,
                    30.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(7.))
                .child(icon(
                    if self.copied == Some((i, true)) {
                        Glyph::Check
                    } else {
                        Glyph::Copy
                    },
                    14.,
                    p.secondary,
                ))
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.copy_step(i, cx);
                }));
                if step.generate_key && !self.provided_key {
                    let key_pending = self
                        .active_key
                        .is_some_and(|(_, f, index)| f == self.selected && index == i);
                    let label = if key_pending {
                        t("Cancel", "取消")
                    } else if !self.error.is_empty() {
                        t("Try again", "重试")
                    } else if self.key.is_some() {
                        t("Generate another key", "重新生成密钥")
                    } else {
                        t("Generate key", "生成密钥")
                    };
                    key_action = Some(
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
                if self.confirm_steps && !done {
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
                let reserved_lines = self
                    .frameworks
                    .iter()
                    .filter_map(|f| f.steps.get(i))
                    .flat_map(|step| &step.snippets)
                    .map(|command| command.text.lines().count())
                    .max()
                    .unwrap_or(1);
                let item = div()
                    .id(("install-step", i))
                    .role(Role::ListItem)
                    .aria_label(if done {
                        format!("{}, {}", step.title, t("done", "已完成"))
                    } else {
                        step.title.to_string()
                    })
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(24.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .border_color(p.line)
                            .bg(if done {
                                p.success.opacity(0.14)
                            } else {
                                p.panel
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .child(marker),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .justify_between()
                                    .items_center()
                                    .when(compact, |v| v.flex_wrap())
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(step.title.clone()),
                                            )
                                            .when(!step.description.is_empty(), |v| {
                                                v.child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .text_color(p.secondary)
                                                        .child(step.description.clone()),
                                                )
                                            }),
                                    )
                                    .when_some(key_action, |v, action| v.child(action.flex_none())),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .rounded(radius)
                                    .border_1()
                                    .border_color(p.line)
                                    .bg(p.stage)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(6.))
                                            .px(px(6.))
                                            .border_b_1()
                                            .border_color(p.line)
                                            .when_some(manager_tabs.take(), |v, tabs| v.child(tabs))
                                            .when(!step.filename.is_empty(), |v| {
                                                v.child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(p.secondary)
                                                        .child(step.filename.clone()),
                                                )
                                            })
                                            .child(copy.ml_auto()),
                                    )
                                    .child(
                                        div()
                                            .id(("install-code", i))
                                            .w_full()
                                            .min_h(px(reserved_lines as f32 * 19. + 20.))
                                            .px(px(10.))
                                            .py(px(9.))
                                            .overflow_x_scroll()
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_size(px(13.))
                                                    .line_height(px(19.))
                                                    .children(snippet.lines().enumerate().map(
                                                        |(line, text)| {
                                                            let mut styled = StyledText::new(
                                                                SharedString::from(text.to_owned()),
                                                            );
                                                            let verb = text
                                                                .find(char::is_whitespace)
                                                                .unwrap_or(text.len());
                                                            if i == 0 && verb > 0 {
                                                                styled =
                                                                    styled.with_highlights([(
                                                                        0..verb,
                                                                        HighlightStyle {
                                                                            color: Some(p.bright),
                                                                            ..Default::default()
                                                                        },
                                                                    )]);
                                                            }
                                                            div()
                                                                .flex()
                                                                .gap_2()
                                                                .whitespace_nowrap()
                                                                .when(
                                                                    i == 2 && reserved_lines > 1,
                                                                    |v| {
                                                                        v.child(
                                                                            div()
                                                                                .w(px(18.))
                                                                                .flex_none()
                                                                                .text_right()
                                                                                .text_color(p.faint)
                                                                                .child(
                                                                                    (line + 1)
                                                                                        .to_string(
                                                                                        ),
                                                                                ),
                                                                        )
                                                                    },
                                                                )
                                                                .child(styled)
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .child(actions),
                    );
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
            .p(px(if compact { 14. } else { 20. }))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(
                                        self.title
                                            .clone()
                                            .unwrap_or_else(|| t("Get started", "快速开始").into()),
                                    ),
                            )
                            .when(!self.description.is_empty(), |view| {
                                view.child(
                                    div()
                                        .text_size(px(14.))
                                        .text_color(p.secondary)
                                        .child(self.description.clone()),
                                )
                            }),
                    )
                    .when_some(
                        self.version.clone().filter(|_| !compact),
                        |view, version| {
                            view.child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(version),
                            )
                        },
                    ),
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
    use super::{InstallCommand, InstallFramework, InstallSnippet, InstallStep};
    use gpui::TestAppContext;

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
        let (view, w) = app.add_window_view(|_, cx| {
            InstallSnippet::new(examples(), cx).with_step_confirmation(true)
        });
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
