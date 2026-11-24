//! Spatial single-agent workbench connecting requester, live agent, attached files, and tools.
//!
//! Renders the person who asked on the left, the active agent with its fixed three-row live work
//! log in the center, uploaded files under a macOS folder on the right, and connected tool cards
//! below—joined by rounded orthogonal connectors with hollow ports and a traveling light pulse.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, ProPresence, pro_agent_avatar, pro_app_tile, pro_btn,
        pro_check_draw, pro_connector_pulse, pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color,
        pro_person_avatar, pro_popover_presence, pro_rolling_number, pro_shimmer_text,
        pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_switch, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_raw_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const MAC_FOLDER_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 26" width="36" height="30"><defs><linearGradient id="fb" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3E9FDF"/><stop offset="1" stop-color="#2B88D2"/></linearGradient><linearGradient id="ff" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#74C9F9"/><stop offset="1" stop-color="#5BB6EE"/></linearGradient></defs><path d="M3.2 0h7.6c1.1 0 1.8.3 2.6 1l1.5 1.3c.6.5 1.2.7 2 .7h11.9A3.2 3.2 0 0 1 32 6.2v16.6a3.2 3.2 0 0 1-3.2 3.2H3.2A3.2 3.2 0 0 1 0 22.8V3.2A3.2 3.2 0 0 1 3.2 0Z" fill="url(#fb)"/><rect y="5" width="32" height="21" rx="3.2" fill="url(#ff)"/><rect x="0.6" y="5.3" width="30.8" height="0.8" rx="0.4" fill="#fff" opacity="0.28"/><rect y="21.2" width="32" height="0.7" fill="#fff" opacity="0.22"/><rect y="22.8" width="32" height="0.7" fill="#fff" opacity="0.16"/></svg>"##;

const EXCEL_FILE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32"><path d="M19.581 15.35 8.512 13.4v14.409A1.192 1.192 0 0 0 9.705 29h19.1A1.192 1.192 0 0 0 30 27.809V22.5Z" fill="#185C37"/><path d="M19.581 3H9.705A1.192 1.192 0 0 0 8.512 4.191V9.5L19.581 16l5.861 1.95L30 16V9.5Z" fill="#21A366"/><path d="M8.512 9.5h11.069V16H8.512Z" fill="#107C41"/><path d="M28.806 3h-9.225v6.5H30V4.191A1.192 1.192 0 0 0 28.806 3Z" fill="#33C481"/><path d="M19.581 16H30v6.5H19.581Z" fill="#107C41"/><rect x="2" y="8" width="16" height="16" rx="1.19" fill="#107C41"/><path d="m5.9 20.4 3.27-5.07-3-5.04h2.4l1.64 3.2c.15.3.25.53.3.68h.02c.1-.24.22-.46.34-.7l1.75-3.18h2.21l-3.07 5.01 3.15 5.1h-2.36l-1.89-3.54a2.86 2.86 0 0 1-.22-.47h-.03a2.2 2.2 0 0 1-.22.46l-1.95 3.55Z" fill="#fff"/></svg>"##;

const WORD_FILE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32"><path d="M28.806 3H9.705A1.192 1.192 0 0 0 8.512 4.191V9.5L19.581 12.75 30 9.5V4.191A1.192 1.192 0 0 0 28.806 3Z" fill="#41A5EE"/><path d="M30 9.5H8.512V16l11.069 1.95L30 16Z" fill="#2B7CD3"/><path d="M8.512 16v6.5l10.418 1.3L30 22.5V16Z" fill="#185ABD"/><path d="M9.705 29h19.1A1.192 1.192 0 0 0 30 27.809V22.5H8.512v5.309A1.192 1.192 0 0 0 9.705 29Z" fill="#103F91"/><rect x="2" y="8" width="16" height="16" rx="1.19" fill="#185ABD"/><path d="M7.05 19.1c.04.27.06.5.07.7h.03c.02-.19.05-.42.1-.69l.11-.6 1.24-5.33h1.6l1.29 5.25c.07.31.13.62.16.94h.03c.03-.31.07-.61.12-.91l1.03-5.28h1.46l-1.8 7.64h-1.71l-1.23-5.07a7.6 7.6 0 0 1-.12-.62 5.6 5.6 0 0 1-.06-.4h-.02l-.08.44c-.03.17-.07.36-.11.56l-1.15 5.09H6.39l-1.84-7.64h1.5l1.13 5.35c.03.11.05.25.07.42Z" fill="#fff"/></svg>"##;

const PDF_FILE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="32" height="32"><rect x="2" y="2" width="20" height="20" rx="5" fill="#B30B00"/><path d="M17.6 14.05c-.62-.63-2.3-.37-2.7-.32-.6-.57-1-1.27-1.15-1.52.21-.62.36-1.24.37-1.9 0-.57-.23-1.19-.87-1.19-.23 0-.45.14-.57.33-.28.48-.17 1.45.27 2.45a15.7 15.7 0 0 1-1.03 2.58c-.95.37-2.95 1.29-3.14 2.3-.06.3.04.6.27.79.23.18.5.25.78.25 1.13 0 2.26-1.55 3.03-2.89.65-.21 1.67-.52 2.69-.7 1.2 1.06 2.25 1.23 2.79 1.23.73 0 1-.32 1.08-.53.15-.37.03-.78-.1-.88Zm-.75.52c-.04.22-.26.37-.6.37-.08 0-.17-.01-.26-.04-.61-.15-1.18-.45-1.67-.85.41-.06 1.34-.15 2.01 0 .26.07.53.23.52.52Zm-4.22-5.13c.04-.07.12-.11.2-.11.21 0 .25.25.25.46-.02.48-.11.95-.27 1.41-.35-.92-.28-1.56-.18-1.76Zm-.04 4.39c.25-.48.6-1.37.73-1.74.28.47.75 1.02 1 1.27 0 .02-.98.21-1.73.47Zm-1.84 1.24c-.72 1.18-1.46 1.93-1.86 1.93a.38.38 0 0 1-.21-.06.32.32 0 0 1-.12-.28c.05-.43.88-1.05 2.19-1.59Z" fill="#fff"/></svg>"##;

fn file_icon(name: &str, side: f32, muted: Hsla) -> AnyElement {
    let lower = name.trim().to_ascii_lowercase();
    if lower.ends_with(".xlsx") || lower.ends_with(".xls") || lower.ends_with(".csv") {
        pro_raw_svg(EXCEL_FILE_SVG, side, side)
    } else if lower.ends_with(".docx") || lower.ends_with(".doc") {
        pro_raw_svg(WORD_FILE_SVG, side, side)
    } else if lower.ends_with(".pdf") {
        pro_raw_svg(PDF_FILE_SVG, side, side)
    } else {
        pro_icon(ProIcon::PageDoc, side, muted)
    }
}

fn spinning_arc(side: f32, stroke: f32, color: Hsla, elapsed_s: f32, spin: bool) -> AnyElement {
    let angle = if spin {
        (elapsed_s / 0.85) * std::f32::consts::TAU
    } else {
        -std::f32::consts::FRAC_PI_2
    };
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let center = bounds.center();
            let r = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) - stroke)
                * 0.5_f32;
            if r <= 1.0 {
                return;
            }
            let steps = 24usize;
            let mut ring = PathBuilder::stroke(px(stroke));
            for i in 0..=steps {
                let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
                let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    ring.move_to(pt);
                } else {
                    ring.line_to(pt);
                }
            }
            if let Ok(path) = ring.build() {
                window.paint_path(path, color.opacity(0.25));
            }
            let mut arc = PathBuilder::stroke(px(stroke));
            for i in 0..=8usize {
                let a = angle + (i as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
                let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    arc.move_to(pt);
                } else {
                    arc.line_to(pt);
                }
            }
            if let Ok(path) = arc.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn paint_hollow_port(pt: Point<Pixels>, active: bool, card_bg: Hsla, blue: Hsla, w: &mut Window) {
    let r = 3.25_f32;
    let d = r * 2.0;
    let rect = Bounds::new(point(pt.x - px(r), pt.y - px(r)), size(px(d), px(d)));
    let stroke_col = if active { blue } else { blue.opacity(0.42) };
    w.paint_quad(quad(
        rect,
        Corners::all(px(r)),
        card_bg,
        Edges::all(px(1.5)),
        stroke_col,
        BorderStyle::Solid,
    ));
}

