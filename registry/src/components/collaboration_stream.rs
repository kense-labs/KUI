//! Inter-agent collaboration stream with `From -> To` avatars, handoff/question badges, file previews, payload cards, and `@agent` composer.
//!
//! Shows what orchestrators, sub-agents, and humans say to each other in real time:
//! every message displays `From -> To` (`pro_agent_avatar` / `pro_person_avatar`), `Handoff` and
//! `Question for you` badges, clickable file chips that open an inline preview modal, collapsible
//! structured payload cards (`pro_fold`), and a live [`Editor`] composer with `@agent` mentions.

use crate::{
    components::pro_chrome::{
        ProPalette, ProSelectionSensor, pro_agent_avatar, pro_app_tile, pro_card_shadow,
        pro_check_draw, pro_ease, pro_eid, pro_empty_state, pro_fold, pro_icon_btn,
        pro_mark_selected, pro_mix_color, pro_person_avatar, pro_pill_indicator,
        pro_popover_presence, pro_presence, pro_selection_sensor, pro_shimmer_text,
        pro_skeleton_bar, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::collections::{HashMap, HashSet};
use web_time::Instant;

const FILE_SHEET_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.35"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;
const FILE_CODE_SVG: &str = r#"<path d="M3.8 3.6 1.6 6l2.2 2.4M8.2 3.6 10.4 6 8.2 8.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"></path><path d="M6.9 2.8 5.1 9.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" opacity="0.55"></path>"#;
const FILE_DOC_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.35"></path><path d="M4 6.2h4M4 8.2h2.8" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

/// Accent color slot assigned to a [`StreamAgent`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamAgentTone {
    /// Primary blue accent (orchestrators).
    #[default]
    Blue,
    /// Cyan accent (research/search agents).
    Cyan,
    /// Purple accent (writing/coding agents).
    Purple,
    /// Green accent (QA/verification agents).
    Green,
    /// Amber accent (ops/billing agents).
    Amber,
}

impl StreamAgentTone {
    fn color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Blue => p.blue,
            Self::Cyan => p.cyan,
            Self::Purple => p.purple,
            Self::Green => p.green,
            Self::Amber => p.amber,
        }
    }
}

/// Agent participant in [`CollaborationStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamAgent {
    /// Unique agent identifier (`"orch"`, `"research"`, `"writer"`).
    pub id: SharedString,
    /// Display name used in headers and `@mentions`.
    pub name: SharedString,
    /// Optional model label (`"Claude Opus 4.6"`).
    pub model: Option<SharedString>,
    /// Deterministic color tone for [`pro_agent_avatar`].
    pub tone: StreamAgentTone,
}

impl StreamAgent {
    /// Construct a new [`StreamAgent`].
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: None,
            tone: StreamAgentTone::Blue,
        }
    }

    /// Set the model badge shown inside `@mention` suggestions and payload cards.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the avatar color tone.
    pub fn with_tone(mut self, tone: StreamAgentTone) -> Self {
        self.tone = tone;
        self
    }
}

/// Semantic kind of a [`StreamMessage`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamMessageKind {
    /// Initial prompt or request.
    Ask,
    /// Direct reply to an earlier message.
    Reply,
    /// Tagged work handoff between two participants (`Handoff` badge).
    Handoff,
    /// Shared findings or status update.
    #[default]
    Share,
    /// Tagged human-in-the-loop question (`Question for you` badge until answered).
    Question,
    /// Quiet inline system row (`"Orchestrator started Researcher"`).
    Spawn,
}

/// Preview format of a [`StreamAttachment`] chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamAttachmentBody {
    /// Tabular CSV/spreadsheet rows (`headers`, `rows`).
    Table {
        headers: Vec<SharedString>,
        rows: Vec<Vec<SharedString>>,
    },
    /// Monospace JSON or source snippet.
    Code(SharedString),
    /// Plain/markdown document prose with optional heading.
    Document {
        heading: Option<SharedString>,
        body: SharedString,
    },
}

/// Clickable file chip attached to a [`StreamMessage`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamAttachment {
    /// File chip label (`"pricing-table.csv"`, `"copy-draft.md"`).
    pub label: SharedString,
    /// Optional size or row-count subtitle shown in the preview header (`"3 rows · CSV"`).
    pub meta: Option<SharedString>,
    /// Rich content rendered when the user clicks the chip.
    pub content: StreamAttachmentBody,
}

impl StreamAttachment {
    /// Construct a CSV/table attachment chip.
    pub fn table(
        label: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        headers: Vec<SharedString>,
        rows: Vec<Vec<SharedString>>,
    ) -> Self {
        Self {
            label: label.into(),
            meta: Some(meta.into()),
            content: StreamAttachmentBody::Table { headers, rows },
        }
    }

    /// Construct a code or JSON attachment chip.
    pub fn code(
        label: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        snippet: impl Into<SharedString>,
    ) -> Self {
        Self {
            label: label.into(),
            meta: Some(meta.into()),
            content: StreamAttachmentBody::Code(snippet.into()),
        }
    }

    /// Construct a document attachment chip.
    pub fn document(
        label: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        heading: Option<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        Self {
            label: label.into(),
            meta: Some(meta.into()),
            content: StreamAttachmentBody::Document {
                heading,
                body: body.into(),
            },
        }
    }

    fn icon_svg(&self) -> &'static str {
        match &self.content {
            StreamAttachmentBody::Table { .. } => FILE_SHEET_SVG,
            StreamAttachmentBody::Code(_) => FILE_CODE_SVG,
            StreamAttachmentBody::Document { .. } => FILE_DOC_SVG,
        }
    }
}

/// Structured item row inside [`StreamPayload`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamPayloadItem {
    /// Optional provider/app tile key (`"linear"`, `"notion"`, `"figma"`).
    pub app: Option<SharedString>,
    /// Primary row title.
    pub label: SharedString,
    /// Secondary detail or metric.
    pub detail: SharedString,
}

impl StreamPayloadItem {
    /// Construct a new [`StreamPayloadItem`].
    pub fn new(
        app: Option<&str>,
        label: impl Into<SharedString>,
        detail: impl Into<SharedString>,
    ) -> Self {
        Self {
            app: app.map(|s| SharedString::from(s.to_string())),
            label: label.into(),
            detail: detail.into(),
        }
    }
}

/// Structured telemetry and payload facts attached under a [`StreamMessage`] (`pro_fold`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamPayload {
    /// Uppercase key-value telemetry pairs (`("MODEL", "Gemini 2.5 Flash")`).
    pub facts: Vec<(SharedString, SharedString)>,
    /// Structured entity/tool rows displayed below the facts strip.
    pub items: Vec<StreamPayloadItem>,
}

impl StreamPayload {
    /// Construct a [`StreamPayload`] with key-value facts and item rows.
    pub fn new(facts: Vec<(SharedString, SharedString)>, items: Vec<StreamPayloadItem>) -> Self {
        Self { facts, items }
    }

    fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.items.is_empty()
    }
}

/// Single message or spawn event in [`CollaborationStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamMessage {
    /// Unique message identifier.
    pub id: SharedString,
    /// Sender id (`"you"` or a [`StreamAgent::id`]).
    pub from: SharedString,
    /// Recipient id (`"you"`, `"all"`, or a [`StreamAgent::id`]).
    pub to: SharedString,
    /// Semantic kind (`Ask`, `Reply`, `Handoff`, `Share`, `Question`, `Spawn`).
    pub kind: StreamMessageKind,
    /// Message text (or live status phrase while `typing` is `true`).
    pub text: SharedString,
    /// Optional wall-clock timestamp (`"14:04"`).
    pub at: Option<SharedString>,
    /// Optional parent message id being answered.
    pub reply_to: Option<SharedString>,
    /// Whether the sender is still actively writing this message.
    pub typing: bool,
    /// Clickable file chips attached to this message.
    pub attachments: Vec<StreamAttachment>,
    /// Optional structured payload card expandable via [`pro_fold`].
    pub details: Option<StreamPayload>,
}

