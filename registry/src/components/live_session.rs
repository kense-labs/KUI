//! Real-time multi-agent collaboration block with conversation stream, live plan, and shared context.
//!
//! Pairs a multi-agent conversation stream and steer composer on the left with the team's
//! step-by-step execution plan and shared token/file context window on the right; selecting any
//! agent chip narrows and highlights both its messages and its assigned plan steps at once.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_mix_color,
        pro_modal_appear, pro_person_avatar, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_shimmer_text, pro_skeleton_bar,
        pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_toast_banner,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const ACTIVE_ARC_SVG: &str = r#"<circle cx="8" cy="8" r="5.8" fill="none" stroke="currentColor" stroke-opacity="0.22" stroke-width="1.6"></circle><path d="M8 2.2a5.8 5.8 0 0 1 5.8 5.8" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"></path>"#;
const PENDING_RING_SVG: &str = r#"<circle cx="8" cy="8" r="5.8" fill="none" stroke="currentColor" stroke-width="1.2" stroke-dasharray="2 2.2"></circle>"#;
const FILE_SHEET_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path>"#;

/// Execution state of the team plan in [`LiveSessionBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LiveSessionStatus {
    /// Agents are actively executing steps and streaming messages.
    #[default]
    Running,
    /// Session execution is paused by the user.
    Paused,
    /// Every step in the session plan has completed.
    Done,
}

/// Progress state of an individual [`SessionStep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionStepStatus {
    /// Queued for a later turn.
    #[default]
    Pending,
    /// Currently executing.
    Active,
    /// Completed.
    Done,
}

/// Kind of entry in the [`LiveSessionBlock`] conversation stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamMessageKind {
    /// Rich message card authored by an agent or `"you"`.
    #[default]
    Chat,
    /// Compact handoff or orchestration notice.
    System,
}

/// Participant agent metadata in [`LiveSessionBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamAgent {
    /// Stable agent identifier (such as `"orchestrator"`, `"research"`, or `"writer"`).
    pub id: SharedString,
    /// Display name shown in chips, message headers, and `@` mentions.
    pub name: SharedString,
    /// Model label (for example `"Opus 5.5"` or `"Sonnet 5.5"`).
    pub model: SharedString,
    /// Whether this agent is actively working (drives pixel-mark wave animation).
    pub busy: bool,
}

impl StreamAgent {
    /// Construct a [`StreamAgent`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: "Sonnet 5.5".into(),
            busy: true,
        }
    }

    /// Set the model badge shown in `@` mention menus and tooltips.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = model.into();
        self
    }

    /// Set whether this agent's pixel avatar twinkles as actively working.
    pub fn with_busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
}

/// Shared artifact or file chip attached to a [`StreamMessage`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamAttachment {
    /// Stable file identifier.
    pub id: SharedString,
    /// Filename (for example `"pricing-matrix.csv"`).
    pub name: SharedString,
    /// Short descriptor (for example `"CSV · 4 rows"`).
    pub meta: SharedString,
    /// Preview body displayed when the file chip is opened.
    pub preview: SharedString,
}

impl StreamAttachment {
    /// Construct a file attachment chip.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        preview: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            meta: meta.into(),
            preview: preview.into(),
        }
    }
}

/// Message or handoff entry in the [`LiveSessionBlock`] conversation stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamMessage {
    /// Stable message identifier.
    pub id: SharedString,
    /// Author agent id (`"you"` for user messages).
    pub from: SharedString,
    /// Optional recipient agent id (`None` means everyone).
    pub to: Option<SharedString>,
    /// Chat card vs compact system handoff row.
    pub kind: StreamMessageKind,
    /// Message text body.
    pub text: SharedString,
    /// Timestamp label (for example `"14:04"`).
    pub at: SharedString,
    /// Optional `(author_name, snippet)` quote this message replies to.
    pub reply_to: Option<(SharedString, SharedString)>,
    /// Optional tool invocation pill (for example `"web_search ×6"`).
    pub tool_badge: Option<SharedString>,
    /// Whether this message asks for immediate user input.
    pub needs_you: bool,
    /// Connected app names mentioned by this turn (rendered as compact brand tiles).
    pub apps: Vec<SharedString>,
    /// Attached shared artifacts or files.
    pub attachments: Vec<StreamAttachment>,
}

impl StreamMessage {
    /// Return true when this message involves `agent_id` as sender, recipient, or handoff subject.
    pub fn involves_agent(&self, agent_id: &str) -> bool {
        self.from.as_ref() == agent_id
            || self.to.as_deref() == Some(agent_id)
            || (self.kind == StreamMessageKind::System
                && self
                    .text
                    .to_ascii_lowercase()
                    .contains(&agent_id.to_ascii_lowercase()))
    }
}

/// Step in the right-hand team plan of [`LiveSessionBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Step title.
    pub title: SharedString,
    /// Assigned agent id (`"you"` for human-in-the-loop decisions).
    pub agent: SharedString,
    /// Execution status (`Done`, `Active`, `Pending`).
    pub status: SessionStepStatus,
    /// Optional duration or elapsed label (for example `"1m 24s"`).
    pub duration_label: Option<SharedString>,
    /// Optional steer note attached to this step.
    pub note: Option<SharedString>,
}

impl SessionStep {
    /// Construct a [`SessionStep`] assigned to `agent`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        status: SessionStepStatus,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            agent: agent.into(),
            status,
            duration_label: None,
            note: None,
        }
    }

    /// Attach a duration readout (`"1m 24s"`).
    pub fn with_duration(mut self, label: impl Into<SharedString>) -> Self {
        self.duration_label = Some(label.into());
        self
    }

    /// Attach an inline steer note below the step title.
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Token slice in the shared context window card of [`LiveSessionBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPart {
    /// Part identifier (`"system"`, `"talk"`, `"files"`, `"plan"`).
    pub id: SharedString,
    /// Human-readable category label.
    pub label: SharedString,
    /// Token count consumed by this slice.
    pub tokens: u32,
    /// Optional shared file names belonging to this slice.
    pub files: Vec<(SharedString, u32)>,
}

/// Shared context specification for [`LiveSessionBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionContextSpec {
    /// Model name displayed in the context card header (such as `"Opus 5.5"`).
    pub model: SharedString,
    /// Context window capacity in tokens (for example `200_000`).
    pub limit: u32,
    /// Ordered token breakdown parts.
    pub parts: Vec<ContextPart>,
}

impl SessionContextSpec {
    /// Total tokens used across all parts.
    pub fn used_tokens(&self) -> u32 {
        self.parts.iter().map(|p| p.tokens).sum()
    }

    /// Utilization ratio in `0.0..=1.0`.
    pub fn utilization(&self) -> f32 {
        if self.limit == 0 {
            0.0
        } else {
            (self.used_tokens() as f32 / self.limit as f32).clamp(0.0, 1.0)
        }
    }
}

/// Events emitted by [`LiveSessionBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveSessionEvent {
    /// User picked an agent chip (`Some(id)`) or returned to `"Everyone"` (`None`).
    FocusAgentChanged(Option<SharedString>),
    /// User sent a message or steer note from the composer.
    MessageSent {
        /// Submitted message body.
        text: SharedString,
        /// Optional `@` target agent id (`None` means everyone).
        to: Option<SharedString>,
        /// Optional message id being replied to.
        reply_to: Option<SharedString>,
    },
    /// User paused or resumed the live session plan.
    StatusChanged(LiveSessionStatus),
    /// User toggled or completed a step in the team plan.
    StepToggled {
        /// Step identifier.
        step_id: SharedString,
        /// Whether the step is now marked done.
        done: bool,
    },
    /// User opened a shared file preview.
    FileOpened(SharedString),
    /// User compacted the shared context window.
    ContextCompacted,
}

