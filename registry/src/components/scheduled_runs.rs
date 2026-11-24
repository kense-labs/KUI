//! Weekly session-quota task packer with gap filling, multi-session task splitting, and fly-in motion.
//!
//! Schedules a backlog of agent tasks across a multi-day grid of quota-capped sessions (`100%`
//! default limit, up to `2` tasks per session): small tasks pack into remaining capacity gaps,
//! oversized tasks (`> 100%`) split cleanly across adjacent sessions, and cards stagger smoothly
//! into their slots with interactive task addition, manual placement, and agent filtering.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_skeleton_bar, pro_spring_bar,
        pro_toast_banner, tr,
    },
    primitives::{
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, person_initials, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::collections::HashMap;
use web_time::Instant;

const BOARD_MAX_W: f32 = 816.0;
const STACK_W: f32 = 220.0;
const CELL_H: f32 = 156.0;
const CARD_H: f32 = 62.0;
const MAX_CARDS_PER_SESSION: usize = 2;

/// Scheduling strategy of [`ScheduledRuns`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScheduleMode {
    /// Automatically packs backlog tasks into free session capacity and splits oversized tasks.
    #[default]
    Auto,
    /// Keeps unbooked tasks in the stack so users can pick a card and place it into any free session.
    Manual,
}

/// Coordinates `(day, session)` of one calendar execution window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionSlot {
    /// Zero-based day column index.
    pub day: usize,
    /// Zero-based session row index within `day`.
    pub session: usize,
}

impl SessionSlot {
    /// Create a [`SessionSlot`] at `(day, session)`.
    pub const fn new(day: usize, session: usize) -> Self {
        Self { day, session }
    }
}

/// Column header descriptor for one day in [`ScheduledRuns`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleDay {
    /// Short weekday label (`"Tue"`, `"Wed"`, `"Thu"`).
    pub name: SharedString,
    /// Optional Chinese weekday label (`"周二"`, `"周三"`, `"周四"`).
    pub name_zh: Option<SharedString>,
    /// Optional calendar day number (`"27"`).
    pub date: Option<SharedString>,
}

impl ScheduleDay {
    /// Create a [`ScheduleDay`] with `name` and optional `date`.
    pub fn new(name: impl Into<SharedString>, date: Option<&str>) -> Self {
        Self {
            name: name.into(),
            name_zh: None,
            date: date.map(|d| SharedString::from(d.to_string())),
        }
    }

    /// Provide a localized Chinese weekday label.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }
}

/// One queued or pre-booked agent task in [`ScheduledRuns`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledTask {
    /// Unique task identifier.
    pub id: SharedString,
    /// Concise task title.
    pub title: SharedString,
    /// Optional Chinese task title.
    pub title_zh: Option<SharedString>,
    /// One-line scope detail shown on the card and inspector.
    pub detail: Option<SharedString>,
    /// Optional Chinese scope detail.
    pub detail_zh: Option<SharedString>,
    /// Agent runner name (`"Claude Code"`, `"Codex"`, `"Gemini CLI"`).
    pub agent: SharedString,
    /// Provider logo key passed to [`pro_app_artwork`] (`"claude"`, `"openai"`, `"gemini"`).
    pub icon: SharedString,
    /// Share of one session's quota consumed by this task (in percent; `> limit` splits across sessions).
    pub usage: u32,
    /// Pre-booked or hand-placed starting [`SessionSlot`], if fixed.
    pub booked_at: Option<SessionSlot>,
}

impl ScheduledTask {
    /// Create an unbooked [`ScheduledTask`] requiring `usage` percent of one session.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        icon: impl Into<SharedString>,
        usage: u32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            title_zh: None,
            detail: None,
            detail_zh: None,
            agent: agent.into(),
            icon: icon.into(),
            usage: usage.max(1),
            booked_at: None,
        }
    }

    /// Provide a localized Chinese task title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the descriptive scope line.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Provide a localized Chinese scope line.
    pub fn with_detail_zh(mut self, detail_zh: impl Into<SharedString>) -> Self {
        self.detail_zh = Some(detail_zh.into());
        self
    }

    /// Pin or pre-book this task into `slot`.
    pub fn with_booked_at(mut self, slot: SessionSlot) -> Self {
        self.booked_at = Some(slot);
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()),
            None => self.title.as_ref(),
        }
    }

    fn display_detail<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.detail.as_deref()?;
        Some(match &self.detail_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }
}

/// One placed task slice inside a specific `(day, session)` cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledPart {
    /// Originating [`ScheduledTask`] identifier.
    pub task_id: SharedString,
    /// Assigned day and session window.
    pub slot: SessionSlot,
    /// Quota share consumed in this session.
    pub usage: u32,
    /// One-based slice number (`1` when unsplit).
    pub part: usize,
    /// Total slice count (`> 1` when an oversized task was split across sessions).
    pub parts: usize,
    /// Whether this slice came from a pre-booked / hand-placed assignment.
    pub manual: bool,
}

/// Result of packing a week of sessions via [`plan_week`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeekPlan {
    /// All placed task slices in chronological slot order.
    pub parts: Vec<ScheduledPart>,
    /// Task ids remaining in the `"To schedule"` stack.
    pub unscheduled_ids: Vec<SharedString>,
    /// Subset of `unscheduled_ids` that could not fit into remaining weekly quota in `Auto` mode.
    pub no_room_ids: Vec<SharedString>,
}