impl StreamMessage {
    /// Construct a new [`StreamMessage`].
    pub fn new(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        kind: StreamMessageKind,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            kind,
            text: text.into(),
            at: None,
            reply_to: None,
            typing: false,
            attachments: Vec::new(),
            details: None,
        }
    }

    /// Set the display timestamp (`"14:04"`).
    pub fn with_at(mut self, at: impl Into<SharedString>) -> Self {
        self.at = Some(at.into());
        self
    }

    /// Link this message as a reply to `parent_id`.
    pub fn with_reply_to(mut self, parent_id: impl Into<SharedString>) -> Self {
        self.reply_to = Some(parent_id.into());
        self
    }

    /// Mark whether the sender is still streaming/writing this message.
    pub fn with_typing(mut self, typing: bool) -> Self {
        self.typing = typing;
        self
    }

    /// Attach clickable file chips that open the inline preview dialog.
    pub fn with_attachments(mut self, attachments: Vec<StreamAttachment>) -> Self {
        self.attachments = attachments;
        self
    }

    /// Attach a structured payload card expandable via [`pro_fold`].
    pub fn with_details(mut self, details: StreamPayload) -> Self {
        self.details = Some(details);
        self
    }
}

/// Strongly typed events emitted by [`CollaborationStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollaborationStreamEvent {
    /// Emitted when the top participant filter changes (`None` = `Everyone`).
    FocusAgentChanged(Option<SharedString>),
    /// Emitted when a message's structured payload card unfolds or closes.
    PayloadToggled {
        message_id: SharedString,
        open: bool,
    },
    /// Emitted when a file attachment preview opens or closes.
    PreviewChanged(Option<SharedString>),
    /// Emitted when the user sends a message or answers a question from the composer.
    Send {
        text: SharedString,
        to: Vec<SharedString>,
        reply_to: Option<SharedString>,
    },
}

impl EventEmitter<CollaborationStreamEvent> for CollaborationStream {}

/// Extract `@agent` mention targets from `text`, falling back to `reply_sender` or `["all"]`.
pub(crate) fn resolve_recipients(
    text: &str,
    agents: &[StreamAgent],
    reply_sender: Option<&SharedString>,
) -> Vec<SharedString> {
    let lower = text.to_ascii_lowercase();
    let mut matched: Vec<SharedString> = Vec::new();
    for ag in agents {
        let by_name = format!("@{}", ag.name.to_ascii_lowercase());
        let by_id = format!("@{}", ag.id.to_ascii_lowercase());
        if lower.contains(&by_name) || lower.contains(&by_id) {
            matched.push(ag.id.clone());
        }
    }
    if !matched.is_empty() {
        matched
    } else if let Some(sender) = reply_sender {
        vec![sender.clone()]
    } else {
        vec!["all".into()]
    }
}

fn active_at_query(text: &str) -> Option<String> {
    let trimmed = text.trim_end_matches('\n');
    let last_at = trimmed.rfind('@')?;
    // Ensure `@` is at start or preceded by whitespace.
    if last_at > 0
        && !trimmed[..last_at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let after = &trimmed[last_at + 1..];
    if after.contains(char::is_whitespace) {
        return None;
    }
    Some(after.to_ascii_lowercase())
}

fn default_stream_agents() -> Vec<StreamAgent> {
    vec![
        StreamAgent::new("orch", "Orchestrator")
            .with_model("Claude Opus 4.6")
            .with_tone(StreamAgentTone::Blue),
        StreamAgent::new("research", "Researcher")
            .with_model("Gemini 2.5 Flash")
            .with_tone(StreamAgentTone::Cyan),
        StreamAgent::new("writer", "Writer")
            .with_model("Claude Sonnet 4.6")
            .with_tone(StreamAgentTone::Purple),
    ]
}

fn default_stream_messages(cx: &App) -> Vec<StreamMessage> {
    vec![
        StreamMessage::new(
            "m1",
            "you",
            "orch",
            StreamMessageKind::Ask,
            tr(
                cx,
                "Launch the new pricing page by Friday — per-seat plans, keep a generous free tier.",
                "周五前上线新版定价页：按席位计费，并保留慷慨的免费档。",
            ),
        )
        .with_at("14:02"),
        StreamMessage::new(
            "m2",
            "orch",
            "research",
            StreamMessageKind::Spawn,
            tr(
                cx,
                "Find out what Linear, Notion, and Figma charge per seat and what their free tiers allow.",
                "调研 Linear、Notion 与 Figma 的席位单价及免费版额度。",
            ),
        )
        .with_at("14:02"),
        StreamMessage::new(
            "m3",
            "research",
            "writer",
            StreamMessageKind::Handoff,
            tr(
                cx,
                "Linear $10, Notion $12, Figma $15 per seat. None cap members on free — Notion caps guests at 10 and Linear caps issues at 250.",
                "Linear $10、Notion $12、Figma $15/席位。三家免费版均不限成员数（Notion 限制 10 名访客，Linear 限制 250 个 Issue）。",
            ),
        )
        .with_at("14:04")
        .with_attachments(vec![
            StreamAttachment::table(
                "pricing-table.csv",
                "3 rows · CSV",
                vec![
                    tr(cx, "Product", "产品").into(),
                    tr(cx, "Pro / seat", "专业版单价").into(),
                    tr(cx, "Free tier cap", "免费版上限").into(),
                ],
                vec![
                    vec![
                        "Linear".into(),
                        "$10 / mo".into(),
                        "Unlimited members · 250 issues".into(),
                    ],
                    vec![
                        "Notion".into(),
                        "$12 / mo".into(),
                        "Unlimited blocks · 10 guests".into(),
                    ],
                    vec![
                        "Figma".into(),
                        "$15 / mo".into(),
                        "Unlimited viewers · 3 files".into(),
                    ],
                ],
            ),
            StreamAttachment::code(
                "sources.json",
                "6 queries · JSON",
                "{\n  \"verified_at\": \"2026-10-30T14:04Z\",\n  \"billing\": \"annual_per_seat\",\n  \"sources\": [\"linear.app/pricing\", \"notion.so/pricing\", \"figma.com/pricing\"]\n}",
            ),
        ])
        .with_details(StreamPayload::new(
            vec![
                (tr(cx, "MODEL", "模型").into(), "Gemini 2.5 Flash".into()),
                (tr(cx, "TOOL CALLS", "工具调用").into(), "web_search × 6".into()),
                (tr(cx, "TOKENS", "TOKEN").into(), "4,820".into()),
                (tr(cx, "DURATION", "耗时").into(), "2.1s".into()),
            ],
            vec![
                StreamPayloadItem::new(
                    Some("linear"),
                    "Linear",
                    tr(cx, "$10 / seat · 250 issue free cap", "$10 / 席 · 250 Issue 上限"),
                ),
                StreamPayloadItem::new(
                    Some("notion"),
                    "Notion",
                    tr(cx, "$12 / seat · 10 guest free cap", "$12 / 席 · 10 访客上限"),
                ),
                StreamPayloadItem::new(
                    Some("figma"),
                    "Figma",
                    tr(cx, "$15 / seat · 3 starter files", "$15 / 席 · 3 初始文件"),
                ),
            ],
        )),
        StreamMessage::new(
            "m4",
            "writer",
            "you",
            StreamMessageKind::Question,
            tr(
                cx,
                "Should the Enterprise tier display 'Custom annual' or a transparent '$49 / seat' anchor price?",
                "企业版一栏应当显示“按年定制报价”，还是直接标出透明的“$49 / 席位”锚定价格？",
            ),
        )
        .with_at("14:05")
        .with_reply_to("m1")
        .with_attachments(vec![StreamAttachment::document(
            "copy-draft.md",
            "Draft v1 · Markdown",
            Some(tr(cx, "Kense Pricing Tiers (Draft v1)", "Kense 套餐方案草稿 v1").into()),
            tr(
                cx,
                "• Free — Unlimited seats, up to 3 active workflows.\n• Pro ($12 / seat) — Unlimited runs, Handoff Swimlanes & Agent Ledger.\n• Enterprise — SSO, custom retention, dedicated cluster.",
                "• 免费版 — 不限成员席位，支持 3 条活跃工作流。\n• 专业版（$12 / 席位）— 不限运行次数，解锁交接泳道与审计账本。\n• 企业版 — SSO 单点登录、专属集群与合规留存。",
            ),
        )])
        .with_details(StreamPayload::new(
            vec![
                (tr(cx, "MODEL", "模型").into(), "Claude Sonnet 4.6".into()),
                (
                    tr(cx, "BLOCKED ON", "等待决策").into(),
                    tr(cx, "Enterprise price display", "企业版价格展示方式").into(),
                ),
            ],
            vec![],
        )),
        StreamMessage::new(
            "m5",
            "orch",
            "research",
            StreamMessageKind::Share,
            tr(
                cx,
                "Cross-checking annual discount percentages while Writer waits on your answer…",
                "等待你确认企业版定价期间，正在交叉核验年付折扣比例…",
            ),
        )
        .with_at("14:06")
        .with_typing(true),
    ]
}

/// Inter-agent collaboration stream with `From -> To` badges, file previews, payload cards, and `@agent` composer.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::collaboration_stream::{
///     CollaborationStream, CollaborationStreamEvent,
/// };
///
/// let stream = cx.new(|cx| {
///     CollaborationStream::new("pricing-collab-stream", cx)
///         .with_task("Launch the new pricing page")
///         .with_live(true)
/// });
/// cx.subscribe(&stream, |_, ev: &CollaborationStreamEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct CollaborationStream {
    id: ElementId,
    title: Option<SharedString>,
    task: Option<SharedString>,
    agents: Vec<StreamAgent>,
    messages: Vec<StreamMessage>,
    focus_agent: Option<SharedString>,
    expanded_payloads: HashSet<SharedString>,
    preview: Option<(SharedString, StreamAttachment)>,
    replying_to: Option<SharedString>,
    mention_menu_open: bool,
    mention_cursor: usize,
    composer_enabled: bool,
    live: bool,
    max_height: f32,
    loading: bool,
    empty: bool,
    next_seq: usize,
    started_at: Instant,
    tab_sensor: ProSelectionSensor,
    msg_entrances: HashMap<SharedString, (Slide, PhaseClock)>,
    answered_clocks: HashMap<SharedString, PhaseClock>,
    send_hover: Slide,
    send_press: Slide,
    at_hover: Slide,
    at_press: Slide,
    toast: Option<(SharedString, Instant)>,
    editor: Option<Entity<Editor>>,
    pending_insert: Option<String>,
    pending_clear: bool,
    pending_focus_editor: bool,
}

