//! Multiplayer document canvas with gliding AI agent pointers, inline suggestions, image handles, and margin comments.
//!
//! Renders a collaborative document and table stage where multiple AI agents work like teammates:
//! smooth spring-gliding [`PathBuilder`] pointers with pill nametags, letter-by-letter prose and code
//! patches, corner-handled image resizing, and expandable margin comment pins.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_swap_label,
        pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::f32::consts::{FRAC_PI_2, TAU};
use web_time::Instant;

const DOC_ICON_SVG: &str = r#"<path d="M14 2.5H7.5a2 2 0 0 0-2 2v15a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2V7z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"></path><path d="M14 2.5V7h4.5M9 12.5h6M9 16h4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Events emitted by [`AgentCursors`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentCursorsEvent {
    /// User accepted one inline suggestion, splicing it into the document blocks.
    AcceptSuggestion(DocSuggestion),
    /// User rejected and dismissed one inline suggestion.
    RejectSuggestion(DocSuggestion),
    /// User clicked `Accept all`, applying every pending suggestion (`count` applied).
    AcceptAll(usize),
    /// User resolved a margin comment card.
    ResolveComment(DocComment),
    /// User posted a quick reply onto a margin comment card.
    ReplyComment {
        comment_id: SharedString,
        text: SharedString,
    },
    /// User toggled following a specific agent (`None` when unfollowed).
    FollowChanged(Option<SharedString>),
    /// User switched between top views (such as `"doc"` and `"table"`).
    TabChanged(SharedString),
    /// User appended a collaborative row in the table view.
    RowAdded(usize),
}

/// Collaborating AI teammate shown in the header bar, document pointers, and comment pins.
#[derive(Clone, Debug, PartialEq)]
pub struct CursorAgent {
    /// Stable agent identifier referenced by cursors, suggestions, and comments.
    pub id: SharedString,
    /// Display name shown on the pointer flag and tooltips.
    pub name: SharedString,
    /// Optional Chinese display name for bilingual surfaces.
    pub name_zh: Option<SharedString>,
    /// Optional model/provider mark key (`"claude"`, `"openai"`, `"gemini"`, etc.).
    pub provider: Option<SharedString>,
    /// Optional signature cursor and highlight color; falls back to shared ink when `None`.
    pub color: Option<Hsla>,
    /// Optional soft tint fill when this agent's header avatar is followed.
    pub tint: Option<Hsla>,
    /// Accumulated USD cost displayed on the cursor flag and summed in the header.
    pub cost: Option<f64>,
}

impl CursorAgent {
    /// Create a [`CursorAgent`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            provider: None,
            color: None,
            tint: None,
            cost: None,
        }
    }

    /// Set the localized Chinese agent name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set the provider mark key rendered inside the agent badge.
    pub fn with_provider(mut self, provider: impl Into<SharedString>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Assign a distinct pointer, underline, and selection color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Assign a custom soft tint used when following this agent.
    pub fn with_tint(mut self, tint: Hsla) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Set the running USD spend attributed to this agent.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
        self
    }

    fn label<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }
}

/// Visual layout role of a [`DocBlock`] inside [`AgentCursors`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocBlockKind {
    /// Primary document title (`20px` medium).
    Title,
    /// Section heading (`14.5px` medium).
    Heading,
    /// Standard prose paragraph (`13.5px`).
    #[default]
    Paragraph,
    /// Bulleted list item grouped consecutively.
    Item,
    /// Monospace code block supporting inline strike/insert patches.
    Code,
    /// Resizable figure/image block with optional corner handles and dimension tag.
    Image,
    /// Structured data table block.
    Table,
}

/// Horizontal float alignment for [`DocBlockKind::Image`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFloat {
    /// Full-row block figure.
    #[default]
    None,
    /// Floats on the left while the next prose block flows alongside on the right.
    Left,
    /// Floats on the right while the next prose block flows alongside on the left.
    Right,
}

/// Semantic pill tone for a [`TableCell`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellTone {
    /// Plain text cell without pill background.
    #[default]
    None,
    /// Neutral gray status pill.
    Gray,
    /// Active blue status pill.
    Blue,
    /// Completed green status pill.
    Green,
    /// Warning amber status pill.
    Amber,
}

/// One cell inside a [`DocBlockKind::Table`] row.
#[derive(Clone, Debug, PartialEq)]
pub struct TableCell {
    /// Primary cell label.
    pub text: SharedString,
    /// Optional Chinese translation.
    pub text_zh: Option<SharedString>,
    /// Optional status pill tone.
    pub tone: CellTone,
}

impl TableCell {
    /// Create a plain text [`TableCell`].
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            text_zh: None,
            tone: CellTone::None,
        }
    }

    /// Attach a Chinese label for bilingual rendering.
    pub fn with_text_zh(mut self, text_zh: impl Into<SharedString>) -> Self {
        self.text_zh = Some(text_zh.into());
        self
    }

    /// Render this cell as a tinted status pill.
    pub fn with_tone(mut self, tone: CellTone) -> Self {
        self.tone = tone;
        self
    }

    fn display<'a>(&'a self, cx: &App) -> &'a str {
        match &self.text_zh {
            Some(zh) => tr(cx, self.text.as_ref(), zh.as_ref()),
            None => self.text.as_ref(),
        }
    }
}

/// Document block rendered inside [`AgentCursors`].
#[derive(Clone, Debug, PartialEq)]
pub struct DocBlock {
    /// Unique block identifier targeted by cursors, suggestions, and comments.
    pub id: SharedString,
    /// Block presentation kind.
    pub kind: DocBlockKind,
    /// Main text content (or image caption when `kind == DocBlockKind::Image`).
    pub text: SharedString,
    /// Optional Chinese text content.
    pub text_zh: Option<SharedString>,
    /// Fractional image width (`0.25..=1.0`) for [`DocBlockKind::Image`].
    pub width_ratio: f32,
    /// Image width-to-height aspect ratio (`3.4` default).
    pub aspect_ratio: f32,
    /// Optional left/right float behavior for [`DocBlockKind::Image`].
    pub float: ImageFloat,
    /// Whether an image block is still shimmering as loading.
    pub loading: bool,
    /// Structured rows when `kind == DocBlockKind::Table` (row `0` is header).
    pub rows: Vec<Vec<TableCell>>,
}

impl DocBlock {
    /// Create a [`DocBlock`] with `id`, `kind`, and `text`.
    pub fn new(
        id: impl Into<SharedString>,
        kind: DocBlockKind,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            text: text.into(),
            text_zh: None,
            width_ratio: 0.50,
            aspect_ratio: 3.45,
            float: ImageFloat::None,
            loading: false,
            rows: Vec::new(),
        }
    }

    /// Set localized Chinese prose for this block (used when no char-index suggestions target it).
    pub fn with_text_zh(mut self, text_zh: impl Into<SharedString>) -> Self {
        self.text_zh = Some(text_zh.into());
        self
    }

    /// Configure image sizing and float layout.
    pub fn with_image_layout(
        mut self,
        width_ratio: f32,
        aspect_ratio: f32,
        float: ImageFloat,
    ) -> Self {
        self.width_ratio = width_ratio.clamp(0.25, 1.0);
        self.aspect_ratio = aspect_ratio.max(0.5);
        self.float = float;
        self
    }

    /// Mark an image block as actively shimmering while loading.
    pub fn with_loading_shimmer(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Supply table rows (first row renders as column headers).
    pub fn with_rows(mut self, rows: Vec<Vec<TableCell>>) -> Self {
        self.rows = rows;
        self
    }
}

/// Handle anchor held on an image block when an agent resizes or inspects it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

/// Live pointer position and activity metadata for one [`CursorAgent`].
#[derive(Clone, Debug, PartialEq)]
pub struct LiveCursor {
    /// Target block or comment id.
    pub block: SharedString,
    /// Character index into `block` (or into `in_suggestion`'s insertion text).
    pub at: usize,
    /// Optional suggestion id when the agent is actively typing inside a suggestion.
    pub in_suggestion: Option<SharedString>,
    /// Selected character span length (`0` when collapsed caret).
    pub select: usize,
    /// Held corner or center handle when positioned on an image block.
    pub point: Option<ImageAnchor>,
    /// Short present-participle status shown on the cursor flag (`"Fixing the date"`).
    pub doing: Option<SharedString>,
    /// Optional Chinese translation of `doing`.
    pub doing_zh: Option<SharedString>,
    /// True while typing character-by-character so the caret stays solid and glide is crisp.
    pub typing: bool,
    /// Normalized horizontal anchor (`0.0..=1.0`) within the document canvas.
    pub x_ratio: f32,
    /// Vertical offset (`px`) from the top of the document canvas.
    pub y_px: f32,
}

impl LiveCursor {
    /// Place a [`LiveCursor`] on `block` at canvas coordinates `(x_ratio, y_px)`.
    pub fn new(block: impl Into<SharedString>, x_ratio: f32, y_px: f32) -> Self {
        Self {
            block: block.into(),
            at: 0,
            in_suggestion: None,
            select: 0,
            point: None,
            doing: None,
            doing_zh: None,
            typing: false,
            x_ratio: x_ratio.clamp(0.04, 0.92),
            y_px: y_px.max(12.0),
        }
    }

    /// Set character offset and optional selection length.
    pub fn with_span(mut self, at: usize, select: usize) -> Self {
        self.at = at;
        self.select = select;
        self
    }

    /// Position this cursor inside an active [`DocSuggestion`] insertion.
    pub fn with_suggestion(mut self, suggestion_id: impl Into<SharedString>, typing: bool) -> Self {
        self.in_suggestion = Some(suggestion_id.into());
        self.typing = typing;
        self
    }

    /// Attach an image corner/center resize handle target.
    pub fn with_image_point(mut self, point: ImageAnchor) -> Self {
        self.point = Some(point);
        self
    }

    /// Set the status subtitle displayed on the pointer's nametag flag.
    pub fn with_doing(
        mut self,
        doing: impl Into<SharedString>,
        doing_zh: impl Into<SharedString>,
    ) -> Self {
        self.doing = Some(doing.into());
        self.doing_zh = Some(doing_zh.into());
        self
    }