/// Pack `tasks` into `num_days × num_sessions` quota-capped windows (`limit` per session, max `2` cards).
///
/// Pre-booked tasks (`booked_at: Some(...)`) always stay in place; in `ScheduleMode::Auto`, remaining
/// tasks pack into gaps without splitting whenever `usage <= limit`, and split cleanly into pieces
/// `>= min_piece` across adjacent sessions when `usage > limit`.
pub fn plan_week(
    tasks: &[ScheduledTask],
    num_days: usize,
    num_sessions: usize,
    now: SessionSlot,
    limit: u32,
    min_piece: u32,
    mode: ScheduleMode,
) -> WeekPlan {
    if num_days == 0 || num_sessions == 0 {
        return WeekPlan {
            parts: Vec::new(),
            unscheduled_ids: tasks.iter().map(|t| t.id.clone()).collect(),
            no_room_ids: Vec::new(),
        };
    }
    let cap = limit.max(10);
    let min_p = min_piece.clamp(1, cap / 2);

    let mut ordered_slots = Vec::with_capacity(num_days * num_sessions);
    for d in 0..num_days {
        for s in 0..num_sessions {
            ordered_slots.push(SessionSlot::new(d, s));
        }
    }

    let mut used_map: HashMap<SessionSlot, (u32, usize)> = ordered_slots
        .iter()
        .map(|&sl| (sl, (0u32, 0usize)))
        .collect();
    let mut parts = Vec::new();
    let mut unscheduled_ids = Vec::new();
    let mut no_room_ids = Vec::new();

    // 1. Place pre-booked / manually placed tasks first (spill across subsequent slots if > limit)
    for task in tasks {
        let Some(start_slot) = task.booked_at else {
            continue;
        };
        if start_slot.day >= num_days || start_slot.session >= num_sessions {
            continue;
        }
        if start_slot < now {
            let entry = used_map.entry(start_slot).or_insert((0, 0));
            let take = task.usage.min(cap.saturating_sub(entry.0)).max(1);
            entry.0 = (entry.0 + take).min(cap);
            entry.1 += 1;
            parts.push(ScheduledPart {
                task_id: task.id.clone(),
                slot: start_slot,
                usage: take,
                part: 1,
                parts: 1,
                manual: true,
            });
            continue;
        }

        let start_idx = ordered_slots
            .iter()
            .position(|&sl| sl == start_slot)
            .unwrap_or(0);
        let mut candidate_order = Vec::new();
        candidate_order.extend_from_slice(&ordered_slots[start_idx..]);
        for &sl in &ordered_slots[..start_idx] {
            if sl >= now {
                candidate_order.push(sl);
            }
        }

        if let Some(allocs) = allocate_across_slots(
            task.usage.max(1),
            cap,
            min_p,
            false,
            &candidate_order,
            &used_map,
        ) {
            let total_p = allocs.len();
            for (idx, (sl, amt)) in allocs.into_iter().enumerate() {
                if let Some(en) = used_map.get_mut(&sl) {
                    en.0 += amt;
                    en.1 += 1;
                }
                parts.push(ScheduledPart {
                    task_id: task.id.clone(),
                    slot: sl,
                    usage: amt,
                    part: idx + 1,
                    parts: total_p,
                    manual: true,
                });
            }
        } else {
            unscheduled_ids.push(task.id.clone());
        }
    }

    // 2. Handle unbooked backlog tasks
    let future_slots: Vec<SessionSlot> = ordered_slots
        .iter()
        .copied()
        .filter(|&sl| sl >= now)
        .collect();

    for task in tasks {
        if task.booked_at.is_some() {
            continue;
        }
        if mode == ScheduleMode::Manual {
            unscheduled_ids.push(task.id.clone());
            continue;
        }
        let single_only = task.usage <= cap;
        if let Some(allocs) = allocate_across_slots(
            task.usage.max(1),
            cap,
            min_p,
            single_only,
            &future_slots,
            &used_map,
        ) {
            let total_p = allocs.len();
            for (idx, (sl, amt)) in allocs.into_iter().enumerate() {
                if let Some(en) = used_map.get_mut(&sl) {
                    en.0 += amt;
                    en.1 += 1;
                }
                parts.push(ScheduledPart {
                    task_id: task.id.clone(),
                    slot: sl,
                    usage: amt,
                    part: idx + 1,
                    parts: total_p,
                    manual: false,
                });
            }
        } else {
            unscheduled_ids.push(task.id.clone());
            no_room_ids.push(task.id.clone());
        }
    }

    WeekPlan {
        parts,
        unscheduled_ids,
        no_room_ids,
    }
}

fn allocate_across_slots(
    needed: u32,
    cap: u32,
    min_piece: u32,
    single_slot_only: bool,
    slots: &[SessionSlot],
    used_map: &HashMap<SessionSlot, (u32, usize)>,
) -> Option<Vec<(SessionSlot, u32)>> {
    if single_slot_only {
        for &sl in slots {
            let (used, cards) = used_map.get(&sl).copied().unwrap_or((0, 0));
            if cards < MAX_CARDS_PER_SESSION && cap.saturating_sub(used) >= needed {
                return Some(vec![(sl, needed)]);
            }
        }
        return None;
    }

    let mut rem = needed;
    let mut allocs = Vec::new();
    for &sl in slots {
        if rem == 0 {
            break;
        }
        let (used, cards) = used_map.get(&sl).copied().unwrap_or((0, 0));
        let avail = cap.saturating_sub(used);
        if cards >= MAX_CARDS_PER_SESSION || avail < min_piece {
            continue;
        }
        let mut take = rem.min(avail);
        if rem > take && rem - take < min_piece {
            let adjusted = rem.saturating_sub(min_piece);
            if adjusted < min_piece {
                continue;
            }
            take = adjusted;
        }
        allocs.push((sl, take));
        rem -= take;
    }
    (rem == 0).then_some(allocs)
}

/// Typed events emitted by [`ScheduledRuns`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledRunsEvent {
    /// User toggled between `Auto` and `Manual` scheduling modes.
    ModeChanged(ScheduleMode),
    /// Weekly schedule recomputed with placed task parts.
    Scheduled(Vec<ScheduledPart>),
    /// User manually placed or moved `task_id` into `slot`.
    TaskPlaced {
        /// Placed task identifier.
        task_id: SharedString,
        /// Destination session slot.
        slot: SessionSlot,
    },
    /// User added a new task to the backlog / schedule.
    TaskAdded(ScheduledTask),
    /// User filtered the board by an agent runner name (`None` when cleared).
    AgentFiltered(Option<SharedString>),
}

impl EventEmitter<ScheduledRunsEvent> for ScheduledRuns {}

