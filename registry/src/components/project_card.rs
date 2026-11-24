//! Two-line project summary card that unfolds a segmented ring, people filter, and subtasks.
//!
//! Renders a compact two-line project header with a multi-arc `PathBuilder`
//! progress ring (one arc segment per subtask), overlapping [`pro_person_avatar`]
//! teammate filters, an inline status picker (`On track`, `At risk`, `Off track`,
//! `Completed`), and a cross-linked subtask checklist that unfolds via [`pro_fold`].

use crate::{
    components::pro_chrome::{
        ProPalette, pro_check_draw, pro_eid, pro_empty_state, pro_fold, pro_mark_selected,
        pro_mix_color, pro_person_avatar, pro_pill_indicator, pro_rolling_number,
        pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_swap_label,
        pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    f32::consts::{FRAC_PI_2, TAU},
    rc::Rc,
};
use web_time::Instant;

const ICON_HALF_CIRCLE: &str = r#"<circle cx="6" cy="6" r="4.25" stroke="currentColor" stroke-width="1.25"/><path d="M6 3.3a2.7 2.7 0 0 1 0 5.4V3.3Z" fill="currentColor"/>"#;
const ICON_ARROW_RIGHT_SM: &str = r#"<path d="M2.5 6h7M6.5 3l3 3-3 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#;

/// Unfold interaction mode for [`ProjectCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectCardTrigger {
    /// Unfolds quietly on pointer hover and can also be toggled via click or keyboard.
    #[default]
    Hover,
    /// Unfolds only when explicitly clicked or activated via `Enter` / `Space`.
    Click,
}

/// Canonical project health status shown in [`ProjectCard`]'s inline picker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectStatus {
    #[default]
    OnTrack,
    AtRisk,
    OffTrack,
    Completed,
}

impl ProjectStatus {
    /// Ordered options shown inside the inline status picker.
    pub const ALL: [Self; 4] = [Self::OnTrack, Self::AtRisk, Self::OffTrack, Self::Completed];

    /// Localized status label.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::OnTrack => tr(cx, "On track", "进展正常"),
            Self::AtRisk => tr(cx, "At risk", "存在风险"),
            Self::OffTrack => tr(cx, "Off track", "进度滞后"),
            Self::Completed => tr(cx, "Completed", "已完成"),
        }
    }

    fn colors(self, p: &ProPalette) -> (Hsla, Hsla) {
        match self {
            Self::OnTrack => (p.green_ink, p.green_soft),
            Self::AtRisk => (p.amber_ink, p.amber_soft),
            Self::OffTrack => (p.red_ink, p.red_soft),
            Self::Completed => {
                let fg = if p.dark {
                    rgb(0x4fb0fe).into()
                } else {
                    rgba(0x0060e2e6).into()
                };
                (fg, p.accent_soft)
            }
        }
    }
}

/// Single subtask represented by one arc segment on [`ProjectCard`]'s progress ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectSubtask {
    pub id: SharedString,
    pub label: SharedString,
    pub done: bool,
    pub assignee: Option<SharedString>,
}

impl ProjectSubtask {
    /// Construct a [`ProjectSubtask`] with `id` and `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            done: false,
            assignee: None,
        }
    }

    /// Set whether this subtask is completed.
    pub fn with_done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    /// Link this subtask to a teammate name in `members`.
    pub fn with_assignee(mut self, assignee: impl Into<SharedString>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }
}

/// Typed events emitted by [`ProjectCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectCardEvent {
    /// Card expanded (`true`) or collapsed (`false`).
    OpenChanged(bool),
    /// Subtask completion toggled; carries `(id, done, all_tasks)`.
    TaskToggled {
        id: SharedString,
        done: bool,
        tasks: Vec<ProjectSubtask>,
    },
    /// Project health status changed from the inline status picker.
    StatusChanged(ProjectStatus),
    /// Teammate filter pinned (`Some(name)`) or cleared (`None`).
    AssigneeFilterChanged(Option<SharedString>),
    /// Footer primary action button (`"Open"`) clicked.
    ActionClicked,
}

