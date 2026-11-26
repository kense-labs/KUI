//! Engineering Kanban task board block with search, label chips, compact switch, drag-and-drop, quick add, and detail drawer.
//!
//! Renders [`TaskBoardBlock`] with a breadcrumb header, [`Editor`] search box, toggleable label
//! filter chips, a capsule `Compact` density switch ([`pro_switch`]), four workflow columns
//! (`Backlog`, `Todo`, `In Progress`, `Done`) supporting pointer drag-and-drop and column move
//! controls, per-column inline quick-add forms, and a [`pro_presence`] slide-over detail drawer.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_card_shadow, pro_check_draw,
        pro_ease, pro_eid, pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected,
        pro_mix_color, pro_person_avatar, pro_pill_indicator, pro_popover_presence, pro_presence,
        pro_progress_ring, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
        pro_swap_label, pro_switch, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::Slide,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

type TaskClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type TaskMouseDownHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;
type TaskKeyDownHandler = Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct ColShiftMiniBtn {
    id: ElementId,
    aria: SharedString,
    icon: ProIcon,
    on_shift: TaskClickHandler,
}

impl RenderOnce for ColShiftMiniBtn {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let st = w.use_keyed_state(pro_eid(&self.id, "mv-st"), cx, |_, _| {
            (Slide::snappy(0.), Slide::snappy(0.))
        });
        let (hov, down) = st.update(cx, |s, cx| {
            (s.0.get(w, cx).clamp(0., 1.), s.1.get(w, cx).clamp(0., 1.))
        });
        let face_bg = pro_mix_color(transparent_black(), p.chip_hover, hov);
        let fg = pro_mix_color(p.faint, p.strong, hov);
        let icon_sz = 10.5 * (1. - 0.12 * down);
        let on_shift = self.on_shift;

        div()
            .id(self.id)
            .role(Role::Button)
            .aria_label(self.aria)
            .size(px(18.))
            .relative()
            .rounded(px(5.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.0.set(if *inside { 1. } else { 0. }, cx);
                        if !*inside {
                            s.1.set(0., cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    cx.stop_propagation();
                    st.update(cx, |s, cx| {
                        s.1.set(1., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    cx.stop_propagation();
                    st.update(cx, |s, cx| {
                        s.1.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.1.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_click(move |ev, w, cx| {
                cx.stop_propagation();
                (on_shift)(ev, w, cx);
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        if face_bg.a <= 0.001 {
                            return;
                        }
                        let inset = 1.2 * down;
                        let bw = f32::from(bounds.size.width);
                        let bh = f32::from(bounds.size.height);
                        let rect = Bounds::new(
                            point(bounds.origin.x + px(inset), bounds.origin.y + px(inset)),
                            size(px((bw - 2. * inset).max(1.)), px((bh - 2. * inset).max(1.))),
                        );
                        window.paint_quad(fill(rect, face_bg).corner_radii(px(4.5)));
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(pro_icon(self.icon, icon_sz, fg))
    }
}

struct TaskCardAnim {
    hover: Slide,
    drag: Slide,
    was_dragging: bool,
}

#[derive(IntoElement)]
struct TaskCardSurface {
    id: ElementId,
    aria: SharedString,
    radius: f32,
    is_open: bool,
    is_dragging: bool,
    bounds_cell: Rc<Cell<Bounds<Pixels>>>,
    on_mouse_down: TaskMouseDownHandler,
    on_key_down: TaskKeyDownHandler,
    content: AnyElement,
}

impl RenderOnce for TaskCardSurface {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_drag = self.is_dragging;
        let st = w.use_keyed_state(pro_eid(&self.id, "tcard-st"), cx, |_, _| TaskCardAnim {
            hover: Slide::snappy(0.),
            drag: Slide::snappy(f32::from(init_drag)),
            was_dragging: init_drag,
        });
        let (hov, drag_t) = st.update(cx, |s, cx| {
            if s.was_dragging != self.is_dragging {
                s.was_dragging = self.is_dragging;
                s.drag.set(f32::from(self.is_dragging), cx);
            }
            (
                s.hover.get(w, cx).clamp(0., 1.),
                s.drag.get(w, cx).clamp(0., 1.),
            )
        });

        let base_border = if self.is_open { p.blue } else { p.line };
        let hov_border = if self.is_open { p.blue } else { p.line_strong };
        let border_col = pro_mix_color(
            pro_mix_color(base_border, hov_border, hov),
            p.blue.opacity(0.65),
            drag_t,
        );
        let card_bg = pro_mix_color(p.card, p.subtle, drag_t * 0.35);
        let opacity = 1.0 - 0.56 * drag_t;
        let lift_y = -2.2 * drag_t - 0.6 * hov * (1.0 - drag_t);
        let bounds_cell = self.bounds_cell;
        let on_md = self.on_mouse_down;
        let on_kd = self.on_key_down;

        div()
            .id(self.id)
            .role(Role::Button)
            .aria_label(self.aria)
            .tab_index(0)
            .group("tb-card-grp")
            .relative()
            .top(px(lift_y))
            .w_full()
            .rounded(px(self.radius))
            .bg(card_bg)
            .border_1()
            .border_color(border_col)
            .shadow(pro_card_shadow(p.dark))
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .opacity(opacity)
            .cursor_pointer()
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, move |ev, w, cx| {
                (on_md)(ev, w, cx);
            })
            .on_key_down(move |ev, w, cx| {
                (on_kd)(ev, w, cx);
            })
            .child(
                canvas(move |b, _, _| bounds_cell.set(b), |_, _, _, _| ())
                    .absolute()
                    .inset_0(),
            )
            .child(self.content)
    }
}

#[derive(IntoElement)]
struct DropSlotBar {
    id: ElementId,
    active: bool,
    bottom_gap: f32,
    top_gap: f32,
}

impl RenderOnce for DropSlotBar {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_act = self.active;
        let st = w.use_keyed_state(pro_eid(&self.id, "dslot"), cx, |_, _| {
            (init_act, Slide::snappy(f32::from(init_act)))
        });
        let amt = st.update(cx, |s, cx| {
            if s.0 != self.active {
                s.0 = self.active;
                s.1.set(f32::from(self.active), cx);
            }
            s.1.get(w, cx).clamp(0., 1.)
        });
        if !self.active && amt <= 0.01 {
            return div().into_any_element();
        }
        let width_frac = (0.72 + 0.28 * amt).clamp(0.1, 1.0);
        div()
            .w_full()
            .mt(px(self.top_gap * amt))
            .mb(px(self.bottom_gap * amt))
            .h(px(3.5 * amt))
            .flex()
            .items_center()
            .justify_center()
            .opacity(amt)
            .child(
                div()
                    .w(relative(width_frac))
                    .h_full()
                    .rounded_full()
                    .bg(p.blue),
            )
            .into_any_element()
    }
}

const SVG_STAR_FILLED: &str = r#"<path d="M13.7276 3.44418L15.4874 6.99288C15.7274 7.48687 16.3673 7.9607 16.9073 8.05143L20.0969 8.58575C22.1367 8.92853 22.6167 10.4206 21.1468 11.8925L18.6671 14.3927C18.2471 14.8161 18.0172 15.6327 18.1471 16.2175L18.8571 19.3125C19.417 21.7623 18.1271 22.71 15.9774 21.4296L12.9877 19.6452C12.4478 19.3226 11.5579 19.3226 11.0079 19.6452L8.01827 21.4296C5.8785 22.71 4.57865 21.7522 5.13859 19.3125L5.84851 16.2175C5.97849 15.6327 5.74852 14.8161 5.32856 14.3927L2.84884 11.8925C1.389 10.4206 1.85895 8.92853 3.89872 8.58575L7.08837 8.05143C7.61831 7.9607 8.25824 7.48687 8.49821 6.99288L10.258 3.44418C11.2179 1.51861 12.7777 1.51861 13.7276 3.44418Z" fill="currentColor" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"></path>"#;
const SVG_BACKLOG_DASH: &str = r#"<circle cx="8" cy="8" r="5.3" stroke="currentColor" stroke-width="1.4" stroke-dasharray="2.4 2.4"></circle>"#;
const SVG_TODO_SQ: &str = r#"<rect x="2.8" y="2.8" width="10.4" height="10.4" rx="2.6" stroke="currentColor" stroke-width="1.4"></rect>"#;
const SVG_REVIEW_EYE: &str = r#"<path d="M1.8 8S4 3.8 8 3.8 14.2 8 14.2 8 12 12.2 8 12.2 1.8 8 1.8 8Z" stroke="currentColor" stroke-width="1.4"></path><circle cx="8" cy="8" r="1.8" stroke="currentColor" stroke-width="1.4"></circle>"#;
const SVG_PROGRESS_CLOCK: &str = r#"<circle cx="8" cy="8" r="5.4" stroke="currentColor" stroke-width="1.4"></circle><path d="M8 5v3l2 1.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SVG_DONE_BOX: &str = r##"<rect x="2.4" y="2.4" width="11.2" height="11.2" rx="3" fill="currentColor"></rect><path d="m5.3 8.2 1.8 1.8 3.5-3.7" stroke="#ffffff" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"></path>"##;

/// Task urgency level (`0`–`4`) rendered as three ascending signal bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskPriority {
    /// Unprioritized (`0`).
    None = 0,
    /// Low priority (`1` blue bar).
    Low = 1,
    /// Medium priority (`2` amber bars).
    #[default]
    Medium = 2,
    /// High priority (`3` red bars).
    High = 3,
    /// Urgent blocker (`3` red bars).
    Urgent = 4,
}

impl TaskPriority {
    const PICKABLE: [Self; 4] = [Self::None, Self::Low, Self::Medium, Self::High];

    fn active_bars(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High | Self::Urgent => 3,
        }
    }

    fn next_quick(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High | Self::Urgent => Self::Low,
            Self::None => Self::Medium,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::None => tr(cx, "No priority", "无优先级"),
            Self::Low => tr(cx, "Low", "低"),
            Self::Medium => tr(cx, "Medium", "中"),
            Self::High => tr(cx, "High", "高"),
            Self::Urgent => tr(cx, "Urgent", "紧急"),
        }
    }

    fn short_label(self, cx: &App) -> &'static str {
        match self {
            Self::None => tr(cx, "None", "无"),
            Self::Low => tr(cx, "Low", "低"),
            Self::Medium => tr(cx, "Med", "中"),
            Self::High | Self::Urgent => tr(cx, "High", "高"),
        }
    }
}

/// Semantic icon style for a [`TaskColumnStatus`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskStatusIcon {
    /// Dashed circle (`Backlog`).
    #[default]
    Backlog,
    /// Rounded square (`Todo`).
    Todo,
    /// Eye glyph (`In review`).
    InReview,
    /// Clock glyph (`In progress`).
    InProgress,
    /// Filled checkmark square (`Done`).
    Done,
}

/// One workflow column definition on [`TaskBoardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskColumnStatus {
    /// Stable status key (`"backlog"`, `"todo"`, `"in_progress"`, `"done"`).
    pub id: SharedString,
    /// Column display title.
    pub label: SharedString,
    /// Leading status icon.
    pub icon: TaskStatusIcon,
}

impl TaskColumnStatus {
    /// Construct a workflow status column.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: TaskStatusIcon,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
        }
    }
}