    fn doing_text<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.doing, &self.doing_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Inline prose or code edit proposed by an agent, awaiting user acceptance.
#[derive(Clone, Debug, PartialEq)]
pub struct DocSuggestion {
    /// Unique suggestion identifier.
    pub id: SharedString,
    /// Proposing agent id.
    pub agent: SharedString,
    /// Target [`DocBlock`] id.
    pub block: SharedString,
    /// Character offset in `block.text` where `remove` starts.
    pub at: usize,
    /// Existing substring struck through (`""` for pure insertion).
    pub remove: SharedString,
    /// Proposed replacement substring underlined in the agent's color.
    pub insert: SharedString,
    /// One-sentence rationale shown inside the suggestion review popover.
    pub note: Option<SharedString>,
    /// Optional Chinese translation of `note`.
    pub note_zh: Option<SharedString>,
}

impl DocSuggestion {
    /// Create a [`DocSuggestion`] replacing `remove` with `insert` at char index `at`.
    pub fn new(
        id: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        block: impl Into<SharedString>,
        at: usize,
        remove: impl Into<SharedString>,
        insert: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            agent: agent.into(),
            block: block.into(),
            at,
            remove: remove.into(),
            insert: insert.into(),
            note: None,
            note_zh: None,
        }
    }

    /// Attach a review note explaining why the agent suggested this edit.
    pub fn with_note(
        mut self,
        note: impl Into<SharedString>,
        note_zh: impl Into<SharedString>,
    ) -> Self {
        self.note = Some(note.into());
        self.note_zh = Some(note_zh.into());
        self
    }

    fn note_text<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.note, &self.note_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Margin comment pin placed beside a document block.
#[derive(Clone, Debug, PartialEq)]
pub struct DocComment {
    /// Unique comment id.
    pub id: SharedString,
    /// Authoring agent id.
    pub agent: SharedString,
    /// Associated [`DocBlock`] id.
    pub block: SharedString,
    /// Normalized horizontal pin anchor (`0.0..=1.0`).
    pub x: f32,
    /// Vertical offset (`px`) from the top of the document canvas.
    pub y: f32,
    /// Comment message body.
    pub text: SharedString,
    /// Optional Chinese translation of `text`.
    pub text_zh: Option<SharedString>,
    /// Whether the comment pin is expanded into its full card.
    pub open: bool,
    /// Quick replies posted to this comment thread.
    pub replies: Vec<SharedString>,
}

impl DocComment {
    /// Create a [`DocComment`] anchored at `(x, y)` on `block`.
    pub fn new(
        id: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        block: impl Into<SharedString>,
        x: f32,
        y: f32,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            agent: agent.into(),
            block: block.into(),
            x: x.clamp(0.02, 0.94),
            y: y.max(24.0),
            text: text.into(),
            text_zh: None,
            open: false,
            replies: Vec::new(),
        }
    }

    /// Set localized Chinese comment text.
    pub fn with_text_zh(mut self, text_zh: impl Into<SharedString>) -> Self {
        self.text_zh = Some(text_zh.into());
        self
    }

    /// Control whether the comment card starts expanded.
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    fn body<'a>(&'a self, cx: &App) -> &'a str {
        match &self.text_zh {
            Some(zh) => tr(cx, self.text.as_ref(), zh.as_ref()),
            None => self.text.as_ref(),
        }
    }
}

/// Top view tab descriptor (`Doc`, `Table`, etc.).
#[derive(Clone, Debug, PartialEq)]
pub struct DocTab {
    /// Tab key.
    pub id: SharedString,
    /// English tab label.
    pub label: SharedString,
    /// Optional Chinese tab label.
    pub label_zh: Option<SharedString>,
}

impl DocTab {
    /// Create a [`DocTab`] with `id` and `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            label_zh: None,
        }
    }

    /// Attach a localized Chinese label.
    pub fn with_label_zh(mut self, label_zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(label_zh.into());
        self
    }

    fn text<'a>(&'a self, cx: &App) -> &'a str {
        match &self.label_zh {
            Some(zh) => tr(cx, self.label.as_ref(), zh.as_ref()),
            None => self.label.as_ref(),
        }
    }
}

/// Apply suggestion `id` into `blocks`, removing it from `suggestions` and shifting sibling character offsets.
pub fn accept_suggestion(
    blocks: &[DocBlock],
    suggestions: &[DocSuggestion],
    id: &str,
) -> (Vec<DocBlock>, Vec<DocSuggestion>) {
    let Some(target) = suggestions.iter().find(|s| s.id.as_ref() == id).cloned() else {
        return (blocks.to_vec(), suggestions.to_vec());
    };
    let rem_chars = target.remove.chars().count();
    let ins_chars = target.insert.chars().count();
    let delta = ins_chars as isize - rem_chars as isize;

    let mut next_blocks = blocks.to_vec();
    for blk in &mut next_blocks {
        if blk.id == target.block {
            let chars: Vec<char> = blk.text.chars().collect();
            let start = target.at.min(chars.len());
            let end = (start + rem_chars).min(chars.len());
            let mut updated = String::with_capacity(blk.text.len() + target.insert.len());
            for &ch in &chars[..start] {
                updated.push(ch);
            }
            updated.push_str(target.insert.as_ref());
            for &ch in &chars[end..] {
                updated.push(ch);
            }
            blk.text = SharedString::from(updated);
            // Clear static translation once underlying source string has been spliced.
            blk.text_zh = None;
        }
    }

    let mut next_suggestions = Vec::with_capacity(suggestions.len().saturating_sub(1));
    for mut s in suggestions.iter().cloned() {
        if s.id == target.id {
            continue;
        }
        if s.block == target.block && s.at > target.at {
            s.at = (s.at as isize + delta).max(0) as usize;
        }
        next_suggestions.push(s);
    }
    (next_blocks, next_suggestions)
}

/// Pair of `(agent_id, LiveCursor)` specifying an agent's active canvas pointer.
pub type CursorEntry = (SharedString, LiveCursor);

struct GlidingPointerState {
    tx: f32,
    ty: f32,
    dimmed: bool,
    x: Slide,
    y: Slide,
    alpha: Slide,
}

#[derive(IntoElement)]
struct GlidingPointer {
    id: ElementId,
    target_x: f32,
    target_y: f32,
    glide_secs: f32,
    dimmed: bool,
    typing: bool,
    free_handle: bool,
    caret_blink_on: bool,
    color: Hsla,
    dark: bool,
    name: SharedString,
    cost_tag: Option<SharedString>,
    doing: Option<SharedString>,
    pink: Hsla,
    blue: Hsla,
}

impl RenderOnce for GlidingPointer {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let tx = self.target_x;
        let ty = self.target_y;
        let init_dim = self.dimmed;
        let init_alpha = if init_dim { 0.22 } else { 1.0 };
        let st = w.use_keyed_state(pro_eid(&self.id, "glide"), cx, |_, _| GlidingPointerState {
            tx,
            ty,
            dimmed: init_dim,
            x: Slide::new(tx),
            y: Slide::new(ty),
            alpha: Slide::snappy(init_alpha),
        });
        let (cur_x, cur_y, ptr_alpha) = st.update(cx, |s, cx| {
            if (s.tx - tx).abs() > 0.4 || (s.ty - ty).abs() > 0.4 {
                s.tx = tx;
                s.ty = ty;
                s.x.duration(tx, self.glide_secs, cx);
                s.y.duration(ty, self.glide_secs, cx);
            }
            if s.dimmed != self.dimmed {
                s.dimmed = self.dimmed;
                s.alpha
                    .duration(if self.dimmed { 0.22 } else { 1.0 }, 0.20, cx);
            }
            (
                s.x.get(w, cx),
                s.y.get(w, cx),
                s.alpha.get(w, cx).clamp(0.18, 1.0),
            )
        });

        let flag_bg: Hsla = if self.dark {
            rgb(0xf4f4f5).into()
        } else {
            rgb(0x141416).into()
        };
        let flag_ink: Hsla = if self.dark {
            rgb(0x141416).into()
        } else {
            rgb(0xffffff).into()
        };
        let flag_soft: Hsla = if self.dark {
            hsla(0., 0., 0., 0.08)
        } else {
            hsla(0., 0., 1., 0.14)
        };
        let arrow_outline: Hsla = if self.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let arrow_fill = self.color;

        let arrow_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.left());
                let oy = f32::from(bounds.top());
                let pts = [
                    point(px(ox + 1.8), px(oy + 1.2)),
                    point(px(ox + 15.2), px(oy + 7.2)),
                    point(px(ox + 8.8), px(oy + 9.8)),
                    point(px(ox + 5.9), px(oy + 16.4)),
                ];
                let mut stroke_b = PathBuilder::stroke(px(2.4));
                stroke_b.move_to(pts[0]);
                stroke_b.line_to(pts[1]);
                stroke_b.line_to(pts[2]);
                stroke_b.line_to(pts[3]);
                stroke_b.close();
                if let Ok(path) = stroke_b.build() {
                    window.paint_path(path, arrow_outline);
                }

                let mut fill_b = PathBuilder::fill();
                fill_b.move_to(pts[0]);
                fill_b.line_to(pts[1]);
                fill_b.line_to(pts[2]);
                fill_b.line_to(pts[3]);
                fill_b.close();
                if let Ok(path) = fill_b.build() {
                    window.paint_path(path, arrow_fill);
                }
            },
        )
        .w(px(17.0))
        .h(px(19.0));

        let pink = self.pink;
        let blue = self.blue;
        let edge_bar = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                window.paint_quad(fill(
                    bounds,
                    linear_gradient(
                        90.0,
                        linear_color_stop(pink, 0.0),
                        linear_color_stop(blue, 1.0),
                    ),
                ));
            },
        )
        .w_full()
        .h(px(1.5));

        let show_caret = !self.free_handle && (self.typing || self.caret_blink_on);
        let arrow_top = if self.free_handle { 0.0 } else { 11.0 };
        let flag_top = if self.free_handle { 18.0 } else { 28.0 };

        div()
            .absolute()
            .left(px(cur_x))
            .top(px(cur_y))
            .opacity(ptr_alpha)
            .when(!self.free_handle, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(-1.0))
                        .top(px(0.0))
                        .w(px(1.8))
                        .h(px(16.0))
                        .rounded(px(1.0))
                        .bg(self.color.opacity(if show_caret { 1.0 } else { 0.0 })),
                )
            })
            .child(
                div()
                    .absolute()
                    .left(px(if self.free_handle { -1.5 } else { 0.0 }))
                    .top(px(arrow_top))
                    .child(arrow_canvas),
            )
            .child(
                div()
                    .absolute()
                    .left(px(14.0))
                    .top(px(flag_top))
                    .rounded(px(8.0))
                    .overflow_hidden()
                    .bg(flag_bg)
                    .border_1()
                    .border_color(flag_soft)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(8.5))
                            .py(px(4.5))
                            .flex()
                            .flex_col()
                            .gap(px(1.5))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.0))
                                    .whitespace_nowrap()
                                    .child(div().size(px(6.0)).rounded_full().bg(self.color))
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(flag_ink)
                                            .child(self.name),
                                    )
                                    .when_some(self.cost_tag, |row, tag| {
                                        row.child(
                                            div()
                                                .px(px(4.5))
                                                .h(px(15.0))
                                                .rounded(px(4.0))
                                                .bg(flag_soft)
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(flag_ink)
                                                .child(tag),
                                        )
                                    }),
                            )
                            .when(!self.dimmed, |col| {
                                col.when_some(self.doing, |col, doing_str| {
                                    col.child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::NORMAL)
                                            .text_color(flag_ink.opacity(0.78))
                                            .whitespace_nowrap()
                                            .child(doing_str),
                                    )
                                })
                            }),
                    )
                    .child(edge_bar),
            )
    }
}

