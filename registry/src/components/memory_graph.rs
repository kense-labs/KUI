//! Radial agent memory and tool-reach graph with provenance cards, live pulses, and Gradient Floor washes.
//!
//! Places the central agent between a six-slot left arc of remembered facts (each with file,
//! chat, or email provenance) and a six-slot right arc of connected tools, joined by rounded
//! orthogonal connectors with hollow agent ports, per-card switches, Forget, and Add popovers.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_connector_pulse,
        pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color, pro_presence, pro_shimmer_text,
        pro_skeleton_bar, pro_switch, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        family::disabled_motion,
        pro_icons::{ProIcon, person_initials, pro_app_artwork, pro_icon, pro_raw_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const CHAT_BUBBLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="32" height="32"><path d="M9 3.5h9A3 3 0 0 1 21 6.5v6a3 3 0 0 1-3 3h-.5V18l-3-2.5H9a3 3 0 0 1-3-3v-6a3 3 0 0 1 3-3Z" fill="#00A1FF"/><path d="M3 9.5a3 3 0 0 1 3-3h8a3 3 0 0 1 3 3v5.5a3 3 0 0 1-3 3H9.2L5.5 21v-3H6a3 3 0 0 1-3-3z" fill="#3CC7FF"/></svg>"##;

const EXCEL_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32"><path d="M19.581 15.35 8.512 13.4v14.409A1.192 1.192 0 0 0 9.705 29h19.1A1.192 1.192 0 0 0 30 27.809V22.5Z" fill="#185C37"/><path d="M19.581 3H9.705A1.192 1.192 0 0 0 8.512 4.191V9.5L19.581 16l5.861 1.95L30 16V9.5Z" fill="#21A366"/><path d="M8.512 9.5h11.069V16H8.512Z" fill="#107C41"/><path d="M28.806 3h-9.225v6.5H30V4.191A1.192 1.192 0 0 0 28.806 3Z" fill="#33C481"/><path d="M19.581 16H30v6.5H19.581Z" fill="#107C41"/><rect x="2" y="8" width="16" height="16" rx="1.19" fill="#107C41"/><path d="m5.9 20.4 3.27-5.07-3-5.04h2.4l1.64 3.2c.15.3.25.53.3.68h.02c.1-.24.22-.46.34-.7l1.75-3.18h2.21l-3.07 5.01 3.15 5.1h-2.36l-1.89-3.54a2.86 2.86 0 0 1-.22-.47h-.03a2.2 2.2 0 0 1-.22.46l-1.95 3.55Z" fill="#fff"/></svg>"##;

const WORD_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32"><path d="M28.806 3H9.705A1.192 1.192 0 0 0 8.512 4.191V9.5L19.581 12.75 30 9.5V4.191A1.192 1.192 0 0 0 28.806 3Z" fill="#41A5EE"/><path d="M30 9.5H8.512V16l11.069 1.95L30 16Z" fill="#2B7CD3"/><path d="M8.512 16v6.5l10.418 1.3L30 22.5V16Z" fill="#185ABD"/><path d="M9.705 29h19.1A1.192 1.192 0 0 0 30 27.809V22.5H8.512v5.309A1.192 1.192 0 0 0 9.705 29Z" fill="#103F91"/><rect x="2" y="8" width="16" height="16" rx="1.19" fill="#185ABD"/><path d="M7.05 19.1c.04.27.06.5.07.7h.03c.02-.19.05-.42.1-.69l.11-.6 1.24-5.33h1.6l1.29 5.25c.07.31.13.62.16.94h.03c.03-.31.07-.61.12-.91l1.03-5.28h1.46l-1.8 7.64h-1.71l-1.23-5.07a7.6 7.6 0 0 1-.12-.62 5.6 5.6 0 0 1-.06-.4h-.02l-.08.44c-.03.17-.07.36-.11.56l-1.15 5.09H6.39l-1.84-7.64h1.5l1.13 5.35c.03.11.05.25.07.42Z" fill="#fff"/></svg>"##;

const PDF_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="32" height="32"><rect x="2" y="2" width="20" height="20" rx="5" fill="#B30B00"/><path d="M17.6 14.05c-.62-.63-2.3-.37-2.7-.32-.6-.57-1-1.27-1.15-1.52.21-.62.36-1.24.37-1.9 0-.57-.23-1.19-.87-1.19-.23 0-.45.14-.57.33-.28.48-.17 1.45.27 2.45a15.7 15.7 0 0 1-1.03 2.58c-.95.37-2.95 1.29-3.14 2.3-.06.3.04.6.27.79.23.18.5.25.78.25 1.13 0 2.26-1.55 3.03-2.89.65-.21 1.67-.52 2.69-.7 1.2 1.06 2.25 1.23 2.79 1.23.73 0 1-.32 1.08-.53.15-.37.03-.78-.1-.88Zm-.75.52c-.04.22-.26.37-.6.37-.08 0-.17-.01-.26-.04-.61-.15-1.18-.45-1.67-.85.41-.06 1.34-.15 2.01 0 .26.07.53.23.52.52Zm-4.22-5.13c.04-.07.12-.11.2-.11.21 0 .25.25.25.46-.02.48-.11.95-.27 1.41-.35-.92-.28-1.56-.18-1.76Zm-.04 4.39c.25-.48.6-1.37.73-1.74.28.47.75 1.02 1 1.27 0 .02-.98.21-1.73.47Zm-1.84 1.24c-.72 1.18-1.46 1.93-1.86 1.93a.38.38 0 0 1-.21-.06.32.32 0 0 1-.12-.28c.05-.43.88-1.05 2.19-1.59Z" fill="#fff"/></svg>"##;

/// Provenance channel of a [`MemoryNode`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryArtifactKind {
    /// Document or spreadsheet attachment inferred by filename extension.
    #[default]
    File,
    /// Direct conversation or feedback thread.
    Chat,
    /// Email message or thread (renders Gmail artwork).
    Email,
    /// Web link or wiki page.
    Link,
}

/// Source artifact metadata displayed along the bottom of a [`MemoryNode`] card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryArtifact {
    /// File name, chat title, or email subject (for example `"budget-q4.xlsx"` or `"Chat · Feedback"`).
    pub name: SharedString,
    /// Provenance channel controlling icon selection.
    pub kind: MemoryArtifactKind,
    /// Optional app mark override (for example `"notion"` or `"gmail"`).
    pub app: Option<SharedString>,
}

impl MemoryArtifact {
    /// Construct a file-provenance artifact from `name`.
    pub fn file(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            kind: MemoryArtifactKind::File,
            app: None,
        }
    }

    /// Construct a chat-provenance artifact from `name`.
    pub fn chat(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            kind: MemoryArtifactKind::Chat,
            app: None,
        }
    }

    /// Construct an email-provenance artifact from `name`.
    pub fn email(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            kind: MemoryArtifactKind::Email,
            app: Some("gmail".into()),
        }
    }

    /// Override the brand app mark rendered next to the artifact name.
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }
}

fn artifact_icon(art: &MemoryArtifact, side: f32, muted: Hsla, cx: &App) -> AnyElement {
    if let Some(app) = &art.app
        && let Some(mark) = pro_app_artwork(app, side, cx)
    {
        return mark;
    }
    match art.kind {
        MemoryArtifactKind::Chat => pro_raw_svg(CHAT_BUBBLE_SVG, side, side),
        MemoryArtifactKind::Email => pro_app_artwork("gmail", side, cx)
            .unwrap_or_else(|| pro_icon(ProIcon::Mail, side, muted)),
        MemoryArtifactKind::Link => pro_icon(ProIcon::Globe, side, muted),
        MemoryArtifactKind::File => {
            let lower = art.name.trim().to_ascii_lowercase();
            if lower.ends_with(".xlsx") || lower.ends_with(".xls") || lower.ends_with(".csv") {
                pro_raw_svg(EXCEL_SVG, side, side)
            } else if lower.ends_with(".docx") || lower.ends_with(".doc") {
                pro_raw_svg(WORD_SVG, side, side)
            } else if lower.ends_with(".pdf") {
                pro_raw_svg(PDF_SVG, side, side)
            } else if lower.ends_with(".md") {
                pro_app_artwork("notion", side, cx)
                    .unwrap_or_else(|| pro_icon(ProIcon::Note, side, muted))
            } else {
                pro_icon(ProIcon::PageDoc, side, muted)
            }
        }
    }
}

fn tool_mark(app: &str, side: f32, p: &ProPalette, cx: &App) -> AnyElement {
    if let Some(mark) = pro_app_artwork(app, side, cx) {
        mark
    } else {
        let ch: String = person_initials(app).chars().take(1).collect();
        div()
            .size(px(side))
            .rounded(px(4.))
            .bg(p.chip)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px((side * 0.62).max(9.)))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.text)
            .child(if ch.is_empty() { "T".to_owned() } else { ch })
            .into_any_element()
    }
}