/// Key-value detail line displayed in the info popover of an agent, file, or tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchFact {
    /// Muted fact label on the left (for example `"Account"` or `"Scope"`).
    pub label: SharedString,
    /// Primary fact value aligned to the right.
    pub value: SharedString,
}

impl WorkbenchFact {
    /// Construct one key-value fact row.
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Requester card specification on the left side of [`AgentWorkbench`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchUser {
    /// Display name of the person who submitted the prompt.
    pub name: SharedString,
    /// Prompt or instruction message sent to the agent (clamped visually to four lines).
    pub message: SharedString,
    /// Optional presence indicator dot rendered on the avatar.
    pub presence: Option<ProPresence>,
}

impl WorkbenchUser {
    /// Construct a [`WorkbenchUser`] with `name` and prompt `message`.
    pub fn new(name: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
            presence: None,
        }
    }

    /// Attach an optional [`ProPresence`] dot to the requester's avatar.
    pub fn with_presence(mut self, presence: Option<ProPresence>) -> Self {
        self.presence = presence;
        self
    }
}

/// Central agent specification in [`AgentWorkbench`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchAgent {
    /// Deterministic seed id driving the 3x3 pixel mark.
    pub id: SharedString,
    /// Agent display name (for example `"Assistant"`).
    pub name: SharedString,
    /// Short account or workspace subtitle beneath the agent name.
    pub subtitle: SharedString,
    /// Metadata rows displayed when clicking the agent card.
    pub details: Vec<WorkbenchFact>,
}

impl WorkbenchAgent {
    /// Construct a [`WorkbenchAgent`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            subtitle: SharedString::default(),
            details: Vec::new(),
        }
    }

    /// Set the muted subtitle line below the agent name.
    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Set the metadata lines displayed in the agent's info popover.
    pub fn with_details(mut self, details: Vec<WorkbenchFact>) -> Self {
        self.details = details;
        self
    }
}

/// Uploaded attachment item listed under the macOS folder on the right of [`AgentWorkbench`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchFile {
    /// Stable file identifier (referenced by `reading` or `WorkbenchStep::file_id`).
    pub id: SharedString,
    /// File name including extension so the matching document icon renders.
    pub name: SharedString,
    /// Short size or page-count metadata (for example `"48 KB"` or `"6 pages"`).
    pub meta: SharedString,
    /// Metadata lines displayed when clicking the file row.
    pub details: Vec<WorkbenchFact>,
}

impl WorkbenchFile {
    /// Construct an uploaded [`WorkbenchFile`] entry.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        meta: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            meta: meta.into(),
            details: Vec::new(),
        }
    }

    /// Attach key-value metadata lines for the file's info card.
    pub fn with_details(mut self, details: Vec<WorkbenchFact>) -> Self {
        self.details = details;
        self
    }
}

/// Connected tool card arrayed along the bottom row of [`AgentWorkbench`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchTool {
    /// Stable tool identifier matched against `WorkbenchStep::at`.
    pub id: SharedString,
    /// Tool display title (for example `"Gmail"` or `"Google Calendar"`).
    pub name: SharedString,
    /// Brand artwork lookup key (defaults to `name`).
    pub app: SharedString,
    /// Permission and account scope subtitle (for example `"Read & send · ryan@lumen.co"`).
    pub scope: SharedString,
    /// Last completed action description when idle.
    pub last: SharedString,
    /// Relative timestamp for `last` (for example `"2h ago"`).
    pub when: SharedString,
    /// Metadata lines displayed in the tool's info card.
    pub details: Vec<WorkbenchFact>,
}

impl WorkbenchTool {
    /// Construct a [`WorkbenchTool`] card with `id`, `name`, and `scope`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        scope: impl Into<SharedString>,
    ) -> Self {
        let name_s: SharedString = name.into();
        Self {
            id: id.into(),
            app: name_s.clone(),
            name: name_s,
            scope: scope.into(),
            last: SharedString::default(),
            when: SharedString::default(),
            details: Vec::new(),
        }
    }

    /// Override the brand artwork identifier if different from `name`.
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = app.into();
        self
    }

    /// Set the last action and relative timestamp shown when this tool is not actively called.
    pub fn with_last(
        mut self,
        last: impl Into<SharedString>,
        when: impl Into<SharedString>,
    ) -> Self {
        self.last = last.into();
        self.when = when.into();
        self
    }

    /// Attach key-value metadata lines for the tool's info card.
    pub fn with_details(mut self, details: Vec<WorkbenchFact>) -> Self {
        self.details = details;
        self
    }
}

/// Single step in the agent's chronological execution log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Present-tense or completed step description.
    pub text: SharedString,
    /// Target endpoint (`"user"`, `"files"`, or a [`WorkbenchTool::id`]).
    pub at: SharedString,
    /// Optional specific [`WorkbenchFile::id`] highlighted when `at == "files"`.
    pub file_id: Option<SharedString>,
    /// Whether this step is actively running right now.
    pub running: bool,
}

impl WorkbenchStep {
    /// Construct a [`WorkbenchStep`] targeting `at`.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        at: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            at: at.into(),
            file_id: None,
            running: false,
        }
    }

    /// Highlight a specific [`WorkbenchFile::id`] while this step runs.
    pub fn with_file_id(mut self, file_id: impl Into<SharedString>) -> Self {
        self.file_id = Some(file_id.into());
        self
    }

    /// Mark whether this step is actively running.
    pub fn with_running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }
}

/// Interactive card target whose detail popover can open inside [`AgentWorkbench`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkbenchTarget {
    /// Central agent card.
    Agent,
    /// Uploaded file row by file id.
    File(SharedString),
    /// Connected tool card by tool id.
    Tool(SharedString),
}

/// Typed events emitted by [`AgentWorkbench`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentWorkbenchEvent {
    /// An agent, file, or tool detail card was opened or dismissed (`None`).
    TargetInspected(Option<WorkbenchTarget>),
    /// Live demo playback advanced to `step_id` (or completed when `step_id` is `"done"`).
    StepAdvanced { index: usize, step_id: SharedString },
    /// Live auto-cycle mode was toggled.
    LiveToggled(bool),
    /// Workbench state was reset to the start of the run.
    Reset,
}

#[derive(Clone, Default)]
struct CardSensor(Rc<Cell<Option<Bounds<Pixels>>>>);