/// Collaborative multiplayer-editor surface showing live AI agent pointers, patches, image handles, and margin comments.
pub struct AgentCursors {
    id: ElementId,
    title: SharedString,
    title_zh: Option<SharedString>,
    agents: Vec<CursorAgent>,
    blocks: Vec<DocBlock>,
    table_rows: Vec<Vec<TableCell>>,
    suggestions: Vec<DocSuggestion>,
    comments: Vec<DocComment>,
    cursors: Vec<CursorEntry>,
    tabs: Vec<DocTab>,
    active_tab: SharedString,
    following: Option<SharedString>,
    hovered_agent: Option<SharedString>,
    open_suggestion: Option<SharedString>,
    last_open_sug: Option<DocSuggestion>,
    toast: Option<(SharedString, Instant)>,
    fixed_height: Option<f32>,
    live_demo: bool,
    loading: bool,
    empty: bool,
    started_at: Instant,
}

impl EventEmitter<AgentCursorsEvent> for AgentCursors {}

impl AgentCursors {
    /// Create an [`AgentCursors`] surface preloaded with the `Pulse 2.0 – launch brief` collaborative team run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: SharedString::from("Product / Launches"),
            title_zh: Some(SharedString::from("产品 / 发布简报")),
            agents: default_agents(),
            blocks: default_doc_blocks(),
            table_rows: default_table_rows(),
            suggestions: default_suggestions(),
            comments: default_comments(),
            cursors: default_cursors(),
            tabs: vec![
                DocTab::new("doc", "Doc").with_label_zh("文档"),
                DocTab::new("table", "Table").with_label_zh("表格"),
            ],
            active_tab: SharedString::from("doc"),
            following: None,
            hovered_agent: None,
            open_suggestion: None,
            last_open_sug: None,
            toast: None,
            fixed_height: None,
            live_demo: true,
            loading: false,
            empty: false,
            started_at: Instant::now(),
        }
    }

    /// Set the document breadcrumb or title shown in the header bar.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self.title_zh = None;
        self
    }

    /// Set a localized Chinese header title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Replace the roster of collaborating agents.
    pub fn with_agents(mut self, agents: Vec<CursorAgent>) -> Self {
        self.agents = agents;
        self
    }

    /// Replace the document block sequence.
    pub fn with_blocks(mut self, blocks: Vec<DocBlock>) -> Self {
        self.blocks = blocks;
        self
    }

    /// Replace the pending inline suggestions.
    pub fn with_suggestions(mut self, suggestions: Vec<DocSuggestion>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// Replace the margin comment pins.
    pub fn with_comments(mut self, comments: Vec<DocComment>) -> Self {
        self.comments = comments;
        self
    }

    /// Supply explicit per-agent cursor states (`(agent_id, LiveCursor)`), disabling automatic demo cycling.
    pub fn with_cursors(mut self, cursors: Vec<CursorEntry>) -> Self {
        self.cursors = cursors;
        self.live_demo = false;
        self
    }

    /// Configure the top view switcher tabs.
    pub fn with_tabs(mut self, tabs: Vec<DocTab>) -> Self {
        self.tabs = tabs;
        self
    }

    /// Set the initially selected tab key (`"doc"` or `"table"`).
    pub fn with_active_tab(mut self, tab: impl Into<SharedString>) -> Self {
        self.active_tab = tab.into();
        self
    }

    /// Start following a specific agent by id (`None` shows all agents at full opacity).
    pub fn with_following(mut self, agent_id: Option<SharedString>) -> Self {
        self.following = agent_id;
        self
    }

    /// Constrain the document sheet height in pixels.
    pub fn with_height(mut self, height: f32) -> Self {
        self.fixed_height = Some(height.max(260.0));
        self
    }

    /// Enable or disable live multi-agent typing, hero image resizing, and pointer gliding.
    pub fn with_live_demo(mut self, live: bool) -> Self {
        self.live_demo = live;
        self
    }

    /// Show skeleton placeholder bars while loading document state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show an empty document state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current document blocks.
    pub fn blocks(&self) -> &[DocBlock] {
        &self.blocks
    }

    /// Return pending inline suggestions.
    pub fn suggestions(&self) -> &[DocSuggestion] {
        &self.suggestions
    }

    /// Return open/pinned margin comments.
    pub fn comments(&self) -> &[DocComment] {
        &self.comments
    }

    /// Return the currently followed agent id, if any.
    pub fn following(&self) -> Option<&str> {
        self.following.as_deref()
    }

    /// Toggle or set the followed agent and emit [`AgentCursorsEvent::FollowChanged`].
    pub fn toggle_follow(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        if self.following.as_deref() == Some(agent_id) {
            self.following = None;
        } else {
            self.following = Some(SharedString::from(agent_id.to_string()));
        }
        cx.emit(AgentCursorsEvent::FollowChanged(self.following.clone()));
        cx.notify();
    }

    /// Accept one pending suggestion by id, splicing its insertion into `self.blocks`.
    pub fn accept_one(&mut self, suggestion_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self
            .suggestions
            .iter()
            .find(|s| s.id.as_ref() == suggestion_id)
            .cloned()
        else {
            return false;
        };
        self.last_open_sug = Some(found.clone());
        let (next_blocks, next_suggestions) =
            accept_suggestion(&self.blocks, &self.suggestions, suggestion_id);
        self.blocks = next_blocks;
        self.suggestions = next_suggestions;
        if self.open_suggestion.as_deref() == Some(suggestion_id) {
            self.open_suggestion = None;
        }
        self.toast = Some((
            tr(cx, "Suggestion accepted", "已采纳修改建议").into(),
            Instant::now(),
        ));
        cx.emit(AgentCursorsEvent::AcceptSuggestion(found));
        cx.notify();
        true
    }

    /// Reject and remove one pending suggestion by id.
    pub fn reject_one(&mut self, suggestion_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(idx) = self
            .suggestions
            .iter()
            .position(|s| s.id.as_ref() == suggestion_id)
        else {
            return false;
        };
        let removed = self.suggestions.remove(idx);
        self.last_open_sug = Some(removed.clone());
        if self.open_suggestion.as_deref() == Some(suggestion_id) {
            self.open_suggestion = None;
        }
        cx.emit(AgentCursorsEvent::RejectSuggestion(removed));
        cx.notify();
        true
    }

    /// Accept all remaining inline suggestions in document order.
    pub fn accept_all(&mut self, cx: &mut Context<Self>) -> usize {
        let count = self.suggestions.len();
        if count == 0 {
            return 0;
        }
        let ids: Vec<SharedString> = self.suggestions.iter().map(|s| s.id.clone()).collect();
        for sid in ids {
            let (nb, ns) = accept_suggestion(&self.blocks, &self.suggestions, sid.as_ref());
            self.blocks = nb;
            self.suggestions = ns;
        }
        self.open_suggestion = None;
        self.toast = Some((
            format!(
                "{count} {}",
                tr(cx, "suggestions accepted", "处建议已全部采纳")
            )
            .into(),
            Instant::now(),
        ));
        cx.emit(AgentCursorsEvent::AcceptAll(count));
        cx.notify();
        count
    }

    /// Resolve and remove a margin comment by id.
    pub fn resolve_comment(&mut self, comment_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(idx) = self
            .comments
            .iter()
            .position(|c| c.id.as_ref() == comment_id)
        else {
            return false;
        };
        let removed = self.comments.remove(idx);
        self.toast = Some((
            tr(cx, "Comment resolved", "评论已标记已解决").into(),
            Instant::now(),
        ));
        cx.emit(AgentCursorsEvent::ResolveComment(removed));
        cx.notify();
        true
    }

    fn agent_color(&self, agent_id: &str, p: &ProPalette) -> Hsla {
        if let Some(ag) = self.agents.iter().find(|a| a.id.as_ref() == agent_id)
            && let Some(c) = ag.color
        {
            return c;
        }
        if p.dark {
            rgb(0xe4e4e7).into()
        } else {
            rgb(0x1f1f22).into()
        }
    }

    fn render_agent_mark(
        &self,
        agent: &CursorAgent,
        side: f32,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        let col = agent.color.unwrap_or(p.blue);
        if let Some(provider) = &agent.provider {
            return pro_app_tile(provider.as_ref(), side, cx);
        }
        pro_agent_avatar(agent.id.as_ref(), col, side, true, elapsed_ms, cx)
    }

    fn render_segmented_text(
        &self,
        block: &DocBlock,
        is_code: bool,
        typed_chars: usize,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut block_sugs: Vec<&DocSuggestion> = self
            .suggestions
            .iter()
            .filter(|s| s.block == block.id)
            .collect();
        block_sugs.sort_by_key(|s| s.at);

        let fs = if is_code { 12.0 } else { 13.5 };
        let lh = if is_code { 20.0 } else { 22.0 };
        let base_fg = if is_code { p.ink } else { p.text };

        if block_sugs.is_empty() {
            let txt = match &block.text_zh {
                Some(zh) => tr(cx, block.text.as_ref(), zh.as_ref()),
                None => block.text.as_ref(),
            };
            return div()
                .text_size(px(fs))
                .line_height(px(lh))
                .text_color(base_fg)
                .child(txt.to_string())
                .into_any_element();
        }

        let chars: Vec<char> = block.text.chars().collect();
        let mut cursor = 0usize;
        let mut wrap = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(3.0))
            .gap_y(px(2.0))
            .text_size(px(fs))
            .line_height(px(lh))
            .text_color(base_fg);

        for sug in block_sugs {
            let start = sug.at.min(chars.len());
            if start > cursor {
                let prefix: String = chars[cursor..start].iter().collect();
                for word_chunk in split_preserving_lines(&prefix) {
                    wrap = wrap.child(div().child(word_chunk));
                }
            }
            let rem_len = sug.remove.chars().count();
            cursor = (start + rem_len).min(chars.len());

            let ag_col = self.agent_color(sug.agent.as_ref(), p);
            let is_open = self.open_suggestion.as_ref() == Some(&sug.id);
            let sug_id = sug.id.clone();
            let sug_snap_key = sug.clone();
            let sug_snap_click = sug.clone();

            // Live letter-by-letter typing for Coder's rollout patch while live_demo is active.
            let ins_full = sug.insert.as_ref();
            let shown_ins: String = if self.live_demo && sug.id.as_ref() == "s-date" {
                let total = ins_full.chars().count().max(1);
                let n = (typed_chars % (total + 5)).clamp(1, total);
                ins_full.chars().take(n).collect()
            } else {
                ins_full.to_string()
            };

            let agent_name = self
                .agents
                .iter()
                .find(|a| a.id == sug.agent)
                .map(|a| a.label(cx).to_string())
                .unwrap_or_else(|| sug.agent.to_string());
            let aria = format!(
                "{agent_name} {}: {}",
                tr(cx, "suggests", "建议修改为"),
                sug.insert
            );

            let chip_bg = ag_col.opacity(if is_open { 0.22 } else { 0.12 });
            let hover_bg = ag_col.opacity(0.22);

            wrap = wrap.child(
                div()
                    .id(pro_eid(&self.id, format!("sug-{}", sug.id)))
                    .role(Role::Button)
                    .aria_label(aria)
                    .aria_expanded(is_open)
                    .tab_index(0)
                    .px(px(4.0))
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_key_down(cx.listener({
                        let sug_id = sug_id.clone();
                        move |this, ev: &KeyDownEvent, _, cx| match ev.keystroke.key.as_str() {
                            "enter" | "space" | " " => {
                                if this.open_suggestion.as_ref() == Some(&sug_id) {
                                    this.open_suggestion = None;
                                } else {
                                    this.last_open_sug = Some(sug_snap_key.clone());
                                    this.open_suggestion = Some(sug_id.clone());
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }
                            "escape" if this.open_suggestion.is_some() => {
                                this.open_suggestion = None;
                                cx.stop_propagation();
                                cx.notify();
                            }
                            _ => {}
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.open_suggestion.as_ref() == Some(&sug_id) {
                            this.open_suggestion = None;
                        } else {
                            this.last_open_sug = Some(sug_snap_click.clone());
                            this.open_suggestion = Some(sug_id.clone());
                        }
                        cx.notify();
                    }))
                    .when(!sug.remove.is_empty(), |d| {
                        d.child(
                            div()
                                .text_color(p.muted)
                                .line_through()
                                .child(sug.remove.clone()),
                        )
                    })
                    .when(!shown_ins.is_empty(), |d| {
                        d.child(
                            div()
                                .px(px(3.5))
                                .rounded(px(3.0))
                                .bg(chip_bg)
                                .hover(move |s| s.bg(hover_bg))
                                .border_b_2()
                                .border_color(ag_col)
                                .text_color(p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(shown_ins),
                        )
                    }),
            );
        }

        if cursor < chars.len() {
            let tail: String = chars[cursor..].iter().collect();
            for word_chunk in split_preserving_lines(&tail) {
                wrap = wrap.child(div().child(word_chunk));
            }
        }

        wrap.into_any_element()
    }
}

impl Render for AgentCursors {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_secs = if reduced {
            0.0
        } else {
            self.started_at.elapsed().as_secs_f32()
        };
        let elapsed_ms = elapsed_secs * 1000.0;

        let active_here = !self.cursors.is_empty() || self.live_demo;
        if !reduced && (self.loading || (self.live_demo && active_here)) {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border = p.line;

        // Top view tabs (Doc / Table) with 4-edge liquid pill indicator
        let tab_sensor = pro_selection_sensor();
        let active_tab_key = self.active_tab.clone();
        let mut tabs_track = div()
            .id(pro_eid(&self.id, "tabs"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Views", "视图"))
            .h(px(32.0))
            .p(px(2.0))
            .rounded(px(8.0))
            .bg(p.chip)
            .border_1()
            .border_color(p.line_soft)
            .relative()
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tab-pill"),
                active_tab_key.clone(),
                tab_sensor.clone(),
                6.0,
                card_bg,
                Some(p.line_strong),
                !p.dark,
            ));

        for tab in self.tabs.clone() {
            let selected = self.active_tab == tab.id;
            let tid = tab.id.clone();
            let lbl = tab.text(cx).to_string();
            tabs_track = tabs_track.child(
                div()
                    .id(pro_eid(&self.id, format!("tab-{tid}")))
                    .role(Role::Tab)
                    .aria_label(lbl.clone())
                    .aria_selected(selected)
                    .tab_index(if selected { 0 } else { -1 })
                    .relative()
                    .h(px(28.0))
                    .px(px(12.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.ink } else { p.muted })
                    .cursor_pointer()
                    .hover(move |s| s.text_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.active_tab != tid {
                            this.active_tab = tid.clone();
                            cx.emit(AgentCursorsEvent::TabChanged(tid.clone()));
                            cx.notify();
                        }
                    }))
                    .when(selected, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .child(div().relative().child(lbl)),
            );
        }
        let tabs_bar = div().mb(px(6.0)).flex().items_center().child(tabs_track);

        // Header bar inside the document sheet
        let total_cost: f64 = self.agents.iter().filter_map(|a| a.cost).sum();
        let sug_count = self.suggestions.len();
        let sug_label = if sug_count == 0 {
            tr(cx, "All accepted", "已全部采纳").to_string()
        } else if sug_count == 1 {
            tr(cx, "1 suggestion", "1 处建议").to_string()
        } else {
            format!("{} {}", sug_count, tr(cx, "suggestions", "处建议"))
        };

        let spin_angle = (elapsed_secs / 0.85).fract() * TAU;
        let mut people_row = div()
            .id(pro_eid(&self.id, "people"))
            .role(Role::Group)
            .aria_label(tr(cx, "Agents in this document", "当前文档中的智能体"))
            .flex()
            .items_center()
            .gap(px(4.0));

        for agent in self.agents.clone() {
            let aid = agent.id.clone();
            let is_followed = self.following.as_ref() == Some(&aid);
            let is_hovered = self.hovered_agent.as_ref() == Some(&aid);
            let ag_col = self.agent_color(aid.as_ref(), &p);
            let tint_bg = agent.tint.unwrap_or_else(|| ag_col.opacity(0.16));
            let present = self.live_demo || self.cursors.iter().any(|(id, _)| id == &aid);
            let doing_tip = self
                .cursors
                .iter()
                .find(|(id, _)| id == &aid)
                .and_then(|(_, c)| c.doing_text(cx))
                .unwrap_or_else(|| tr(cx, "Collaborating", "协作编辑中"))
                .to_string();
            let name_str = agent.label(cx).to_string();
            let aria = format!("{} {} – {}", tr(cx, "Follow", "跟随"), name_str, doing_tip);

            let arc_ring = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    if !present {
                        return;
                    }
                    let center = bounds.center();
                    let r = (f32::from(bounds.size.width) - 2.0) * 0.5;
                    if r <= 2.0 {
                        return;
                    }
                    let steps = 20usize;
                    let sweep = TAU * 0.72;
                    let start_a = if reduced { -FRAC_PI_2 } else { spin_angle };
                    let mut arc = PathBuilder::stroke(px(1.5));
                    for i in 0..=steps {
                        let a = start_a + (i as f32 / steps as f32) * sweep;
                        let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                        if i == 0 {
                            arc.move_to(pt);
                        } else {
                            arc.line_to(pt);
                        }
                    }
                    if let Ok(path) = arc.build() {
                        window.paint_path(path, ag_col);
                    }
                },
            )
            .absolute()
            .inset_0();

            let mark = self.render_agent_mark(&agent, 15.0, elapsed_ms, &p, cx);
            let tip_primary = p.primary;
            let tip_on = p.on_primary;
            let tip_pop =
                div()
                    .absolute()
                    .top(px(31.0))
                    .right(px(-4.0))
                    .child(pro_popover_presence(
                        pro_eid(&self.id, format!("face-tip-{aid}")),
                        is_hovered,
                        move || {
                            div()
                                .px(px(8.0))
                                .py(px(4.5))
                                .rounded(px(6.0))
                                .bg(tip_primary)
                                .text_color(tip_on)
                                .shadow_md()
                                .whitespace_nowrap()
                                .flex()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(name_str),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(tip_on.opacity(0.76))
                                        .child(doing_tip),
                                )
                                .into_any_element()
                        },
                    ));

            let aid_click = aid.clone();
            let aid_hover = aid.clone();
            let ring_btn = pro_tactile(
                pro_eid(&self.id, format!("face-{aid}")),
                aria,
                999.,
                if is_followed { tint_bg } else { card_bg },
                tint_bg,
                if is_followed { ag_col } else { p.line },
                ag_col,
                is_followed,
                div()
                    .size(px(26.0))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(arc_ring)
                    .child(mark),
                cx,
                move |this, _, cx| {
                    this.toggle_follow(aid_click.as_ref(), cx);
                },
            )
            .size(px(26.0))
            .rounded_full()
            .opacity(if present { 1.0 } else { 0.42 })
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_agent = Some(aid_hover.clone());
                } else if this.hovered_agent.as_ref() == Some(&aid_hover) {
                    this.hovered_agent = None;
                }
                cx.notify();
            }));

            people_row = people_row.child(
                div()
                    .relative()
                    .size(px(26.0))
                    .child(ring_btn)
                    .child(tip_pop),
            );
        }

        let title_str = match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()).to_string(),
            None => self.title.to_string(),
        };

        let header = div()
            .h(px(44.0))
            .px(px(14.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(pro_svg("0 0 24 24", DOC_ICON_SVG, 16.0, p.muted))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(title_str),
            )
            .child(people_row)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .when(sug_count == 0, |d| {
                        d.child(pro_check_draw(
                            pro_eid(&self.id, "accept-all-chk"),
                            true,
                            14.0,
                            false,
                            p.green,
                        ))
                    })
                    .child(pro_swap_label(
                        pro_eid(&self.id, "sug-count-lbl"),
                        sug_label,
                        12.0,
                        FontWeight::NORMAL,
                        if sug_count == 0 { p.green_ink } else { p.muted },
                    ))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "total-cost"),
                        total_cost,
                        2,
                        "$",
                        "",
                        12.0,
                        FontWeight::SEMIBOLD,
                        p.strong,
                    )),
            )
            .when(sug_count > 0, |bar| {
                bar.child(pro_btn(
                    pro_eid(&self.id, "accept-all"),
                    tr(cx, "Accept all", "全部采纳"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.accept_all(cx);
                    },
                ))
            });

        if self.loading {
            let phase = (elapsed_secs * 0.9).fract();
            let skel = div()
                .p(px(32.0))
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(pro_skeleton_bar(240.0, 22.0, 6.0, phase, cx))
                .child(pro_skeleton_bar(520.0, 14.0, 4.0, phase + 0.15, cx))
                .child(pro_skeleton_bar(460.0, 14.0, 4.0, phase + 0.30, cx))
                .child(pro_skeleton_bar(300.0, 88.0, 8.0, phase + 0.45, cx))
                .child(pro_skeleton_bar(500.0, 74.0, 8.0, phase + 0.60, cx));
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(700.0))
                        .flex()
                        .flex_col()
                        .child(tabs_bar)
                        .child(
                            div()
                                .w_full()
                                .rounded(px(12.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(card_border)
                                .when(!p.dark, |d| d.shadow_sm())
                                .flex()
                                .flex_col()
                                .child(header)
                                .child(skel),
                        ),
                )
                .into_any_element();
        }

        if self.empty || (self.blocks.is_empty() && self.table_rows.is_empty()) {
            let empty_view = pro_empty_state(
                ProIcon::PageDoc,
                tr(cx, "Empty document", "暂无文档内容"),
                tr(
                    cx,
                    "Load blocks or switch to a live collaborative brief to watch agents edit together.",
                    "加载文档块或恢复示例简报，即可实时查看多个智能体协同编辑。",
                ),
                Some((
                    pro_eid(&self.id, "restore-doc"),
                    SharedString::from(tr(cx, "Load launch brief", "加载发布简报")),
                    Box::new(|this: &mut Self, _, cx| {
                        this.empty = false;
                        if this.blocks.is_empty() {
                            this.blocks = default_doc_blocks();
                            this.suggestions = default_suggestions();
                            this.comments = default_comments();
                        }
                        cx.notify();
                    }),
                )),
                cx,
            );
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(700.0))
                        .flex()
                        .flex_col()
                        .child(tabs_bar)
                        .child(
                            div()
                                .w_full()
                                .rounded(px(12.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(card_border)
                                .when(!p.dark, |d| d.shadow_sm())
                                .flex()
                                .flex_col()
                                .child(header)
                                .child(empty_view),
                        ),
                )
                .into_any_element();
        }

        // Dynamic simulation parameters for live demo mode
        let typed_step = if self.live_demo && !reduced {
            (elapsed_secs * 3.6) as usize
        } else {
            6usize
        };
        let hero_wave = if self.live_demo && !reduced {
            0.5 + 0.5 * (elapsed_secs * 1.15).sin()
        } else {
            0.65
        };
        let hero_w_target = 276.0 + 36.0 * hero_wave;
        let hero_w = pro_spring_f32(pro_eid(&self.id, "hero-w"), hero_w_target, 0.38, w, cx);
        let hero_h = (hero_w / 3.35).round();
        let disp_w = (hero_w * 2.0).round() as u32;
        let disp_h = (hero_h * 2.0).round() as u32;

        let body_content = if self.active_tab.as_ref() == "table" {
            self.render_table_stage(elapsed_secs, reduced, &p, cx)
        } else {
            self.render_doc_stage(
                elapsed_secs,
                elapsed_ms,
                typed_step,
                hero_w,
                hero_h,
                disp_w,
                disp_h,
                reduced,
                &p,
                cx,
            )
        };

        let transitioned_body = pro_view_transition(
            pro_eid(&self.id, "stage-tx"),
            self.active_tab.clone(),
            body_content,
        );

        let mut sheet = div()
            .w_full()
            .relative()
            .rounded(px(12.0))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .when(!p.dark, |d| d.shadow_sm())
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(header)
            .child(transitioned_body)
            .child(pro_toast_banner(self.toast.clone()));

        if let Some(h) = self.fixed_height {
            sheet = sheet.h(px(h));
        }

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(700.0))
                    .flex()
                    .flex_col()
                    .child(tabs_bar)
                    .child(sheet),
            )
            .into_any_element()
    }
}

