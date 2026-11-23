//! Edge-to-edge application shell split by single hairlines.
//!
//! Shares the navigation, command search, and tabbed work-area behavior of
//! [`AppShell`](crate::components::app_shell::AppShell) while laying the sidebar,
//! header bar, main scroll container, and work area flat across the window.

use crate::components::app_shell::{
    AppShell, AppShellEvent, AppShellLayout, AppShellPreset, ShellNavGroup,
};
use gpui::{prelude::*, *};

/// Edge-to-edge variant of [`AppShell`] with a continuous header bar and hairline dividers.
pub struct AppShellFlat {
    inner: Entity<AppShell>,
}

impl EventEmitter<AppShellEvent> for AppShellFlat {}

impl AppShellFlat {
    /// Construct a flat application shell with default `Acme` navigation and right work area.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id = id.into();
        let inner = cx.new(|cx| AppShell::new(id, cx).with_layout(AppShellLayout::Flat));
        cx.subscribe(&inner, |_, _, ev: &AppShellEvent, cx| {
            cx.emit(ev.clone());
        })
        .detach();
        Self { inner }
    }

    /// Apply a canonical sidebar preset (`Chat`, `Agent`, `Settings`, `Issues`, `Crm`).
    pub fn with_preset(self, preset: AppShellPreset, cx: &mut Context<Self>) -> Self {
        self.inner.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_preset(preset, cx)
                .with_layout(AppShellLayout::Flat);
        });
        self
    }

    /// Override the workspace name shown in the top-left header.
    pub fn with_workspace(
        self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.inner.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_workspace(ws)
                .with_layout(AppShellLayout::Flat);
        });
        self
    }

    /// Override custom navigation groups.
    pub fn set_groups(&self, groups: Vec<ShellNavGroup>, cx: &mut App) {
        self.inner.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_groups(groups)
                .with_layout(AppShellLayout::Flat);
            cx.notify();
        });
    }

    /// Access the underlying [`AppShell`] entity for custom page or panel wiring.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.inner
    }
}

impl Render for AppShellFlat {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.inner.clone()
    }
}
