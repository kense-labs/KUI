//! Collapsible onboarding setup guide with animated check drawing and strikethrough groups.
//!
//! Renders an onboarding checklist card with a spring-driven progress bar,
//! `"3 of 7 completed"` step counter, collapsible task groups (`pro_fold`),
//! progressive `PathBuilder` check-mark drawing, smooth left-to-right title
//! strikethroughs, and per-step action buttons.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_icon_btn, pro_progress_ring, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
        pro_swap_label, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const ICON_MINIMIZE: &str = r#"<path d="M9.5 2.5v4h4M9.5 6.5l4.5-4.5M6.5 13.5v-4h-4M6.5 9.5 2 14" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#;
const ICON_EXPAND: &str = r#"<path d="M10 2h4v4M14 2 9.2 6.8M6 14H2v-4M2 14l4.8-4.8" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#;

/// Single onboarding checklist task inside a [`SetupGroup`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupTask {
    pub id: SharedString,
    pub label: SharedString,
    pub action_label: Option<SharedString>,
    pub done: bool,
}

impl SetupTask {
    /// Construct an incomplete [`SetupTask`] with `id` and `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            action_label: None,
            done: false,
        }
    }

    /// Attach a per-step call-to-action button label (e.g. `"Connect"`, `"Verify"`).
    pub fn with_action(mut self, action: impl Into<SharedString>) -> Self {
        self.action_label = Some(action.into());
        self
    }

    /// Mark whether this task starts completed.
    pub fn with_done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }
}

/// Collapsible section of [`SetupTask`]s inside [`SetupGuide`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupGroup {
    pub id: SharedString,
    pub title: SharedString,
    pub tasks: Vec<SetupTask>,
    pub open: bool,
}

impl SetupGroup {
    /// Construct a [`SetupGroup`] with `id`, `title`, and `tasks`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        tasks: Vec<SetupTask>,
    ) -> Self {
        let all_done = !tasks.is_empty() && tasks.iter().all(|t| t.done);
        Self {
            id: id.into(),
            title: title.into(),
            tasks,
            open: !all_done,
        }
    }

    /// Explicitly set whether this group starts expanded.
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Return `true` when every task in this group is completed.
    pub fn is_complete(&self) -> bool {
        !self.tasks.is_empty() && self.tasks.iter().all(|t| t.done)
    }

    /// Return `(completed_count, total_count)` for this group.
    pub fn counts(&self) -> (usize, usize) {
        let done = self.tasks.iter().filter(|t| t.done).count();
        (done, self.tasks.len())
    }
}

/// Typed events emitted by [`SetupGuide`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupGuideEvent {
    /// Task completion toggled; carries `(task_id, done, all_completed_ids)`.
    TaskToggled {
        task_id: SharedString,
        done: bool,
        completed_ids: Vec<SharedString>,
    },
    /// User clicked the per-step action button on a task.
    ActionTriggered {
        group_id: SharedString,
        task_id: SharedString,
        action: SharedString,
    },
    /// Group disclosure header opened or closed.
    GroupToggled { group_id: SharedString, open: bool },
    /// Whole panel minimized or expanded.
    Minimized(bool),
    /// Header `"Edit"` button clicked.
    EditRequested,
    /// Header close button clicked.
    Dismissed,
}

