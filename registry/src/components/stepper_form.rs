//! Multi-step onboarding and setup wizard card with animated step checks, validation, and review.
//!
//! Guides users through sequential steps with filling progress bars and self-drawing check marks,
//! directional horizontal transitions (`pro_step_transition`), measured height morphing (`Slide`),
//! inline field validation (`pro_fold` + error shake), email invite chips, gliding plan cards,
//! jump-back `Edit` review rows, and a celebratory completion screen with `Reset`.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_selection_sensor,
        pro_skeleton_bar, pro_spring_bar, pro_step_transition, pro_swap_label, pro_tactile,
        pro_toast_banner, tr,
    },
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Selectable plan option displayed on the plan step of [`StepperForm`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepperPlanOption {
    /// Plan identifier (e.g. `"starter"`, `"pro"`, `"enterprise"`).
    pub id: SharedString,
    /// Display title.
    pub title: SharedString,
    /// Localized Chinese title.
    pub title_zh: SharedString,
    /// Price summary shown beside the radio circle.
    pub price: SharedString,
    /// Short plan description.
    pub description: SharedString,
    /// Localized Chinese description.
    pub description_zh: SharedString,
    /// Optional highlight pill (e.g. `"Popular"`).
    pub badge: Option<SharedString>,
}

impl StepperPlanOption {
    /// Construct a new [`StepperPlanOption`].
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        price: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        let title = title.into();
        let description = description.into();
        Self {
            id: id.into(),
            title_zh: title.clone(),
            title,
            price: price.into(),
            description_zh: description.clone(),
            description,
            badge: None,
        }
    }

    /// Add localized Chinese title and description.
    pub fn with_zh(
        mut self,
        title_zh: impl Into<SharedString>,
        description_zh: impl Into<SharedString>,
    ) -> Self {
        self.title_zh = title_zh.into();
        self.description_zh = description_zh.into();
        self
    }

    /// Attach an accent badge (such as `"Popular"`).
    pub fn with_badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Shared values collected across all steps of [`StepperForm`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepperValues {
    /// Workspace display name.
    pub workspace_name: SharedString,
    /// URL slug under `kense.app/`.
    pub workspace_slug: SharedString,
    /// Invited teammate email addresses.
    pub invites: Vec<SharedString>,
    /// Chosen billing plan id.
    pub plan_id: SharedString,
}

impl Default for StepperValues {
    fn default() -> Self {
        Self {
            workspace_name: "Lumen Studio".into(),
            workspace_slug: "lumen-studio".into(),
            invites: vec!["maya@kense.dev".into()],
            plan_id: "pro".into(),
        }
    }
}

/// Metadata for one step in the top progress bar of [`StepperForm`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepperStepMeta {
    /// Stable step id (`"workspace"`, `"team"`, `"plan"`, `"review"`).
    pub id: SharedString,
    /// Short tab title in the progress strip.
    pub title: SharedString,
    /// Localized Chinese tab title.
    pub title_zh: SharedString,
    /// Main pane heading.
    pub heading: SharedString,
    /// Localized Chinese pane heading.
    pub heading_zh: SharedString,
    /// Supporting description under heading.
    pub description: SharedString,
    /// Localized Chinese description.
    pub description_zh: SharedString,
}

/// Convert a workspace name into a clean lowercase URL slug (`"Lumen Studio!" -> "lumen-studio"`).
pub fn slugify_workspace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

fn is_valid_slug(slug: &str) -> bool {
    let s = slug.trim();
    s.len() >= 2
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_valid_email(raw: &str) -> bool {
    let trimmed = raw.trim();
    let Some((local, domain)) = trimmed.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !trimmed.contains(' ')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// Typed events emitted by [`StepperForm`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepperFormEvent {
    /// Active step index changed (`0..steps.len()`).
    StepChanged(usize),
    /// Form values updated on any step.
    ValuesChanged(StepperValues),
    /// Final step submitted and completed.
    Completed(StepperValues),
    /// Completed state reset back to step `0`.
    Reset,
}

/// Multi-step onboarding & workspace setup wizard card.
pub struct StepperForm {
    id: ElementId,
    steps: Vec<StepperStepMeta>,
    plans: Vec<StepperPlanOption>,
    values: StepperValues,
    initial_values: StepperValues,
    step: usize,
    completed: bool,
    copied_url_at: Option<Instant>,
    slug_manually_edited: bool,
    submit_label: SharedString,
    submit_label_zh: SharedString,
    name_error: Option<SharedString>,
    slug_error: Option<SharedString>,
    invite_error: Option<SharedString>,
    shake_at: Option<(SharedString, Instant)>,
    name_editor: Option<Entity<Editor>>,
    slug_editor: Option<Entity<Editor>>,
    invite_editor: Option<Entity<Editor>>,
    pane_height_cell: Rc<Cell<f32>>,
    height_slide: Slide,
    last_height_target: f32,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
}

impl EventEmitter<StepperFormEvent> for StepperForm {}

impl StepperForm {
    /// Construct a 4-step [`StepperForm`] (`Workspace`, `Team`, `Plan`, `Review`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let defaults = StepperValues::default();
        Self {
            id: id.into(),
            steps: default_step_metas(),
            plans: default_plan_options(),
            values: defaults.clone(),
            initial_values: defaults,
            step: 0,
            completed: false,
            copied_url_at: None,
            slug_manually_edited: false,
            submit_label: "Create workspace".into(),
            submit_label_zh: "创建工作区".into(),
            name_error: None,
            slug_error: None,
            invite_error: None,
            shake_at: None,
            name_editor: None,
            slug_editor: None,
            invite_editor: None,
            pane_height_cell: Rc::new(Cell::new(236.)),
            height_slide: Slide::new(236.),
            last_height_target: 236.,
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(480.)),
            toast: None,
            focus: cx.focus_handle(),
        }
    }

    /// Override initial [`StepperValues`].
    pub fn with_values(mut self, values: StepperValues) -> Self {
        self.initial_values = values.clone();
        self.values = values;
        self
    }

    /// Set the starting step index (`0..=3`).
    pub fn with_initial_step(mut self, step: usize) -> Self {
        let max_idx = self.steps.len().saturating_sub(1);
        self.step = step.min(max_idx);
        self
    }

    /// Customize the final step submit button label (default `"Create workspace"`).
    pub fn with_submit_label(mut self, label: impl Into<SharedString>) -> Self {
        let l = label.into();
        self.submit_label_zh = l.clone();
        self.submit_label = l;
        self
    }

    /// Customize selectable plans on the `Plan` step.
    pub fn with_plans(mut self, plans: Vec<StepperPlanOption>) -> Self {
        self.plans = plans;
        self
    }

    /// Toggle skeleton loading preview state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle forced empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current zero-based step index.
    pub fn step(&self) -> usize {
        self.step
    }

    /// Return whether the wizard has been completed.
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Return current [`StepperValues`].
    pub fn values(&self) -> &StepperValues {
        &self.values
    }

    /// Return whether the current step passes validation without side effects.
    pub fn is_current_step_valid(&self) -> bool {
        match self.step {
            0 => {
                !self.values.workspace_name.trim().is_empty()
                    && is_valid_slug(&self.values.workspace_slug)
            }
            2 => self.plans.iter().any(|p| p.id == self.values.plan_id),
            _ => true,
        }
    }

    /// Programmatically update workspace name (and auto-slugify unless manually edited).
    pub fn set_workspace_name(
        &mut self,
        name: impl Into<SharedString>,
        mut w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let n = name.into();
        self.values.workspace_name = n.clone();
        if !n.trim().is_empty() {
            self.name_error = None;
        }
        if !self.slug_manually_edited {
            let slug = slugify_workspace(n.as_ref());
            if !slug.is_empty() {
                self.values.workspace_slug = SharedString::from(slug.clone());
                self.slug_error = None;
                if let Some(win) = w.as_deref_mut()
                    && let Some(se) = &self.slug_editor
                {
                    se.update(cx, |e, cx| e.set_value(slug, win, cx));
                }
            }
        }
        if let Some(win) = w
            && let Some(ne) = &self.name_editor
        {
            let cur = ne.read(cx).value().to_string();
            if cur != n.as_ref() {
                ne.update(cx, |e, cx| e.set_value(n.to_string(), win, cx));
            }
        }
        cx.emit(StepperFormEvent::ValuesChanged(self.values.clone()));
        cx.notify();
    }

    /// Programmatically update workspace URL slug.
    pub fn set_workspace_slug(
        &mut self,
        slug: impl Into<SharedString>,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let s = slug.into();
        self.slug_manually_edited = true;
        self.values.workspace_slug = s.clone();
        if is_valid_slug(&s) {
            self.slug_error = None;
        }
        if let Some(win) = w
            && let Some(se) = &self.slug_editor
        {
            let cur = se.read(cx).value().to_string();
            if cur != s.as_ref() {
                se.update(cx, |e, cx| e.set_value(s.to_string(), win, cx));
            }
        }
        cx.emit(StepperFormEvent::ValuesChanged(self.values.clone()));
        cx.notify();
    }

    /// Add one or more comma-separated email invites on the `Team` step.
    pub fn add_invite(
        &mut self,
        raw_input: &str,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut added_any = false;
        for part in raw_input.split([',', ';', ' ']) {
            let trimmed = part.trim().to_ascii_lowercase();
            if trimmed.is_empty() {
                continue;
            }
            if !is_valid_email(&trimmed) {
                self.invite_error = Some("Enter a valid email like alex@company.com".into());
                self.shake_at = Some(("invite".into(), Instant::now()));
                cx.notify();
                return false;
            }
            let shared = SharedString::from(trimmed);
            if !self.values.invites.contains(&shared) {
                self.values.invites.push(shared);
                added_any = true;
            }
        }
        if added_any {
            self.invite_error = None;
            if let Some(win) = w
                && let Some(ie) = &self.invite_editor
            {
                ie.update(cx, |e, cx| e.set_value("", win, cx));
            }
            cx.emit(StepperFormEvent::ValuesChanged(self.values.clone()));
            cx.notify();
        }
        added_any
    }

    /// Remove an invited email chip by index.
    pub fn remove_invite(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.values.invites.len() {
            return false;
        }
        self.values.invites.remove(index);
        cx.emit(StepperFormEvent::ValuesChanged(self.values.clone()));
        cx.notify();
        true
    }

    /// Select a plan by id.
    pub fn select_plan(&mut self, plan_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self.plans.iter().find(|p| p.id.as_ref() == plan_id) else {
            return false;
        };
        self.values.plan_id = found.id.clone();
        cx.emit(StepperFormEvent::ValuesChanged(self.values.clone()));
        cx.notify();
        true
    }

    /// Jump back to a previously completed step (`target < self.step`).
    pub fn go_to_step(&mut self, target: usize, cx: &mut Context<Self>) -> bool {
        if self.completed || target >= self.steps.len() || target > self.step {
            return false;
        }
        if target == self.step {
            return true;
        }
        self.step = target;
        cx.emit(StepperFormEvent::StepChanged(self.step));
        cx.notify();
        true
    }

    /// Retreat one step (`Back` button).
    pub fn prev_step(&mut self, cx: &mut Context<Self>) -> bool {
        if self.completed || self.step == 0 {
            return false;
        }
        self.step -= 1;
        cx.emit(StepperFormEvent::StepChanged(self.step));
        cx.notify();
        true
    }

    /// Validate the active step and advance — or complete on the final step.
    pub fn next_or_submit(&mut self, mut w: Option<&mut Window>, cx: &mut Context<Self>) -> bool {
        if self.completed {
            return false;
        }
        if self.step == 0 {
            let name_ok = !self.values.workspace_name.trim().is_empty();
            let slug_ok = is_valid_slug(&self.values.workspace_slug);
            self.name_error = if name_ok {
                None
            } else {
                Some("Give your workspace a name".into())
            };
            self.slug_error = if slug_ok {
                None
            } else {
                Some("Use at least 2 lowercase letters, digits, or single hyphens".into())
            };
            if !name_ok {
                self.shake_at = Some(("name".into(), Instant::now()));
                if let Some(win) = w.as_deref_mut()
                    && let Some(ne) = &self.name_editor
                {
                    ne.clone().update(cx, |e, cx| e.focus(win, cx));
                }
                cx.notify();
                return false;
            }
            if !slug_ok {
                self.shake_at = Some(("slug".into(), Instant::now()));
                if let Some(win) = w.as_deref_mut()
                    && let Some(se) = &self.slug_editor
                {
                    se.clone().update(cx, |e, cx| e.focus(win, cx));
                }
                cx.notify();
                return false;
            }
        } else if self.step == 1 {
            // If user typed a pending email into the invite editor without pressing Enter, add it first
            let pending = self
                .invite_editor
                .as_ref()
                .map(|ie| ie.read(cx).value().trim().to_string())
                .unwrap_or_default();
            if !pending.is_empty() && !self.add_invite(&pending, w, cx) {
                return false;
            }
            self.invite_error = None;
        }

        if self.step + 1 < self.steps.len() {
            self.step += 1;
            cx.emit(StepperFormEvent::StepChanged(self.step));
            cx.notify();
            true
        } else {
            self.completed = true;
            cx.emit(StepperFormEvent::Completed(self.values.clone()));
            cx.notify();
            true
        }
    }

    /// Reset the wizard from the completed state back to Step 0.
    pub fn reset(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        self.completed = false;
        self.step = 0;
        self.slug_manually_edited = false;
        self.values = self.initial_values.clone();
        self.name_error = None;
        self.slug_error = None;
        self.invite_error = None;
        if let Some(win) = w {
            let n = self.values.workspace_name.to_string();
            let s = self.values.workspace_slug.to_string();
            if let Some(ne) = &self.name_editor {
                ne.update(cx, |e, cx| e.set_value(n, win, cx));
            }
            if let Some(se) = &self.slug_editor {
                se.update(cx, |e, cx| e.set_value(s, win, cx));
            }
            if let Some(ie) = &self.invite_editor {
                ie.update(cx, |e, cx| e.set_value("", win, cx));
            }
        }
        cx.emit(StepperFormEvent::Reset);
        cx.notify();
    }

    fn field_shake_dx(&self, key: &str, cx: &App) -> f32 {
        if disabled_motion(cx) {
            return 0.0;
        }
        let Some((k, started)) = &self.shake_at else {
            return 0.0;
        };
        if k.as_ref() != key {
            return 0.0;
        }
        let age = started.elapsed().as_secs_f32();
        if age >= 0.38 {
            return 0.0;
        }
        let u = age / 0.38;
        (u * std::f32::consts::TAU * 2.0).sin() * 3.5 * (1.0 - u)
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.name_editor.is_none() {
            let init_name = self.values.workspace_name.to_string();
            let ne = cx.new(|cx| {
                let mut e = Editor::new("Workspace name", InputMode::Single, w, cx);
                e.set_placeholder("Lumen Studio");
                e.set_value(init_name, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ne, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let next = ed_ent.read(cx).value().to_string();
                    this.set_workspace_name(next, Some(w), cx);
                }
                EditEvent::Submitted => {
                    this.next_or_submit(Some(w), cx);
                }
                _ => {}
            })
            .detach();
            self.name_editor = Some(ne);
        }

        if self.slug_editor.is_none() {
            let init_slug = self.values.workspace_slug.to_string();
            let se = cx.new(|cx| {
                let mut e = Editor::new("Workspace URL", InputMode::Single, w, cx);
                e.set_placeholder("lumen-studio");
                e.set_value(init_slug, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&se, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let next = ed_ent.read(cx).value().to_string();
                    this.set_workspace_slug(next, None, cx);
                }
                EditEvent::Submitted => {
                    this.next_or_submit(Some(w), cx);
                }
                _ => {}
            })
            .detach();
            self.slug_editor = Some(se);
        }

        if self.invite_editor.is_none() {
            let ie = cx.new(|cx| {
                let mut e = Editor::new("Invite email", InputMode::Single, w, cx);
                e.set_placeholder("teammate@kense.dev");
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ie, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let cur = ed_ent.read(cx).value().to_string();
                    this.invite_error = None;
                    if cur.ends_with(',') || cur.ends_with(' ') {
                        let clean = cur.trim_end_matches([',', ' ']).to_string();
                        if !clean.is_empty() {
                            this.add_invite(&clean, Some(w), cx);
                        }
                    } else {
                        cx.notify();
                    }
                }
                EditEvent::Submitted => {
                    let cur = ed_ent.read(cx).value().trim().to_string();
                    if cur.is_empty() {
                        this.next_or_submit(Some(w), cx);
                    } else {
                        this.add_invite(&cur, Some(w), cx);
                    }
                }
                _ => {}
            })
            .detach();
            self.invite_editor = Some(ie);
        }
    }

    fn render_step_pane(&self, narrow: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);

        if self.completed {
            let plan_title = self
                .plans
                .iter()
                .find(|pl| pl.id == self.values.plan_id)
                .map(|pl| tr(cx, pl.title.as_ref(), pl.title_zh.as_ref()).to_string())
                .unwrap_or_else(|| "Pro".to_string());

            let url_full = format!("https://kense.app/{}", self.values.workspace_slug);
            let url_copy = url_full.clone();
            let recently_copied_url = self
                .copied_url_at
                .is_some_and(|t0| t0.elapsed().as_secs_f32() < 1.8);
            let copy_url_lbl = tr(
                cx,
                if recently_copied_url {
                    "Copied URL"
                } else {
                    "Copy URL"
                },
                if recently_copied_url {
                    "已复制链接"
                } else {
                    "复制链接"
                },
            );
            let copy_url_icon = div()
                .relative()
                .size(px(14.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .opacity(if recently_copied_url { 0.0 } else { 1.0 })
                        .child(pro_icon(ProIcon::Copy, 13.5, p.muted)),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(if recently_copied_url { 1.0 } else { 0.0 })
                        .child(pro_check_draw(
                            pro_eid(&self.id, "copy-ws-chk"),
                            recently_copied_url,
                            14.,
                            false,
                            p.green,
                        )),
                );

            return div()
                .w_full()
                .py(px(26.))
                .px(px(20.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .size(px(40.))
                        .mb(px(4.))
                        .rounded_full()
                        .bg(p.green_soft)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_check_draw(
                            pro_eid(&self.id, "done-hero-chk"),
                            true,
                            22.,
                            false,
                            p.green,
                        )),
                )
                .child(
                    div()
                        .text_size(px(15.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Workspace created", "工作区已创建就绪")),
                )
                .child(div().text_size(px(13.)).text_color(p.text).child(format!(
                    "{} · {} · kense.app/{}",
                    self.values.workspace_name, plan_title, self.values.workspace_slug
                )))
                .child(
                    div()
                        .mt(px(10.))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(8.))
                        .child(
                            pro_tactile(
                                pro_eid(&self.id, "copy-ws-url"),
                                tr(cx, "Copy workspace URL", "复制工作区链接"),
                                6.,
                                if recently_copied_url {
                                    p.green_soft
                                } else {
                                    p.card
                                },
                                p.hover,
                                if recently_copied_url {
                                    p.green.opacity(0.38)
                                } else {
                                    p.line
                                },
                                p.line_strong,
                                recently_copied_url,
                                div()
                                    .h(px(28.))
                                    .px(px(10.))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .whitespace_nowrap()
                                    .child(copy_url_icon)
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "copy-ws-lbl"),
                                        copy_url_lbl,
                                        12.5,
                                        FontWeight::MEDIUM,
                                        if recently_copied_url {
                                            p.green_ink
                                        } else {
                                            p.strong
                                        },
                                    )),
                                cx,
                                move |this, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        url_copy.clone(),
                                    ));
                                    this.copied_url_at = Some(Instant::now());
                                    this.toast = Some((
                                        SharedString::from(tr(
                                            cx,
                                            "Workspace URL copied",
                                            "工作区链接已复制",
                                        )),
                                        Instant::now(),
                                    ));
                                    cx.notify();
                                },
                            )
                            .flex_none(),
                        )
                        .child(pro_btn(
                            pro_eid(&self.id, "reset-wizard"),
                            tr(cx, "Reset", "重新开始"),
                            Some(ProIcon::Refresh),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |this, w, cx| {
                                this.reset(Some(w), cx);
                            },
                        )),
                )
                .into_any_element();
        }

        let Some(meta) = self.steps.get(self.step) else {
            return div().into_any_element();
        };
        let heading = tr(cx, meta.heading.as_ref(), meta.heading_zh.as_ref()).to_string();
        let desc = tr(cx, meta.description.as_ref(), meta.description_zh.as_ref()).to_string();

        let body: AnyElement = match self.step {
            0 => {
                let name_invalid = self.name_error.is_some();
                let slug_invalid = self.slug_error.is_some();
                let name_err_msg = self.name_error.clone().unwrap_or_default();
                let slug_err_msg = self.slug_error.clone().unwrap_or_default();
                let name_dx = self.field_shake_dx("name", cx);
                let slug_dx = self.field_shake_dx("slug", cx);

                let name_ed = self
                    .name_editor
                    .as_ref()
                    .map(|e| div().flex_1().min_w_0().child(e.clone()).into_any_element())
                    .unwrap_or_else(|| div().flex_1().into_any_element());
                let slug_ed = self
                    .slug_editor
                    .as_ref()
                    .map(|e| div().flex_1().min_w_0().child(e.clone()).into_any_element())
                    .unwrap_or_else(|| div().flex_1().into_any_element());

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .mb(px(6.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(tr(cx, "Workspace name", "工作区名称")),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(34.))
                                    .px(px(10.))
                                    .relative()
                                    .left(px(name_dx))
                                    .rounded(px(8.))
                                    .bg(if name_invalid { p.red_soft } else { p.card })
                                    .border_1()
                                    .border_color(if name_invalid {
                                        p.red.opacity(0.55)
                                    } else {
                                        p.line_strong
                                    })
                                    .flex()
                                    .items_center()
                                    .child(name_ed),
                            )
                            .child(pro_fold(
                                pro_eid(&self.id, "name-err-fold"),
                                name_invalid,
                                20.,
                                0.,
                                move || {
                                    div()
                                        .pt(px(5.))
                                        .text_size(px(12.))
                                        .text_color(p.red_ink)
                                        .child(name_err_msg)
                                        .into_any_element()
                                },
                            )),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .mb(px(6.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(tr(cx, "Workspace URL", "工作区访问路径")),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .h(px(34.))
                                    .px(px(10.))
                                    .relative()
                                    .left(px(slug_dx))
                                    .rounded(px(8.))
                                    .bg(if slug_invalid { p.red_soft } else { p.card })
                                    .border_1()
                                    .border_color(if slug_invalid {
                                        p.red.opacity(0.55)
                                    } else {
                                        p.line_strong
                                    })
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(13.))
                                            .text_color(p.muted)
                                            .child("kense.app/"),
                                    )
                                    .child(div().flex_1().min_w_0().h(px(22.)).child(slug_ed)),
                            )
                            .child(if slug_invalid {
                                pro_fold(
                                    pro_eid(&self.id, "slug-err-fold"),
                                    true,
                                    20.,
                                    0.,
                                    move || {
                                        div()
                                            .pt(px(5.))
                                            .text_size(px(12.))
                                            .text_color(p.red_ink)
                                            .child(slug_err_msg)
                                            .into_any_element()
                                    },
                                )
                            } else {
                                div()
                                    .pt(px(5.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Lowercase letters, numbers, and single hyphens.",
                                        "支持小写字母、数字与单个连字符。",
                                    ))
                                    .into_any_element()
                            }),
                    )
                    .into_any_element()
            }
            1 => {
                let inv_invalid = self.invite_error.is_some();
                let inv_err = self.invite_error.clone().unwrap_or_default();
                let inv_dx = self.field_shake_dx("invite", cx);

                let mut chips_wrap = div()
                    .w_full()
                    .min_h(px(38.))
                    .p(px(5.))
                    .relative()
                    .left(px(inv_dx))
                    .rounded(px(8.))
                    .bg(if inv_invalid { p.red_soft } else { p.card })
                    .border_1()
                    .border_color(if inv_invalid {
                        p.red.opacity(0.55)
                    } else {
                        p.line_strong
                    })
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(5.));

                for (idx, email) in self.values.invites.iter().enumerate() {
                    chips_wrap = chips_wrap.child(
                        div()
                            .h(px(24.))
                            .pl(px(8.))
                            .pr(px(4.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(email.clone())
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("rm-inv-{idx}")),
                                format!("Remove {email}"),
                                ProIcon::Close,
                                18.,
                                false,
                                cx,
                                move |this, _, cx| {
                                    this.remove_invite(idx, cx);
                                },
                            )),
                    );
                }

                if let Some(ie) = &self.invite_editor {
                    chips_wrap = chips_wrap.child(
                        div()
                            .flex_1()
                            .min_w(px(150.))
                            .h(px(24.))
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .child(ie.clone()),
                    );
                }

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .mb(px(6.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(tr(cx, "Email addresses", "成员邮箱地址")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(tr(cx, "Optional", "选填")),
                            ),
                    )
                    .child(chips_wrap)
                    .child(pro_fold(
                        pro_eid(&self.id, "inv-err-fold"),
                        inv_invalid,
                        20.,
                        0.,
                        move || {
                            div()
                                .pt(px(5.))
                                .text_size(px(12.))
                                .text_color(p.red_ink)
                                .child(inv_err)
                                .into_any_element()
                        },
                    ))
                    .child(
                        div()
                            .pt(px(6.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "Type an email and press Enter or comma to add a chip.",
                                "输入邮箱后按 Enter 或逗号生成标签，留空可直接继续。",
                            )),
                    )
                    .into_any_element()
            }
            2 => {
                let plan_sensor = pro_selection_sensor();
                let mut choices = div()
                    .w_full()
                    .relative()
                    .flex()
                    .when(narrow, |d| d.flex_col())
                    .gap(px(8.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "plan-ring"),
                        self.values.plan_id.clone(),
                        plan_sensor.clone(),
                        8.,
                        p.accent_soft,
                        Some(p.blue.opacity(0.45)),
                        false,
                    ));

                for plan in &self.plans {
                    let selected = plan.id == self.values.plan_id;
                    let pid = plan.id.clone();
                    let ptitle = tr(cx, plan.title.as_ref(), plan.title_zh.as_ref()).to_string();
                    let pdesc =
                        tr(cx, plan.description.as_ref(), plan.description_zh.as_ref()).to_string();

                    choices = choices.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("plan-card-{pid}")))
                            .role(Role::Button)
                            .aria_label(format!("{ptitle} {}", plan.price))
                            .tab_index(0)
                            .flex_1()
                            .min_w_0()
                            .p(px(12.))
                            .relative()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(if selected {
                                transparent_black()
                            } else {
                                p.line
                            })
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .cursor_pointer()
                            .when(!selected, move |d| d.hover(move |s| s.bg(p.hover)))
                            .focus_visible(move |s| s.border_color(p.focus))
                            .when(selected, |d| d.child(pro_mark_selected(&plan_sensor)))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(ptitle),
                                    )
                                    .when_some(plan.badge.clone(), |d, b| {
                                        d.child(
                                            div()
                                                .h(px(16.))
                                                .px(px(5.))
                                                .rounded(px(4.))
                                                .bg(p.accent_soft)
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.blue)
                                                .child(b),
                                        )
                                    })
                                    .child(
                                        div()
                                            .ml_auto()
                                            .size(px(14.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(if selected {
                                                p.blue
                                            } else {
                                                p.line_strong
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(selected, |d| {
                                                d.child(
                                                    div().size(px(7.)).rounded_full().bg(p.blue),
                                                )
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(plan.price.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .line_height(px(16.))
                                    .text_color(p.text)
                                    .child(pdesc),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.select_plan(&pid, cx);
                        },
                    ));
                }
                choices.into_any_element()
            }
            _ => {
                let plan_summary = self
                    .plans
                    .iter()
                    .find(|pl| pl.id == self.values.plan_id)
                    .map(|pl| {
                        format!(
                            "{} · {}",
                            tr(cx, pl.title.as_ref(), pl.title_zh.as_ref()),
                            pl.price
                        )
                    })
                    .unwrap_or_else(|| self.values.plan_id.to_string());

                let team_summary = if self.values.invites.is_empty() {
                    tr(cx, "Just you for now", "暂不邀请其他成员").to_string()
                } else {
                    self.values
                        .invites
                        .iter()
                        .map(SharedString::as_ref)
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let review_rows: [(SharedString, SharedString, usize); 3] = [
                    (
                        SharedString::from(tr(cx, "Workspace", "工作区")),
                        SharedString::from(format!(
                            "{} (kense.app/{})",
                            self.values.workspace_name, self.values.workspace_slug
                        )),
                        0,
                    ),
                    (
                        SharedString::from(tr(cx, "Team", "团队成员")),
                        SharedString::from(team_summary),
                        1,
                    ),
                    (
                        SharedString::from(tr(cx, "Plan", "订阅方案")),
                        SharedString::from(plan_summary),
                        2,
                    ),
                ];

                let mut box_el = div()
                    .w_full()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.subtle)
                    .flex()
                    .flex_col();

                for (r_idx, (label, val, step_target)) in review_rows.into_iter().enumerate() {
                    box_el = box_el.child(
                        div()
                            .w_full()
                            .py(px(10.))
                            .pl(px(12.))
                            .pr(px(8.))
                            .when(r_idx > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(84.))
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(val),
                            )
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("rev-edit-{r_idx}")),
                                    tr(cx, "Edit step", "返回修改"),
                                    5.,
                                    transparent_black(),
                                    p.hover,
                                    transparent_black(),
                                    transparent_black(),
                                    false,
                                    div()
                                        .h(px(24.))
                                        .px(px(8.))
                                        .flex()
                                        .items_center()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.blue)
                                        .whitespace_nowrap()
                                        .child(tr(cx, "Edit", "修改")),
                                    cx,
                                    move |this, _, cx| {
                                        this.go_to_step(step_target, cx);
                                    },
                                )
                                .flex_none(),
                            ),
                    );
                }

                box_el.into_any_element()
            }
        };

        div()
            .w_full()
            .p(px(20.))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(
                div()
                    .mt(px(2.))
                    .text_size(px(13.))
                    .text_color(p.text)
                    .child(desc),
            )
            .child(div().mt(px(16.)).w_full().child(body))
            .into_any_element()
    }
}