/// Colored label tag used on task cards, toolbar filter chips, and the detail drawer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskBoardLabel {
    /// Stable label id (`"website"`, `"billing"`).
    pub id: SharedString,
    /// Human-readable label text.
    pub label: SharedString,
    /// RGB hex swatch (`0x4fb0fe`).
    pub color_rgb: u32,
}

impl TaskBoardLabel {
    /// Construct a colored board label.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        color_rgb: u32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color_rgb,
        }
    }
}

/// Team member assignable to a [`BoardTask`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskBoardPerson {
    /// Stable person id.
    pub id: SharedString,
    /// Full display name rendered via [`pro_person_avatar`].
    pub name: SharedString,
}

impl TaskBoardPerson {
    /// Construct a team member entry.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// One subtask item inside a [`BoardTask`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSubtask {
    /// Subtask title.
    pub title: SharedString,
    /// Whether this subtask is complete.
    pub done: bool,
}

impl TaskSubtask {
    /// Construct a subtask checklist item.
    pub fn new(title: impl Into<SharedString>, done: bool) -> Self {
        Self {
            title: title.into(),
            done,
        }
    }
}

/// One engineering task card managed by [`TaskBoardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardTask {
    /// Issue key (`"NW-214"`).
    pub key: SharedString,
    /// Task summary title.
    pub title: SharedString,
    /// Owning column status id (`"backlog"`, `"todo"`, `"in_progress"`, `"done"`).
    pub status: SharedString,
    /// Signal-bar priority.
    pub priority: TaskPriority,
    /// Optional T-shirt size estimate (`"S"`, `"M"`, `"L"`, `"XL"`).
    pub estimate: Option<SharedString>,
    /// Attached label ids.
    pub labels: Vec<SharedString>,
    /// Assigned team member id.
    pub assignee: Option<SharedString>,
    /// Days until due (`<= 2` highlights amber, `< 0` highlights overdue red).
    pub due_in_days: Option<i32>,
    /// Subtask checklist items.
    pub subtasks: Vec<TaskSubtask>,
    /// Linked pull request number (`Some(412)` -> `"#412"`).
    pub pull_request: Option<u32>,
    /// Short creation date (`"Oct 1"`, `"Sep 28"`).
    pub created: SharedString,
}

/// Events emitted by [`TaskBoardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskBoardEvent {
    /// Emitted with the full updated task list after any move, edit, add, or delete.
    TasksChanged(Vec<BoardTask>),
    /// A task card moved to another column or position.
    TaskMoved {
        /// Task issue key (`"NW-214"`).
        key: SharedString,
        /// Destination status id.
        to_status: SharedString,
    },
    /// A new task was added via a column's inline quick-add form.
    TaskAdded(BoardTask),
    /// A task was deleted from the detail drawer.
    TaskDeleted(SharedString),
    /// A task card was opened or closed in the slide-over detail drawer.
    InspectorChanged(Option<SharedString>),
    /// Compact card density was toggled via the header switch.
    CompactChanged(bool),
    /// Board favorite star was toggled.
    StarredChanged(bool),
}

#[derive(Clone, Debug)]
struct DragSession {
    key: SharedString,
    start_pos: Point<Pixels>,
    active: bool,
    target_col: Option<usize>,
    target_slot: Option<usize>,
}

/// Default four-column engineering workflow (`Backlog`, `Todo`, `In Progress`, `Done`).
pub fn default_board_statuses() -> Vec<TaskColumnStatus> {
    vec![
        TaskColumnStatus::new("backlog", "Backlog", TaskStatusIcon::Backlog),
        TaskColumnStatus::new("todo", "Todo", TaskStatusIcon::Todo),
        TaskColumnStatus::new("in_progress", "In progress", TaskStatusIcon::InProgress),
        TaskColumnStatus::new("done", "Done", TaskStatusIcon::Done),
    ]
}

/// Default colored label catalog matching Northwind engineering tags.
pub fn default_board_labels() -> Vec<TaskBoardLabel> {
    vec![
        TaskBoardLabel::new("website", "Website", 0x3898ff),
        TaskBoardLabel::new("feature", "Feature", 0x8bb8e8),
        TaskBoardLabel::new("billing", "Billing", 0x9b66f2),
        TaskBoardLabel::new("bug", "Bug", 0xf0487e),
        TaskBoardLabel::new("mobile", "Mobile", 0x1fc2a7),
        TaskBoardLabel::new("infra", "Infra", 0xe8b000),
    ]
}

/// Default engineering assignees.
pub fn default_board_people() -> Vec<TaskBoardPerson> {
    vec![
        TaskBoardPerson::new("alex", "Alex Morgan"),
        TaskBoardPerson::new("maya", "Maya Lin"),
        TaskBoardPerson::new("sam", "Sam Rivera"),
        TaskBoardPerson::new("devon", "Devon Park"),
    ]
}

/// Default Sprint 12 task backlog across `Backlog`, `Todo`, `In progress`, and `Done`.
pub fn default_board_tasks() -> Vec<BoardTask> {
    vec![
        BoardTask {
            key: "NW-224".into(),
            title: "Audit webhook retry backoff for Stripe events".into(),
            status: "backlog".into(),
            priority: TaskPriority::Medium,
            estimate: Some("M".into()),
            labels: vec!["billing".into(), "infra".into()],
            assignee: Some("maya".into()),
            due_in_days: None,
            subtasks: vec![
                TaskSubtask::new("Inspect idempotency keys", true),
                TaskSubtask::new("Cap exponential jitter at 60s", true),
                TaskSubtask::new("Replay dead-letter queue fixture", false),
            ],
            pull_request: None,
            created: "Oct 3".into(),
        },
        BoardTask {
            key: "NW-218".into(),
            title: "Add keyboard shortcuts to command menu".into(),
            status: "backlog".into(),
            priority: TaskPriority::Low,
            estimate: Some("S".into()),
            labels: vec!["website".into(), "feature".into()],
            assignee: Some("sam".into()),
            due_in_days: None,
            subtasks: Vec::new(),
            pull_request: None,
            created: "Oct 1".into(),
        },
        BoardTask {
            key: "NW-214".into(),
            title: "Add the launch banner to the marketing site".into(),
            status: "todo".into(),
            priority: TaskPriority::Medium,
            estimate: Some("M".into()),
            labels: vec!["website".into(), "feature".into()],
            assignee: Some("alex".into()),
            due_in_days: None,
            subtasks: vec![
                TaskSubtask::new("Desktop & mobile hero bar", true),
                TaskSubtask::new("Persist dismiss cookie", false),
            ],
            pull_request: None,
            created: "Oct 1".into(),
        },
        BoardTask {
            key: "NW-209".into(),
            title: "Currency rounding in annual invoices".into(),
            status: "todo".into(),
            priority: TaskPriority::High,
            estimate: None,
            labels: vec!["billing".into(), "bug".into()],
            assignee: Some("devon".into()),
            due_in_days: Some(2),
            subtasks: Vec::new(),
            pull_request: None,
            created: "Sep 28".into(),
        },
        BoardTask {
            key: "NW-197".into(),
            title: "Follow up with the beta customers about pricing".into(),
            status: "todo".into(),
            priority: TaskPriority::Low,
            estimate: None,
            labels: Vec::new(),
            assignee: Some("maya".into()),
            due_in_days: None,
            subtasks: vec![
                TaskSubtask::new("Send annual discount survey", true),
                TaskSubtask::new("Book 4 founder calls", true),
                TaskSubtask::new("Synthesize seat tier feedback", false),
                TaskSubtask::new("Share notes in #launch", false),
            ],
            pull_request: None,
            created: "Sep 24".into(),
        },
        BoardTask {
            key: "NW-230".into(),
            title: "Export guide for the help center".into(),
            status: "in_progress".into(),
            priority: TaskPriority::Medium,
            estimate: None,
            labels: vec!["website".into()],
            assignee: Some("sam".into()),
            due_in_days: Some(1),
            subtasks: Vec::new(),
            pull_request: None,
            created: "Sep 30".into(),
        },
        BoardTask {
            key: "NW-201".into(),
            title: "Proration when teams add seats mid-month".into(),
            status: "in_progress".into(),
            priority: TaskPriority::High,
            estimate: Some("L".into()),
            labels: vec!["billing".into()],
            assignee: Some("devon".into()),
            due_in_days: Some(4),
            subtasks: Vec::new(),
            pull_request: Some(412),
            created: "Sep 23".into(),
        },
        BoardTask {
            key: "NW-176".into(),
            title: "Offline mode for the task list".into(),
            status: "in_progress".into(),
            priority: TaskPriority::Medium,
            estimate: Some("XL".into()),
            labels: vec!["mobile".into(), "feature".into()],
            assignee: Some("alex".into()),
            due_in_days: None,
            subtasks: Vec::new(),
            pull_request: Some(398),
            created: "Sep 19".into(),
        },
        BoardTask {
            key: "NW-160".into(),
            title: "Dark mode for the dashboard".into(),
            status: "done".into(),
            priority: TaskPriority::Medium,
            estimate: None,
            labels: vec!["website".into(), "feature".into()],
            assignee: Some("sam".into()),
            due_in_days: None,
            subtasks: Vec::new(),
            pull_request: None,
            created: "Sep 12".into(),
        },
        BoardTask {
            key: "NW-171".into(),
            title: "Fix crash when uploading large avatars".into(),
            status: "done".into(),
            priority: TaskPriority::High,
            estimate: None,
            labels: vec!["mobile".into(), "bug".into()],
            assignee: Some("devon".into()),
            due_in_days: None,
            subtasks: Vec::new(),
            pull_request: None,
            created: "Sep 15".into(),
        },
    ]
}