fn default_agents() -> Vec<StreamAgent> {
    vec![
        StreamAgent::new("orchestrator", "Orchestrator")
            .with_model("Opus 5.5")
            .with_busy(true),
        StreamAgent::new("research", "Researcher")
            .with_model("Sonnet 5.5")
            .with_busy(true),
        StreamAgent::new("writer", "Writer")
            .with_model("Opus 5.5")
            .with_busy(true),
    ]
}

fn default_messages() -> Vec<StreamMessage> {
    vec![
        StreamMessage {
            id: "m1".into(),
            from: "you".into(),
            to: Some("orchestrator".into()),
            kind: StreamMessageKind::Chat,
            text: "Launch the new pricing page by Friday – per-seat plans, keep a free tier.".into(),
            at: "14:02".into(),
            reply_to: None,
            tool_badge: None,
            needs_you: false,
            apps: Vec::new(),
            attachments: Vec::new(),
        },
        StreamMessage {
            id: "m2".into(),
            from: "orchestrator".into(),
            to: Some("research".into()),
            kind: StreamMessageKind::System,
            text: "Orchestrator started Researcher · Compare Linear, Notion and Figma seat pricing"
                .into(),
            at: "14:02".into(),
            reply_to: None,
            tool_badge: None,
            needs_you: false,
            apps: Vec::new(),
            attachments: Vec::new(),
        },
        StreamMessage {
            id: "m3".into(),
            from: "orchestrator".into(),
            to: Some("writer".into()),
            kind: StreamMessageKind::System,
            text: "Orchestrator started Writer · Draft short copy for Free, Pro and Team".into(),
            at: "14:03".into(),
            reply_to: None,
            tool_badge: None,
            needs_you: false,
            apps: Vec::new(),
            attachments: Vec::new(),
        },
        StreamMessage {
            id: "m4".into(),
            from: "research".into(),
            to: Some("orchestrator".into()),
            kind: StreamMessageKind::Chat,
            text: "Linear $10, Notion $12, Figma $15 per seat. All three keep an unlimited-seat free tier with usage caps.".into(),
            at: "14:04".into(),
            reply_to: Some((
                "Orchestrator".into(),
                "Find out what Linear, Notion and Figma charge per seat, and what their free tiers allow.".into(),
            )),
            tool_badge: Some("web_search ×6".into()),
            needs_you: false,
            apps: vec!["Linear".into(), "Notion".into(), "Figma".into()],
            attachments: vec![StreamAttachment::new(
                "f-matrix",
                "pricing-matrix.csv",
                "CSV · 4 rows",
                "Vendor,Free tier,Pro seat,Team seat\nLinear,Unlimited members (250 issues),$10/mo,$16/mo\nNotion,Unlimited blocks (10 guests),$12/mo,$18/mo\nFigma,3 collaborative files,$15/mo,$45/mo",
            )],
        },
        StreamMessage {
            id: "m5".into(),
            from: "writer".into(),
            to: Some("research".into()),
            kind: StreamMessageKind::Chat,
            text: "Do any of them cap seats on the free tier? I want our headline to promise unlimited teammates on Free.".into(),
            at: "14:04".into(),
            reply_to: None,
            tool_badge: None,
            needs_you: false,
            apps: Vec::new(),
            attachments: Vec::new(),
        },
        StreamMessage {
            id: "m6".into(),
            from: "research".into(),
            to: Some("writer".into()),
            kind: StreamMessageKind::Chat,
            text: "None cap seats – Notion limits external guests to 10 and Linear caps active issues at 250.".into(),
            at: "14:05".into(),
            reply_to: None,
            tool_badge: None,
            needs_you: false,
            apps: Vec::new(),
            attachments: vec![StreamAttachment::new(
                "f-notes",
                "tier-notes.md",
                "Markdown · 1.8 KB",
                "# Free Tier Guardrails\n- Unlimited workspace members on Free\n- Cap active agent runs at 250 / month\n- Pro unlocks custom domains & Stripe billing sync",
            )],
        },
        StreamMessage {
            id: "m7".into(),
            from: "writer".into(),
            to: Some("you".into()),
            kind: StreamMessageKind::Chat,
            text: "Drafted Free ($0) and Pro ($12/seat). Should Team land at $20 or $24 per seat with SSO included?".into(),
            at: "14:06".into(),
            reply_to: None,
            tool_badge: None,
            needs_you: true,
            apps: vec!["Stripe".into()],
            attachments: vec![StreamAttachment::new(
                "f-copy",
                "pricing-copy.md",
                "Markdown · 3 plans",
                "# Pricing Plans\n- **Free ($0)**: Unlimited teammates, 250 monthly runs.\n- **Pro ($12 / seat)**: Priority models, shared context, GitHub & Linear sync.\n- **Team ($20–$24 / seat)**: SAML SSO, audit trail, dedicated rate limits.",
            )],
        },
    ]
}

fn default_steps() -> Vec<SessionStep> {
    vec![
        SessionStep::new(
            "s1",
            "Compare per-seat prices across 3 peers",
            "research",
            SessionStepStatus::Done,
        )
        .with_duration("1m 24s"),
        SessionStep::new(
            "s2",
            "Draft the copy for Free, Pro and Team",
            "writer",
            SessionStepStatus::Active,
        )
        .with_duration("1m 48s"),
        SessionStep::new(
            "s3",
            "Pick the Team per-seat price",
            "you",
            SessionStepStatus::Pending,
        ),
        SessionStep::new(
            "s4",
            "Verify Stripe price IDs & open PR",
            "orchestrator",
            SessionStepStatus::Pending,
        ),
    ]
}

fn default_context() -> SessionContextSpec {
    SessionContextSpec {
        model: "Opus 5.5".into(),
        limit: 200_000,
        parts: vec![
            ContextPart {
                id: "talk".into(),
                label: "Conversation".into(),
                tokens: 28_400,
                files: Vec::new(),
            },
            ContextPart {
                id: "files".into(),
                label: "Shared files".into(),
                tokens: 24_200,
                files: vec![
                    ("pricing-matrix.csv".into(), 9_400),
                    ("tier-notes.md".into(), 6_800),
                    ("pricing-copy.md".into(), 8_000),
                ],
            },
            ContextPart {
                id: "system".into(),
                label: "Tools & schema".into(),
                tokens: 15_800,
                files: Vec::new(),
            },
        ],
    }
}

fn agent_color(agent_id: &str, p: &ProPalette) -> Hsla {
    match agent_id {
        "orchestrator" => p.blue,
        "research" => p.purple,
        "writer" => p.teal,
        _ => p.orange,
    }
}

/// Real-time multi-agent session block combining conversation stream, steer composer, team plan, and shared context.
pub struct LiveSessionBlock {
    id: ElementId,
    focus: FocusHandle,
    task: SharedString,
    agents: Vec<StreamAgent>,
    messages: Vec<StreamMessage>,
    steps: Vec<SessionStep>,
    status: LiveSessionStatus,
    context: Option<SessionContextSpec>,
    live: bool,
    focus_agent: Option<SharedString>,
    selected_msg: Option<SharedString>,
    reply_target: Option<(SharedString, SharedString, SharedString)>,
    mention_target: Option<SharedString>,
    mention_menu_open: bool,
    files_expanded: bool,
    hovered_ctx_part: Option<SharedString>,
    preview_file: Option<StreamAttachment>,
    composer_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    max_height: f32,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    boot_at: Instant,
    next_seq: u32,
}

impl EventEmitter<LiveSessionEvent> for LiveSessionBlock {}