/// One remembered fact card on the left half of [`MemoryGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryNode {
    /// Stable memory identifier.
    pub id: SharedString,
    /// Concise present-tense fact remembered by the agent.
    pub text: SharedString,
    /// Short provenance explanation (for example `"You asked for it twice"`).
    pub source: SharedString,
    /// Category tag shown in the detail popover (for example `"Preference"` or `"Project"`).
    pub tag: SharedString,
    /// File, chat, or email artifact this memory came from.
    pub artifact: MemoryArtifact,
    /// Optional tool id through which this memory was learned.
    pub via: Option<SharedString>,
    /// Optional older memory id superseded by this memory.
    pub replaces: Option<SharedString>,
    /// Whether a newer memory superseded this card.
    pub outdated: bool,
    /// Related memory ids linked on hover.
    pub related: Vec<SharedString>,
    /// Human-readable learned date (for example `"Nov 12"`).
    pub learned: SharedString,
    /// Number of runs that recalled this memory.
    pub uses: usize,
    /// Whether the user switched this memory off.
    pub disabled: bool,
}

impl MemoryNode {
    /// Construct a [`MemoryNode`] with `id`, `text`, `source`, and `artifact`.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        source: impl Into<SharedString>,
        artifact: MemoryArtifact,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            source: source.into(),
            tag: "Context".into(),
            artifact,
            via: None,
            replaces: None,
            outdated: false,
            related: Vec::new(),
            learned: "Nov 14".into(),
            uses: 4,
            disabled: false,
        }
    }

    /// Set the category tag displayed in the detail card.
    pub fn with_tag(mut self, tag: impl Into<SharedString>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Link the tool id through which this memory was learned.
    pub fn with_via(mut self, tool_id: impl Into<SharedString>) -> Self {
        self.via = Some(tool_id.into());
        self
    }

    /// Mark an older memory id as superseded by this memory.
    pub fn with_replaces(mut self, old_id: impl Into<SharedString>) -> Self {
        self.replaces = Some(old_id.into());
        self
    }

    /// Mark this memory as outdated.
    pub fn with_outdated(mut self, outdated: bool) -> Self {
        self.outdated = outdated;
        self
    }

    /// Set related memory ids highlighted when hovering this card.
    pub fn with_related<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.related = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Set learned date label and recall count.
    pub fn with_stats(mut self, learned: impl Into<SharedString>, uses: usize) -> Self {
        self.learned = learned.into();
        self.uses = uses;
        self
    }

    /// Start this memory switched off.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Connected tool card on the right half of [`MemoryGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryTool {
    /// Stable tool identifier.
    pub id: SharedString,
    /// Tool display title (for example `"GitHub"` or `"Figma"`).
    pub name: SharedString,
    /// Brand mark lookup name.
    pub app: SharedString,
    /// Permission level (`"Read"`, `"Read & write"`, `"Deploy"`).
    pub access: SharedString,
    /// Target workspace or repository scope (`"lumen/web"`).
    pub scope: SharedString,
    /// Last action note (`"Opened PR #482"`).
    pub note: SharedString,
    /// Relative last-used label (`"42m ago"`).
    pub when: SharedString,
    /// Total call count shown in details.
    pub calls: usize,
    /// Whether the user switched this tool off.
    pub disabled: bool,
}

impl MemoryTool {
    /// Construct a [`MemoryTool`] with `id`, `name`, `access`, and `scope`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        access: impl Into<SharedString>,
        scope: impl Into<SharedString>,
    ) -> Self {
        let n: SharedString = name.into();
        Self {
            id: id.into(),
            app: n.clone(),
            name: n,
            access: access.into(),
            scope: scope.into(),
            note: "Ready to run".into(),
            when: "1h ago".into(),
            calls: 8,
            disabled: false,
        }
    }

    /// Set the last action note and relative timestamp.
    pub fn with_last(
        mut self,
        note: impl Into<SharedString>,
        when: impl Into<SharedString>,
    ) -> Self {
        self.note = note.into();
        self.when = when.into();
        self
    }

    /// Set the cumulative call count displayed in the tool's detail card.
    pub fn with_calls(mut self, calls: usize) -> Self {
        self.calls = calls;
        self
    }

    /// Start this tool switched off.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Central agent identity in [`MemoryGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAgent {
    /// Deterministic seed id driving the 3x3 pixel mark.
    pub id: SharedString,
    /// Agent display name.
    pub name: SharedString,
    /// Idle subtitle shown when no tool or recall is active.
    pub subtitle: SharedString,
}

impl MemoryAgent {
    /// Construct a [`MemoryAgent`] with `id`, `name`, and `subtitle`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            subtitle: subtitle.into(),
        }
    }
}

/// Kind of ring item toggled on or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryNodeKind {
    /// Left-ring remembered fact.
    Memory,
    /// Right-ring connected tool.
    Tool,
}

/// Currently open popover in [`MemoryGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryGraphOverlay {
    /// Agent "Add file or connect tool" dialog.
    AddMenu,
    /// Detail card for a specific memory id.
    Memory(SharedString),
    /// Detail card for a specific tool id.
    Tool(SharedString),
}

/// Typed events emitted by [`MemoryGraph`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryGraphEvent {
    /// A memory, tool, or the agent Add popover was opened or closed (`None`).
    OverlayChanged(Option<MemoryGraphOverlay>),
    /// A memory or tool switch was toggled.
    Toggled {
        kind: MemoryNodeKind,
        id: SharedString,
        enabled: bool,
    },
    /// A memory was permanently forgotten.
    MemoryForgotten(SharedString),
    /// A new file-backed memory was added via the agent popover.
    FileAdded {
        memory_id: SharedString,
        file_name: SharedString,
    },
    /// A catalog tool was connected to the right half of the ring.
    ToolConnected(SharedString),
}

fn default_memories() -> Vec<MemoryNode> {
    vec![
        MemoryNode::new(
            "m1",
            "Prefers short answers",
            "You asked for it twice",
            MemoryArtifact::chat("Chat · Feedback"),
        )
        .with_tag("Preference")
        .with_stats("Nov 08", 19),
        MemoryNode::new(
            "m2",
            "Product designer at Lumen",
            "From your profile",
            MemoryArtifact::file("maya-cv.pdf"),
        )
        .with_tag("Work")
        .with_stats("Oct 29", 14)
        .with_related(["m3"]),
        MemoryNode::new(
            "m3",
            "Redesign ships Dec 4",
            "From the kickoff notes",
            MemoryArtifact::file("kickoff-notes.docx"),
        )
        .with_tag("Project")
        .with_via("notion")
        .with_stats("Nov 12", 7)
        .with_related(["m2", "m4"]),
        MemoryNode::new(
            "m4",
            "Q4 design budget €48k",
            "From the budget sheet",
            MemoryArtifact::file("budget-q4.xlsx"),
        )
        .with_tag("Finance")
        .with_stats("Nov 11", 5)
        .with_related(["m3"]),
    ]
}

fn default_tools() -> Vec<MemoryTool> {
    vec![
        MemoryTool::new("github", "GitHub", "Read & write", "lumen/web")
            .with_last("Opened PR #482", "42m ago")
            .with_calls(24),
        MemoryTool::new("linear", "Linear", "Read", "Redesign")
            .with_last("Listed the redesign issues", "3h ago")
            .with_calls(16),
        MemoryTool::new("notion", "Notion", "Read", "Design wiki")
            .with_last("Read the standup notes", "1d ago")
            .with_calls(11),
        MemoryTool::new("figma", "Figma", "Read", "Lumen files")
            .with_last("Looked at the latest drafts", "In use")
            .with_calls(9),
        MemoryTool::new("vercel", "Vercel", "Deploy", "Previews")
            .with_last("Deployed a preview", "2d ago")
            .with_calls(6),
    ]
}

fn default_catalog() -> Vec<MemoryTool> {
    vec![
        MemoryTool::new("slack", "Slack", "Read & post", "#design-ops")
            .with_last("Connected just now", "Just now")
            .with_calls(1),
        MemoryTool::new("drive", "Google Drive", "Read", "Brand assets")
            .with_last("Indexed 38 files", "Just now")
            .with_calls(1),
        MemoryTool::new("stripe", "Stripe", "Read", "Subscriptions")
            .with_last("Synced MRR snapshot", "Just now")
            .with_calls(1),
    ]
}

