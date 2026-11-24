//! Stacked deck of pending agent approval requests with inline diff/payload editing and decided log.
//!
//! Renders human-in-the-loop approval gates for an active agent run as a physical card stack:
//! the front card shows the requesting agent (`pro_agent_avatar`), exact diff or command payload,
//! reason, expandable run context, and `Edit` / `Reject` / `Approve` controls, while open gates
//! display downstream held steps and resolved gates drop smoothly into an expandable `Decided` list.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_rolling_number, pro_skeleton_bar,
        pro_spring_bar, pro_step_transition, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use web_time::Instant;

const STACK_MARK_SVG: &str = r#"<rect x="3.4" y="1.6" width="9.2" height="3" rx="1.2" fill="currentColor" opacity="0.3"></rect><rect x="2.2" y="3.6" width="11.6" height="3.4" rx="1.3" fill="currentColor" opacity="0.55"></rect><rect x="1" y="6.2" width="14" height="8.2" rx="2" fill="currentColor"></rect>"#;
const MINI_CROSS_SVG: &str = r#"<path d="M4.8 4.8 11.2 11.2M11.2 4.8 4.8 11.2" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"></path>"#;

/// Category of action gated by an [`ApprovalGate`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GateKind {
    /// CLI command, SQL query, or external API tool call.
    #[default]
    Tool,
    /// Source or configuration file patch with diff lines.
    File,
    /// Spawning or delegating to a sub-agent.
    Agent,
    /// Outbound customer email, Slack post, or webhook message.
    Message,
}

impl GateKind {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Tool => tr(cx, "Tool call", "工具调用"),
            Self::File => tr(cx, "File change", "文件变更"),
            Self::Agent => tr(cx, "Sub-agent", "子智能体"),
            Self::Message => tr(cx, "Outbound message", "外发消息"),
        }
    }
}

/// Resolution state of a single [`ApprovalGate`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GateStatus {
    /// Waiting in the pending deck and holding downstream steps.
    #[default]
    Open,
    /// Approved as proposed.
    Approved,
    /// Approved with inline payload or instruction edits from the user.
    Edited,
    /// Rejected by the user (stops downstream steps that depend on it).
    Rejected,
}

/// Decision action chosen by the user on an open [`ApprovalGate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision {
    /// Approved without modifications.
    Approved,
    /// Approved with modified input or change note.
    Edited,
    /// Rejected, halting dependent steps.
    Rejected,
}

/// Line classification inside a [`GateFileDiff`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GateDiffLineKind {
    /// Unchanged surrounding context line.
    #[default]
    Context,
    /// Added line (`+`).
    Add,
    /// Removed line (`-`).
    Del,
}

/// Single line inside a [`GateFileDiff`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateDiffLine {
    /// Whether the line is context, added, or removed.
    pub kind: GateDiffLineKind,
    /// Source code text for the line.
    pub text: SharedString,
}

impl GateDiffLine {
    /// Create a context line.
    pub fn context(text: impl Into<SharedString>) -> Self {
        Self {
            kind: GateDiffLineKind::Context,
            text: text.into(),
        }
    }

    /// Create an added (`+`) diff line.
    pub fn add(text: impl Into<SharedString>) -> Self {
        Self {
            kind: GateDiffLineKind::Add,
            text: text.into(),
        }
    }

    /// Create a deleted (`-`) diff line.
    pub fn del(text: impl Into<SharedString>) -> Self {
        Self {
            kind: GateDiffLineKind::Del,
            text: text.into(),
        }
    }
}

/// Per-file diff preview attached to an [`ApprovalGate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateFileDiff {
    /// Repository-relative file path (for example `"src/billing/pricing.ts"`).
    pub file: SharedString,
    /// Ordered diff lines.
    pub lines: Vec<GateDiffLine>,
}

impl GateFileDiff {
    /// Construct a [`GateFileDiff`] for `file`.
    pub fn new(file: impl Into<SharedString>, lines: Vec<GateDiffLine>) -> Self {
        Self {
            file: file.into(),
            lines,
        }
    }
}

/// Individual approval request in [`ApprovalGates`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalGate {
    /// Stable gate identifier.
    pub id: SharedString,
    /// Requesting agent name (drives [`pro_agent_avatar`]).
    pub agent: SharedString,
    /// Kind of gated operation (`Tool`, `File`, `Agent`, `Message`).
    pub kind: GateKind,
    /// Concise imperative description of what will happen.
    pub action: SharedString,
    /// One-line explanation of why human review is required.
    pub reason: SharedString,
    /// Current lifecycle status (`Open`, `Approved`, `Edited`, `Rejected`).
    pub status: GateStatus,
    /// Relative timestamp label (for example `"1m ago"`).
    pub at: SharedString,
    /// Optional command, arguments, or message body.
    pub input: Option<SharedString>,
    /// Optional structured file diffs.
    pub diff: Vec<GateFileDiff>,
    /// Preceding steps that led to this gate (shown under `Full context`).
    pub context: Vec<SharedString>,
    /// Downstream plan steps blocked behind this gate while `Open`.
    pub holds: Vec<SharedString>,
    /// Optional user note or edited payload recorded when decided.
    pub note: Option<SharedString>,
}