impl AgentCursors {
    #[allow(clippy::too_many_arguments)]
    fn render_doc_stage(
        &self,
        elapsed_secs: f32,
        elapsed_ms: f32,
        typed_step: usize,
        hero_w: f32,
        hero_h: f32,
        disp_w: u32,
        disp_h: u32,
        reduced: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut doc_col = div()
            .w_full()
            .px(px(40.0))
            .pt(px(28.0))
            .pb(px(42.0))
            .flex()
            .flex_col()
            .gap(px(14.0));

        let writer_col = self.agent_color("writer", p);
        let designer_col = self.agent_color("designer", p);
        let writer_dimmed = self
            .following
            .as_ref()
            .is_some_and(|f| f.as_ref() != "writer");

        let mut idx = 0usize;
        while idx < self.blocks.len() {
            let blk = &self.blocks[idx];
            match blk.kind {
                DocBlockKind::Title => {
                    let title_txt = match &blk.text_zh {
                        Some(zh) => tr(cx, blk.text.as_ref(), zh.as_ref()),
                        None => blk.text.as_ref(),
                    };
                    // Split last two words so Writer's selection highlight sits cleanly behind them
                    let (lead, highlighted) = split_title_selection(title_txt);
                    doc_col = doc_col.child(
                        div()
                            .id(pro_eid(&self.id, format!("doc-h2-{idx}")))
                            .role(Role::Heading)
                            .aria_level(2)
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.0))
                            .text_size(px(20.0))
                            .line_height(px(28.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(lead)
                            .when(!highlighted.is_empty(), |d| {
                                d.child(
                                    div()
                                        .px(px(4.0))
                                        .rounded(px(4.0))
                                        .bg(writer_col.opacity(if writer_dimmed {
                                            0.06
                                        } else {
                                            0.18
                                        }))
                                        .child(highlighted),
                                )
                            }),
                    );
                    idx += 1;
                }
                DocBlockKind::Heading => {
                    let h_txt = match &blk.text_zh {
                        Some(zh) => tr(cx, blk.text.as_ref(), zh.as_ref()),
                        None => blk.text.as_ref(),
                    };
                    doc_col = doc_col.child(
                        div()
                            .id(pro_eid(&self.id, format!("doc-h3-{idx}")))
                            .pt(px(8.0))
                            .role(Role::Heading)
                            .aria_level(3)
                            .text_size(px(14.5))
                            .line_height(px(22.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(h_txt.to_string()),
                    );
                    idx += 1;
                }
                DocBlockKind::Paragraph => {
                    doc_col =
                        doc_col.child(self.render_segmented_text(blk, false, typed_step, p, cx));
                    idx += 1;
                }
                DocBlockKind::Image => {
                    let handle_bg: Hsla = if p.dark {
                        rgb(0x1c1c1f).into()
                    } else {
                        rgb(0xffffff).into()
                    };
                    let shimmer_phase = if reduced {
                        0.35
                    } else {
                        (elapsed_secs / 1.6).fract()
                    };
                    let sheen_x = -80.0 + shimmer_phase * (hero_w + 140.0);

                    let figure_el = div()
                        .relative()
                        .w(px(hero_w))
                        .h(px(hero_h))
                        .flex_none()
                        .child(
                            div()
                                .size_full()
                                .rounded(px(8.0))
                                .bg(p.subtle)
                                .border_2()
                                .border_color(designer_col)
                                .overflow_hidden()
                                .relative()
                                .p(px(10.0))
                                .flex()
                                .flex_col()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(5.0))
                                                .child(
                                                    div()
                                                        .size(px(7.0))
                                                        .rounded_full()
                                                        .bg(designer_col.opacity(0.65)),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(56.0))
                                                        .h(px(6.0))
                                                        .rounded(px(3.0))
                                                        .bg(p.line_strong),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .w(px(34.0))
                                                .h(px(6.0))
                                                .rounded(px(3.0))
                                                .bg(p.chip_hover),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(5.0))
                                        .child(
                                            div()
                                                .w(relative(0.82))
                                                .h(px(6.0))
                                                .rounded(px(3.0))
                                                .bg(p.chip_hover),
                                        )
                                        .child(
                                            div()
                                                .w(relative(0.58))
                                                .h(px(6.0))
                                                .rounded(px(3.0))
                                                .bg(p.chip),
                                        ),
                                )
                                .when(blk.loading && !reduced, |card| {
                                    card.child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .bottom_0()
                                            .left(px(sheen_x))
                                            .w(px(72.0))
                                            .bg(p.ink.opacity(0.04)),
                                    )
                                })
                                .child(
                                    div()
                                        .absolute()
                                        .bottom(px(7.0))
                                        .left_0()
                                        .right_0()
                                        .flex()
                                        .justify_center()
                                        .child(
                                            div()
                                                .px(px(6.5))
                                                .h(px(18.0))
                                                .rounded(px(5.0))
                                                .bg(designer_col)
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0xffffff))
                                                .child(format!("{disp_w} × {disp_h}")),
                                        ),
                                ),
                        )
                        // 4 corner resize handles
                        .child(corner_handle(
                            -4.0,
                            Some(-4.0),
                            None,
                            None,
                            handle_bg,
                            designer_col,
                        ))
                        .child(corner_handle(
                            -4.0,
                            None,
                            Some(-4.0),
                            None,
                            handle_bg,
                            designer_col,
                        ))
                        .child(corner_handle_bottom(
                            -4.0,
                            Some(-4.0),
                            None,
                            handle_bg,
                            designer_col,
                        ))
                        .child(corner_handle_bottom(
                            -4.0,
                            None,
                            Some(-4.0),
                            handle_bg,
                            designer_col,
                        ));