/// Default 5 subtasks (`3 of 5` completed) assigned across `Mara`, `Jonas`, and `Priya`.
pub fn default_project_subtasks(zh: bool) -> Vec<ProjectSubtask> {
    if zh {
        vec![
            ProjectSubtask::new("welcome", "欢迎引导页与品牌主题适配")
                .with_done(true)
                .with_assignee("Mara"),
            ProjectSubtask::new("workspace", "工作区初始化接口联调")
                .with_done(true)
                .with_assignee("Jonas"),
            ProjectSubtask::new("oauth", "第三方 OAuth 权限范围校验")
                .with_done(true)
                .with_assignee("Priya"),
            ProjectSubtask::new("invites", "成员邮件邀请模板走查")
                .with_done(false)
                .with_assignee("Mara"),
            ProjectSubtask::new("rollout", "生产环境灰度发布核对单")
                .with_done(false)
                .with_assignee("Jonas"),
        ]
    } else {
        vec![
            ProjectSubtask::new("welcome", "Welcome screen & brand polish")
                .with_done(true)
                .with_assignee("Mara"),
            ProjectSubtask::new("workspace", "Workspace provisioning API")
                .with_done(true)
                .with_assignee("Jonas"),
            ProjectSubtask::new("oauth", "OAuth connector permissions")
                .with_done(true)
                .with_assignee("Priya"),
            ProjectSubtask::new("invites", "Invite teammates email flow")
                .with_done(false)
                .with_assignee("Mara"),
            ProjectSubtask::new("rollout", "Production rollout checklist")
                .with_done(false)
                .with_assignee("Jonas"),
        ]
    }
}

/// Map a local point `(dx, dy)` relative to the ring center onto a subtask segment `0..count`.
pub fn hit_test_ring_segment(dx: f32, dy: f32, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let dist = (dx * dx + dy * dy).sqrt();
    if !(7.0..=17.5).contains(&dist) {
        return None;
    }
    let mut angle = dy.atan2(dx) + FRAC_PI_2;
    if angle < 0.0 {
        angle += TAU;
    }
    let frac = (angle / TAU).clamp(0., 0.9999);
    Some(((frac * count as f32).floor() as usize).min(count - 1))
}

/// Two-line compact project card unfolding a segmented progress ring, people filter, and status picker.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::project_card::{ProjectCard, ProjectCardEvent, ProjectStatus};
///
/// # fn doc(cx: &mut gpui::App) {
/// let card = cx.new(|cx| {
///     ProjectCard::new("onboarding-project", cx)
///         .with_title("Onboarding flow")
///         .with_status(ProjectStatus::OnTrack)
///         .with_due("Oct 24")
/// });
/// cx.subscribe(&card, |_, ev: &ProjectCardEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct ProjectCard {
    id: ElementId,
    title: Option<SharedString>,
    due: Option<SharedString>,
    action_label: Option<SharedString>,
    status: ProjectStatus,
    members: Vec<SharedString>,
    tasks: Vec<ProjectSubtask>,
    custom_tasks: bool,
    tasks_lang_zh: bool,
    trigger: ProjectCardTrigger,
    pinned_open: bool,
    hovered_card: bool,
    status_picker_open: bool,
    hovered_task_idx: Option<usize>,
    hovered_member: Option<SharedString>,
    pinned_member: Option<SharedString>,
    ring_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl EventEmitter<ProjectCardEvent> for ProjectCard {}

impl ProjectCard {
    /// Construct a [`ProjectCard`] (`"Onboarding flow"`, 5 subtasks across 3 teammates, `On track`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        Self {
            id: id.into(),
            title: None,
            due: None,
            action_label: None,
            status: ProjectStatus::OnTrack,
            members: vec!["Mara".into(), "Jonas".into(), "Priya".into()],
            tasks: default_project_subtasks(zh),
            custom_tasks: false,
            tasks_lang_zh: zh,
            trigger: ProjectCardTrigger::Hover,
            pinned_open: false,
            hovered_card: false,
            status_picker_open: false,
            hovered_task_idx: None,
            hovered_member: None,
            pinned_member: None,
            ring_bounds: Rc::new(Cell::new(None)),
            measured_width: Rc::new(Cell::new(320.)),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the project title.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the due date string (e.g. `"Oct 24"`).
    pub fn with_due(mut self, due: impl Into<SharedString>) -> Self {
        self.due = Some(due.into());
        self
    }