impl CollaborationStream {
    /// Create a [`CollaborationStream`] preloaded with the 3-agent `"Launch the new pricing page"` run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: None,
            task: Some(tr(cx, "Launch the new pricing page", "上线新版定价页").into()),
            agents: default_stream_agents(),
            messages: default_stream_messages(cx),
            focus_agent: None,
            expanded_payloads: HashSet::new(),
            preview: None,
            replying_to: None,
            mention_menu_open: false,
            mention_cursor: 0,
            composer_enabled: true,
            live: true,
            max_height: 484.0,
            loading: false,
            empty: false,
            next_seq: 100,
            started_at: Instant::now(),
            tab_sensor: pro_selection_sensor(),
            msg_entrances: HashMap::new(),
            answered_clocks: HashMap::new(),
            send_hover: Slide::snappy(0.0),
            send_press: Slide::snappy(0.0),
            at_hover: Slide::snappy(0.0),
            at_press: Slide::snappy(0.0),
            toast: None,
            editor: None,
            pending_insert: None,
            pending_clear: false,
            pending_focus_editor: false,
        }
    }

    /// Override the stream accessible heading (`"Collaboration stream"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the task subtitle shown in the stream header.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Supply the participating [`StreamAgent`] roster.
    pub fn with_agents(mut self, agents: Vec<StreamAgent>) -> Self {
        self.agents = agents;
        self
    }

    /// Supply the chronological [`StreamMessage`] list.
    pub fn with_messages(mut self, messages: Vec<StreamMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Pre-filter the feed to a specific participant (`"you"` or an agent id; `None` = `Everyone`).
    pub fn with_focus_agent(mut self, agent_id: Option<SharedString>) -> Self {
        self.focus_agent = agent_id;
        self
    }

    /// Show or hide the live pulse indicator in the header.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Enable or hide the bottom [`Editor`] composer.
    pub fn with_composer(mut self, enabled: bool) -> Self {
        self.composer_enabled = enabled;
        self
    }

    /// Configure the scrollable feed height in pixels (`440.0` default).
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(200.0);
        self
    }

    /// Render skeleton bubbles while collaboration messages load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty stream state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.composer_enabled {
            return;
        }
        if self.editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Message the agents — type @ to mention or answer a question…",
                "向智能体发送指令 — 输入 @ 点名特定智能体或直接回答提问…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.0;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.mention_cursor = 0;
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.submit_composer(cx);
                }
                EditEvent::Escape => {
                    if this.mention_menu_open {
                        this.mention_menu_open = false;
                    } else if this.replying_to.is_some() {
                        this.replying_to = None;
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.editor = Some(ed);
        }

        if self.pending_clear {
            self.pending_clear = false;
            if let Some(ed) = &self.editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
        }
        if let Some(next_text) = self.pending_insert.take()
            && let Some(ed) = &self.editor
        {
            ed.update(cx, |e, cx| e.set_value(next_text, w, cx));
        }
        if self.pending_focus_editor {
            self.pending_focus_editor = false;
            if let Some(ed) = &self.editor {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
        }
    }

    fn current_draft(&self, cx: &App) -> String {
        self.editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default()
    }

    fn insert_mention(&mut self, agent_name: &str, cx: &mut Context<Self>) {
        let cur = self.current_draft(cx);
        let next = if let Some(last_at) = cur.rfind('@') {
            format!("{}@{} ", &cur[..last_at], agent_name)
        } else if cur.is_empty() || cur.ends_with(' ') {
            format!("{cur}@{agent_name} ")
        } else {
            format!("{cur} @{agent_name} ")
        };
        self.mention_menu_open = false;
        self.pending_insert = Some(next);
        self.pending_focus_editor = true;
        cx.notify();
    }

    fn start_reply(&mut self, msg_id: SharedString, cx: &mut Context<Self>) {
        self.replying_to = Some(msg_id);
        self.pending_focus_editor = true;
        cx.notify();
    }

    fn submit_composer(&mut self, cx: &mut Context<Self>) {
        let raw = self.current_draft(cx);
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let reply_sender = self
            .replying_to
            .as_ref()
            .and_then(|rid| self.messages.iter().find(|m| &m.id == rid))
            .map(|m| m.from.clone());
        let to_list = resolve_recipients(&trimmed, &self.agents, reply_sender.as_ref());
        let primary_to = to_list
            .first()
            .cloned()
            .unwrap_or_else(|| SharedString::from("all"));
        let reply_id = self.replying_to.take();

        self.next_seq += 1;
        let new_id: SharedString = format!("u-{}", self.next_seq).into();
        let mut msg = StreamMessage::new(
            new_id.clone(),
            "you",
            primary_to,
            if reply_id.is_some() {
                StreamMessageKind::Reply
            } else {
                StreamMessageKind::Ask
            },
            trimmed.clone(),
        )
        .with_at(tr(cx, "now", "刚刚"));
        if let Some(rid) = &reply_id {
            msg = msg.with_reply_to(rid.clone());
            self.answered_clocks
                .entry(rid.clone())
                .or_default()
                .restart(cx);
        }
        let mut ent_slide = Slide::new(12.0);
        ent_slide.duration(0.0, 0.26, cx);
        let mut ent_clk = PhaseClock::default();
        ent_clk.restart(cx);
        self.msg_entrances.insert(new_id, (ent_slide, ent_clk));
        self.messages.push(msg);

        self.mention_menu_open = false;
        self.pending_clear = true;
        let toast_msg: SharedString =
            tr(cx, "Sent to collaboration stream", "已发送至协作流").into();
        self.toast = Some((toast_msg, Instant::now()));
        cx.emit(CollaborationStreamEvent::Send {
            text: trimmed.into(),
            to: to_list,
            reply_to: reply_id,
        });
        cx.notify();
    }

    fn toggle_payload(&mut self, msg_id: SharedString, cx: &mut Context<Self>) {
        let open = if self.expanded_payloads.remove(&msg_id) {
            false
        } else {
            self.expanded_payloads.insert(msg_id.clone());
            true
        };
        cx.emit(CollaborationStreamEvent::PayloadToggled {
            message_id: msg_id,
            open,
        });
        cx.notify();
    }

    fn open_preview(
        &mut self,
        sender_name: SharedString,
        att: StreamAttachment,
        cx: &mut Context<Self>,
    ) {
        let label = att.label.clone();
        self.preview = Some((sender_name, att));
        cx.emit(CollaborationStreamEvent::PreviewChanged(Some(label)));
        cx.notify();
    }

    fn close_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview.take().is_some() {
            cx.emit(CollaborationStreamEvent::PreviewChanged(None));
            cx.notify();
        }
    }

    fn set_focus_agent(&mut self, next: Option<SharedString>, cx: &mut Context<Self>) {
        if self.focus_agent != next {
            self.focus_agent = next.clone();
            cx.emit(CollaborationStreamEvent::FocusAgentChanged(next));
            cx.notify();
        }
    }

    fn participant_display(&self, id: &str, cx: &App) -> SharedString {
        if id == "you" {
            tr(cx, "You", "你").into()
        } else if id == "all" {
            tr(cx, "Everyone", "全员").into()
        } else if let Some(ag) = self.agents.iter().find(|a| a.id.as_ref() == id) {
            ag.name.clone()
        } else {
            SharedString::from(id.to_string())
        }
    }

    fn render_participant_avatar(
        &self,
        id: &str,
        side: f32,
        busy: bool,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        if id == "you" {
            pro_person_avatar("You", side, true, None, cx)
        } else if id == "all" {
            div()
                .size(px(side))
                .flex_none()
                .rounded(px((side * 0.30).round()))
                .bg(p.chip)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::Users, (side * 0.62).max(10.0), p.muted))
                .into_any_element()
        } else if let Some(ag) = self.agents.iter().find(|a| a.id.as_ref() == id) {
            let seed = format!("{} {}", ag.model.as_deref().unwrap_or("claude"), ag.name);
            pro_agent_avatar(&seed, ag.tone.color(p), side, busy, elapsed_ms, cx)
        } else {
            let fallback_seed = format!("{id} claude");
            pro_agent_avatar(&fallback_seed, p.blue, side, busy, elapsed_ms, cx)
        }
    }

    fn is_question_answered(&self, question_id: &SharedString) -> bool {
        self.messages
            .iter()
            .any(|m| m.from.as_ref() == "you" && m.reply_to.as_ref() == Some(question_id))
    }

    fn render_preview_overlay(
        &self,
        sender: &SharedString,
        att: &StreamAttachment,
        amount: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let card_bg: Hsla = if p.dark {
            rgb(0x202024).into()
        } else {
            rgb(0xffffff).into()
        };
        let scrim_c: Hsla = if p.dark {
            hsla(240. / 360., 0.06, 0.07, 0.72 * amount)
        } else {
            hsla(0., 0., 0.98, 0.76 * amount)
        };

        let body_el: AnyElement = match &att.content {
            StreamAttachmentBody::Table { headers, rows } => {
                let mut tbl = div().w_full().flex().flex_col();
                let mut hdr_row = div()
                    .w_full()
                    .pb(px(6.0))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(10.0));
                for h in headers {
                    hdr_row = hdr_row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .truncate()
                            .child(h.clone()),
                    );
                }
                tbl = tbl.child(hdr_row);

                for r in rows {
                    let mut r_el = div()
                        .w_full()
                        .py(px(7.0))
                        .border_b_1()
                        .border_color(p.line_soft)
                        .flex()
                        .items_center()
                        .gap(px(10.0));
                    for (ci, cell) in r.iter().enumerate() {
                        r_el = r_el.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.5))
                                .font_weight(if ci == 0 {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if ci == 0 { p.ink } else { p.text })
                                .truncate()
                                .child(cell.clone()),
                        );
                    }
                    tbl = tbl.child(r_el);
                }
                tbl.into_any_element()
            }
            StreamAttachmentBody::Code(snippet) => div()
                .w_full()
                .p(px(10.0))
                .rounded(px(8.0))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .text_size(px(12.0))
                .line_height(px(18.5))
                .text_color(p.ink)
                .child(snippet.clone())
                .into_any_element(),
            StreamAttachmentBody::Document { heading, body } => div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .when_some(heading.clone(), |d, h| {
                    d.child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink)
                            .child(h),
                    )
                })
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(19.5))
                        .text_color(p.text)
                        .child(body.clone()),
                )
                .into_any_element(),
        };

        let meta_line = match &att.meta {
            Some(m) => format!("{sender} · {m}"),
            None => sender.to_string(),
        };

        div()
            .id(pro_eid(&self.id, "file-preview-scrim"))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.close_preview(cx)),
            )
            .absolute()
            .inset_0()
            .p(px(16.0))
            .bg(scrim_c)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(pro_eid(&self.id, "file-preview-win"))
                    .role(Role::Group)
                    .aria_label(att.label.clone())
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .relative()
                    .top(px(6.0 * (1.0 - amount)))
                    .opacity(amount)
                    .w_full()
                    .max_w(px(520.0))
                    .rounded(px(14.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(14.0))
                            .py(px(11.0))
                            .border_b_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .size(px(28.0))
                                    .flex_none()
                                    .rounded(px(8.0))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_svg("0 0 12 12", att.icon_svg(), 14.0, p.muted)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(att.label.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(meta_line),
                                    ),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "close-preview"),
                                SharedString::from(tr(cx, "Close file preview", "关闭文件预览")),
                                ProIcon::Close,
                                28.0,
                                false,
                                cx,
                                |this, _, cx| this.close_preview(cx),
                            )),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "preview-scroll"))
                            .max_h(px(280.0))
                            .overflow_y_scroll()
                            .p(px(14.0))
                            .child(body_el),
                    ),
            )
            .into_any_element()
    }
}