                    if blk.float != ImageFloat::None
                        && idx + 1 < self.blocks.len()
                        && self.blocks[idx + 1].kind == DocBlockKind::Paragraph
                    {
                        let beside = self.render_segmented_text(
                            &self.blocks[idx + 1],
                            false,
                            typed_step,
                            p,
                            cx,
                        );
                        let row = if blk.float == ImageFloat::Left {
                            div()
                                .w_full()
                                .flex()
                                .items_start()
                                .gap(px(22.0))
                                .child(figure_el)
                                .child(div().flex_1().min_w_0().child(beside))
                        } else {
                            div()
                                .w_full()
                                .flex()
                                .items_start()
                                .gap(px(22.0))
                                .child(div().flex_1().min_w_0().child(beside))
                                .child(figure_el)
                        };
                        doc_col = doc_col.child(row);
                        idx += 2;
                    } else {
                        doc_col = doc_col.child(figure_el);
                        idx += 1;
                    }
                }
                DocBlockKind::Code => {
                    let code_bg: Hsla = if p.dark {
                        hsla(0., 0., 1., 0.035)
                    } else {
                        hsla(0., 0., 0., 0.025)
                    };
                    doc_col = doc_col.child(
                        div()
                            .w_full()
                            .px(px(16.0))
                            .py(px(13.0))
                            .rounded(px(8.0))
                            .bg(code_bg)
                            .border_1()
                            .border_color(p.line)
                            .child(self.render_segmented_text(blk, true, typed_step, p, cx)),
                    );
                    idx += 1;
                }
                DocBlockKind::Item => {
                    let mut list_el = div().w_full().pl(px(6.0)).flex().flex_col().gap(px(5.0));
                    while idx < self.blocks.len() && self.blocks[idx].kind == DocBlockKind::Item {
                        let item_blk = &self.blocks[idx];
                        list_el = list_el.child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(9.0))
                                .child(
                                    div()
                                        .mt(px(8.5))
                                        .size(px(4.5))
                                        .rounded_full()
                                        .flex_none()
                                        .bg(p.faint),
                                )
                                .child(div().flex_1().min_w_0().child(
                                    self.render_segmented_text(item_blk, false, typed_step, p, cx),
                                )),
                        );
                        idx += 1;
                    }
                    doc_col = doc_col.child(list_el);
                }
                DocBlockKind::Table => {
                    idx += 1;
                }
            }
        }

        // Open suggestion review popover card with bidirectional spring presence
        let active_sug = self
            .open_suggestion
            .as_ref()
            .and_then(|sid| self.suggestions.iter().find(|s| &s.id == sid).cloned());
        let shown_sug = active_sug.clone().or_else(|| self.last_open_sug.clone());
        let sug_is_open = active_sug.is_some();
        let suggestion_popover = shown_sug.map(|sug| {
            let agent = self.agents.iter().find(|a| a.id == sug.agent).cloned();
            let ag_name = agent
                .as_ref()
                .map(|a| a.label(cx).to_string())
                .unwrap_or_else(|| sug.agent.to_string());
            let mark = agent
                .as_ref()
                .map(|a| self.render_agent_mark(a, 18.0, elapsed_ms, p, cx));
            let note = sug
                .note_text(cx)
                .unwrap_or_else(|| tr(cx, "Proposed inline change", "智能体建议的内联修改"))
                .to_string();
            let sid_accept = sug.id.clone();
            let sid_reject = sug.id.clone();
            let top_y = if sug.id.as_ref() == "s-date" {
                238.0
            } else {
                96.0
            };

            let close_btn = pro_icon_btn(
                pro_eid(&self.id, "sug-pop-close"),
                tr(cx, "Close", "关闭"),
                ProIcon::Close,
                22.0,
                false,
                cx,
                |this, _, cx| {
                    this.open_suggestion = None;
                    cx.notify();
                },
            );
            let rej_btn = pro_btn(
                pro_eid(&self.id, "sug-reject"),
                tr(cx, "Reject", "拒绝"),
                None,
                ProBtnStyle::Secondary,
                true,
                cx,
                move |this, _, cx| {
                    this.reject_one(sid_reject.as_ref(), cx);
                },
            );
            let acc_btn = pro_btn(
                pro_eid(&self.id, "sug-accept"),
                tr(cx, "Accept", "采纳"),
                Some(ProIcon::Check),
                ProBtnStyle::Primary,
                true,
                cx,
                move |this, _, cx| {
                    this.accept_one(sid_accept.as_ref(), cx);
                },
            );
            let aria_sug = format!("{ag_name} {}", tr(cx, "suggestion", "修改建议"));
            let suggests_lbl = tr(cx, "suggests", "建议").to_string();
            let pop_id = pro_eid(&self.id, "sug-popover");
            let p_card = *p;

            div()
                .absolute()
                .right(px(28.0))
                .top(px(top_y))
                .child(pro_popover_presence(
                    pro_eid(&self.id, "sug-pop-pres"),
                    sug_is_open,
                    move || {
                        div()
                            .id(pop_id)
                            .role(Role::Group)
                            .aria_label(aria_sug)
                            .w(px(264.0))
                            .p(px(11.0))
                            .rounded(px(10.0))
                            .bg(p_card.card)
                            .border_1()
                            .border_color(p_card.line_strong)
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .gap(px(7.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .when_some(mark, |d, m| d.child(m))
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(p_card.ink)
                                                    .child(ag_name),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(p_card.muted)
                                                    .child(suggests_lbl),
                                            ),
                                    )
                                    .child(close_btn),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(18.0))
                                    .text_color(p_card.text)
                                    .child(note),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(6.0))
                                    .pt(px(2.0))
                                    .child(rej_btn)
                                    .child(acc_btn),
                            )
                            .into_any_element()
                    },
                ))
        });

        // Margin comment pins & expandable cards with bidirectional spring presence
        let mut comments_layer = div().absolute().inset_0();
        for comment in self.comments.clone() {
            let cid = comment.id.clone();
            let agent = self.agents.iter().find(|a| a.id == comment.agent).cloned();
            let ag_name = agent
                .as_ref()
                .map(|a| a.label(cx).to_string())
                .unwrap_or_else(|| comment.agent.to_string());
            let mark_open = agent
                .as_ref()
                .map(|a| self.render_agent_mark(a, 14.0, elapsed_ms, p, cx));
            let mark_pin = agent
                .as_ref()
                .map(|a| self.render_agent_mark(a, 14.0, elapsed_ms, p, cx));
            let pin_bg: Hsla = if p.dark {
                rgb(0xf4f4f5).into()
            } else {
                rgb(0x141416).into()
            };
            let pin_ink: Hsla = if p.dark {
                rgb(0x141416).into()
            } else {
                rgb(0xffffff).into()
            };
            let pin_soft: Hsla = if p.dark {
                hsla(0., 0., 0., 0.08)
            } else {
                hsla(0., 0., 1., 0.14)
            };

            let pin_x = (comment.x * 640.0).clamp(12.0, 420.0);
            let pin_y = comment.y;
            let is_cmt_open = comment.open;

            // Expanded comment card
            let cid_toggle = cid.clone();
            let cid_reply = cid.clone();
            let cid_resolve = cid.clone();
            let body_str = comment.body(cx).to_string();
            let reply_count = comment.replies.len();
            let latest_reply = comment.replies.last().cloned();
            let cmt_from_aria = format!("{} {ag_name}", tr(cx, "Comment from", "评论来自"));
            let now_lbl = tr(cx, "now", "刚刚").to_string();
            let you_prefix = tr(cx, "You", "你").to_string();

            let fold_btn = pro_tactile(
                pro_eid(&self.id, format!("cmt-fold-{cid}")),
                tr(cx, "Collapse comment", "收起评论"),
                5.0,
                transparent_black(),
                pin_soft,
                transparent_black(),
                transparent_black(),
                false,
                div()
                    .size(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Minus, 11.0, pin_ink.opacity(0.7))),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.comments.iter_mut().find(|c| c.id == cid_toggle) {
                        c.open = false;
                        cx.notify();
                    }
                },
            )
            .size(px(20.0));

            let reply_btn = pro_tactile(
                pro_eid(&self.id, format!("cmt-reply-{cid}")),
                tr(cx, "Reply to comment", "快速回复"),
                6.0,
                pin_soft,
                pin_ink.opacity(0.18),
                transparent_black(),
                pin_ink.opacity(0.24),
                false,
                div()
                    .h(px(28.0))
                    .px(px(9.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(pin_ink.opacity(0.78))
                    .child(tr(cx, "Reply “Use Nov 3”…", "回复“统一用 Nov 3”…")),
                cx,
                move |this, _, cx| {
                    let reply_txt = SharedString::from(tr(
                        cx,
                        "Use \"Nov 3\" everywhere",
                        "统一采用 \"Nov 3\" 格式",
                    ));
                    if let Some(c) = this.comments.iter_mut().find(|c| c.id == cid_reply) {
                        c.replies.push(reply_txt.clone());
                        cx.emit(AgentCursorsEvent::ReplyComment {
                            comment_id: cid_reply.clone(),
                            text: reply_txt,
                        });
                        cx.notify();
                    }
                },
            )
            .flex_1()
            .min_w_0();

            let res_btn = pro_tactile(
                pro_eid(&self.id, format!("cmt-res-{cid}")),
                tr(cx, "Resolve comment", "标记评论已解决"),
                6.0,
                pin_ink,
                p.green,
                transparent_black(),
                transparent_black(),
                false,
                div()
                    .size(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("cmt-chk-{cid}")),
                        true,
                        15.0,
                        false,
                        p.green,
                    )),
                cx,
                move |this, _, cx| {
                    this.resolve_comment(cid_resolve.as_ref(), cx);
                },
            )
            .size(px(28.0))
            .flex_none();

            let open_card_id = pro_eid(&self.id, format!("cmt-open-{cid}"));
            let ag_name_open = ag_name.clone();
            comments_layer =
                comments_layer.child(div().absolute().right(px(16.0)).top(px(pin_y)).child(
                    pro_popover_presence(
                        pro_eid(&self.id, format!("cmt-open-pop-{cid}")),
                        is_cmt_open,
                        move || {
                            div()
                                .id(open_card_id)
                                .role(Role::Group)
                                .aria_label(cmt_from_aria)
                                .w(px(248.0))
                                .p(px(10.0))
                                .rounded(px(10.0))
                                .bg(pin_bg)
                                .border_1()
                                .border_color(pin_soft)
                                .shadow_lg()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .when_some(mark_open, |d, m| d.child(m))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .text_size(px(12.0))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(pin_ink)
                                                .child(ag_name_open),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(pin_ink.opacity(0.52))
                                                .child(now_lbl),
                                        )
                                        .child(fold_btn),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .line_height(px(18.5))
                                        .text_color(pin_ink.opacity(0.88))
                                        .child(body_str),
                                )
                                .when(reply_count > 0, |card| {
                                    if let Some(latest) = latest_reply {
                                        card.child(
                                            div()
                                                .px(px(8.0))
                                                .py(px(4.5))
                                                .rounded(px(6.0))
                                                .bg(pin_soft)
                                                .text_size(px(11.5))
                                                .text_color(pin_ink.opacity(0.85))
                                                .child(format!("{you_prefix}: {latest}")),
                                        )
                                    } else {
                                        card
                                    }
                                })
                                .child(
                                    div()
                                        .pt(px(2.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(reply_btn)
                                        .child(res_btn),
                                )
                                .into_any_element()
                        },
                    ),
                ));

            // Collapsed comment pin
            let cid_open = cid.clone();
            let pin_aria = format!(
                "{} {ag_name}: {}",
                tr(cx, "Comment from", "评论来自"),
                comment.body(cx)
            );
            let pin_btn = pro_tactile(
                pro_eid(&self.id, format!("cmt-pin-{cid}")),
                pin_aria,
                14.0,
                pin_bg,
                pin_bg,
                pin_soft,
                p.blue,
                false,
                div()
                    .size(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when_some(mark_pin, |d, m| d.child(m)),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.comments.iter_mut().find(|c| c.id == cid_open) {
                        c.open = true;
                        cx.notify();
                    }
                },
            )
            .size(px(28.0))
            .shadow_md();

            comments_layer =
                comments_layer.child(div().absolute().left(px(pin_x)).top(px(pin_y)).child(
                    pro_popover_presence(
                        pro_eid(&self.id, format!("cmt-pin-pop-{cid}")),
                        !is_cmt_open,
                        move || pin_btn.into_any_element(),
                    ),
                ));
        }

        // Gliding cursors layer
        let caret_blink_on = reduced || ((elapsed_secs * 1.8) as usize).is_multiple_of(2);
        let phase_slot = if self.live_demo && !reduced {
            ((elapsed_secs / 2.8) as usize) % 3
        } else {
            0usize
        };
        let mut cursors_layer = div().absolute().inset_0();
        for (agent_id, cur) in &self.cursors {
            let Some(agent) = self.agents.iter().find(|a| &a.id == agent_id) else {
                continue;
            };
            let dimmed = self.following.as_ref().is_some_and(|f| f != agent_id);
            let ag_col = self.agent_color(agent_id.as_ref(), p);
            let (tx, ty, glide_secs) = self.compute_cursor_xy(
                agent_id.as_ref(),
                cur,
                phase_slot,
                typed_step,
                hero_w,
                hero_h,
            );
            let cost_tag = agent.cost.map(|c| SharedString::from(format!("${c:.2}")));
            let doing_lbl = cur.doing_text(cx).map(SharedString::from);

            cursors_layer = cursors_layer.child(GlidingPointer {
                id: pro_eid(&self.id, format!("ptr-{agent_id}")),
                target_x: tx,
                target_y: ty,
                glide_secs,
                dimmed,
                typing: cur.typing,
                free_handle: cur.point.is_some(),
                caret_blink_on,
                color: ag_col,
                dark: p.dark,
                name: SharedString::from(agent.label(cx).to_string()),
                cost_tag,
                doing: doing_lbl,
                pink: p.pink,
                blue: p.blue,
            });
        }

        div()
            .relative()
            .w_full()
            .child(doc_col)
            .child(cursors_layer)
            .child(comments_layer)
            .when_some(suggestion_popover, |d, pop| d.child(pop))
            .into_any_element()
    }

    fn compute_cursor_xy(
        &self,
        agent_id: &str,
        cur: &LiveCursor,
        phase_slot: usize,
        typed_step: usize,
        hero_w: f32,
        _hero_h: f32,
    ) -> (f32, f32, f32) {
        if !self.live_demo {
            return (40.0 + cur.x_ratio * 580.0, cur.y_px, 0.48);
        }
        match agent_id {
            "designer" => {
                // Tracks top-right corner handle of the resizing hero image so its flag stays above margin comments
                (40.0 + hero_w - 2.0, 148.0, 0.36)
            }
            "coder" => {
                // Advances character-by-character along `"Nov 3"` inside the rollout code block, clear of right margin comments
                let n = (typed_step % 11).clamp(1, 6) as f32;
                (224.0 + n * 6.4, 314.0, 0.14)
            }
            "researcher" => {
                let offsets = [(164.0, 82.0), (192.0, 82.0), (144.0, 82.0)];
                let (x, y) = offsets[phase_slot % offsets.len()];
                (x, y, 0.54)
            }
            "writer" => {
                let offsets = [(274.0, 24.0), (318.0, 426.0), (290.0, 24.0)];
                let (x, y) = offsets[phase_slot % offsets.len()];
                (x, y, 0.56)
            }
            _ => (40.0 + cur.x_ratio * 580.0, cur.y_px, 0.48),
        }
    }

    fn render_table_stage(
        &self,
        elapsed_secs: f32,
        reduced: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut table_card = div()
            .w_full()
            .rounded(px(10.0))
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col();

        for (r_idx, row) in self.table_rows.iter().enumerate() {
            let is_head = r_idx == 0;
            let mut row_el = div()
                .id(pro_eid(&self.id, format!("tbl-row-{r_idx}")))
                .role(Role::Row)
                .h(px(40.0))
                .px(px(14.0))
                .when(r_idx > 0, |d| d.border_t_1().border_color(p.line))
                .when(is_head, |d| d.bg(p.subtle))
                .flex()
                .items_center()
                .gap(px(12.0));

            for (c_idx, cell) in row.iter().enumerate() {
                let txt = cell.display(cx).to_string();
                let cell_el = if is_head {
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .truncate()
                        .child(txt)
                        .into_any_element()
                } else if cell.tone != CellTone::None {
                    let (bg, fg) = match cell.tone {
                        CellTone::Green => (p.green_soft, p.green_ink),
                        CellTone::Blue => (p.accent_soft, p.blue),
                        CellTone::Amber => (p.amber_soft, p.amber_ink),
                        CellTone::Gray | CellTone::None => (p.chip, p.muted),
                    };
                    div()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .h(px(21.0))
                                .px(px(8.0))
                                .rounded_full()
                                .bg(bg)
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(fg)
                                .child(txt),
                        )
                        .into_any_element()
                } else {
                    div()
                        .text_size(px(13.0))
                        .text_color(p.ink)
                        .truncate()
                        .child(txt)
                        .into_any_element()
                };

                let wrapper = if c_idx == 0 {
                    div().w(relative(0.40)).min_w_0().child(cell_el)
                } else {
                    div().flex_1().min_w_0().child(cell_el)
                };
                row_el = row_el.child(wrapper);
            }
            table_card = table_card.child(row_el);
        }

        // Footer `+ Add row` button
        table_card = table_card.child(
            div()
                .h(px(40.0))
                .px(px(8.0))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .child(pro_btn(
                    pro_eid(&self.id, "add-table-row"),
                    tr(cx, "Add deliverable", "添加交付项"),
                    Some(ProIcon::Plus),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        let idx = this.table_rows.len();
                        this.table_rows.push(vec![
                            TableCell::new(format!("Launch checklist #{idx}"))
                                .with_text_zh(format!("发布核对项 #{idx}")),
                            TableCell::new("Writer").with_text_zh("撰稿智能体"),
                            TableCell::new("Drafting")
                                .with_text_zh("起草中")
                                .with_tone(CellTone::Blue),
                            TableCell::new("Nov 10").with_text_zh("11月10日"),
                        ]);
                        cx.emit(AgentCursorsEvent::RowAdded(this.table_rows.len()));
                        cx.notify();
                    },
                )),
        );

        let slot = if self.live_demo && !reduced {
            ((elapsed_secs / 2.6) as usize) % 2
        } else {
            0
        };
        let researcher_col = self.agent_color("researcher", p);
        let coder_col = self.agent_color("coder", p);

        div()
            .relative()
            .w_full()
            .p(px(28.0))
            .child(
                div()
                    .id(pro_eid(&self.id, "tbl-root"))
                    .role(Role::Table)
                    .w_full()
                    .child(table_card),
            )
            .child(GlidingPointer {
                id: pro_eid(&self.id, "tbl-ptr-r"),
                target_x: if slot == 0 { 418.0 } else { 442.0 },
                target_y: if slot == 0 { 92.0 } else { 132.0 },
                glide_secs: 0.50,
                dimmed: self
                    .following
                    .as_ref()
                    .is_some_and(|f| f.as_ref() != "researcher"),
                typing: false,
                free_handle: true,
                caret_blink_on: true,
                color: researcher_col,
                dark: p.dark,
                name: SharedString::from(tr(cx, "Researcher", "研究智能体")),
                cost_tag: Some(SharedString::from("$0.14")),
                doing: Some(SharedString::from(tr(cx, "Verifying owners", "核对负责人"))),
                pink: p.pink,
                blue: p.blue,
            })
            .child(GlidingPointer {
                id: pro_eid(&self.id, "tbl-ptr-c"),
                target_x: if slot == 0 { 548.0 } else { 536.0 },
                target_y: if slot == 0 { 172.0 } else { 92.0 },
                glide_secs: 0.52,
                dimmed: self
                    .following
                    .as_ref()
                    .is_some_and(|f| f.as_ref() != "coder"),
                typing: false,
                free_handle: true,
                caret_blink_on: true,
                color: coder_col,
                dark: p.dark,
                name: SharedString::from(tr(cx, "Coder", "编码智能体")),
                cost_tag: Some(SharedString::from("$0.14")),
                doing: Some(SharedString::from(tr(
                    cx,
                    "Syncing rollout dates",
                    "同步上线排期",
                ))),
                pink: p.pink,
                blue: p.blue,
            })
            .into_any_element()
    }
}