impl CardSensor {
    fn probe(&self) -> AnyElement {
        let cell = self.0.clone();
        canvas(
            move |bounds, _, _| {
                cell.set(Some(bounds));
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }

    fn get(&self) -> Option<Bounds<Pixels>> {
        self.0.get()
    }
}

fn default_user() -> WorkbenchUser {
    WorkbenchUser::new(
        "Ryan Brooks",
        "Find 30 minutes with Marc next week and send him the Q4 budget before we meet.",
    )
}

fn default_agent() -> WorkbenchAgent {
    WorkbenchAgent::new("assistant", "Assistant")
        .with_subtitle("Works in your Google account")
        .with_details(vec![
            WorkbenchFact::new("Model", "Claude Sonnet 4.5"),
            WorkbenchFact::new("Account", "ryan@lumen.co"),
            WorkbenchFact::new("Guardrail", "Ask before external sends"),
        ])
}

fn default_files() -> Vec<WorkbenchFile> {
    vec![
        WorkbenchFile::new("budget", "budget-q4.xlsx", "48 KB").with_details(vec![
            WorkbenchFact::new("Type", "Excel workbook"),
            WorkbenchFact::new("Sheets", "Summary, Headcount, Runway"),
            WorkbenchFact::new("Updated", "Today, 09:14"),
        ]),
        WorkbenchFile::new("brief", "kickoff-brief.docx", "6 pages").with_details(vec![
            WorkbenchFact::new("Type", "Word document"),
            WorkbenchFact::new("Author", "Ryan Brooks"),
            WorkbenchFact::new("Updated", "Yesterday"),
        ]),
        WorkbenchFile::new("contract", "vendor-contract.pdf", "12 pages").with_details(vec![
            WorkbenchFact::new("Type", "PDF document"),
            WorkbenchFact::new("Status", "Signed"),
            WorkbenchFact::new("Updated", "Nov 10"),
        ]),
    ]
}

fn default_tools() -> Vec<WorkbenchTool> {
    vec![
        WorkbenchTool::new("gmail", "Gmail", "Read & send · ryan@lumen.co")
            .with_last("Sent the weekly digest", "2h ago")
            .with_details(vec![
                WorkbenchFact::new("Account", "ryan@lumen.co"),
                WorkbenchFact::new("Permissions", "Read threads & send drafts"),
                WorkbenchFact::new("Rate limit", "60 sends / hr"),
            ]),
        WorkbenchTool::new("calendar", "Google Calendar", "Read & book · Work")
            .with_last("Moved the design review", "Yesterday")
            .with_details(vec![
                WorkbenchFact::new("Calendar", "Work · America/New_York"),
                WorkbenchFact::new("Permissions", "Free/busy & hold slots"),
                WorkbenchFact::new("Default duration", "30 minutes"),
            ]),
        WorkbenchTool::new("drive", "Google Drive", "Read · Q4 planning")
            .with_last("Found the Q3 report", "Mon")
            .with_details(vec![
                WorkbenchFact::new("Folder", "Lumen / Q4 planning"),
                WorkbenchFact::new("Permissions", "Read-only access"),
                WorkbenchFact::new("Indexed", "142 files"),
            ]),
    ]
}

fn default_playbook() -> Vec<WorkbenchStep> {
    vec![
        WorkbenchStep::new("s1", "Reading Ryan's request", "user"),
        WorkbenchStep::new("s2", "Reading budget-q4.xlsx", "files").with_file_id("budget"),
        WorkbenchStep::new("s3", "Checking Marc's calendar next week", "calendar"),
        WorkbenchStep::new("s4", "Verifying Q4 folder permissions", "drive"),
        WorkbenchStep::new("s5", "Sending Q4 budget & Tuesday 2pm invite", "gmail"),
    ]
}

/// Spatial single-agent workbench with live step log, macOS files card, tool cards, and pulsing wires.
pub struct AgentWorkbench {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    user: WorkbenchUser,
    agent: WorkbenchAgent,
    files: Vec<WorkbenchFile>,
    tools: Vec<WorkbenchTool>,
    custom_steps: Option<Vec<WorkbenchStep>>,
    playbook: Vec<WorkbenchStep>,
    demo_phase: usize,
    phase_started: Instant,
    boot_at: Instant,
    reading_override: Option<SharedString>,
    done_override: Option<bool>,
    live: bool,
    hovered_link: Option<SharedString>,
    inspected: Option<WorkbenchTarget>,
    last_target: Option<WorkbenchTarget>,
    measured_width: Rc<Cell<f32>>,
    user_sensor: CardSensor,
    agent_sensor: CardSensor,
    files_sensor: CardSensor,
    tool_sensors: [CardSensor; 4],
    loading: bool,
    empty: bool,
}

impl AgentWorkbench {
    /// Create an [`AgentWorkbench`] preloaded with Ryan Brooks' scheduling + Q4 budget request.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Agent workbench".into(),
            user: default_user(),
            agent: default_agent(),
            files: default_files(),
            tools: default_tools(),
            custom_steps: None,
            playbook: default_playbook(),
            // Start on step index 1 ("Reading budget-q4.xlsx" running, "Reading Ryan's request" done)
            // matching the canonical KUI Pro stage snapshot.
            demo_phase: 1,
            phase_started: now,
            boot_at: now,
            reading_override: None,
            done_override: None,
            live: true,
            hovered_link: None,
            inspected: None,
            last_target: None,
            measured_width: Rc::new(Cell::new(804.0)),
            user_sensor: CardSensor::default(),
            agent_sensor: CardSensor::default(),
            files_sensor: CardSensor::default(),
            tool_sensors: std::array::from_fn(|_| CardSensor::default()),
            loading: false,
            empty: false,
        }
    }

    /// Override the accessible section name.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the requester card on the left.
    pub fn with_user(mut self, user: WorkbenchUser) -> Self {
        self.user = user;
        self
    }

    /// Set the central agent card metadata.
    pub fn with_agent(mut self, agent: WorkbenchAgent) -> Self {
        self.agent = agent;
        self
    }

    /// Set the uploaded files listed under the macOS folder on the right.
    pub fn with_files(mut self, files: Vec<WorkbenchFile>) -> Self {
        self.files = files;
        self
    }

    /// Set the connected tool cards arrayed below the agent.
    pub fn with_tools(mut self, tools: Vec<WorkbenchTool>) -> Self {
        self.tools = tools;
        self
    }

    /// Provide an explicit static or controlled step sequence (disables default playbook cycling unless `with_live(true)` is set).
    pub fn with_steps(mut self, steps: Vec<WorkbenchStep>) -> Self {
        self.playbook = steps.clone();
        self.custom_steps = Some(steps);
        self.live = false;
        self
    }

    /// Explicitly highlight one file id as currently being read.
    pub fn with_reading(mut self, file_id: Option<SharedString>) -> Self {
        self.reading_override = file_id;
        self
    }

    /// Explicitly mark the workbench run as finished (`Done`).
    pub fn with_done(mut self, done: bool) -> Self {
        self.done_override = Some(done);
        if done {
            self.live = false;
        }
        self
    }