/// Default 3-group, 7-step onboarding guide (`"3 of 7 completed"` initially).
pub fn default_setup_groups(zh: bool) -> Vec<SetupGroup> {
    if zh {
        vec![
            SetupGroup::new(
                "workspace",
                "配置团队工作区",
                vec![
                    SetupTask::new("ws-name", "设置工作区名称与标识")
                        .with_action("修改")
                        .with_done(true),
                    SetupTask::new("ws-invite", "邀请核心协作成员")
                        .with_action("邀请")
                        .with_done(true),
                ],
            )
            .with_open(false),
            SetupGroup::new(
                "integrations",
                "接入研发与沟通工具",
                vec![
                    SetupTask::new("link-github", "关联 GitHub 代码仓库")
                        .with_action("管理")
                        .with_done(true),
                    SetupTask::new("link-slack", "连接 Slack 消息通知频道").with_action("连接"),
                    SetupTask::new("webhook-key", "生成 Webhook 签名密钥").with_action("生成"),
                ],
            )
            .with_open(true),
            SetupGroup::new(
                "verify",
                "验证并上线生产环境",
                vec![
                    SetupTask::new("custom-domain", "验证自定义访问域名").with_action("验证"),
                    SetupTask::new("first-check", "运行首次健康发布检查").with_action("运行"),
                ],
            )
            .with_open(false),
        ]
    } else {
        vec![
            SetupGroup::new(
                "workspace",
                "Set up your workspace",
                vec![
                    SetupTask::new("ws-name", "Choose a workspace name")
                        .with_action("Rename")
                        .with_done(true),
                    SetupTask::new("ws-invite", "Invite your teammates")
                        .with_action("Invite")
                        .with_done(true),
                ],
            )
            .with_open(false),
            SetupGroup::new(
                "integrations",
                "Connect your tools",
                vec![
                    SetupTask::new("link-github", "Link GitHub repository")
                        .with_action("Manage")
                        .with_done(true),
                    SetupTask::new("link-slack", "Connect Slack notifications")
                        .with_action("Connect"),
                    SetupTask::new("webhook-key", "Generate webhook signing key")
                        .with_action("Generate"),
                ],
            )
            .with_open(true),
            SetupGroup::new(
                "verify",
                "Verify & go live",
                vec![
                    SetupTask::new("custom-domain", "Verify custom domain").with_action("Verify"),
                    SetupTask::new("first-check", "Run first deployment check")
                        .with_action("Run check"),
                ],
            )
            .with_open(false),
        ]
    }
}

/// Render single-line text with an animated left-to-right strikethrough line (`0.0..=1.0`).
#[allow(clippy::too_many_arguments)]
fn render_strikethrough_label(
    id: ElementId,
    label: SharedString,
    struck: bool,
    font_size: f32,
    weight: FontWeight,
    active_fg: Hsla,
    done_fg: Hsla,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let strike_t =
        pro_spring_f32(pro_eid(&id, "strike"), f32::from(struck), 0.30, w, cx).clamp(0., 1.);
    let fg = if struck { done_fg } else { active_fg };

    div()
        .relative()
        .flex()
        .items_center()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .relative()
                .min_w_0()
                .text_size(px(font_size))
                .line_height(px(20.))
                .font_weight(weight)
                .text_color(fg)
                .truncate()
                .child(label)
                .when(strike_t > 0.005, |d| {
                    d.child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(10.))
                            .h(px(1.0))
                            .w(relative(strike_t))
                            .bg(done_fg),
                    )
                }),
        )
        .into_any_element()
}

/// Collapsible onboarding panel with spring progress bar, check drawing, and task groups.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::setup_guide::{SetupGuide, SetupGuideEvent};
///
/// # fn doc(cx: &mut gpui::App) {
/// let guide = cx.new(|cx| SetupGuide::new("workspace-onboarding", cx));
/// cx.subscribe(&guide, |_, ev: &SetupGuideEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct SetupGuide {
    id: ElementId,
    title: Option<SharedString>,
    groups: Vec<SetupGroup>,
    custom_groups: bool,
    groups_lang_zh: bool,
    minimized: bool,
    dismissed: bool,
    show_edit: bool,
    show_close: bool,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl EventEmitter<SetupGuideEvent> for SetupGuide {}

impl SetupGuide {
    /// Construct a [`SetupGuide`] with 3 groups and 7 onboarding tasks (`3 of 7 completed`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        Self {
            id: id.into(),
            title: None,
            groups: default_setup_groups(zh),
            custom_groups: false,
            groups_lang_zh: zh,
            minimized: false,
            dismissed: false,
            show_edit: true,
            show_close: true,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the panel heading above the progress bar.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the onboarding task groups.
    pub fn with_groups(mut self, groups: Vec<SetupGroup>) -> Self {
        self.groups = groups;
        self.custom_groups = true;
        self
    }

    /// Set whether the task list starts minimized into a compact progress header.
    pub fn with_minimized(mut self, minimized: bool) -> Self {
        self.minimized = minimized;
        self
    }

    /// Show or hide the header `"Edit"` button.
    pub fn with_edit_button(mut self, show: bool) -> Self {
        self.show_edit = show;
        self
    }