    /// Set the initial [`ProjectStatus`] (`OnTrack`, `AtRisk`, `OffTrack`, `Completed`).
    pub fn with_status(mut self, status: ProjectStatus) -> Self {
        self.status = status;
        self
    }

    /// Override the teammate names rendered as overlapping avatars in the header.
    pub fn with_members<I, S>(mut self, members: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.members = members.into_iter().map(Into::into).collect();
        self
    }

    /// Replace the project subtasks (one progress ring segment and checklist row each).
    pub fn with_tasks(mut self, tasks: Vec<ProjectSubtask>) -> Self {
        self.tasks = tasks;
        self.custom_tasks = true;
        self
    }

    /// Override the footer call-to-action label (`"Open"` by default).
    pub fn with_action_label(mut self, label: impl Into<SharedString>) -> Self {
        self.action_label = Some(label.into());
        self
    }

    /// Choose whether the card unfolds on pointer hover or only on click / `Enter`.
    pub fn with_trigger(mut self, trigger: ProjectCardTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Start with the card already unfolded.
    pub fn with_default_open(mut self, open: bool) -> Self {
        self.pinned_open = open;
        self
    }

    /// Enable or disable the loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state override.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return whether the card body is currently unfolded.
    pub fn is_open(&self) -> bool {
        self.pinned_open || (self.trigger == ProjectCardTrigger::Hover && self.hovered_card)
    }

    /// Return the current [`ProjectStatus`].
    pub fn status(&self) -> ProjectStatus {
        self.status
    }

    /// Return the current subtasks.
    pub fn tasks(&self) -> &[ProjectSubtask] {
        &self.tasks
    }

    /// Return `(completed_count, total_count)` across all subtasks.
    pub fn progress_counts(&self) -> (usize, usize) {
        let done = self.tasks.iter().filter(|t| t.done).count();
        (done, self.tasks.len())
    }

    /// Return the active teammate filter (`hovered_member` previews, otherwise `pinned_member`).
    pub fn active_member_filter(&self) -> Option<&SharedString> {
        self.hovered_member.as_ref().or(self.pinned_member.as_ref())
    }

    fn sync_language_defaults(&mut self, cx: &App) {
        if self.custom_tasks {
            return;
        }
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        if self.tasks_lang_zh != zh {
            let states: Vec<(SharedString, bool)> =
                self.tasks.iter().map(|t| (t.id.clone(), t.done)).collect();
            let mut next = default_project_subtasks(zh);
            for t in &mut next {
                if let Some((_, d)) = states.iter().find(|(id, _)| id == &t.id) {
                    t.done = *d;
                }
            }
            self.tasks = next;
            self.tasks_lang_zh = zh;
        }
    }

    /// Toggle whether the card is pinned open.
    pub fn toggle_open(&mut self, cx: &mut Context<Self>) {
        self.pinned_open = !self.pinned_open;
        if !self.pinned_open {
            self.status_picker_open = false;
        }
        cx.emit(ProjectCardEvent::OpenChanged(self.is_open()));
        cx.notify();
    }

    /// Toggle completion of subtask at `idx`.
    pub fn toggle_task(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(task) = self.tasks.get_mut(idx) else {
            return;
        };
        task.done = !task.done;
        let done = task.done;
        let id = task.id.clone();
        let snapshot = self.tasks.clone();
        cx.emit(ProjectCardEvent::TaskToggled {
            id,
            done,
            tasks: snapshot,
        });
        cx.notify();
    }

    /// Update the project status and close the inline status picker.
    pub fn set_status(&mut self, status: ProjectStatus, cx: &mut Context<Self>) {
        self.status = status;
        self.status_picker_open = false;
        cx.emit(ProjectCardEvent::StatusChanged(status));
        cx.notify();
    }

    /// Pin or unpin the teammate filter for `member`.
    pub fn toggle_member_pin(&mut self, member: SharedString, cx: &mut Context<Self>) {
        if self.pinned_member.as_ref() == Some(&member) {
            self.pinned_member = None;
        } else {
            self.pinned_member = Some(member);
        }
        cx.emit(ProjectCardEvent::AssigneeFilterChanged(
            self.pinned_member.clone(),
        ));
        cx.notify();
    }
}

impl Render for ProjectCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_language_defaults(cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let narrow = width < 290.;

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            let phase = (self.started_at.elapsed().as_secs_f32() / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(332.))
                        .p(px(12.))
                        .rounded(px(10.))
                        .bg(if p.dark { rgb(0x1c1c1e) } else { rgb(0xffffff) })
                        .border_1()
                        .border_color(p.line)
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(32., 32., 16., phase, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(pro_skeleton_bar(140., 13., 4., phase + 0.1, cx))
                                .child(pro_skeleton_bar(110., 12., 4., phase + 0.2, cx)),
                        )
                        .child(pro_skeleton_bar(48., 20., 10., phase + 0.28, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(332.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x1c1c1e) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::ProjectFolder,
                    tr(cx, "No project milestones", "暂无项目子任务"),
                    tr(
                        cx,
                        "Add subtasks and teammates to track segmented progress inline.",
                        "添加子任务与协作成员后即可查看分段圆环进度。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let ring_track: Hsla = if p.dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.09)
        };
        let ring_hot_track: Hsla = if p.dark {
            hsla(0., 0., 1., 0.28)
        } else {
            hsla(0., 0., 0., 0.22)
        };
        let accent_col: Hsla = if self.status == ProjectStatus::Completed {
            p.green
        } else if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgba(0x0060e2d8).into()
        };

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Onboarding flow", "新用户引导改版").into());
        let due_raw: SharedString = self
            .due
            .clone()
            .unwrap_or_else(|| tr(cx, "Oct 24", "10 月 24 日").into());
        let due_display: SharedString = if narrow {
            due_raw
        } else if matches!(UiLanguage::current(cx), UiLanguage::Chinese) {
            format!("截止 {due_raw}").into()
        } else {
            format!("Due {due_raw}").into()
        };

        let open = self.is_open();
        let (done_count, total_count) = self.progress_counts();
        let active_filter = self.active_member_filter().cloned();
        let hovered_task_assignee = self
            .hovered_task_idx
            .and_then(|idx| self.tasks.get(idx))
            .and_then(|t| t.assignee.clone());

        // Pre-sample per-segment completion, stroke width & opacity springs for the multi-arc ring
        let seg_count = self.tasks.len();
        let mut seg_specs: Vec<(f32, f32, f32)> = Vec::with_capacity(seg_count);
        for (i, t) in self.tasks.iter().enumerate() {
            let is_hot = self.hovered_task_idx == Some(i);
            let is_dim = if let Some(ref flt) = active_filter {
                t.assignee.as_ref() != Some(flt)
            } else {
                false
            };
            let done_t = pro_spring_f32(
                pro_eid(&self.id, format!("seg-d-{i}")),
                if t.done { 1.0 } else { 0.0 },
                0.26,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let sw = pro_spring_f32(
                pro_eid(&self.id, format!("seg-w-{i}")),
                if is_hot { 4.25 } else { 3.0 },
                0.22,
                w,
                cx,
            );
            let alpha = pro_spring_f32(
                pro_eid(&self.id, format!("seg-a-{i}")),
                if is_dim { 0.28 } else { 1.0 },
                0.22,
                w,
                cx,
            )
            .clamp(0.15, 1.0);
            seg_specs.push((done_t, sw, alpha));
        }

        let ring_bounds_cell = self.ring_bounds.clone();
        let ring_bounds_for_move = self.ring_bounds.clone();
        let ring_bounds_for_click = self.ring_bounds.clone();
        let hot_idx = self.hovered_task_idx;

        let ring_canvas = canvas(
            move |bounds, _, _| {
                ring_bounds_cell.set(Some(bounds));
            },
            move |bounds, _, window, _| {
                let center = bounds.center();
                let r = 12.5f32;
                let n = seg_specs.len();
                if n == 0 {
                    return;
                }
                let gap_rad = if n == 1 {
                    0.0
                } else {
                    (0.24f32).min(TAU / (n as f32 * 3.2))
                };
                let slice = TAU / n as f32;
                for (i, &(done_t, sw, alpha)) in seg_specs.iter().enumerate() {
                    let a0 = -FRAC_PI_2 + i as f32 * slice + gap_rad * 0.5;
                    let a1 = -FRAC_PI_2 + (i + 1) as f32 * slice - gap_rad * 0.5;
                    if a1 <= a0 {
                        continue;
                    }
                    let base_track = if hot_idx == Some(i) {
                        ring_hot_track
                    } else {
                        ring_track
                    };
                    let col = pro_mix_color(base_track, accent_col, done_t).opacity(alpha);
                    let steps = 14usize;
                    let mut pb = PathBuilder::stroke(px(sw));
                    for k in 0..=steps {
                        let a = a0 + (a1 - a0) * (k as f32 / steps as f32);
                        let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                        if k == 0 {
                            pb.move_to(pt);
                        } else {
                            pb.line_to(pt);
                        }
                    }
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, col);
                    }
                }
            },
        )
        .size(px(32.));

        let ring_widget = div()
            .id(pro_eid(&self.id, "seg-ring"))
            .size(px(32.))
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_move(cx.listener(move |v, ev: &MouseMoveEvent, _, cx| {
                if let Some(b) = ring_bounds_for_move.get() {
                    let c = b.center();
                    let dx = f32::from(ev.position.x - c.x);
                    let dy = f32::from(ev.position.y - c.y);
                    let hit = hit_test_ring_segment(dx, dy, v.tasks.len());
                    if v.hovered_task_idx != hit {
                        v.hovered_task_idx = hit;
                        cx.notify();
                    }
                }
            }))
            .on_hover(cx.listener(|v, inside: &bool, _, cx| {
                if !*inside && v.hovered_task_idx.is_some() {
                    v.hovered_task_idx = None;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |v, ev: &ClickEvent, _, cx| {
                cx.stop_propagation();
                if let Some(b) = ring_bounds_for_click.get() {
                    let pos = ev.position();
                    let c = b.center();
                    let dx = f32::from(pos.x - c.x);
                    let dy = f32::from(pos.y - c.y);
                    if let Some(seg_idx) = hit_test_ring_segment(dx, dy, v.tasks.len()) {
                        v.toggle_task(seg_idx, cx);
                        return;
                    }
                }
                v.toggle_open(cx);
            }))
            .child(div().absolute().inset_0().child(ring_canvas))
            .child(pro_rolling_number(
                pro_eid(&self.id, "done-num"),
                done_count as f64,
                0,
                "",
                "",
                12.,
                FontWeight::MEDIUM,
                p.strong,
            ));

        // Status pill in header
        let (st_fg, st_bg) = self.status.colors(&p);
        let st_lbl: SharedString = self.status.label(cx).into();
        let status_pill = div()
            .id(pro_eid(&self.id, "status-pill"))
            .role(Role::Button)
            .aria_label(st_lbl.clone())
            .aria_expanded(self.status_picker_open)
            .tab_index(0)
            .h(px(20.))
            .pl(px(5.))
            .pr(px(6.))
            .flex_none()
            .rounded(px(5.))
            .bg(st_bg)
            .when(self.status_picker_open, |d| {
                d.border_1().border_color(st_fg.opacity(0.38))
            })
            .flex()
            .items_center()
            .gap(px(4.5))
            .text_size(px(12.))
            .line_height(px(16.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(st_fg)
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                    w.prevent_default();
                    cx.stop_propagation();
                    v.pinned_open = true;
                    v.status_picker_open = !v.status_picker_open;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|v, _, _, cx| {
                cx.stop_propagation();
                v.pinned_open = true;
                v.status_picker_open = !v.status_picker_open;
                cx.notify();
            }))
            .child(pro_svg("0 0 12 12", ICON_HALF_CIRCLE, 12., st_fg))
            .child(pro_swap_label(
                pro_eid(&self.id, "st-pill-lbl"),
                st_lbl,
                12.,
                FontWeight::MEDIUM,
                st_fg,
            ));

        // Overlapping teammate avatars
        let mut people_row = div().flex_none().flex().items_center();
        for (m_idx, member) in self.members.iter().enumerate() {
            let m_name = member.clone();
            let is_pinned = self.pinned_member.as_ref() == Some(&m_name);
            let is_spotlight = active_filter.as_ref() == Some(&m_name)
                || hovered_task_assignee.as_ref() == Some(&m_name);
            let is_dimmed =
                (active_filter.is_some() || hovered_task_assignee.is_some()) && !is_spotlight;

            let m_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("mem-a-{m_idx}")),
                if is_dimmed { 0.38 } else { 1.0 },
                0.20,
                w,
                cx,
            )
            .clamp(0.2, 1.0);

            let aria_lbl: SharedString = format!("Filter by {m_name}").into();
            let m_hover = m_name.clone();
            let m_click = m_name.clone();
            let m_key = m_name.clone();

            people_row = people_row.child(
                div()
                    .id(pro_eid(&self.id, format!("mem-btn-{m_idx}")))
                    .role(Role::Button)
                    .aria_label(aria_lbl)
                    .tab_index(0)
                    .when(m_idx > 0, |d| d.ml(px(if narrow { -7. } else { -5. })))
                    .size(px(22.))
                    .rounded_full()
                    .opacity(m_alpha)
                    .bg(card_bg)
                    .p(px(1.))
                    .when(is_pinned, |d| d.border_2().border_color(accent_col))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .active(|s| s.top(px(0.75)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_hover(cx.listener(move |v, inside: &bool, _, cx| {
                        if *inside {
                            v.hovered_member = Some(m_hover.clone());
                        } else if v.hovered_member.as_ref() == Some(&m_hover) {
                            v.hovered_member = None;
                        }
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                        if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                            w.prevent_default();
                            cx.stop_propagation();
                            v.pinned_open = true;
                            v.toggle_member_pin(m_key.clone(), cx);
                        }
                    }))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        cx.stop_propagation();
                        v.pinned_open = true;
                        v.toggle_member_pin(m_click.clone(), cx);
                    }))
                    .child(pro_person_avatar(m_name.as_ref(), 20., false, None, cx)),
            );
        }

        // Two-line header
        let header_aria: SharedString = format!(
            "{title_str}: {done_count} of {total_count} done, {}, {due_display}",
            self.status.label(cx)
        )
        .into();

        let head = div()
            .id(pro_eid(&self.id, "head-toggle"))
            .role(Role::Button)
            .aria_label(header_aria)
            .aria_expanded(open)
            .tab_index(0)
            .p(px(if narrow { 10. } else { 12. }))
            .flex()
            .items_center()
            .gap(px(if narrow { 8. } else { 10. }))
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                    w.prevent_default();
                    cx.stop_propagation();
                    v.toggle_open(cx);
                }
            }))
            .on_click(cx.listener(|v, _, _, cx| {
                v.toggle_open(cx);
            }))
            .child(ring_widget)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(title_str),
                    )
                    .child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(status_pill)
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(due_display),
                            ),
                    ),
            )
            .child(people_row);

        // Inline status picker fold with gliding pill indicator
        let cur_status = self.status;
        let st_sensor = pro_selection_sensor();
        let mut status_strip = div()
            .relative()
            .px(px(12.))
            .pb(px(10.))
            .pt(px(2.))
            .flex()
            .flex_wrap()
            .gap(px(4.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "st-pill-glide"),
                format!("{cur_status:?}"),
                st_sensor.clone(),
                6.,
                p.select,
                Some(p.line_strong),
                false,
            ));
        for opt in ProjectStatus::ALL {
            let (opt_fg, _) = opt.colors(&p);
            let selected = opt == cur_status;
            let opt_lbl: SharedString = opt.label(cx).into();
            status_strip = status_strip.child(
                div()
                    .id(pro_eid(&self.id, format!("st-opt-{opt:?}")))
                    .role(Role::RadioButton)
                    .aria_label(opt_lbl.clone())
                    .aria_toggled(selected.into())
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if selected {
                        transparent_black()
                    } else {
                        p.line
                    })
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.ink } else { p.text })
                    .cursor_pointer()
                    .when(!selected, move |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&st_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_key_down(cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                        if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                            w.prevent_default();
                            cx.stop_propagation();
                            v.set_status(opt, cx);
                        }
                    }))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        cx.stop_propagation();
                        v.set_status(opt, cx);
                    }))
                    .child(div().size(px(6.)).rounded(px(1.5)).bg(opt_fg))
                    .child(opt_lbl),
            );
        }

        let status_fold = pro_fold(
            pro_eid(&self.id, "status-picker-fold"),
            self.status_picker_open,
            38.,
            0.,
            move || status_strip.into_any_element(),
        );

        let bar_ratio = if total_count == 0 {
            0.0
        } else {
            done_count as f32 / total_count as f32
        };

        // Subtask rows (cross-linked with ring segments and teammate avatars)
        let mut tasks_col = div()
            .mx(px(12.))
            .py(px(6.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(div().pb(px(5.)).child(pro_spring_bar(
                pro_eid(&self.id, "sub-prog-bar"),
                bar_ratio,
                2.5,
                1.25,
                ring_track,
                accent_col,
            )));

        let tasks_snapshot = self.tasks.clone();
        for (t_idx, task) in tasks_snapshot.into_iter().enumerate() {
            let is_hot = self.hovered_task_idx == Some(t_idx);
            let is_dim = if let Some(ref flt) = active_filter {
                task.assignee.as_ref() != Some(flt)
            } else {
                false
            };

            let row_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("task-a-{t_idx}")),
                if is_dim { 0.34 } else { 1.0 },
                0.20,
                w,
                cx,
            )
            .clamp(0.2, 1.0);
            let avatar_reveal = pro_spring_f32(
                pro_eid(&self.id, format!("task-av-{t_idx}")),
                f32::from(is_hot || active_filter.is_some()),
                0.20,
                w,
                cx,
            )
            .clamp(0., 1.);

            let tick_el = pro_check_draw(
                pro_eid(&self.id, format!("sub-chk-{}", task.id)),
                task.done,
                15.5,
                false,
                accent_col,
            );

            let trailing_avatar = task.assignee.as_ref().map(|person| {
                div()
                    .opacity(avatar_reveal)
                    .relative()
                    .left(px(4. * (1. - avatar_reveal)))
                    .child(pro_person_avatar(person.as_ref(), 18., false, None, cx))
            });

            tasks_col = tasks_col.child(
                div()
                    .id(pro_eid(&self.id, format!("subtask-row-{}", task.id)))
                    .role(Role::CheckBox)
                    .aria_label(task.label.clone())
                    .aria_toggled(task.done.into())
                    .tab_index(0)
                    .mx(px(-6.))
                    .px(px(6.))
                    .h(px(32.))
                    .rounded(px(6.))
                    .opacity(row_alpha)
                    .bg(if is_hot { p.hover } else { transparent_black() })
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .active(|s| s.top(px(0.75)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_hover(cx.listener(move |v, inside: &bool, _, cx| {
                        if *inside {
                            v.hovered_task_idx = Some(t_idx);
                        } else if v.hovered_task_idx == Some(t_idx) {
                            v.hovered_task_idx = None;
                        }
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                        if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                            w.prevent_default();
                            cx.stop_propagation();
                            v.toggle_task(t_idx, cx);
                        }
                    }))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        cx.stop_propagation();
                        v.toggle_task(t_idx, cx);
                    }))
                    .child(tick_el)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(if task.done { p.muted } else { p.ink })
                            .truncate()
                            .child(task.label),
                    )
                    .when_some(trailing_avatar, |d, av| d.child(av)),
            );
        }

        // Dynamic footer caption linked to hover / filter state
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let footer_caption: SharedString = if let Some(idx) = self.hovered_task_idx
            && let Some(t) = self.tasks.get(idx)
        {
            if let Some(ref who) = t.assignee {
                if zh {
                    format!("负责人：{who}").into()
                } else {
                    format!("Assigned to {who}").into()
                }
            } else {
                t.label.clone()
            }
        } else if let Some(ref who) = active_filter {
            let m_total = self
                .tasks
                .iter()
                .filter(|t| t.assignee.as_ref() == Some(who))
                .count();
            let m_done = self
                .tasks
                .iter()
                .filter(|t| t.assignee.as_ref() == Some(who) && t.done)
                .count();
            if zh {
                format!("{who} · 已完成 {m_done} / {m_total}").into()
            } else {
                format!("{who} · {m_done} of {m_total} done").into()
            }
        } else {
            let remaining = total_count.saturating_sub(done_count);
            if remaining == 0 {
                tr(cx, "All done", "已全部完成").into()
            } else if zh {
                format!("剩余 {remaining} 项").into()
            } else {
                format!("{remaining} left").into()
            }
        };

        let open_cta: SharedString = self
            .action_label
            .clone()
            .unwrap_or_else(|| tr(cx, "Open", "打开").into());
        let hover_primary = pro_mix_color(p.primary, p.on_primary, 0.10);

        let footer_tray = div()
            .h(px(44.))
            .pl(px(12.))
            .pr(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(div().flex_1().min_w_0().truncate().child(pro_swap_label(
                pro_eid(&self.id, "tray-cap-vtx"),
                footer_caption,
                12.,
                FontWeight::NORMAL,
                p.muted,
            )))
            .child(
                div()
                    .flex_none()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "open-action-btn"),
                            open_cta.clone(),
                            6.,
                            p.primary,
                            hover_primary,
                            transparent_black(),
                            transparent_black(),
                            false,
                            div()
                                .h(px(28.))
                                .pl(px(10.))
                                .pr(px(7.))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.on_primary)
                                .whitespace_nowrap()
                                .child(open_cta)
                                .child(pro_svg(
                                    "0 0 12 12",
                                    ICON_ARROW_RIGHT_SM,
                                    12.,
                                    p.on_primary,
                                )),
                            cx,
                            |_, _, cx| {
                                cx.emit(ProjectCardEvent::ActionClicked);
                            },
                        )
                        .flex_none(),
                    ),
            );

        let est_body_h = (self.tasks.len() as f32) * 33.0 + 66.0;
        let unfold_body = pro_fold(
            pro_eid(&self.id, "card-body-fold"),
            open,
            est_body_h,
            0.,
            move || {
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(status_fold)
                    .child(tasks_col)
                    .child(footer_tray)
                    .into_any_element()
            },
        );

        let width_cell = self.measured_width.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(b) = bounds.first() {
                    let w_px = f32::from(b.size.width);
                    if w_px > 10. {
                        width_cell.set(w_px);
                    }
                }
            })
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .capture_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" {
                    if v.status_picker_open {
                        v.status_picker_open = false;
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                    } else if v.pinned_member.is_some() {
                        v.pinned_member = None;
                        cx.emit(ProjectCardEvent::AssigneeFilterChanged(None));
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                    } else if v.is_open() {
                        v.pinned_open = false;
                        v.hovered_card = false;
                        cx.emit(ProjectCardEvent::OpenChanged(false));
                        cx.notify();
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                }
            }))
            .child(
                div()
                    .id(pro_eid(&self.id, "card-shell"))
                    .w_full()
                    .max_w(px(332.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow_sm()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .on_hover(cx.listener(|v, inside: &bool, _, cx| {
                        let prev = v.is_open();
                        v.hovered_card = *inside;
                        if !*inside {
                            v.hovered_task_idx = None;
                            v.hovered_member = None;
                        }
                        let now_open = v.is_open();
                        if prev != now_open {
                            cx.emit(ProjectCardEvent::OpenChanged(now_open));
                        }
                        cx.notify();
                    }))
                    .child(head)
                    .child(unfold_body),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{ProjectCard, ProjectStatus, hit_test_ring_segment};
    use gpui::TestAppContext;

    #[test]
    fn polar_ring_hit_test_maps_all_five_segments_around_clock() {
        // Segment 0 starts at top (-90 deg -> slightly clockwise, e.g. (+4, -12))
        assert_eq!(hit_test_ring_segment(4.0, -12.0, 5), Some(0));
        // Right-lower quadrant -> segment 1
        assert_eq!(hit_test_ring_segment(12.0, 4.0, 5), Some(1));
        // Bottom -> segment 2
        assert_eq!(hit_test_ring_segment(0.0, 12.5, 5), Some(2));
        // Center deadzone returns None
        assert_eq!(hit_test_ring_segment(1.0, 1.0, 5), None);
    }

    #[gpui::test]
    fn subtask_ticks_status_picker_and_member_filter_sync(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let card = cx.new(|cx| ProjectCard::new("pc-test", cx));
        card.update(cx, |pc, cx| {
            assert_eq!(pc.progress_counts(), (3, 5));
            assert_eq!(pc.status(), ProjectStatus::OnTrack);
            pc.toggle_task(3, cx);
            assert_eq!(pc.progress_counts(), (4, 5));

            pc.set_status(ProjectStatus::AtRisk, cx);
            assert_eq!(pc.status(), ProjectStatus::AtRisk);

            pc.toggle_member_pin("Mara".into(), cx);
            assert_eq!(pc.active_member_filter().map(|s| s.as_ref()), Some("Mara"));
            pc.toggle_member_pin("Mara".into(), cx);
            assert!(pc.active_member_filter().is_none());
        });
    }
}