impl LiveSessionBlock {
    /// Create a new [`LiveSessionBlock`] preloaded with the multi-agent pricing-launch session.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            task: "Launch the new pricing page".into(),
            agents: default_agents(),
            messages: default_messages(),
            steps: default_steps(),
            status: LiveSessionStatus::Running,
            context: Some(default_context()),
            live: true,
            focus_agent: None,
            selected_msg: None,
            reply_target: None,
            mention_target: None,
            mention_menu_open: false,
            files_expanded: true,
            hovered_ctx_part: None,
            preview_file: None,
            composer_editor: None,
            editor_focused: false,
            max_height: 520.,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(894.)),
            toast: None,
            boot_at: Instant::now(),
            next_seq: 1,
        }
    }

    /// Set the overall session task heading.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = task.into();
        self
    }

    /// Override the participating [`StreamAgent`] roster.
    pub fn with_agents(mut self, agents: Vec<StreamAgent>) -> Self {
        self.agents = agents;
        self
    }

    /// Override the initial [`StreamMessage`] conversation stream.
    pub fn with_messages(mut self, messages: Vec<StreamMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Override the team's ordered [`SessionStep`] plan.
    pub fn with_steps(mut self, steps: Vec<SessionStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Set the initial [`LiveSessionStatus`] (`Running`, `Paused`, `Done`).
    pub fn with_status(mut self, status: LiveSessionStatus) -> Self {
        self.status = status;
        self
    }

    /// Configure or hide (`None`) the shared context window card below the plan.
    pub fn with_context(mut self, context: Option<SessionContextSpec>) -> Self {
        self.context = context;
        self
    }

    /// Toggle live streaming indicator and avatar wave animations.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Narrow the session to a specific agent id (`None` shows everyone).
    pub fn with_focus_agent(mut self, agent_id: Option<SharedString>) -> Self {
        self.focus_agent = agent_id;
        self
    }

    /// Set the maximum stream height when stacked on narrow viewports.
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(260.);
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty session state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently focused agent id, if narrowed.
    pub fn focus_agent(&self) -> Option<&str> {
        self.focus_agent.as_deref()
    }

    /// Set or clear the focused agent filter and emit [`LiveSessionEvent::FocusAgentChanged`].
    pub fn set_focus_agent(&mut self, next: Option<SharedString>, cx: &mut Context<Self>) {
        if self.focus_agent != next {
            self.focus_agent = next.clone();
            cx.emit(LiveSessionEvent::FocusAgentChanged(next));
            cx.notify();
        }
    }

    fn agent_name(&self, id: &str, cx: &App) -> SharedString {
        if id == "you" {
            return tr(cx, "You", "你").into();
        }
        self.agents
            .iter()
            .find(|a| a.id.as_ref() == id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| SharedString::from(id.to_owned()))
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Message the team or steer the active step…",
                "向团队发送指令或引导当前步骤…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    let v = ed_ent.read(cx).value().to_owned();
                    if v.ends_with('@') {
                        this.mention_menu_open = true;
                    }
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty() {
                        this.send_steer_message(val, cx);
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    if this.mention_menu_open {
                        this.mention_menu_open = false;
                    } else if this.reply_target.take().is_some() {
                    } else {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }
    }

    fn send_steer_message(&mut self, raw_text: String, cx: &mut Context<Self>) {
        let trimmed = raw_text.trim();
        if trimmed.is_empty() {
            return;
        }
        let msg_id = SharedString::from(format!("user-{}", self.next_seq));
        self.next_seq += 1;

        let to = self
            .mention_target
            .clone()
            .or_else(|| self.focus_agent.clone().filter(|id| id.as_ref() != "you"));
        let reply_meta = self.reply_target.take();
        let reply_id = reply_meta.as_ref().map(|(id, _, _)| id.clone());
        let reply_pair = reply_meta.map(|(_, who, snippet)| (who, snippet));

        // Clear any resolved "needs_you" flag when the user answers
        for m in &mut self.messages {
            if m.needs_you {
                m.needs_you = false;
            }
        }

        self.messages.push(StreamMessage {
            id: msg_id,
            from: "you".into(),
            to: to.clone(),
            kind: StreamMessageKind::Chat,
            text: SharedString::from(trimmed.to_owned()),
            at: "now".into(),
            reply_to: reply_pair,
            tool_badge: None,
            needs_you: false,
            apps: Vec::new(),
            attachments: Vec::new(),
        });

        // Attach steer note to the active step (or mark "you" step done)
        if let Some(you_step) = self
            .steps
            .iter_mut()
            .find(|s| s.agent.as_ref() == "you" && s.status != SessionStepStatus::Done)
        {
            you_step.status = SessionStepStatus::Done;
            you_step.duration_label = Some("just now".into());
            you_step.note = Some(SharedString::from(trimmed.to_owned()));
        } else if let Some(act_step) = self
            .steps
            .iter_mut()
            .find(|s| s.status == SessionStepStatus::Active)
        {
            act_step.note = Some(SharedString::from(trimmed.to_owned()));
        }

        // Advance conversation tokens in shared context
        if let Some(ctx) = &mut self.context
            && let Some(talk) = ctx.parts.iter_mut().find(|p| p.id.as_ref() == "talk")
        {
            talk.tokens = talk.tokens.saturating_add(1_200);
        }

        self.mention_menu_open = false;
        self.toast = Some((
            tr(cx, "Sent steer note to team", "已向智能体团队发送引导指令").into(),
            Instant::now(),
        ));
        cx.emit(LiveSessionEvent::MessageSent {
            text: SharedString::from(trimmed.to_owned()),
            to,
            reply_to: reply_id,
        });
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.editor_focused || ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control
        {
            return;
        }
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.preview_file.take().is_some()
                    || self.mention_menu_open
                    || self.selected_msg.take().is_some()
                {
                    self.mention_menu_open = false;
                    cx.stop_propagation();
                    cx.notify();
                } else if self.focus_agent.is_some() {
                    self.set_focus_agent(None, cx);
                    cx.stop_propagation();
                }
            }
            "left" => {
                let mut tabs: Vec<Option<SharedString>> = vec![None, Some("you".into())];
                for a in &self.agents {
                    tabs.push(Some(a.id.clone()));
                }
                let pos = tabs
                    .iter()
                    .position(|t| t == &self.focus_agent)
                    .unwrap_or(0);
                let prev = if pos == 0 { tabs.len() - 1 } else { pos - 1 };
                self.set_focus_agent(tabs[prev].clone(), cx);
                cx.stop_propagation();
            }
            "right" => {
                let mut tabs: Vec<Option<SharedString>> = vec![None, Some("you".into())];
                for a in &self.agents {
                    tabs.push(Some(a.id.clone()));
                }
                let pos = tabs
                    .iter()
                    .position(|t| t == &self.focus_agent)
                    .unwrap_or(0);
                let next = (pos + 1) % tabs.len();
                self.set_focus_agent(tabs[next].clone(), cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    fn render_top_tabs(&self, elapsed_ms: f32, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let sensor = pro_selection_sensor();
        let active_key: SharedString = self
            .focus_agent
            .clone()
            .unwrap_or_else(|| SharedString::from("all"));
        let is_live_running = self.live && self.status == LiveSessionStatus::Running;

        let mut tabs = div()
            .relative()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(3.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "agent-tabs-pill"),
                active_key,
                sensor.clone(),
                7.,
                p.chip_hover,
                Some(p.line),
                false,
            ));

        // "Everyone" tab
        let all_sel = self.focus_agent.is_none();
        tabs = tabs.child(
            pro_tactile(
                pro_eid(&self.id, "tab-everyone"),
                tr(cx, "Everyone", "全部成员"),
                7.,
                transparent_black(),
                if all_sel {
                    transparent_black()
                } else {
                    p.hover
                },
                transparent_black(),
                transparent_black(),
                all_sel,
                div()
                    .h(px(28.))
                    .px(px(9.))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .text_size(px(12.5))
                    .font_weight(if all_sel {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if all_sel { p.ink } else { p.muted })
                    .child(tr(cx, "Everyone", "全部")),
                cx,
                |this, _, cx| {
                    this.set_focus_agent(None, cx);
                },
            )
            .role(Role::Tab)
            .relative()
            .flex_none()
            .when(all_sel, |d| d.child(pro_mark_selected(&sensor))),
        );

        // "You" tab
        let you_sel = self.focus_agent.as_deref() == Some("you");
        tabs = tabs.child(
            pro_tactile(
                pro_eid(&self.id, "tab-you"),
                tr(cx, "You", "你"),
                7.,
                transparent_black(),
                if you_sel {
                    transparent_black()
                } else {
                    p.hover
                },
                transparent_black(),
                transparent_black(),
                you_sel,
                div()
                    .h(px(28.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .whitespace_nowrap()
                    .text_size(px(12.5))
                    .font_weight(if you_sel {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if you_sel { p.ink } else { p.muted })
                    .child(pro_person_avatar("You", 18., true, None, cx))
                    .child(tr(cx, "You", "你")),
                cx,
                |this, _, cx| {
                    let next = if this.focus_agent.as_deref() == Some("you") {
                        None
                    } else {
                        Some(SharedString::from("you"))
                    };
                    this.set_focus_agent(next, cx);
                },
            )
            .role(Role::Tab)
            .relative()
            .flex_none()
            .when(you_sel, |d| d.child(pro_mark_selected(&sensor))),
        );

        // Agent chips with twinkling 3x3 pixel avatars
        for agent in &self.agents {
            let aid = agent.id.clone();
            let sel = self.focus_agent.as_ref() == Some(&aid);
            let col = agent_color(&aid, &p);
            let working = is_live_running && agent.busy;
            tabs = tabs.child(
                pro_tactile(
                    pro_eid(&self.id, format!("tab-agent-{aid}")),
                    agent.name.clone(),
                    7.,
                    transparent_black(),
                    if sel { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    sel,
                    div()
                        .h(px(28.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .whitespace_nowrap()
                        .text_size(px(12.5))
                        .font_weight(if sel {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if sel { p.ink } else { p.muted })
                        .child(pro_agent_avatar(&aid, col, 18., working, elapsed_ms, cx))
                        .child(agent.name.clone()),
                    cx,
                    move |this, _, cx| {
                        let next = if this.focus_agent.as_ref() == Some(&aid) {
                            None
                        } else {
                            Some(aid.clone())
                        };
                        this.set_focus_agent(next, cx);
                    },
                )
                .role(Role::Tab)
                .relative()
                .flex_none()
                .when(sel, |d| d.child(pro_mark_selected(&sensor))),
            );
        }

        let (dot_col, status_txt) = match self.status {
            LiveSessionStatus::Running if self.live => (p.blue, tr(cx, "Live", "实时协作")),
            LiveSessionStatus::Running => (p.blue, tr(cx, "Active", "进行中")),
            LiveSessionStatus::Paused => (p.amber, tr(cx, "Paused", "已暂停")),
            LiveSessionStatus::Done => (p.green, tr(cx, "Done", "已完成")),
        };

        div()
            .w_full()
            .pb(px(6.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(tabs)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(div().size(px(6.)).rounded_full().bg(dot_col))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "live-status-lbl"),
                        status_txt,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            )
    }

    fn render_message_row(
        &self,
        msg: &StreamMessage,
        elapsed_ms: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let kept = self
            .focus_agent
            .as_ref()
            .is_none_or(|fid| msg.involves_agent(fid));
        let row_opacity = pro_spring_f32(
            pro_eid(&self.id, format!("msg-op-{}", msg.id)),
            if kept { 1.0 } else { 0.30 },
            0.22,
            w,
            cx,
        );

        if msg.kind == StreamMessageKind::System {
            return div()
                .w_full()
                .pl(px(32.))
                .py(px(3.))
                .opacity(row_opacity)
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(div().size(px(4.)).rounded_full().bg(p.faint))
                .child(div().min_w_0().truncate().child(msg.text.clone()))
                .into_any_element();
        }

        let is_you = msg.from.as_ref() == "you";
        let sender_name = self.agent_name(&msg.from, cx);
        let to_badge = msg.to.as_ref().map(|tid| {
            let tname = self.agent_name(tid, cx);
            format!("→ {tname}")
        });
        let msg_id = msg.id.clone();
        let selected = self.selected_msg.as_ref() == Some(&msg_id);
        let sel_mix = pro_spring_f32(
            pro_eid(&self.id, format!("msg-sel-{}", msg.id)),
            if selected { 1.0 } else { 0.0 },
            0.20,
            w,
            cx,
        );

        let reply_author = sender_name.clone();
        let reply_snip: SharedString = msg.text.chars().take(64).collect::<String>().into();
        let reply_mid = msg.id.clone();

        let bubble_bg: Hsla = if is_you { p.subtle } else { p.card };
        let base_border: Hsla = if msg.needs_you {
            p.amber.opacity(0.45)
        } else {
            p.line
        };
        let bubble_border: Hsla = pro_mix_color(base_border, p.blue.opacity(0.58), sel_mix);

        let quote_el = msg.reply_to.as_ref().map(|(who, snippet)| {
            div()
                .w_full()
                .px(px(8.))
                .py(px(5.))
                .rounded(px(7.))
                .bg(p.chip)
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child(who.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .truncate()
                        .child(snippet.clone()),
                )
        });

        let apps_and_files = if !msg.apps.is_empty() || !msg.attachments.is_empty() {
            let mut chips = div()
                .pt(px(2.))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.));
            for app in &msg.apps {
                chips = chips.child(
                    div()
                        .h(px(22.))
                        .px(px(6.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.5))
                        .text_color(p.ink)
                        .child(pro_app_tile(app, 14., cx))
                        .child(app.clone()),
                );
            }
            for att in &msg.attachments {
                let att_clone = att.clone();
                chips = chips.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("att-{}-{}", msg.id, att.id)),
                        att.name.clone(),
                        6.,
                        p.chip,
                        p.chip_hover,
                        p.line,
                        p.line_strong,
                        false,
                        div()
                            .h(px(24.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .whitespace_nowrap()
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .child(pro_svg("0 0 12 12", FILE_SHEET_SVG, 11., p.blue))
                            .child(att.name.clone()),
                        cx,
                        move |this, _, cx| {
                            cx.emit(LiveSessionEvent::FileOpened(att_clone.name.clone()));
                            this.preview_file = Some(att_clone.clone());
                            cx.notify();
                        },
                    )
                    .flex_none(),
                );
            }
            Some(chips)
        } else {
            None
        };

        let card = div()
            .max_w(px(490.))
            .min_w_0()
            .rounded(px(12.))
            .bg(bubble_bg)
            .border_1()
            .border_color(bubble_border)
            .px(px(12.))
            .py(px(9.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .when_some(quote_el, |d, q| d.child(q))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(sender_name),
                    )
                    .when_some(to_badge, |d, tb| {
                        d.child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(p.text)
                                .child(tb),
                        )
                    })
                    .when_some(msg.tool_badge.clone(), |d, tool| {
                        d.child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(pro_icon(ProIcon::Globe, 10., p.muted))
                                .child(tool),
                        )
                    })
                    .when(msg.needs_you, |d| {
                        d.child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(p.amber_soft)
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.amber_ink)
                                .child(tr(cx, "Needs you", "待你确认")),
                        )
                    })
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child({
                                let msg_from = msg.from.clone();
                                pro_tactile(
                                    pro_eid(&self.id, format!("reply-btn-{}", msg.id)),
                                    tr(cx, "Reply", "回复"),
                                    4.,
                                    transparent_black(),
                                    p.chip,
                                    transparent_black(),
                                    transparent_black(),
                                    false,
                                    div()
                                        .h(px(18.))
                                        .px(px(5.))
                                        .flex()
                                        .items_center()
                                        .whitespace_nowrap()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .child(tr(cx, "Reply", "回复")),
                                    cx,
                                    move |this, w, cx| {
                                        this.reply_target = Some((
                                            reply_mid.clone(),
                                            reply_author.clone(),
                                            reply_snip.clone(),
                                        ));
                                        if !is_you {
                                            this.mention_target = Some(msg_from.clone());
                                        }
                                        if let Some(ed) = &this.composer_editor {
                                            ed.read(cx).focus.clone().focus(w, cx);
                                        }
                                        cx.notify();
                                    },
                                )
                                .flex_none()
                            })
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child(msg.at.clone()),
                            ),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(19.))
                    .text_color(if selected { p.ink } else { p.text })
                    .child(msg.text.clone()),
            )
            .when_some(apps_and_files, |d, af| d.child(af));

        if is_you {
            let is_dynamic_user = msg.id.as_ref().starts_with("user-");
            let row = div()
                .w_full()
                .opacity(row_opacity)
                .flex()
                .justify_end()
                .child(card)
                .into_any_element();
            if is_dynamic_user {
                SessionMessageEntrance {
                    id: pro_eid(&self.id, format!("msg-ent-{}", msg.id)),
                    content: row,
                }
                .into_any_element()
            } else {
                row
            }
        } else {
            let col = agent_color(&msg.from, &p);
            let working = self.live && self.status == LiveSessionStatus::Running;
            div()
                .id(pro_eid(&self.id, format!("msg-row-{msg_id}")))
                .w_full()
                .opacity(row_opacity)
                .flex()
                .items_start()
                .gap(px(10.))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.selected_msg.as_ref() == Some(&msg_id) {
                        this.selected_msg = None;
                    } else {
                        this.selected_msg = Some(msg_id.clone());
                    }
                    cx.notify();
                }))
                .child(div().mt(px(4.)).child(pro_agent_avatar(
                    &msg.from, col, 22., working, elapsed_ms, cx,
                )))
                .child(div().flex_1().min_w_0().child(card))
                .into_any_element()
        }
    }

    fn render_composer(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let draft_val = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let can_send = !draft_val.is_empty();
        let send_t = pro_spring_f32(
            pro_eid(&self.id, "ls-send-t"),
            f32::from(can_send),
            0.18,
            w,
            cx,
        );
        let send_bg = pro_mix_color(p.chip, p.primary, send_t);
        let send_fg = pro_mix_color(p.faint, p.on_primary, send_t);

        let target_label = match &self.mention_target {
            Some(tid) => format!("{} @{}", tr(cx, "to", "发给"), self.agent_name(tid, cx)),
            None => tr(cx, "to everyone", "发给全部智能体").to_owned(),
        };

        let agents = self.agents.clone();
        let current_target = self.mention_target.clone();
        let base_id = self.id.clone();
        let ent = cx.entity();
        let mention_popup =
            div()
                .absolute()
                .bottom(px(94.))
                .left(px(8.))
                .child(pro_popover_presence(
                    pro_eid(&self.id, "mention-pop"),
                    self.mention_menu_open,
                    move || {
                        let sel_all = current_target.is_none();
                        let ent_all = ent.clone();
                        div()
                            .w(px(224.))
                            .p(px(4.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow_md()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .id(pro_eid(&base_id, "at-opt-all"))
                                    .h(px(28.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .bg(if sel_all { p.chip } else { transparent_black() })
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(12.5))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        ent_all.update(cx, |this, cx| {
                                            this.mention_target = None;
                                            this.mention_menu_open = false;
                                            cx.notify();
                                        });
                                    })
                                    .child("Everyone")
                                    .when(sel_all, |d| {
                                        d.child(pro_check_draw(
                                            pro_eid(&base_id, "at-chk-all"),
                                            true,
                                            13.,
                                            false,
                                            p.blue,
                                        ))
                                    }),
                            )
                            .children(agents.into_iter().map(|ag| {
                                let ent_ag = ent.clone();
                                let aid = ag.id.clone();
                                let sel = current_target.as_ref() == Some(&aid);
                                div()
                                    .id(pro_eid(&base_id, format!("at-opt-{aid}")))
                                    .h(px(28.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .bg(if sel { p.chip } else { transparent_black() })
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(6.))
                                    .text_size(px(12.5))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        let pick = aid.clone();
                                        ent_ag.update(cx, |this, cx| {
                                            this.mention_target = Some(pick);
                                            this.mention_menu_open = false;
                                            cx.notify();
                                        });
                                    })
                                    .child(format!("@{}", ag.name))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(p.faint)
                                                    .child(ag.model),
                                            )
                                            .when(sel, |d| {
                                                d.child(pro_check_draw(
                                                    pro_eid(&base_id, format!("at-chk-{}", ag.id)),
                                                    true,
                                                    13.,
                                                    false,
                                                    p.blue,
                                                ))
                                            }),
                                    )
                            }))
                            .into_any_element()
                    },
                ));

        let ed_focus_click = self.composer_editor.clone();
        let ed_submit = self.composer_editor.clone();
        let has_reply = self.reply_target.is_some();
        let (reply_who, reply_snip) = self
            .reply_target
            .as_ref()
            .map(|(_, w_s, s_s)| (w_s.clone(), s_s.clone()))
            .unwrap_or_default();
        let clear_reply_btn = pro_icon_btn(
            pro_eid(&self.id, "clear-reply"),
            tr(cx, "Clear reply", "取消回复"),
            ProIcon::Close,
            20.,
            false,
            cx,
            |this, _, cx| {
                this.reply_target = None;
                cx.notify();
            },
        );

        div()
            .relative()
            .w_full()
            .pt(px(8.))
            .child(mention_popup)
            .child(
                div()
                    .w_full()
                    .rounded(px(14.))
                    .bg(p.card)
                    .border_1()
                    .border_color(if self.editor_focused {
                        p.line_strong
                    } else {
                        p.line
                    })
                    .px(px(12.))
                    .py(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_fold(
                        pro_eid(&self.id, "ls-reply-fold"),
                        has_reply,
                        28.,
                        8.,
                        move || {
                            div()
                                .w_full()
                                .px(px(8.))
                                .py(px(4.))
                                .rounded(px(7.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(11.5))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(reply_who),
                                        )
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_color(p.muted)
                                                .truncate()
                                                .child(reply_snip),
                                        ),
                                )
                                .child(clear_reply_btn)
                                .into_any_element()
                        },
                    ))
                    .child(
                        div()
                            .id(pro_eid(&self.id, "composer-input-hit"))
                            .min_h(px(24.))
                            .w_full()
                            .flex()
                            .items_center()
                            .cursor_text()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_, _, w, cx| {
                                    if let Some(ed) = &ed_focus_click {
                                        ed.read(cx).focus.clone().focus(w, cx);
                                    }
                                }),
                            )
                            .when_some(self.composer_editor.clone(), |d, ed| {
                                d.child(div().flex_1().min_w_0().h(px(22.)).child(ed))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .min_w_0()
                                    .child(
                                        pro_tactile(
                                            pro_eid(&self.id, "at-mention-btn"),
                                            tr(cx, "Mention an agent", "指定智能体"),
                                            999.,
                                            if self.mention_menu_open {
                                                p.chip_hover
                                            } else {
                                                p.chip
                                            },
                                            p.chip_hover,
                                            transparent_black(),
                                            transparent_black(),
                                            self.mention_menu_open,
                                            div()
                                                .size(px(26.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child("@"),
                                            cx,
                                            |this, _, cx| {
                                                this.mention_menu_open = !this.mention_menu_open;
                                                cx.notify();
                                            },
                                        )
                                        .flex_none(),
                                    )
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "ls-mention-lbl"),
                                        target_label,
                                        12.,
                                        FontWeight::NORMAL,
                                        p.muted,
                                    ))
                                    .when(self.mention_target.is_some(), |d| {
                                        d.child(
                                            pro_tactile(
                                                pro_eid(&self.id, "clear-mention"),
                                                tr(cx, "Reset", "清除"),
                                                4.,
                                                transparent_black(),
                                                p.hover,
                                                transparent_black(),
                                                transparent_black(),
                                                false,
                                                div()
                                                    .h(px(18.))
                                                    .px(px(5.))
                                                    .flex()
                                                    .items_center()
                                                    .whitespace_nowrap()
                                                    .text_size(px(11.))
                                                    .text_color(p.faint)
                                                    .child(tr(cx, "Reset", "清除")),
                                                cx,
                                                |this, _, cx| {
                                                    this.mention_target = None;
                                                    cx.notify();
                                                },
                                            )
                                            .flex_none(),
                                        )
                                    }),
                            )
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, "composer-send"),
                                    tr(cx, "Send message", "发送"),
                                    999.,
                                    send_bg,
                                    pro_mix_color(send_bg, p.hover, 0.25),
                                    transparent_black(),
                                    transparent_black(),
                                    can_send,
                                    div()
                                        .size(px(28.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::ArrowUp, 14., send_fg)),
                                    cx,
                                    move |this, w, cx| {
                                        if let Some(ed) = &ed_submit {
                                            let val = ed.read(cx).value().trim().to_owned();
                                            if !val.is_empty() {
                                                this.send_steer_message(val, cx);
                                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                                            }
                                        }
                                    },
                                )
                                .flex_none(),
                            ),
                    ),
            )
    }

    fn render_plan_card(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let done_count = self
            .steps
            .iter()
            .filter(|s| s.status == SessionStepStatus::Done)
            .count();
        let total_steps = self.steps.len().max(1);
        let current_step_num = (done_count + 1).min(total_steps);
        let plan_ratio = (done_count as f32 / total_steps as f32).clamp(0.0, 1.0);

        let spark_col = match self.status {
            LiveSessionStatus::Running => p.blue,
            LiveSessionStatus::Paused => p.amber,
            LiveSessionStatus::Done => p.green,
        };
        let state_label = match self.status {
            LiveSessionStatus::Running => tr(cx, "Running · 4m 12s", "运行中 · 4m 12s"),
            LiveSessionStatus::Paused => tr(cx, "Paused", "已暂停"),
            LiveSessionStatus::Done => tr(cx, "Completed", "已完成"),
        };
        let toggle_icon = if self.status == LiveSessionStatus::Running {
            ProIcon::Pause
        } else {
            ProIcon::Play
        };

        let mut steps_col = div()
            .w_full()
            .px(px(14.))
            .py(px(10.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        let step_len = self.steps.len();
        for (idx, step) in self.steps.iter().enumerate() {
            let is_last = idx + 1 == step_len;
            let faded = self
                .focus_agent
                .as_ref()
                .is_some_and(|fid| &step.agent != fid);
            let sid = step.id.clone();
            let step_opacity = pro_spring_f32(
                pro_eid(&self.id, format!("step-op-{sid}")),
                if faded { 0.36 } else { 1.0 },
                0.22,
                w,
                cx,
            );
            let agent_lbl = self.agent_name(&step.agent, cx);
            let is_you_step = step.agent.as_ref() == "you";
            let is_done = step.status == SessionStepStatus::Done;

            let status_glyph: AnyElement = match step.status {
                SessionStepStatus::Done => pro_check_draw(
                    pro_eid(&self.id, format!("step-chk-{sid}")),
                    true,
                    15.,
                    false,
                    p.blue,
                ),
                SessionStepStatus::Active => {
                    pro_svg("0 0 16 16", ACTIVE_ARC_SVG, 15., p.blue).into_any_element()
                }
                SessionStepStatus::Pending => {
                    pro_svg("0 0 16 16", PENDING_RING_SVG, 15., p.faint).into_any_element()
                }
            };

            let title_el: AnyElement = if step.status == SessionStepStatus::Active
                && self.status == LiveSessionStatus::Running
            {
                pro_shimmer_text(
                    pro_eid(&self.id, format!("step-shim-{sid}")),
                    step.title.clone(),
                    p.ink,
                    p.blue,
                    12.5,
                    FontWeight::MEDIUM,
                    true,
                )
            } else {
                div()
                    .text_size(px(12.5))
                    .font_weight(if step.status == SessionStepStatus::Active {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(match step.status {
                        SessionStepStatus::Done => p.text,
                        SessionStepStatus::Active => p.ink,
                        SessionStepStatus::Pending => p.muted,
                    })
                    .truncate()
                    .child(step.title.clone())
                    .into_any_element()
            };

            let has_note = step.note.is_some();
            let note_txt = step.note.clone().unwrap_or_default();
            let sid_click = sid.clone();

            steps_col = steps_col.child(
                div()
                    .relative()
                    .w_full()
                    .py(px(3.))
                    .opacity(step_opacity)
                    .when(!is_last, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(px(11.5))
                                .top(px(23.))
                                .bottom(px(-4.))
                                .w(px(1.))
                                .bg(p.line_strong),
                        )
                    })
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("plan-step-{sid}")),
                            step.title.clone(),
                            6.,
                            transparent_black(),
                            p.hover,
                            transparent_black(),
                            transparent_black(),
                            is_done,
                            div()
                                .w_full()
                                .px(px(4.))
                                .py(px(2.))
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .child(div().size(px(15.)).flex_none().child(status_glyph))
                                .child(div().flex_1().min_w_0().overflow_hidden().child(title_el))
                                .child(
                                    div()
                                        .flex_none()
                                        .h(px(18.))
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(if is_you_step { p.amber_soft } else { p.chip })
                                        .flex()
                                        .items_center()
                                        .text_size(px(10.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if is_you_step { p.amber_ink } else { p.muted })
                                        .child(agent_lbl),
                                )
                                .when_some(step.duration_label.clone(), |d, dur| {
                                    d.child(
                                        div()
                                            .flex_none()
                                            .text_size(px(11.5))
                                            .text_color(p.faint)
                                            .child(dur),
                                    )
                                }),
                            cx,
                            move |this, _, cx| {
                                if let Some(st) = this.steps.iter_mut().find(|s| s.id == sid_click)
                                {
                                    st.status = match st.status {
                                        SessionStepStatus::Done => SessionStepStatus::Pending,
                                        _ => SessionStepStatus::Done,
                                    };
                                    let now_done = st.status == SessionStepStatus::Done;
                                    if this
                                        .steps
                                        .iter()
                                        .all(|s| s.status == SessionStepStatus::Done)
                                    {
                                        this.status = LiveSessionStatus::Done;
                                    } else if this.status == LiveSessionStatus::Done {
                                        this.status = LiveSessionStatus::Running;
                                    }
                                    cx.emit(LiveSessionEvent::StepToggled {
                                        step_id: sid_click.clone(),
                                        done: now_done,
                                    });
                                    cx.notify();
                                }
                            },
                        )
                        .w_full(),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, format!("step-note-fold-{sid}")),
                        has_note,
                        28.,
                        0.,
                        move || {
                            div()
                                .ml(px(28.))
                                .mt(px(4.))
                                .px(px(8.))
                                .py(px(4.))
                                .rounded(px(6.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line)
                                .text_size(px(11.5))
                                .text_color(p.ink)
                                .child(format!("↳ {note_txt}"))
                                .into_any_element()
                        },
                    )),
            );
        }

        let sub_summary = format!(
            "{} {current_step_num}/{total_steps} · {state_label}",
            tr(cx, "Step", "步骤")
        );

        div()
            .w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(14.))
                    .pt(px(12.))
                    .pb(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_icon(ProIcon::Sparkles, 15., spark_col))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(self.task.clone()),
                                    )
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "plan-sub-lbl"),
                                        sub_summary,
                                        11.5,
                                        FontWeight::NORMAL,
                                        p.muted,
                                    )),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "plan-pause-btn"),
                                tr(cx, "Pause or resume plan", "暂停或恢复计划"),
                                toggle_icon,
                                28.,
                                self.status == LiveSessionStatus::Paused,
                                cx,
                                |this, _, cx| {
                                    this.status = match this.status {
                                        LiveSessionStatus::Running => LiveSessionStatus::Paused,
                                        LiveSessionStatus::Paused | LiveSessionStatus::Done => {
                                            LiveSessionStatus::Running
                                        }
                                    };
                                    cx.emit(LiveSessionEvent::StatusChanged(this.status));
                                    cx.notify();
                                },
                            )),
                    )
                    .child(pro_spring_bar(
                        pro_eid(&self.id, "plan-prog-bar"),
                        plan_ratio,
                        4.,
                        2.,
                        p.chip,
                        spark_col,
                    )),
            )
            .child(steps_col)
    }

    fn render_context_card(
        &self,
        spec: &SessionContextSpec,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let used = spec.used_tokens();
        let limit = spec.limit.max(1);
        let pct = ((used as f64 / limit as f64) * 100.0).round() as u32;
        let free_k = (limit.saturating_sub(used)) as f64 / 1000.0;

        let part_colors: [Hsla; 4] = [p.blue, p.purple, p.teal, p.amber];
        let total_cells = 60usize;
        let mut cell_owners: Vec<Option<(SharedString, Hsla)>> = Vec::with_capacity(total_cells);
        for (idx, part) in spec.parts.iter().enumerate() {
            let col = part_colors[idx % part_colors.len()];
            let n_cells = ((part.tokens as f32 / limit as f32) * total_cells as f32)
                .round()
                .max(if part.tokens > 0 { 1.0 } else { 0.0 }) as usize;
            for _ in 0..n_cells {
                if cell_owners.len() < total_cells {
                    cell_owners.push(Some((part.id.clone(), col)));
                }
            }
        }
        while cell_owners.len() < total_cells {
            cell_owners.push(None);
        }

        // 3 rows x 20 cols spring-animated context waffle grid
        let mut waffle = div().mt(px(8.)).flex().flex_col().gap(px(2.5));
        for r in 0..3usize {
            let mut row_el = div().w_full().flex().gap(px(2.5));
            for c in 0..20usize {
                let idx = r * 20 + c;
                let entry = &cell_owners[idx];
                let (target_fill, base_col) = match entry {
                    Some((pid, col)) => {
                        let dim = self.hovered_ctx_part.as_ref().is_some_and(|hid| hid != pid);
                        (if dim { 0.25 } else { 1.0 }, *col)
                    }
                    None => (0.0, p.blue),
                };
                let fill_amt = pro_spring_f32(
                    pro_eid(&self.id, format!("ctx-cell-{idx}")),
                    target_fill,
                    0.24,
                    w,
                    cx,
                )
                .clamp(0.0, 1.0);
                let cell_bg = pro_mix_color(p.chip, base_col, fill_amt);
                row_el = row_el.child(div().flex_1().h(px(8.)).rounded(px(2.)).bg(cell_bg));
            }
            waffle = waffle.child(row_el);
        }

        let (pill_bg, pill_fg) = if pct >= 85 {
            (p.red_soft, p.red_ink)
        } else if pct >= 65 {
            (p.amber_soft, p.amber_ink)
        } else {
            (p.chip, p.text)
        };

        let mut parts_list = div().mt(px(8.)).flex().flex_col().gap(px(2.));
        for (idx, part) in spec.parts.iter().enumerate() {
            let col = part_colors[idx % part_colors.len()];
            let pid = part.id.clone();
            let pid_hover = pid.clone();
            let has_files = !part.files.is_empty();
            let files_open = has_files && self.files_expanded;
            let part_ratio = (part.tokens as f32 / limit as f32).clamp(0.0, 1.0);

            parts_list = parts_list.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("ctx-part-{pid}")),
                            part.label.clone(),
                            6.,
                            transparent_black(),
                            p.hover,
                            transparent_black(),
                            transparent_black(),
                            files_open,
                            div()
                                .w_full()
                                .h(px(28.))
                                .px(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(div().size(px(8.)).rounded(px(2.)).bg(col))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(part.label.clone()),
                                )
                                .child(div().w(px(38.)).flex_none().child(pro_spring_bar(
                                    pro_eid(&self.id, format!("ctx-pbar-{pid}")),
                                    part_ratio,
                                    4.,
                                    2.,
                                    p.chip,
                                    col,
                                )))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("ctx-ptok-{pid}")),
                                    part.tokens as f64 / 1000.0,
                                    1,
                                    "",
                                    "k",
                                    12.,
                                    FontWeight::NORMAL,
                                    p.muted,
                                ))
                                .when(has_files, |d| {
                                    d.child(pro_icon(
                                        if files_open {
                                            ProIcon::ChevronUp
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        12.,
                                        p.muted,
                                    ))
                                }),
                            cx,
                            move |this, _, cx| {
                                if has_files {
                                    this.files_expanded = !this.files_expanded;
                                    cx.notify();
                                }
                            },
                        )
                        .w_full()
                        .on_hover(cx.listener(
                            move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_ctx_part = Some(pid_hover.clone());
                                } else if this.hovered_ctx_part.as_ref() == Some(&pid_hover) {
                                    this.hovered_ctx_part = None;
                                }
                                cx.notify();
                            },
                        )),
                    )
                    .when(has_files, |d| {
                        let files_snapshot = part.files.clone();
                        let base_id = self.id.clone();
                        let ent = cx.entity();
                        d.child(pro_fold(
                            pro_eid(&self.id, "ctx-files-fold"),
                            files_open,
                            78.,
                            0.,
                            move || {
                                div()
                                    .pl(px(20.))
                                    .pb(px(4.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .children(files_snapshot.into_iter().map(|(fname, ftoks)| {
                                        let ent_rm = ent.clone();
                                        let fname_rm = fname.clone();
                                        div()
                                            .h(px(24.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_size(px(11.5))
                                            .child(pro_svg(
                                                "0 0 12 12",
                                                FILE_SHEET_SVG,
                                                10.,
                                                p.muted,
                                            ))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_color(p.text)
                                                    .truncate()
                                                    .child(fname.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_color(p.faint)
                                                    .child(format!("{:.1}k", ftoks as f64 / 1000.)),
                                            )
                                            .child(
                                                div()
                                                    .id(pro_eid(
                                                        &base_id,
                                                        format!("rm-ctx-{fname}"),
                                                    ))
                                                    .size(px(18.))
                                                    .rounded(px(4.))
                                                    .hover(move |s| s.bg(p.red_soft))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .cursor_pointer()
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        move |_, _, cx| {
                                                            let target = fname_rm.clone();
                                                            ent_rm.update(cx, |this, cx| {
                                                                if let Some(ctx) = &mut this.context
                                                                    && let Some(fp) = ctx
                                                                        .parts
                                                                        .iter_mut()
                                                                        .find(|p| {
                                                                            p.id.as_ref() == "files"
                                                                        })
                                                                {
                                                                    fp.files.retain(|(n, _)| {
                                                                        n != &target
                                                                    });
                                                                    fp.tokens = fp
                                                                        .files
                                                                        .iter()
                                                                        .map(|(_, t)| *t)
                                                                        .sum();
                                                                }
                                                                cx.notify();
                                                            });
                                                        },
                                                    )
                                                    .child(pro_icon(ProIcon::Close, 10., p.muted)),
                                            )
                                    }))
                                    .into_any_element()
                            },
                        ))
                    }),
            );
        }

        div()
            .w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(14.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Shared context", "共享上下文")),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .when_some(pro_app_artwork("claude", 11., cx), |d, mark| {
                                        d.child(mark)
                                    })
                                    .child(spec.model.clone()),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .flex()
                            .items_baseline()
                            .gap(px(6.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "ctx-used-num"),
                                used as f64 / 1000.0,
                                1,
                                "",
                                "k",
                                22.,
                                FontWeight::MEDIUM,
                                p.strong,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{} {}k", tr(cx, "of", "/"), limit / 1000)),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(pill_bg)
                                    .flex()
                                    .items_center()
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "ctx-pct-num"),
                                        pct as f64,
                                        0,
                                        "",
                                        "%",
                                        11.5,
                                        FontWeight::MEDIUM,
                                        pill_fg,
                                    )),
                            ),
                    )
                    .child(waffle)
                    .child(parts_list),
            )
            .child(
                div()
                    .h(px(40.))
                    .px(px(14.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "ctx-free-num"),
                        free_k,
                        1,
                        "",
                        format!("k {}", tr(cx, "free", "剩余可用")),
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "ctx-compact-btn"),
                        tr(cx, "Compact", "压缩"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            if let Some(ctx) = &mut this.context
                                && let Some(talk) =
                                    ctx.parts.iter_mut().find(|p| p.id.as_ref() == "talk")
                            {
                                talk.tokens = (talk.tokens / 2).max(6_400);
                            }
                            this.toast = Some((
                                tr(
                                    cx,
                                    "Compacted earlier conversation turns",
                                    "已压缩早期对话上下文",
                                )
                                .into(),
                                Instant::now(),
                            ));
                            cx.emit(LiveSessionEvent::ContextCompacted);
                            cx.notify();
                        },
                    )),
            )
    }
}