impl ApprovalGate {
    /// Construct a new open [`ApprovalGate`].
    pub fn new(
        id: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        kind: GateKind,
        action: impl Into<SharedString>,
        reason: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            agent: agent.into(),
            kind,
            action: action.into(),
            reason: reason.into(),
            status: GateStatus::Open,
            at: "just now".into(),
            input: None,
            diff: Vec::new(),
            context: Vec::new(),
            holds: Vec::new(),
            note: None,
        }
    }

    /// Set the lifecycle [`GateStatus`].
    pub fn with_status(mut self, status: GateStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the relative timestamp label.
    pub fn with_at(mut self, at: impl Into<SharedString>) -> Self {
        self.at = at.into();
        self
    }

    /// Attach a command, argument block, or message payload.
    pub fn with_input(mut self, input: impl Into<SharedString>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Attach file diffs for [`GateKind::File`] gates.
    pub fn with_diff(mut self, diff: Vec<GateFileDiff>) -> Self {
        self.diff = diff;
        self
    }

    /// Attach prior steps displayed under `Full context`.
    pub fn with_context<I, S>(mut self, steps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.context = steps.into_iter().map(Into::into).collect();
        self
    }

    /// Attach downstream steps held behind this gate while open.
    pub fn with_holds<I, S>(mut self, steps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.holds = steps.into_iter().map(Into::into).collect();
        self
    }
}

/// Events emitted by [`ApprovalGates`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalGatesEvent {
    /// User approved, edited-and-approved, or rejected a gate.
    Decided {
        /// Identifier of the resolved gate.
        id: SharedString,
        /// Chosen decision (`Approved`, `Edited`, `Rejected`).
        decision: GateDecision,
        /// Edited payload or change note when `decision == GateDecision::Edited`.
        value: Option<SharedString>,
    },
    /// User navigated to another open gate in the stacked deck (`index` into open gates).
    ActiveGateChanged(usize),
    /// User expanded or collapsed a decided gate row.
    DecidedRowToggled {
        /// Gate identifier.
        id: SharedString,
        /// Whether the row details are now expanded.
        open: bool,
    },
    /// User reset the showcase back to its initial gate queue.
    Reset,
}

impl EventEmitter<ApprovalGatesEvent> for ApprovalGates {}

fn default_gates() -> Vec<ApprovalGate> {
    vec![
        ApprovalGate::new(
            "migrate",
            "Coder",
            GateKind::Tool,
            "Run the database migration",
            "Adds billing_interval column to production subscriptions schema",
        )
        .with_status(GateStatus::Approved)
        .with_at("6m ago")
        .with_input("pnpm prisma migrate deploy --schema prisma/billing.prisma"),
        ApprovalGate::new(
            "prices",
            "Coder",
            GateKind::File,
            "Change prices in pricing.ts",
            "Changes what customers pay from Monday",
        )
        .with_at("1m ago")
        .with_diff(vec![GateFileDiff::new(
            "src/billing/pricing.ts",
            vec![
                GateDiffLine::context("export const PLANS = {"),
                GateDiffLine::del("  pro: { seat: 8, annualSeat: 7 },"),
                GateDiffLine::add("  pro: { seat: 12, annualSeat: 10 },"),
                GateDiffLine::del("  team: { seat: 16, sso: false },"),
                GateDiffLine::add("  team: { seat: 20, sso: true },"),
                GateDiffLine::context("} as const;"),
            ],
        )])
        .with_context([
            "Compared seat pricing across Linear ($10), Notion ($12), and Figma ($15)",
            "Verified Stripe live-mode price IDs in staging checkout",
        ])
        .with_holds(["Deploy to production", "Post to #launch"]),
        ApprovalGate::new(
            "email-trials",
            "Outreach",
            GateKind::Message,
            "Email 1,284 trial users",
            "Sends external plan announcement to active trial workspaces",
        )
        .with_at("just now")
        .with_input(
            "To: active-trials@segment.kense.dev (1,284 workspaces)\nSubject: New Pro & Team plans — existing trials keep $8/seat for 90 days",
        )
        .with_context([
            "Queried active trial cohorts created within the last 30 days",
            "Excluded 19 enterprise annual contracts",
        ])
        .with_holds(["Enable self-serve upgrade banner"]),
    ]
}

fn agent_accent(agent: &str, p: &ProPalette) -> Hsla {
    match agent.to_ascii_lowercase().as_str() {
        "coder" => p.blue,
        "outreach" | "writer" => p.purple,
        "ops" | "deployer" => p.teal,
        _ => p.orange,
    }
}