fn corner_handle(
    top: f32,
    left: Option<f32>,
    right: Option<f32>,
    _unused: Option<f32>,
    bg: Hsla,
    border: Hsla,
) -> Div {
    let mut d = div()
        .absolute()
        .top(px(top))
        .size(px(8.0))
        .rounded(px(2.5))
        .bg(bg)
        .border_2()
        .border_color(border);
    if let Some(l) = left {
        d = d.left(px(l));
    }
    if let Some(r) = right {
        d = d.right(px(r));
    }
    d
}

fn corner_handle_bottom(
    bottom: f32,
    left: Option<f32>,
    right: Option<f32>,
    bg: Hsla,
    border: Hsla,
) -> Div {
    let mut d = div()
        .absolute()
        .bottom(px(bottom))
        .size(px(8.0))
        .rounded(px(2.5))
        .bg(bg)
        .border_2()
        .border_color(border);
    if let Some(l) = left {
        d = d.left(px(l));
    }
    if let Some(r) = right {
        d = d.right(px(r));
    }
    d
}

fn split_title_selection(title: &str) -> (String, String) {
    if let Some((a, b)) = title.rsplit_once(" – ") {
        (format!("{a} –"), b.to_string())
    } else {
        (title.to_string(), String::new())
    }
}

fn split_preserving_lines(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(std::string::ToString::to_string)
        .collect()
}