/// Engineering Kanban task board block with search, label chips, compact switch, quick-add, and detail drawer.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use kui::components::task_board::TaskBoardBlock;
///
/// let board = cx.new(|cx| TaskBoardBlock::new("task-board", cx));
/// # }
/// ```
pub struct TaskBoardBlock {
    id: ElementId,
    path: Vec<SharedString>,
    statuses: Vec<TaskColumnStatus>,
    labels: Vec<TaskBoardLabel>,
    people: Vec<TaskBoardPerson>,
    tasks: Vec<BoardTask>,
    active_labels: Vec<SharedString>,
    filter_open: bool,
    compact: bool,
    starred: bool,
    open_task_key: Option<SharedString>,
    last_open_task: Option<BoardTask>,
    quick_add_col: Option<SharedString>,
    quick_add_priority: TaskPriority,
    next_seq: u32,
    search_editor: Option<Entity<Editor>>,
    quick_editor: Option<Entity<Editor>>,
    title_editor: Option<Entity<Editor>>,
    synced_title_key: Option<SharedString>,
    drag: Option<DragSession>,
    col_bounds: Vec<Rc<Cell<Bounds<Pixels>>>>,
    #[allow(clippy::type_complexity)]
    card_bounds: Vec<(SharedString, usize, usize, Rc<Cell<Bounds<Pixels>>>)>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot_at: Instant,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<TaskBoardEvent> for TaskBoardBlock {}

impl TaskBoardBlock {
    /// Create a [`TaskBoardBlock`] populated with the `Northwind / Launch / Sprint 12` board.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let statuses = default_board_statuses();
        let col_bounds = (0..statuses.len())
            .map(|_| Rc::new(Cell::new(Bounds::default())))
            .collect();
        Self {
            id: id.into(),
            path: vec!["Northwind".into(), "Launch".into(), "Sprint 12".into()],
            statuses,
            labels: default_board_labels(),
            people: default_board_people(),
            tasks: default_board_tasks(),
            active_labels: Vec::new(),
            filter_open: false,
            compact: false,
            starred: false,
            open_task_key: None,
            last_open_task: None,
            quick_add_col: None,
            quick_add_priority: TaskPriority::Medium,
            next_seq: 235,
            search_editor: None,
            quick_editor: None,
            title_editor: None,
            synced_title_key: None,
            drag: None,
            col_bounds,
            card_bounds: Vec::new(),
            toast: None,
            loading: false,
            empty: false,
            boot_at: Instant::now(),
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(894.0)),
        }
    }

    /// Replace the initial task list.
    pub fn with_tasks(mut self, tasks: Vec<BoardTask>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Replace the workflow status columns.
    pub fn with_statuses(mut self, statuses: Vec<TaskColumnStatus>) -> Self {
        self.col_bounds = (0..statuses.len())
            .map(|_| Rc::new(Cell::new(Bounds::default())))
            .collect();
        self.statuses = statuses;
        self
    }

    /// Replace the label catalog.
    pub fn with_labels(mut self, labels: Vec<TaskBoardLabel>) -> Self {
        self.labels = labels;
        self
    }

    /// Replace the assignable team members.
    pub fn with_people(mut self, people: Vec<TaskBoardPerson>) -> Self {
        self.people = people;
        self
    }

    /// Override the breadcrumb path above the board.
    pub fn with_path<I, S>(mut self, path: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.path = path.into_iter().map(Into::into).collect();
        self
    }

    /// Set whether compact card density is enabled initially.
    pub fn with_compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Set whether the board star toggle starts active.
    pub fn with_starred(mut self, starred: bool) -> Self {
        self.starred = starred;
        self
    }

    /// Configure skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return all tasks currently on the board.
    pub fn tasks(&self) -> &[BoardTask] {
        &self.tasks
    }

    /// Return whether compact card density is active.
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    /// Return the issue key currently open in the detail drawer, if any.
    pub fn open_task(&self) -> Option<&SharedString> {
        self.open_task_key.as_ref()
    }

    /// Toggle compact card density and emit [`TaskBoardEvent::CompactChanged`].
    pub fn set_compact(&mut self, compact: bool, cx: &mut Context<Self>) {
        if self.compact != compact {
            self.compact = compact;
            cx.emit(TaskBoardEvent::CompactChanged(compact));
            cx.notify();
        }
    }

    /// Toggle a label filter chip by id.
    pub fn toggle_label_filter(&mut self, label_id: &str, cx: &mut Context<Self>) {
        if let Some(pos) = self
            .active_labels
            .iter()
            .position(|l| l.as_ref() == label_id)
        {
            self.active_labels.remove(pos);
        } else {
            self.active_labels
                .push(SharedString::from(label_id.to_owned()));
        }
        cx.notify();
    }

    /// Open a task in the slide-over detail drawer (`None` closes the drawer).
    pub fn inspect_task(&mut self, key: Option<SharedString>, cx: &mut Context<Self>) {
        if let Some(ref k) = key
            && let Some(found) = self.tasks.iter().find(|t| &t.key == k)
        {
            self.last_open_task = Some(found.clone());
        }
        self.open_task_key = key.clone();
        cx.emit(TaskBoardEvent::InspectorChanged(key));
        cx.notify();
    }

    /// Move `key` to `to_status` (optionally inserting at `slot_within_col`) and emit change events.
    pub fn move_task(
        &mut self,
        key: &str,
        to_status: &str,
        slot_within_col: Option<usize>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(orig_pos) = self.tasks.iter().position(|t| t.key.as_ref() == key) else {
            return false;
        };
        let mut card = self.tasks.remove(orig_pos);
        card.status = SharedString::from(to_status.to_owned());
        let moved_key = card.key.clone();
        let dest_status = card.status.clone();

        if let Some(slot) = slot_within_col {
            let mut seen = 0usize;
            let mut insert_at = self.tasks.len();
            for (idx, t) in self.tasks.iter().enumerate() {
                if t.status.as_ref() == to_status {
                    if seen == slot {
                        insert_at = idx;
                        break;
                    }
                    seen += 1;
                }
            }
            self.tasks.insert(insert_at, card);
        } else {
            self.tasks.push(card);
        }

        if self.open_task_key.as_deref() == Some(key)
            && let Some(updated) = self.tasks.iter().find(|t| t.key.as_ref() == key)
        {
            self.last_open_task = Some(updated.clone());
        }

        cx.emit(TaskBoardEvent::TaskMoved {
            key: moved_key,
            to_status: dest_status,
        });
        cx.emit(TaskBoardEvent::TasksChanged(self.tasks.clone()));
        cx.notify();
        true
    }

    /// Shift `key` one column left (`delta = -1`) or right (`delta = 1`).
    pub fn shift_task_column(&mut self, key: &str, delta: isize, cx: &mut Context<Self>) -> bool {
        let Some(cur_status) = self
            .tasks
            .iter()
            .find(|t| t.key.as_ref() == key)
            .map(|t| t.status.clone())
        else {
            return false;
        };
        let Some(col_idx) = self.statuses.iter().position(|s| s.id == cur_status) else {
            return false;
        };
        let next_idx =
            (col_idx as isize + delta).clamp(0, self.statuses.len() as isize - 1) as usize;
        if next_idx == col_idx {
            return false;
        }
        let target_id = self.statuses[next_idx].id.to_string();
        self.move_task(key, &target_id, None, cx)
    }

    /// Append a new task to `status_id` and emit [`TaskBoardEvent::TaskAdded`].
    pub fn add_task(
        &mut self,
        status_id: &str,
        title: impl Into<String>,
        priority: TaskPriority,
        cx: &mut Context<Self>,
    ) -> Option<SharedString> {
        let trimmed = title.into().trim().to_owned();
        if trimmed.is_empty() {
            return None;
        }
        let key: SharedString = format!("NW-{}", self.next_seq).into();
        self.next_seq += 1;
        let task = BoardTask {
            key: key.clone(),
            title: trimmed.into(),
            status: SharedString::from(status_id.to_owned()),
            priority,
            estimate: Some("M".into()),
            labels: self.active_labels.iter().take(2).cloned().collect(),
            assignee: self.people.first().map(|p| p.id.clone()),
            due_in_days: None,
            subtasks: Vec::new(),
            pull_request: None,
            created: "Just now".into(),
        };
        self.tasks.insert(0, task.clone());
        self.empty = false;
        self.toast = Some((
            format!("{} {}", tr(cx, "Added", "已新建"), key).into(),
            Instant::now(),
        ));
        cx.emit(TaskBoardEvent::TaskAdded(task));
        cx.emit(TaskBoardEvent::TasksChanged(self.tasks.clone()));
        cx.notify();
        Some(key)
    }

    /// Delete `key` from the board and emit [`TaskBoardEvent::TaskDeleted`].
    pub fn delete_task(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(pos) = self.tasks.iter().position(|t| t.key.as_ref() == key) else {
            return false;
        };
        let removed = self.tasks.remove(pos);
        if self.open_task_key.as_deref() == Some(key) {
            self.open_task_key = None;
            cx.emit(TaskBoardEvent::InspectorChanged(None));
        }
        self.toast = Some((
            format!("{} {}", tr(cx, "Deleted", "已删除"), removed.key).into(),
            Instant::now(),
        ));
        cx.emit(TaskBoardEvent::TaskDeleted(removed.key));
        cx.emit(TaskBoardEvent::TasksChanged(self.tasks.clone()));
        cx.notify();
        true
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search", "搜索任务…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Search, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ed, w, |_, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Escape => {
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.quick_editor.is_none() {
            let ph: SharedString = tr(cx, "New task title…", "输入新任务标题…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    if let Some(col_id) = this.quick_add_col.clone() {
                        let val = ed_ent.read(cx).value().to_string();
                        let prio = this.quick_add_priority;
                        if this.add_task(&col_id, val, prio, cx).is_some() {
                            ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                            this.quick_add_col = None;
                        }
                    }
                }
                EditEvent::Escape => {
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    this.quick_add_col = None;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.quick_editor = Some(ed);
        }

        if self.title_editor.is_none() {
            let ph: SharedString = tr(cx, "Task title", "任务标题").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 14.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, _, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted)
                    && let Some(open_k) = this.open_task_key.clone()
                {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty()
                        && let Some(t) = this.tasks.iter_mut().find(|t| t.key == open_k)
                        && t.title.as_ref() != val
                    {
                        t.title = val.into();
                        this.last_open_task = Some(t.clone());
                        cx.emit(TaskBoardEvent::TasksChanged(this.tasks.clone()));
                        cx.notify();
                    }
                }
            })
            .detach();
            self.title_editor = Some(ed);
        }

        if self.synced_title_key != self.open_task_key {
            self.synced_title_key = self.open_task_key.clone();
            if let (Some(k), Some(ed)) = (&self.open_task_key, &self.title_editor)
                && let Some(t) = self.tasks.iter().find(|t| &t.key == k)
            {
                let cur_title = t.title.to_string();
                ed.update(cx, |e, cx| e.set_value(cur_title, w, cx));
            }
        }
    }

    fn matches_filters(&self, task: &BoardTask, query: &str) -> bool {
        if !self.active_labels.is_empty()
            && !self
                .active_labels
                .iter()
                .any(|lid| task.labels.contains(lid))
        {
            return false;
        }
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        task.key.to_ascii_lowercase().contains(&q)
            || task.title.to_ascii_lowercase().contains(&q)
            || task
                .labels
                .iter()
                .any(|l| l.to_ascii_lowercase().contains(&q))
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.open_task_key.is_some() {
                self.inspect_task(None, cx);
            } else if self.quick_add_col.is_some() {
                self.quick_add_col = None;
                cx.notify();
            } else if !self.active_labels.is_empty() {
                self.active_labels.clear();
                cx.notify();
            }
            return;
        }
        if let Some(open_k) = self.open_task_key.clone()
            && ev.keystroke.modifiers.alt
        {
            match key {
                "left" => {
                    self.shift_task_column(&open_k, -1, cx);
                    w.prevent_default();
                }
                "right" => {
                    self.shift_task_column(&open_k, 1, cx);
                    w.prevent_default();
                }
                _ => {}
            }
        }
    }

    fn handle_board_mouse_move(
        &mut self,
        ev: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut drag) = self.drag.clone() else {
            return;
        };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.drag = None;
            cx.notify();
            return;
        }
        let dx = f32::from(ev.position.x - drag.start_pos.x);
        let dy = f32::from(ev.position.y - drag.start_pos.y);
        if !drag.active && dx.hypot(dy) > 5.0 {
            drag.active = true;
        }
        if drag.active {
            let mut found_col = None;
            for (ci, cell) in self.col_bounds.iter().enumerate() {
                let b = cell.get();
                if b.size.width > px(8.0) && ev.position.x >= b.left() && ev.position.x <= b.right()
                {
                    found_col = Some(ci);
                    break;
                }
            }
            if let Some(ci) = found_col {
                let mut slot = 0usize;
                for (_, card_col, card_slot, cb_cell) in &self.card_bounds {
                    if *card_col == ci {
                        let cb = cb_cell.get();
                        if cb.size.height > px(4.0) && ev.position.y > cb.center().y {
                            slot = (*card_slot) + 1;
                        }
                    }
                }
                drag.target_col = Some(ci);
                drag.target_slot = Some(slot);
            }
            self.drag = Some(drag);
            cx.notify();
        }
    }

    fn handle_board_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if drag.active {
            if let Some(ci) = drag.target_col
                && let Some(status) = self.statuses.get(ci).map(|s| s.id.to_string())
            {
                self.move_task(&drag.key, &status, drag.target_slot, cx);
            } else {
                cx.notify();
            }
        } else {
            self.inspect_task(Some(drag.key), cx);
        }
    }

    fn render_priority_bars(prio: TaskPriority, p: &ProPalette) -> Div {
        let on_count = prio.active_bars();
        let active_col = match prio {
            TaskPriority::None => p.faint,
            TaskPriority::Low => p.blue,
            TaskPriority::Medium => p.amber,
            TaskPriority::High | TaskPriority::Urgent => p.red,
        };
        div()
            .w(px(14.))
            .h(px(14.))
            .flex_none()
            .flex()
            .items_end()
            .justify_center()
            .gap(px(1.5))
            .pb(px(1.5))
            .children(
                [(1u8, 4.0), (2u8, 7.0), (3u8, 10.0)]
                    .into_iter()
                    .map(|(bar_idx, h)| {
                        let on = on_count >= bar_idx;
                        div().w(px(2.6)).h(px(h)).rounded(px(1.0)).bg(if on {
                            active_col
                        } else {
                            p.faint.opacity(0.35)
                        })
                    }),
            )
    }

    fn render_status_glyph(icon: TaskStatusIcon, side: f32, p: &ProPalette) -> AnyElement {
        let (svg, col) = match icon {
            TaskStatusIcon::Backlog => (SVG_BACKLOG_DASH, p.muted),
            TaskStatusIcon::Todo => (SVG_TODO_SQ, p.muted),
            TaskStatusIcon::InReview => (SVG_REVIEW_EYE, p.muted),
            TaskStatusIcon::InProgress => (SVG_PROGRESS_CLOCK, p.muted),
            TaskStatusIcon::Done => (SVG_DONE_BOX, p.muted),
        };
        pro_svg("0 0 16 16", svg, side, col)
    }

    fn find_label(&self, id: &str) -> Option<&TaskBoardLabel> {
        self.labels.iter().find(|l| l.id.as_ref() == id)
    }

    fn find_person_name(&self, id: &str) -> Option<&SharedString> {
        self.people
            .iter()
            .find(|p| p.id.as_ref() == id)
            .map(|p| &p.name)
    }

    fn render_assignee_avatar(
        &self,
        person_id: &str,
        side: f32,
        p: &ProPalette,
        cx: &App,
    ) -> Option<AnyElement> {
        let pname = self.find_person_name(person_id)?;
        let low_id = person_id.to_ascii_lowercase();
        let low_name = pname.to_ascii_lowercase();
        let agent_seed = if low_id.contains("claude")
            || low_name.contains("claude")
            || low_name.contains("opus")
            || low_name.contains("sonnet")
        {
            Some("claude")
        } else if low_id.contains("gpt")
            || low_id.contains("openai")
            || low_id.contains("codex")
            || low_name.contains("gpt")
            || low_name.contains("codex")
        {
            Some("gpt")
        } else if low_id.contains("gemini") || low_name.contains("gemini") {
            Some("gemini")
        } else {
            None
        };
        if let Some(seed) = agent_seed {
            Some(pro_agent_avatar(seed, p.blue, side, false, 0., cx))
        } else {
            Some(pro_person_avatar(pname, side, false, None, cx).into_any_element())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_task_card(
        &self,
        task: &BoardTask,
        status_meta: &TaskColumnStatus,
        col_idx: usize,
        total_cols: usize,
        bounds_cell: Rc<Cell<Bounds<Pixels>>>,
        is_dragging: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_open = self.open_task_key.as_ref() == Some(&task.key);
        let key_down = task.key.clone();
        let key_enter = task.key.clone();
        let key_left = task.key.to_string();
        let key_right = task.key.to_string();

        let on_md: TaskMouseDownHandler =
            Rc::new(cx.listener(move |this, ev: &MouseDownEvent, _, _| {
                this.drag = Some(DragSession {
                    key: key_down.clone(),
                    start_pos: ev.position,
                    active: false,
                    target_col: Some(col_idx),
                    target_slot: None,
                });
            }));
        let on_kd: TaskKeyDownHandler =
            Rc::new(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                if matches!(ev.keystroke.key.as_str(), "enter" | " ") {
                    this.inspect_task(Some(key_enter.clone()), cx);
                }
            }));

        let person_avatar = task.assignee.as_deref().and_then(|pid| {
            self.render_assignee_avatar(pid, if self.compact { 16. } else { 18. }, p, cx)
        });

        if self.compact {
            let dots: Vec<_> = task
                .labels
                .iter()
                .filter_map(|lid| self.find_label(lid))
                .take(3)
                .map(|lbl| div().size(px(6.)).rounded_full().bg(rgb(lbl.color_rgb)))
                .collect();

            let compact_inner = div()
                .px(px(9.))
                .py(px(7.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(Self::render_priority_bars(task.priority, p))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(11.))
                        .text_color(p.muted)
                        .child(task.key.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(task.title.clone()),
                )
                .when(!dots.is_empty(), |d| {
                    d.child(div().flex().items_center().gap(px(3.)).children(dots))
                })
                .when_some(person_avatar, |d, av| d.child(av))
                .into_any_element();

            return TaskCardSurface {
                id: pro_eid(&self.id, format!("tb-card-{}", task.key)),
                aria: format!("{} {}", task.key, task.title).into(),
                radius: 8.,
                is_open,
                is_dragging,
                bounds_cell,
                on_mouse_down: on_md,
                on_key_down: on_kd,
                content: compact_inner,
            }
            .into_any_element();
        }

        // Standard card layout matching reference pixel geometry
        let meta_line = match &task.estimate {
            Some(est) => format!(
                "{} · {} {est} · {} {}",
                task.key,
                tr(cx, "Estimate", "预估"),
                tr(cx, "Created", "创建于"),
                task.created
            ),
            None => format!(
                "{} · {} {}",
                task.key,
                tr(cx, "Created", "创建于"),
                task.created
            ),
        };

        let resolved_labels: Vec<&TaskBoardLabel> = task
            .labels
            .iter()
            .filter_map(|lid| self.find_label(lid))
            .collect();
        let tags_row = if resolved_labels.is_empty() {
            div()
                .text_size(px(11.5))
                .text_color(p.faint)
                .child(tr(cx, "No labels", "无标签"))
        } else {
            let extra = resolved_labels.len().saturating_sub(3);
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.))
                .text_size(px(11.5))
                .text_color(p.muted)
                .children(resolved_labels.into_iter().take(3).map(|lbl| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.5))
                        .child(div().size(px(6.)).rounded_full().bg(rgb(lbl.color_rgb)))
                        .child(lbl.label.clone())
                }))
                .when(extra > 0, |d| {
                    d.child(div().text_color(p.faint).child(format!("+{extra}")))
                })
        };

        let due_badge = task.due_in_days.map(|days| {
            let (bg, fg, txt) = if days < 0 {
                (p.red_soft, p.red_ink, format!("{}d", days.abs()))
            } else if days <= 2 {
                (p.amber_soft, p.amber_ink, format!("{days}d"))
            } else {
                (transparent_black(), p.faint, format!("{days}d"))
            };
            div()
                .h(px(17.))
                .px(px(5.))
                .rounded(px(4.5))
                .bg(bg)
                .flex()
                .items_center()
                .gap(px(3.))
                .text_size(px(11.))
                .font_weight(if days <= 2 {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(fg)
                .child(pro_icon(ProIcon::Clock, 10.5, fg))
                .child(txt)
        });

        let subtask_badge = (!task.subtasks.is_empty()).then(|| {
            let done_n = task.subtasks.iter().filter(|s| s.done).count();
            let tot_n = task.subtasks.len();
            let ratio = done_n as f32 / tot_n as f32;
            div()
                .flex()
                .items_center()
                .gap(px(3.5))
                .text_size(px(11.))
                .text_color(p.faint)
                .child(pro_progress_ring(ratio, 11., 1.8, p.muted, p.line_strong))
                .child(pro_swap_label(
                    pro_eid(&self.id, format!("tb-csub-cnt-{}", task.key)),
                    format!("{done_n}/{tot_n}"),
                    11.,
                    FontWeight::NORMAL,
                    p.faint,
                ))
        });

        let pr_badge = task.pull_request.map(|pr_num| {
            div()
                .flex()
                .items_center()
                .gap(px(3.))
                .text_size(px(11.))
                .text_color(p.faint)
                .child(pro_icon(ProIcon::GitBranch, 10.5, p.faint))
                .child(format!("#{pr_num}"))
        });

        let zh_loc = UiLanguage::current(cx) == UiLanguage::Chinese;
        let disp_title: SharedString = if zh_loc {
            match task.title.as_ref() {
                "Audit webhook retry backoff for Stripe events" => {
                    "排查 Stripe 事件 Webhook 重试退避策略".into()
                }
                "Add keyboard shortcuts to command menu" => "为命令面板补充全局键盘快捷键".into(),
                "Add the launch banner to the marketing site" => {
                    "在官网首页上线版本发布横幅".into()
                }
                "Currency rounding in annual invoices" => "修复年度账单币种四舍五入精度问题".into(),
                "Follow up with the beta customers about pricing" => {
                    "跟进内测客户对新版定价方案的反馈".into()
                }
                "Export guide for the help center" => "编写帮助中心的数据导出操作指南".into(),
                "Proration when teams add seats mid-month" => {
                    "支持团队月中增购席位按比例折算计费".into()
                }
                "Offline mode for the task list" => "支持移动端任务列表离线缓存模式".into(),
                "Dark mode for the dashboard" => "完成数据看板深色模式主题适配".into(),
                "Fix crash when uploading large avatars" => {
                    "修复上传超大尺寸头像时的崩溃缺陷".into()
                }
                _ => task.title.clone(),
            }
        } else {
            task.title.clone()
        };
        let disp_status_lbl: SharedString = if zh_loc {
            match status_meta.label.as_ref() {
                "Backlog" => "待规划".into(),
                "Todo" => "待处理".into(),
                "In progress" | "In Progress" => "进行中".into(),
                "Done" => "已完成".into(),
                _ => status_meta.label.clone(),
            }
        } else {
            status_meta.label.clone()
        };
        let can_left = col_idx > 0;
        let can_right = col_idx + 1 < total_cols;

        let shift_left_btn = can_left.then(|| ColShiftMiniBtn {
            id: pro_eid(&self.id, format!("tb-mv-l-{}", task.key)),
            aria: tr(cx, "Move left", "左移一列").into(),
            icon: ProIcon::ChevronLeft,
            on_shift: Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.shift_task_column(&key_left, -1, cx);
            })),
        });
        let shift_right_btn = can_right.then(|| ColShiftMiniBtn {
            id: pro_eid(&self.id, format!("tb-mv-r-{}", task.key)),
            aria: tr(cx, "Move right", "右移一列").into(),
            icon: ProIcon::ChevronRight,
            on_shift: Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.shift_task_column(&key_right, 1, cx);
            })),
        });
        let card_bg_hover = p.card;

        let standard_inner = div()
            .relative()
            .px(px(10.))
            .pt(px(9.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(6.))
                    .child(
                        div()
                            .mt(px(1.5))
                            .child(Self::render_priority_bars(task.priority, p)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.2))
                            .line_height(px(16.8))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(disp_title),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(11.2))
                            .line_height(px(15.5))
                            .text_color(p.muted)
                            .child(meta_line),
                    )
                    .child(tags_row),
            )
            .child(
                div()
                    .pt(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(5.))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(4.5))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(Self::render_status_glyph(status_meta.icon, 11.5, p))
                            .child(div().whitespace_nowrap().child(disp_status_lbl)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .when_some(due_badge, |d, b| d.child(b))
                            .when_some(subtask_badge, |d, b| d.child(b))
                            .when_some(pr_badge, |d, b| d.child(b))
                            .when_some(person_avatar, |d, av| d.child(av)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(6.))
                    .right(px(6.))
                    .px(px(2.))
                    .py(px(1.))
                    .rounded(px(5.))
                    .bg(card_bg_hover)
                    .border_1()
                    .border_color(p.line)
                    .opacity(0.)
                    .group_hover("tb-card-grp", |s| s.opacity(1.))
                    .flex()
                    .items_center()
                    .gap(px(1.))
                    .when_some(shift_left_btn, |d, b| d.child(b))
                    .when_some(shift_right_btn, |d, b| d.child(b)),
            )
            .into_any_element();

        TaskCardSurface {
            id: pro_eid(&self.id, format!("tb-card-{}", task.key)),
            aria: format!("{} {}", task.key, task.title).into(),
            radius: 9.,
            is_open,
            is_dragging,
            bounds_cell,
            on_mouse_down: on_md,
            on_key_down: on_kd,
            content: standard_inner,
        }
        .into_any_element()
    }

    fn render_detail_drawer(
        &self,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_open = self.open_task_key.is_some();
        let active_task = self
            .open_task_key
            .as_ref()
            .and_then(|k| self.tasks.iter().find(|t| &t.key == k).cloned())
            .or_else(|| self.last_open_task.clone());

        let Some(task) = active_task else {
            return div().into_any_element();
        };

        let task_key = task.key.clone();
        let key_left = task_key.to_string();
        let key_right = task_key.to_string();
        let key_del = task_key.to_string();
        let cur_col_idx = self
            .statuses
            .iter()
            .position(|s| s.id == task.status)
            .unwrap_or(0);
        let can_left = cur_col_idx > 0;
        let can_right = cur_col_idx + 1 < self.statuses.len();

        let thumb_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };

        // Status segmented strip with pro_pill_indicator
        let st_sensor = pro_selection_sensor();
        let mut status_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tb-drw-st-pill"),
                task.status.clone(),
                st_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for st in &self.statuses {
            let on = st.id == task.status;
            let k_mv = task_key.to_string();
            let st_id = st.id.to_string();
            status_strip = status_strip.child(
                pro_tactile(
                    pro_eid(&self.id, format!("tb-drw-st-{}", st.id)),
                    st.label.clone(),
                    6.,
                    transparent_black(),
                    if on { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    false,
                    div()
                        .h(px(24.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(4.5))
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(if on {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if on { p.strong } else { p.text })
                        .child(Self::render_status_glyph(st.icon, 11., p))
                        .child(st.label.clone()),
                    cx,
                    move |this, _, cx| {
                        this.move_task(&k_mv, &st_id, None, cx);
                    },
                )
                .relative()
                .flex_none()
                .when(on, |d| d.child(pro_mark_selected(&st_sensor))),
            );
        }

        let prio_buttons: Vec<_> = TaskPriority::PICKABLE
            .into_iter()
            .map(|pr| {
                let on = task.priority == pr
                    || (pr == TaskPriority::High && task.priority == TaskPriority::Urgent);
                let k_pr = task_key.clone();
                let lbl = pr.label(cx);
                pro_tactile(
                    pro_eid(&self.id, format!("tb-drw-pr-{}", pr as u8)),
                    lbl,
                    6.,
                    if on { p.select } else { p.chip },
                    if on { p.select } else { p.chip_hover },
                    if on { p.line_strong } else { p.line_soft },
                    p.line_strong,
                    on,
                    div()
                        .h(px(26.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .whitespace_nowrap()
                        .text_size(px(12.))
                        .font_weight(if on {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if on { p.strong } else { p.text })
                        .child(Self::render_priority_bars(pr, p))
                        .child(pr.short_label(cx)),
                    cx,
                    move |this, _, cx| {
                        if let Some(t) = this.tasks.iter_mut().find(|t| t.key == k_pr) {
                            t.priority = pr;
                            this.last_open_task = Some(t.clone());
                            cx.emit(TaskBoardEvent::TasksChanged(this.tasks.clone()));
                            cx.notify();
                        }
                    },
                )
                .flex_none()
            })
            .collect();

        let est_buttons: Vec<_> = ["S", "M", "L", "XL"]
            .into_iter()
            .map(|sz| {
                let on = task.estimate.as_deref() == Some(sz);
                let k_est = task_key.clone();
                pro_tactile(
                    pro_eid(&self.id, format!("tb-drw-est-{sz}")),
                    sz,
                    5.,
                    if on { p.select } else { p.chip },
                    if on { p.select } else { p.chip_hover },
                    if on { p.line_strong } else { p.line_soft },
                    p.line_strong,
                    on,
                    div()
                        .h(px(24.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if on { p.strong } else { p.muted })
                        .child(sz),
                    cx,
                    move |this, _, cx| {
                        if let Some(t) = this.tasks.iter_mut().find(|t| t.key == k_est) {
                            t.estimate = if t.estimate.as_deref() == Some(sz) {
                                None
                            } else {
                                Some(sz.into())
                            };
                            this.last_open_task = Some(t.clone());
                            cx.emit(TaskBoardEvent::TasksChanged(this.tasks.clone()));
                            cx.notify();
                        }
                    },
                )
                .flex_none()
            })
            .collect();

        let person_buttons: Vec<_> = self
            .people
            .iter()
            .map(|per| {
                let on = task.assignee.as_ref() == Some(&per.id);
                let k_per = task_key.clone();
                let pid = per.id.clone();
                pro_tactile(
                    pro_eid(&self.id, format!("tb-drw-per-{}", per.id)),
                    per.name.clone(),
                    999.,
                    if on { p.select } else { p.chip },
                    if on { p.select } else { p.chip_hover },
                    if on { p.line_strong } else { p.line_soft },
                    p.line_strong,
                    on,
                    div()
                        .h(px(26.))
                        .pl(px(5.))
                        .pr(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .text_color(if on { p.strong } else { p.text })
                        .child(pro_person_avatar(&per.name, 16., false, None, cx))
                        .child(per.name.clone()),
                    cx,
                    move |this, _, cx| {
                        if let Some(t) = this.tasks.iter_mut().find(|t| t.key == k_per) {
                            t.assignee = Some(pid.clone());
                            this.last_open_task = Some(t.clone());
                            cx.emit(TaskBoardEvent::TasksChanged(this.tasks.clone()));
                            cx.notify();
                        }
                    },
                )
                .flex_none()
            })
            .collect();

        let label_toggles: Vec<_> = self
            .labels
            .iter()
            .map(|lbl| {
                let on = task.labels.contains(&lbl.id);
                let k_lbl = task_key.clone();
                let lid = lbl.id.clone();
                pro_tactile(
                    pro_eid(&self.id, format!("tb-drw-lbl-{}", lbl.id)),
                    lbl.label.clone(),
                    999.,
                    if on { p.select } else { p.chip },
                    if on { p.select } else { p.chip_hover },
                    if on { p.line_strong } else { p.line_soft },
                    p.line_strong,
                    on,
                    div()
                        .h(px(24.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .text_color(if on { p.strong } else { p.muted })
                        .child(div().size(px(6.5)).rounded_full().bg(rgb(lbl.color_rgb)))
                        .child(lbl.label.clone()),
                    cx,
                    move |this, _, cx| {
                        if let Some(t) = this.tasks.iter_mut().find(|t| t.key == k_lbl) {
                            if let Some(pos) = t.labels.iter().position(|x| x == &lid) {
                                t.labels.remove(pos);
                            } else {
                                t.labels.push(lid.clone());
                            }
                            this.last_open_task = Some(t.clone());
                            cx.emit(TaskBoardEvent::TasksChanged(this.tasks.clone()));
                            cx.notify();
                        }
                    },
                )
                .flex_none()
            })
            .collect();

        let sub_done_count = task.subtasks.iter().filter(|s| s.done).count();
        let sub_total_count = task.subtasks.len();
        let sub_ratio = if sub_total_count > 0 {
            sub_done_count as f32 / sub_total_count as f32
        } else {
            0.0
        };

        let subtask_rows: Vec<_> = task
            .subtasks
            .iter()
            .enumerate()
            .map(|(si, sub)| {
                let k_sub = task_key.clone();
                let done = sub.done;
                pro_tactile(
                    pro_eid(&self.id, format!("tb-drw-sub-{si}")),
                    sub.title.clone(),
                    6.,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    done,
                    div()
                        .px(px(8.))
                        .py(px(5.5))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(if done { p.muted } else { p.ink })
                        .child(pro_check_draw(
                            pro_eid(&self.id, format!("tb-sub-chk-{task_key}-{si}")),
                            done,
                            14.,
                            true,
                            p.blue,
                        ))
                        .child(div().flex_1().min_w_0().child(sub.title.clone())),
                    cx,
                    move |this, _, cx| {
                        if let Some(t) = this.tasks.iter_mut().find(|t| t.key == k_sub)
                            && let Some(st) = t.subtasks.get_mut(si)
                        {
                            st.done = !st.done;
                            this.last_open_task = Some(t.clone());
                            cx.emit(TaskBoardEvent::TasksChanged(this.tasks.clone()));
                            cx.notify();
                        }
                    },
                )
                .w_full()
            })
            .collect();

        let title_ed = self.title_editor.clone();
        let drawer_w = if narrow { 320.0 } else { 336.0 };

        let sheet_body =
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(task_key.clone()),
                                )
                                .child("·")
                                .child(task.created.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(div().when(!can_left, |d| d.opacity(0.35)).child(
                                    pro_icon_btn(
                                        pro_eid(&self.id, "tb-drw-prev-col"),
                                        tr(cx, "Previous status", "移至前一状态"),
                                        ProIcon::ArrowLeft,
                                        26.,
                                        false,
                                        cx,
                                        move |this, _, cx| {
                                            if can_left {
                                                this.shift_task_column(&key_left, -1, cx);
                                            }
                                        },
                                    ),
                                ))
                                .child(div().when(!can_right, |d| d.opacity(0.35)).child(
                                    pro_icon_btn(
                                        pro_eid(&self.id, "tb-drw-next-col"),
                                        tr(cx, "Next status", "移至下一状态"),
                                        ProIcon::ArrowRight,
                                        26.,
                                        false,
                                        cx,
                                        move |this, _, cx| {
                                            if can_right {
                                                this.shift_task_column(&key_right, 1, cx);
                                            }
                                        },
                                    ),
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "tb-drw-del"),
                                    tr(cx, "Delete task", "删除任务"),
                                    ProIcon::Trash,
                                    26.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        this.delete_task(&key_del, cx);
                                    },
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "tb-drw-close"),
                                    tr(cx, "Close details", "关闭详情"),
                                    ProIcon::Close,
                                    26.,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.inspect_task(None, cx);
                                    },
                                )),
                        ),
                )
                .child(
                    div()
                        .h(px(34.))
                        .px(px(8.))
                        .py(px(5.))
                        .rounded(px(8.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.line)
                        .when_some(title_ed, |d, ed| d.child(ed)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Status", "状态")),
                        )
                        .child(status_strip),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Priority", "优先级")),
                        )
                        .child(div().flex().flex_wrap().gap(px(5.)).children(prio_buttons)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Estimate", "工作量预估")),
                        )
                        .child(div().flex().gap(px(5.)).children(est_buttons)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Assignee", "负责人")),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(5.))
                                .children(person_buttons),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Labels", "标签")),
                        )
                        .child(div().flex().flex_wrap().gap(px(5.)).children(label_toggles)),
                )
                .when(!subtask_rows.is_empty(), |d| {
                    d.child(
                        div()
                            .pt(px(6.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "Subtasks", "子任务清单")),
                                    )
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, format!("tb-drw-scnt-{task_key}")),
                                        format!("{sub_done_count}/{sub_total_count}"),
                                        11.,
                                        FontWeight::MEDIUM,
                                        p.muted,
                                    )),
                            )
                            .child(pro_spring_bar(
                                pro_eid(&self.id, format!("tb-drw-sbar-{task_key}")),
                                sub_ratio,
                                3.5,
                                2.,
                                p.chip,
                                if sub_done_count == sub_total_count {
                                    p.green
                                } else {
                                    p.blue
                                },
                            ))
                            .children(subtask_rows),
                    )
                });

        let sheet = div()
            .id(pro_eid(&self.id, "tb-detail-sheet"))
            .role(Role::Dialog)
            .aria_label(tr(cx, "Task details", "任务详情"))
            .w(px(drawer_w))
            .max_w_full()
            .h_full()
            .p(px(16.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .overflow_y_scroll()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .child(pro_view_transition(
                pro_eid(&self.id, "tb-drw-task-vtx"),
                task_key,
                sheet_body,
            ));

        let scrim_alpha = if p.dark { 0.36 } else { 0.14 };
        let dismiss = cx.listener(|this, _: &MouseDownEvent, _, cx| {
            this.inspect_task(None, cx);
        });

        pro_presence(
            pro_eid(&self.id, "tb-drawer-pres"),
            is_open,
            0.22,
            0.16,
            move |amt| {
                let e = pro_ease(amt);
                div()
                    .id("tb-drawer-scrim")
                    .absolute()
                    .inset_0()
                    .bg(hsla(0., 0., 0., scrim_alpha * e))
                    .p(px(8.))
                    .flex()
                    .justify_end()
                    .on_mouse_down(MouseButton::Left, dismiss)
                    .child(
                        div()
                            .h_full()
                            .relative()
                            .left(px(36.0 * (1.0 - e)))
                            .opacity(e)
                            .child(sheet),
                    )
                    .into_any_element()
            },
        )
    }
}