    /// Show or hide the header close (`×`) button.
    pub fn with_close_button(mut self, show: bool) -> Self {
        self.show_close = show;
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

    /// Return `(completed_tasks, total_tasks)` across all groups.
    pub fn progress_counts(&self) -> (usize, usize) {
        let mut done = 0usize;
        let mut total = 0usize;
        for g in &self.groups {
            for t in &g.tasks {
                total += 1;
                if t.done {
                    done += 1;
                }
            }
        }
        (done, total)
    }

    /// Return the ids of all completed tasks in catalog order.
    pub fn completed_ids(&self) -> Vec<SharedString> {
        self.groups
            .iter()
            .flat_map(|g| g.tasks.iter())
            .filter(|t| t.done)
            .map(|t| t.id.clone())
            .collect()
    }

    /// Return the current onboarding groups.
    pub fn groups(&self) -> &[SetupGroup] {
        &self.groups
    }

    /// Return whether the body is minimized.
    pub fn is_minimized(&self) -> bool {
        self.minimized
    }

    fn sync_language_defaults(&mut self, cx: &App) {
        if self.custom_groups {
            return;
        }
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        if self.groups_lang_zh != zh {
            let done_map: Vec<(SharedString, bool)> = self
                .groups
                .iter()
                .flat_map(|g| g.tasks.iter().map(|t| (t.id.clone(), t.done)))
                .collect();
            let open_map: Vec<(SharedString, bool)> =
                self.groups.iter().map(|g| (g.id.clone(), g.open)).collect();
            let mut next = default_setup_groups(zh);
            for g in &mut next {
                if let Some((_, open)) = open_map.iter().find(|(id, _)| id == &g.id) {
                    g.open = *open;
                }
                for t in &mut g.tasks {
                    if let Some((_, d)) = done_map.iter().find(|(id, _)| id == &t.id) {
                        t.done = *d;
                    }
                }
            }
            self.groups = next;
            self.groups_lang_zh = zh;
        }
    }

    /// Toggle whether `group_id` is expanded.
    pub fn toggle_group(&mut self, group_id: &SharedString, cx: &mut Context<Self>) {
        if let Some(g) = self.groups.iter_mut().find(|g| &g.id == group_id) {
            g.open = !g.open;
            let open = g.open;
            let id = g.id.clone();
            cx.emit(SetupGuideEvent::GroupToggled { group_id: id, open });
            cx.notify();
        }
    }

    /// Toggle completion of `task_id`; auto-advances open group when a group finishes.
    pub fn toggle_task(&mut self, group_idx: usize, task_idx: usize, cx: &mut Context<Self>) {
        let Some(group) = self.groups.get_mut(group_idx) else {
            return;
        };
        let was_complete = group.is_complete();
        let Some(task) = group.tasks.get_mut(task_idx) else {
            return;
        };
        task.done = !task.done;
        let new_done = task.done;
        let task_id = task.id.clone();
        let now_complete = group.is_complete();

        if !was_complete && now_complete {
            group.open = false;
            // Open the next incomplete group automatically
            let total_groups = self.groups.len();
            for offset in 1..total_groups {
                let candidate = (group_idx + offset) % total_groups;
                if !self.groups[candidate].is_complete() {
                    self.groups[candidate].open = true;
                    break;
                }
            }
        }

        let completed_ids = self.completed_ids();
        cx.emit(SetupGuideEvent::TaskToggled {
            task_id,
            done: new_done,
            completed_ids,
        });
        cx.notify();
    }

    fn trigger_task_action(&mut self, group_idx: usize, task_idx: usize, cx: &mut Context<Self>) {
        let Some(group) = self.groups.get(group_idx) else {
            return;
        };
        let Some(task) = group.tasks.get(task_idx) else {
            return;
        };
        let Some(action) = task.action_label.clone() else {
            return;
        };
        let group_id = group.id.clone();
        let task_id = task.id.clone();
        let already_done = task.done;

        cx.emit(SetupGuideEvent::ActionTriggered {
            group_id,
            task_id,
            action,
        });
        if !already_done {
            self.toggle_task(group_idx, task_idx, cx);
        } else {
            cx.notify();
        }
    }
}

impl Render for SetupGuide {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_language_defaults(cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            let phase = (self.started_at.elapsed().as_secs_f32() / 1.4).fract();
            let card_bg: Hsla = if p.dark {
                rgb(0x1c1c1e).into()
            } else {
                rgb(0xffffff).into()
            };
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(368.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(124., 16., 5., phase, cx))
                        .child(pro_skeleton_bar(320., 5., 3., phase + 0.1, cx))
                        .child(pro_skeleton_bar(336., 34., 7., phase + 0.2, cx))
                        .child(pro_skeleton_bar(336., 96., 7., phase + 0.3, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(368.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x1c1c1e) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line)
                .shadow(pro_card_shadow(p.dark))
                .child(pro_empty_state(
                    ProIcon::Tasks,
                    tr(cx, "No setup steps", "暂无初始化任务"),
                    tr(
                        cx,
                        "All workspace onboarding milestones are configured.",
                        "当前工作区已完成全部初始化配置。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        if self.dismissed {
            let reopen_lbl: SharedString = tr(cx, "Reopen setup guide", "重新打开配置向导").into();
            let (done, total) = self.progress_counts();
            let reopen_bg: Hsla = if p.dark {
                rgb(0x1c1c1e).into()
            } else {
                rgb(0xffffff).into()
            };
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    pro_tactile(
                        pro_eid(&self.id, "reopen-pill"),
                        reopen_lbl.clone(),
                        8.,
                        reopen_bg,
                        p.hover,
                        p.line_strong,
                        p.line_strong,
                        false,
                        div()
                            .h(px(34.))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .whitespace_nowrap()
                            .child(pro_icon(ProIcon::Tasks, 14., p.blue))
                            .child(reopen_lbl)
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(format!("{done}/{total}")),
                            ),
                        cx,
                        |v, _, cx| {
                            v.dismissed = false;
                            cx.notify();
                        },
                    )
                    .flex_none()
                    .shadow(pro_card_shadow(p.dark)),
                )
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
        let open_group_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.025)
        };
        let track_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let accent_col: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgba(0x0060e2c4).into()
        };

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Setup guide", "配置向导").into());
        let (done_count, total_count) = self.progress_counts();
        let ratio = if total_count == 0 {
            0.0
        } else {
            done_count as f32 / total_count as f32
        };
        let all_finished = total_count > 0 && done_count == total_count;
        let bar_fill = if all_finished { p.green } else { accent_col };