/// Stacked deck of pending agent approval requests with inline editing, held-step tracking, and decided history.
pub struct ApprovalGates {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    task: Option<SharedString>,
    live: bool,
    gates: Vec<ApprovalGate>,
    initial_gates: Vec<ApprovalGate>,
    open_cursor: usize,
    gate_step_serial: i32,
    context_open: bool,
    editing_gate: Option<SharedString>,
    payload_editor: Option<Entity<Editor>>,
    editor_seeded_for: Option<SharedString>,
    editor_focused: bool,
    expanded_decided: HashSet<SharedString>,
    decided_clocks: HashMap<SharedString, PhaseClock>,
    clear_clock: PhaseClock,
    deck_rise: Slide,
    reject_shake: PhaseClock,
    last_rejected: bool,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl ApprovalGates {
    /// Create an [`ApprovalGates`] stack preloaded with the pricing-launch run (2 open gates, 1 decided).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let initial = default_gates();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Approvals".into(),
            task: Some("Launch the new pricing page".into()),
            live: true,
            gates: initial.clone(),
            initial_gates: initial,
            open_cursor: 0,
            gate_step_serial: 0,
            context_open: false,
            editing_gate: None,
            payload_editor: None,
            editor_seeded_for: None,
            editor_focused: false,
            expanded_decided: HashSet::new(),
            decided_clocks: HashMap::new(),
            clear_clock: PhaseClock::default(),
            deck_rise: Slide::new(1.0),
            reject_shake: PhaseClock::default(),
            last_rejected: false,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(560.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the summary block heading (defaults to `"Approvals"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set or hide (`None`) the run task description next to the title.
    pub fn with_task(mut self, task: Option<SharedString>) -> Self {
        self.task = task;
        self
    }

    /// Set whether the agent run is still live when all open gates clear.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Provide a custom list of [`ApprovalGate`] items in chronological order.
    pub fn with_gates(mut self, gates: Vec<ApprovalGate>) -> Self {
        self.initial_gates = gates.clone();
        self.gates = gates;
        self.open_cursor = 0;
        self
    }

    /// Toggle loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Number of currently open gates waiting in the stack.
    pub fn open_count(&self) -> usize {
        self.gates
            .iter()
            .filter(|g| g.status == GateStatus::Open)
            .count()
    }

    /// Number of gates already decided (`Approved`, `Edited`, or `Rejected`).
    pub fn decided_count(&self) -> usize {
        self.gates
            .iter()
            .filter(|g| g.status != GateStatus::Open)
            .count()
    }

    /// Total downstream steps currently held behind open gates.
    pub fn held_step_count(&self) -> usize {
        self.gates
            .iter()
            .filter(|g| g.status == GateStatus::Open)
            .map(|g| g.holds.len())
            .sum()
    }

    fn open_indices(&self) -> Vec<usize> {
        self.gates
            .iter()
            .enumerate()
            .filter_map(|(i, g)| (g.status == GateStatus::Open).then_some(i))
            .collect()
    }

    fn clamped_cursor(&self, open_len: usize) -> usize {
        if open_len == 0 {
            0
        } else {
            self.open_cursor.min(open_len - 1)
        }
    }

    /// Decide the currently focused open gate (or a specific gate `id`) and advance the stack.
    pub fn decide(
        &mut self,
        id: &str,
        decision: GateDecision,
        value: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let Some(gate) = self
            .gates
            .iter_mut()
            .find(|g| g.id.as_ref() == id && g.status == GateStatus::Open)
        else {
            return;
        };

        gate.status = match decision {
            GateDecision::Approved => GateStatus::Approved,
            GateDecision::Edited => GateStatus::Edited,
            GateDecision::Rejected => GateStatus::Rejected,
        };
        if let Some(v) = &value {
            if gate.kind != GateKind::File {
                gate.input = Some(v.clone());
            }
            gate.note = Some(v.clone());
        }

        self.editing_gate = None;
        self.editor_seeded_for = None;
        self.context_open = false;
        self.gate_step_serial += 1;

        let mut dec_clk = PhaseClock::default();
        dec_clk.restart(cx);
        self.decided_clocks.insert(id.to_owned().into(), dec_clk);

        let new_open_len = self.open_count();
        if new_open_len == 0 {
            self.clear_clock.restart(cx);
        } else if self.open_cursor >= new_open_len {
            self.open_cursor = new_open_len - 1;
        }

        self.deck_rise.reset(0.0);
        self.deck_rise.duration(1.0, 0.30, cx);
        if decision == GateDecision::Rejected {
            self.last_rejected = true;
            self.reject_shake.restart(cx);
        } else {
            self.last_rejected = false;
        }

        let toast_txt: SharedString = match decision {
            GateDecision::Approved => tr(
                cx,
                "Gate approved · next card ready",
                "已批准 · 下一张待审卡就绪",
            )
            .into(),
            GateDecision::Edited => tr(cx, "Approved with your changes", "已按修改内容批准").into(),
            GateDecision::Rejected => tr(
                cx,
                "Gate rejected · dependent steps held",
                "已拒绝 · 下游步骤已中止",
            )
            .into(),
        };
        self.toast = Some((toast_txt, Instant::now()));

        cx.emit(ApprovalGatesEvent::Decided {
            id: id.to_owned().into(),
            decision,
            value,
        });
        cx.notify();
    }

    /// Reset all gates back to their initial configuration.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.gates = self.initial_gates.clone();
        self.open_cursor = 0;
        self.gate_step_serial += 1;
        self.editing_gate = None;
        self.editor_seeded_for = None;
        self.context_open = false;
        self.expanded_decided.clear();
        self.decided_clocks.clear();
        self.deck_rise.reset(0.0);
        self.deck_rise.duration(1.0, 0.28, cx);
        cx.emit(ApprovalGatesEvent::Reset);
        cx.notify();
    }

    fn step_open(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.open_count();
        if len <= 1 {
            return;
        }
        let cur = self.clamped_cursor(len) as isize;
        let next = (cur + delta).rem_euclid(len as isize) as usize;
        if next != self.open_cursor {
            self.open_cursor = next;
            self.gate_step_serial += if delta >= 0 { 1 } else { -1 };
            self.editing_gate = None;
            self.editor_seeded_for = None;
            self.context_open = false;
            self.deck_rise.reset(0.0);
            self.deck_rise.duration(1.0, 0.24, cx);
            cx.emit(ApprovalGatesEvent::ActiveGateChanged(next));
            cx.notify();
        }
    }

    fn ensure_editor(
        &mut self,
        seed_id: &str,
        seed_text: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ph: SharedString = tr(
            cx,
            "Edit arguments or describe what should change…",
            "编辑参数内容，或说明需要修改的地方…",
        )
        .into();
        if self.payload_editor.is_none() {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 12.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Escape => {
                    this.editing_gate = None;
                    this.editor_seeded_for = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditorShortcutEvent, w, cx| {
                let EditorShortcutEvent::CommandSubmit = ev;
                if let Some(gid) = this.editing_gate.clone() {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    let note = (!val.is_empty()).then(|| SharedString::from(val));
                    this.decide(gid.as_ref(), GateDecision::Edited, note, cx);
                    this.focus.focus(w, cx);
                }
            })
            .detach();
            self.payload_editor = Some(ed);
        }
        if self.editor_seeded_for.as_deref() != Some(seed_id)
            && let Some(ed) = &self.payload_editor
        {
            let text = seed_text.to_owned();
            ed.update(cx, |e, cx| e.set_value(text, w, cx));
            self.editor_seeded_for = Some(seed_id.to_owned().into());
        }
    }
}

impl Render for ApprovalGates {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);

        if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() * 0.5).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(24.0))
                .px(px(16.0))
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(560.0))
                        .flex()
                        .flex_col()
                        .gap(px(14.0))
                        .child(pro_skeleton_bar(560.0, 92.0, 12.0, phase, cx))
                        .child(pro_skeleton_bar(560.0, 280.0, 14.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.gates.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Shield,
                    tr(cx, "No approval gates in this run", "本次运行暂无审批关卡"),
                    tr(
                        cx,
                        "Gates appear automatically whenever an agent pauses for file, tool, or outbound message approval.",
                        "当智能体因文件改动、工具调用或对外消息暂停等待确认时，审批卡将自动堆叠于此。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let open_idxs = self.open_indices();
        let open_len = open_idxs.len();
        let cur_pos = self.clamped_cursor(open_len);
        let active_gate_idx = open_idxs.get(cur_pos).copied();

        if let Some(g_idx) = active_gate_idx {
            let gate_id = self.gates[g_idx].id.clone();
            if self.editing_gate.as_ref() == Some(&gate_id) {
                let seed = self.gates[g_idx]
                    .input
                    .as_deref()
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        if let Some(df) = self.gates[g_idx].diff.first() {
                            df.lines
                                .iter()
                                .filter(|l| l.kind != GateDiffLineKind::Del)
                                .map(|l| l.text.as_ref())
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            String::new()
                        }
                    });
                self.ensure_editor(gate_id.as_ref(), &seed, w, cx);
            }
        }

        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.0;
        let reduced = disabled_motion(cx);
        if !reduced && (open_len > 0 || self.live) {
            w.request_animation_frame();
        }

        let rise_t = if reduced {
            1.0
        } else {
            self.deck_rise.get(w, cx).clamp(0.0, 1.0)
        };
        let shake_dx = if self.last_rejected && !reduced {
            let age = self.reject_shake.time(0.28, w, cx);
            if age < 0.28 {
                let u = age / 0.28;
                (u * std::f32::consts::TAU * 2.0).sin() * 4.0 * (1.0 - u)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let cw = self.measured_width.get();
        let narrow = cw < 500.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let well_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.035)
        } else {
            hsla(0.0, 0.0, 0.0, 0.026)
        };

        let total_len = self.gates.len();
        let decided_len = self.decided_count();
        let held_steps = self.held_step_count();
        let any_rejected = self.gates.iter().any(|g| g.status == GateStatus::Rejected);

        // Top Summary Card
        let (pill_bg, pill_fg, pill_txt, pulse_dot) = if open_len > 0 {
            (
                p.amber_soft,
                p.amber_ink,
                tr(cx, "Waiting on you", "等待你审批"),
                true,
            )
        } else if any_rejected {
            (p.red_soft, p.red_ink, tr(cx, "Stopped", "已中止"), false)
        } else if self.live {
            (p.accent_soft, p.blue, tr(cx, "Running", "运行中"), true)
        } else {
            (
                p.green_soft,
                p.green_ink,
                tr(cx, "All clear", "全部通过"),
                false,
            )
        };

        let pulse_alpha = if pulse_dot && !reduced {
            0.45 + 0.55 * ((elapsed_ms / 1400.0) * std::f32::consts::TAU).cos().abs()
        } else {
            1.0
        };

        let title_str: SharedString = if self.title.as_ref() == "Approvals" {
            tr(cx, "Approvals", "运行审批关卡").into()
        } else {
            self.title.clone()
        };
        let task_str: Option<SharedString> = self.task.as_ref().map(|t| {
            if t.as_ref() == "Launch the new pricing page" {
                tr(cx, "Launch the new pricing page", "上线新版定价页面").into()
            } else {
                t.clone()
            }
        });

        let run_summary: SharedString = if open_len > 0 {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已暂停 — {held_steps} 个下游步骤等待 {open_len} 道关卡")
            } else {
                format!(
                    "Paused – {held_steps} step{} wait on {open_len} gate{}",
                    if held_steps == 1 { "" } else { "s" },
                    if open_len == 1 { "" } else { "s" }
                )
            }
            .into()
        } else if any_rejected {
            tr(cx, "Stopped at rejected gate", "已在被拒绝的关卡处停止").into()
        } else if self.live {
            tr(cx, "Running – no gates blocking", "运行中 — 当前无阻塞关卡").into()
        } else {
            tr(
                cx,
                "Completed – all gates decided",
                "已完成 — 全部关卡均已处理",
            )
            .into()
        };

        let prog_ratio = (decided_len as f32) / (total_len.max(1) as f32);
        let prog_fill = if any_rejected {
            p.red
        } else if open_len == 0 {
            p.green
        } else {
            p.blue
        };

        let summary_card = div()
            .w_full()
            .rounded(px(12.0))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .when(!p.dark, |d| d.shadow_xs())
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .px(px(12.0))
                    .py(px(9.0))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(pro_svg("0 0 16 16", STACK_MARK_SVG, 16.0, p.muted))
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str),
                    )
                    .when(!narrow, |d| {
                        d.when_some(task_str, |d, t| {
                            d.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.0))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(t),
                            )
                        })
                    })
                    .child(div().flex_1())
                    .when(decided_len > 1, |d| {
                        let replay_lbl = tr(cx, "Replay", "重置");
                        d.child(pro_btn(
                            pro_eid(&self.id, "top-reset"),
                            replay_lbl,
                            Some(ProIcon::Undo),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| {
                                this.reset(cx);
                            },
                        ))
                    })
                    .child(
                        div()
                            .h(px(20.0))
                            .px(px(7.0))
                            .flex_none()
                            .rounded(px(5.0))
                            .bg(pill_bg)
                            .text_color(pill_fg)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .when(pulse_dot, |d| {
                                d.child(
                                    div()
                                        .size(px(6.0))
                                        .rounded_full()
                                        .bg(pill_fg.opacity(pulse_alpha)),
                                )
                            })
                            .child(pro_swap_label(
                                pro_eid(&self.id, "sum-pill-lbl"),
                                pill_txt,
                                11.5,
                                FontWeight::MEDIUM,
                                pill_fg,
                            )),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .py(px(9.0))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .child(
                        div()
                            .w(px(if narrow { 120.0 } else { 132.0 }))
                            .flex_none()
                            .px(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "WAITING ON YOU", "待你处理")),
                            )
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "open-num"),
                                open_len as f64,
                                0,
                                "",
                                "",
                                13.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            )),
                    )
                    .child(
                        div()
                            .w(px(if narrow { 100.0 } else { 104.0 }))
                            .flex_none()
                            .px(px(12.0))
                            .border_l_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "DECIDED", "已决定")),
                            )
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "decided-num"),
                                decided_len as f64,
                                0,
                                "",
                                format!(" / {total_len}"),
                                13.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(180.0))
                            .px(px(12.0))
                            .when(!narrow, |d| d.border_l_1().border_color(p.line))
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "RUN", "运行状态")),
                            )
                            .child(div().w_full().overflow_hidden().child(pro_swap_label(
                                pro_eid(&self.id, "run-sum-lbl"),
                                run_summary,
                                13.0,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .px(px(12.0))
                    .pb(px(9.0))
                    .child(pro_spring_bar(
                        pro_eid(&self.id, "gates-prog-bar"),
                        prog_ratio,
                        3.0,
                        2.0,
                        p.chip,
                        prog_fill,
                    )),
            );

        // Stacked Open Deck + Holds/Pager Footer
        let stack_section = if let Some(g_idx) = active_gate_idx {
            let active = self.gates[g_idx].clone();
            let peek = 11.0_f32;
            let back_count = open_len.saturating_sub(1).min(2);
            let top_pad = peek * 2.0;

            let mut pile = div().relative().w_full().pt(px(top_pad)).left(px(shake_dx));

            // Render up to 2 peeking cards behind the active card (from backmost k=2 to k=1)
            for k in (1..=back_count).rev() {
                let next_open_pos = (cur_pos + k) % open_len;
                let behind_gate = &self.gates[open_idxs[next_open_pos]];
                let behind_action = behind_gate.action.clone();
                let inset_x = (k as f32) * 14.0;
                let lift_y = top_pad - (k as f32) * peek;
                let alpha = (1.0 - (k as f32) * 0.22).clamp(0.4, 0.9);
                pile = pile.child(
                    div()
                        .id(pro_eid(&self.id, format!("peek-{k}")))
                        .role(Role::Button)
                        .aria_label(behind_action.clone())
                        .tab_index(0)
                        .absolute()
                        .top(px(lift_y))
                        .left(px(inset_x))
                        .right(px(inset_x))
                        .h(px(64.0))
                        .rounded(px(14.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .opacity(alpha)
                        .cursor_pointer()
                        .hover(|s| s.opacity(1.0))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.open_cursor = next_open_pos;
                            this.gate_step_serial += 1;
                            this.editing_gate = None;
                            this.editor_seeded_for = None;
                            this.context_open = false;
                            this.deck_rise.reset(0.0);
                            this.deck_rise.duration(1.0, 0.24, cx);
                            cx.emit(ApprovalGatesEvent::ActiveGateChanged(next_open_pos));
                            cx.notify();
                        }))
                        .px(px(14.0))
                        .pt(px(2.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(p.muted)
                                .truncate()
                                .child(behind_action),
                        ),
                );
            }

            // Active Top Card
            let is_editing = self.editing_gate.as_ref() == Some(&active.id);
            let avatar_el = pro_agent_avatar(
                active.agent.as_ref(),
                agent_accent(active.agent.as_ref(), &p),
                20.0,
                true,
                elapsed_ms,
                cx,
            );
            let who_suffix = format!(" · {} · {}", active.kind.label(cx), active.at);

            let action_localized: SharedString = match active.action.as_ref() {
                "Change prices in pricing.ts" => tr(
                    cx,
                    "Change prices in pricing.ts",
                    "修改 pricing.ts 中的套餐单价",
                )
                .into(),
                "Email 1,284 trial users" => tr(
                    cx,
                    "Email 1,284 trial users",
                    "向 1,284 个试用工作区发送通知邮件",
                )
                .into(),
                _ => active.action.clone(),
            };
            let reason_localized: SharedString = match active.reason.as_ref() {
                "Changes what customers pay from Monday" => tr(
                    cx,
                    "Changes what customers pay from Monday",
                    "将直接变更周一零点生效的客户订阅结算价格",
                )
                .into(),
                "Sends external plan announcement to active trial workspaces" => tr(
                    cx,
                    "Sends external plan announcement to active trial workspaces",
                    "向外部活跃试用客户批量发送套餐调整公告",
                )
                .into(),
                _ => active.reason.clone(),
            };

            let edit_box = {
                let ed_view = self.payload_editor.clone();
                let edit_hint = tr(
                    cx,
                    "EDIT BEFORE APPROVING  ·  ⌘↵ TO APPROVE",
                    "批准前修改参数或指令  ·  按 ⌘↵ 确认",
                );
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.blue)
                            .child(edit_hint),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(82.0))
                            .p(px(9.0))
                            .rounded(px(8.0))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.focus)
                            .when_some(ed_view, |d, ed| d.child(ed)),
                    )
                    .into_any_element()
            };

            let readonly_box = if !active.diff.is_empty() {
                let mut diff_box = div()
                    .id(pro_eid(&self.id, "active-diff"))
                    .w_full()
                    .max_h(px(138.0))
                    .overflow_y_scroll()
                    .py(px(7.0))
                    .rounded(px(8.0))
                    .bg(well_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(6.0));

                for file_diff in &active.diff {
                    let mut lines_col = div().w_full().flex().flex_col();
                    for dl in &file_diff.lines {
                        let (row_bg, row_fg, sigil) = match dl.kind {
                            GateDiffLineKind::Context => (transparent_black(), p.text, " "),
                            GateDiffLineKind::Add => (
                                p.green.opacity(if p.dark { 0.12 } else { 0.10 }),
                                p.green_ink,
                                "+",
                            ),
                            GateDiffLineKind::Del => (
                                p.red.opacity(if p.dark { 0.12 } else { 0.09 }),
                                p.red_ink,
                                "−",
                            ),
                        };
                        lines_col = lines_col.child(
                            div()
                                .w_full()
                                .px(px(10.0))
                                .py(px(1.0))
                                .bg(row_bg)
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .font_family("Menlo")
                                .text_size(px(11.5))
                                .line_height(px(18.0))
                                .text_color(row_fg)
                                .child(div().w(px(10.0)).flex_none().child(sigil))
                                .child(div().flex_1().min_w_0().truncate().child(dl.text.clone())),
                        );
                    }
                    diff_box = diff_box.child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .px(px(10.0))
                                    .font_family("Menlo")
                                    .text_size(px(11.0))
                                    .text_color(p.muted)
                                    .child(file_diff.file.clone()),
                            )
                            .child(lines_col),
                    );
                }
                Some(diff_box.into_any_element())
            } else {
                active.input.as_ref().map(|inp| {
                    div()
                        .id(pro_eid(&self.id, "active-inp"))
                        .w_full()
                        .max_h(px(128.0))
                        .overflow_y_scroll()
                        .px(px(10.0))
                        .py(px(8.0))
                        .rounded(px(8.0))
                        .bg(well_bg)
                        .border_1()
                        .border_color(p.line)
                        .font_family("Menlo")
                        .text_size(px(11.5))
                        .line_height(px(18.0))
                        .text_color(p.ink)
                        .child(inp.clone())
                        .into_any_element()
                })
            };

            let payload_el = div()
                .w_full()
                .flex()
                .flex_col()
                .child(pro_fold(
                    pro_eid(&self.id, "gate-edit-fold"),
                    is_editing,
                    104.0,
                    0.0,
                    move || edit_box,
                ))
                .when_some(readonly_box, |d, ro| {
                    d.child(pro_fold(
                        pro_eid(&self.id, "gate-ro-fold"),
                        !is_editing,
                        116.0,
                        0.0,
                        move || ro,
                    ))
                })
                .into_any_element();

            // Full context fold
            let ctx_steps = active.context.clone();
            let has_ctx = !ctx_steps.is_empty();
            let ctx_open = self.context_open;
            let context_section = has_ctx.then(|| {
                let ctx_lbl = tr(cx, "Full context", "完整上下文");
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id(pro_eid(&self.id, "ctx-btn"))
                            .role(Role::Button)
                            .aria_label(ctx_lbl)
                            .aria_expanded(ctx_open)
                            .tab_index(0)
                            .w(px(116.0))
                            .h(px(22.0))
                            .px(px(6.0))
                            .ml(px(-6.0))
                            .rounded(px(5.0))
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover).text_color(p.ink))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.context_open = !this.context_open;
                                cx.notify();
                            }))
                            .child(ctx_lbl)
                            .child(pro_icon(
                                if ctx_open {
                                    ProIcon::ChevronUp
                                } else {
                                    ProIcon::ChevronDown
                                },
                                12.0,
                                p.muted,
                            )),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, "ctx-fold"),
                        ctx_open,
                        56.0,
                        0.0,
                        move || {
                            let mut ol = div().pt(px(6.0)).flex().flex_col().gap(px(4.0));
                            for (idx, step_txt) in ctx_steps.iter().enumerate() {
                                ol = ol.child(
                                    div()
                                        .flex()
                                        .items_start()
                                        .gap(px(7.0))
                                        .text_size(px(12.0))
                                        .line_height(px(17.0))
                                        .text_color(p.text)
                                        .child(
                                            div()
                                                .w(px(14.0))
                                                .flex_none()
                                                .text_size(px(11.0))
                                                .text_color(p.faint)
                                                .child(format!("{}.", idx + 1)),
                                        )
                                        .child(div().flex_1().min_w_0().child(step_txt.clone())),
                                );
                            }
                            ol.into_any_element()
                        },
                    ))
            });

            let gid_edit = active.id.clone();
            let gid_rej = active.id.clone();
            let gid_app = active.id.clone();
            let edit_lbl = if is_editing {
                tr(cx, "Cancel", "取消编辑")
            } else {
                tr(cx, "Edit", "修改后批准")
            };
            let reject_lbl = tr(cx, "Reject", "拒绝");
            let approve_lbl = if is_editing {
                tr(cx, "Approve changes", "按修改批准")
            } else {
                tr(cx, "Approve", "批准")
            };

            let dy_rise = -10.0 * (1.0 - pro_ease(rise_t));
            let alpha_rise = 0.65 + 0.35 * pro_ease(rise_t);

            let stepped_body = pro_step_transition(
                pro_eid(&self.id, "gate-step-tx"),
                self.gate_step_serial,
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(avatar_el)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.0))
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.text)
                                            .child(active.agent.clone()),
                                    )
                                    .child(div().text_color(p.muted).truncate().child(who_suffix)),
                            )
                            .child(
                                div()
                                    .h(px(20.0))
                                    .px(px(7.0))
                                    .flex_none()
                                    .rounded(px(5.0))
                                    .bg(p.amber_soft)
                                    .text_color(p.amber_ink)
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Blocking", "阻塞中")),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .line_height(px(22.0))
                                    .text_color(p.strong)
                                    .child(action_localized),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(18.0))
                                    .text_color(p.text)
                                    .child(reason_localized),
                            ),
                    )
                    .child(payload_el)
                    .when_some(context_section, |d, cs| d.child(cs)),
            );

            let front_card = div()
                .relative()
                .top(px(dy_rise))
                .opacity(alpha_rise)
                .w_full()
                .min_h(px(262.0))
                .p(px(if narrow { 13.0 } else { 15.0 }))
                .rounded(px(14.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(stepped_body)
                .child(div().flex_1())
                .child(
                    div()
                        .w_full()
                        .pt(px(4.0))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(6.0))
                        .child(pro_btn(
                            pro_eid(&self.id, "edit-gate-btn"),
                            edit_lbl,
                            Some(ProIcon::NewChat),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            move |this, w, cx| {
                                if this.editing_gate.as_ref() == Some(&gid_edit) {
                                    this.editing_gate = None;
                                    this.editor_seeded_for = None;
                                } else {
                                    this.editing_gate = Some(gid_edit.clone());
                                    this.editor_seeded_for = None;
                                }
                                cx.notify();
                                w.refresh();
                            },
                        ))
                        .child(div().flex_1())
                        .child(pro_btn(
                            pro_eid(&self.id, "reject-gate-btn"),
                            reject_lbl,
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                let note = if this.editing_gate.as_ref() == Some(&gid_rej)
                                    && let Some(ed) = &this.payload_editor
                                {
                                    let v = ed.read(cx).value().trim().to_owned();
                                    (!v.is_empty()).then(|| SharedString::from(v))
                                } else {
                                    None
                                };
                                this.decide(gid_rej.as_ref(), GateDecision::Rejected, note, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "approve-gate-btn"),
                            approve_lbl,
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                if this.editing_gate.as_ref() == Some(&gid_app) {
                                    let val = this
                                        .payload_editor
                                        .as_ref()
                                        .map(|ed| ed.read(cx).value().trim().to_owned())
                                        .unwrap_or_default();
                                    let note = (!val.is_empty()).then(|| SharedString::from(val));
                                    this.decide(gid_app.as_ref(), GateDecision::Edited, note, cx);
                                } else {
                                    this.decide(gid_app.as_ref(), GateDecision::Approved, None, cx);
                                }
                            },
                        )),
                );

            pile = pile.child(front_card);

            // Below-stack Holds + Pager row
            let holds_el = (!active.holds.is_empty()).then(|| {
                let n = active.holds.len();
                let prefix = if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("阻塞 {n} 个下游步骤：")
                } else {
                    format!("Holds {n} step{}: ", if n == 1 { "" } else { "s" })
                };
                let joined = active
                    .holds
                    .iter()
                    .map(|s| s.as_ref())
                    .collect::<Vec<_>>()
                    .join(", ");
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .overflow_hidden()
                    .child(pro_icon(ProIcon::Lock, 12.0, p.amber_ink))
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.amber_ink)
                            .child(prefix),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(p.muted)
                            .truncate()
                            .child(joined),
                    )
            });

            let pager_el = (open_len > 1).then(|| {
                let prev_lbl = tr(cx, "Previous request", "上一项请求");
                let next_lbl = tr(cx, "Next request", "下一项请求");
                let page_txt: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("{} / {}", cur_pos + 1, open_len).into()
                } else {
                    format!("{} of {}", cur_pos + 1, open_len).into()
                };
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "prev-gate"),
                        prev_lbl,
                        ProIcon::ChevronLeft,
                        24.0,
                        false,
                        cx,
                        |this, _, cx| {
                            this.step_open(-1, cx);
                        },
                    ))
                    .child(
                        div()
                            .min_w(px(44.0))
                            .flex()
                            .justify_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, "gate-page-lbl"),
                                page_txt,
                                12.0,
                                FontWeight::NORMAL,
                                p.muted,
                            )),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "next-gate"),
                        next_lbl,
                        ProIcon::ChevronRight,
                        24.0,
                        false,
                        cx,
                        |this, _, cx| {
                            this.step_open(1, cx);
                        },
                    ))
            });

            Some(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(pile)
                    .child(
                        div()
                            .w_full()
                            .min_h(px(34.0))
                            .pt(px(10.0))
                            .px(px(2.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.0))
                            .child(holds_el.unwrap_or_else(|| div().flex_1()))
                            .when_some(pager_el, |d, pg| d.child(pg)),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let clear_age = self.clear_clock.time(0.28, w, cx);
        let clear_chk_on = !any_rejected && (reduced || clear_age > 0.012);
        let cleared_card = {
            let (mark_bg, mark_fg, head_txt, sub_txt) = if any_rejected {
                (
                    p.red_soft,
                    p.red_ink,
                    tr(cx, "Run stopped at rejected gate", "运行已在拒绝关卡处停止"),
                    tr(
                        cx,
                        "Downstream steps held behind the rejected gate were not executed.",
                        "受拒绝关卡保护的下游步骤均未执行。",
                    ),
                )
            } else {
                (
                    p.green_soft,
                    p.green_ink,
                    tr(cx, "All approval gates cleared", "所有审批关卡均已放行"),
                    tr(
                        cx,
                        "Downstream deployment and launch steps are unblocked.",
                        "下游部署与发布步骤已解除阻塞并继续执行。",
                    ),
                )
            };
            let reset_lbl = tr(cx, "Replay stack", "重新演示审批栈");
            div()
                .w_full()
                .py(px(22.0))
                .px(px(16.0))
                .rounded(px(14.0))
                .bg(well_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(5.0))
                .child(if any_rejected {
                    div()
                        .size(px(28.0))
                        .rounded_full()
                        .bg(mark_bg)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 16 16", MINI_CROSS_SVG, 12.0, mark_fg))
                        .into_any_element()
                } else {
                    pro_check_draw(
                        pro_eid(&self.id, "all-clear-chk"),
                        clear_chk_on,
                        28.0,
                        false,
                        p.green,
                    )
                })
                .child(
                    div()
                        .text_size(px(13.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(head_txt),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(p.muted)
                        .text_center()
                        .child(sub_txt),
                )
                .child(div().mt(px(6.0)).child(pro_btn(
                    pro_eid(&self.id, "empty-replay"),
                    reset_lbl,
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.reset(cx);
                    },
                )))
                .into_any_element()
        };

        let stack_section = div()
            .w_full()
            .flex()
            .flex_col()
            .child(pro_fold(
                pro_eid(&self.id, "open-deck-fold"),
                open_len > 0,
                310.0,
                0.0,
                move || stack_section.unwrap_or_else(|| div().into_any_element()),
            ))
            .child(pro_fold(
                pro_eid(&self.id, "cleared-deck-fold"),
                open_len == 0,
                138.0,
                0.0,
                move || cleared_card,
            ));

        // Decided List Below (each row folds open smoothly and draws progressive `pro_check_draw`)
        let all_gates = self.gates.clone();
        let decided_section = (decided_len > 0).then(|| {
            let mut rows = div().w_full().flex().flex_col().gap(px(4.0));
            for dg in all_gates {
                let is_decided = dg.status != GateStatus::Open;
                let row_open = is_decided && self.expanded_decided.contains(&dg.id);
                let is_rej = dg.status == GateStatus::Rejected;
                let dec_age = self
                    .decided_clocks
                    .get(&dg.id)
                    .map_or(f32::INFINITY, |clk| clk.time(0.28, w, cx));
                let chk_on = is_decided && !is_rej && (reduced || dec_age > 0.012);
                let chk_color = if dg.status == GateStatus::Edited {
                    p.blue
                } else {
                    p.green
                };
                let outcome_el = match dg.status {
                    GateStatus::Approved => div()
                        .text_size(px(11.5))
                        .text_color(p.text)
                        .child(tr(cx, "Approved", "已批准"))
                        .into_any_element(),
                    GateStatus::Edited => div()
                        .h(px(18.0))
                        .px(px(6.0))
                        .rounded(px(5.0))
                        .bg(p.accent_soft)
                        .text_color(p.blue)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Edited", "已修改批准"))
                        .into_any_element(),
                    GateStatus::Rejected | GateStatus::Open => div()
                        .h(px(18.0))
                        .px(px(6.0))
                        .rounded(px(5.0))
                        .bg(p.red_soft)
                        .text_color(p.red_ink)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Stopped the run", "已中止运行"))
                        .into_any_element(),
                };

                let dg_id = dg.id.clone();
                let dg_reason = dg.reason.clone();
                let dg_note = dg.note.clone();
                let dg_input = dg.input.clone();

                let act_txt: SharedString = match dg.action.as_ref() {
                    "Run the database migration" => {
                        tr(cx, "Run the database migration", "执行数据库结构迁移").into()
                    }
                    "Change prices in pricing.ts" => tr(
                        cx,
                        "Change prices in pricing.ts",
                        "修改 pricing.ts 中的套餐单价",
                    )
                    .into(),
                    "Email 1,284 trial users" => tr(
                        cx,
                        "Email 1,284 trial users",
                        "向 1,284 个试用工作区发送通知邮件",
                    )
                    .into(),
                    _ => dg.action.clone(),
                };

                let row_header_content = div()
                    .w_full()
                    .h(px(34.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(if is_rej {
                        div()
                            .size(px(16.0))
                            .flex_none()
                            .rounded_full()
                            .bg(p.red_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 16 16", MINI_CROSS_SVG, 9.5, p.red_ink))
                            .into_any_element()
                    } else {
                        pro_check_draw(
                            pro_eid(&self.id, format!("dec-chk-{}", dg.id)),
                            chk_on,
                            16.0,
                            false,
                            chk_color,
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(act_txt.clone()),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(pro_agent_avatar(
                                    dg.agent.as_ref(),
                                    agent_accent(dg.agent.as_ref(), &p),
                                    15.0,
                                    false,
                                    0.0,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .child(dg.agent.clone()),
                                ),
                        )
                    })
                    .child(outcome_el)
                    .child(pro_icon(
                        if row_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        12.0,
                        p.faint,
                    ));

                let header_tactile = pro_tactile(
                    pro_eid(&self.id, format!("dec-row-{}", dg.id)),
                    act_txt,
                    9.0,
                    transparent_black(),
                    well_bg,
                    transparent_black(),
                    transparent_black(),
                    row_open,
                    row_header_content,
                    cx,
                    move |this, _, cx| {
                        let now_open = if this.expanded_decided.contains(&dg_id) {
                            this.expanded_decided.remove(&dg_id);
                            false
                        } else {
                            this.expanded_decided.insert(dg_id.clone());
                            true
                        };
                        cx.emit(ApprovalGatesEvent::DecidedRowToggled {
                            id: dg_id.clone(),
                            open: now_open,
                        });
                        cx.notify();
                    },
                )
                .w_full();

                let card_item = div()
                    .w_full()
                    .rounded(px(9.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(header_tactile)
                    .child(pro_fold(
                        pro_eid(&self.id, format!("dec-fold-{}", dg.id)),
                        row_open,
                        64.0,
                        0.0,
                        move || {
                            div()
                                .w_full()
                                .pl(px(32.0))
                                .pr(px(10.0))
                                .pb(px(10.0))
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.text)
                                        .child(dg_reason),
                                )
                                .when_some(dg_input, |d, inp| {
                                    d.child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(6.0))
                                            .rounded(px(6.0))
                                            .bg(well_bg)
                                            .font_family("Menlo")
                                            .text_size(px(11.0))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(inp),
                                    )
                                })
                                .when_some(dg_note, |d, note| {
                                    d.child(
                                        div()
                                            .px(px(8.0))
                                            .py(px(6.0))
                                            .rounded(px(6.0))
                                            .bg(if is_rej { p.red_soft } else { p.accent_soft })
                                            .text_size(px(12.0))
                                            .text_color(if is_rej { p.red_ink } else { p.blue })
                                            .child(note),
                                    )
                                })
                                .into_any_element()
                        },
                    ));

                rows = rows.child(pro_fold(
                    pro_eid(&self.id, format!("dec-entry-{}", dg.id)),
                    is_decided,
                    38.0,
                    4.0,
                    move || card_item.into_any_element(),
                ));
            }

            div()
                .w_full()
                .mt(px(6.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(tr(cx, "DECIDED", "已决定记录")),
                )
                .child(rows)
        });

        let width_sensor = self.measured_width.clone();
        let a11y_group = tr(
            cx,
            "Stacked agent approval gates and decided history",
            "智能体运行待审批卡堆叠与已决定记录",
        );
        let focus_ring = p.focus;

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(a11y_group)
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .w_full()
            .py(px(22.0))
            .px(px(16.0))
            .flex()
            .flex_col()
            .items_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.0)).border_color(focus_ring))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if this.editor_focused {
                    return;
                }
                match ev.keystroke.key.as_str() {
                    "left" => {
                        this.step_open(-1, cx);
                        cx.stop_propagation();
                    }
                    "right" => {
                        this.step_open(1, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(560.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            let w_px = f32::from(b.size.width);
                            if w_px > 20.0 {
                                width_sensor.set(w_px);
                            }
                        }
                    })
                    .child(summary_card)
                    .child(stack_section)
                    .when_some(decided_section, |d, ds| d.child(ds)),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{ApprovalGate, ApprovalGates, GateDecision, GateKind, GateStatus};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn approval_gates_stack_advances_and_tracks_held_steps(cx: &mut TestAppContext) {
        let gates_view = cx.new(|cx| ApprovalGates::new("test-approval-gates", cx));

        gates_view.update(cx, |v, cx| {
            assert_eq!(v.open_count(), 2);
            assert_eq!(v.decided_count(), 1);
            assert_eq!(v.held_step_count(), 3);

            v.decide(
                "prices",
                GateDecision::Edited,
                Some("Set Team to $22/seat".into()),
                cx,
            );
            assert_eq!(v.open_count(), 1);
            assert_eq!(v.decided_count(), 2);
            assert_eq!(v.held_step_count(), 1);

            v.decide("email-trials", GateDecision::Rejected, None, cx);
            assert_eq!(v.open_count(), 0);
            assert_eq!(v.decided_count(), 3);
            assert_eq!(v.held_step_count(), 0);

            v.reset(cx);
            assert_eq!(v.open_count(), 2);
        });

        let custom = ApprovalGate::new(
            "deploy",
            "Ops",
            GateKind::Tool,
            "Deploy release",
            "Touches prod",
        )
        .with_status(GateStatus::Open);
        assert_eq!(custom.status, GateStatus::Open);
    }
}
