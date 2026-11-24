//! Interactive UML sequence diagram of an autonomous multi-agent run.
//!
//! Renders vertical dashed lifelines (`You`, agents, and connected tools) with activation bars,
//! directional gradient-faded solid call arrows and dashed reply arrows via [`PathBuilder`],
//! self-call loops, labelled `par` / `retry` combined fragment frames, animated tip lights with
//! landing ripples, and click-to-inspect popover cards for any participant or message.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_check_draw, pro_ease, pro_eid, pro_empty_state, pro_fold, pro_icon_btn,
        pro_mix_color, pro_presence, pro_rolling_number, pro_skeleton_bar, pro_spring_f32,
        pro_tactile, pro_view_transition, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_app_artwork},
    },
};
use gpui::{prelude::*, *};
use std::{
    collections::{HashMap, HashSet},
    f32::consts::TAU,
};
use web_time::Instant;

const GUTTER_W: f32 = 42.0;
const MSG_ROW_H: f32 = 38.0;
const SELF_ROW_H: f32 = 44.0;
const FRAME_TOP_PAD: f32 = 18.0;
const FRAME_BOT_PAD: f32 = 6.0;

/// Kind of lifeline participant in [`AgentSequence`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SeqParticipantKind {
    /// Human user rendered with a full 3x3 ink mark.
    Person,
    /// Autonomous agent rendered with its deterministic 3x3 pixel mark.
    #[default]
    Agent,
    /// External tool or service rendered with its provider mark (`GitHub`, `Vercel`, `Stripe`, etc.).
    Tool,
}

/// One vertical lifeline column in [`AgentSequence`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqParticipant {
    /// Stable participant id referenced by [`SeqMessage::from`] and [`SeqMessage::to`].
    pub id: SharedString,
    /// Column chip label (`"You"`, `"Lead"`, `"GitHub"`).
    pub name: SharedString,
    /// Optional Chinese translation.
    pub name_zh: Option<SharedString>,
    /// Participant role kind (`Person`, `Agent`, or `Tool`).
    pub kind: SeqParticipantKind,
    /// Optional provider logo key for `Tool` (or custom `Agent`) columns (`"github"`, `"vercel"`).
    pub logo: Option<SharedString>,
    /// Optional model tag shown in the participant inspection card (`"Claude Opus"`, `"GPT-6 Sol"`).
    pub model: Option<SharedString>,
    /// Optional one-line role description shown in the participant card.
    pub description: Option<SharedString>,
    /// Optional Chinese description.
    pub description_zh: Option<SharedString>,
    /// Optional custom RGB hex color (`0xRRGGBB`).
    pub color_hex: Option<u32>,
}

impl SeqParticipant {
    /// Create a human [`SeqParticipant`].
    pub fn person(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            kind: SeqParticipantKind::Person,
            logo: None,
            model: None,
            description: None,
            description_zh: None,
            color_hex: None,
        }
    }

    /// Create an autonomous agent [`SeqParticipant`].
    pub fn agent(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            kind: SeqParticipantKind::Agent,
            logo: None,
            model: None,
            description: None,
            description_zh: None,
            color_hex: None,
        }
    }

    /// Create a tool/integration [`SeqParticipant`] with `logo` key (`"github"`, `"vercel"`, `"stripe"`).
    pub fn tool(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        logo: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            kind: SeqParticipantKind::Tool,
            logo: Some(logo.into()),
            model: None,
            description: None,
            description_zh: None,
            color_hex: None,
        }
    }

    /// Set a localized Chinese name.
    pub fn with_name_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(zh.into());
        self
    }

    /// Set the model badge shown in the participant inspection popover.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the role description shown in the participant inspection popover.
    pub fn with_description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set a localized Chinese role description.
    pub fn with_description_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.description_zh = Some(zh.into());
        self
    }

    /// Override the lifeline accent color with a custom hex (`0xRRGGBB`).
    pub fn with_color_hex(mut self, hex: u32) -> Self {
        self.color_hex = Some(hex);
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        if let Some(zh) = &self.name_zh {
            tr(cx, self.name.as_ref(), zh.as_ref())
        } else {
            self.name.as_ref()
        }
    }

    fn display_description<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.description, &self.description_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Kind of sequence arrow (`Call` solid vs `Reply` dashed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SeqMessageKind {
    /// Solid arrow that opens an activation bar on `to` until answered by a matching `Reply`.
    #[default]
    Call,
    /// Dashed return arrow closing the activation bar opened by `reply_to`.
    Reply,
}

/// Optional exception/human status on a [`SeqMessage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeqMessageStatus {
    /// Red failed arrow terminating in a cross (`×`).
    Failed,
    /// Amber arrow awaiting human input.
    Waiting,
}

/// One ordered message arrow in [`AgentSequence`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqMessage {
    /// Stable message identifier.
    pub id: SharedString,
    /// Sender [`SeqParticipant::id`].
    pub from: SharedString,
    /// Receiver [`SeqParticipant::id`] (equal to `from` for a self-loop).
    pub to: SharedString,
    /// Short pill tag shown on the arrow (`"search_code(\"emptyCart\")"`).
    pub label: SharedString,
    /// Optional Chinese translation.
    pub label_zh: Option<SharedString>,
    /// Call (`solid`) or Reply (`dashed`).
    pub kind: SeqMessageKind,
    /// Matching [`SeqMessage::id`] when `kind == Reply`.
    pub reply_to: Option<SharedString>,
    /// Optional `Failed` or `Waiting` status override.
    pub status: Option<SeqMessageStatus>,
    /// Timestamp in milliseconds from run start (`5_000` -> `"0:05"`).
    pub at_ms: Option<u64>,
    /// Optional combined fragment frame tag (`"par"`, `"retry"`, `"loop ×3"`); adjacent messages sharing a frame group together.
    pub frame: Option<SharedString>,
    /// Detailed monospace payload (arguments, return body, or error trace) shown in the click popover.
    pub detail: Option<SharedString>,
}

impl SeqMessage {
    /// Create a solid `Call` [`SeqMessage`] from `from` to `to`.
    pub fn call(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            label: label.into(),
            label_zh: None,
            kind: SeqMessageKind::Call,
            reply_to: None,
            status: None,
            at_ms: None,
            frame: None,
            detail: None,
        }
    }

    /// Create a dashed `Reply` [`SeqMessage`] answering `reply_to`.
    pub fn reply(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        reply_to: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            label: label.into(),
            label_zh: None,
            kind: SeqMessageKind::Reply,
            reply_to: Some(reply_to.into()),
            status: None,
            at_ms: None,
            frame: None,
            detail: None,
        }
    }

    /// Set a localized Chinese label.
    pub fn with_label_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(zh.into());
        self
    }

    /// Set an exception or waiting status (`Failed` or `Waiting`).
    pub fn with_status(mut self, status: SeqMessageStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the timestamp in milliseconds from run start.
    pub fn with_at_ms(mut self, at_ms: u64) -> Self {
        self.at_ms = Some(at_ms);
        self
    }

    /// Group adjacent messages into a labelled UML frame (`"par"`, `"retry"`, `"loop ×2"`).
    pub fn with_frame(mut self, frame: impl Into<SharedString>) -> Self {
        self.frame = Some(frame.into());
        self
    }

    /// Attach a monospace payload/trace shown when the message card is opened.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> &'a str {
        if let Some(zh) = &self.label_zh {
            tr(cx, self.label.as_ref(), zh.as_ref())
        } else {
            self.label.as_ref()
        }
    }
}

/// Currently inspected card in [`AgentSequence`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SequenceSelection {
    /// Participant column id.
    Participant(SharedString),
    /// Message id.
    Message(SharedString),
}

/// Typed events emitted by [`AgentSequence`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentSequenceEvent {
    /// Opened or closed a participant or message inspection card (`None` when closed).
    SelectionChanged(Option<SequenceSelection>),
    /// Replay started.
    ReplayStarted,
}