impl Render for StepperForm {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let narrow = width_cell.get() < 420.;

        let sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 10. {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        if self.loading {
            let phase = if disabled_motion(cx) { 0.3 } else { 0.5 };
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(500.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(20.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(pro_skeleton_bar(440., 18., 4., phase, cx))
                .child(pro_skeleton_bar(220., 16., 4., phase, cx))
                .child(pro_skeleton_bar(440., 36., 8., phase, cx))
                .child(pro_skeleton_bar(440., 36., 8., phase, cx))
                .into_any_element();
        }

        if self.empty || self.steps.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(500.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Tasks,
                    tr(cx, "No wizard steps configured", "尚未配置向导步骤"),
                    tr(
                        cx,
                        "Provide onboarding steps to guide users through setup.",
                        "请添加步骤以引导用户完成初始化设置。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-restore"),
                        SharedString::from(tr(cx, "Load 4-step wizard", "加载默认四步向导")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.steps.is_empty() {
                                this.steps = default_step_metas();
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        if ((self
            .shake_at
            .as_ref()
            .is_some_and(|(_, t0)| t0.elapsed().as_secs_f32() < 0.40))
            || (self
                .copied_url_at
                .as_ref()
                .is_some_and(|t0| t0.elapsed().as_secs_f32() < 1.8)))
            && !disabled_motion(cx)
        {
            w.request_animation_frame();
        }

        // Keep hero check state mounted across completion transitions so stroke animates on complete
        let pre_hero_slot = if !self.completed {
            div()
                .size(px(0.))
                .overflow_hidden()
                .opacity(0.)
                .child(pro_check_draw(
                    pro_eid(&self.id, "done-hero-chk"),
                    false,
                    22.,
                    false,
                    p.green,
                ))
        } else {
            div()
        };

        // Top step indicator bar
        let total_steps = self.steps.len();
        let top_progress = if narrow && !self.completed {
            let cur_title = self
                .steps
                .get(self.step)
                .map(|m| tr(cx, m.title.as_ref(), m.title_zh.as_ref()))
                .unwrap_or("");
            div()
                .w_full()
                .pt(px(16.))
                .px(px(20.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(10.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(
                    tr(
                        cx,
                        &format!("STEP {} OF {total_steps}", self.step + 1),
                        &format!("第 {} / {total_steps} 步", self.step + 1),
                    )
                    .to_string(),
                )
                .child(div().text_color(p.faint).child("·"))
                .child(div().text_color(p.strong).child(cur_title.to_uppercase()))
                .into_any_element()
        } else {
            let mut step_strip = div()
                .w_full()
                .pt(px(16.))
                .px(px(20.))
                .flex()
                .items_center()
                .gap(px(8.));

            for (idx, sm) in self.steps.iter().enumerate() {
                let is_done = self.completed || idx < self.step;
                let is_current = !self.completed && idx == self.step;
                let fill_target = if is_done {
                    1.0
                } else if is_current {
                    0.55
                } else {
                    0.0
                };

                let stitle = tr(cx, sm.title.as_ref(), sm.title_zh.as_ref()).to_uppercase();
                let mark_el = div()
                    .relative()
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("step-chk-{idx}")),
                        is_done,
                        15.,
                        false,
                        p.blue,
                    ))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .opacity(if is_done { 0.0 } else { 1.0 })
                            .text_size(px(9.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if is_current { p.blue } else { p.faint })
                            .child(format!("{}", idx + 1)),
                    )
                    .into_any_element();

                let step_inner = div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_spring_bar(
                        pro_eid(&self.id, format!("sbar-fill-{idx}")),
                        fill_target,
                        2.5,
                        1.25,
                        p.line,
                        p.blue,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .min_w_0()
                            .child(mark_el)
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if is_current {
                                        p.strong
                                    } else if is_done {
                                        p.text
                                    } else {
                                        p.muted
                                    })
                                    .child(stitle.clone()),
                            ),
                    );

                let clickable_back = !self.completed && idx < self.step;
                let cell = if clickable_back {
                    pro_tactile(
                        pro_eid(&self.id, format!("step-jump-{idx}")),
                        format!("Jump back to {stitle}"),
                        6.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        false,
                        div().w_full().p(px(3.)).child(step_inner),
                        cx,
                        move |this, _, cx| {
                            this.go_to_step(idx, cx);
                        },
                    )
                    .flex_1()
                    .min_w_0()
                    .m(px(-3.))
                    .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(step_inner)
                        .into_any_element()
                };
                step_strip = step_strip.child(cell);
            }
            step_strip.into_any_element()
        };

        // Smooth measured height morph + directional horizontal step transition
        let tx_index = if self.completed {
            total_steps as i32
        } else {
            self.step as i32
        };
        let pane_content = self.render_step_pane(narrow, cx);
        let transitioned_pane =
            pro_step_transition(pro_eid(&self.id, "pane-stx"), tx_index, pane_content);

        let measured_h = self.pane_height_cell.get();
        if (self.last_height_target - measured_h).abs() > 0.5 {
            self.last_height_target = measured_h;
            self.height_slide.duration(measured_h, 0.28, cx);
        }
        let anim_h = self.height_slide.get(w, cx);
        let height_animating = !disabled_motion(cx) && (anim_h - measured_h).abs() > 1.0;

        let h_cell = self.pane_height_cell.clone();
        let viewport = div()
            .w_full()
            .when(height_animating, |d| {
                d.h(px(anim_h.max(80.))).overflow_hidden()
            })
            .child(
                div()
                    .w_full()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            let h = f32::from(b.size.height);
                            if h > 40. {
                                h_cell.set(h);
                            }
                        }
                    })
                    .child(transitioned_pane),
            );

        // Footer with Back & Continue / Create workspace (folded out on completion)
        let is_last = self.step + 1 >= total_steps;
        let step_valid = self.is_current_step_valid();
        let primary_label = if is_last {
            tr(
                cx,
                self.submit_label.as_ref(),
                self.submit_label_zh.as_ref(),
            )
            .to_string()
        } else {
            tr(cx, "Continue", "下一步").to_string()
        };
        let can_go_back = self.step > 0;

        let back_btn = if can_go_back {
            pro_btn(
                pro_eid(&self.id, "foot-back"),
                tr(cx, "Back", "上一步"),
                Some(ProIcon::ChevronLeft),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.prev_step(cx);
                },
            )
            .into_any_element()
        } else {
            div().into_any_element()
        };