fn default_agents() -> Vec<CursorAgent> {
    vec![
        CursorAgent::new("researcher", "Researcher")
            .with_name_zh("研究智能体")
            .with_provider("claude")
            .with_color(rgb(0x0190fe).into())
            .with_cost(0.14),
        CursorAgent::new("designer", "Designer")
            .with_name_zh("设计智能体")
            .with_provider("gemini")
            .with_color(rgb(0xa651f0).into())
            .with_cost(0.11),
        CursorAgent::new("coder", "Coder")
            .with_name_zh("编码智能体")
            .with_provider("github")
            .with_color(rgb(0xf28a2e).into())
            .with_cost(0.14),
        CursorAgent::new("writer", "Writer")
            .with_name_zh("撰稿智能体")
            .with_provider("openai")
            .with_color(rgb(0x13b3a0).into())
            .with_cost(0.08),
    ]
}

fn default_doc_blocks() -> Vec<DocBlock> {
    vec![
        DocBlock::new("title", DocBlockKind::Title, "Pulse 2.0 – launch brief")
            .with_text_zh("Pulse 2.0 – 发布简报"),
        DocBlock::new(
            "why",
            DocBlockKind::Paragraph,
            "Teams lose about 40% of their week to status updates. Pulse turns that work into a short daily brief, so nobody has to write one.",
        ),
        DocBlock::new("hero", DocBlockKind::Image, "Monday morning brief preview")
            .with_image_layout(0.50, 3.45, ImageFloat::Left)
            .with_loading_shimmer(true),
        DocBlock::new(
            "hero-copy",
            DocBlockKind::Paragraph,
            "The hero is the brief itself: one calm screen a team lead reads on Monday morning. Short, current and written from the work, so the week starts without a status meeting.",
        )
        .with_text_zh("首屏即简报本身：团队负责人在周一清晨阅读的一屏清爽摘要，直接由项目进展生成，无需再开例会同步进度。"),
        DocBlock::new("rollout-head", DocBlockKind::Heading, "Rollout")
            .with_text_zh("上线排期"),
        DocBlock::new(
            "rollout-code",
            DocBlockKind::Code,
            "const rollout = { waitlist: \"Nov 3\", everyone: \"second week of November\" };",
        ),
        DocBlock::new(
            "launch-week",
            DocBlockKind::Paragraph,
            "Launch week: one short post, a demo video and an email to the waitlist.",
        )
        .with_text_zh("发布周计划：一篇精炼公告、一段演示视频，以及面向候补名单的邮件推送。"),
        DocBlock::new("ships-head", DocBlockKind::Heading, "What ships")
            .with_text_zh("本次交付"),
        DocBlock::new(
            "ship-1",
            DocBlockKind::Item,
            "Daily briefs for every project, written from the work itself",
        )
        .with_text_zh("每个项目自动生成每日简报，直接提炼自真实研发进展"),
        DocBlock::new(
            "ship-2",
            DocBlockKind::Item,
            "Nudges when a task has been quiet for too long",
        )
        .with_text_zh("当某项任务静默时间过长时自动发出温和提醒"),
    ]
}