impl EventEmitter<AgentSequenceEvent> for AgentSequence {}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedSeqRow {
    pub msg_idx: usize,
    pub from_col: usize,
    pub to_col: usize,
    pub top_y: f32,
    pub height: f32,
    pub arrow_y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedSeqFrame {
    pub label: SharedString,
    pub min_col: usize,
    pub max_col: usize,
    pub top_y: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedActivationBar {
    pub col: usize,
    pub top_y: f32,
    pub bottom_y: f32,
    pub open: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedSequenceLayout {
    pub rows: Vec<ComputedSeqRow>,
    pub frames: Vec<ComputedSeqFrame>,
    pub activations: Vec<ComputedActivationBar>,
    pub total_h: f32,
}

/// Compute vertical row coordinates, `par`/`retry` frame boxes, and lifeline activation bars!
pub(crate) fn compute_sequence_layout(
    participants: &[SeqParticipant],
    messages: &[SeqMessage],
    visible_count: usize,
) -> ComputedSequenceLayout {
    let mut col_map: HashMap<&str, usize> = HashMap::with_capacity(participants.len());
    for (i, pt) in participants.iter().enumerate() {
        col_map.insert(pt.id.as_ref(), i);
    }

    // Filter out messages referencing unknown participants (defensive spec rule)
    let valid_msgs: Vec<(usize, &SeqMessage, usize, usize)> = messages
        .iter()
        .take(visible_count)
        .enumerate()
        .filter_map(|(i, m)| {
            let fc = *col_map.get(m.from.as_ref())?;
            let tc = *col_map.get(m.to.as_ref())?;
            Some((i, m, fc, tc))
        })
        .collect();

    let mut rows: Vec<ComputedSeqRow> = Vec::with_capacity(valid_msgs.len());
    let mut frames: Vec<ComputedSeqFrame> = Vec::new();
    let mut y = 8.0f32;

    let mut idx = 0usize;
    while idx < valid_msgs.len() {
        let frame_tag = valid_msgs[idx].1.frame.clone();
        if let Some(ftag) = frame_tag {
            let frame_start_y = y;
            y += FRAME_TOP_PAD;
            let mut min_c = valid_msgs[idx].2.min(valid_msgs[idx].3);
            let mut max_c = valid_msgs[idx].2.max(valid_msgs[idx].3);
            while idx < valid_msgs.len() && valid_msgs[idx].1.frame.as_ref() == Some(&ftag) {
                let (mi, _, fc, tc) = valid_msgs[idx];
                min_c = min_c.min(fc).min(tc);
                max_c = max_c.max(fc).max(tc);
                let is_self = fc == tc;
                let rh = if is_self { SELF_ROW_H } else { MSG_ROW_H };
                let arrow_y = y + if is_self { 16.0 } else { 24.5 };
                rows.push(ComputedSeqRow {
                    msg_idx: mi,
                    from_col: fc,
                    to_col: tc,
                    top_y: y,
                    height: rh,
                    arrow_y,
                });
                y += rh;
                idx += 1;
            }
            y += FRAME_BOT_PAD;
            frames.push(ComputedSeqFrame {
                label: ftag,
                min_col: min_c,
                max_col: max_c,
                top_y: frame_start_y,
                height: y - frame_start_y,
            });
            y += 6.0;
        } else {
            let (mi, _, fc, tc) = valid_msgs[idx];
            let is_self = fc == tc;
            let rh = if is_self { SELF_ROW_H } else { MSG_ROW_H };
            let arrow_y = y + if is_self { 16.0 } else { 24.5 };
            rows.push(ComputedSeqRow {
                msg_idx: mi,
                from_col: fc,
                to_col: tc,
                top_y: y,
                height: rh,
                arrow_y,
            });
            y += rh;
            idx += 1;
        }
    }

    let total_h = (y + 10.0).max(96.0);

    // Build activation bars: each `Call` (`from != to` and `status != Failed`) opens a bar on `to_col`
    // until a `Reply` with `reply_to == Some(call.id)` arrives (or `total_h - 8` if still open).
    let mut reply_y_by_call: HashMap<&str, f32> = HashMap::new();
    for r in &rows {
        let m = &messages[r.msg_idx];
        if m.kind == SeqMessageKind::Reply
            && let Some(rt) = m.reply_to.as_deref()
        {
            reply_y_by_call.entry(rt).or_insert(r.arrow_y);
        }
    }

    let mut raw_bars: Vec<ComputedActivationBar> = Vec::new();
    for r in &rows {
        let m = &messages[r.msg_idx];
        if m.kind == SeqMessageKind::Call
            && r.from_col != r.to_col
            && m.status != Some(SeqMessageStatus::Failed)
        {
            if let Some(&end_y) = reply_y_by_call.get(m.id.as_ref()) {
                raw_bars.push(ComputedActivationBar {
                    col: r.to_col,
                    top_y: r.arrow_y,
                    bottom_y: (end_y).max(r.arrow_y + 10.0),
                    open: false,
                });
            } else {
                raw_bars.push(ComputedActivationBar {
                    col: r.to_col,
                    top_y: r.arrow_y,
                    bottom_y: (total_h - 6.0).max(r.arrow_y + 14.0),
                    open: true,
                });
            }
        }
    }

    // Merge overlapping activation bars on the same column for clean visual stacking
    let mut activations: Vec<ComputedActivationBar> = Vec::new();
    for bar in raw_bars {
        if let Some(prev) = activations
            .iter_mut()
            .rfind(|b| b.col == bar.col && bar.top_y <= b.bottom_y + 2.0)
        {
            prev.bottom_y = prev.bottom_y.max(bar.bottom_y);
            prev.open |= bar.open;
        } else {
            activations.push(bar);
        }
    }

    ComputedSequenceLayout {
        rows,
        frames,
        activations,
        total_h,
    }
}

fn format_seq_ms(ms: u64) -> String {
    let total_secs = ms / 1_000;
    let h = total_secs / 3_600;
    let m = (total_secs % 3_600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn participant_seq_color(agent_idx: usize, part: &SeqParticipant, p: &ProPalette) -> Hsla {
    if let Some(hex) = part.color_hex {
        return rgb(hex).into();
    }
    match part.kind {
        SeqParticipantKind::Person => p.ink,
        SeqParticipantKind::Tool => p.muted,
        SeqParticipantKind::Agent => {
            let palette = [p.blue, p.purple, p.teal, p.green, p.pink, p.orange];
            palette[agent_idx % palette.len()]
        }
    }
}

fn render_seq_chip_icon(
    part: &SeqParticipant,
    color: Hsla,
    busy: bool,
    wall_ms: f32,
    reduced: bool,
    cx: &App,
) -> AnyElement {
    if part.kind == SeqParticipantKind::Tool
        && let Some(logo) = &part.logo
        && let Some(mark) = pro_app_artwork(logo.as_ref(), 14., cx)
    {
        return div()
            .size(px(16.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(mark)
            .into_any_element();
    }
    let cells = if part.kind == SeqParticipantKind::Person {
        [true; 9]
    } else {
        agent_pixel_cells(part.id.as_ref())
    };
    let mut grid = div()
        .size(px(16.))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(1.5));
    for r in 0..3usize {
        let mut row = div().flex().items_center().gap(px(1.5));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if busy && !reduced && part.kind == SeqParticipantKind::Agent {
                let ph = ((wall_ms + AGENT_WAVE_MS[i]) / 1200.0) * TAU;
                let w = 0.55 + 0.45 * ph.cos();
                if on { w } else { 0.16 * w }
            } else if on {
                1.0
            } else {
                0.0
            };
            row = row.child(div().size(px(3.)).rounded(px(0.8)).bg(color.opacity(alpha)));
        }
        grid = grid.child(row);
    }
    grid.into_any_element()
}

fn default_seq_participants() -> Vec<SeqParticipant> {
    vec![
        SeqParticipant::person("you", "You")
            .with_name_zh("您")
            .with_description("Prompt author & final reviewer")
            .with_description_zh("发起修复请求并进行最终确认"),
        SeqParticipant::agent("lead", "Lead")
            .with_name_zh("主控")
            .with_model("Claude Opus")
            .with_description("Coordinates root-cause lookup, patch, and preview")
            .with_description_zh("统筹根因排查、分支补丁与预览部署"),
        SeqParticipant::agent("research", "Researcher")
            .with_name_zh("检索")
            .with_model("Claude Sonnet")
            .with_description("Searches repository symbols and stack traces")
            .with_description_zh("检索代码符号引用与崩溃调用栈"),
        SeqParticipant::agent("coder", "Coder")
            .with_name_zh("编码")
            .with_model("GPT-6 Sol")
            .with_description("Applies empty-cart guard and triggers preview deploys")
            .with_description_zh("编写空购物车守卫代码并触发预览部署"),
        SeqParticipant::tool("github", "GitHub", "github")
            .with_description("Code search & pull request API")
            .with_description_zh("代码检索与合并请求接口"),
        SeqParticipant::tool("vercel", "Vercel", "vercel")
            .with_description("Ephemeral branch preview builder")
            .with_description_zh("分支预览环境自动构建服务"),
    ]
}

fn default_seq_messages() -> Vec<SeqMessage> {
    vec![
        SeqMessage::call("m1", "you", "lead", "Fix the checkout crash")
            .with_label_zh("修复结算页崩溃")
            .with_at_ms(0)
            .with_detail("POST /runs\n{\"issue\": \"TypeError: null.subtotal on /checkout\"}"),
        SeqMessage::call("m2", "lead", "lead", "Plan: find, patch, preview")
            .with_label_zh("规划：定位、修补、预览")
            .with_at_ms(2_000)
            .with_detail(
                "1. Locate unguarded cart subtotal\n2. Patch & test on branch\n3. Deploy preview",
            ),
        SeqMessage::call("m3", "lead", "research", "Find the cause")
            .with_label_zh("定位崩溃根因")
            .with_at_ms(3_000)
            .with_frame("par")
            .with_detail("delegated_task(target=\"checkout/cart.ts\")"),
        SeqMessage::call("m4", "lead", "coder", "Prepare hotfix branch")
            .with_label_zh("准备热修复分支")
            .with_at_ms(3_400)
            .with_frame("par")
            .with_detail("git checkout -b fix/empty-cart-crash"),
        SeqMessage::call("m5", "research", "github", "search_code(\"emptyCart\")")
            .with_at_ms(5_000)
            .with_frame("par")
            .with_detail("GET /search/code?q=emptyCart+repo:acme/storefront"),
        SeqMessage::reply("m6", "github", "research", "m5", "3 matches in cart.ts")
            .with_label_zh("cart.ts 中匹配 3 处")
            .with_at_ms(6_400)
            .with_frame("par")
            .with_detail("src/checkout/cart.ts:42, :142, :209"),
        SeqMessage::reply("m7", "research", "lead", "m3", "Null total at cart.ts:142")
            .with_label_zh("cart.ts:142 存在空指针")
            .with_at_ms(9_600)
            .with_detail("cart.items.reduce(...) called when cart === null"),
        SeqMessage::call("m8", "coder", "vercel", "deploy_preview(\"fix/cart\")")
            .with_status(SeqMessageStatus::Failed)
            .with_at_ms(13_000)
            .with_frame("retry")
            .with_detail("Error 422: Missing STRIPE_WEBHOOK_SECRET in preview target"),
        SeqMessage::call(
            "m9",
            "coder",
            "vercel",
            "deploy_preview(\"fix/cart\", --env)",
        )
        .with_at_ms(16_000)
        .with_frame("retry")
        .with_detail("POST /v13/deployments (with preview secret bindings)"),
    ]
}

/// Interactive UML sequence diagram of an autonomous multi-agent run.
pub struct AgentSequence {
    id: ElementId,
    title: Option<SharedString>,
    participants: Vec<SeqParticipant>,
    messages: Vec<SeqMessage>,
    running: bool,
    show_times: bool,
    replay_mode: bool,
    selection: Option<SequenceSelection>,
    last_selection: Option<SequenceSelection>,
    hovered_msg_id: Option<SharedString>,
    hovered_part_id: Option<SharedString>,
    focused_msg_idx: Option<usize>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl AgentSequence {
    /// Create an [`AgentSequence`] diagram on `"Fix the checkout crash"` across 6 lifelines and 9 messages.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: Some("Fix the checkout crash".into()),
            participants: default_seq_participants(),
            messages: default_seq_messages(),
            running: true,
            show_times: true,
            replay_mode: false,
            selection: None,
            last_selection: None,
            hovered_msg_id: None,
            hovered_part_id: None,
            focused_msg_idx: None,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the header title.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the participant lifeline columns left to right.
    pub fn with_participants(mut self, participants: Vec<SeqParticipant>) -> Self {
        self.participants = participants;
        self
    }

    /// Set the ordered sequence messages top to bottom.
    pub fn with_messages(mut self, messages: Vec<SeqMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set whether the run is still active (`false` stops open activation bar shimmer and agent twinkle).
    pub fn with_running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    /// Show or hide the left timestamp gutter (`true` by default).
    pub fn with_times(mut self, show_times: bool) -> Self {
        self.show_times = show_times;
        self
    }

    /// Render skeleton placeholder columns and rows.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state view.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Select or close (`None`) a participant or message inspection card.
    pub fn set_selection(&mut self, next: Option<SequenceSelection>, cx: &mut Context<Self>) {
        if self.selection != next {
            self.selection = next.clone();
            cx.emit(AgentSequenceEvent::SelectionChanged(next));
            cx.notify();
        }
    }

    /// Replay the sequence diagram message-by-message with animated tip lights and landing ripples.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_mode = true;
        self.selection = None;
        self.started_at = Instant::now();
        cx.emit(AgentSequenceEvent::ReplayStarted);
        cx.notify();
    }
}

impl Render for AgentSequence {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let wall_ms = elapsed_s * 1000.0;

        let total_msgs = self.messages.len();
        // During replay (or subtle live tail draw on mount), reveal messages sequentially with animated tip light!
        let (visible_msgs, latest_draw_progress) = if self.replay_mode && !reduced && total_msgs > 0
        {
            let step_dur = 0.58f32;
            let raw = elapsed_s / step_dur;
            let count = ((raw.floor() as usize) + 1).clamp(1, total_msgs);
            let prog = (raw.fract() / 0.78).clamp(0., 1.);
            if count >= total_msgs && raw.fract() >= 0.95 {
                self.replay_mode = false;
                (total_msgs, 1.0f32)
            } else {
                (count, prog)
            }
        } else if !reduced && elapsed_s < 0.75 && total_msgs > 0 {
            // Animate the final message drawing in with tip light on initial mount!
            (total_msgs, (elapsed_s / 0.65).clamp(0.08, 1.0))
        } else {
            (total_msgs, 1.0f32)
        };

        if ((self.running || self.replay_mode || latest_draw_progress < 0.999) && !reduced)
            || (self.loading && !reduced)
        {
            window.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let life_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.16)
        } else {
            hsla(0., 0., 0., 0.14)
        };
        let amber_c: Hsla = if p.dark {
            rgb(0xffb35c).into()
        } else {
            rgb(0xe08a00).into()
        };
        let red_c = p.red;
        let blue_fg: Hsla = if p.dark {
            rgb(0x6cbcff).into()
        } else {
            rgba(0x006dcbf2).into()
        };

        let layout = compute_sequence_layout(&self.participants, &self.messages, visible_msgs);
        let working_cols: usize = if self.running {
            layout
                .activations
                .iter()
                .filter(|a| a.open)
                .map(|a| a.col)
                .collect::<std::collections::HashSet<_>>()
                .len()
        } else {
            0
        };
        let failed_cnt = self
            .messages
            .iter()
            .take(visible_msgs)
            .filter(|m| m.status == Some(SeqMessageStatus::Failed))
            .count();
        let max_at_ms = self
            .messages
            .iter()
            .take(visible_msgs)
            .filter_map(|m| m.at_ms)
            .max()
            .unwrap_or(0);

        let title_str: SharedString = self
            .title
            .as_deref()
            .map(|t| {
                if t == "Fix the checkout crash" {
                    tr(cx, "Fix the checkout crash", "修复结算页崩溃")
                } else {
                    t
                }
            })
            .unwrap_or_else(|| tr(cx, "Agent sequence", "智能体序列图"))
            .into();

        if let Some(s) = &self.selection {
            self.last_selection = Some(s.clone());
        }

        let msg_sfx = if tr(cx, "en", "zh") == "zh" {
            " 条消息"
        } else if layout.rows.len() == 1 {
            " message"
        } else {
            " messages"
        };

        let header = div()
            .mb(px(14.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .child(
                div()
                    .flex_none()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .when(working_cols > 0, |d| {
                        let w_txt = if tr(cx, "en", "zh") == "zh" {
                            format!("{working_cols} 个执行中")
                        } else {
                            format!("{working_cols} working")
                        };
                        d.child(div().text_color(blue_fg).child(w_txt))
                    })
                    .when(failed_cnt > 0, |d| {
                        let f_txt = if tr(cx, "en", "zh") == "zh" {
                            format!("{failed_cnt} 次失败")
                        } else {
                            format!("{failed_cnt} failed")
                        };
                        d.child(div().text_color(red_c).child(f_txt))
                    })
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "asq-msg-cnt"),
                        layout.rows.len() as f64,
                        0,
                        "",
                        msg_sfx,
                        12.0,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .child(format_seq_ms(max_at_ms))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "asq-replay"),
                        tr(cx, "Replay sequence diagram", "重放序列图"),
                        ProIcon::Refresh,
                        22.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.replay(cx);
                        },
                    )),
            );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (elapsed_s * 0.9).fract()
            };
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(760.))
                        .p(px(16.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(header)
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .child(pro_skeleton_bar(72., 26., 7., phase, cx))
                                .child(pro_skeleton_bar(78., 26., 7., phase + 0.1, cx))
                                .child(pro_skeleton_bar(92., 26., 7., phase + 0.2, cx))
                                .child(pro_skeleton_bar(76., 26., 7., phase + 0.3, cx))
                                .child(pro_skeleton_bar(80., 26., 7., phase + 0.4, cx)),
                        )
                        .child(pro_skeleton_bar(700., 180., 8., phase + 0.15, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.participants.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Thread,
                tr(cx, "No sequence messages", "暂无交互消息"),
                tr(
                    cx,
                    "Calls, replies, and parallel frames between agents and tools appear here.",
                    "人类用户、智能体与外部工具之间的调用、返回及并行片段将在此绘制。",
                ),
                None,
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
                        .max_w(px(760.))
                        .p(px(16.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        let n_cols = self.participants.len();
        let gutter = if self.show_times { GUTTER_W } else { 8.0 };
        let mut col_colors: Vec<Hsla> = Vec::with_capacity(n_cols);
        let mut agent_ord = 0usize;
        for pt in &self.participants {
            let col = participant_seq_color(agent_ord, pt, &p);
            if pt.kind == SeqParticipantKind::Agent {
                agent_ord += 1;
            }
            col_colors.push(col);
        }

        // Connected call/reply chain spotlight via pro_spring_f32!
        let focus_msg = self
            .hovered_msg_id
            .clone()
            .or_else(|| match &self.selection {
                Some(SequenceSelection::Message(m)) => Some(m.clone()),
                _ => None,
            });
        let focus_part = self
            .hovered_part_id
            .clone()
            .or_else(|| match &self.selection {
                Some(SequenceSelection::Participant(pt)) => Some(pt.clone()),
                _ => None,
            });
        let mut lit_msgs: HashSet<SharedString> = HashSet::new();
        let mut lit_parts: HashSet<SharedString> = HashSet::new();
        let has_spotlight = if let Some(fmid) = &focus_msg {
            lit_msgs.insert(fmid.clone());
            if let Some(fm) = self.messages.iter().find(|m| &m.id == fmid) {
                lit_parts.insert(fm.from.clone());
                lit_parts.insert(fm.to.clone());
                if let Some(rt) = &fm.reply_to {
                    lit_msgs.insert(rt.clone());
                    if let Some(cm) = self.messages.iter().find(|m| &m.id == rt) {
                        lit_parts.insert(cm.from.clone());
                        lit_parts.insert(cm.to.clone());
                    }
                }
                for m in &self.messages {
                    if m.reply_to.as_ref() == Some(fmid) {
                        lit_msgs.insert(m.id.clone());
                    }
                    if let Some(fr) = &fm.frame
                        && fr.as_ref() == "retry"
                        && m.frame.as_ref() == Some(fr)
                    {
                        lit_msgs.insert(m.id.clone());
                    }
                }
            }
            true
        } else if let Some(fpid) = &focus_part {
            lit_parts.insert(fpid.clone());
            for m in self.messages.iter().take(visible_msgs) {
                if &m.from == fpid || &m.to == fpid {
                    lit_msgs.insert(m.id.clone());
                    lit_parts.insert(m.from.clone());
                    lit_parts.insert(m.to.clone());
                }
            }
            true
        } else {
            false
        };

        let head_opacities: Vec<f32> = self
            .participants
            .iter()
            .map(|pt| {
                let target_op = if has_spotlight {
                    if lit_parts.contains(&pt.id) {
                        1.0_f32
                    } else {
                        0.42_f32
                    }
                } else {
                    1.0_f32
                };
                pro_spring_f32(
                    pro_eid(&self.id, format!("seq-hop-{}", pt.id)),
                    target_op,
                    0.22,
                    window,
                    cx,
                )
            })
            .collect();

        let row_opacities: Vec<f32> = layout
            .rows
            .iter()
            .map(|r| {
                let mid = &self.messages[r.msg_idx].id;
                let target_op = if has_spotlight {
                    if lit_msgs.contains(mid) {
                        1.0_f32
                    } else {
                        0.28_f32
                    }
                } else {
                    1.0_f32
                };
                pro_spring_f32(
                    pro_eid(&self.id, format!("seq-mop-{mid}")),
                    target_op,
                    0.22,
                    window,
                    cx,
                )
            })
            .collect();

        let answered_call_ids: HashSet<SharedString> = self
            .messages
            .iter()
            .take(visible_msgs)
            .filter_map(|m| {
                if m.kind == SeqMessageKind::Reply && m.status.is_none() {
                    m.reply_to.clone()
                } else {
                    None
                }
            })
            .collect();

        // Participant header pills (`ui-asqc__heads`)
        let mut heads_row = div().w_full().pl(px(gutter)).h(px(28.)).mb(px(8.)).flex();
        for (c_idx, pt) in self.participants.iter().enumerate() {
            let pid = pt.id.clone();
            let is_busy =
                self.running && layout.activations.iter().any(|a| a.col == c_idx && a.open);
            let is_selected = self.selection == Some(SequenceSelection::Participant(pid.clone()));
            let hop = head_opacities.get(c_idx).copied().unwrap_or(1.0);
            let icon_el =
                render_seq_chip_icon(pt, col_colors[c_idx], is_busy, wall_ms, reduced, cx);
            let click_pid = pid.clone();
            let hov_pid = pid.clone();

            heads_row = heads_row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("seq-head-{pid}")),
                            pt.display_name(cx).to_string(),
                            7.0,
                            card_bg,
                            p.hover,
                            if is_selected { p.focus } else { p.line_strong },
                            p.line_strong,
                            is_selected,
                            div()
                                .h(px(28.))
                                .pl(px(6.))
                                .pr(px(9.))
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .child(icon_el)
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(pt.display_name(cx).to_string()),
                                ),
                            cx,
                            move |this, _, cx| {
                                let target =
                                    Some(SequenceSelection::Participant(click_pid.clone()));
                                let next = if this.selection == target {
                                    None
                                } else {
                                    target
                                };
                                this.set_selection(next, cx);
                            },
                        )
                        .h(px(28.))
                        .opacity(hop)
                        .when(!p.dark, |d| d.shadow_xs())
                        .on_hover(cx.listener(
                            move |this, inside: &bool, _, cx| {
                                let next = if *inside {
                                    Some(hov_pid.clone())
                                } else if this.hovered_part_id.as_ref() == Some(&hov_pid) {
                                    None
                                } else {
                                    this.hovered_part_id.clone()
                                };
                                if this.hovered_part_id != next {
                                    this.hovered_part_id = next;
                                    cx.notify();
                                }
                            },
                        )),
                    ),
            );
        }