        // Header row
        let min_aria: SharedString = if self.minimized {
            tr(cx, "Expand setup guide", "展开配置向导").into()
        } else {
            tr(cx, "Minimize setup guide", "收起配置向导").into()
        };
        let edit_lbl: SharedString = tr(cx, "Edit", "编辑").into();
        let close_aria: SharedString = tr(cx, "Close setup guide", "关闭配置向导").into();
        let is_min = self.minimized;

        let header = div()
            .pl(px(16.))
            .pr(px(10.))
            .pt(px(12.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .min_w_0()
                    .text_size(px(15.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .truncate()
                    .child(title_str),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .when(self.show_edit, |d| {
                        d.child(
                            pro_tactile(
                                pro_eid(&self.id, "edit-btn"),
                                edit_lbl.clone(),
                                6.,
                                transparent_black(),
                                p.hover,
                                transparent_black(),
                                transparent_black(),
                                false,
                                div()
                                    .h(px(28.))
                                    .px(px(8.))
                                    .flex()
                                    .items_center()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .whitespace_nowrap()
                                    .child(edit_lbl),
                                cx,
                                |_, _, cx| {
                                    cx.emit(SetupGuideEvent::EditRequested);
                                },
                            )
                            .flex_none(),
                        )
                    })
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "min-btn"),
                            min_aria,
                            6.,
                            transparent_black(),
                            p.hover,
                            transparent_black(),
                            transparent_black(),
                            is_min,
                            div()
                                .size(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_svg(
                                    "0 0 16 16",
                                    if is_min { ICON_EXPAND } else { ICON_MINIMIZE },
                                    15.,
                                    p.muted,
                                )),
                            cx,
                            |v, _, cx| {
                                v.minimized = !v.minimized;
                                cx.emit(SetupGuideEvent::Minimized(v.minimized));
                                cx.notify();
                            },
                        )
                        .flex_none(),
                    )
                    .when(self.show_close, |d| {
                        d.child(
                            pro_icon_btn(
                                pro_eid(&self.id, "close-btn"),
                                close_aria,
                                ProIcon::Close,
                                28.,
                                false,
                                cx,
                                |v, _, cx| {
                                    v.dismissed = true;
                                    cx.emit(SetupGuideEvent::Dismissed);
                                    cx.notify();
                                },
                            )
                            .flex_none(),
                        )
                    }),
            );