    /// Enable or disable live step progression along the connectors.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self.phase_started = Instant::now();
        self
    }

    /// Toggle skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state presentation.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Currently open info-card target, if any.
    pub fn inspected_target(&self) -> Option<&WorkbenchTarget> {
        self.inspected.as_ref()
    }

    /// Whether the run is currently in the `Done` state.
    pub fn is_done(&self) -> bool {
        if let Some(d) = self.done_override {
            return d;
        }
        if let Some(steps) = &self.custom_steps {
            return !steps.is_empty() && steps.iter().all(|s| !s.running);
        }
        self.demo_phase >= self.playbook.len()
    }

    /// Chronological steps up to the current execution point (oldest first).
    pub fn active_steps(&self) -> Vec<WorkbenchStep> {
        if let Some(steps) = &self.custom_steps {
            let done = self.done_override.unwrap_or(false);
            return steps
                .iter()
                .cloned()
                .map(|mut s| {
                    if done {
                        s.running = false;
                    }
                    s
                })
                .collect();
        }
        if self.playbook.is_empty() {
            return Vec::new();
        }
        let total = self.playbook.len();
        let done = self.is_done();
        let upto = if done {
            total
        } else {
            (self.demo_phase + 1).min(total)
        };
        self.playbook
            .iter()
            .take(upto)
            .enumerate()
            .map(|(idx, s)| {
                let mut copy = s.clone();
                copy.running = !done && idx == self.demo_phase;
                copy
            })
            .collect()
    }

    /// Up to three most recent steps shown in the agent's work area (newest first).
    pub fn visible_steps(&self) -> Vec<WorkbenchStep> {
        let mut v = self.active_steps();
        v.reverse();
        v.truncate(3);
        v
    }

    /// Open or close (`None`) a card's detail popover.
    pub fn inspect(&mut self, target: Option<WorkbenchTarget>, cx: &mut Context<Self>) {
        let next = if self.inspected == target {
            None
        } else {
            target
        };
        if let Some(t) = &next {
            self.last_target = Some(t.clone());
        }
        self.inspected = next.clone();
        cx.emit(AgentWorkbenchEvent::TargetInspected(next));
        cx.notify();
    }

    /// Advance one step along the demo playbook (wrapping back after `Done`).
    pub fn advance_demo_step(&mut self, cx: &mut Context<Self>) {
        if self.playbook.is_empty() {
            return;
        }
        self.custom_steps = None;
        self.done_override = None;
        let cycle_len = self.playbook.len() + 1;
        self.demo_phase = (self.demo_phase + 1) % cycle_len;
        self.phase_started = Instant::now();
        let step_id: SharedString = if self.demo_phase < self.playbook.len() {
            self.playbook[self.demo_phase].id.clone()
        } else {
            "done".into()
        };
        cx.emit(AgentWorkbenchEvent::StepAdvanced {
            index: self.demo_phase,
            step_id,
        });
        cx.notify();
    }

    /// Toggle live playback on or off.
    pub fn set_live(&mut self, live: bool, cx: &mut Context<Self>) {
        if self.live != live {
            self.live = live;
            self.phase_started = Instant::now();
            cx.emit(AgentWorkbenchEvent::LiveToggled(live));
            cx.notify();
        }
    }

    /// Reset the workbench to step 1 and close any open detail popover.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.demo_phase = if self.playbook.len() > 1 { 1 } else { 0 };
        self.phase_started = Instant::now();
        self.inspected = None;
        self.done_override = None;
        cx.emit(AgentWorkbenchEvent::Reset);
        cx.notify();
    }

    fn localize_default<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Assistant" => tr(cx, "Assistant", "智能助手"),
            "Works in your Google account" => {
                tr(cx, "Works in your Google account", "运行于你的 Google 账户")
            }
            "Find 30 minutes with Marc next week and send him the Q4 budget before we meet." => tr(
                cx,
                "Find 30 minutes with Marc next week and send him the Q4 budget before we meet.",
                "下周帮我约 Marc 聊 30 分钟，并在会前把 Q4 预算表发给他。",
            ),
            "Reading Ryan's request" => tr(cx, "Reading Ryan's request", "正在阅读 Ryan 的请求"),
            "Reading budget-q4.xlsx" => tr(cx, "Reading budget-q4.xlsx", "正在读取 budget-q4.xlsx"),
            "Checking Marc's calendar next week" => tr(
                cx,
                "Checking Marc's calendar next week",
                "正在查询 Marc 下周的日历空档",
            ),
            "Verifying Q4 folder permissions" => tr(
                cx,
                "Verifying Q4 folder permissions",
                "正在核对 Q4 文件夹访问权限",
            ),
            "Sending Q4 budget & Tuesday 2pm invite" => tr(
                cx,
                "Sending Q4 budget & Tuesday 2pm invite",
                "正在发送 Q4 预算与周二 14:00 邀请",
            ),
            "Read & send · ryan@lumen.co" => tr(
                cx,
                "Read & send · ryan@lumen.co",
                "读取与发送 · ryan@lumen.co",
            ),
            "Read & book · Work" => tr(cx, "Read & book · Work", "读取与预订 · 工作日历"),
            "Read · Q4 planning" => tr(cx, "Read · Q4 planning", "只读 · Q4 规划目录"),
            "Sent the weekly digest" => tr(cx, "Sent the weekly digest", "已发送每周摘要邮件"),
            "Moved the design review" => tr(cx, "Moved the design review", "已调整设计评审时间"),
            "Found the Q3 report" => tr(cx, "Found the Q3 report", "已找到 Q3 季度报告"),
            "2h ago" => tr(cx, "2h ago", "2 小时前"),
            "Yesterday" => tr(cx, "Yesterday", "昨天"),
            "Mon" => tr(cx, "Mon", "周一"),
            "6 pages" => tr(cx, "6 pages", "6 页"),
            "12 pages" => tr(cx, "12 pages", "12 页"),
            other => other,
        }
    }

    fn render_popover(
        &self,
        target: &WorkbenchTarget,
        amount: f32,
        steps: &[WorkbenchStep],
        done: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pop_bg: Hsla = if p.dark {
            rgb(0x232326).into()
        } else {
            rgb(0xffffff).into()
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.03)
        };
        let edge_col: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.08)
        };
        let blue_fg: Hsla = if p.dark {
            rgb(0x8ab4ff).into()
        } else {
            rgb(0x0060e2).into()
        };
        let blue_bg: Hsla = if p.dark {
            hsla(218. / 360., 1.0, 0.65, 0.12)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.07)
        };
        let ok_fg = p.green_ink;
        let ok_bg = p.green_soft;

        let (head_icon, title, subtitle, badge_txt, badge_bg, badge_fg, facts, matching_steps): (
            AnyElement,
            SharedString,
            SharedString,
            SharedString,
            Hsla,
            Hsla,
            Vec<WorkbenchFact>,
            Vec<WorkbenchStep>,
        ) = match target {
            WorkbenchTarget::Agent => {
                let working = !done && steps.iter().any(|s| s.running);
                let mark_col: Hsla = if p.dark {
                    rgba(0x4d8dffda).into()
                } else {
                    rgba(0x0060e2c7).into()
                };
                let ic = pro_agent_avatar(
                    &self.agent.id,
                    mark_col,
                    26.,
                    working,
                    self.boot_at.elapsed().as_secs_f32() * 1000.0,
                    cx,
                );
                let (st_lbl, bg, fg) = if done {
                    (tr(cx, "Done", "已完成").into(), ok_bg, ok_fg)
                } else if working {
                    (tr(cx, "Working", "运行中").into(), blue_bg, blue_fg)
                } else {
                    (tr(cx, "Ready", "就绪").into(), p.chip, p.text)
                };
                let mut rev = steps.to_vec();
                rev.reverse();
                (
                    ic,
                    self.localize_default(&self.agent.name, cx)
                        .to_owned()
                        .into(),
                    self.localize_default(&self.agent.subtitle, cx)
                        .to_owned()
                        .into(),
                    st_lbl,
                    bg,
                    fg,
                    self.agent.details.clone(),
                    rev,
                )
            }
            WorkbenchTarget::File(fid) => {
                let file = self
                    .files
                    .iter()
                    .find(|f| &f.id == fid)
                    .cloned()
                    .unwrap_or_else(|| WorkbenchFile::new(fid.clone(), fid.clone(), ""));
                let reading_now = self.reading_override.as_ref() == Some(fid)
                    || steps
                        .iter()
                        .any(|s| s.running && s.file_id.as_ref() == Some(fid));
                let used = steps
                    .iter()
                    .any(|s| !s.running && (s.file_id.as_ref() == Some(fid) || s.at == "files"));
                let (st_lbl, bg, fg) = if reading_now {
                    (tr(cx, "Reading", "读取中").into(), blue_bg, blue_fg)
                } else if used {
                    (tr(cx, "Read", "已读取").into(), ok_bg, ok_fg)
                } else {
                    (tr(cx, "Attached", "已附加").into(), p.chip, p.text)
                };
                let rel: Vec<WorkbenchStep> = steps
                    .iter()
                    .filter(|s| s.file_id.as_ref() == Some(fid) || s.at == "files")
                    .cloned()
                    .collect();
                (
                    div()
                        .size(px(26.))
                        .rounded(px(7.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(file_icon(&file.name, 15., p.muted))
                        .into_any_element(),
                    file.name.clone(),
                    self.localize_default(&file.meta, cx).to_owned().into(),
                    st_lbl,
                    bg,
                    fg,
                    file.details,
                    rel,
                )
            }
            WorkbenchTarget::Tool(tid) => {
                let tool = self
                    .tools
                    .iter()
                    .find(|t| &t.id == tid)
                    .cloned()
                    .unwrap_or_else(|| WorkbenchTool::new(tid.clone(), tid.clone(), ""));
                let calling = !done && steps.iter().any(|s| s.running && &s.at == tid);
                let used = steps.iter().any(|s| !s.running && &s.at == tid);
                let (st_lbl, bg, fg) = if calling {
                    (tr(cx, "Calling", "调用中").into(), blue_bg, blue_fg)
                } else if used {
                    (tr(cx, "Used", "已使用").into(), ok_bg, ok_fg)
                } else {
                    (tr(cx, "Ready", "就绪").into(), p.chip, p.text)
                };
                let rel: Vec<WorkbenchStep> =
                    steps.iter().filter(|s| &s.at == tid).cloned().collect();
                (
                    pro_app_tile(&tool.app, 26., cx),
                    tool.name.clone(),
                    self.localize_default(&tool.scope, cx).to_owned().into(),
                    st_lbl,
                    bg,
                    fg,
                    tool.details,
                    rel,
                )
            }
        };

        let mut facts_box = div()
            .w_full()
            .px(px(10.))
            .py(px(4.))
            .rounded(px(8.))
            .bg(fill_bg)
            .border_1()
            .border_color(edge_col)
            .flex()
            .flex_col();
        for f in &facts {
            facts_box = facts_box.child(
                div()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .text_size(px(12.))
                    .child(div().flex_none().text_color(p.muted).child(f.label.clone()))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(p.ink)
                            .child(f.value.clone()),
                    ),
            );
        }

        let mut run_list = div().flex().flex_col().gap(px(2.));
        run_list = run_list.child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(tr(cx, "For this request", "本次请求记录")),
        );
        if matching_steps.is_empty() {
            run_list = run_list.child(
                div()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "Not touched yet in this run", "本次运行尚未调用")),
            );
        } else {
            for s in matching_steps.iter().take(4) {
                let is_live = s.running && !done;
                run_list = run_list.child(
                    div()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(12.))
                        .text_color(if is_live { p.ink } else { p.text })
                        .child(if is_live {
                            spinning_arc(
                                11.,
                                1.5,
                                blue_fg,
                                self.boot_at.elapsed().as_secs_f32(),
                                !(disabled_motion(cx) || Theme::global(cx).reduced_motion),
                            )
                        } else {
                            pro_check_draw(
                                pro_eid(&self.id, format!("wb-pop-chk-{}", s.id)),
                                true,
                                12.0,
                                false,
                                ok_fg,
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(self.localize_default(&s.text, cx).to_owned()),
                        ),
                );
            }
        }

        let is_above = matches!(target, WorkbenchTarget::Tool(_));
        let dy = if is_above {
            4.0 * (1.0 - amount)
        } else {
            -4.0 * (1.0 - amount)
        };

        div()
            .id(pro_eid(&self.id, "info-pop"))
            .role(Role::Group)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .relative()
            .top(px(dy))
            .opacity(amount)
            .w_full()
            .p(px(12.))
            .rounded(px(12.))
            .bg(pop_bg)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .min_w_0()
                    .child(head_icon)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(title),
                            )
                            .when(!subtitle.is_empty(), |d| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(subtitle),
                                )
                            }),
                    )
                    .child(
                        div()
                            .h(px(18.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(badge_bg)
                            .text_color(badge_fg)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(badge_txt),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "pop-close"),
                        tr(cx, "Close details", "关闭详情"),
                        ProIcon::Close,
                        24.,
                        false,
                        cx,
                        |this, _, cx| this.inspect(None, cx),
                    )),
            )
            .when(!facts.is_empty(), |d| d.child(facts_box))
            .child(run_list)
            .into_any_element()
    }
}