const SAMPLE_NEW_FILES: [(&str, &str, &str); 3] = [
    (
        "Design system tokens freeze Nov 28",
        "Extracted from uploaded spec",
        "tokens-freeze.pdf",
    ),
    (
        "Enterprise SSO launches in Q1",
        "From roadmap spreadsheet",
        "q1-roadmap.xlsx",
    ),
    (
        "Copy tone stays crisp and direct",
        "From brand guidelines",
        "voice-guide.docx",
    ),
];

/// Six-slot left/right C-arc geometry relative to a `stage_w x stage_h` canvas.
/// Slots fill from the middle outward (`0, 1` inner middle -> `2, 3` mid -> `4, 5` outer top/bottom)
/// so adding a card never shifts previously placed cards!
fn ring_slot_pos(
    side_right: bool,
    slot: usize,
    stage_w: f32,
    stage_h: f32,
    card_w: f32,
    card_h: f32,
) -> (f32, f32) {
    let cy = stage_h * 0.5_f32;
    let scale_x = ((stage_w - 520.0) / 240.0).clamp(0.0, 1.0);
    let (inset_x, dy): (f32, f32) = match slot {
        0 => (4.0, -50.0),
        1 => (4.0, 50.0),
        2 => (10.0 + 26.0 * scale_x, -142.0),
        3 => (10.0 + 26.0 * scale_x, 142.0),
        4 => (24.0 + 84.0 * scale_x, -234.0),
        _ => (24.0 + 84.0 * scale_x, 234.0),
    };
    let x = if side_right {
        (stage_w - card_w - inset_x).max(0.0)
    } else {
        inset_x
    };
    let y = (cy + dy - card_h * 0.5_f32).clamp(4.0, (stage_h - card_h - 4.0).max(4.0));
    (x, y)
}

fn agent_port_point(
    side_right: bool,
    slot: usize,
    agent_rect: Bounds<Pixels>,
) -> (Point<Pixels>, bool) {
    let ax = f32::from(agent_rect.origin.x);
    let ay = f32::from(agent_rect.origin.y);
    let aw = f32::from(agent_rect.size.width);
    let ah = f32::from(agent_rect.size.height);
    let edge_x = if side_right { ax + aw } else { ax };
    match slot {
        0 => (point(px(edge_x), px(ay + ah * 0.38)), false),
        1 => (point(px(edge_x), px(ay + ah * 0.62)), false),
        2 => (point(px(edge_x), px(ay + ah * 0.18)), false),
        3 => (point(px(edge_x), px(ay + ah * 0.82)), false),
        4 => {
            let px_x = if side_right {
                ax + aw * 0.72
            } else {
                ax + aw * 0.28
            };
            (point(px(px_x), px(ay)), true)
        }
        _ => {
            let px_x = if side_right {
                ax + aw * 0.72
            } else {
                ax + aw * 0.28
            };
            (point(px(px_x), px(ay + ah)), true)
        }
    }
}

fn paint_ring_port(pt: Point<Pixels>, stroke_col: Hsla, card_bg: Hsla, w: &mut Window) {
    let r = 3.0_f32;
    let rect = Bounds::new(
        point(pt.x - px(r), pt.y - px(r)),
        size(px(r * 2.0), px(r * 2.0)),
    );
    w.paint_quad(quad(
        rect,
        Corners::all(px(r)),
        card_bg,
        Edges::all(px(1.5)),
        stroke_col,
        BorderStyle::Solid,
    ));
}

fn paint_dashed_segment(
    p0: Point<Pixels>,
    p1: Point<Pixels>,
    color: Hsla,
    stroke_w: f32,
    w: &mut Window,
) {
    let x0 = f32::from(p0.x);
    let y0 = f32::from(p0.y);
    let x1 = f32::from(p1.x);
    let y1 = f32::from(p1.y);
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 2.0 {
        return;
    }
    let step = 8.0_f32;
    let count = (len / step).ceil() as usize;
    for i in 0..count {
        let t0 = (i as f32 * step) / len;
        let t1 = ((i as f32 * step + 3.5) / len).min(1.0);
        if t1 <= t0 {
            continue;
        }
        let mut seg = PathBuilder::stroke(px(stroke_w));
        seg.move_to(point(px(x0 + dx * t0), px(y0 + dy * t0)));
        seg.line_to(point(px(x0 + dx * t1), px(y0 + dy * t1)));
        if let Ok(path) = seg.build() {
            w.paint_path(path, color);
        }
    }
}