        let count_caption = if matches!(UiLanguage::current(cx), UiLanguage::Chinese) {
            format!("已完成 {done_count} / {total_count} 项")
        } else {
            format!("{done_count} of {total_count} completed")
        };

        let progress_row = div()
            .px(px(16.))
            .pt(px(8.))
            .pb(px(12.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(div().flex_1().min_w_0().child(pro_spring_bar(
                pro_eid(&self.id, "overall-bar"),
                ratio,
                4.,
                2.,
                track_bg,
                bar_fill,
            )))
            .child(div().flex_none().child(pro_swap_label(
                pro_eid(&self.id, "overall-cnt-lbl"),
                count_caption,
                12.,
                FontWeight::NORMAL,
                if all_finished { p.green_ink } else { p.muted },
            )));

        // Identify the very first incomplete task globally so its circle gets a solid current ring
        let mut first_incomplete: Option<(usize, usize)> = None;
        for (gi, g) in self.groups.iter().enumerate() {
            for (ti, t) in g.tasks.iter().enumerate() {
                if !t.done && first_incomplete.is_none() {
                    first_incomplete = Some((gi, ti));
                }
            }
        }

        // Pre-build group elements while `w` and `cx` are available
        let mut groups_col = div()
            .px(px(4.))
            .py(px(4.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(2.));

        let group_snapshots = self.groups.clone();
        for (g_idx, group) in group_snapshots.into_iter().enumerate() {
            let g_complete = group.is_complete();
            let (g_done, g_total) = group.counts();
            let g_open = group.open;
            let g_id = group.id.clone();

            let head_title = render_strikethrough_label(
                pro_eid(&self.id, format!("grp-lbl-{g_id}")),
                group.title.clone(),
                g_complete,
                13.,
                FontWeight::MEDIUM,
                p.strong,
                p.muted,
                w,
                cx,
            );

            let g_ratio_target = if g_total == 0 {
                0.0
            } else {
                g_done as f32 / g_total as f32
            };
            let g_ratio = pro_spring_f32(
                pro_eid(&self.id, format!("grp-ring-sp-{g_id}")),
                g_ratio_target,
                0.26,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let g_ring = pro_progress_ring(
                g_ratio,
                14.,
                1.8,
                if g_complete { p.green } else { accent_col },
                track_bg,
            );

            let g_id_click = g_id.clone();
            let g_id_key = g_id.clone();
            let group_head = div()
                .id(pro_eid(&self.id, format!("grp-head-{g_id}")))
                .role(Role::Button)
                .aria_label(group.title.clone())
                .aria_expanded(g_open)
                .tab_index(0)
                .w_full()
                .pl(px(12.))
                .pr(px(8.))
                .py(px(8.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .cursor_pointer()
                .active(|s| s.top(px(0.75)))
                .when(!g_open, move |d| d.hover(move |s| s.bg(p.hover)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_key_down(cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        w.prevent_default();
                        cx.stop_propagation();
                        v.toggle_group(&g_id_key, cx);
                    }
                }))
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.toggle_group(&g_id_click, cx);
                }))
                .child(div().flex_1().min_w_0().child(head_title))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(g_ring)
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("grp-cnt-{g_id}")),
                            format!("{g_done}/{g_total}"),
                            11.5,
                            FontWeight::NORMAL,
                            p.muted,
                        ))
                        .child(pro_icon(
                            if g_open {
                                ProIcon::ChevronUp
                            } else {
                                ProIcon::ChevronDown
                            },
                            15.,
                            p.muted,
                        )),
                );

            let mut tasks_list = div().px(px(4.)).pb(px(6.)).flex().flex_col().gap(px(2.));