        // Canvas painting: labelled `par` / `retry` frames, dashed lifelines, activation bars (+ shimmer sweep),
        // directional arrows (solid call, dashed reply, self-call loops, failed `×`), tip light, and landing ripple!
        let layout_snap = layout.clone();
        let msgs_snap = self.messages.clone();
        let col_colors_snap = col_colors.clone();
        let row_ops_snap = row_opacities.clone();
        let running_flag = self.running;
        let sweep_phase = if reduced {
            0.0
        } else {
            (elapsed_s / 1.40).fract()
        };
        let frame_border_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.14)
        } else {
            hsla(0., 0., 0., 0.12)
        };

        let diagram_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, win, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                let plot_w = (bw - gutter).max(20.0);
                if n_cols == 0 || bh <= 4.0 {
                    return;
                }
                let ox = f32::from(bounds.left()) + gutter;
                let oy = f32::from(bounds.top());
                let col_w = plot_w / (n_cols as f32);
                let col_x = |c: usize| -> f32 { ox + (c as f32 + 0.5) * col_w };

                // 1) Combined fragment frames (`par`, `retry`) background + hairline border
                for fr in &layout_snap.frames {
                    let fx0 = (col_x(fr.min_col) - col_w * 0.36).max(ox + 2.0);
                    let fx1 = (col_x(fr.max_col) + col_w * 0.36).min(ox + plot_w - 2.0);
                    let f_rect = Bounds::new(
                        point(px(fx0), px(oy + fr.top_y)),
                        size(px((fx1 - fx0).max(24.0)), px(fr.height)),
                    );
                    win.paint_quad(quad(
                        f_rect,
                        Corners::all(px(8.)),
                        card_bg,
                        Edges::all(px(1.)),
                        frame_border_c,
                        BorderStyle::Solid,
                    ));
                }

                // 2) Vertical dashed lifelines
                for c in 0..n_cols {
                    let lx = col_x(c);
                    let nd = ((bh / 7.0).ceil() as usize).max(1);
                    for d in 0..nd {
                        let y0 = oy + (d as f32) * 7.0;
                        let y1 = (y0 + 3.0).min(oy + bh);
                        if y1 > y0 {
                            let mut pb = PathBuilder::stroke(px(1.0));
                            pb.move_to(point(px(lx), px(y0)));
                            pb.line_to(point(px(lx), px(y1)));
                            if let Ok(path) = pb.build() {
                                win.paint_path(path, life_c);
                            }
                        }
                    }
                }

                // 3) Lifeline activation bars (`6px` wide) + vertical shimmer sweep on open bars!
                for act in &layout_snap.activations {
                    let cx_bar = col_x(act.col);
                    let c_col = col_colors_snap[act.col];
                    let bar_h = (act.bottom_y - act.top_y).max(8.0);
                    let bar_rect = Bounds::new(
                        point(px(cx_bar - 3.0), px(oy + act.top_y)),
                        size(px(6.0), px(bar_h)),
                    );
                    win.paint_quad(quad(
                        bar_rect,
                        Corners::all(px(3.)),
                        pro_mix_color(card_bg, c_col, 0.16),
                        Edges::all(px(1.)),
                        c_col.opacity(0.55),
                        BorderStyle::Solid,
                    ));
                    if act.open && running_flag && !reduced {
                        let glint_y = oy + act.top_y + sweep_phase * (bar_h - 14.0).max(0.0);
                        let glint_h = 14.0f32.min(bar_h);
                        win.paint_quad(
                            fill(
                                Bounds::new(
                                    point(px(cx_bar - 2.0), px(glint_y)),
                                    size(px(4.0), px(glint_h)),
                                ),
                                c_col.opacity(0.45),
                            )
                            .corner_radii(px(2.)),
                        );
                    }
                }

                // 4) Message arrows, self-loops, animated tip light, and landing ripples!
                let n_rows = layout_snap.rows.len();
                for (r_pos, row) in layout_snap.rows.iter().enumerate() {
                    let msg = &msgs_snap[row.msg_idx];
                    let is_last_row = r_pos + 1 == n_rows;
                    let draw_prog = if is_last_row {
                        pro_ease(latest_draw_progress)
                    } else {
                        1.0
                    };

                    let dim_mul = row_ops_snap.get(r_pos).copied().unwrap_or(1.0);

                    let base_col = match msg.status {
                        Some(SeqMessageStatus::Failed) => red_c,
                        Some(SeqMessageStatus::Waiting) => amber_c,
                        None => col_colors_snap[row.from_col],
                    };

                    if row.from_col == row.to_col {
                        // Self-call loop (`from == to`)
                        let lx = col_x(row.from_col);
                        let dir_sign = if row.from_col + 1 == n_cols && n_cols > 1 {
                            -1.0
                        } else {
                            1.0
                        };
                        let x_start = lx + dir_sign * 3.5;
                        let x_loop = lx + dir_sign * 19.0;
                        let y_top = oy + row.arrow_y;
                        let y_bot = y_top + 15.0;
                        let mut loop_pb = PathBuilder::stroke(px(1.25));
                        loop_pb.move_to(point(px(x_start), px(y_top)));
                        loop_pb.line_to(point(px(x_loop), px(y_top)));
                        loop_pb.line_to(point(px(x_loop), px(y_bot)));
                        loop_pb.line_to(point(px(x_start), px(y_bot)));
                        if let Ok(path) = loop_pb.build() {
                            win.paint_path(path, base_col.opacity(0.80 * dim_mul));
                        }
                        // Arrowhead pointing back into lifeline at `y_bot`
                        let mut head = PathBuilder::fill();
                        head.move_to(point(px(x_start), px(y_bot)));
                        head.line_to(point(px(x_start + dir_sign * 5.0), px(y_bot - 3.0)));
                        head.line_to(point(px(x_start + dir_sign * 5.0), px(y_bot + 3.0)));
                        head.close();
                        if let Ok(path) = head.build() {
                            win.paint_path(path, base_col.opacity(0.85 * dim_mul));
                        }
                    } else {
                        let x_from_raw = col_x(row.from_col);
                        let x_to_raw = col_x(row.to_col);
                        let dir = (x_to_raw - x_from_raw).signum();
                        let x0 = x_from_raw + dir * 4.0;
                        let is_failed = msg.status == Some(SeqMessageStatus::Failed);
                        let x1_target = x_to_raw - dir * if is_failed { 11.0 } else { 4.5 };
                        let x_cur = x0 + (x1_target - x0) * draw_prog;
                        let ay = oy + row.arrow_y;

                        // Directional gradient-faded arrow shaft (faint at origin -> full at destination!)
                        let segs = 18usize;
                        for s in 0..segs {
                            let u0 = s as f32 / segs as f32;
                            let u1 = (s + 1) as f32 / segs as f32;
                            if msg.kind == SeqMessageKind::Reply && s % 2 == 1 {
                                continue;
                            }
                            let sx0 = x0 + (x_cur - x0) * u0;
                            let sx1 = x0 + (x_cur - x0) * u1;
                            let grad_alpha = (0.20 + 0.72 * u1) * dim_mul;
                            let mut seg = PathBuilder::stroke(px(1.3));
                            seg.move_to(point(px(sx0), px(ay)));
                            seg.line_to(point(px(sx1), px(ay)));
                            if let Ok(path) = seg.build() {
                                win.paint_path(path, base_col.opacity(grad_alpha));
                            }
                        }

                        // Animated glowing tip light while drawing across!
                        if is_last_row && latest_draw_progress < 0.98 && !reduced {
                            let tip_w = 24.0f32.min((x_cur - x0).abs());
                            let tx_left = if dir > 0.0 { x_cur - tip_w } else { x_cur };
                            win.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(px(tx_left), px(ay - 2.0)),
                                        size(px(tip_w.max(2.0)), px(4.0)),
                                    ),
                                    base_col.opacity(0.55),
                                )
                                .corner_radii(px(2.)),
                            );
                        }

                        // Arrowhead or Failed `×` cross once tip reaches target
                        if draw_prog >= 0.85 {
                            if is_failed {
                                let cx_cross = x_to_raw - dir * 6.5;
                                let mut c1 = PathBuilder::stroke(px(1.45));
                                c1.move_to(point(px(cx_cross - 3.2), px(ay - 3.2)));
                                c1.line_to(point(px(cx_cross + 3.2), px(ay + 3.2)));
                                if let Ok(path) = c1.build() {
                                    win.paint_path(path, red_c.opacity(dim_mul));
                                }
                                let mut c2 = PathBuilder::stroke(px(1.45));
                                c2.move_to(point(px(cx_cross + 3.2), px(ay - 3.2)));
                                c2.line_to(point(px(cx_cross - 3.2), px(ay + 3.2)));
                                if let Ok(path) = c2.build() {
                                    win.paint_path(path, red_c.opacity(dim_mul));
                                }
                            } else if msg.kind == SeqMessageKind::Reply {
                                // Open chevron head for dashed reply
                                let mut ch = PathBuilder::stroke(px(1.35));
                                ch.move_to(point(px(x1_target - dir * 5.2), px(ay - 3.2)));
                                ch.line_to(point(px(x1_target), px(ay)));
                                ch.line_to(point(px(x1_target - dir * 5.2), px(ay + 3.2)));
                                if let Ok(path) = ch.build() {
                                    win.paint_path(path, base_col.opacity(0.90 * dim_mul));
                                }
                            } else {
                                // Solid filled triangle head for call
                                let mut tri = PathBuilder::fill();
                                tri.move_to(point(px(x1_target), px(ay)));
                                tri.line_to(point(px(x1_target - dir * 5.8), px(ay - 3.3)));
                                tri.line_to(point(px(x1_target - dir * 5.8), px(ay + 3.3)));
                                tri.close();
                                if let Ok(path) = tri.build() {
                                    win.paint_path(path, base_col.opacity(0.92 * dim_mul));
                                }
                            }
                        }

                        // Landing ripple right when the arrow lands (`0.72..1.0`)
                        if is_last_row && (0.70..0.995).contains(&latest_draw_progress) && !reduced
                        {
                            let rt = (latest_draw_progress - 0.70) / 0.30;
                            let rr = 2.5 + 7.5 * pro_ease(rt);
                            let ra = (1.0 - rt) * 0.45;
                            let mut ring = PathBuilder::stroke(px(1.3));
                            for i in 0..=18 {
                                let a = (i as f32 / 18.0) * TAU;
                                let pt = point(px(x1_target + rr * a.cos()), px(ay + rr * a.sin()));
                                if i == 0 {
                                    ring.move_to(pt);
                                } else {
                                    ring.line_to(pt);
                                }
                            }
                            if let Ok(path) = ring.build() {
                                win.paint_path(path, base_col.opacity(ra));
                            }
                        }
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let mut body_box = div()
            .id(pro_eid(&self.id, "seq-body"))
            .role(Role::List)
            .aria_label(tr(
                cx,
                "Sequence messages; use Up and Down arrow keys to walk messages and Enter to inspect",
                "序列图交互消息列表；使用上下方向键切换并按 Enter 打开详情卡片",
            ))
            .tab_index(0)
            .w_full()
            .h(px(layout.total_h))
            .relative()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if this.messages.is_empty() {
                    return;
                }
                let max_i = this.messages.len() - 1;
                let cur = this.focused_msg_idx.unwrap_or(0).min(max_i);
                match ev.keystroke.key.as_str() {
                    "down" | "arrowdown" => {
                        let next_i = (cur + 1).min(max_i);
                        this.focused_msg_idx = Some(next_i);
                        let mid = this.messages[next_i].id.clone();
                        this.set_selection(Some(SequenceSelection::Message(mid)), cx);
                        cx.stop_propagation();
                    }
                    "up" | "arrowup" => {
                        let next_i = cur.saturating_sub(1);
                        this.focused_msg_idx = Some(next_i);
                        let mid = this.messages[next_i].id.clone();
                        this.set_selection(Some(SequenceSelection::Message(mid)), cx);
                        cx.stop_propagation();
                    }
                    "home" => {
                        this.focused_msg_idx = Some(0);
                        let mid = this.messages[0].id.clone();
                        this.set_selection(Some(SequenceSelection::Message(mid)), cx);
                        cx.stop_propagation();
                    }
                    "end" => {
                        this.focused_msg_idx = Some(max_i);
                        let mid = this.messages[max_i].id.clone();
                        this.set_selection(Some(SequenceSelection::Message(mid)), cx);
                        cx.stop_propagation();
                    }
                    "enter" | "space" | " " => {
                        let mid = this.messages[cur].id.clone();
                        let target = Some(SequenceSelection::Message(mid));
                        let next = if this.selection == target {
                            None
                        } else {
                            target
                        };
                        this.set_selection(next, cx);
                        cx.stop_propagation();
                    }
                    "escape" if this.selection.is_some() => {
                        this.set_selection(None, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(diagram_canvas);

        // Render uppercase frame corner badges (`PAR`, `RETRY`)
        for fr in &layout.frames {
            let left_frac = ((fr.min_col as f32 + 0.14) / (n_cols as f32)).clamp(0., 0.90);
            body_box = body_box.child(
                div()
                    .absolute()
                    .top(px(fr.top_y + 4.0))
                    .left(px(gutter))
                    .right_0()
                    .child(
                        div().relative().w_full().child(
                            div()
                                .absolute()
                                .left(relative(left_frac))
                                .ml(px(6.))
                                .h(px(15.))
                                .px(px(5.))
                                .rounded(px(4.))
                                .bg(card_bg)
                                .border_1()
                                .border_color(frame_border_c)
                                .text_size(px(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .flex()
                                .items_center()
                                .child(fr.label.to_uppercase()),
                        ),
                    ),
            );
        }

        // Render interactive message rows (left time + opaque colored pill tag above arrow!)
        for (r_pos, row) in layout.rows.iter().enumerate() {
            let msg = &self.messages[row.msg_idx];
            let mid = msg.id.clone();
            let is_open = self.selection == Some(SequenceSelection::Message(mid.clone()));
            let mop = row_opacities.get(r_pos).copied().unwrap_or(1.0);
            let show_done_chk = msg.status.is_none()
                && (msg.kind == SeqMessageKind::Reply || answered_call_ids.contains(&mid));

            let tag_tint = match msg.status {
                Some(SeqMessageStatus::Failed) => red_c,
                Some(SeqMessageStatus::Waiting) => amber_c,
                None => col_colors[row.from_col],
            };
            let tag_bg = pro_mix_color(card_bg, tag_tint, if p.dark { 0.16 } else { 0.12 });
            let tag_hover_bg = pro_mix_color(card_bg, tag_tint, if p.dark { 0.24 } else { 0.18 });
            let tag_fg = pro_mix_color(tag_tint, p.strong, 0.22);

            let lbl_str = msg.display_label(cx).to_string();
            let is_self = row.from_col == row.to_col;
            let tag_click_mid = mid.clone();
            let tag_row_i = row.msg_idx;
            let from_name = self.participants[row.from_col].display_name(cx);
            let to_name = self.participants[row.to_col].display_name(cx);

            let tag_pill = pro_tactile(
                pro_eid(&self.id, format!("seq-tag-{mid}")),
                format!("{from_name} → {to_name}: {lbl_str}"),
                5.0,
                tag_bg,
                tag_hover_bg,
                if is_open {
                    tag_tint.opacity(0.55)
                } else {
                    transparent_black()
                },
                tag_tint.opacity(0.45),
                is_open,
                div()
                    .h(px(19.))
                    .px(px(6.))
                    .max_w(px(230.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .when(show_done_chk, |d| {
                        d.child(pro_check_draw(
                            pro_eid(&self.id, format!("seq-chk-{mid}")),
                            true,
                            12.0,
                            false,
                            tag_fg,
                        ))
                    })
                    .child(
                        div()
                            .truncate()
                            .text_size(px(11.5))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(tag_fg)
                            .child(lbl_str.clone()),
                    )
                    .when(msg.status == Some(SeqMessageStatus::Failed), |d| {
                        d.child(
                            div()
                                .flex_none()
                                .text_size(px(10.5))
                                .text_color(red_c)
                                .child(format!("· {}", tr(cx, "Failed", "失败"))),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.focused_msg_idx = Some(tag_row_i);
                    let target = Some(SequenceSelection::Message(tag_click_mid.clone()));
                    let next = if this.selection == target {
                        None
                    } else {
                        target
                    };
                    this.set_selection(next, cx);
                },
            )
            .h(px(19.))
            .on_click(|_, _, cx| cx.stop_propagation());

            let tag_positioned: AnyElement = if is_self {
                let on_last = row.from_col + 1 == n_cols && n_cols > 1;
                let anchor_frac = (row.from_col as f32 + 0.5) / (n_cols as f32);
                div()
                    .w_full()
                    .h_full()
                    .relative()
                    .child(if on_last {
                        div()
                            .absolute()
                            .top(px(11.))
                            .right(relative((1.0 - anchor_frac + 0.03).clamp(0., 0.92)))
                            .child(tag_pill)
                    } else {
                        div()
                            .absolute()
                            .top(px(11.))
                            .left(relative((anchor_frac + 0.03).clamp(0., 0.85)))
                            .child(tag_pill)
                    })
                    .into_any_element()
            } else {
                let c_min = row.from_col.min(row.to_col);
                let c_max = row.from_col.max(row.to_col);
                let left_f = ((c_min as f32 + 0.5) / (n_cols as f32)).clamp(0., 0.95);
                let span_f = (((c_max - c_min) as f32) / (n_cols as f32)).clamp(0.08, 1.0 - left_f);
                div()
                    .w_full()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top(px(2.))
                            .left(relative(left_f))
                            .w(relative(span_f))
                            .flex()
                            .justify_center()
                            .child(tag_pill),
                    )
                    .into_any_element()
            };

            let click_mid = mid.clone();
            let hov_mid = mid.clone();
            let row_i = row.msg_idx;

            body_box = body_box.child(
                div()
                    .id(pro_eid(&self.id, format!("seq-msg-{mid}")))
                    .role(Role::ListItem)
                    .aria_label(format!("{from_name} → {to_name}: {lbl_str}"))
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(row.top_y))
                    .h(px(row.height))
                    .opacity(mop)
                    .flex()
                    .items_start()
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = if *inside {
                            Some(hov_mid.clone())
                        } else if this.hovered_msg_id.as_ref() == Some(&hov_mid) {
                            None
                        } else {
                            this.hovered_msg_id.clone()
                        };
                        if this.hovered_msg_id != next {
                            this.hovered_msg_id = next;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.focused_msg_idx = Some(row_i);
                        let target = Some(SequenceSelection::Message(click_mid.clone()));
                        let next = if this.selection == target {
                            None
                        } else {
                            target
                        };
                        this.set_selection(next, cx);
                    }))
                    .when(self.show_times, |d| {
                        let t_txt = msg.at_ms.map(format_seq_ms).unwrap_or_default();
                        d.child(
                            div()
                                .w(px(gutter - 8.0))
                                .mr(px(8.))
                                .pt(px(if is_self { 12. } else { 16. }))
                                .flex_none()
                                .text_right()
                                .text_size(px(11.))
                                .line_height(px(15.))
                                .text_color(p.faint)
                                .child(t_txt),
                        )
                    })
                    .child(div().flex_1().h_full().child(tag_positioned)),
            );
        }

        // Click-to-Inspect Popover Card (`Participant` OR `Message`) with bidirectional presence!
        let is_pop_open = self.selection.is_some();
        let shown_sel = self
            .selection
            .clone()
            .or_else(|| self.last_selection.clone());
        let pop_card: AnyElement = match shown_sel {
            Some(SequenceSelection::Participant(sel_pid)) => {
                let part_opt = self
                    .participants
                    .iter()
                    .enumerate()
                    .find(|(_, pt)| pt.id == sel_pid);
                if let Some((c_idx, pt)) = part_opt {
                    let pname = pt.display_name(cx).to_string();
                    let pdesc = pt.display_description(cx).map(str::to_string);
                    let pmodel = pt.model.clone();
                    let sent_n = self
                        .messages
                        .iter()
                        .take(visible_msgs)
                        .filter(|m| m.from == sel_pid && m.from != m.to)
                        .count();
                    let recv_n = self
                        .messages
                        .iter()
                        .take(visible_msgs)
                        .filter(|m| m.to == sel_pid && m.from != m.to)
                        .count();
                    let is_busy_now =
                        self.running && layout.activations.iter().any(|a| a.col == c_idx && a.open);
                    let kind_tag = match pt.kind {
                        SeqParticipantKind::Person => tr(cx, "Person", "用户"),
                        SeqParticipantKind::Agent => tr(cx, "Agent", "智能体"),
                        SeqParticipantKind::Tool => tr(cx, "Tool", "工具"),
                    }
                    .to_string();
                    let working_tag = tr(cx, "Working", "执行中").to_string();
                    let sent_lbl = tr(cx, "Sent", "发送").to_string();
                    let recv_lbl = tr(cx, "Received", "接收").to_string();
                    let align_right = c_idx * 2 >= n_cols;
                    let col_frac = ((c_idx as f32 + 0.5) / (n_cols as f32)).clamp(0.05, 0.90);
                    let close_btn = pro_icon_btn(
                        "seq-pcard-close",
                        "Close card",
                        ProIcon::Close,
                        22.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.set_selection(None, cx);
                        },
                    );
                    let sent_num = pro_rolling_number(
                        pro_eid(&self.id, "seq-sent-cnt"),
                        sent_n as f64,
                        0,
                        "",
                        "",
                        12.5,
                        FontWeight::MEDIUM,
                        p.ink,
                    );
                    let recv_num = pro_rolling_number(
                        pro_eid(&self.id, "seq-recv-cnt"),
                        recv_n as f64,
                        0,
                        "",
                        "",
                        12.5,
                        FontWeight::MEDIUM,
                        p.ink,
                    );
                    let card_vtx = pro_view_transition(
                        pro_eid(&self.id, "seq-pop-vtx"),
                        format!("part:{sel_pid}"),
                        div()
                            .id("seq-participant-dialog")
                            .role(Role::Dialog)
                            .w(px(244.))
                            .p(px(10.))
                            .rounded(px(9.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow_md()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .truncate()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(pname),
                                    )
                                    .child(close_btn),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(5.))
                                            .bg(p.chip)
                                            .text_size(px(11.))
                                            .text_color(p.text)
                                            .child(kind_tag),
                                    )
                                    .when_some(pmodel, |d, m| {
                                        d.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(5.))
                                                .bg(p.chip)
                                                .text_size(px(11.))
                                                .text_color(p.text)
                                                .child(m),
                                        )
                                    })
                                    .when(is_busy_now, |d| {
                                        d.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(5.))
                                                .bg(blue_fg.opacity(0.14))
                                                .text_size(px(11.))
                                                .text_color(blue_fg)
                                                .child(working_tag),
                                        )
                                    }),
                            )
                            .when_some(pdesc, |d, desc| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.text)
                                        .child(desc),
                                )
                            })
                            .child(
                                div()
                                    .pt(px(7.))
                                    .border_t_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .gap(px(18.))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(p.muted)
                                                    .child(sent_lbl),
                                            )
                                            .child(sent_num),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(p.muted)
                                                    .child(recv_lbl),
                                            )
                                            .child(recv_num),
                                    ),
                            ),
                    );

                    pro_presence(
                        pro_eid(&self.id, "seq-pop-pres"),
                        is_pop_open,
                        0.16,
                        0.12,
                        move |amt| {
                            let wrap = div().absolute().top(px(2. + 3. * (1. - amt))).opacity(amt);
                            if align_right {
                                wrap.right(px(8.)).child(card_vtx).into_any_element()
                            } else {
                                wrap.left(relative(col_frac.clamp(0.02, 0.55)))
                                    .child(card_vtx)
                                    .into_any_element()
                            }
                        },
                    )
                } else {
                    div().into_any_element()
                }
            }
            Some(SequenceSelection::Message(sel_mid)) => {
                let found = layout
                    .rows
                    .iter()
                    .find(|r| self.messages[r.msg_idx].id == sel_mid);
                if let Some(row) = found {
                    let msg = &self.messages[row.msg_idx];
                    let from_name = self.participants[row.from_col].display_name(cx).to_string();
                    let to_name = self.participants[row.to_col].display_name(cx).to_string();
                    let route = if row.from_col == row.to_col {
                        format!("{from_name} ↻")
                    } else {
                        format!("{from_name} → {to_name}")
                    };
                    let lbl_txt = msg.display_label(cx).to_string();
                    let tint = match msg.status {
                        Some(SeqMessageStatus::Failed) => red_c,
                        Some(SeqMessageStatus::Waiting) => amber_c,
                        None => col_colors[row.from_col],
                    };
                    let kind_chip = match msg.kind {
                        SeqMessageKind::Call => tr(cx, "Call", "调用"),
                        SeqMessageKind::Reply => tr(cx, "Reply", "返回"),
                    }
                    .to_string();
                    let frame_chip = msg.frame.as_ref().map(|f| f.to_uppercase());
                    let status_chip: Option<(String, Hsla)> = match msg.status {
                        Some(SeqMessageStatus::Failed) => {
                            Some((tr(cx, "Failed", "失败").to_string(), red_c))
                        }
                        Some(SeqMessageStatus::Waiting) => {
                            Some((tr(cx, "Waiting", "待确认").to_string(), amber_c))
                        }
                        None => None,
                    };
                    let detail = msg.detail.clone();

                    // Compute call->reply roundtrip latency if paired!
                    let latency_ms: Option<u64> = if msg.kind == SeqMessageKind::Call
                        && let Some(t0) = msg.at_ms
                    {
                        self.messages
                            .iter()
                            .find(|m| m.reply_to.as_ref() == Some(&msg.id))
                            .and_then(|rep| rep.at_ms)
                            .map(|t1| t1.saturating_sub(t0))
                    } else if msg.kind == SeqMessageKind::Reply
                        && let Some(t1) = msg.at_ms
                        && let Some(rt) = &msg.reply_to
                    {
                        self.messages
                            .iter()
                            .find(|m| &m.id == rt)
                            .and_then(|call| call.at_ms)
                            .map(|t0| t1.saturating_sub(t0))
                    } else {
                        None
                    };

                    let mut meta_parts = Vec::new();
                    if let Some(at) = msg.at_ms {
                        meta_parts.push(format!(
                            "{} {}",
                            tr(cx, "Sent", "发送于"),
                            format_seq_ms(at)
                        ));
                    }
                    if let Some(lat) = latency_ms {
                        meta_parts.push(format!(
                            "{} {:.1}s",
                            tr(cx, "Took", "耗时"),
                            lat as f32 / 1000.0
                        ));
                    }
                    let meta_str = meta_parts.join(" · ");

                    let top_y = (row.top_y + 26.0).min((layout.total_h - 130.0).max(6.0));
                    let align_right = (row.from_col + row.to_col) >= n_cols;
                    let close_btn = pro_icon_btn(
                        "seq-mcard-close",
                        "Close message card",
                        ProIcon::Close,
                        22.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.set_selection(None, cx);
                        },
                    );
                    let p_hover = p.hover;
                    let p_ink = p.ink;
                    let detail_fold = detail.map(|det| {
                        let line_cnt = det.lines().count().max(1) as f32;
                        pro_fold(
                            pro_eid(&self.id, "seq-det-fold"),
                            is_pop_open,
                            14.0 + line_cnt * 16.0,
                            7.0,
                            move || {
                                div()
                                    .px(px(8.))
                                    .py(px(6.))
                                    .rounded(px(6.))
                                    .bg(p_hover)
                                    .font_family("Menlo")
                                    .text_size(px(10.5))
                                    .line_height(px(15.5))
                                    .text_color(p_ink)
                                    .child(det)
                                    .into_any_element()
                            },
                        )
                    });
                    let card_vtx = pro_view_transition(
                        pro_eid(&self.id, "seq-pop-vtx"),
                        format!("msg:{sel_mid}"),
                        div()
                            .id("seq-message-dialog")
                            .role(Role::Dialog)
                            .w(px(256.))
                            .p(px(10.))
                            .rounded(px(9.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow_md()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .truncate()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(route),
                                    )
                                    .child(close_btn),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(17.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(pro_mix_color(tint, p.strong, 0.25))
                                    .child(lbl_txt),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(5.))
                                            .bg(tint.opacity(0.13))
                                            .text_size(px(11.))
                                            .text_color(tint)
                                            .child(kind_chip),
                                    )
                                    .when_some(frame_chip, |d, fc| {
                                        d.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(5.))
                                                .bg(p.chip)
                                                .text_size(px(10.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.text)
                                                .child(fc),
                                        )
                                    })
                                    .when_some(status_chip, |d, (st_lbl, st_c)| {
                                        d.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(5.))
                                                .bg(st_c.opacity(0.14))
                                                .text_size(px(11.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(st_c)
                                                .child(st_lbl),
                                        )
                                    }),
                            )
                            .when_some(detail_fold, |d, df| d.child(df))
                            .when(!meta_str.is_empty(), |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .child(meta_str),
                                )
                            }),
                    );

                    pro_presence(
                        pro_eid(&self.id, "seq-pop-pres"),
                        is_pop_open,
                        0.16,
                        0.12,
                        move |amt| {
                            let wrap = div()
                                .absolute()
                                .top(px(top_y + 3. * (1. - amt)))
                                .opacity(amt);
                            if align_right {
                                wrap.right(px(8.)).child(card_vtx).into_any_element()
                            } else {
                                wrap.left(px(gutter + 8.))
                                    .child(card_vtx)
                                    .into_any_element()
                            }
                        },
                    )
                } else {
                    div().into_any_element()
                }
            }
            None => div().into_any_element(),
        };

        body_box = body_box.child(pop_card);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(760.))
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .when(!p.dark, |d| d.shadow_sm())
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(heads_row)
                    .child(body_box),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentSequence, SequenceSelection, compute_sequence_layout, default_seq_messages,
        default_seq_participants,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn sequence_layout_groups_par_and_retry_frames_and_pairs_activation_bars() {
        let parts = default_seq_participants();
        let msgs = default_seq_messages();
        let layout = compute_sequence_layout(&parts, &msgs, msgs.len());

        assert_eq!(layout.rows.len(), 9);
        assert_eq!(layout.frames.len(), 2);
        assert_eq!(layout.frames[0].label.as_ref(), "par");
        assert_eq!(layout.frames[1].label.as_ref(), "retry");

        // `Researcher` (`col == 2`) closed its activation bar when `m7` replied to `m3`!
        let research_bar = layout
            .activations
            .iter()
            .find(|a| a.col == 2)
            .expect("Researcher activation bar");
        assert!(!research_bar.open);

        // `Vercel` (`col == 5`) has an open activation bar from `m9` (`m8` failed so didn't open one)!
        let vercel_bar = layout
            .activations
            .iter()
            .find(|a| a.col == 5)
            .expect("Vercel activation bar");
        assert!(vercel_bar.open);
    }

    #[gpui::test]
    fn agent_sequence_opens_participant_and_message_cards(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let seq = cx.new(|cx| AgentSequence::new("test-seq", cx));

        seq.update(cx, |v, cx| {
            v.set_selection(Some(SequenceSelection::Participant("lead".into())), cx);
            assert_eq!(
                v.selection,
                Some(SequenceSelection::Participant("lead".into()))
            );

            v.set_selection(Some(SequenceSelection::Message("m5".into())), cx);
            assert_eq!(v.selection, Some(SequenceSelection::Message("m5".into())));

            v.replay(cx);
            assert!(v.replay_mode);
            assert_eq!(v.selection, None);
        });
    }
}