/// Weekly session-quota task packer with gap packing, multi-session splitting, and fly-in stagger.
pub struct ScheduledRuns {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    note: SharedString,
    note_zh: SharedString,
    days: Vec<ScheduleDay>,
    sessions: Vec<SharedString>,
    tasks: Vec<ScheduledTask>,
    now_slot: SessionSlot,
    limit: u32,
    min_piece: u32,
    mode: ScheduleMode,
    agent_filter: Option<SharedString>,
    held_task_id: Option<SharedString>,
    inspected_task_id: Option<SharedString>,
    pack_clock: PhaseClock,
    toast: Option<(SharedString, Instant)>,
    next_seq: usize,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl ScheduledRuns {
    /// Create a [`ScheduledRuns`] board preloaded with a 9-task coding backlog across `Tue–Thu` 5-hour sessions.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let mut pack_clock = PhaseClock::default();
        pack_clock.restart(cx);
        Self {
            id: id.into(),
            title: SharedString::from("This week"),
            title_zh: SharedString::from("本周排期"),
            note: SharedString::from("5-hour sessions"),
            note_zh: SharedString::from("5 小时配额窗口"),
            days: default_days(),
            sessions: vec![
                SharedString::from("07:00"),
                SharedString::from("12:00"),
                SharedString::from("17:00"),
            ],
            tasks: default_weekly_tasks(),
            now_slot: SessionSlot::new(0, 1), // Tue 12:00 running now; Tue 07:00 is past
            limit: 100,
            min_piece: 15,
            mode: ScheduleMode::Auto,
            agent_filter: None,
            held_task_id: None,
            inspected_task_id: None,
            pack_clock,
            toast: None,
            next_seq: 20,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Set the board heading (`"This week"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Provide a localized Chinese board heading.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = title_zh.into();
        self
    }

    /// Set the subtitle note beside the heading (`"5-hour sessions"` by default).
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = note.into();
        self
    }

    /// Provide a localized Chinese subtitle note.
    pub fn with_note_zh(mut self, note_zh: impl Into<SharedString>) -> Self {
        self.note_zh = note_zh.into();
        self
    }

    /// Supply the day column headers.
    pub fn with_days(mut self, days: Vec<ScheduleDay>) -> Self {
        self.days = days;
        self
    }

    /// Supply the session start-time row headers.
    pub fn with_sessions(
        mut self,
        sessions: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.sessions = sessions.into_iter().map(Into::into).collect();
        self
    }

    /// Supply the pre-booked and queued tasks.
    pub fn with_tasks(mut self, tasks: Vec<ScheduledTask>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Set the currently running `(day, session)` boundary (`SessionSlot::new(0, 1)` by default).
    pub fn with_now(mut self, now: SessionSlot) -> Self {
        self.now_slot = now;
        self
    }

    /// Set the quota capacity per session (`100` by default).
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit.max(10);
        self
    }

    /// Set the minimum slice size when splitting oversized tasks (`15` by default).
    pub fn with_min_piece(mut self, min_piece: u32) -> Self {
        self.min_piece = min_piece.max(5);
        self
    }

    /// Set the initial scheduling mode (`Auto` or `Manual`).
    pub fn with_mode(mut self, mode: ScheduleMode) -> Self {
        self.mode = mode;
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty backlog state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.tasks.clear();
        }
        self
    }

    /// Return the configured tasks.
    pub fn tasks(&self) -> &[ScheduledTask] {
        &self.tasks
    }

    /// Return the current scheduling mode (`Auto` or `Manual`).
    pub fn mode(&self) -> ScheduleMode {
        self.mode
    }

    /// Compute the current [`WeekPlan`] across the week's sessions.
    pub fn plan(&self) -> WeekPlan {
        plan_week(
            &self.tasks,
            self.days.len(),
            self.sessions.len(),
            self.now_slot,
            self.limit,
            self.min_piece,
            self.mode,
        )
    }

    /// Switch between `Auto` and `Manual` mode, re-triggering staggered fly-in when entering `Auto`.
    pub fn set_mode(&mut self, mode: ScheduleMode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.held_task_id = None;
        if mode == ScheduleMode::Auto {
            self.pack_clock.restart(cx);
        }
        let parts = self.plan().parts;
        cx.emit(ScheduledRunsEvent::ModeChanged(mode));
        cx.emit(ScheduledRunsEvent::Scheduled(parts));
        cx.notify();
    }

    /// Re-pack all unbooked tasks into `Auto` mode and replay the staggered fly-in deal.
    pub fn rebalance(&mut self, cx: &mut Context<Self>) {
        self.mode = ScheduleMode::Auto;
        self.held_task_id = None;
        // Release manually dropped overrides except the past pre-booked anchor so gaps repack cleanly
        let now = self.now_slot;
        for (i, t) in self.tasks.iter_mut().enumerate() {
            if i > 1 && t.booked_at.is_some_and(|sl| sl >= now) {
                t.booked_at = None;
            }
        }
        self.pack_clock.restart(cx);
        let parts = self.plan().parts;
        self.toast = Some((
            SharedString::from(tr(
                cx,
                "Rebalanced tasks across session quotas",
                "已按会话配额重新均衡排布任务",
            )),
            Instant::now(),
        ));
        cx.emit(ScheduledRunsEvent::Scheduled(parts));
        cx.notify();
    }

    /// Append `task` and repack the week.
    pub fn add_task(&mut self, task: ScheduledTask, cx: &mut Context<Self>) {
        self.empty = false;
        self.tasks.push(task.clone());
        if self.mode == ScheduleMode::Auto {
            self.pack_clock.restart(cx);
        }
        let parts = self.plan().parts;
        cx.emit(ScheduledRunsEvent::TaskAdded(task));
        cx.emit(ScheduledRunsEvent::Scheduled(parts));
        cx.notify();
    }

    /// Append a realistic coding task (`35%`–`125%` quota) into the schedule.
    pub fn add_sample_task(&mut self, cx: &mut Context<Self>) {
        self.next_seq += 1;
        let samples = [
            (
                "Harden webhook signature check",
                "加固 Webhook 签名校验",
                "Replay attack window & clock skew tests",
                "补充防重放窗口与时钟漂移单测",
                "Claude Code",
                "claude",
                35u32,
            ),
            (
                "Generate OpenAPI 3.1 SDK types",
                "生成 OpenAPI 3.1 SDK 类型",
                "Regenerate Rust & TypeScript client bindings",
                "重新生成多语言强类型客户端绑定",
                "Codex",
                "openai",
                45u32,
            ),
            (
                "Shard analytics events table",
                "拆分分析事件大表分区",
                "Online monthly partitioning + backfill migration",
                "在线按月分区改造与历史数据回填",
                "Gemini CLI",
                "gemini",
                125u32,
            ),
        ];
        let (en, zh, det_en, det_zh, agent, icon, usage) = samples[self.next_seq % samples.len()];
        let task = ScheduledTask::new(format!("task-{}", self.next_seq), en, agent, icon, usage)
            .with_title_zh(zh)
            .with_detail(det_en)
            .with_detail_zh(det_zh);
        let title_shown = task.display_title(cx).to_string();
        self.toast = Some((
            SharedString::from(format!(
                "{}: {title_shown} ({usage}%)",
                tr(cx, "Queued", "已加入排期")
            )),
            Instant::now(),
        ));
        self.add_task(task, cx);
    }

    /// Manually book or move `task_id` into `slot` (splitting forward if `usage` exceeds remaining capacity).
    pub fn place_task_at(
        &mut self,
        task_id: &str,
        slot: SessionSlot,
        cx: &mut Context<Self>,
    ) -> bool {
        if slot < self.now_slot
            || slot.day >= self.days.len()
            || slot.session >= self.sessions.len()
        {
            return false;
        }
        let Some(task) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id) else {
            return false;
        };
        task.booked_at = Some(slot);
        self.held_task_id = None;
        self.pack_clock.restart(cx);
        let parts = self.plan().parts;
        cx.emit(ScheduledRunsEvent::TaskPlaced {
            task_id: SharedString::from(task_id.to_string()),
            slot,
        });
        cx.emit(ScheduledRunsEvent::Scheduled(parts));
        cx.notify();
        true
    }

    /// Toggle filtering by an agent runner name (`"Claude Code"`, `"Codex"`, `"Gemini CLI"`).
    pub fn toggle_agent_filter(&mut self, agent: &str, cx: &mut Context<Self>) {
        self.agent_filter = if self.agent_filter.as_deref() == Some(agent) {
            None
        } else {
            Some(SharedString::from(agent.to_string()))
        };
        cx.emit(ScheduledRunsEvent::AgentFiltered(self.agent_filter.clone()));
        cx.notify();
    }
}

fn agent_mark_tile(icon: &str, agent: &str, side: f32, p: &ProPalette, cx: &App) -> AnyElement {
    let bg: Hsla = if p.dark { rgb(0x232327).into() } else { p.card };
    let art_sz = (side * 0.64).round();
    let content = if let Some(art) = pro_app_artwork(icon, art_sz, cx) {
        art
    } else {
        let init = person_initials(agent);
        div()
            .text_size(px(9.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink)
            .child(if init.is_empty() {
                "A".to_string()
            } else {
                init
            })
            .into_any_element()
    };
    div()
        .size(px(side))
        .flex_none()
        .rounded(px(5.))
        .bg(bg)
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .child(content)
        .into_any_element()
}

impl Render for ScheduledRuns {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let card_bg: Hsla = if p.dark { rgb(0x1c1c1f).into() } else { p.card };
        let lane_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf4f4f5).into()
        };
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.024)
        };

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_s / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(BOARD_MAX_W))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(260., 22., 5., phase, cx))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(STACK_W, 420., 12., phase + 0.1, cx))
                        .child(pro_skeleton_bar(560., 420., 12., phase + 0.2, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.tasks.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(BOARD_MAX_W))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Calendar,
                    tr(cx, "No agent tasks queued this week", "本周暂无待排期任务"),
                    tr(
                        cx,
                        "Queue coding or research tasks to pack them into session quota windows.",
                        "添加编码或调研任务，系统将自动填补配额空隙并跨窗口拆分超长任务。",
                    ),
                    Some((
                        pro_eid(&self.id, "load-week"),
                        SharedString::from(tr(cx, "Load weekly backlog", "加载本周任务栈")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.tasks = default_weekly_tasks();
                            this.pack_clock.restart(cx);
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let plan = self.plan();
        let pack_window = 0.85_f32;
        let pack_age = self.pack_clock.time(pack_window, window, cx);

        // Distinct agent crew list for header filter chips
        let mut crew: Vec<(SharedString, SharedString)> = Vec::new();
        for t in &self.tasks {
            if !crew.iter().any(|(ag, _)| ag == &t.agent) {
                crew.push((t.agent.clone(), t.icon.clone()));
            }
        }

        let mut crew_row = div()
            .id(pro_eid(&self.id, "crew"))
            .role(Role::Group)
            .aria_label(tr(cx, "Filter tasks by agent", "按智能体筛选任务"))
            .flex()
            .items_center()
            .gap(px(4.));

        for (ag_name, ag_icon) in &crew {
            let is_on = self.agent_filter.as_ref() == Some(ag_name);
            let dimmed = self.agent_filter.is_some() && !is_on;
            let ag_click = ag_name.clone();
            let hov_border = p.line_strong;

            crew_row = crew_row.child(
                div()
                    .id(pro_eid(&self.id, format!("crew-{ag_name}")))
                    .role(Role::Button)
                    .aria_label(ag_name.clone())
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(6.))
                    .rounded(px(7.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(if is_on { p.blue } else { p.line_strong })
                    .opacity(if dimmed { 0.52 } else { 1.0 })
                    .cursor_pointer()
                    .hover(move |s| s.border_color(hov_border))
                    .focus_visible(move |s| s.border_2().border_color(p.focus))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.toggle_agent_filter(ag_click.as_ref(), cx);
                    }))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(
                        if let Some(art) = pro_app_artwork(ag_icon.as_ref(), 13.5, cx) {
                            art
                        } else {
                            div().text_size(px(10.)).child("A").into_any_element()
                        },
                    )
                    .when(is_on, |d| {
                        d.child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(ag_name.clone()),
                        )
                    }),
            );
        }

        let add_hover = p.chip_hover;
        let rebal_hover = p.chip_hover;

        // Auto / Manual segmented toggle
        let is_auto = self.mode == ScheduleMode::Auto;
        let mode_switch = div()
            .id(pro_eid(&self.id, "modes"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Scheduling mode", "排期模式"))
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(
                div()
                    .id(pro_eid(&self.id, "mode-auto"))
                    .role(Role::Tab)
                    .aria_label(tr(cx, "Auto pack sessions", "自动装箱排期"))
                    .tab_index(0)
                    .h(px(24.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(if is_auto {
                        card_bg
                    } else {
                        transparent_black()
                    })
                    .when(is_auto && !p.dark, |d| d.shadow_sm())
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.set_mode(ScheduleMode::Auto, cx);
                    }))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if is_auto { p.strong } else { p.text })
                    .child(pro_icon(
                        ProIcon::Sparkles,
                        11.5,
                        if is_auto { p.blue } else { p.muted },
                    ))
                    .child(tr(cx, "Auto", "自动")),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "mode-manual"))
                    .role(Role::Tab)
                    .aria_label(tr(cx, "Manual session placement", "手动放入窗口"))
                    .tab_index(0)
                    .h(px(24.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(if !is_auto {
                        card_bg
                    } else {
                        transparent_black()
                    })
                    .when(!is_auto && !p.dark, |d| d.shadow_sm())
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.set_mode(ScheduleMode::Manual, cx);
                    }))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if !is_auto { p.strong } else { p.text })
                    .child(tr(cx, "Manual", "手动")),
            );

        let top_bar = div()
            .w_full()
            .mb(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, self.note.as_ref(), self.note_zh.as_ref()).to_string()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(crew_row)
                    .child(
                        div()
                            .id(pro_eid(&self.id, "add-task"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Add task to week", "新增待排任务"))
                            .tab_index(0)
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line_strong)
                            .cursor_pointer()
                            .hover(move |s| s.bg(add_hover))
                            .focus_visible(move |s| s.border_2().border_color(p.focus))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.add_sample_task(cx);
                            }))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(pro_icon(ProIcon::Plus, 11.5, p.muted))
                            .child(tr(cx, "Add task", "新增任务")),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "rebalance-btn"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Rebalance weekly slots", "重新均衡排期"))
                            .tab_index(0)
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line_strong)
                            .cursor_pointer()
                            .hover(move |s| s.bg(rebal_hover))
                            .focus_visible(move |s| s.border_2().border_color(p.focus))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.rebalance(cx);
                            }))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(pro_icon(ProIcon::Refresh, 11.5, p.muted))
                            .child(tr(cx, "Rebalance", "重排均衡")),
                    )
                    .child(
                        div().text_size(px(12.)).text_color(p.muted).child(
                            tr(
                                cx,
                                &format!("{} tasks", self.tasks.len()),
                                &format!("{} 项任务", self.tasks.len()),
                            )
                            .to_string(),
                        ),
                    )
                    .child(mode_switch),
            );

        // Left Stack: "To schedule" unscheduled cards
        let stack_total_h = 30.0 + self.sessions.len() as f32 * (CELL_H + 4.0);
        let mut stack_cards = div()
            .id(pro_eid(&self.id, "stack-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(px(2.))
            .pb(px(4.))
            .flex()
            .flex_col()
            .gap(px(4.));

        if plan.unscheduled_ids.is_empty() {
            stack_cards = stack_cards.child(
                div()
                    .flex_1()
                    .min_h(px(220.))
                    .px(px(10.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_center()
                    .child(pro_icon(ProIcon::Check, 16., p.green_ink))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "Everything's scheduled",
                        "全部任务已排入会话",
                    )))
                    .child(div().text_size(px(11.)).text_color(p.faint).child(tr(
                        cx,
                        "Switch to Manual or click + Add task to test packing",
                        "切换至“手动”或点击“新增任务”测试配额装箱",
                    ))),
            );
        } else {
            for uid in &plan.unscheduled_ids {
                let Some(task) = self.tasks.iter().find(|t| &t.id == uid) else {
                    continue;
                };
                let no_room = plan.no_room_ids.contains(uid);
                let is_held = self.held_task_id.as_ref() == Some(uid);
                let dimmed = self
                    .agent_filter
                    .as_ref()
                    .is_some_and(|af| af != &task.agent);
                let tid_click = task.id.clone();

                let sub_txt = if no_room {
                    tr(
                        cx,
                        "No room left in this week's sessions",
                        "本周剩余会话配额不足",
                    )
                    .to_string()
                } else if is_held {
                    tr(
                        cx,
                        "Click any highlighted session cell to place →",
                        "请点击右侧高亮会话格放入 →",
                    )
                    .to_string()
                } else {
                    format!("{} · {}", task.agent, task.display_detail(cx).unwrap_or(""))
                };

                let card = div()
                    .id(pro_eid(&self.id, format!("stk-{}", task.id)))
                    .role(Role::Button)
                    .aria_label(format!("{} ({}%)", task.display_title(cx), task.usage))
                    .tab_index(0)
                    .w_full()
                    .h(px(CARD_H))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(if is_held {
                        p.blue
                    } else if no_room {
                        p.amber.opacity(0.55)
                    } else {
                        p.line_strong
                    })
                    .when(!p.dark, |d| d.shadow_sm())
                    .opacity(if dimmed { 0.35 } else { 1.0 })
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_2().border_color(p.focus))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.held_task_id = if this.held_task_id.as_ref() == Some(&tid_click) {
                            None
                        } else {
                            Some(tid_click.clone())
                        };
                        this.inspected_task_id = Some(tid_click.clone());
                        cx.notify();
                    }))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(agent_mark_tile(
                                task.icon.as_ref(),
                                task.agent.as_ref(),
                                18.0,
                                &p,
                                cx,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(task.display_title(cx).to_string()),
                            )
                            .child(
                                div()
                                    .px(px(5.))
                                    .h(px(15.))
                                    .rounded(px(4.))
                                    .bg(if task.usage > self.limit {
                                        p.accent_soft
                                    } else {
                                        p.chip
                                    })
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if task.usage > self.limit {
                                        p.blue
                                    } else {
                                        p.text
                                    })
                                    .flex()
                                    .items_center()
                                    .child(format!("{}%", task.usage)),
                            ),
                    )
                    .child(
                        div()
                            .pl(px(24.))
                            .truncate()
                            .text_size(px(10.5))
                            .text_color(if no_room {
                                p.amber_ink
                            } else if is_held {
                                p.blue
                            } else {
                                p.muted
                            })
                            .child(sub_txt),
                    );

                stack_cards = stack_cards.child(card);
            }
        }

        let left_stack = div()
            .w(px(STACK_W))
            .flex_none()
            .h(px(stack_total_h))
            .p(px(4.))
            .rounded(px(14.))
            .bg(lane_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(30.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "To schedule", "待排入队列")),
                    )
                    .child(
                        div()
                            .min_w(px(20.))
                            .h(px(18.))
                            .px(px(5.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(plan.unscheduled_ids.len().to_string()),
                    ),
            )
            .child(stack_cards);

        // Right Week Grid: Day column headers + Session rows
        let mut head_row = div()
            .w_full()
            .h(px(30.))
            .flex()
            .items_center()
            .gap(px(4.))
            .child(div().w(px(34.)).flex_none());

        for day in &self.days {
            head_row = head_row.child(
                div()
                    .flex_1()
                    .min_w(px(136.))
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .bg(well_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_baseline()
                    .gap(px(5.))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(day.display_name(cx).to_uppercase()),
                    )
                    .when_some(day.date.clone(), |d, dt| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(dt),
                        )
                    }),
            );
        }

        let mut week_col = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(head_row);

        let meter_fill_default: Hsla = if p.dark {
            rgb(0x55555c).into()
        } else {
            rgb(0xc0c0c8).into()
        };

        for (s_idx, s_time) in self.sessions.iter().enumerate() {
            let mut s_row = div()
                .w_full()
                .h(px(CELL_H))
                .flex()
                .items_stretch()
                .gap(px(4.))
                .child(
                    div()
                        .w(px(34.))
                        .flex_none()
                        .pt(px(6.))
                        .text_size(px(11.))
                        .text_color(p.faint)
                        .child(s_time.clone()),
                );

            for d_idx in 0..self.days.len() {
                let slot = SessionSlot::new(d_idx, s_idx);
                let is_past = slot < self.now_slot;
                let is_now = slot == self.now_slot;

                let cell_parts: Vec<(usize, &ScheduledPart)> = plan
                    .parts
                    .iter()
                    .enumerate()
                    .filter(|(_, pt)| pt.slot == slot)
                    .collect();
                let used_sum: u32 = cell_parts.iter().map(|(_, pt)| pt.usage).sum();
                let rem_cap = self.limit.saturating_sub(used_sum);
                let can_drop = !is_past
                    && cell_parts.len() < MAX_CARDS_PER_SESSION
                    && rem_cap >= self.min_piece;
                let holding_any = self.held_task_id.is_some();

                let cell_bg = if is_past {
                    transparent_black()
                } else if holding_any && can_drop {
                    p.blue.opacity(if p.dark { 0.10 } else { 0.065 })
                } else {
                    well_bg
                };
                let cell_border = if holding_any && can_drop {
                    p.blue.opacity(0.50)
                } else {
                    p.line
                };

                // Diagonal hatch overlay when holding a card and this slot is blocked (past or full)
                let blocked_hatch = (holding_any && !can_drop).then(|| {
                    let hc: Hsla = if p.dark {
                        hsla(0., 0., 1., 0.05)
                    } else {
                        hsla(0., 0., 0., 0.045)
                    };
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let w = f32::from(bounds.size.width);
                            let h = f32::from(bounds.size.height);
                            let mut off = -h;
                            while off <= w + h {
                                let x0 = off.max(0.0);
                                let y0 = (x0 - off).max(0.0);
                                let x1 = (off + h).min(w);
                                let y1 = (x1 - off).min(h);
                                if x1 > x0 + 1.0 && y1 > y0 + 1.0 {
                                    let mut ln = PathBuilder::stroke(px(1.0));
                                    ln.move_to(point(
                                        bounds.left() + px(x0),
                                        bounds.top() + px(y0),
                                    ));
                                    ln.line_to(point(
                                        bounds.left() + px(x1),
                                        bounds.top() + px(y1),
                                    ));
                                    if let Ok(path) = ln.build() {
                                        window.paint_path(path, hc);
                                    }
                                }
                                off += 8.0;
                            }
                        },
                    )
                    .absolute()
                    .inset_0()
                });

                let mut chips_col = div().relative().flex_1().flex().flex_col().gap(px(4.));

                for (global_idx, pt) in &cell_parts {
                    let Some(task) = self.tasks.iter().find(|t| t.id == pt.task_id) else {
                        continue;
                    };
                    let dimmed = is_past
                        || self
                            .agent_filter
                            .as_ref()
                            .is_some_and(|af| af != &task.agent);
                    let is_inspected = self.inspected_task_id.as_ref() == Some(&task.id);
                    let is_held = self.held_task_id.as_ref() == Some(&task.id);

                    // Staggered fly-in from left stack into session slot
                    let fly_t = if reduced || is_past {
                        1.0
                    } else {
                        let delay = (*global_idx as f32 * 0.055).min(0.45);
                        pro_ease(((pack_age - delay) / 0.34).clamp(0.0, 1.0))
                    };
                    let dx = if reduced { 0.0 } else { -16.0 * (1.0 - fly_t) };
                    let dy = if reduced { 0.0 } else { -5.0 * (1.0 - fly_t) };

                    let tid_click = task.id.clone();
                    let movable = !is_past;

                    let chip = div()
                        .id(pro_eid(
                            &self.id,
                            format!("chip-{}-{d_idx}-{s_idx}-{}", pt.task_id, pt.part),
                        ))
                        .role(Role::Button)
                        .aria_label(format!("{} ({}%)", task.display_title(cx), pt.usage))
                        .tab_index(0)
                        .relative()
                        .left(px(dx))
                        .top(px(dy))
                        .w_full()
                        .h(px(CARD_H))
                        .px(px(7.))
                        .py(px(6.))
                        .rounded(px(7.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(if is_held || is_inspected {
                            p.blue
                        } else {
                            p.line_strong
                        })
                        .when(!p.dark && !is_past, |d| d.shadow_sm())
                        .opacity((if dimmed { 0.44 } else { 1.0 }) * (0.2 + 0.8 * fly_t))
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_2().border_color(p.focus))
                        .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                            let _ = ev;
                            cx.stop_propagation();
                            this.inspected_task_id =
                                if this.inspected_task_id.as_ref() == Some(&tid_click) {
                                    None
                                } else {
                                    Some(tid_click.clone())
                                };
                            if movable {
                                this.held_task_id =
                                    if this.held_task_id.as_ref() == Some(&tid_click) {
                                        None
                                    } else {
                                        Some(tid_click.clone())
                                    };
                            }
                            cx.notify();
                        }))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .child(agent_mark_tile(
                                    task.icon.as_ref(),
                                    task.agent.as_ref(),
                                    17.0,
                                    &p,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(task.display_title(cx).to_string()),
                                )
                                .when(pt.parts > 1, |d| {
                                    d.child(
                                        div()
                                            .flex_none()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.blue)
                                            .child(format!("{}/{}", pt.part, pt.parts)),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .pl(px(22.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(10.5))
                                        .text_color(p.muted)
                                        .child(
                                            task.display_detail(cx)
                                                .unwrap_or(task.agent.as_ref())
                                                .to_string(),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .px(px(4.5))
                                        .h(px(15.))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .text_size(px(10.))
                                        .text_color(p.text)
                                        .flex()
                                        .items_center()
                                        .child(format!("{}%", pt.usage)),
                                ),
                        );

                    chips_col = chips_col.child(chip);
                }

                // Red "Now" session start indicator line with left dot
                let now_marker = is_now.then(|| {
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(-2.))
                        .h(px(2.))
                        .bg(p.red)
                        .child(
                            div()
                                .absolute()
                                .left(px(-3.5))
                                .top(px(-3.))
                                .size(px(8.))
                                .rounded_full()
                                .bg(p.red),
                        )
                });

                let ratio = (used_sum as f32 / self.limit.max(1) as f32).clamp(0.0, 1.0);
                let meter_fill = if ratio >= 0.92 {
                    p.amber
                } else if is_now {
                    p.blue
                } else {
                    meter_fill_default
                };

                let held_for_drop = self.held_task_id.clone();
                let mut cell = div()
                    .id(pro_eid(&self.id, format!("cell-{d_idx}-{s_idx}")))
                    .relative()
                    .flex_1()
                    .min_w(px(136.))
                    .h(px(CELL_H))
                    .p(px(4.))
                    .pb(px(18.))
                    .rounded(px(10.))
                    .bg(cell_bg)
                    .border_1()
                    .border_color(cell_border)
                    .flex()
                    .flex_col()
                    .children(blocked_hatch)
                    .children(now_marker)
                    .child(chips_col)
                    .child(
                        div()
                            .absolute()
                            .left(px(8.))
                            .right(px(8.))
                            .bottom(px(7.))
                            .opacity(if is_past { 0.45 } else { 1.0 })
                            .child(pro_spring_bar(
                                pro_eid(&self.id, format!("sbar-{d_idx}-{s_idx}")),
                                ratio,
                                4.0,
                                2.0,
                                p.chip,
                                meter_fill,
                            )),
                    );

                if let Some(hid) = held_for_drop
                    && can_drop
                {
                    cell = cell.cursor_pointer().on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.place_task_at(hid.as_ref(), slot, cx);
                        },
                    ));
                }

                s_row = s_row.child(cell);
            }

            week_col = week_col.child(s_row);
        }

        // Bottom selected task inspector bar when a card is clicked
        let inspector_bar = self.inspected_task_id.as_ref().and_then(|tid| {
            let task = self.tasks.iter().find(|t| &t.id == tid)?;
            let placed_slices: Vec<&ScheduledPart> =
                plan.parts.iter().filter(|pt| &pt.task_id == tid).collect();
            let where_str = if placed_slices.is_empty() {
                tr(cx, "Waiting in stack", "位于待排栈").to_string()
            } else {
                placed_slices
                    .iter()
                    .map(|pt| {
                        let d_lbl = self
                            .days
                            .get(pt.slot.day)
                            .map_or("Day", |d| d.display_name(cx));
                        let s_lbl = self
                            .sessions
                            .get(pt.slot.session)
                            .map_or("00:00", |s| s.as_ref());
                        format!("{d_lbl} {s_lbl} ({}%)", pt.usage)
                    })
                    .collect::<Vec<_>>()
                    .join(" + ")
            };
            let clear_hover = p.chip_hover;

            Some(
                div()
                    .w_full()
                    .mt(px(10.))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(9.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .child(agent_mark_tile(
                        task.icon.as_ref(),
                        task.agent.as_ref(),
                        20.0,
                        &p,
                        cx,
                    ))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(task.display_title(cx).to_string()),
                    )
                    .child(
                        div()
                            .text_color(p.muted)
                            .child(format!("{} · {}% → {where_str}", task.agent, task.usage)),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .text_size(px(11.5))
                            .text_color(p.blue)
                            .child(tr(
                                cx,
                                "Click any highlighted session to move this task",
                                "点击任意高亮会话格即可迁移此任务",
                            )),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "close-insp"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Close task details", "关闭详情"))
                            .tab_index(0)
                            .size(px(20.))
                            .rounded(px(5.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(clear_hover))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.inspected_task_id = None;
                                this.held_task_id = None;
                                cx.notify();
                            }))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Close, 11., p.muted)),
                    ),
            )
        });

        let body = div()
            .w_full()
            .flex()
            .items_start()
            .gap(px(12.))
            .child(left_stack)
            .child(week_col);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w_full()
                    .max_w(px(BOARD_MAX_W))
                    .flex()
                    .flex_col()
                    .child(top_bar)
                    .child(body)
                    .children(inspector_bar)
                    .child(pro_toast_banner(self.toast.clone())),
            )
            .into_any_element()
    }
}

fn default_days() -> Vec<ScheduleDay> {
    vec![
        ScheduleDay::new("Tue", Some("27")).with_name_zh("周二"),
        ScheduleDay::new("Wed", Some("28")).with_name_zh("周三"),
        ScheduleDay::new("Thu", Some("29")).with_name_zh("周四"),
    ]
}

fn default_weekly_tasks() -> Vec<ScheduledTask> {
    vec![
        // Completed past session on Tue 07:00
        ScheduledTask::new(
            "metrics",
            "Weekly metrics report",
            "Gemini CLI",
            "gemini",
            55,
        )
        .with_title_zh("周度核心指标周报")
        .with_detail("Compiled retention & MRR cohorts")
        .with_detail_zh("汇总留存率与经常性收入队列")
        .with_booked_at(SessionSlot::new(0, 0)),
        // Pre-booked anchor in the running Tue 12:00 session (40% used -> 60% gap left!)
        ScheduledTask::new("triage", "Release blocker sweep", "Codex", "openai", 40)
            .with_title_zh("发版阻塞缺陷清扫")
            .with_detail("Verify 8 P0 regressions before freeze")
            .with_detail_zh("代码冻结前核验 8 项 P0 回归")
            .with_booked_at(SessionSlot::new(0, 1)),
        // Oversized task (140% > 100%!) -> splits cleanly across sessions (60% into Tue 12:00 gap + 80% into Tue 17:00!)
        ScheduledTask::new(
            "oauth",
            "Migrate auth to OAuth 2.1",
            "Claude Code",
            "claude",
            140,
        )
        .with_title_zh("鉴权升级至 OAuth 2.1")
        .with_detail("Move every login to PKCE & preserve refresh tokens")
        .with_detail_zh("全量迁移至 PKCE 并兼容存量刷新令牌"),
        // Small 20% task -> packs right into the remaining 20% gap of Tue 17:00!
        ScheduledTask::new(
            "changelog",
            "Draft release notes",
            "Gemini CLI",
            "gemini",
            20,
        )
        .with_title_zh("生成版本发布说明")
        .with_detail("Group merged PRs since Nov 1 for customers")
        .with_detail_zh("按功能模块归纳合并 PR 说明"),
        // Fits Wed 07:00 (55%)
        ScheduledTask::new("api-docs", "Public API reference", "Codex", "openai", 55)
            .with_title_zh("补全公开 API 参考手册")
            .with_detail("Every endpoint with curl and Rust SDK examples")
            .with_detail_zh("为每个端点生成 curl 与 SDK 调用示例"),
        // Small 40% task -> packs into Wed 07:00 (55% + 40% = 95%!)
        ScheduledTask::new(
            "flaky",
            "Fix flaky checkout tests",
            "Claude Code",
            "claude",
            40,
        )
        .with_title_zh("修复结账偶发失败测试")
        .with_detail("Isolate webhook race in 12 integration specs")
        .with_detail_zh("隔离 12 组集成用例中的回调竞态"),
        // Oversized 155% task -> splits across Wed 12:00 (100%) + Wed 17:00 (55%)!
        ScheduledTask::new(
            "billing",
            "Extract billing webhooks",
            "Claude Code",
            "claude",
            155,
        )
        .with_title_zh("拆出计费 Webhook 服务")
        .with_detail("Move invoices, usage meters & retries out of monolith")
        .with_detail_zh("将账单、用量计量与重试从单体应用解耦"),
        // Small 45% task -> packs into Wed 17:00 (55% + 45% = 100%!)
        ScheduledTask::new(
            "tickets",
            "Cluster support themes",
            "Gemini CLI",
            "gemini",
            45,
        )
        .with_title_zh("聚类客服工单主题")
        .with_detail("Group 480 tickets with representative quotes")
        .with_detail_zh("对 480 张工单提炼高频反馈原声"),
        // Thu 07:00 (65%) leaving Thu 07:00 gap + Thu 12:00 + Thu 17:00 ready for + Add task!
        ScheduledTask::new("audit", "Dependency security audit", "Codex", "openai", 65)
            .with_title_zh("依赖项安全合规审计")
            .with_detail("Patch advisory crates & verify lockfile MSRV")
            .with_detail_zh("升级安全公告依赖并校验锁文件兼容性"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ScheduleMode, ScheduledRuns, ScheduledTask, SessionSlot, plan_week};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn packs_small_tasks_into_gaps_and_splits_oversized_tasks_respecting_min_piece() {
        let tasks = vec![
            ScheduledTask::new("booked", "Booked", "Codex", "openai", 40)
                .with_booked_at(SessionSlot::new(0, 0)),
            ScheduledTask::new("big", "Migrate OAuth", "Claude Code", "claude", 140),
            ScheduledTask::new("small", "Quick fix", "Gemini CLI", "gemini", 20),
        ];
        let plan = plan_week(
            &tasks,
            2,
            2,
            SessionSlot::new(0, 0),
            100,
            15,
            ScheduleMode::Auto,
        );
        assert!(plan.unscheduled_ids.is_empty());

        // "big" (140%) splits into 60% in (0,0) + 80% in (0,1), and "small" (20%) packs into (0,1)!
        let big_parts: Vec<_> = plan.parts.iter().filter(|p| p.task_id == "big").collect();
        assert_eq!(big_parts.len(), 2);
        assert_eq!(big_parts[0].slot, SessionSlot::new(0, 0));
        assert_eq!(big_parts[0].usage, 60);
        assert_eq!(big_parts[1].slot, SessionSlot::new(0, 1));
        assert_eq!(big_parts[1].usage, 80);

        let small_part = plan
            .parts
            .iter()
            .find(|p| p.task_id == "small")
            .expect("small task scheduled");
        assert_eq!(small_part.slot, SessionSlot::new(0, 1));
        assert_eq!(small_part.usage, 20);
    }

    #[gpui::test]
    fn manual_mode_returns_unbooked_tasks_to_stack_and_supports_interactive_add(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let view = cx.new(|cx| ScheduledRuns::new("test-srn", cx));

        view.update(cx, |v, cx| {
            let auto_plan = v.plan();
            assert!(
                auto_plan.unscheduled_ids.is_empty(),
                "all 9 default tasks fit across Tue-Thu in Auto mode"
            );

            // Switching to Manual returns unbooked tasks to the stack while keeping booked tasks placed
            v.set_mode(ScheduleMode::Manual, cx);
            let manual_plan = v.plan();
            assert_eq!(manual_plan.unscheduled_ids.len(), 7);
            assert_eq!(manual_plan.parts.len(), 2);

            // Back to Auto + adding a new task packs it cleanly
            v.set_mode(ScheduleMode::Auto, cx);
            let before_n = v.tasks().len();
            v.add_sample_task(cx);
            assert_eq!(v.tasks().len(), before_n + 1);
            assert!(v.plan().unscheduled_ids.is_empty());
        });
    }
}