impl Render for TaskBoardBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.loading && !reduced {
            w.request_animation_frame();
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() >= 2.45)
        {
            self.toast = None;
        }

        let width = self.measured_width.get();
        let narrow = width < 620.0;
        let very_narrow = width < 440.0;
        let measure_cell = self.measured_width.clone();

        let query = self
            .search_editor
            .as_ref()
            .map(|ed| ed.read(cx).value().to_string())
            .unwrap_or_default();

        let visible_total = self
            .tasks
            .iter()
            .filter(|t| self.matches_filters(t, &query))
            .count();

        // Top header bar: breadcrumb + star on left; task count + search Editor + Compact pro_switch on right
        let crumbs_slice: &[SharedString] = if very_narrow && self.path.len() > 2 {
            &self.path[(self.path.len() - 2)..]
        } else {
            &self.path
        };
        let last_crumb_idx = crumbs_slice.len().saturating_sub(1);

        let star_btn = pro_tactile(
            pro_eid(&self.id, "tb-star"),
            tr(cx, "Add to favorites", "添加到收藏"),
            7.,
            transparent_black(),
            p.hover,
            transparent_black(),
            transparent_black(),
            self.starred,
            div()
                .size(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .child(if self.starred {
                    pro_svg("0 0 24 24", SVG_STAR_FILLED, 14., p.amber)
                } else {
                    pro_icon(ProIcon::Star, 14., p.muted)
                }),
            cx,
            |this, _, cx| {
                this.starred = !this.starred;
                cx.emit(TaskBoardEvent::StarredChanged(this.starred));
                cx.notify();
            },
        )
        .flex_none();

        let breadcrumb = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.))
            .text_size(px(13.))
            .children(crumbs_slice.iter().enumerate().map(|(ci, seg)| {
                let is_last = ci == last_crumb_idx;
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when(ci > 0, |d| d.child(div().text_color(p.faint).child("/")))
                    .child(
                        div()
                            .truncate()
                            .font_weight(if is_last {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if is_last { p.ink } else { p.muted })
                            .child(seg.clone()),
                    )
            }))
            .child(star_btn);

        let search_ed = self.search_editor.clone();
        let filter_active = self.filter_open || !self.active_labels.is_empty();
        let active_lbl_cnt = self.active_labels.len();
        let filter_btn = pro_tactile(
            pro_eid(&self.id, "tb-filter-btn"),
            tr(cx, "Filter labels", "筛选标签"),
            7.,
            if filter_active {
                p.chip
            } else {
                transparent_black()
            },
            p.hover,
            if filter_active {
                p.line
            } else {
                transparent_black()
            },
            p.line_strong,
            filter_active,
            div()
                .h(px(26.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .text_size(px(12.))
                .font_weight(if filter_active {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if filter_active { p.ink } else { p.muted })
                .child(pro_icon(
                    ProIcon::Filter,
                    12.5,
                    if filter_active { p.ink } else { p.muted },
                ))
                .child(tr(cx, "Filter", "筛选"))
                .when(active_lbl_cnt > 0, |d| {
                    d.child(
                        div()
                            .min_w(px(16.))
                            .h(px(16.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.blue)
                            .child(format!("{active_lbl_cnt}")),
                    )
                }),
            cx,
            |this, _, cx| {
                this.filter_open = !this.filter_open;
                if !this.filter_open {
                    this.active_labels.clear();
                }
                cx.notify();
            },
        )
        .flex_none();

        let top_bar = div()
            .px(px(if narrow { 10. } else { 14. }))
            .pt(px(9.))
            .pb(px(8.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(breadcrumb)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .when(!very_narrow, |d| {
                        d.child(pro_swap_label(
                            pro_eid(&self.id, "tb-vis-count"),
                            format!("{visible_total} {}", tr(cx, "tasks", "项任务")),
                            11.5,
                            FontWeight::NORMAL,
                            p.faint,
                        ))
                    })
                    .child(
                        div()
                            .w(px(if very_narrow { 120. } else { 144. }))
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(pro_icon(ProIcon::Search, 12., p.muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(19.))
                                    .when_some(search_ed, |d, ed| d.child(ed)),
                            ),
                    )
                    .child(filter_btn)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(pro_switch(
                                pro_eid(&self.id, "tb-compact-sw"),
                                tr(cx, "Compact cards", "紧凑卡片模式"),
                                self.compact,
                                cx,
                                |this, _, cx| {
                                    let next = !this.compact;
                                    this.set_compact(next, cx);
                                },
                            ))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(if self.compact { p.ink } else { p.muted })
                                    .child(tr(cx, "Compact", "紧凑")),
                            ),
                    ),
            );

        // Label filter chips strip with liquid capsule pro_pill_indicator
        let all_active = self.active_labels.is_empty();
        let pill_key: SharedString = self
            .active_labels
            .last()
            .cloned()
            .unwrap_or_else(|| "all".into());
        let label_sensor = pro_selection_sensor();
        let pill_fill: Hsla = if all_active { p.select } else { p.accent_soft };
        let pill_border: Option<Hsla> = Some(if all_active {
            p.line_strong
        } else {
            p.blue.opacity(0.45)
        });

        let mut filter_strip = div()
            .relative()
            .px(px(if narrow { 10. } else { 14. }))
            .pb(px(8.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(5.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tb-label-pill"),
                pill_key.clone(),
                label_sensor.clone(),
                999.,
                pill_fill,
                pill_border,
                false,
            ))
            .child(
                pro_tactile(
                    pro_eid(&self.id, "tb-fchip-all"),
                    tr(cx, "All labels", "全部标签"),
                    999.,
                    if all_active {
                        transparent_black()
                    } else {
                        p.chip
                    },
                    if all_active {
                        transparent_black()
                    } else {
                        p.chip_hover
                    },
                    if all_active {
                        transparent_black()
                    } else {
                        p.line_soft
                    },
                    if all_active {
                        transparent_black()
                    } else {
                        p.line_strong
                    },
                    false,
                    div()
                        .h(px(23.))
                        .px(px(8.5))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(if all_active {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if all_active { p.strong } else { p.muted })
                        .child(tr(cx, "All labels", "全部标签")),
                    cx,
                    |this, _, cx| {
                        this.active_labels.clear();
                        cx.notify();
                    },
                )
                .relative()
                .flex_none()
                .when(all_active, |d| d.child(pro_mark_selected(&label_sensor))),
            );

        for lbl in &self.labels {
            let on = self.active_labels.contains(&lbl.id);
            let is_pill_anchor = pill_key == lbl.id;
            let lid = lbl.id.to_string();
            filter_strip = filter_strip.child(
                pro_tactile(
                    pro_eid(&self.id, format!("tb-fchip-{}", lbl.id)),
                    lbl.label.clone(),
                    999.,
                    if is_pill_anchor {
                        transparent_black()
                    } else if on {
                        p.accent_soft
                    } else {
                        p.chip
                    },
                    if is_pill_anchor {
                        transparent_black()
                    } else if on {
                        p.accent_soft
                    } else {
                        p.chip_hover
                    },
                    if is_pill_anchor {
                        transparent_black()
                    } else if on {
                        p.blue.opacity(0.45)
                    } else {
                        p.line_soft
                    },
                    if is_pill_anchor {
                        transparent_black()
                    } else if on {
                        p.blue.opacity(0.55)
                    } else {
                        p.line_strong
                    },
                    false,
                    div()
                        .h(px(23.))
                        .px(px(8.5))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(if on {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if on { p.blue } else { p.text })
                        .child(div().size(px(6.5)).rounded_full().bg(rgb(lbl.color_rgb)))
                        .child(lbl.label.clone()),
                    cx,
                    move |this, _, cx| {
                        this.toggle_label_filter(&lid, cx);
                    },
                )
                .relative()
                .flex_none()
                .when(is_pill_anchor, |d| {
                    d.child(pro_mark_selected(&label_sensor))
                }),
            );
        }
        let filter_fold = pro_fold(
            pro_eid(&self.id, "tb-filter-fold"),
            filter_active,
            34.,
            0.,
            move || filter_strip.into_any_element(),
        );

        while self.col_bounds.len() < self.statuses.len() {
            self.col_bounds.push(Rc::new(Cell::new(Bounds::default())));
        }
        self.card_bounds.clear();

        let board_body = if self.loading {
            let ph = (self.boot_at.elapsed().as_secs_f32() / 1.4).fract();
            div()
                .px(px(12.))
                .pb(px(12.))
                .flex()
                .gap(px(10.))
                .children((0..4).map(|i| {
                    div()
                        .flex_1()
                        .min_w(px(196.))
                        .p(px(8.))
                        .rounded(px(14.))
                        .bg(p.subtle)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(96., 12., 4., ph + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(180., 92., 10., ph + i as f32 * 0.15, cx))
                        .child(pro_skeleton_bar(180., 92., 10., ph + i as f32 * 0.2, cx))
                }))
                .into_any_element()
        } else if self.empty {
            pro_empty_state(
                ProIcon::Tasks,
                tr(cx, "No tasks in this sprint", "当前迭代暂无任务"),
                tr(
                    cx,
                    "Add tasks in any workflow column or restore the sample Sprint 12 backlog.",
                    "可在任意状态列内快速新建卡片，或载入示例迭代数据。",
                ),
                Some((
                    pro_eid(&self.id, "tb-restore-empty"),
                    tr(cx, "Load Sprint 12 tasks", "载入示例迭代任务").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        this.empty = false;
                        if this.tasks.is_empty() {
                            this.tasks = default_board_tasks();
                        }
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else {
            let statuses_snapshot = self.statuses.clone();
            let tasks_snapshot = self.tasks.clone();
            let total_cols = statuses_snapshot.len();
            let dragging_key = self
                .drag
                .as_ref()
                .filter(|d| d.active)
                .map(|d| d.key.clone());
            let drag_target = self
                .drag
                .as_ref()
                .filter(|d| d.active)
                .and_then(|d| d.target_col.map(|ci| (ci, d.target_slot.unwrap_or(0))));

            let col_frame_bg: Hsla = if p.dark {
                hsla(0., 0., 1., 0.032)
            } else {
                hsla(0., 0., 0., 0.024)
            };

            let mut col_els = Vec::with_capacity(total_cols);
            for (ci, st_col) in statuses_snapshot.iter().enumerate() {
                let col_cards: Vec<&BoardTask> = tasks_snapshot
                    .iter()
                    .filter(|t| t.status == st_col.id && self.matches_filters(t, &query))
                    .collect();
                let count = col_cards.len();
                let count_txt = if count > 999 {
                    "999+".to_owned()
                } else {
                    format!("{count}")
                };

                let is_drop_col = drag_target.is_some_and(|(tc, _)| tc == ci);
                let drop_col_t = pro_spring_f32(
                    pro_eid(&self.id, format!("tb-dcol-{}", st_col.id)),
                    f32::from(is_drop_col),
                    0.18,
                    w,
                    cx,
                )
                .clamp(0., 1.);
                let col_bg = pro_mix_color(col_frame_bg, p.accent_soft, drop_col_t);
                let col_border =
                    pro_mix_color(transparent_black(), p.blue.opacity(0.45), drop_col_t);
                let col_cell = self.col_bounds[ci].clone();

                let mut card_els: Vec<AnyElement> = Vec::with_capacity(col_cards.len());
                for (si, card) in col_cards.iter().take(50).enumerate() {
                    let slot_before = drag_target == Some((ci, si));
                    let cb_cell = Rc::new(Cell::new(Bounds::default()));
                    self.card_bounds
                        .push((card.key.clone(), ci, si, cb_cell.clone()));
                    let is_card_dragging = dragging_key.as_ref() == Some(&card.key);
                    let card_el = self.render_task_card(
                        card,
                        st_col,
                        ci,
                        total_cols,
                        cb_cell,
                        is_card_dragging,
                        &p,
                        cx,
                    );
                    card_els.push(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(DropSlotBar {
                                id: pro_eid(&self.id, format!("tb-slot-{ci}-{si}")),
                                active: slot_before,
                                bottom_gap: 6.0,
                                top_gap: 0.0,
                            })
                            .child(card_el)
                            .into_any_element(),
                    );
                }
                let tail_slot_active =
                    drag_target.is_some_and(|(tc, slot)| tc == ci && slot >= col_cards.len());
                let tail_slot = DropSlotBar {
                    id: pro_eid(&self.id, format!("tb-slot-tail-{ci}")),
                    active: tail_slot_active,
                    bottom_gap: 0.0,
                    top_gap: 2.0,
                };

                // Inline quick-add card form in each column
                let quick_open = self.quick_add_col.as_ref() == Some(&st_col.id);
                let quick_ed = self.quick_editor.clone();
                let st_add_id = st_col.id.to_string();
                let st_submit_id = st_col.id.to_string();
                let prio_now = self.quick_add_priority;
                let p_copy = p;

                let prio_cycle_btn = pro_tactile(
                    pro_eid(&self.id, format!("tb-qa-prio-{}", st_col.id)),
                    prio_now.label(cx),
                    5.,
                    p.chip,
                    p.chip_hover,
                    transparent_black(),
                    p.line_strong,
                    false,
                    div()
                        .h(px(24.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .whitespace_nowrap()
                        .text_size(px(11.))
                        .text_color(p.text)
                        .child(Self::render_priority_bars(prio_now, &p))
                        .child(prio_now.short_label(cx)),
                    cx,
                    |this, _, cx| {
                        this.quick_add_priority = this.quick_add_priority.next_quick();
                        cx.notify();
                    },
                )
                .flex_none()
                .into_any_element();

                let quick_ed_submit = quick_ed.clone();
                let save_btn = pro_btn(
                    pro_eid(&self.id, format!("tb-qa-save-{}", st_col.id)),
                    tr(cx, "Add", "添加"),
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, w, cx| {
                        if let Some(ed) = &quick_ed_submit {
                            let val = ed.read(cx).value().to_string();
                            let pr = this.quick_add_priority;
                            if this.add_task(&st_submit_id, val, pr, cx).is_some() {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                                this.quick_add_col = None;
                            }
                        }
                    },
                )
                .into_any_element();

                let cancel_btn = pro_icon_btn(
                    pro_eid(&self.id, format!("tb-qa-cancel-{}", st_col.id)),
                    tr(cx, "Cancel", "取消"),
                    ProIcon::Close,
                    24.,
                    false,
                    cx,
                    |this, w, cx| {
                        if let Some(ed) = &this.quick_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        this.quick_add_col = None;
                        cx.notify();
                    },
                )
                .into_any_element();

                let quick_fold = pro_fold(
                    pro_eid(&self.id, format!("tb-qa-fold-{}", st_col.id)),
                    quick_open,
                    82.,
                    0.,
                    move || {
                        div()
                            .mt(px(4.))
                            .p(px(9.))
                            .rounded(px(9.))
                            .bg(p_copy.card)
                            .border_1()
                            .border_color(p_copy.blue.opacity(0.45))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(div().h(px(24.)).when_some(quick_ed, |d, ed| d.child(ed)))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(6.))
                                    .child(prio_cycle_btn)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(cancel_btn)
                                            .child(save_btn),
                                    ),
                            )
                            .into_any_element()
                    },
                );

                let st_open_id = st_col.id.clone();
                let add_btn_el = pro_tactile(
                    pro_eid(&self.id, format!("tb-add-btn-{st_add_id}")),
                    format!("{} {}", tr(cx, "Add task to", "添加任务到"), st_col.label),
                    7.,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    false,
                    div()
                        .h(px(24.))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Plus, 12.5, p.muted)),
                    cx,
                    move |this, w, cx| {
                        this.quick_add_col = Some(st_open_id.clone());
                        if let Some(ed) = &this.quick_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                        cx.notify();
                    },
                )
                .mt(px(1.))
                .w_full()
                .into_any_element();

                let add_trigger = pro_popover_presence(
                    pro_eid(&self.id, format!("tb-add-pres-{}", st_col.id)),
                    !quick_open,
                    move || add_btn_el,
                );

                let col_w = if narrow { 232.0 } else { 0.0 };
                let col_div = div()
                    .id(pro_eid(&self.id, format!("tb-col-{}", st_col.id)))
                    .relative()
                    .when(narrow, |d| d.w(px(col_w)).flex_none())
                    .when(!narrow, |d| d.flex_1().min_w(px(184.)))
                    .min_h(px(210.))
                    .p(px(4.))
                    .rounded(px(11.))
                    .bg(col_bg)
                    .border_1()
                    .border_color(col_border)
                    .flex()
                    .flex_col()
                    .child(
                        canvas(move |b, _, _| col_cell.set(b), |_, _, _, _| ())
                            .absolute()
                            .inset_0(),
                    )
                    .child(
                        div()
                            .h(px(28.))
                            .px(px(6.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(5.))
                            .child(Self::render_status_glyph(st_col.icon, 12.5, &p))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_center()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(if UiLanguage::current(cx) == UiLanguage::Chinese {
                                        match st_col.label.as_ref() {
                                            "Backlog" => "待规划".to_owned(),
                                            "Todo" => "待处理".to_owned(),
                                            "In progress" | "In Progress" => "进行中".to_owned(),
                                            "Done" => "已完成".to_owned(),
                                            _ => st_col.label.to_uppercase(),
                                        }
                                    } else {
                                        st_col.label.to_uppercase()
                                    }),
                            )
                            .child(
                                div()
                                    .min_w(px(18.))
                                    .h(px(16.))
                                    .px(px(4.5))
                                    .rounded(px(5.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, format!("tb-col-cnt-{}", st_col.id)),
                                        count_txt,
                                        10.5,
                                        FontWeight::MEDIUM,
                                        p.muted,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .pt(px(1.))
                            .children(card_els)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .child(tail_slot)
                                    .child(quick_fold)
                                    .child(add_trigger),
                            ),
                    );

                col_els.push(col_div);
            }

            div()
                .id(pro_eid(&self.id, "tb-cols-scroll"))
                .px(px(10.))
                .pb(px(10.))
                .w_full()
                .overflow_x_scroll()
                .flex()
                .items_start()
                .gap(px(7.))
                .children(col_els)
                .into_any_element()
        };

        let mode_key: SharedString = if self.loading {
            "loading".into()
        } else if self.empty {
            "empty".into()
        } else if self.compact {
            "compact".into()
        } else {
            "standard".into()
        };

        let drawer = self.render_detail_drawer(narrow, &p, cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_move(cx.listener(Self::handle_board_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_board_mouse_up))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if narrow { 12. } else { 18. }))
            .py(px(if narrow { 14. } else { 18. }))
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .rounded(px(12.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .shadow(pro_card_shadow(p.dark))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(top_bar)
                    .child(filter_fold)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "tb-mode-vtx"),
                        mode_key,
                        board_body,
                    ))
                    .child(drawer),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskBoardBlock, TaskPriority, default_board_statuses, default_board_tasks};

    #[test]
    fn default_sprint_board_defines_four_workflow_columns_and_ten_tasks() {
        let statuses = default_board_statuses();
        let tasks = default_board_tasks();
        assert_eq!(statuses.len(), 4);
        assert_eq!(statuses[0].id.as_ref(), "backlog");
        assert_eq!(statuses[1].id.as_ref(), "todo");
        assert_eq!(statuses[2].id.as_ref(), "in_progress");
        assert_eq!(statuses[3].id.as_ref(), "done");
        assert_eq!(tasks.len(), 10);
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn task_board_moves_adds_inspects_and_toggles_compact_density() {
        let mut app = gpui::TestAppContext::single();
        app.update(crate::init);
        let (board, w) = app.add_window_view(|_, cx| TaskBoardBlock::new("test-task-board", cx));
        w.update(|_, cx| {
            board.update(cx, |b, cx| {
                assert!(!b.is_compact());
                b.set_compact(true, cx);
                assert!(b.is_compact());

                assert!(b.move_task("NW-214", "in_progress", Some(0), cx));
                let moved = b
                    .tasks()
                    .iter()
                    .find(|t| t.key.as_ref() == "NW-214")
                    .expect("NW-214 exists");
                assert_eq!(moved.status.as_ref(), "in_progress");

                let created = b
                    .add_task(
                        "backlog",
                        "Wire up webhook replay CLI",
                        TaskPriority::High,
                        cx,
                    )
                    .expect("created task key");
                assert_eq!(created.as_ref(), "NW-235");
                assert_eq!(b.tasks().len(), 11);

                b.inspect_task(Some(created.clone()), cx);
                assert_eq!(b.open_task(), Some(&created));
                assert!(b.delete_task(created.as_ref(), cx));
                assert_eq!(b.open_task(), None);
                assert_eq!(b.tasks().len(), 10);
            });
        });
    }
}