#[derive(IntoElement)]
struct SessionMessageEntrance {
    id: ElementId,
    content: AnyElement,
}

impl RenderOnce for SessionMessageEntrance {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let st = w.use_keyed_state(self.id, cx, |_, inner_cx| {
            let mut s = Slide::new(0.);
            s.duration(1., 0.24, inner_cx);
            s
        });
        let amt = if disabled_motion(cx) {
            1.0
        } else {
            st.update(cx, |s, cx| s.get(w, cx).clamp(0., 1.))
        };
        div()
            .w_full()
            .relative()
            .top(px(6. * (1. - amt)))
            .opacity(amt)
            .child(self.content)
    }
}

impl Render for LiveSessionBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let is_animating = (self.live && self.status == LiveSessionStatus::Running) || self.loading;
        if is_animating && !reduced {
            w.request_animation_frame();
        }

        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.0;
        let width = self.measured_width.get();
        let stacked = width < 820.;
        let compact = width < 480.;
        let measure_cell = self.measured_width.clone();

        if self.loading {
            let phase = (elapsed_ms / 1100.0).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .flex()
                .when(stacked, |d| d.flex_col())
                .gap(px(16.))
                .child(
                    div()
                        .flex_1()
                        .h(px(420.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(220., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(280., 64., 10., phase + 0.15, cx))
                        .child(pro_skeleton_bar(260., 64., 10., phase + 0.30, cx)),
                )
                .child(
                    div()
                        .when(!stacked, |d| d.w(px(308.)).flex_none())
                        .when(stacked, |d| d.w_full())
                        .h(px(420.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(160., 18., 5., phase + 0.1, cx))
                        .child(pro_skeleton_bar(220., 120., 8., phase + 0.25, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                    .child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Sparkles,
                            tr(cx, "No active agent session", "当前没有运行中的协作会话"),
                            tr(
                                cx,
                                "Assign a goal to launch your agent team, follow their messages, and steer each step live.",
                                "指派目标即可启动多智能体团队，实时查看协作对话并引导每一步计划。",
                            ),
                            None,
                            cx,
                        )),
                );
        }

        let top_tabs = self.render_top_tabs(elapsed_ms, cx);
        let snapshot_msgs = self.messages.clone();
        let msg_rows: Vec<AnyElement> = snapshot_msgs
            .iter()
            .map(|m| self.render_message_row(m, elapsed_ms, w, cx))
            .collect();
        let composer = self.render_composer(w, cx);

        let stream_h = if stacked {
            self.max_height.min(360.)
        } else {
            392.
        };
        let left_stream = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(top_tabs)
            .child(
                div()
                    .id(pro_eid(&self.id, "msg-scroll"))
                    .w_full()
                    .h(px(stream_h))
                    .overflow_y_scroll()
                    .pr(px(4.))
                    .py(px(6.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .children(msg_rows),
            )
            .child(composer);

        let plan_card = self.render_plan_card(w, cx);
        let ctx_card = self
            .context
            .clone()
            .map(|spec| self.render_context_card(&spec, w, cx));

        let right_rail = div()
            .when(!stacked, |d| d.w(px(308.)).flex_none())
            .when(stacked, |d| d.w_full())
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(plan_card)
            .when_some(ctx_card, |d, c| d.child(c));

        let preview_overlay = self.preview_file.clone().map(|att| {
            let dialog = div()
                .w_full()
                .max_w(px(460.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .px(px(14.))
                        .py(px(10.))
                        .border_b_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(pro_svg("0 0 12 12", FILE_SHEET_SVG, 14., p.blue))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .truncate()
                                                .child(att.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.muted)
                                                .child(att.meta),
                                        ),
                                ),
                        )
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "close-file-preview"),
                            tr(cx, "Close preview", "关闭预览"),
                            ProIcon::Close,
                            26.,
                            false,
                            cx,
                            |this, _, cx| {
                                this.preview_file = None;
                                cx.notify();
                            },
                        )),
                )
                .child(
                    div()
                        .p(px(14.))
                        .bg(p.subtle)
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.ink)
                        .child(att.preview),
                );
            div().absolute().inset_0().child(pro_modal_appear(
                pro_eid(&self.id, "file-modal"),
                att.id,
                if p.dark { 0.58 } else { 0.36 },
                dialog,
            ))
        });

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.task.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if compact {
                16.
            } else if width < 980. {
                20.
            } else {
                32.
            }))
            .py(px(if compact { 20. } else { 28. }))
            .flex()
            .flex_col()
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
                .w_full()
                .h(px(0.)),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "session-vtx"),
                SharedString::from("live-session-ready"),
                div()
                    .w_full()
                    .flex()
                    .when(stacked, |d| d.flex_col())
                    .items_start()
                    .gap(px(16.))
                    .child(left_stream)
                    .child(right_rail),
            ))
            .when_some(preview_overlay, |d, ov| d.child(ov))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SessionStepStatus, default_agents, default_context, default_messages, default_steps,
    };

    #[test]
    fn agent_focus_narrows_both_messages_and_plan_steps() {
        let agents = default_agents();
        let messages = default_messages();
        let steps = default_steps();

        assert_eq!(agents.len(), 3);
        let research_msgs = messages
            .iter()
            .filter(|m| m.involves_agent("research"))
            .count();
        assert_eq!(research_msgs, 4);

        let writer_steps: Vec<&str> = steps
            .iter()
            .filter(|s| s.agent.as_ref() == "writer")
            .map(|s| s.id.as_ref())
            .collect();
        assert_eq!(writer_steps, vec!["s2"]);
        assert_eq!(steps[0].status, SessionStepStatus::Done);
        assert_eq!(steps[1].status, SessionStepStatus::Active);
    }

    #[test]
    fn shared_context_calculates_token_totals_and_utilization() {
        let ctx = default_context();
        assert_eq!(ctx.used_tokens(), 68_400);
        let ratio = ctx.utilization();
        assert!((ratio - 0.342).abs() < 0.002);
        assert_eq!(ctx.parts[1].files.len(), 3);
    }
}