/// Radial agent memory & connected-tools graph with live pulses, Gradient Floor washes, and popovers.
pub struct MemoryGraph {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    agent: MemoryAgent,
    memories: Vec<MemoryNode>,
    tools: Vec<MemoryTool>,
    catalog: Vec<MemoryTool>,
    using_override: Option<Vec<SharedString>>,
    recalling_override: Option<Vec<SharedString>>,
    overlay: Option<MemoryGraphOverlay>,
    last_overlay: Option<MemoryGraphOverlay>,
    hovered_memory: Option<SharedString>,
    mem_page: usize,
    live: bool,
    cycle_step: usize,
    cycle_started: Instant,
    wash_card: Option<(SharedString, Instant)>,
    added_file_seq: usize,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl MemoryGraph {
    /// Create a [`MemoryGraph`] preloaded with 4 provenance memories, 5 tools (`Figma` active), and 3 catalog integrations.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        let mut memories = default_memories();
        Self::sync_outdated(&mut memories);
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "What Assistant knows and can reach".into(),
            agent: MemoryAgent::new("assistant", "Assistant", "Personal assistant"),
            memories,
            tools: default_tools(),
            catalog: default_catalog(),
            using_override: None,
            recalling_override: None,
            overlay: None,
            last_overlay: None,
            hovered_memory: None,
            mem_page: 0,
            live: true,
            cycle_step: 0,
            cycle_started: now,
            wash_card: Some(("linear".into(), now)),
            added_file_seq: 0,
            boot_at: now,
            measured_width: Rc::new(Cell::new(760.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    fn sync_outdated(memories: &mut [MemoryNode]) {
        let replaced_ids: Vec<SharedString> =
            memories.iter().filter_map(|m| m.replaces.clone()).collect();
        for m in memories.iter_mut() {
            if replaced_ids.contains(&m.id) {
                m.outdated = true;
            }
        }
    }

    /// Override the header bar title.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the central agent identity.
    pub fn with_agent(mut self, agent: MemoryAgent) -> Self {
        self.agent = agent;
        self
    }

    /// Supply custom left-ring [`MemoryNode`] cards.
    pub fn with_memories(mut self, mut memories: Vec<MemoryNode>) -> Self {
        Self::sync_outdated(&mut memories);
        self.memories = memories;
        self
    }

    /// Supply custom right-ring [`MemoryTool`] cards.
    pub fn with_tools(mut self, tools: Vec<MemoryTool>) -> Self {
        self.tools = tools;
        self
    }

    /// Supply unconnected catalog tools available in the agent's Add popover.
    pub fn with_catalog(mut self, catalog: Vec<MemoryTool>) -> Self {
        self.catalog = catalog;
        self
    }

    /// Explicitly control which tool ids are currently in use.
    pub fn with_using<I, S>(mut self, tool_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.using_override = Some(tool_ids.into_iter().map(Into::into).collect());
        self.live = false;
        self
    }

    /// Explicitly control which memory ids are currently recalled.
    pub fn with_recalling<I, S>(mut self, memory_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.recalling_override = Some(memory_ids.into_iter().map(Into::into).collect());
        self.live = false;
        self
    }

    /// Enable or disable live tool-use and recall pulse cycling.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self.cycle_started = Instant::now();
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

    /// Number of stored memories currently in the graph.
    pub fn memory_count(&self) -> usize {
        self.memories.len()
    }

    /// Number of connected tools currently in the graph.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Currently open popover overlay, if any.
    pub fn active_overlay(&self) -> Option<&MemoryGraphOverlay> {
        self.overlay.as_ref()
    }

    /// Open or close (`None`) a memory, tool, or Add popover.
    pub fn set_overlay(&mut self, next: Option<MemoryGraphOverlay>, cx: &mut Context<Self>) {
        let resolved = if self.overlay == next { None } else { next };
        if let Some(ov) = &resolved {
            self.last_overlay = Some(ov.clone());
        }
        self.overlay = resolved.clone();
        cx.emit(MemoryGraphEvent::OverlayChanged(resolved));
        cx.notify();
    }

    /// Toggle whether a memory or tool is enabled.
    pub fn toggle_node(
        &mut self,
        kind: MemoryNodeKind,
        id: &str,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        match kind {
            MemoryNodeKind::Memory => {
                let mem = self.memories.iter_mut().find(|m| m.id.as_ref() == id)?;
                mem.disabled = !mem.disabled;
                let enabled = !mem.disabled;
                cx.emit(MemoryGraphEvent::Toggled {
                    kind,
                    id: id.to_owned().into(),
                    enabled,
                });
                cx.notify();
                Some(enabled)
            }
            MemoryNodeKind::Tool => {
                let tool = self.tools.iter_mut().find(|t| t.id.as_ref() == id)?;
                tool.disabled = !tool.disabled;
                let enabled = !tool.disabled;
                cx.emit(MemoryGraphEvent::Toggled {
                    kind,
                    id: id.to_owned().into(),
                    enabled,
                });
                cx.notify();
                Some(enabled)
            }
        }
    }

    /// Permanently forget a memory by id.
    pub fn forget_memory(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let before = self.memories.len();
        self.memories.retain(|m| m.id.as_ref() != id);
        if self.memories.len() == before {
            return false;
        }
        Self::sync_outdated(&mut self.memories);
        if self.overlay == Some(MemoryGraphOverlay::Memory(id.to_owned().into())) {
            self.overlay = None;
        }
        self.toast = Some((
            tr(cx, "Memory forgotten", "已遗忘该条记忆").into(),
            Instant::now(),
        ));
        cx.emit(MemoryGraphEvent::MemoryForgotten(id.to_owned().into()));
        cx.notify();
        true
    }

    /// Drop/browse a sample file in the Add dialog, creating a newly learned memory and lighting its Gradient Floor.
    pub fn add_sample_file(&mut self, cx: &mut Context<Self>) {
        let (fact, src, fname) = SAMPLE_NEW_FILES[self.added_file_seq % SAMPLE_NEW_FILES.len()];
        self.added_file_seq += 1;
        let mem_id: SharedString = format!("m-learned-{}", self.added_file_seq).into();
        let node = MemoryNode::new(mem_id.clone(), fact, src, MemoryArtifact::file(fname))
            .with_tag("Learned")
            .with_stats("Just now", 1);
        self.memories.insert(0, node);
        self.mem_page = 0;
        self.overlay = None;
        let now = Instant::now();
        self.wash_card = Some((mem_id.clone(), now));
        self.cycle_started = now;
        self.toast = Some((
            format!("{} {fname}", tr(cx, "Learned from", "已从文件学习：")).into(),
            now,
        ));
        cx.emit(MemoryGraphEvent::FileAdded {
            memory_id: mem_id,
            file_name: fname.into(),
        });
        cx.notify();
    }

    /// Connect a tool from `catalog` onto the right half of the ring.
    pub fn connect_catalog_tool(&mut self, tool_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(pos) = self.catalog.iter().position(|t| t.id.as_ref() == tool_id) else {
            return false;
        };
        let tool = self.catalog.remove(pos);
        let tid = tool.id.clone();
        let tname = tool.name.clone();
        self.tools.push(tool);
        self.overlay = None;
        let now = Instant::now();
        self.wash_card = Some((tid.clone(), now));
        self.cycle_started = now;
        self.toast = Some((
            format!("{} {tname}", tr(cx, "Connected", "已连接")).into(),
            now,
        ));
        cx.emit(MemoryGraphEvent::ToolConnected(tid));
        cx.notify();
        true
    }

    fn active_using_and_recalling(&self) -> (Vec<SharedString>, Vec<SharedString>) {
        let enabled_tools: Vec<&MemoryTool> = self.tools.iter().filter(|t| !t.disabled).collect();
        let enabled_mems: Vec<&MemoryNode> = self.memories.iter().filter(|m| !m.disabled).collect();

        if self.using_override.is_some() || self.recalling_override.is_some() {
            let u = self
                .using_override
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| enabled_tools.iter().any(|t| &t.id == id))
                .collect();
            let r = self
                .recalling_override
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| enabled_mems.iter().any(|m| &m.id == id))
                .collect();
            return (u, r);
        }

        if enabled_tools.is_empty() && enabled_mems.is_empty() {
            return (Vec::new(), Vec::new());
        }

        match self.cycle_step % 3 {
            0 => {
                // Default canonical snapshot: "Using Figma" (or last enabled tool) + recalling m1
                let tool_pick = enabled_tools
                    .iter()
                    .find(|t| t.id == "figma")
                    .or_else(|| enabled_tools.first())
                    .map(|t| vec![t.id.clone()])
                    .unwrap_or_default();
                let mem_pick = enabled_mems
                    .first()
                    .map(|m| vec![m.id.clone()])
                    .unwrap_or_default();
                (tool_pick, mem_pick)
            }
            1 => {
                let tool_pick = enabled_tools
                    .get(1)
                    .or_else(|| enabled_tools.first())
                    .map(|t| vec![t.id.clone()])
                    .unwrap_or_default();
                let mem_pick = enabled_mems.iter().take(2).map(|m| m.id.clone()).collect();
                (tool_pick, mem_pick)
            }
            _ => {
                let tool_pick = enabled_tools
                    .iter()
                    .find(|t| t.id == "notion")
                    .or_else(|| enabled_tools.first())
                    .map(|t| vec![t.id.clone()])
                    .unwrap_or_default();
                let mem_pick = enabled_mems
                    .iter()
                    .find(|m| m.via.as_deref() == Some("notion"))
                    .or_else(|| enabled_mems.last())
                    .map(|m| vec![m.id.clone()])
                    .unwrap_or_default();
                (tool_pick, mem_pick)
            }
        }
    }

    fn localize_text<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "What Assistant knows and can reach" => tr(
                cx,
                "What Assistant knows and can reach",
                "智能助手的记忆与可触达工具",
            ),
            "Assistant" => tr(cx, "Assistant", "智能助手"),
            "Personal assistant" => tr(cx, "Personal assistant", "个人工作助手"),
            "Prefers short answers" => tr(cx, "Prefers short answers", "偏好简短直接的回答"),
            "You asked for it twice" => tr(cx, "You asked for it twice", "你曾在对话中两次提及"),
            "Chat · Feedback" => tr(cx, "Chat · Feedback", "对话 · 反馈"),
            "Product designer at Lumen" => tr(cx, "Product designer at Lumen", "Lumen 产品设计师"),
            "From your profile" => tr(cx, "From your profile", "提取自个人履历"),
            "Redesign ships Dec 4" => tr(cx, "Redesign ships Dec 4", "改版定于 12 月 4 日上线"),
            "From the kickoff notes" => tr(cx, "From the kickoff notes", "来自项目启动纪要"),
            "Q4 design budget €48k" => tr(cx, "Q4 design budget €48k", "Q4 设计预算为 €48k"),
            "From the budget sheet" => tr(cx, "From the budget sheet", "来自季度预算表格"),
            "Read & write" => tr(cx, "Read & write", "读写"),
            "Read" => tr(cx, "Read", "只读"),
            "Read & post" => tr(cx, "Read & post", "读取与发送"),
            "Deploy" => tr(cx, "Deploy", "部署"),
            "Opened PR #482" => tr(cx, "Opened PR #482", "已创建 PR #482"),
            "Listed the redesign issues" => {
                tr(cx, "Listed the redesign issues", "已列出改版任务单")
            }
            "Read the standup notes" => tr(cx, "Read the standup notes", "已读取站会纪要"),
            "Looked at the latest drafts" => {
                tr(cx, "Looked at the latest drafts", "已查阅最新设计稿")
            }
            "Deployed a preview" => tr(cx, "Deployed a preview", "已部署预览环境"),
            "In use" => tr(cx, "In use", "使用中"),
            other => other,
        }
    }

    fn render_overlay_card(
        &self,
        ov: &MemoryGraphOverlay,
        using: &[SharedString],
        recalling: &[SharedString],
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pop_bg: Hsla = if p.dark {
            rgb(0x232327).into()
        } else {
            rgb(0xffffff).into()
        };
        let blue_fg: Hsla = if p.dark {
            rgb(0x6cbcff).into()
        } else {
            rgb(0x006dcb).into()
        };
        let blue_soft: Hsla = if p.dark {
            hsla(207. / 360., 0.99, 0.65, 0.14)
        } else {
            hsla(209. / 360., 1.0, 0.44, 0.09)
        };

        let body = match ov {
            MemoryGraphOverlay::AddMenu => {
                let hover_bg = p.hover;
                let focus_col = p.focus;
                let drop_zone =
                    activate(
                        div()
                            .id(pro_eid(&self.id, "add-drop-file"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Drop a file or browse", "拖入文件或浏览上传"))
                            .tab_index(0)
                            .w_full()
                            .p(px(10.))
                            .rounded(px(8.))
                            .bg(p.chip)
                            .border_1()
                            .border_color(p.line_strong)
                            .hover(move |s| s.bg(hover_bg))
                            .focus_visible(move |s| s.border_2().border_color(focus_col))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(
                                div()
                                    .size(px(28.))
                                    .rounded(px(7.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(ProIcon::Upload, 14., blue_fg)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(tr(
                                                cx,
                                                "Drop a file to learn from",
                                                "添加文档提取新记忆",
                                            )),
                                    )
                                    .child(div().text_size(px(11.5)).text_color(blue_fg).child(
                                        tr(cx, "Browse sample document →", "选择示例文档导入 →"),
                                    )),
                            ),
                        cx,
                        |this, _, cx| {
                            this.add_sample_file(cx);
                        },
                    );

                let mut cat_col = div().flex().flex_col().gap(px(3.));
                if self.catalog.is_empty() {
                    cat_col = cat_col.child(
                        div()
                            .py(px(6.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "All catalog tools connected", "目录工具已全部连接")),
                    );
                } else {
                    for item in &self.catalog {
                        let cid = item.id.clone();
                        let row = activate(
                            div()
                                .id(pro_eid(&self.id, format!("cat-{cid}")))
                                .role(Role::Button)
                                .aria_label(format!("{} · {}", item.name, item.scope))
                                .tab_index(0)
                                .w_full()
                                .px(px(6.))
                                .py(px(5.))
                                .rounded(px(7.))
                                .hover(move |s| s.bg(hover_bg))
                                .focus_visible(move |s| s.border_1().border_color(focus_col))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .child(pro_app_tile(&item.app, 28., cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .truncate()
                                                .child(item.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.muted)
                                                .truncate()
                                                .child(format!(
                                                    "{} · {}",
                                                    self.localize_text(&item.access, cx),
                                                    item.scope
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .size(px(24.))
                                        .rounded(px(6.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::Plus, 14., p.muted)),
                                ),
                            cx,
                            move |this, _, cx| {
                                this.connect_catalog_tool(cid.as_ref(), cx);
                            },
                        );
                        cat_col = cat_col.child(row);
                    }
                }

                div()
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Add to Assistant", "为智能助手添加知识或工具")),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "close-add"),
                                tr(cx, "Close", "关闭"),
                                ProIcon::Close,
                                22.,
                                false,
                                cx,
                                |this, _, cx| this.set_overlay(None, cx),
                            )),
                    )
                    .child(drop_zone)
                    .child(
                        div()
                            .pt(px(2.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(tr(cx, "Connect a tool", "连接外部工具")),
                    )
                    .child(cat_col)
            }
            MemoryGraphOverlay::Memory(mid) => {
                let Some(mem) = self.memories.iter().find(|m| &m.id == mid).cloned() else {
                    return div().into_any_element();
                };
                let is_recalling = recalling.contains(&mem.id) && !mem.disabled;
                let mut chips = div().flex().flex_wrap().items_center().gap(px(4.));
                chips = chips.child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(5.))
                        .bg(p.chip)
                        .text_size(px(11.))
                        .text_color(p.text)
                        .child(mem.tag.clone()),
                );
                if is_recalling {
                    chips = chips.child(
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(5.))
                            .bg(blue_soft)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(blue_fg)
                            .child(tr(cx, "Recalled", "正在召回")),
                    );
                }
                if mem.outdated {
                    chips = chips.child(
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(5.))
                            .bg(p.amber_soft)
                            .text_size(px(11.))
                            .text_color(p.amber_ink)
                            .child(tr(cx, "Outdated", "已过期")),
                    );
                }
                if mem.disabled {
                    chips = chips.child(
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(5.))
                            .bg(p.chip)
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(tr(cx, "Off", "已停用")),
                    );
                }

                let mid_toggle = mem.id.clone();
                let mid_forget = mem.id.clone();
                let stats_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("学习于 {} · 已使用 {} 次", mem.learned, mem.uses)
                } else {
                    format!("Learned {} · {} uses", mem.learned, mem.uses)
                };

                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.))
                            .child(chips)
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "close-mem"),
                                tr(cx, "Close", "关闭"),
                                ProIcon::Close,
                                22.,
                                false,
                                cx,
                                |this, _, cx| this.set_overlay(None, cx),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if mem.outdated { p.muted } else { p.strong })
                            .child(self.localize_text(&mem.text, cx).to_owned()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.text)
                            .child(self.localize_text(&mem.source, cx).to_owned()),
                    )
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(7.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(artifact_icon(&mem.artifact, 20., p.muted, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(
                                                self.localize_text(&mem.artifact.name, cx)
                                                    .to_owned(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(stats_txt),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(4.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .child(div().text_size(px(12.)).text_color(p.ink).child(tr(
                                cx,
                                "Use this memory",
                                "启用该条记忆",
                            )))
                            .child(pro_switch(
                                pro_eid(&self.id, format!("msw-{mid_toggle}")),
                                tr(cx, "Use this memory", "启用该条记忆"),
                                !mem.disabled,
                                cx,
                                move |this, _, cx| {
                                    this.toggle_node(
                                        MemoryNodeKind::Memory,
                                        mid_toggle.as_ref(),
                                        cx,
                                    );
                                },
                            )),
                    )
                    .child(
                        div()
                            .pt(px(8.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(11.5)).text_color(p.muted).child(
                                if let Some(via) = &mem.via {
                                    format!("{} {via}", tr(cx, "Via", "来源工具："))
                                } else {
                                    tr(cx, "Direct context", "直接提取").to_owned()
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, format!("mforget-{mid_forget}")),
                                tr(cx, "Forget", "遗忘"),
                                Some(ProIcon::Trash),
                                ProBtnStyle::Danger,
                                true,
                                cx,
                                move |this, _, cx| {
                                    this.forget_memory(mid_forget.as_ref(), cx);
                                },
                            )),
                    )
            }
            MemoryGraphOverlay::Tool(tid) => {
                let Some(tool) = self.tools.iter().find(|t| &t.id == tid).cloned() else {
                    return div().into_any_element();
                };
                let is_using = using.contains(&tool.id) && !tool.disabled;
                let tid_toggle = tool.id.clone();
                let calls_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!(
                        "累计调用 {} 次 · {}",
                        tool.calls,
                        self.localize_text(&tool.when, cx)
                    )
                } else {
                    format!("{} calls · {}", tool.calls, tool.when)
                };

                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_app_tile(&tool.app, 26., cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(tool.name.clone()),
                            )
                            .when(is_using, |d| {
                                d.child(
                                    div()
                                        .px(px(6.))
                                        .py(px(1.))
                                        .rounded(px(5.))
                                        .bg(blue_soft)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(blue_fg)
                                        .child(tr(cx, "In use", "使用中")),
                                )
                            })
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "close-tool"),
                                tr(cx, "Close", "关闭"),
                                ProIcon::Close,
                                22.,
                                false,
                                cx,
                                |this, _, cx| this.set_overlay(None, cx),
                            )),
                    )
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(6.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format!(
                                        "{} · {}",
                                        self.localize_text(&tool.access, cx),
                                        tool.scope
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(self.localize_text(&tool.note, cx).to_owned()),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .child(div().text_size(px(12.)).text_color(p.ink).child(tr(
                                cx,
                                "Allow tool access",
                                "允许访问此工具",
                            )))
                            .child(pro_switch(
                                pro_eid(&self.id, format!("tsw-{tid_toggle}")),
                                tr(cx, "Allow tool access", "允许访问此工具"),
                                !tool.disabled,
                                cx,
                                move |this, _, cx| {
                                    this.toggle_node(MemoryNodeKind::Tool, tid_toggle.as_ref(), cx);
                                },
                            )),
                    )
                    .child(
                        div()
                            .pt(px(6.))
                            .border_t_1()
                            .border_color(p.line)
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(calls_txt),
                    )
            }
        };

        div()
            .id(pro_eid(&self.id, "mg-pop"))
            .role(Role::Group)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(272.))
            .p(px(11.))
            .rounded(px(10.))
            .bg(pop_bg)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .child(body)
            .into_any_element()
    }
}