fn default_table_rows() -> Vec<Vec<TableCell>> {
    vec![
        vec![
            TableCell::new("Deliverable").with_text_zh("交付项"),
            TableCell::new("Owner").with_text_zh("负责人"),
            TableCell::new("Status").with_text_zh("状态"),
            TableCell::new("Target").with_text_zh("目标日期"),
        ],
        vec![
            TableCell::new("Daily project brief engine").with_text_zh("项目每日简报引擎"),
            TableCell::new("Coder").with_text_zh("编码智能体"),
            TableCell::new("Ready")
                .with_text_zh("已就绪")
                .with_tone(CellTone::Green),
            TableCell::new("Nov 3").with_text_zh("11月3日"),
        ],
        vec![
            TableCell::new("Monday hero interactive preview").with_text_zh("周一首屏交互预览"),
            TableCell::new("Designer").with_text_zh("设计智能体"),
            TableCell::new("Resizing")
                .with_text_zh("调优尺寸")
                .with_tone(CellTone::Blue),
            TableCell::new("Nov 3").with_text_zh("11月3日"),
        ],
        vec![
            TableCell::new("Waitlist & GA rollout post").with_text_zh("候补名单与全量公告"),
            TableCell::new("Researcher").with_text_zh("研究智能体"),
            TableCell::new("In review")
                .with_text_zh("审校中")
                .with_tone(CellTone::Amber),
            TableCell::new("Nov 10").with_text_zh("11月10日"),
        ],
    ]
}

fn default_suggestions() -> Vec<DocSuggestion> {
    vec![
        DocSuggestion::new("s-num", "researcher", "why", 11, "about 40%", "38%").with_note(
            "Verified against the Q3 survey across 140 engineering teams.",
            "已与覆盖 140 支工程团队的 Q3 调研数据核对。",
        ),
        DocSuggestion::new(
            "s-date",
            "coder",
            "rollout-code",
            48,
            "second week of November",
            "Nov 10",
        )
        .with_note(
            "Matches the launch calendar date format.",
            "与发布日历中的日期格式保持一致。",
        ),
    ]
}

fn default_comments() -> Vec<DocComment> {
    vec![
        DocComment::new(
            "c-date",
            "coder",
            "rollout-code",
            0.03,
            274.0,
            "Should waitlist and everyone share one date format? I'd use \"Nov 3\" everywhere.",
        )
        .with_text_zh("waitlist 与 everyone 是否应统一日期格式？建议统一写成“Nov 3”风格。")
        .with_open(true),
    ]
}

fn default_cursors() -> Vec<CursorEntry> {
    vec![
        (
            SharedString::from("researcher"),
            LiveCursor::new("why", 0.30, 74.0)
                .with_span(11, 3)
                .with_doing("Fixing the number", "核准数据指标"),
        ),
        (
            SharedString::from("designer"),
            LiveCursor::new("hero", 0.48, 226.0)
                .with_image_point(ImageAnchor::BottomRight)
                .with_doing("Resizing the hero", "调整首屏配图尺寸"),
        ),
        (
            SharedString::from("coder"),
            LiveCursor::new("rollout-code", 0.56, 314.0)
                .with_suggestion("s-date", true)
                .with_doing("Fixing the date", "修正上线日期"),
        ),
        (
            SharedString::from("writer"),
            LiveCursor::new("title", 0.42, 28.0)
                .with_span(12, 12)
                .with_doing("Finding a sharper title", "打磨更精炼的标题"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{AgentCursors, DocBlock, DocBlockKind, DocSuggestion, accept_suggestion};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn accept_suggestion_splices_text_and_shifts_later_offsets() {
        let blocks = vec![DocBlock::new(
            "b1",
            DocBlockKind::Paragraph,
            "Teams lose about 40% of their week to long meetings.",
        )];
        let sugs = vec![
            DocSuggestion::new("s1", "researcher", "b1", 11, "about 40%", "38%"),
            DocSuggestion::new("s2", "writer", "b1", 38, "long", "status"),
        ];

        let (next_blocks, next_sugs) = accept_suggestion(&blocks, &sugs, "s1");
        assert_eq!(
            next_blocks[0].text.as_ref(),
            "Teams lose 38% of their week to long meetings."
        );
        assert_eq!(next_sugs.len(), 1);
        // "about 40%" (9 chars) -> "38%" (3 chars), delta = -6, so 38 -> 32
        assert_eq!(next_sugs[0].at, 32);

        let (final_blocks, final_sugs) = accept_suggestion(&next_blocks, &next_sugs, "s2");
        assert_eq!(
            final_blocks[0].text.as_ref(),
            "Teams lose 38% of their week to status meetings."
        );
        assert!(final_sugs.is_empty());
    }

    #[gpui::test]
    fn follow_accept_and_resolve_update_collaborative_state(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| AgentCursors::new("test-agent-cursors", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.suggestions().len(), 2);
            assert_eq!(v.comments().len(), 1);
            assert_eq!(v.following(), None);

            v.toggle_follow("coder", cx);
            assert_eq!(v.following(), Some("coder"));

            assert!(v.accept_one("s-num", cx));
            assert_eq!(v.suggestions().len(), 1);
            assert!(v.blocks()[1].text.contains("38%"));

            assert!(v.resolve_comment("c-date", cx));
            assert!(v.comments().is_empty());

            let remaining = v.accept_all(cx);
            assert_eq!(remaining, 1);
            assert!(v.suggestions().is_empty());
        });
    }
}