        let next_btn = div().opacity(if step_valid { 1.0 } else { 0.45 }).child(
            pro_tactile(
                pro_eid(&self.id, "foot-next"),
                primary_label.clone(),
                6.,
                p.primary,
                if p.dark {
                    hsla(0., 0., 1., 0.88)
                } else {
                    hsla(0., 0., 0., 0.84)
                },
                transparent_black(),
                transparent_black(),
                false,
                div()
                    .h(px(28.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .whitespace_nowrap()
                    .child(pro_swap_label(
                        pro_eid(&self.id, "foot-next-lbl"),
                        primary_label,
                        12.5,
                        FontWeight::MEDIUM,
                        p.on_primary,
                    )),
                cx,
                |this, w, cx| {
                    this.next_or_submit(Some(w), cx);
                },
            )
            .flex_none(),
        );

        let footer_fold = pro_fold(
            pro_eid(&self.id, "foot-fold"),
            !self.completed,
            48.,
            0.,
            move || {
                div()
                    .w_full()
                    .h(px(48.))
                    .px(px(14.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(back_btn)
                    .child(next_btn)
                    .into_any_element()
            },
        );

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if !this.completed && this.step == 2 {
                    let len = this.plans.len();
                    if len > 0
                        && let Some(cur_idx) = this
                            .plans
                            .iter()
                            .position(|pl| pl.id == this.values.plan_id)
                    {
                        match ev.keystroke.key.as_str() {
                            "left" => {
                                let prev = (cur_idx + len - 1) % len;
                                let pid = this.plans[prev].id.to_string();
                                this.select_plan(&pid, cx);
                            }
                            "right" => {
                                let next = (cur_idx + 1) % len;
                                let pid = this.plans[next].id.to_string();
                                this.select_plan(&pid, cx);
                            }
                            _ => {}
                        }
                    }
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(sensor)
            .child(pre_hero_slot)
            .child(
                div()
                    .w_full()
                    .max_w(px(500.))
                    .mx_auto()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(top_progress)
                    .child(viewport)
                    .child(footer_fold),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn default_step_metas() -> Vec<StepperStepMeta> {
    vec![
        StepperStepMeta {
            id: "workspace".into(),
            title: "Workspace".into(),
            title_zh: "工作区".into(),
            heading: "Name your workspace".into(),
            heading_zh: "命名你的工作区".into(),
            description: "This is where your team's projects live.".into(),
            description_zh: "团队的所有项目与组件都将汇聚于此。".into(),
        },
        StepperStepMeta {
            id: "team".into(),
            title: "Team".into(),
            title_zh: "团队".into(),
            heading: "Invite teammates".into(),
            heading_zh: "邀请团队成员".into(),
            description: "Collaborate in real time — you can always invite more people later."
                .into(),
            description_zh: "输入邮箱添加协作成员，也可以稍后再邀请。".into(),
        },
        StepperStepMeta {
            id: "plan".into(),
            title: "Plan".into(),
            title_zh: "方案".into(),
            heading: "Select a plan".into(),
            heading_zh: "选择订阅方案".into(),
            description: "Start free or unlock unlimited workspaces and SSO.".into(),
            description_zh: "按需选择方案，随时可在计费设置中变更。".into(),
        },
        StepperStepMeta {
            id: "review".into(),
            title: "Review".into(),
            title_zh: "确认".into(),
            heading: "Review setup".into(),
            heading_zh: "核对创建信息".into(),
            description: "Confirm your workspace configuration before launching.".into(),
            description_zh: "确认无误后点击创建即可立即启用工作区。".into(),
        },
    ]
}

fn default_plan_options() -> Vec<StepperPlanOption> {
    vec![
        StepperPlanOption::new(
            "starter",
            "Starter",
            "$0 / mo",
            "Up to 3 members and 5 active projects.",
        )
        .with_zh("基础版", "最多支持 3 位成员与 5 个活跃项目。"),
        StepperPlanOption::new(
            "pro",
            "Pro",
            "$29 / mo",
            "Unlimited projects, SSO, and priority builds.",
        )
        .with_zh("专业版", "无限项目、单点登录与高优构建队列。")
        .with_badge("Popular"),
        StepperPlanOption::new(
            "enterprise",
            "Scale",
            "$99 / mo",
            "Audit logs, dedicated region, and custom SLA.",
        )
        .with_zh("企业版", "完整审计日志、专属区域与定制 SLA。"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{StepperForm, slugify_workspace};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn slugify_converts_workspace_names_to_clean_urls() {
        assert_eq!(slugify_workspace("Lumen Studio!"), "lumen-studio");
        assert_eq!(slugify_workspace("  KUI   Pro  2026 "), "kui-pro-2026");
    }

    #[gpui::test]
    fn validation_blocks_invalid_step_and_completes_with_reset(cx: &mut TestAppContext) {
        let wizard = cx.new(|cx| StepperForm::new("test-stepper-form", cx));

        wizard.update(cx, |sf, cx| {
            assert_eq!(sf.step(), 0);
            // Clearing workspace name blocks Continue
            sf.set_workspace_name("", None, cx);
            assert!(!sf.is_current_step_valid());
            assert!(!sf.next_or_submit(None, cx));
            assert_eq!(sf.step(), 0);

            // Restoring valid name auto-slugifies and advances to Team
            sf.set_workspace_name("Kense Cloud", None, cx);
            assert_eq!(sf.values().workspace_slug.as_ref(), "kense-cloud");
            assert!(sf.next_or_submit(None, cx));
            assert_eq!(sf.step(), 1);

            // Invalid email rejected, valid email chip added
            assert!(!sf.add_invite("not-an-email", None, cx));
            assert!(sf.add_invite("leo@kense.dev", None, cx));
            assert_eq!(sf.values().invites.len(), 2);

            // Advance through Plan -> Review -> Complete -> Reset
            assert!(sf.next_or_submit(None, cx));
            assert_eq!(sf.step(), 2);
            assert!(sf.select_plan("enterprise", cx));
            assert!(sf.next_or_submit(None, cx));
            assert_eq!(sf.step(), 3);
            assert!(sf.go_to_step(0, cx));
            assert_eq!(sf.step(), 0);
            assert!(sf.next_or_submit(None, cx));
            assert!(sf.next_or_submit(None, cx));
            assert!(sf.next_or_submit(None, cx));
            assert!(sf.next_or_submit(None, cx));
            assert!(sf.is_completed());

            sf.reset(None, cx);
            assert!(!sf.is_completed());
            assert_eq!(sf.step(), 0);
        });
    }
}