impl EventEmitter<MemoryGraphEvent> for MemoryGraph {}

impl Focusable for MemoryGraph {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for MemoryGraph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

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
                .gap(px(16.))
                .child(pro_skeleton_bar(260., 18., 6., phase, cx))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(24.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(14.))
                                .child(pro_skeleton_bar(204., 74., 8., phase + 0.1, cx))
                                .child(pro_skeleton_bar(204., 74., 8., phase + 0.2, cx)),
                        )
                        .child(pro_skeleton_bar(220., 100., 8., phase + 0.15, cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(14.))
                                .child(pro_skeleton_bar(204., 74., 8., phase + 0.25, cx))
                                .child(pro_skeleton_bar(204., 74., 8., phase + 0.35, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Memory,
                    tr(cx, "No memories or tools yet", "暂无已保存记忆或工具"),
                    tr(
                        cx,
                        "Drop a document or connect an integration so the assistant can learn and act.",
                        "上传参考文档或连接外部工具后，助手即可建立上下文记忆。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-mg"),
                        tr(cx, "Load sample graph", "加载示例图谱").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.memories = default_memories();
                            this.tools = default_tools();
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let cycle_age = self.cycle_started.elapsed().as_secs_f32();
        if self.live && !reduced && self.overlay.is_none() {
            if cycle_age >= 3.4_f32 {
                self.cycle_step = self.cycle_step.wrapping_add(1);
                self.cycle_started = Instant::now();
                let (u, r) = self.active_using_and_recalling();
                if let Some(target_id) = r.first().or_else(|| u.first()).cloned() {
                    self.wash_card = Some((target_id, Instant::now()));
                }
            }
            w.request_animation_frame();
        } else if let Some((_, started)) = self.wash_card
            && started.elapsed().as_secs_f32() < 1.05
            && !reduced
        {
            w.request_animation_frame();
        }

        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let pulse_t = if reduced {
            None
        } else {
            Some(((elapsed_s / 1.45).fract()).clamp(0.0, 1.0))
        };

        let (using, recalling) = self.active_using_and_recalling();
        let is_busy = !using.is_empty() || !recalling.is_empty();

        let status_line: String = if let Some(tid) = using.first()
            && let Some(t) = self.tools.iter().find(|t| &t.id == tid)
        {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("正在调用 {}", t.name)
            } else {
                format!("Using {}", t.name)
            }
        } else if !recalling.is_empty() {
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("正在召回 {} 条记忆", recalling.len())
            } else if recalling.len() == 1 {
                "Recalling 1 memory".to_owned()
            } else {
                format!("Recalling {} memories", recalling.len())
            }
        } else {
            self.localize_text(&self.agent.subtitle, cx).to_owned()
        };

        let width_cell = self.measured_width.clone();
        let stage_w = width_cell.get().max(320.0);
        let stacked = stage_w < 640.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let light_blue: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0190fe).into()
        };
        let wire_col: Hsla = if p.dark {
            rgba(0x4fb0fe8c).into()
        } else {
            rgb(0x83c7fe).into()
        };
        let hot_bg = pro_mix_color(card_bg, light_blue, if p.dark { 0.09 } else { 0.065 });
        let hot_border = light_blue.opacity(0.52);
        let focus_col = p.focus;
        let hover_border = p.line_strong;

        // Header bar
        let mem_sum = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("{} 条记忆", self.memories.len())
        } else if self.memories.len() == 1 {
            "1 memory".to_owned()
        } else {
            format!("{} memories", self.memories.len())
        };
        let tool_sum = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("{} 个工具", self.tools.len())
        } else if self.tools.len() == 1 {
            "1 tool".to_owned()
        } else {
            format!("{} tools", self.tools.len())
        };

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.localize_text(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(mem_sum)
                    .child(tool_sum),
            );

        // Build Agent card element (shared by ring & stacked modes)
        let agent_w = if stacked {
            stage_w
        } else {
            220.0_f32.min((stage_w * 0.30).max(186.0))
        };
        let agent_h = 100.0_f32;
        let mut mini_dock = div().flex().items_center().gap(px(5.));
        let max_dock = 5usize;
        for tool in self.tools.iter().take(max_dock) {
            let on = using.contains(&tool.id) && !tool.disabled;
            let tile_alpha = if tool.disabled {
                0.14
            } else if on {
                1.0
            } else {
                0.38
            };
            mini_dock = mini_dock.child(
                div()
                    .size(px(22.))
                    .rounded(px(6.))
                    .bg(if on {
                        pro_mix_color(card_bg, light_blue, 0.10)
                    } else {
                        card_bg
                    })
                    .border_1()
                    .border_color(if on { light_blue.opacity(0.50) } else { p.line })
                    .opacity(tile_alpha)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(tool_mark(&tool.app, 12., &p, cx)),
            );
        }
        if self.tools.len() > max_dock {
            mini_dock = mini_dock.child(
                div()
                    .h(px(22.))
                    .px(px(5.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(format!("+{}", self.tools.len() - max_dock)),
            );
        }

        let agent_is_open = self.overlay == Some(MemoryGraphOverlay::AddMenu);
        let agent_aria: SharedString = format!(
            "{}, {} – {}. {}",
            self.localize_text(&self.agent.name, cx),
            self.localize_text(&self.agent.subtitle, cx),
            status_line,
            tr(cx, "Add files or tools", "点击添加文件或工具")
        )
        .into();

        let agent_btn = activate(
            div()
                .id(pro_eid(&self.id, "mg-agent"))
                .role(Role::Button)
                .aria_label(agent_aria)
                .aria_expanded(agent_is_open)
                .tab_index(0)
                .w(px(agent_w))
                .h(px(agent_h))
                .p(px(15.))
                .rounded(px(8.))
                .bg(card_bg)
                .border_1()
                .border_color(if agent_is_open {
                    hot_border
                } else {
                    p.line_strong
                })
                .shadow_sm()
                .hover(move |s| s.border_color(hover_border))
                .focus_visible(move |s| s.border_2().border_color(focus_col))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .cursor_pointer()
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .min_w_0()
                        .child(pro_agent_avatar(
                            &self.agent.id,
                            light_blue,
                            30.,
                            is_busy,
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
                                        .line_height(px(18.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(self.localize_text(&self.agent.name, cx).to_owned()),
                                )
                                .child(if is_busy {
                                    pro_shimmer_text(
                                        pro_eid(&self.id, "agent-st-shim"),
                                        status_line.clone(),
                                        p.muted,
                                        p.strong,
                                        12.,
                                        FontWeight::NORMAL,
                                        true,
                                    )
                                } else {
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(status_line.clone())
                                        .into_any_element()
                                }),
                        ),
                )
                .child(mini_dock),
            cx,
            |this, _, cx| {
                this.set_overlay(Some(MemoryGraphOverlay::AddMenu), cx);
            },
        );

        // Determine visible memory slice (up to 6 slots; if > 6, 5 cards + "+N more" cycler)
        let mem_overflow = self.memories.len() > 6;
        let mem_take = if mem_overflow { 5 } else { self.memories.len() };
        let mem_start = if mem_overflow {
            (self.mem_page * 5) % self.memories.len().max(1)
        } else {
            0
        };
        let shown_memories: Vec<MemoryNode> = self
            .memories
            .iter()
            .cycle()
            .skip(mem_start)
            .take(mem_take.min(self.memories.len()))
            .cloned()
            .collect();

        let tool_overflow = self.tools.len() > 6;
        let shown_tools: Vec<MemoryTool> = self
            .tools
            .iter()
            .take(if tool_overflow { 5 } else { 6 })
            .cloned()
            .collect();

        let card_w = if stacked {
            stage_w
        } else {
            ((stage_w - agent_w - 92.0) * 0.5_f32).clamp(168.0, 208.0)
        };
        let card_h = 74.0_f32;
        let stage_h = 564.0_f32;

        // Wash intensity helper (`Gradient Floor` rising from bottom edge when card is learned/used)
        let card_wash_alpha = |cid: &SharedString| -> f32 {
            if reduced {
                return 0.0;
            }
            if let Some((wid, started)) = &self.wash_card
                && wid == cid
            {
                let age = started.elapsed().as_secs_f32();
                if age < 1.0 {
                    let env = if age < 0.25 {
                        age / 0.25
                    } else {
                        (1.0 - (age - 0.25) / 0.75).clamp(0.0, 1.0)
                    };
                    return env * env;
                }
            }
            0.0
        };

        let render_mem_card = |mem: &MemoryNode, cx: &mut Context<Self>| -> Stateful<Div> {
            let mid = mem.id.clone();
            let hot = recalling.contains(&mid) && !mem.disabled;
            let open = self.overlay == Some(MemoryGraphOverlay::Memory(mid.clone()));
            let linked = self.hovered_memory.as_ref().is_some_and(|hid| {
                self.memories
                    .iter()
                    .find(|m| &m.id == hid)
                    .is_some_and(|hm| {
                        hm.related.contains(&mid)
                            || hm.replaces.as_ref() == Some(&mid)
                            || mem.replaces.as_ref() == Some(hid)
                    })
            });
            let wash = card_wash_alpha(&mid);
            let bg = if hot { hot_bg } else { card_bg };
            let bdr = if hot || open {
                hot_border
            } else if linked {
                light_blue.opacity(0.45)
            } else {
                p.line
            };
            let op = if mem.disabled { 0.38_f32 } else { 1.0_f32 };
            let aria: SharedString = format!(
                "{}, from {}",
                self.localize_text(&mem.text, cx),
                self.localize_text(&mem.artifact.name, cx)
            )
            .into();
            let mid_click = mid.clone();
            let mid_hov = mid.clone();

            activate(
                div()
                    .id(pro_eid(&self.id, format!("mem-{mid}")))
                    .role(Role::Button)
                    .aria_label(aria)
                    .aria_expanded(open)
                    .tab_index(0)
                    .w(px(card_w))
                    .h(px(card_h))
                    .px(px(11.))
                    .relative()
                    .overflow_hidden()
                    .rounded(px(8.))
                    .bg(bg)
                    .border_1()
                    .border_color(bdr)
                    .opacity(op)
                    .when(!p.dark && !mem.outdated, |d| d.shadow_xs())
                    .hover(move |s| s.border_color(hover_border).opacity(op.max(0.62)))
                    .focus_visible(move |s| s.border_2().border_color(focus_col))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = (*inside).then(|| mid_hov.clone());
                        if this.hovered_memory != next {
                            this.hovered_memory = next;
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(2.))
                    .when(wash > 0.01, |d| {
                        d.child(div().absolute().inset_0().bg(linear_gradient(
                            0.,
                            linear_color_stop(light_blue.opacity(0.32 * wash), 0.0),
                            linear_color_stop(transparent_black(), 0.78),
                        )))
                    })
                    .child(
                        div()
                            .relative()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if mem.outdated { p.muted } else { p.strong })
                            .truncate()
                            .child(self.localize_text(&mem.text, cx).to_owned()),
                    )
                    .child(
                        div()
                            .relative()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .truncate()
                            .child(self.localize_text(&mem.source, cx).to_owned()),
                    )
                    .child(
                        div()
                            .relative()
                            .mt(px(3.))
                            .h(px(18.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(
                                div()
                                    .opacity(if mem.outdated { 0.45 } else { 1.0 })
                                    .child(artifact_icon(&mem.artifact, 15., p.muted, cx)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.text)
                                    .child(self.localize_text(&mem.artifact.name, cx).to_owned()),
                            ),
                    ),
                cx,
                move |this, _, cx| {
                    this.set_overlay(Some(MemoryGraphOverlay::Memory(mid_click.clone())), cx);
                },
            )
        };

        let render_tool_card = |tool: &MemoryTool, cx: &mut Context<Self>| -> Stateful<Div> {
            let tid = tool.id.clone();
            let hot = using.contains(&tid) && !tool.disabled;
            let open = self.overlay == Some(MemoryGraphOverlay::Tool(tid.clone()));
            let wash = card_wash_alpha(&tid);
            let bg = if hot { hot_bg } else { card_bg };
            let bdr = if hot || open { hot_border } else { p.line };
            let op = if tool.disabled { 0.38_f32 } else { 1.0_f32 };
            let when_label = if tool.disabled {
                tr(cx, "Off", "已停用").to_owned()
            } else if hot {
                tr(cx, "In use", "使用中").to_owned()
            } else {
                self.localize_text(&tool.when, cx).to_owned()
            };
            let aria: SharedString =
                format!("{}, {}, {}", tool.name, tool.access, tool.scope).into();
            let tid_click = tid.clone();

            activate(
                div()
                    .id(pro_eid(&self.id, format!("mtool-{tid}")))
                    .role(Role::Button)
                    .aria_label(aria)
                    .aria_expanded(open)
                    .tab_index(0)
                    .w(px(card_w))
                    .h(px(card_h))
                    .px(px(11.))
                    .relative()
                    .overflow_hidden()
                    .rounded(px(8.))
                    .bg(bg)
                    .border_1()
                    .border_color(bdr)
                    .opacity(op)
                    .when(!p.dark, |d| d.shadow_xs())
                    .hover(move |s| s.border_color(hover_border).opacity(op.max(0.62)))
                    .focus_visible(move |s| s.border_2().border_color(focus_col))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(2.))
                    .when(wash > 0.01, |d| {
                        d.child(div().absolute().inset_0().bg(linear_gradient(
                            0.,
                            linear_color_stop(light_blue.opacity(0.30 * wash), 0.0),
                            linear_color_stop(transparent_black(), 0.78),
                        )))
                    })
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(tool_mark(&tool.app, 14., &p, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .line_height(px(18.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tool.name.clone()),
                            )
                            .child(if hot {
                                pro_shimmer_text(
                                    pro_eid(&self.id, format!("twhen-{tid}")),
                                    when_label,
                                    p.muted,
                                    p.strong,
                                    11.,
                                    FontWeight::MEDIUM,
                                    true,
                                )
                            } else {
                                div()
                                    .flex_none()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(when_label)
                                    .into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .relative()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.text)
                            .truncate()
                            .child(format!(
                                "{} · {}",
                                self.localize_text(&tool.access, cx),
                                tool.scope
                            )),
                    )
                    .child(
                        div()
                            .relative()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .truncate()
                            .child(self.localize_text(&tool.note, cx).to_owned()),
                    ),
                cx,
                move |this, _, cx| {
                    this.set_overlay(Some(MemoryGraphOverlay::Tool(tid_click.clone())), cx);
                },
            )
        };

        // Popover layer rendered via pro_presence so opening and closing both animate cleanly
        let active_or_closing_ov = self.overlay.clone().or_else(|| self.last_overlay.clone());
        let is_ov_open = self.overlay.is_some();

        let stage_el = if stacked {
            let mut col = div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(agent_btn);
            if let Some(ov) = &active_or_closing_ov
                && is_ov_open
            {
                let pop = self.render_overlay_card(ov, &using, &recalling, &p, cx);
                col = col.child(div().w_full().flex().justify_center().child(pop));
            }
            col = col.child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .child(tr(cx, "Memories", "记忆条目")),
            );
            for m in &shown_memories {
                col = col.child(render_mem_card(m, cx));
            }
            col = col.child(
                div()
                    .pt(px(4.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .child(tr(cx, "Connected tools", "已连接工具")),
            );
            for t in &shown_tools {
                col = col.child(render_tool_card(t, cx));
            }
            col.into_any_element()
        } else {
            let agent_x = (stage_w - agent_w) * 0.5_f32;
            let agent_y = (stage_h - agent_h) * 0.5_f32;

            // Precompute slot coordinates for wires, ports, and cards
            let mem_slots: Vec<(f32, f32, SharedString, bool, bool, bool)> = shown_memories
                .iter()
                .enumerate()
                .map(|(slot, m)| {
                    let (sx, sy) = ring_slot_pos(false, slot, stage_w, stage_h, card_w, card_h);
                    let hot = recalling.contains(&m.id) && !m.disabled;
                    (sx, sy, m.id.clone(), hot, m.disabled, m.outdated)
                })
                .collect();

            let tool_slots: Vec<(f32, f32, SharedString, bool, bool)> = shown_tools
                .iter()
                .enumerate()
                .map(|(slot, t)| {
                    let (sx, sy) = ring_slot_pos(true, slot, stage_w, stage_h, card_w, card_h);
                    let hot = using.contains(&t.id) && !t.disabled;
                    (sx, sy, t.id.clone(), hot, t.disabled)
                })
                .collect();

            // Related/superseded links on hover
            #[allow(clippy::type_complexity)]
            let mut hover_links: Vec<((f32, f32), (f32, f32), bool)> = Vec::new();
            if let Some(hid) = &self.hovered_memory
                && let Some(h_idx) = shown_memories.iter().position(|m| &m.id == hid)
            {
                let p_from = (mem_slots[h_idx].0, mem_slots[h_idx].1 + card_h * 0.5_f32);
                let hm = &shown_memories[h_idx];
                for (j, other) in shown_memories.iter().enumerate() {
                    if j == h_idx {
                        continue;
                    }
                    let is_rep = hm.replaces.as_ref() == Some(&other.id)
                        || other.replaces.as_ref() == Some(&hm.id);
                    if is_rep || hm.related.contains(&other.id) {
                        let p_to = (mem_slots[j].0, mem_slots[j].1 + card_h * 0.5_f32);
                        hover_links.push((p_from, p_to, is_rep));
                    }
                }
            }

            let m_slots_w = mem_slots.clone();
            let t_slots_w = tool_slots.clone();
            let amber_col = p.amber;
            let wires_canvas = canvas(
                |_, _, _| (),
                move |surface, _, window, _| {
                    let ox = f32::from(surface.origin.x);
                    let oy = f32::from(surface.origin.y);
                    let agent_rect = Bounds::new(
                        point(px(ox + agent_x), px(oy + agent_y)),
                        size(px(agent_w), px(agent_h)),
                    );

                    // Left memory connectors (agent -> memory card right edge)
                    for (slot, (sx, sy, _, hot, disabled, outdated)) in m_slots_w.iter().enumerate()
                    {
                        let (port_pt, vert_first) = agent_port_point(false, slot, agent_rect);
                        let card_pt = point(px(ox + *sx + card_w), px(oy + *sy + card_h * 0.5));
                        if *disabled {
                            paint_dashed_segment(
                                port_pt,
                                card_pt,
                                wire_col.opacity(0.28),
                                1.4,
                                window,
                            );
                        } else {
                            let base_c = if *hot {
                                light_blue.opacity(0.85)
                            } else if *outdated {
                                wire_col.opacity(0.45)
                            } else {
                                wire_col
                            };
                            pro_connector_pulse(
                                port_pt,
                                card_pt,
                                8.0,
                                vert_first,
                                base_c,
                                light_blue,
                                if *hot { pulse_t } else { None },
                                1.5,
                                window,
                            );
                        }
                        paint_ring_port(
                            port_pt,
                            if *hot { light_blue } else { wire_col },
                            card_bg,
                            window,
                        );
                    }

                    // Right tool connectors (agent -> tool card left edge)
                    for (slot, (sx, sy, _, hot, disabled)) in t_slots_w.iter().enumerate() {
                        let (port_pt, vert_first) = agent_port_point(true, slot, agent_rect);
                        let card_pt = point(px(ox + *sx), px(oy + *sy + card_h * 0.5));
                        if *disabled {
                            paint_dashed_segment(
                                port_pt,
                                card_pt,
                                wire_col.opacity(0.28),
                                1.4,
                                window,
                            );
                        } else {
                            pro_connector_pulse(
                                port_pt,
                                card_pt,
                                8.0,
                                vert_first,
                                if *hot {
                                    light_blue.opacity(0.85)
                                } else {
                                    wire_col
                                },
                                light_blue,
                                if *hot { pulse_t } else { None },
                                1.5,
                                window,
                            );
                        }
                        paint_ring_port(
                            port_pt,
                            if *hot { light_blue } else { wire_col },
                            card_bg,
                            window,
                        );
                    }

                    // Outer dashed relationship / superseding arc when hovering a memory
                    for (a, b, is_rep) in &hover_links {
                        let outer_x = ox + a.0.min(b.0) - 10.0;
                        let pa = point(px(ox + a.0), px(oy + a.1));
                        let pm1 = point(px(outer_x), px(oy + a.1));
                        let pm2 = point(px(outer_x), px(oy + b.1));
                        let pb = point(px(ox + b.0), px(oy + b.1));
                        let col = if *is_rep {
                            amber_col.opacity(0.75)
                        } else {
                            light_blue.opacity(0.65)
                        };
                        paint_dashed_segment(pa, pm1, col, 1.4, window);
                        paint_dashed_segment(pm1, pm2, col, 1.4, window);
                        paint_dashed_segment(pm2, pb, col, 1.4, window);
                    }
                },
            )
            .absolute()
            .inset_0();

            let mut canvas_stage = div()
                .w_full()
                .h(px(stage_h))
                .relative()
                .child(wires_canvas)
                .child(
                    div()
                        .absolute()
                        .left(px(agent_x))
                        .top(px(agent_y))
                        .child(agent_btn),
                );

            for (slot, mem) in shown_memories.iter().enumerate() {
                let (sx, sy, _, _, _, _) = mem_slots[slot];
                canvas_stage = canvas_stage.child(
                    div()
                        .absolute()
                        .left(px(sx))
                        .top(px(sy))
                        .child(render_mem_card(mem, cx)),
                );
            }

            if mem_overflow {
                let (sx, sy) = ring_slot_pos(false, 5, stage_w, stage_h, card_w, card_h);
                let extra = self.memories.len() - shown_memories.len();
                let more_lbl = if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("另外 {extra} 条记忆 · 点击轮换")
                } else {
                    format!("+{extra} more · click to cycle")
                };
                let more_btn = activate(
                    div()
                        .id(pro_eid(&self.id, "mem-more"))
                        .role(Role::Button)
                        .aria_label(more_lbl.clone())
                        .tab_index(0)
                        .w(px(card_w))
                        .h(px(card_h))
                        .px(px(12.))
                        .rounded(px(8.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .hover(move |s| s.border_color(hover_border))
                        .focus_visible(move |s| s.border_2().border_color(focus_col))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child(more_lbl),
                    cx,
                    |this, _, cx| {
                        this.mem_page = this.mem_page.wrapping_add(1);
                        cx.notify();
                    },
                );
                canvas_stage =
                    canvas_stage.child(div().absolute().left(px(sx)).top(px(sy)).child(more_btn));
            }

            for (slot, tool) in shown_tools.iter().enumerate() {
                let (sx, sy, _, _, _) = tool_slots[slot];
                canvas_stage = canvas_stage.child(
                    div()
                        .absolute()
                        .left(px(sx))
                        .top(px(sy))
                        .child(render_tool_card(tool, cx)),
                );
            }

            // Positioned detail / Add popover anchored smartly near the selected card
            if let Some(ov) = &active_or_closing_ov {
                let pop_card = self.render_overlay_card(ov, &using, &recalling, &p, cx);
                let (pop_x, pop_y) = match ov {
                    MemoryGraphOverlay::AddMenu => (
                        ((stage_w - 272.0) * 0.5_f32).max(8.0),
                        (agent_y + agent_h + 10.0).min(stage_h - 260.0),
                    ),
                    MemoryGraphOverlay::Memory(mid) => {
                        if let Some((sx, sy, _, _, _, _)) =
                            mem_slots.iter().find(|(_, _, id, _, _, _)| id == mid)
                        {
                            let px_x = (*sx + card_w + 10.0).min(stage_w - 280.0);
                            let px_y = sy.clamp(8.0, stage_h - 248.0);
                            (px_x, px_y)
                        } else {
                            ((stage_w - 272.0) * 0.5_f32, 120.0)
                        }
                    }
                    MemoryGraphOverlay::Tool(tid) => {
                        if let Some((sx, sy, _, _, _)) =
                            tool_slots.iter().find(|(_, _, id, _, _)| id == tid)
                        {
                            let px_x = (*sx - 282.0).max(8.0);
                            let px_y = sy.clamp(8.0, stage_h - 230.0);
                            (px_x, px_y)
                        } else {
                            ((stage_w - 272.0) * 0.5_f32, 120.0)
                        }
                    }
                };
                canvas_stage = canvas_stage.child(pro_presence(
                    pro_eid(&self.id, "mg-ov-pres"),
                    is_ov_open,
                    0.18,
                    0.13,
                    move |amt| {
                        div()
                            .absolute()
                            .left(px(pop_x))
                            .top(px(pop_y - 3.0 * (1.0 - amt)))
                            .opacity(amt)
                            .child(pop_card)
                            .into_any_element()
                    },
                ));
            }

            canvas_stage.into_any_element()
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_text(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && this.overlay.is_some() {
                    this.set_overlay(None, cx);
                    this.focus.focus(w, cx);
                }
            }))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                if this.overlay.is_some() {
                    this.set_overlay(None, cx);
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .gap(px(12.))
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
            .child(top_bar)
            .child(stage_el)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryArtifact, MemoryGraph, MemoryGraphOverlay, MemoryNode, MemoryNodeKind};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn memory_graph_toggles_forgets_and_adds_files_and_tools(cx: &mut TestAppContext) {
        let view = cx.new(|cx| MemoryGraph::new("test-memory-graph", cx));

        view.update(cx, |mg, cx| {
            assert_eq!(mg.memory_count(), 4);
            assert_eq!(mg.tool_count(), 5);

            mg.set_overlay(Some(MemoryGraphOverlay::AddMenu), cx);
            assert_eq!(mg.active_overlay(), Some(&MemoryGraphOverlay::AddMenu));

            mg.add_sample_file(cx);
            assert_eq!(mg.memory_count(), 5);
            assert_eq!(mg.active_overlay(), None);

            assert!(mg.connect_catalog_tool("slack", cx));
            assert_eq!(mg.tool_count(), 6);

            assert_eq!(
                mg.toggle_node(MemoryNodeKind::Tool, "figma", cx),
                Some(false)
            );
            assert!(mg.forget_memory("m1", cx));
            assert_eq!(mg.memory_count(), 4);
        });

        let old = MemoryNode::new(
            "old",
            "Ships Nov 28",
            "Initial plan",
            MemoryArtifact::file("plan.md"),
        );
        let newer = MemoryNode::new(
            "new",
            "Ships Dec 4",
            "Updated plan",
            MemoryArtifact::file("plan.md"),
        )
        .with_replaces("old");
        let superseding =
            cx.new(|cx| MemoryGraph::new("sup-mg", cx).with_memories(vec![old, newer]));
        superseding.read_with(cx, |mg, _| {
            assert!(mg.memories[0].outdated);
        });
    }
}