impl EventEmitter<AgentWorkbenchEvent> for AgentWorkbench {}

impl Focusable for AgentWorkbench {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AgentWorkbench {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;

        if self.loading {
            let phase = self.boot_at.elapsed().as_secs_f32() * 0.9;
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .flex_col()
                .gap(px(28.))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(24.))
                        .child(pro_skeleton_bar(200., 148., 12., phase, cx).flex_1())
                        .child(pro_skeleton_bar(280., 148., 12., phase + 0.15, cx).flex_1())
                        .child(pro_skeleton_bar(220., 148., 12., phase + 0.30, cx).flex_1()),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(16.))
                        .child(pro_skeleton_bar(240., 84., 12., phase + 0.1, cx).flex_1())
                        .child(pro_skeleton_bar(240., 84., 12., phase + 0.25, cx).flex_1())
                        .child(pro_skeleton_bar(240., 84., 12., phase + 0.4, cx).flex_1()),
                )
                .into_any_element();
        }

        if self.empty || (self.tools.is_empty() && self.files.is_empty()) {
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No active workbench", "暂无活动工作台"),
                    tr(
                        cx,
                        "Attach files or connect tools to see this agent's live request flow.",
                        "附加文件或连接工具后即可查看智能体的实时工作轨迹。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-demo"),
                        tr(cx, "Load sample run", "加载示例运行").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.files = default_files();
                            this.tools = default_tools();
                            this.reset(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        // Auto-advance demo steps smoothly when live and no popover is open.
        let phase_elapsed = self.phase_started.elapsed().as_secs_f32();
        let step_period = 2.65_f32;
        if self.live && !reduced && self.custom_steps.is_none() && self.inspected.is_none() {
            if phase_elapsed >= step_period {
                self.advance_demo_step(cx);
            }
            w.request_animation_frame();
        } else if !self.is_done() && !reduced {
            w.request_animation_frame();
        }

        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let pulse_t = if reduced {
            None
        } else {
            Some(((elapsed_s / 1.35).fract()).clamp(0.0, 1.0))
        };

        let width_cell = self.measured_width.clone();
        let container_w = width_cell.get();
        let stacked = container_w < 720.0;

        let done = self.is_done();
        let steps = self.active_steps();
        let visible_steps = self.visible_steps();
        let running_step = if done {
            None
        } else {
            steps.iter().find(|s| s.running).cloned()
        };
        let active_at = running_step.as_ref().map(|s| s.at.clone());
        let active_file_id = self.reading_override.clone().or_else(|| {
            running_step.as_ref().and_then(|s| {
                if s.at == "files" {
                    s.file_id
                        .clone()
                        .or_else(|| self.files.first().map(|f| f.id.clone()))
                } else {
                    None
                }
            })
        });

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.03)
        };
        let edge_col: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.08)
        };
        let tag_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.07)
        } else {
            hsla(0., 0., 0., 0.05)
        };
        let blue: Hsla = if p.dark {
            rgba(0x4d8dffda).into()
        } else {
            rgba(0x0060e2c7).into()
        };
        let blue_fg: Hsla = if p.dark {
            rgb(0x8ab4ff).into()
        } else {
            rgb(0x0060e2).into()
        };
        let blue_bg: Hsla = if p.dark {
            hsla(218. / 360., 1.0, 0.65, 0.12)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.065)
        };
        let wire_idle: Hsla = if p.dark {
            rgba(0x4d8dff47).into()
        } else {
            rgba(0x0060e238).into()
        };
        let wire_active = pro_mix_color(wire_idle, blue, 0.62);
        let ok_fg = p.green_ink;
        let ok_bg = p.green_soft;

        let hov_key = self.hovered_link.as_deref();
        let insp_is_agent = matches!(self.inspected, Some(WorkbenchTarget::Agent));
        let insp_is_files = matches!(self.inspected, Some(WorkbenchTarget::File(_)));
        let any_focus = hov_key.is_some() || self.inspected.is_some();
        let any_spot = pro_spring_f32(
            pro_eid(&self.id, "wb-any-spot"),
            if any_focus { 1.0 } else { 0.0 },
            0.22,
            w,
            cx,
        );

        let u_lit = active_at.as_deref() == Some("user")
            || hov_key == Some("user")
            || hov_key == Some("agent")
            || insp_is_agent;
        let u_spot = pro_spring_f32(
            pro_eid(&self.id, "wb-u-spot"),
            if u_lit { 1.0 } else { 0.0 },
            0.24,
            w,
            cx,
        );

        let f_lit = active_at.as_deref() == Some("files")
            || hov_key == Some("files")
            || hov_key == Some("agent")
            || insp_is_files
            || insp_is_agent;
        let f_spot = pro_spring_f32(
            pro_eid(&self.id, "wb-f-spot"),
            if f_lit { 1.0 } else { 0.0 },
            0.24,
            w,
            cx,
        );

        // 1. User Card (Left)
        let user_msg = self.localize_default(&self.user.message, cx).to_owned();
        let user_card = div()
            .id(pro_eid(&self.id, "user-card"))
            .relative()
            .when(!stacked, |d| d.flex_1().min_w_0())
            .when(stacked, |d| d.w_full())
            .pt(px(12.))
            .px(px(14.))
            .pb(px(14.))
            .rounded(px(12.))
            .bg(card_bg)
            .border_1()
            .border_color(pro_mix_color(p.line, blue.opacity(0.55), u_spot))
            .when(!p.dark, |d| d.shadow_xs())
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_link = Some("user".into());
                    cx.notify();
                } else if this.hovered_link.as_deref() == Some("user") {
                    this.hovered_link = None;
                    cx.notify();
                }
            }))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(self.user_sensor.probe())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(pro_person_avatar(
                        &self.user.name,
                        26.,
                        false,
                        self.user.presence,
                        cx,
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(self.user.name.clone()),
                    ),
            )
            .child(
                div()
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(10.))
                    .rounded_tl(px(4.))
                    .bg(fill_bg)
                    .border_1()
                    .border_color(edge_col)
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.ink)
                    .line_clamp(4)
                    .child(user_msg),
            );

        // 2. Central Agent Card
        let is_working = running_step.is_some() && !done;
        let agent_open = matches!(self.inspected, Some(WorkbenchTarget::Agent));
        let agent_aria: SharedString = format!(
            "{} – {}",
            self.localize_default(&self.agent.name, cx),
            tr(cx, "details", "详情")
        )
        .into();

        let mut work_box = div()
            .id(pro_eid(&self.id, "work-log"))
            .role(Role::Status)
            .w_full()
            .px(px(10.))
            .py(px(6.))
            .rounded(px(8.))
            .bg(fill_bg)
            .border_1()
            .border_color(edge_col)
            .flex()
            .flex_col();

        for slot in 0..3usize {
            if steps.is_empty() && slot == 0 {
                work_box = work_box.child(
                    div()
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(tr(cx, "Waiting for a request", "等待新请求")),
                );
            } else if let Some(st) = visible_steps.get(slot) {
                let row_live = st.running && !done;
                let step_label = self.localize_default(&st.text, cx).to_owned();
                let trailing: Option<AnyElement> = if st.at == "files" {
                    Some(pro_raw_svg(MAC_FOLDER_SVG, 13., 11.))
                } else if st.at == "user" {
                    None
                } else if let Some(tool) = self.tools.iter().find(|t| t.id == st.at) {
                    pro_app_artwork(&tool.app, 13., cx)
                } else {
                    None
                };

                work_box = work_box.child(
                    div()
                        .h(px(24.))
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(16.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if row_live {
                                    spinning_arc(12., 1.6, blue, elapsed_s, !reduced)
                                } else {
                                    pro_check_draw(
                                        pro_eid(&self.id, format!("wb-step-chk-{}", st.id)),
                                        true,
                                        12.0,
                                        false,
                                        ok_fg,
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .child(if row_live {
                                    pro_shimmer_text(
                                        pro_eid(&self.id, format!("step-shim-{}", st.id)),
                                        step_label,
                                        p.ink,
                                        p.strong,
                                        13.,
                                        FontWeight::NORMAL,
                                        true,
                                    )
                                } else {
                                    div()
                                        .text_size(px(13.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(step_label)
                                        .into_any_element()
                                }),
                        )
                        .when_some(trailing, |d, tr_ic| {
                            d.child(
                                div()
                                    .size(px(14.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(tr_ic),
                            )
                        }),
                );
            } else {
                work_box = work_box.child(div().h(px(24.)));
            }
        }

        let agent_pop_rendered = if agent_open
            || matches!(self.last_target, Some(WorkbenchTarget::Agent))
        {
            let content = self.render_popover(&WorkbenchTarget::Agent, 1.0, &steps, done, &p, cx);
            let mut cell = Some(content);
            Some(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(relative(1.0))
                    .pt(px(8.))
                    .child(pro_popover_presence(
                        pro_eid(&self.id, "agent-pop-layer"),
                        agent_open,
                        move || cell.take().unwrap_or_else(|| div().into_any_element()),
                    )),
            )
        } else {
            None
        };

        let hover_border = p.line_strong;
        let base_border = if agent_open {
            blue.opacity(0.55)
        } else {
            p.line
        };
        let agent_status_pill = if done {
            div()
                .h(px(22.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(ok_bg)
                .flex()
                .items_center()
                .gap(px(5.))
                .child(pro_check_draw(
                    pro_eid(&self.id, "wb-done-chk"),
                    true,
                    12.0,
                    false,
                    ok_fg,
                ))
                .child(pro_swap_label(
                    pro_eid(&self.id, "wb-agent-st"),
                    tr(cx, "Done", "已完成"),
                    12.,
                    FontWeight::MEDIUM,
                    ok_fg,
                ))
        } else if is_working {
            div()
                .h(px(22.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(blue_bg)
                .flex()
                .items_center()
                .gap(px(6.))
                .child(spinning_arc(12., 1.7, blue_fg, elapsed_s, !reduced))
                .child(pro_swap_label(
                    pro_eid(&self.id, "wb-agent-st"),
                    tr(cx, "Working", "运行中"),
                    12.,
                    FontWeight::MEDIUM,
                    blue_fg,
                ))
        } else {
            div()
                .h(px(22.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(tag_bg)
                .flex()
                .items_center()
                .child(pro_swap_label(
                    pro_eid(&self.id, "wb-agent-st"),
                    tr(cx, "Ready", "就绪"),
                    12.,
                    FontWeight::MEDIUM,
                    p.text,
                ))
        };

        let agent_hit = pro_tactile(
            pro_eid(&self.id, "agent-hit"),
            agent_aria,
            12.0,
            card_bg,
            pro_mix_color(card_bg, p.ink, 0.02),
            base_border,
            hover_border,
            agent_open,
            div()
                .w_full()
                .pt(px(12.))
                .px(px(14.))
                .pb(px(14.))
                .relative()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(self.agent_sensor.probe())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .min_w_0()
                        .child(pro_agent_avatar(
                            &self.agent.id,
                            blue,
                            26.,
                            is_working,
                            elapsed_s * 1000.0,
                            cx,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(16.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(
                                            self.localize_default(&self.agent.name, cx).to_owned(),
                                        ),
                                )
                                .when(!self.agent.subtitle.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(
                                                self.localize_default(&self.agent.subtitle, cx)
                                                    .to_owned(),
                                            ),
                                    )
                                }),
                        )
                        .child(agent_status_pill),
                )
                .child(work_box),
            cx,
            |this, _, cx| {
                this.inspect(Some(WorkbenchTarget::Agent), cx);
            },
        )
        .aria_expanded(agent_open)
        .when(!p.dark, |d| d.shadow_xs())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_hover(cx.listener(|this, inside: &bool, _, cx| {
            if *inside {
                this.hovered_link = Some("agent".into());
                cx.notify();
            } else if this.hovered_link.as_deref() == Some("agent") {
                this.hovered_link = None;
                cx.notify();
            }
        }));

        let agent_wrap = div()
            .relative()
            .when(!stacked, |d| {
                d.flex_basis(relative(0.)).flex_grow(1.42).min_w_0()
            })
            .when(stacked, |d| d.w_full())
            .child(agent_hit)
            .when_some(agent_pop_rendered, |d, pop| d.child(pop));

        // 3. Files Card (Right)
        let max_file_rows = if self.files.len() > 4 {
            3
        } else {
            self.files.len()
        };
        let extra_files = self.files.len().saturating_sub(max_file_rows);
        let mut file_rows = div().flex().flex_col().gap(px(2.));
        let mut file_pop_rendered: Option<AnyElement> = None;

        for file in self.files.iter().take(max_file_rows) {
            let fid = file.id.clone();
            let is_reading = active_file_id.as_ref() == Some(&fid);
            let file_used = steps
                .iter()
                .any(|s| !s.running && s.file_id.as_ref() == Some(&fid));
            let is_open = self.inspected == Some(WorkbenchTarget::File(fid.clone()));
            if is_open || self.last_target == Some(WorkbenchTarget::File(fid.clone())) {
                let content = self.render_popover(
                    &WorkbenchTarget::File(fid.clone()),
                    1.0,
                    &steps,
                    done,
                    &p,
                    cx,
                );
                let mut cell = Some(content);
                file_pop_rendered = Some(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(relative(1.0))
                        .pt(px(8.))
                        .child(pro_popover_presence(
                            pro_eid(&self.id, format!("fpop-{fid}")),
                            is_open,
                            move || cell.take().unwrap_or_else(|| div().into_any_element()),
                        ))
                        .into_any_element(),
                );
            }
            let row_bg = if is_reading {
                blue_bg
            } else if is_open {
                fill_bg
            } else {
                transparent_black()
            };
            let row_hover = if is_reading { blue_bg } else { fill_bg };
            let file_aria: SharedString =
                format!("{} – {}", file.name, tr(cx, "details", "详情")).into();
            let fid_click = fid.clone();
            let row_el = pro_tactile(
                pro_eid(&self.id, format!("file-{fid}")),
                file_aria,
                6.0,
                row_bg,
                row_hover,
                transparent_black(),
                if is_reading {
                    blue.opacity(0.35)
                } else {
                    edge_col
                },
                is_open,
                div()
                    .w_full()
                    .h(px(28.))
                    .px(px(5.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(
                        div()
                            .size(px(16.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(file_icon(&file.name, 15., p.muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .text_color(if is_reading { blue_fg } else { p.ink })
                            .font_weight(if is_reading {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(file.name.clone()),
                    )
                    .when(file_used && !is_reading, |d| {
                        d.child(pro_check_draw(
                            pro_eid(&self.id, format!("f-used-chk-{fid}")),
                            true,
                            12.0,
                            false,
                            ok_fg,
                        ))
                    })
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(if is_reading { blue_fg } else { p.muted })
                            .child(self.localize_default(&file.meta, cx).to_owned()),
                    ),
                cx,
                move |this, _, cx| {
                    this.inspect(Some(WorkbenchTarget::File(fid_click.clone())), cx);
                },
            )
            .aria_expanded(is_open)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
            file_rows = file_rows.child(row_el);
        }
        if extra_files > 0 {
            let more_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("另外 {extra_files} 个文件")
            } else {
                format!("+{extra_files} more")
            };
            file_rows = file_rows.child(
                div()
                    .h(px(24.))
                    .pl(px(28.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(more_txt),
            );
        }

        let files_count_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("{} 个文件", self.files.len())
        } else if self.files.len() == 1 {
            "1 file".to_owned()
        } else {
            format!("{} files", self.files.len())
        };

        let files_card = div()
            .id(pro_eid(&self.id, "files-card"))
            .relative()
            .when(!stacked, |d| {
                d.flex_basis(relative(0.)).flex_grow(1.08).min_w_0()
            })
            .when(stacked, |d| d.w_full())
            .pt(px(12.))
            .px(px(10.))
            .pb(px(10.))
            .rounded(px(12.))
            .bg(card_bg)
            .border_1()
            .border_color(pro_mix_color(p.line, blue.opacity(0.50), f_spot))
            .when(!p.dark, |d| d.shadow_xs())
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_link = Some("files".into());
                    cx.notify();
                } else if this.hovered_link.as_deref() == Some("files") {
                    this.hovered_link = None;
                    cx.notify();
                }
            }))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(self.files_sensor.probe())
            .child(
                div()
                    .px(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .min_w_0()
                    .child(
                        div()
                            .size(px(30.))
                            .flex_none()
                            .rounded(px(8.))
                            .bg(if p.dark { rgb(0x242428) } else { rgb(0xffffff) })
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_raw_svg(MAC_FOLDER_SVG, 18., 15.)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(tr(cx, "Uploaded files", "已上传文件")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .child(files_count_txt),
                            ),
                    ),
            )
            .child(file_rows)
            .when_some(file_pop_rendered, |d, pop| d.child(pop));

        // 4. Tools Row (Bottom)
        let has_overflow_tools = self.tools.len() > 4;
        let shown_tool_count = if has_overflow_tools {
            3
        } else {
            self.tools.len()
        };
        let total_cards = shown_tool_count + usize::from(has_overflow_tools);
        for s in self.tool_sensors.iter().skip(total_cards) {
            s.0.set(None);
        }

        let mut t_spots = [0.0_f32; 4];
        let mut tools_row = div()
            .w_full()
            .flex()
            .when(stacked, |d| d.flex_col().gap(px(12.)))
            .when(!stacked, |d| {
                d.items_stretch().justify_center().gap(px(16.))
            });

        for (idx, tool) in self.tools.iter().take(shown_tool_count).enumerate() {
            let tid = tool.id.clone();
            let calling = !done && active_at.as_ref() == Some(&tid);
            let used = steps.iter().any(|s| !s.running && s.at == tid);
            let is_open = self.inspected == Some(WorkbenchTarget::Tool(tid.clone()));
            let is_hov = hov_key == Some(tid.as_ref()) || hov_key == Some("agent") || insp_is_agent;
            let slot_lit = calling || is_open || is_hov;
            let t_spot = pro_spring_f32(
                pro_eid(&self.id, format!("wb-t-spot-{tid}")),
                if slot_lit { 1.0 } else { 0.0 },
                0.24,
                w,
                cx,
            );
            t_spots[idx] = t_spot;

            let (tag_lbl, t_bg, t_fg) = if calling {
                (tr(cx, "Calling", "调用中"), blue_bg, blue_fg)
            } else if used {
                (tr(cx, "Used", "已使用"), ok_bg, ok_fg)
            } else {
                (tr(cx, "Ready", "就绪"), tag_bg, p.text)
            };

            let live_call_text = if calling {
                running_step
                    .as_ref()
                    .map(|s| self.localize_default(&s.text, cx).to_owned())
            } else {
                None
            };
            let last_txt = if tool.last.is_empty() {
                tr(cx, "Nothing yet", "暂无调用记录").to_owned()
            } else {
                self.localize_default(&tool.last, cx).to_owned()
            };
            let when_txt = self.localize_default(&tool.when, cx).to_owned();

            let tool_pop =
                if is_open || self.last_target == Some(WorkbenchTarget::Tool(tid.clone())) {
                    let content = self.render_popover(
                        &WorkbenchTarget::Tool(tid.clone()),
                        1.0,
                        &steps,
                        done,
                        &p,
                        cx,
                    );
                    let mut cell = Some(content);
                    Some(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom(relative(1.0))
                            .pb(px(8.))
                            .child(pro_popover_presence(
                                pro_eid(&self.id, format!("tpop-{tid}")),
                                is_open,
                                move || cell.take().unwrap_or_else(|| div().into_any_element()),
                            )),
                    )
                } else {
                    None
                };

            let tool_aria: SharedString =
                format!("{} – {}", tool.name, tr(cx, "details", "详情")).into();
            let tid_click = tid.clone();
            let tid_hov = tid.clone();
            let t_border = pro_mix_color(p.line, blue.opacity(0.55), t_spot);

            let tool_hit = pro_tactile(
                pro_eid(&self.id, format!("tool-{tid}")),
                tool_aria,
                12.0,
                card_bg,
                pro_mix_color(card_bg, p.ink, 0.02),
                t_border,
                hover_border,
                is_open,
                div()
                    .w_full()
                    .py(px(12.))
                    .px(px(14.))
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(self.tool_sensors[idx].probe())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .min_w_0()
                            .child(pro_app_tile(&tool.app, 30., cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(16.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(tool.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(
                                                self.localize_default(&tool.scope, cx).to_owned(),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(18.))
                                    .px(px(6.))
                                    .flex_none()
                                    .self_start()
                                    .rounded(px(5.))
                                    .bg(t_bg)
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .when(used && !calling, |d| {
                                        d.child(pro_check_draw(
                                            pro_eid(&self.id, format!("tool-used-chk-{tid}")),
                                            true,
                                            12.0,
                                            false,
                                            t_fg,
                                        ))
                                    })
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, format!("tool-st-{tid}")),
                                        tag_lbl,
                                        11.,
                                        FontWeight::MEDIUM,
                                        t_fg,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .text_size(px(12.))
                            .child(if let Some(live_txt) = live_call_text {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .child(pro_shimmer_text(
                                        pro_eid(&self.id, format!("tshim-{tid}")),
                                        live_txt,
                                        p.ink,
                                        p.strong,
                                        12.,
                                        FontWeight::NORMAL,
                                        true,
                                    ))
                                    .into_any_element()
                            } else {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(p.text)
                                    .child(last_txt)
                                    .into_any_element()
                            })
                            .when(!calling && !when_txt.is_empty(), |d| {
                                d.child(div().flex_none().text_color(p.muted).child(when_txt))
                            }),
                    ),
                cx,
                move |this, _, cx| {
                    this.inspect(Some(WorkbenchTarget::Tool(tid_click.clone())), cx);
                },
            )
            .aria_expanded(is_open)
            .when(!p.dark, |d| d.shadow_xs())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_link = Some(tid_hov.clone());
                    cx.notify();
                } else if this.hovered_link.as_ref() == Some(&tid_hov) {
                    this.hovered_link = None;
                    cx.notify();
                }
            }));

            let card_wrap = div()
                .relative()
                .when(!stacked && total_cards >= 3, |d| d.flex_1().min_w_0())
                .when(!stacked && total_cards < 3, |d| d.w(px(258.)).min_w_0())
                .when(stacked, |d| d.w_full())
                .child(tool_hit)
                .when_some(tool_pop, |d, pop| d.child(pop));
            tools_row = tools_row.child(card_wrap);
        }

        if has_overflow_tools {
            let extra_tool_ids_chk: Vec<SharedString> = self
                .tools
                .iter()
                .skip(shown_tool_count)
                .map(|t| t.id.clone())
                .collect();
            let extra_lit = active_at
                .as_ref()
                .is_some_and(|id| extra_tool_ids_chk.contains(id))
                || hov_key == Some("agent")
                || insp_is_agent;
            t_spots[3] = pro_spring_f32(
                pro_eid(&self.id, "wb-t-spot-overflow"),
                if extra_lit { 1.0 } else { 0.0 },
                0.24,
                w,
                cx,
            );
            let extra_n = self.tools.len() - shown_tool_count;
            let mut logos = div().flex().items_center().gap(px(4.));
            for extra_tool in self.tools.iter().skip(shown_tool_count).take(3) {
                logos = logos.child(pro_app_tile(&extra_tool.app, 26., cx));
            }
            let summary_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("另外 {extra_n} 个工具")
            } else {
                format!("+{extra_n} more tools")
            };
            tools_row = tools_row.child(
                div()
                    .relative()
                    .when(!stacked, |d| d.flex_1().min_w_0())
                    .when(stacked, |d| d.w_full())
                    .py(px(12.))
                    .px(px(14.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .justify_between()
                    .gap(px(10.))
                    .child(self.tool_sensors[3].probe())
                    .child(logos)
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(summary_txt),
                    ),
            );
        }

        // Background wire canvas + foreground hollow-port canvas when side-by-side (`!stacked`)
        let wires_bg = {
            let u_s = self.user_sensor.clone();
            let a_s = self.agent_sensor.clone();
            let f_s = self.files_sensor.clone();
            let t_s = self.tool_sensors.clone();
            canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    if stacked {
                        return;
                    }
                    let (Some(ub), Some(ab), Some(fb)) = (u_s.get(), a_s.get(), f_s.get()) else {
                        return;
                    };
                    let dim_idle = wire_idle.opacity(wire_idle.a * (1.0 - 0.45 * any_spot));
                    // 1. User -> Agent horizontal wire
                    let uy = (ub.center().y + ab.center().y) * 0.5_f32;
                    let u_col = pro_mix_color(dim_idle, wire_active, u_spot);
                    pro_connector_pulse(
                        point(ub.right(), uy),
                        point(ab.left(), uy),
                        8.0,
                        false,
                        u_col,
                        blue,
                        if u_spot > 0.35 { pulse_t } else { None },
                        1.4 + 0.65 * u_spot,
                        window,
                    );
                    // 2. Agent -> Files horizontal wire
                    let fy = (ab.center().y + fb.center().y) * 0.5_f32;
                    let f_col = pro_mix_color(dim_idle, wire_active, f_spot);
                    pro_connector_pulse(
                        point(ab.right(), fy),
                        point(fb.left(), fy),
                        8.0,
                        false,
                        f_col,
                        blue,
                        if f_spot > 0.35 { pulse_t } else { None },
                        1.4 + 0.65 * f_spot,
                        window,
                    );
                    // 3. Agent -> Tools vertical-first wires
                    let aw = f32::from(ab.size.width);
                    for (slot, &ts) in t_spots.iter().enumerate().take(total_cards) {
                        let Some(tb) = t_s[slot].get() else {
                            continue;
                        };
                        let frac = if total_cards <= 1 {
                            0.5_f32
                        } else {
                            0.18_f32 + 0.64_f32 * (slot as f32 / (total_cards - 1) as f32)
                        };
                        let p0 = point(ab.left() + px(aw * frac), ab.bottom());
                        let p1 = point(tb.center().x, tb.top());
                        let t_col = pro_mix_color(dim_idle, wire_active, ts);
                        pro_connector_pulse(
                            p0,
                            p1,
                            10.0,
                            true,
                            t_col,
                            blue,
                            if ts > 0.35 { pulse_t } else { None },
                            1.4 + 0.65 * ts,
                            window,
                        );
                    }
                },
            )
            .absolute()
            .inset_0()
        };

        let ports_fg = {
            let u_s = self.user_sensor.clone();
            let a_s = self.agent_sensor.clone();
            let f_s = self.files_sensor.clone();
            let t_s = self.tool_sensors.clone();
            let has_pop = self.inspected.is_some();
            canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    if stacked || has_pop {
                        return;
                    }
                    let (Some(ub), Some(ab), Some(fb)) = (u_s.get(), a_s.get(), f_s.get()) else {
                        return;
                    };
                    let uy = (ub.center().y + ab.center().y) * 0.5_f32;
                    paint_hollow_port(point(ub.right(), uy), u_spot > 0.35, card_bg, blue, window);
                    paint_hollow_port(point(ab.left(), uy), u_spot > 0.35, card_bg, blue, window);

                    let fy = (ab.center().y + fb.center().y) * 0.5_f32;
                    paint_hollow_port(point(ab.right(), fy), f_spot > 0.35, card_bg, blue, window);
                    paint_hollow_port(point(fb.left(), fy), f_spot > 0.35, card_bg, blue, window);

                    let aw = f32::from(ab.size.width);
                    for (slot, &ts) in t_spots.iter().enumerate().take(total_cards) {
                        let Some(tb) = t_s[slot].get() else {
                            continue;
                        };
                        let frac = if total_cards <= 1 {
                            0.5_f32
                        } else {
                            0.18_f32 + 0.64_f32 * (slot as f32 / (total_cards - 1) as f32)
                        };
                        let p0 = point(ab.left() + px(aw * frac), ab.bottom());
                        let p1 = point(tb.center().x, tb.top());
                        paint_hollow_port(p0, ts > 0.35, card_bg, blue, window);
                        paint_hollow_port(p1, ts > 0.35, card_bg, blue, window);
                    }
                },
            )
            .absolute()
            .inset_0()
        };

        let top_row = if stacked {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(user_card)
                .child(agent_wrap)
                .child(files_card)
        } else {
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(36.))
                .child(user_card)
                .child(agent_wrap)
                .child(files_card)
        };

        let footer_bar = if self.custom_steps.is_none() {
            let live_now = self.live;
            let completed_steps = steps.iter().filter(|s| !s.running || done).count();
            let total_steps = self.playbook.len().max(1);
            Some(
                div()
                    .w_full()
                    .pt(px(4.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Hover or click any card to illuminate links and inspect permissions.",
                                        "悬停或点击任意卡片即可点亮连接线路并查看权限与调用明细。",
                                    )),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(7.))
                                    .rounded(px(5.))
                                    .bg(fill_bg)
                                    .border_1()
                                    .border_color(edge_col)
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "wb-steps-done"),
                                        completed_steps as f64,
                                        0,
                                        "",
                                        "",
                                        11.5,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child(format!("/ {total_steps}")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_btn(
                                pro_eid(&self.id, "step-btn"),
                                tr(cx, "Next step", "下一步"),
                                Some(ProIcon::ArrowRight),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.live = false;
                                    this.advance_demo_step(cx);
                                },
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if live_now { p.strong } else { p.muted })
                                            .child(tr(cx, "Auto", "自动")),
                                    )
                                    .child(pro_switch(
                                        pro_eid(&self.id, "live-btn"),
                                        tr(cx, "Auto-advance demo steps", "自动推进演示步骤"),
                                        live_now,
                                        cx,
                                        move |this, _, cx| {
                                            this.set_live(!live_now, cx);
                                        },
                                    )),
                            ),
                    ),
            )
        } else {
            None
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && this.inspected.is_some() {
                    this.inspect(None, cx);
                    this.focus.focus(w, cx);
                }
            }))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                if this.inspected.is_some() {
                    this.inspect(None, cx);
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                canvas(
                    move |bounds, _, _| {
                        let bw = f32::from(bounds.size.width);
                        if bw > 1.0 {
                            width_cell.set(bw);
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .h(px(1.)),
            )
            .child(
                div()
                    .w_full()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(if stacked { 12. } else { 64. }))
                    .child(wires_bg)
                    .child(top_row)
                    .child(tools_row)
                    .child(ports_fg),
            )
            .when_some(footer_bar, |d, fb| d.child(fb))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentWorkbench, WorkbenchStep, WorkbenchTarget, WorkbenchTool};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn agent_workbench_tracks_steps_inspection_and_done_state(cx: &mut TestAppContext) {
        let view = cx.new(|cx| AgentWorkbench::new("test-agent-workbench", cx));

        view.update(cx, |wb, cx| {
            assert!(!wb.is_done());
            assert_eq!(wb.visible_steps().len(), 2);
            assert!(wb.visible_steps()[0].running);

            wb.inspect(Some(WorkbenchTarget::Tool("gmail".into())), cx);
            assert_eq!(
                wb.inspected_target(),
                Some(&WorkbenchTarget::Tool("gmail".into()))
            );
            // Toggling the same target closes it.
            wb.inspect(Some(WorkbenchTarget::Tool("gmail".into())), cx);
            assert_eq!(wb.inspected_target(), None);

            // Advance through remaining steps to Done.
            for _ in 0..4 {
                wb.advance_demo_step(cx);
            }
            assert!(wb.is_done());

            wb.reset(cx);
            assert!(!wb.is_done());
        });

        let controlled = cx.new(|cx| {
            AgentWorkbench::new("controlled-wb", cx)
                .with_tools(vec![WorkbenchTool::new("slack", "Slack", "Post · #ops")])
                .with_steps(vec![
                    WorkbenchStep::new("a", "Posted deploy summary", "slack").with_running(false),
                ])
                .with_done(true)
        });
        controlled.read_with(cx, |wb, _| {
            assert!(wb.is_done());
            assert_eq!(wb.visible_steps().len(), 1);
            assert!(!wb.visible_steps()[0].running);
        });
    }
}