impl Render for CollaborationStream {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editor(window, cx);

        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_secs = self.started_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_secs * 1000.0;
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.055)
        } else {
            hsla(0., 0., 0., 0.035)
        };
        let you_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.024)
        };

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Collaboration stream", "多智能体协作流").into());

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_secs / 1.4).fract();
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(260.0, 26.0, 7.0, phase, cx))
                        .child(pro_skeleton_bar(48.0, 16.0, 4.0, phase + 0.15, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .children((0..3usize).map(|i| {
                            div()
                                .p(px(12.0))
                                .rounded(px(12.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(pro_skeleton_bar(
                                    160.0,
                                    13.0,
                                    4.0,
                                    phase + i as f32 * 0.08,
                                    cx,
                                ))
                                .child(pro_skeleton_bar(
                                    360.0,
                                    14.0,
                                    4.0,
                                    phase + i as f32 * 0.12,
                                    cx,
                                ))
                        })),
                );
        }

        let any_typing = self.messages.iter().any(|m| m.typing);
        if (self.live || any_typing || self.toast.is_some()) && !reduced {
            window.request_animation_frame();
        }

        // Top participant filter tabs (`Everyone`, `You`, and each `StreamAgent`) with persistent liquid pro_pill_indicator.
        let tab_sensor = self.tab_sensor.clone();
        let active_tab_key: SharedString = self
            .focus_agent
            .clone()
            .unwrap_or_else(|| SharedString::from("__everyone__"));

        let mut tabs_row = div()
            .id(pro_eid(&self.id, "agent-tabs"))
            .role(Role::TabList)
            .aria_label(match &self.task {
                Some(t) => format!("{title_str}: {t}"),
                None => title_str.to_string(),
            })
            .relative()
            .min_w_0()
            .p(px(2.0))
            .rounded(px(9.0))
            .bg(p.subtle)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.0))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tab-pill"),
                active_tab_key.clone(),
                tab_sensor.clone(),
                7.0,
                card_bg,
                Some(p.line),
                true,
            ));

        let mut tab_defs: Vec<(Option<SharedString>, SharedString, Option<AnyElement>)> =
            Vec::new();
        tab_defs.push((None, tr(cx, "Everyone", "全员").into(), None));
        tab_defs.push((
            Some("you".into()),
            tr(cx, "You", "你").into(),
            Some(pro_person_avatar("You", 16.0, true, None, cx)),
        ));
        for ag in &self.agents {
            let ag_busy = self.live && self.messages.iter().any(|m| m.from == ag.id && m.typing);
            let tab_seed = format!("{} {}", ag.model.as_deref().unwrap_or("claude"), ag.name);
            let av = pro_agent_avatar(&tab_seed, ag.tone.color(&p), 16.0, ag_busy, elapsed_ms, cx);
            tab_defs.push((Some(ag.id.clone()), ag.name.clone(), Some(av)));
        }

        for (opt_id, lbl, av_opt) in tab_defs {
            let is_sel = self.focus_agent == opt_id;
            let key_str = opt_id
                .as_ref()
                .map(|s| s.as_ref().to_string())
                .unwrap_or_else(|| "everyone".to_string());
            let focus_c = p.focus;
            let next_val = opt_id.clone();
            tabs_row = tabs_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("ptab-{key_str}")))
                    .role(Role::Tab)
                    .aria_selected(is_sel)
                    .tab_index(0)
                    .relative()
                    .h(px(26.0))
                    .px(px(8.0))
                    .rounded(px(7.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(12.5))
                    .font_weight(if is_sel {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if is_sel { p.ink } else { p.muted })
                    .hover(move |s| if is_sel { s } else { s.text_color(p.ink) })
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(focus_c))
                    .when(is_sel, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .when_some(av_opt, |d, av| d.child(av))
                    .child(lbl),
                cx,
                move |this, _, cx| this.set_focus_agent(next_val.clone(), cx),
            ));
        }

        let live_dot_alpha = if !self.live {
            0.35
        } else if reduced {
            1.0
        } else {
            0.40 + 0.60 * (0.5 + 0.5 * ((elapsed_secs / 1.4) * std::f32::consts::TAU).cos())
        };
        let top_bar = div()
            .w_full()
            .pb(px(4.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.0))
            .child(tabs_row)
            .child(
                div()
                    .id(pro_eid(&self.id, "stream-live-toggle-btn"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(26.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.live = !this.live;
                        cx.notify();
                    }))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().size(px(6.0)).rounded_full().bg(if self.live {
                        p.blue.opacity(live_dot_alpha)
                    } else {
                        p.faint
                    }))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "live-state-swap"),
                        if self.live {
                            tr(cx, "Live", "实时")
                        } else {
                            tr(cx, "Paused", "已暂停")
                        },
                        12.0,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        if self.empty || self.messages.is_empty() {
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(top_bar)
                .child(
                    div()
                        .rounded(px(14.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Thread,
                            tr(cx, "No inter-agent messages yet", "暂无智能体协作消息"),
                            tr(
                                cx,
                                "Handoffs, shared files, and human-in-the-loop questions appear here as agents collaborate.",
                                "多智能体协同过程中的交接消息、共享文件与人工提问将实时呈现在此。",
                            ),
                            None,
                            cx,
                        )),
                );
        }

        // Build stream items.
        let mut feed_col = div().w_full().py(px(8.0)).flex().flex_col().gap(px(8.0));

        for msg in &self.messages {
            let matches_focus = match &self.focus_agent {
                None => true,
                Some(fid) => &msg.from == fid || &msg.to == fid,
            };
            let item_opacity = if matches_focus { 1.0 } else { 0.30 };
            let from_name = self.participant_display(msg.from.as_ref(), cx);
            let to_name = self.participant_display(msg.to.as_ref(), cx);

            if msg.kind == StreamMessageKind::Spawn {
                feed_col = feed_col.child(
                    div()
                        .w_full()
                        .pl(px(32.0))
                        .py(px(2.0))
                        .opacity(item_opacity)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(5.0))
                        .text_size(px(12.0))
                        .text_color(p.faint)
                        .child(format!("{from_name} {}", tr(cx, "started", "启动了")))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(to_name),
                        )
                        .child("—")
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(120.0))
                                .text_color(p.muted)
                                .truncate()
                                .child(msg.text.clone()),
                        ),
                );
                continue;
            }

            let mid = msg.id.clone();
            let is_you = msg.from.as_ref() == "you";
            let payload_open = self.expanded_payloads.contains(&mid);
            let has_payload = msg.details.as_ref().is_some_and(|d| !d.is_empty());
            let is_unanswered_q = msg.kind == StreamMessageKind::Question
                && msg.to.as_ref() == "you"
                && !self.is_question_answered(&mid);
            let is_answered_q = msg.kind == StreamMessageKind::Question
                && msg.to.as_ref() == "you"
                && !is_unanswered_q;
            let ans_drawn = if is_answered_q {
                if reduced {
                    true
                } else if let Some(clk) = self.answered_clocks.get(&mid) {
                    clk.time(0.28, window, cx) > 0.012
                } else {
                    true
                }
            } else {
                false
            };

            let (ent_dy, ent_alpha) = if reduced {
                (0.0, 1.0)
            } else if let Some((sl, clk)) = self.msg_entrances.get(&mid) {
                let y = sl.get(window, cx);
                let age = clk.time(0.24, window, cx);
                let a = 0.25 + 0.75 * pro_ease((age / 0.24).clamp(0.0, 1.0));
                (y, a)
            } else {
                (0.0, 1.0)
            };

            // Header row: From -> [To avatar] To + Badges + Payload toggle + Reply + Time.
            let to_av =
                self.render_participant_avatar(msg.to.as_ref(), 15.0, false, elapsed_ms, &p, cx);
            let reply_target_id = mid.clone();
            let payload_toggle_id = mid.clone();

            let head_row = div()
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(from_name.clone()),
                )
                .child(div().text_size(px(12.0)).text_color(p.faint).child("→"))
                .child(
                    div().flex().items_center().gap(px(4.5)).child(to_av).child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(to_name.clone()),
                    ),
                )
                .when(msg.kind == StreamMessageKind::Handoff, |d| {
                    d.child(
                        div()
                            .h(px(18.0))
                            .px(px(6.0))
                            .rounded(px(5.0))
                            .bg(p.accent_soft)
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.blue)
                            .flex()
                            .items_center()
                            .child(tr(cx, "Handoff", "任务移交")),
                    )
                })
                .when(is_unanswered_q, |d| {
                    d.child(
                        div()
                            .h(px(18.0))
                            .px(px(6.0))
                            .rounded(px(5.0))
                            .bg(p.amber_soft)
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.amber_ink)
                            .flex()
                            .items_center()
                            .child(tr(cx, "Question for you", "向你提问")),
                    )
                })
                .when(is_answered_q, |d| {
                    d.child(
                        div()
                            .h(px(18.0))
                            .px(px(6.0))
                            .rounded(px(5.0))
                            .bg(p.green_soft)
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.green_ink)
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("ans-chk-{mid}")),
                                ans_drawn,
                                12.0,
                                false,
                                p.green_ink,
                            ))
                            .child(tr(cx, "Answered", "已回答")),
                    )
                })
                .child(
                    div()
                        .ml_auto()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .when(has_payload, |d| {
                            let pld_inner = div()
                                .h(px(20.0))
                                .px(px(6.0))
                                .flex()
                                .items_center()
                                .gap(px(3.0))
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if payload_open { p.ink } else { p.muted })
                                .child(tr(cx, "Details", "结构化负载"))
                                .child(pro_icon(
                                    if payload_open {
                                        ProIcon::ChevronUp
                                    } else {
                                        ProIcon::ChevronDown
                                    },
                                    11.0,
                                    p.muted,
                                ));
                            d.child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("pld-btn-{mid}")),
                                    SharedString::from(tr(
                                        cx,
                                        "Toggle structured payload",
                                        "切换结构化详情",
                                    )),
                                    5.0,
                                    if payload_open {
                                        p.select
                                    } else {
                                        transparent_black()
                                    },
                                    p.chip_hover,
                                    transparent_black(),
                                    transparent_black(),
                                    payload_open,
                                    pld_inner,
                                    cx,
                                    move |this, _, cx| {
                                        this.toggle_payload(payload_toggle_id.clone(), cx)
                                    },
                                )
                                .flex_none(),
                            )
                        })
                        .when(self.composer_enabled && !is_you, |d| {
                            let lbl = if is_unanswered_q {
                                tr(cx, "Answer", "回答")
                            } else {
                                tr(cx, "Reply", "回复")
                            };
                            let rpl_inner = div()
                                .h(px(20.0))
                                .px(px(6.0))
                                .flex()
                                .items_center()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if is_unanswered_q {
                                    p.amber_ink
                                } else {
                                    p.muted
                                })
                                .child(lbl);
                            d.child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("rpl-btn-{mid}")),
                                    SharedString::from(format!("{lbl}: {from_name}")),
                                    5.0,
                                    if is_unanswered_q {
                                        p.amber_soft
                                    } else {
                                        transparent_black()
                                    },
                                    p.chip_hover,
                                    transparent_black(),
                                    transparent_black(),
                                    false,
                                    rpl_inner,
                                    cx,
                                    move |this, _, cx| {
                                        this.start_reply(reply_target_id.clone(), cx)
                                    },
                                )
                                .flex_none(),
                            )
                        })
                        .when_some(msg.at.clone(), |d, tm| {
                            d.child(
                                div()
                                    .pl(px(4.0))
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child(tm),
                            )
                        }),
                );

            // Optional reply quote block.
            let quote_el: Option<AnyElement> = msg
                .reply_to
                .as_ref()
                .and_then(|rid| self.messages.iter().find(|m| &m.id == rid))
                .map(|parent| {
                    let p_author = self.participant_display(parent.from.as_ref(), cx);
                    div()
                        .w_full()
                        .px(px(9.0))
                        .py(px(5.5))
                        .rounded(px(8.0))
                        .bg(if is_you { card_bg } else { well_bg })
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(format!("{} {p_author}", tr(cx, "Replying to", "回复"))),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.muted)
                                .truncate()
                                .child(parent.text.clone()),
                        )
                        .into_any_element()
                });

            // Body text or live shimmer typing indicator.
            let body_text_el: AnyElement = if msg.typing {
                pro_shimmer_text(
                    pro_eid(&self.id, format!("typing-{mid}")),
                    msg.text.clone(),
                    p.muted,
                    p.strong,
                    13.0,
                    FontWeight::NORMAL,
                    true,
                )
            } else {
                div()
                    .text_size(px(13.0))
                    .line_height(px(19.5))
                    .text_color(p.ink)
                    .child(msg.text.clone())
                    .into_any_element()
            };

            // Clickable file attachment chips with tactile KUI physics.
            let files_row: Option<AnyElement> = (!msg.attachments.is_empty()).then(|| {
                let mut row = div().pt(px(2.0)).flex().flex_wrap().gap(px(6.0));
                for (ai, att) in msg.attachments.iter().enumerate() {
                    let att_clone = att.clone();
                    let sender_clone = from_name.clone();
                    let chip_inner = div()
                        .h(px(24.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .text_size(px(12.0))
                        .text_color(p.ink)
                        .child(pro_svg("0 0 12 12", att.icon_svg(), 12.0, p.muted))
                        .child(att.label.clone());
                    row = row.child(
                        pro_tactile(
                            pro_eid(&self.id, format!("file-{mid}-{ai}")),
                            SharedString::from(format!("Preview {}", att.label)),
                            7.0,
                            well_bg,
                            p.chip_hover,
                            p.line,
                            p.line_strong,
                            false,
                            chip_inner,
                            cx,
                            move |this, _, cx| {
                                this.open_preview(sender_clone.clone(), att_clone.clone(), cx)
                            },
                        )
                        .flex_none(),
                    );
                }
                row.into_any_element()
            });

            let bubble_el = div()
                .w_full()
                .max_w(px(560.0))
                .px(px(12.0))
                .py(px(10.0))
                .rounded(px(12.0))
                .bg(if is_you { you_bg } else { card_bg })
                .border_1()
                .border_color(if is_unanswered_q {
                    p.amber.opacity(0.45)
                } else {
                    p.line
                })
                .when(!is_you, |d| d.shadow(pro_card_shadow(p.dark)))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(head_row)
                .when_some(quote_el, |d, q| d.child(q))
                .child(body_text_el)
                .when_some(files_row, |d, f| d.child(f));

            // Collapsible structured payload card (`pro_fold`) right beneath the message bubble!
            let payload_fold: AnyElement = if let Some(details) = msg.details.clone()
                && !details.is_empty()
            {
                let item_tiles: Vec<(Option<AnyElement>, SharedString, SharedString)> = details
                    .items
                    .iter()
                    .map(|it| {
                        let tile = it.app.as_ref().map(|a| pro_app_tile(a, 20.0, cx));
                        (tile, it.label.clone(), it.detail.clone())
                    })
                    .collect();
                let facts = details.facts.clone();
                let line_c = p.line;
                let muted_c = p.muted;
                let ink_c = p.ink;
                let faint_c = p.faint;
                let payload_shadow = pro_card_shadow(p.dark);

                pro_fold(
                    pro_eid(&self.id, format!("payload-{mid}")),
                    payload_open,
                    120.0,
                    0.0,
                    move || {
                        let mut card = div()
                            .mt(px(4.0))
                            .w_full()
                            .max_w(px(560.0))
                            .px(px(12.0))
                            .py(px(10.0))
                            .rounded(px(11.0))
                            .bg(card_bg)
                            .border_1()
                            .border_color(line_c)
                            .shadow(payload_shadow)
                            .flex()
                            .flex_col()
                            .gap(px(8.0));

                        if !facts.is_empty() {
                            let mut facts_row =
                                div().flex().flex_wrap().gap_x(px(18.0)).gap_y(px(6.0));
                            for (k, v) in facts {
                                facts_row = facts_row.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(muted_c)
                                                .child(k),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(ink_c)
                                                .child(v),
                                        ),
                                );
                            }
                            card = card.child(facts_row);
                        }

                        if !item_tiles.is_empty() {
                            let mut items_col = div()
                                .pt(px(7.0))
                                .border_t_1()
                                .border_color(line_c)
                                .flex()
                                .flex_col()
                                .gap(px(4.0));
                            for (tile_opt, lbl, det) in item_tiles {
                                let tile_el = tile_opt.unwrap_or_else(|| {
                                    div()
                                        .size(px(20.0))
                                        .rounded(px(5.0))
                                        .bg(card_bg)
                                        .border_1()
                                        .border_color(line_c)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(div().size(px(6.0)).rounded(px(2.0)).bg(faint_c))
                                        .into_any_element()
                                });
                                items_col = items_col.child(
                                    div()
                                        .h(px(26.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .min_w_0()
                                        .child(tile_el)
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(ink_c)
                                                .child(lbl),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_size(px(12.0))
                                                .text_color(muted_c)
                                                .truncate()
                                                .child(det),
                                        ),
                                );
                            }
                            card = card.child(items_col);
                        }
                        card.into_any_element()
                    },
                )
            } else {
                div().into_any_element()
            };

            let sender_av = (!is_you).then(|| {
                div().pt(px(6.0)).child(self.render_participant_avatar(
                    msg.from.as_ref(),
                    22.0,
                    msg.typing,
                    elapsed_ms,
                    &p,
                    cx,
                ))
            });

            feed_col = feed_col.child(
                div()
                    .w_full()
                    .relative()
                    .top(px(ent_dy))
                    .opacity(item_opacity * ent_alpha)
                    .flex()
                    .when(is_you, |d| d.justify_end())
                    .items_start()
                    .gap(px(10.0))
                    .when_some(sender_av, |d, av| d.child(av))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .when(is_you, |d| d.items_end())
                            .when(!is_you, |d| d.items_start())
                            .child(bubble_el)
                            .child(payload_fold),
                    ),
            );
        }

        let preview_layer = if let Some((sender, att)) = self.preview.clone() {
            let sender_c = sender.clone();
            let att_c = att.clone();
            let rendered_win = self.render_preview_overlay(&sender_c, &att_c, 1.0, cx);
            pro_presence(
                pro_eid(&self.id, "file-prev-pres"),
                true,
                0.20,
                0.15,
                move |amt| {
                    div()
                        .absolute()
                        .inset_0()
                        .opacity(pro_ease(amt))
                        .child(rendered_win)
                        .into_any_element()
                },
            )
        } else {
            div().into_any_element()
        };

        let scroll_panel = div()
            .relative()
            .w_full()
            .child(
                div()
                    .id(pro_eid(&self.id, "stream-scroll"))
                    .w_full()
                    .max_h(px(self.max_height))
                    .overflow_y_scroll()
                    .pr(px(4.0))
                    .child(feed_col),
            )
            .child(preview_layer);

        // Composer with `@agent` mentions and direct reply/answer banner.
        let composer_el: AnyElement = if self.composer_enabled {
            let draft_text = self.current_draft(cx);
            let typed_at = active_at_query(&draft_text);
            let show_mentions = self.mention_menu_open || typed_at.is_some();
            let filter_q = typed_at.unwrap_or_default();

            let matching_agents: Vec<StreamAgent> = self
                .agents
                .iter()
                .filter(|ag| {
                    filter_q.is_empty()
                        || ag.name.to_ascii_lowercase().contains(&filter_q)
                        || ag.id.to_ascii_lowercase().contains(&filter_q)
                })
                .cloned()
                .collect();

            let mention_open = show_mentions && !matching_agents.is_empty();
            let mention_rows: Vec<AnyElement> = matching_agents
                .iter()
                .enumerate()
                .map(|(mi, ag)| {
                    let nm = ag.name.clone();
                    let av_seed =
                        format!("{} {}", ag.model.as_deref().unwrap_or("claude"), ag.name);
                    let av =
                        pro_agent_avatar(&av_seed, ag.tone.color(&p), 18.0, false, elapsed_ms, cx);
                    let is_cur = mi == self.mention_cursor.min(matching_agents.len() - 1);
                    let m_inner = div()
                        .w_full()
                        .h(px(32.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(av)
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format!("@{}", ag.name)),
                        )
                        .when_some(ag.model.clone(), |d, mdl| {
                            d.child(
                                div()
                                    .ml_auto()
                                    .text_size(px(11.0))
                                    .text_color(p.muted)
                                    .child(mdl),
                            )
                        });
                    pro_tactile(
                        pro_eid(&self.id, format!("mopt-{}", ag.id)),
                        SharedString::from(format!("@{}", ag.name)),
                        7.0,
                        if is_cur {
                            p.select
                        } else {
                            transparent_black()
                        },
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        is_cur,
                        m_inner,
                        cx,
                        move |this, _, cx| this.insert_mention(nm.as_ref(), cx),
                    )
                    .w_full()
                    .into_any_element()
                })
                .collect();

            let mention_aria = tr(cx, "Mention an agent", "选择点名的智能体").to_owned();
            let menu_eid = pro_eid(&self.id, "mention-menu");
            let mention_popup = pro_popover_presence(
                pro_eid(&self.id, "mention-pop-pres"),
                mention_open,
                move || {
                    div()
                        .id(menu_eid)
                        .role(Role::ListBox)
                        .aria_label(mention_aria)
                        .mb(px(6.0))
                        .p(px(4.0))
                        .w_full()
                        .max_w(px(280.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .children(mention_rows)
                        .into_any_element()
                },
            );

            let reply_target_data = self
                .replying_to
                .as_ref()
                .and_then(|rid| self.messages.iter().find(|m| &m.id == rid))
                .map(|target| {
                    let who = self.participant_display(target.from.as_ref(), cx);
                    let is_q = target.kind == StreamMessageKind::Question;
                    let summary = format!(
                        "{} {who}: {}",
                        if is_q {
                            tr(cx, "Answering", "正在回答")
                        } else {
                            tr(cx, "Replying to", "正在回复")
                        },
                        target.text
                    );
                    (is_q, summary)
                });
            let has_reply_banner = reply_target_data.is_some();
            let clear_btn = has_reply_banner.then(|| {
                pro_icon_btn(
                    pro_eid(&self.id, "clear-reply"),
                    SharedString::from(tr(cx, "Cancel reply", "取消回复")),
                    ProIcon::Close,
                    20.0,
                    false,
                    cx,
                    |this, _, cx| {
                        this.replying_to = None;
                        cx.notify();
                    },
                )
            });
            let reply_banner = pro_fold(
                pro_eid(&self.id, "reply-banner-fold"),
                has_reply_banner,
                32.0,
                0.0,
                move || {
                    let Some((is_q, summary)) = reply_target_data else {
                        return div().into_any_element();
                    };
                    div()
                        .w_full()
                        .px(px(9.0))
                        .py(px(5.0))
                        .rounded(px(8.0))
                        .bg(if is_q { p.amber_soft } else { well_bg })
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.0))
                                .text_color(if is_q { p.amber_ink } else { p.text })
                                .truncate()
                                .child(summary),
                        )
                        .when_some(clear_btn, |d, cb| d.child(cb))
                        .into_any_element()
                },
            );

            let reply_sender = self
                .replying_to
                .as_ref()
                .and_then(|rid| self.messages.iter().find(|m| &m.id == rid))
                .map(|m| m.from.clone());
            let recipients = resolve_recipients(&draft_text, &self.agents, reply_sender.as_ref());
            let to_summary: SharedString =
                if recipients.len() == 1 && recipients[0].as_ref() == "all" {
                    tr(cx, "to everyone", "发送给全员").into()
                } else {
                    let names: Vec<String> = recipients
                        .iter()
                        .map(|id| self.participant_display(id.as_ref(), cx).to_string())
                        .collect();
                    format!("{} {}", tr(cx, "to", "发送给"), names.join(", ")).into()
                };

            let can_send = !draft_text.trim().is_empty();
            let focus_c = p.focus;
            let at_hov = self.at_hover.get(window, cx).clamp(0.0, 1.0);
            let at_down = self.at_press.get(window, cx).clamp(0.0, 1.0);
            let at_base = if show_mentions {
                p.select
            } else {
                transparent_black()
            };
            let at_bg = pro_mix_color(at_base, p.chip_hover, at_hov);
            let at_inset = 1.3 * at_down;

            let send_hov = self.send_hover.get(window, cx).clamp(0.0, 1.0);
            let send_down = if can_send {
                self.send_press.get(window, cx).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let send_base = if can_send { p.primary } else { p.chip };
            let send_bg = if can_send {
                pro_mix_color(
                    send_base,
                    pro_mix_color(p.primary, p.on_primary, 0.14),
                    send_hov,
                )
            } else {
                send_base
            };
            let send_fg = if can_send { p.on_primary } else { p.faint };
            let send_inset = 1.4 * send_down;

            div()
                .w_full()
                .pt(px(4.0))
                .flex()
                .flex_col()
                .child(mention_popup)
                .child(
                    div()
                        .w_full()
                        .px(px(12.0))
                        .pt(px(10.0))
                        .pb(px(9.0))
                        .rounded(px(16.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow(pro_card_shadow(p.dark))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(reply_banner)
                        .when_some(self.editor.clone(), |d, ed| {
                            d.child(div().w_full().h(px(26.0)).px(px(2.0)).child(ed))
                        })
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .id(pro_eid(&self.id, "at-mention-btn"))
                                        .role(Role::Button)
                                        .aria_label(tr(cx, "Mention an agent", "点名智能体"))
                                        .aria_expanded(show_mentions)
                                        .tab_index(0)
                                        .size(px(28.0))
                                        .relative()
                                        .rounded_full()
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if show_mentions { p.ink } else { p.muted })
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .focus_visible(move |s| s.border_1().border_color(focus_c))
                                        .on_hover(cx.listener(|this, inside: &bool, w, cx| {
                                            this.at_hover.set(if *inside { 1.0 } else { 0.0 }, cx);
                                            if !*inside {
                                                this.at_press.set(0.0, cx);
                                            }
                                            cx.notify();
                                            w.refresh();
                                        }))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, w, cx| {
                                                this.at_press.set(1.0, cx);
                                                cx.notify();
                                                w.refresh();
                                            }),
                                        )
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, w, cx| {
                                                this.at_press.set(0.0, cx);
                                                cx.notify();
                                                w.refresh();
                                            }),
                                        )
                                        .on_mouse_up_out(
                                            MouseButton::Left,
                                            cx.listener(|this, _, w, cx| {
                                                this.at_press.set(0.0, cx);
                                                cx.notify();
                                                w.refresh();
                                            }),
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.mention_menu_open = !this.mention_menu_open;
                                            cx.notify();
                                        }))
                                        .child(
                                            canvas(
                                                |_, _, _| (),
                                                move |bounds, _, win, _| {
                                                    if at_bg.a <= 0.001 {
                                                        return;
                                                    }
                                                    let bw = f32::from(bounds.size.width);
                                                    let bh = f32::from(bounds.size.height);
                                                    let rect = Bounds::new(
                                                        point(
                                                            bounds.origin.x + px(at_inset),
                                                            bounds.origin.y + px(at_inset),
                                                        ),
                                                        size(
                                                            px((bw - 2.0 * at_inset).max(2.0)),
                                                            px((bh - 2.0 * at_inset).max(2.0)),
                                                        ),
                                                    );
                                                    win.paint_quad(
                                                        fill(rect, at_bg)
                                                            .corner_radii(rect.size.height / 2.),
                                                    );
                                                },
                                            )
                                            .absolute()
                                            .inset_0(),
                                        )
                                        .child(div().relative().child("@")),
                                )
                                .child(div().flex_1().min_w_0().truncate().child(pro_swap_label(
                                    pro_eid(&self.id, "to-summary-swap"),
                                    to_summary,
                                    12.0,
                                    FontWeight::NORMAL,
                                    p.muted,
                                )))
                                .child(
                                    div()
                                        .id(pro_eid(&self.id, "send-msg-btn"))
                                        .role(Role::Button)
                                        .aria_label(tr(cx, "Send message", "发送消息"))
                                        .tab_index(0)
                                        .size(px(28.0))
                                        .relative()
                                        .rounded_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .when(can_send, |d| d.cursor_pointer())
                                        .focus_visible(move |s| s.border_1().border_color(focus_c))
                                        .on_hover(cx.listener(|this, inside: &bool, w, cx| {
                                            this.send_hover
                                                .set(if *inside { 1.0 } else { 0.0 }, cx);
                                            if !*inside {
                                                this.send_press.set(0.0, cx);
                                            }
                                            cx.notify();
                                            w.refresh();
                                        }))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, w, cx| {
                                                this.send_press.set(1.0, cx);
                                                cx.notify();
                                                w.refresh();
                                            }),
                                        )
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _, w, cx| {
                                                this.send_press.set(0.0, cx);
                                                cx.notify();
                                                w.refresh();
                                            }),
                                        )
                                        .on_mouse_up_out(
                                            MouseButton::Left,
                                            cx.listener(|this, _, w, cx| {
                                                this.send_press.set(0.0, cx);
                                                cx.notify();
                                                w.refresh();
                                            }),
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.submit_composer(cx);
                                        }))
                                        .child(
                                            canvas(
                                                |_, _, _| (),
                                                move |bounds, _, win, _| {
                                                    let bw = f32::from(bounds.size.width);
                                                    let bh = f32::from(bounds.size.height);
                                                    let rect = Bounds::new(
                                                        point(
                                                            bounds.origin.x + px(send_inset),
                                                            bounds.origin.y + px(send_inset),
                                                        ),
                                                        size(
                                                            px((bw - 2.0 * send_inset).max(2.0)),
                                                            px((bh - 2.0 * send_inset).max(2.0)),
                                                        ),
                                                    );
                                                    win.paint_quad(
                                                        fill(rect, send_bg)
                                                            .corner_radii(rect.size.height / 2.),
                                                    );
                                                },
                                            )
                                            .absolute()
                                            .inset_0(),
                                        )
                                        .child(div().relative().child(pro_icon(
                                            ProIcon::ArrowUp,
                                            15.0 * (1.0 - 0.08 * send_down),
                                            send_fg,
                                        ))),
                                ),
                        ),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title_str)
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(top_bar)
            .child(scroll_panel)
            .child(composer_el)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{StreamAgent, active_at_query, resolve_recipients};
    use gpui::SharedString;

    #[test]
    fn mention_parser_extracts_named_agents_and_falls_back_to_reply_sender() {
        let agents = vec![
            StreamAgent::new("orch", "Orchestrator"),
            StreamAgent::new("research", "Researcher"),
            StreamAgent::new("writer", "Writer"),
        ];

        let named =
            resolve_recipients("Hey @Researcher and @Writer check Free tier", &agents, None);
        assert_eq!(
            named,
            vec![SharedString::from("research"), SharedString::from("writer")]
        );

        let reply_author = SharedString::from("writer");
        let reply_target = resolve_recipients("Use $49 / seat", &agents, Some(&reply_author));
        assert_eq!(reply_target, vec![SharedString::from("writer")]);

        let broadcast = resolve_recipients("Ship by Friday", &agents, None);
        assert_eq!(broadcast, vec![SharedString::from("all")]);

        assert_eq!(active_at_query("Hello @res"), Some("res".to_string()));
        assert_eq!(active_at_query("email@domain.com"), None);
    }
}