            let est_h = (group.tasks.len() as f32) * 34.0 + 8.0;
            for (t_idx, task) in group.tasks.into_iter().enumerate() {
                let t_id = task.id.clone();
                let is_done = task.done;
                let is_next = first_incomplete == Some((g_idx, t_idx));

                let chk_el = div()
                    .relative()
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(is_next && !is_done, |d| {
                        d.rounded_full()
                            .border_1()
                            .border_color(accent_col.opacity(0.45))
                    })
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("task-chk-{t_id}")),
                        is_done,
                        16.,
                        false,
                        accent_col,
                    ))
                    .into_any_element();
                let title_el = render_strikethrough_label(
                    pro_eid(&self.id, format!("task-lbl-{t_id}")),
                    task.label.clone(),
                    is_done,
                    13.,
                    FontWeight::NORMAL,
                    p.ink,
                    p.muted,
                    w,
                    cx,
                );

                let action_btn = task.action_label.clone().map(|act_lbl| {
                    let act_rest_bg: Hsla = if is_done {
                        transparent_black()
                    } else if is_next {
                        p.card
                    } else {
                        p.chip
                    };
                    let act_rest_bdr: Hsla = if is_next && !is_done {
                        p.line_strong
                    } else {
                        p.line
                    };
                    div()
                        .flex_none()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            pro_tactile(
                                pro_eid(&self.id, format!("task-act-{t_id}")),
                                act_lbl.clone(),
                                5.,
                                act_rest_bg,
                                p.chip_hover,
                                act_rest_bdr,
                                p.line_strong,
                                false,
                                div()
                                    .h(px(23.))
                                    .px(px(7.5))
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if is_done { p.muted } else { p.ink })
                                    .whitespace_nowrap()
                                    .child(act_lbl),
                                cx,
                                move |v, _, cx| {
                                    v.trigger_task_action(g_idx, t_idx, cx);
                                },
                            )
                            .flex_none(),
                        )
                });

                tasks_list = tasks_list.child(
                    div()
                        .id(pro_eid(&self.id, format!("task-row-{t_id}")))
                        .role(Role::CheckBox)
                        .aria_label(task.label)
                        .aria_toggled(is_done.into())
                        .tab_index(0)
                        .w_full()
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .cursor_pointer()
                        .active(|s| s.top(px(0.75)))
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .on_key_down(cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                            if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                                w.prevent_default();
                                cx.stop_propagation();
                                v.toggle_task(g_idx, t_idx, cx);
                            }
                        }))
                        .on_click(cx.listener(move |v, _, _, cx| {
                            v.toggle_task(g_idx, t_idx, cx);
                        }))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(chk_el)
                                .child(div().flex_1().min_w_0().child(title_el)),
                        )
                        .when_some(action_btn, |d, btn| d.child(btn)),
                );
            }

            let fold_panel = pro_fold(
                pro_eid(&self.id, format!("grp-fold-{g_id}")),
                g_open,
                est_h,
                0.,
                move || tasks_list.into_any_element(),
            );

            groups_col = groups_col.child(
                div()
                    .w_full()
                    .rounded(px(8.))
                    .when(g_open, |d| d.bg(open_group_bg))
                    .flex()
                    .flex_col()
                    .child(group_head)
                    .child(fold_panel),
            );
        }

        let body_fold = pro_fold(
            pro_eid(&self.id, "guide-body-fold"),
            !self.minimized,
            240.,
            0.,
            move || groups_col.into_any_element(),
        );

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(368.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(pro_card_shadow(p.dark))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(progress_row)
                    .child(body_fold),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{SetupGuide, default_setup_groups};
    use gpui::TestAppContext;

    #[test]
    fn default_groups_start_at_three_of_seven_completed() {
        let groups = default_setup_groups(false);
        assert_eq!(groups.len(), 3);
        let done: usize = groups.iter().map(|g| g.counts().0).sum();
        let total: usize = groups.iter().map(|g| g.counts().1).sum();
        assert_eq!((done, total), (3, 7));
        assert!(groups[0].is_complete());
        assert!(!groups[1].is_complete());
        assert!(groups[1].open);
    }

    #[gpui::test]
    fn completing_group_auto_folds_and_opens_next_incomplete_group(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let guide = cx.new(|cx| SetupGuide::new("sg-test", cx));
        guide.update(cx, |sg, cx| {
            assert_eq!(sg.progress_counts(), (3, 7));
            assert!(sg.groups()[1].open);
            assert!(!sg.groups()[2].open);

            // Complete remaining two tasks in group 1 ("Connect your tools")
            sg.toggle_task(1, 1, cx);
            sg.toggle_task(1, 2, cx);
            assert_eq!(sg.progress_counts(), (5, 7));
            assert!(sg.groups()[1].is_complete());
            assert!(!sg.groups()[1].open);
            assert!(sg.groups()[2].open);
        });
    }
}
