//! Streamed AI answer card with source previews, version switching, and feedback.
//!
//! Renders grounded assistant responses with horizontal source cards, inline
//! `[1]` citation chips that preview their source on hover or focus, live
//! word-by-word streaming controls, version history (`1/2`), copy, thumbs
//! feedback with follow-up reason tags, and clickable related questions.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_eid, pro_empty_state, pro_fold,
        pro_presence, pro_shimmer_text, pro_skeleton_bar, pro_toast_banner, pro_view_transition,
        tr,
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_SPARK_MARK: &str = r#"<path d="M6.5 2.2c.2 0 .36.14.4.33.53 2.6 1.4 3.48 4 4.02a.4.4 0 0 1 0 .79c-2.6.53-3.47 1.4-4 4a.4.4 0 0 1-.8 0c-.53-2.6-1.4-3.47-4-4a.4.4 0 0 1 0-.79c2.6-.54 3.47-1.42 4-4.02a.4.4 0 0 1 .4-.33Z" fill="currentColor"></path><path d="M12 9.6c.12 0 .22.08.25.2.28 1.3.72 1.75 2.03 2.03a.25.25 0 0 1 0 .49c-1.31.28-1.75.72-2.03 2.03a.25.25 0 0 1-.5 0c-.28-1.31-.72-1.75-2.03-2.03a.25.25 0 0 1 0-.49c1.31-.28 1.75-.73 2.03-2.03.03-.12.13-.2.25-.2Z" fill="currentColor" opacity="0.72"></path>"#;

const SVG_THUMB_UP: &str = r#"<path d="M7 11V19C7 19.5523 6.55228 20 6 20H4C3.44772 20 3 19.5523 3 19V12C3 11.4477 3.44772 11 4 11H7ZM7 11L10.6 4.7C11.1 3.8 12.3 3.6 13.1 4.2C13.7 4.7 13.9 5.5 13.7 6.3L12.8 10H18.4C19.8 10 20.8 11.3 20.4 12.7L18.9 18.2C18.6 19.3 17.6 20 16.5 20H9C7.9 20 7 19.1 7 18" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_THUMB_DOWN: &str = r#"<path d="M7 13V5C7 4.44772 6.55228 4 6 4H4C3.44772 4 3 4.44772 3 5V12C3 12.5523 3.44772 13 4 13H7ZM7 13L10.6 19.3C11.1 20.2 12.3 20.4 13.1 19.8C13.7 19.3 13.9 18.5 13.7 17.7L12.8 14H18.4C19.8 14 20.8 12.7 20.4 11.3L18.9 5.8C18.6 4.7 17.6 4 16.5 4H9C7.9 4 7 4.9 7 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Helpful or unhelpful rating recorded on [`AiAnswer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiAnswerFeedback {
    /// Positive rating.
    Up,
    /// Negative rating, unlocking follow-up reason tags.
    Down,
}

/// Lifecycle phase displayed in the [`AiAnswer`] status header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiAnswerPhase {
    /// Reading retrieved sources before writing begins.
    Reading,
    /// Streaming answer tokens word by word.
    Writing,
    /// Finished streaming all paragraphs and citations.
    #[default]
    Done,
    /// Generation halted early by the user.
    Stopped,
}

/// Grounding source cited by an [`AiAnswer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnswerSource {
    /// Short provider or publication name shown on cards and citation chips.
    pub name: SharedString,
    /// Document or thread title.
    pub title: SharedString,
    /// Optional destination URL.
    pub href: Option<SharedString>,
    /// Optional brand key passed to `pro_app_tile` (defaults to `name`).
    pub icon: Option<SharedString>,
    /// Excerpt shown inside the citation preview popover.
    pub snippet: Option<SharedString>,
}

impl AnswerSource {
    /// Create a source entry with `name` and `title`.
    pub fn new(name: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            href: None,
            icon: None,
            snippet: None,
        }
    }

    /// Attach a canonical link URL.
    pub fn with_href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Override the brand icon key used for the source tile.
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Attach a short excerpt displayed when previewing citations.
    pub fn with_snippet(mut self, snippet: impl Into<SharedString>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    fn brand_key(&self) -> &str {
        self.icon.as_deref().unwrap_or(self.name.as_ref())
    }
}

/// Typed interaction events emitted by [`AiAnswer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiAnswerEvent {
    /// User clicked a related follow-up question.
    FollowUp(SharedString),
    /// User requested a fresh regeneration (`1`-based version index).
    Regenerate { version: usize },
    /// User switched between generated answer versions (`1`-based index).
    SwitchVersion { version: usize },
    /// User stopped an active answer stream.
    Stop,
    /// User replayed the streaming animation.
    Replay,
    /// User rated the answer or picked a thumbs-down reason tag.
    Feedback {
        value: AiAnswerFeedback,
        reason: Option<SharedString>,
    },
    /// User copied the plain-text answer and numbered source bibliography.
    Copy(SharedString),
    /// User activated a source card or preview link (`1`-based index).
    OpenSource { index: usize, source: AnswerSource },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnswerToken {
    Word { text: String, bold: bool },
    Citation(Vec<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnswerBlock {
    Paragraph(Vec<AnswerToken>),
    Bullets(Vec<Vec<AnswerToken>>),
}

pub(crate) fn parse_inline_tokens(line: &str, max_source: usize) -> Vec<AnswerToken> {
    let mut out = Vec::new();
    let mut bold = false;
    let mut rest = line.trim();

    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("**") {
            bold = !bold;
            rest = after;
            continue;
        }
        if rest.starts_with('[')
            && let Some(end_idx) = rest.find(']')
        {
            let inside = &rest[1..end_idx];
            let parsed_nums: Option<Vec<usize>> = inside
                .split(',')
                .map(|part| part.trim().parse::<usize>().ok())
                .collect();
            if let Some(nums) = parsed_nums {
                let valid: Vec<usize> = nums
                    .into_iter()
                    .filter(|&n| n >= 1 && (max_source == 0 || n <= max_source))
                    .collect();
                if !valid.is_empty() {
                    out.push(AnswerToken::Citation(valid));
                }
                rest = rest[end_idx + 1..].trim_start();
                continue;
            }
        }

        let mut cut = rest.len();
        if let Some(b) = rest.find("**") {
            cut = cut.min(b);
        }
        if let Some(c) = rest.find('[')
            && c > 0
        {
            cut = cut.min(c);
        }
        let chunk = if cut == 0 {
            let ch_len = rest.chars().next().map_or(1, char::len_utf8);
            let piece = &rest[..ch_len];
            rest = &rest[ch_len..];
            piece
        } else {
            let piece = &rest[..cut];
            rest = &rest[cut..];
            piece
        };

        for word in chunk.split_whitespace() {
            if !word.is_empty() {
                out.push(AnswerToken::Word {
                    text: word.to_owned(),
                    bold,
                });
            }
        }
    }
    out
}

pub(crate) fn parse_answer_blocks(raw: &str, max_source: usize) -> Vec<AnswerBlock> {
    let mut blocks = Vec::new();
    let mut para_lines: Vec<&str> = Vec::new();
    let mut bullet_items: Vec<Vec<AnswerToken>> = Vec::new();

    let flush_para = |para: &mut Vec<&str>, out: &mut Vec<AnswerBlock>| {
        if !para.is_empty() {
            let joined = para.join(" ");
            let toks = parse_inline_tokens(&joined, max_source);
            if !toks.is_empty() {
                out.push(AnswerBlock::Paragraph(toks));
            }
            para.clear();
        }
    };

    let flush_bullets = |bullets: &mut Vec<Vec<AnswerToken>>, out: &mut Vec<AnswerBlock>| {
        if !bullets.is_empty() {
            out.push(AnswerBlock::Bullets(std::mem::take(bullets)));
        }
    };

    for raw_line in raw.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            flush_para(&mut para_lines, &mut blocks);
            flush_bullets(&mut bullet_items, &mut blocks);
            continue;
        }
        if let Some(item_str) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("• "))
        {
            flush_para(&mut para_lines, &mut blocks);
            let toks = parse_inline_tokens(item_str, max_source);
            if !toks.is_empty() {
                bullet_items.push(toks);
            }
        } else {
            flush_bullets(&mut bullet_items, &mut blocks);
            para_lines.push(trimmed);
        }
    }
    flush_para(&mut para_lines, &mut blocks);
    flush_bullets(&mut bullet_items, &mut blocks);
    blocks
}

pub(crate) fn count_block_tokens(blocks: &[AnswerBlock]) -> usize {
    blocks
        .iter()
        .map(|b| match b {
            AnswerBlock::Paragraph(t) => t.len(),
            AnswerBlock::Bullets(items) => items.iter().map(Vec::len).sum(),
        })
        .sum()
}

/// Format an answer and its numbered sources into clean plain text for clipboard copy.
pub fn format_plain_answer(raw: &str, sources: &[AnswerSource]) -> String {
    let blocks = parse_answer_blocks(raw, sources.len());
    let mut lines = Vec::new();
    for block in blocks {
        match block {
            AnswerBlock::Paragraph(toks) => {
                let text = join_plain_tokens(&toks);
                if !text.is_empty() {
                    lines.push(text);
                }
            }
            AnswerBlock::Bullets(items) => {
                let bullet_block: Vec<String> = items
                    .iter()
                    .map(|t| format!("- {}", join_plain_tokens(t)))
                    .collect();
                if !bullet_block.is_empty() {
                    lines.push(bullet_block.join("\n"));
                }
            }
        }
    }
    let mut out = lines.join("\n\n");
    if !sources.is_empty() {
        out.push_str("\n\nSources:\n");
        for (idx, src) in sources.iter().enumerate() {
            match &src.href {
                Some(url) => {
                    out.push_str(&format!(
                        "[{}] {} — {} ({})\n",
                        idx + 1,
                        src.name,
                        src.title,
                        url
                    ));
                }
                None => {
                    out.push_str(&format!("[{}] {} — {}\n", idx + 1, src.name, src.title));
                }
            }
        }
    }
    out.trim_end().to_owned()
}

fn join_plain_tokens(tokens: &[AnswerToken]) -> String {
    let mut out = String::new();
    for tok in tokens {
        if let AnswerToken::Word { text, .. } = tok {
            let punct_only = text
                .chars()
                .all(|c| matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')'));
            if !out.is_empty() && !punct_only {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
    out
}

fn default_sources() -> Vec<AnswerSource> {
    vec![
        AnswerSource::new("Notion", "Q4 Offsite plan — Lisbon shortlist & €18k budget")
            .with_icon("notion")
            .with_href("https://notion.so/kense/q4-lisbon-offsite")
            .with_snippet(
                "Shortlisted LX Factory Loft (€4,400 for 3 days, 28 seats, fiber) and Alfama River House (€5,100) within the €18k team cap.",
            ),
        AnswerSource::new("Slack", "#team-offsite — October date availability poll")
            .with_icon("slack")
            .with_href("https://slack.com/archives/C04OFFSITE")
            .with_snippet(
                "21 of 22 engineers confirmed October 14–17; morning workshop blocks from 09:30–13:00 got unanimous votes.",
            ),
        AnswerSource::new("Google Drive", "Lisbon venue quotes & catering matrix.xlsx")
            .with_icon("google drive")
            .with_href("https://drive.google.com/file/d/lisbon-quotes")
            .with_snippet(
                "LX Factory includes two breakout rooms, A/V displays, and €38/person daily lunch; Sintra shuttle adds €420 round-trip.",
            ),
        AnswerSource::new("Linear", "OPS-142 — Offsite travel & hotel room block")
            .with_icon("linear")
            .with_href("https://linear.app/kense/issue/OPS-142")
            .with_snippet(
                "24-room group hold at Locke de Santa Joana expires Friday at 17:00 WET; 12-minute tram ride to Alcântara.",
            ),
        AnswerSource::new("GitHub", "kense-handbook/offsite-playbook.md")
            .with_icon("github")
            .with_href("https://github.com/kense-labs/handbook/blob/main/offsite.md")
            .with_snippet(
                "Standard KUI engineering offsites reserve Day 2 afternoon for live ship demos and one half-day coastal walk.",
            ),
    ]
}

fn default_versions() -> Vec<SharedString> {
    vec![
        concat!(
            "Book **LX Factory Loft** in Alcântara as the primary workshop hub for **October 14–17** [1,2]. ",
            "It fits all 22 teammates with dedicated fiber, dual 4K displays, and two quiet breakout rooms while staying **€1,700 under** the venue budget [1,3].\n\n",
            "- **Stay:** Confirm the 24-room hold at **Locke de Santa Joana** before Friday 17:00 WET (12 min by tram) [4].\n",
            "- **Schedule:** Run architecture sessions **09:30–13:00**, keep Day 2 afternoon for live KUI demos, and book the **Sintra coastal shuttle** (€420) on Thursday [2,3,5]."
        )
        .into(),
        concat!(
            "Choose **Alfama River House** if you prioritize walkable dinners and a single private townhouse for **October 14–17** [1,2]. ",
            "At **€5,100** for three days it runs €700 higher than LX Factory, but bundles terrace catering and keeps every session under one roof [1,3].\n\n",
            "- **Trade-off:** One main salon plus a courtyard tent—best for whole-team plenaries rather than parallel tracks [3].\n",
            "- **Action:** Lock the **Locke de Santa Joana** room block today and reserve Day 2 afternoon for live ship demos [4,5]."
        )
        .into(),
    ]
}

fn default_follow_ups() -> Vec<(SharedString, SharedString)> {
    vec![
        (
            "Compare LX Factory Loft vs Alfama River House costs".into(),
            concat!(
                "**LX Factory Loft** totals **€6,910** with lunches and the Sintra shuttle (€4,400 venue + €2,090 catering + €420 transit), leaving **€11,090** for flights and rooms [1,3].\n\n",
                "- **Alfama River House** comes to **€7,480** all-in with terrace dining, saving transit time but trimming hotel headroom by €570 [1,3,4]."
            )
            .into(),
        ),
        (
            "Who still needs to confirm October 14–17 travel dates?".into(),
            concat!(
                "**21 of 22** teammates confirmed **October 14–17** in `#team-offsite`; only one Berlin flight arrival time is waiting on passport renewal [2].\n\n",
                "- **Next step:** Confirm the **24-room hold** on OPS-142 before Friday 17:00 WET so late flight tweaks keep the group rate [2,4]."
            )
            .into(),
        ),
        (
            "Draft the agenda for Day 2 ship demos and Sintra retreat".into(),
            concat!(
                "Follow the handbook rhythm: **09:30–12:30** deep-work sprints in two breakout rooms, **14:00–16:30** live 8-minute ship demos, then Thursday's **Sintra coastal walk** [3,5].\n\n",
                "- **Logistics:** Pre-book the €420 shuttle for 13:30 departure right after lunch at LX Factory [3,5]."
            )
            .into(),
        ),
    ]
}

fn default_feedback_reasons() -> Vec<SharedString> {
    vec![
        "Not accurate".into(),
        "Wrong source".into(),
        "Too long".into(),
        "Out of date".into(),
    ]
}

/// Streamed AI answer component with source cards, inline citation previews, version history, and feedback.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::ai_answer::{AiAnswer, AiAnswerEvent};
///
/// fn mount(cx: &mut gpui::App) {
///     let view = cx.new(|cx| AiAnswer::new("offsite-answer", cx).with_framed(true));
///     cx.subscribe(&view, |_, ev: &AiAnswerEvent, _| {
///         let _ = ev;
///     })
///     .detach();
/// }
/// ```
pub struct AiAnswer {
    id: ElementId,
    focus: FocusHandle,
    base_question: SharedString,
    active_question: SharedString,
    sources: Vec<AnswerSource>,
    versions: Vec<SharedString>,
    active_version: usize,
    follow_up_override: Option<SharedString>,
    follow_ups: Vec<(SharedString, SharedString)>,
    feedback_reasons: Vec<SharedString>,
    phase: AiAnswerPhase,
    animate: bool,
    speed: f32,
    framed: bool,
    stream_started: Option<Instant>,
    stopped_tokens: Option<usize>,
    expand_all_sources: bool,
    hovered_source: Option<usize>,
    active_cite_chip: Option<(SharedString, Vec<usize>)>,
    copied_at: Option<Instant>,
    feedback: Option<AiAnswerFeedback>,
    picked_reason: Option<SharedString>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl EventEmitter<AiAnswerEvent> for AiAnswer {}

impl AiAnswer {
    /// Create a grounded [`AiAnswer`] preloaded with sources, two versions, citations, and follow-ups.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let q: SharedString = "Where should we hold the team offsite in Lisbon?".into();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            base_question: q.clone(),
            active_question: q,
            sources: default_sources(),
            versions: default_versions(),
            active_version: 0,
            follow_up_override: None,
            follow_ups: default_follow_ups(),
            feedback_reasons: default_feedback_reasons(),
            phase: AiAnswerPhase::Done,
            animate: true,
            speed: 38.0,
            framed: true,
            stream_started: None,
            stopped_tokens: None,
            expand_all_sources: false,
            hovered_source: None,
            active_cite_chip: None,
            copied_at: None,
            feedback: None,
            picked_reason: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(640.)),
            toast: None,
        }
    }

    /// Set the question heading displayed above the source cards.
    pub fn with_question(mut self, question: impl Into<SharedString>) -> Self {
        let q = question.into();
        self.base_question = q.clone();
        self.active_question = q;
        self
    }

    /// Supply custom [`AnswerSource`] cards (numbered `1..=N` in slice order).
    pub fn with_sources(mut self, sources: Vec<AnswerSource>) -> Self {
        self.sources = sources;
        self
    }

    /// Set a single markdown-lite answer body (`**bold**`, `- ` lists, and `[1]` citations).
    pub fn with_answer(mut self, answer: impl Into<SharedString>) -> Self {
        self.versions = vec![answer.into()];
        self.active_version = 0;
        self.follow_up_override = None;
        self
    }

    /// Supply multiple generated answer versions enabled by the `1/N` switcher and `Regenerate`.
    pub fn with_versions(mut self, versions: Vec<SharedString>) -> Self {
        if !versions.is_empty() {
            self.versions = versions;
            self.active_version = 0;
        }
        self
    }

    /// Start immediately in the live `Reading` -> `Writing` streaming sequence when `true`.
    pub fn with_streaming(mut self, streaming: bool) -> Self {
        if streaming {
            self.phase = AiAnswerPhase::Reading;
            self.stream_started = Some(Instant::now());
            self.stopped_tokens = None;
        } else {
            self.phase = AiAnswerPhase::Done;
            self.stream_started = None;
        }
        self
    }

    /// Control whether `replay` and `regenerate` animate word-by-word (`true` by default).
    pub fn with_animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }

    /// Set the streaming reveal speed in words per second (`38.0` by default).
    pub fn with_speed(mut self, words_per_second: f32) -> Self {
        self.speed = words_per_second.clamp(8.0, 160.0);
        self
    }

    /// Provide related follow-up questions shown underneath the completed answer.
    pub fn with_follow_ups(mut self, questions: Vec<SharedString>) -> Self {
        self.follow_ups = questions
            .into_iter()
            .map(|q| {
                let fallback = SharedString::from(format!(
                    "Based on the **{}** workspace sources, here is the focused breakdown for **{}** [1,2].",
                    self.sources.len().max(1),
                    q
                ));
                (q, fallback)
            })
            .collect();
        self
    }

    /// Override the tag pills offered when the user clicks thumbs down.
    pub fn with_feedback_reasons(mut self, reasons: Vec<SharedString>) -> Self {
        self.feedback_reasons = reasons;
        self
    }

    /// Wrap the answer in a rounded elevated card (`true` by default) or render chromeless (`false`).
    pub fn with_framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state presentation.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return whether the answer is currently streaming (`Reading` or `Writing`).
    pub fn is_streaming(&self) -> bool {
        matches!(self.phase, AiAnswerPhase::Reading | AiAnswerPhase::Writing)
    }

    /// Return the `1`-based active version index.
    pub fn active_version(&self) -> usize {
        self.active_version + 1
    }

    /// Return the total number of available answer versions.
    pub fn version_count(&self) -> usize {
        self.versions.len().max(1)
    }

    /// Return the plain-text clipboard representation of the active answer plus sources.
    pub fn plain_copy_text(&self) -> String {
        format_plain_answer(self.current_raw_answer(), &self.sources)
    }

    fn current_raw_answer(&self) -> &str {
        if let Some(ov) = &self.follow_up_override {
            return ov.as_ref();
        }
        self.versions
            .get(self.active_version)
            .map_or("", SharedString::as_ref)
    }

    /// Replay the `Reading` -> `Writing` streaming reveal from the beginning.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.stopped_tokens = None;
        self.active_cite_chip = None;
        if !self.animate || disabled_motion(cx) {
            self.phase = AiAnswerPhase::Done;
            self.stream_started = None;
        } else {
            self.phase = AiAnswerPhase::Reading;
            self.stream_started = Some(Instant::now());
        }
        cx.emit(AiAnswerEvent::Replay);
        cx.notify();
    }

    /// Stop an in-flight stream at the currently revealed word boundary.
    pub fn stop(&mut self, visible_tokens: usize, cx: &mut Context<Self>) {
        if self.is_streaming() {
            self.phase = AiAnswerPhase::Stopped;
            self.stream_started = None;
            self.stopped_tokens = Some(visible_tokens.max(1));
            cx.emit(AiAnswerEvent::Stop);
            cx.notify();
        }
    }

    /// Advance to the next answer version (or create an alternative pass) and stream it in.
    pub fn regenerate(&mut self, cx: &mut Context<Self>) {
        self.follow_up_override = None;
        self.active_question = self.base_question.clone();
        if self.versions.len() < 2 {
            let alt = format!(
                "{}\n\n- **Updated pass:** Verified against all {} connected sources [1].",
                self.current_raw_answer(),
                self.sources.len().max(1)
            );
            self.versions.push(alt.into());
        }
        self.active_version = (self.active_version + 1) % self.versions.len();
        self.feedback = None;
        self.picked_reason = None;
        self.stopped_tokens = None;
        self.active_cite_chip = None;
        if !self.animate || disabled_motion(cx) {
            self.phase = AiAnswerPhase::Done;
            self.stream_started = None;
        } else {
            self.phase = AiAnswerPhase::Reading;
            self.stream_started = Some(Instant::now());
        }
        cx.emit(AiAnswerEvent::Regenerate {
            version: self.active_version + 1,
        });
        cx.notify();
    }

    /// Switch directly to a `0`-based version slot.
    pub fn switch_version(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.versions.len()
            && (idx != self.active_version || self.follow_up_override.is_some())
        {
            self.follow_up_override = None;
            self.active_question = self.base_question.clone();
            self.active_version = idx;
            self.phase = AiAnswerPhase::Done;
            self.stream_started = None;
            self.stopped_tokens = None;
            self.active_cite_chip = None;
            cx.emit(AiAnswerEvent::SwitchVersion { version: idx + 1 });
            cx.notify();
        }
    }

    /// Copy the active plain-text answer and source list to the system clipboard.
    pub fn copy_answer(&mut self, cx: &mut Context<Self>) {
        let payload = self.plain_copy_text();
        clipboard::write(&payload, cx).detach();
        self.copied_at = Some(Instant::now());
        self.toast = Some((
            tr(cx, "Copied answer with sources", "已复制回答与引用来源").into(),
            Instant::now(),
        ));
        cx.emit(AiAnswerEvent::Copy(payload.into()));
        cx.notify();
    }

    /// Record thumbs-up or thumbs-down feedback (toggling off if already selected).
    pub fn set_feedback(&mut self, value: AiAnswerFeedback, cx: &mut Context<Self>) {
        if self.feedback == Some(value) {
            self.feedback = None;
            self.picked_reason = None;
        } else {
            self.feedback = Some(value);
            if value == AiAnswerFeedback::Up {
                self.picked_reason = None;
            }
            cx.emit(AiAnswerEvent::Feedback {
                value,
                reason: None,
            });
        }
        cx.notify();
    }

    /// Select a follow-up reason pill after thumbs-down feedback.
    pub fn pick_feedback_reason(&mut self, reason: SharedString, cx: &mut Context<Self>) {
        self.feedback = Some(AiAnswerFeedback::Down);
        self.picked_reason = Some(reason.clone());
        cx.emit(AiAnswerEvent::Feedback {
            value: AiAnswerFeedback::Down,
            reason: Some(reason),
        });
        cx.notify();
    }

    fn ask_follow_up(
        &mut self,
        question: SharedString,
        reply: SharedString,
        cx: &mut Context<Self>,
    ) {
        self.active_question = question.clone();
        self.follow_up_override = Some(reply);
        self.feedback = None;
        self.picked_reason = None;
        self.stopped_tokens = None;
        self.active_cite_chip = None;
        if !self.animate || disabled_motion(cx) {
            self.phase = AiAnswerPhase::Done;
            self.stream_started = None;
        } else {
            self.phase = AiAnswerPhase::Reading;
            self.stream_started = Some(Instant::now());
        }
        cx.emit(AiAnswerEvent::FollowUp(question));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" && self.active_cite_chip.is_some() {
            self.active_cite_chip = None;
            cx.notify();
        }
    }

    fn render_source_card(
        &self,
        idx_1based: usize,
        src: &AnswerSource,
        lit: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let src_clone = src.clone();
        let card_bg = if lit {
            if p.dark {
                rgb(0x252529).into()
            } else {
                rgb(0xf4f7fc).into()
            }
        } else if self.framed {
            p.subtle
        } else {
            p.card
        };
        let bdr = if lit { p.blue.opacity(0.45) } else { p.line };

        div()
            .id(pro_eid(&self.id, format!("src-card-{idx_1based}")))
            .role(Role::Button)
            .aria_label(format!("{}: {} — {}", idx_1based, src.name, src.title))
            .tab_index(0)
            .flex_1()
            .min_w_0()
            .p(px(9.))
            .rounded(px(8.))
            .bg(card_bg)
            .border_1()
            .border_color(bdr)
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap(px(5.))
            .focus_visible(move |s| s.border_color(p.focus))
            .hover(move |s| s.bg(if p.dark { rgb(0x232327) } else { rgb(0xf6f6f7) }))
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                this.hovered_source = if *inside { Some(idx_1based) } else { None };
                cx.notify();
            }))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toast = Some((
                    SharedString::from(format!(
                        "{} · {}",
                        src_clone.name,
                        src_clone
                            .href
                            .as_deref()
                            .unwrap_or(src_clone.title.as_ref())
                    )),
                    Instant::now(),
                ));
                cx.emit(AiAnswerEvent::OpenSource {
                    index: idx_1based,
                    source: src_clone.clone(),
                });
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_w_0()
                    .child(pro_app_tile(src.brand_key(), 15., cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(11.5))
                            .text_color(p.text)
                            .child(src.name.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if lit { p.blue } else { p.faint })
                            .child(format!("{idx_1based}")),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .line_height(px(17.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .line_clamp(2)
                    .child(src.title.clone()),
            )
    }

    fn render_token_stream(
        &self,
        tokens: &[AnswerToken],
        start_offset: usize,
        visible_limit: usize,
        lit_sources: &[usize],
        key_prefix: &str,
        cx: &mut Context<Self>,
    ) -> (Div, usize, bool) {
        let p = ProPalette::get(cx);
        let mut wrap = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(3.5))
            .gap_y(px(2.));
        let mut cursor = start_offset;
        let mut truncated = false;

        for (i, tok) in tokens.iter().enumerate() {
            if cursor >= visible_limit {
                truncated = true;
                break;
            }
            cursor += 1;
            match tok {
                AnswerToken::Word { text, bold } => {
                    wrap = wrap.child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .font_weight(if *bold {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if *bold { p.strong } else { p.ink })
                            .child(text.clone()),
                    );
                }
                AnswerToken::Citation(nums) => {
                    let chip_key = SharedString::from(format!("{key_prefix}-c-{i}"));
                    let is_chip_active = self
                        .active_cite_chip
                        .as_ref()
                        .is_some_and(|(k, _)| k == &chip_key);
                    let is_lit = is_chip_active || nums.iter().any(|n| lit_sources.contains(n));
                    let first_num = nums[0];
                    let lead_name = self
                        .sources
                        .get(first_num.saturating_sub(1))
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| SharedString::from(format!("{first_num}")));
                    let badge_text = if nums.len() > 1 {
                        format!("{} +{}", lead_name, nums.len() - 1)
                    } else {
                        format!("{} [{}]", lead_name, first_num)
                    };
                    let nums_hover = nums.clone();
                    let nums_click = nums.clone();
                    let key_hover = chip_key.clone();
                    let key_click = chip_key.clone();

                    wrap = wrap.child(
                        div()
                            .id(pro_eid(&self.id, chip_key))
                            .role(Role::Button)
                            .aria_label(format!("Citation {badge_text}"))
                            .tab_index(0)
                            .h(px(19.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(if is_lit { p.accent_soft } else { p.chip })
                            .border_1()
                            .border_color(if is_lit { p.blue.opacity(0.40) } else { p.line })
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .cursor_pointer()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if is_lit { p.blue } else { p.text })
                            .focus_visible(move |s| s.border_color(p.focus))
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.active_cite_chip =
                                        Some((key_hover.clone(), nums_hover.clone()));
                                } else if this
                                    .active_cite_chip
                                    .as_ref()
                                    .is_some_and(|(k, _)| k == &key_hover)
                                {
                                    this.active_cite_chip = None;
                                }
                                cx.notify();
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let toggle_off = this
                                    .active_cite_chip
                                    .as_ref()
                                    .is_some_and(|(k, _)| k == &key_click);
                                if toggle_off {
                                    this.active_cite_chip = None;
                                } else {
                                    this.active_cite_chip =
                                        Some((key_click.clone(), nums_click.clone()));
                                }
                                cx.notify();
                            }))
                            .child(badge_text),
                    );
                }
            }
        }

        (wrap, cursor, truncated)
    }
}

impl Render for AiAnswer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let measure_cell = self.measured_width.clone();
        let compact = width < 480.;

        if let Some(t) = self.copied_at
            && t.elapsed().as_secs_f32() < 2.1
            && !reduced
        {
            w.request_animation_frame();
        }

        let blocks = parse_answer_blocks(self.current_raw_answer(), self.sources.len());
        let total_tokens = count_block_tokens(&blocks);

        // Advance live streaming state machine when active.
        let mut visible_tokens = total_tokens;
        let mut pulse_alpha = 1.0f32;
        if let Some(started) = self.stream_started {
            if reduced || !self.animate {
                self.phase = AiAnswerPhase::Done;
                self.stream_started = None;
            } else {
                let elapsed = started.elapsed().as_secs_f32();
                let read_dur = 0.55;
                if elapsed < read_dur {
                    self.phase = AiAnswerPhase::Reading;
                    visible_tokens = 0;
                    w.request_animation_frame();
                } else {
                    let write_elapsed = elapsed - read_dur;
                    let revealed = ((write_elapsed * self.speed).floor() as usize).max(1);
                    if revealed >= total_tokens {
                        self.phase = AiAnswerPhase::Done;
                        self.stream_started = None;
                        visible_tokens = total_tokens;
                    } else {
                        self.phase = AiAnswerPhase::Writing;
                        visible_tokens = revealed;
                        pulse_alpha =
                            0.35 + 0.65 * ((elapsed * std::f32::consts::TAU).sin() * 0.5 + 0.5);
                        w.request_animation_frame();
                    }
                }
            }
        } else if self.phase == AiAnswerPhase::Stopped
            && let Some(limit) = self.stopped_tokens
        {
            visible_tokens = limit.min(total_tokens);
        }

        if self.loading {
            let phase = 0.35;
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(640.))
                        .p(px(20.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(320., 18., 5., phase, cx))
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(170., 66., 8., phase, cx))
                                .child(pro_skeleton_bar(170., 66., 8., phase, cx))
                                .child(pro_skeleton_bar(170., 66., 8., phase, cx)),
                        )
                        .child(pro_skeleton_bar(520., 14., 4., phase, cx))
                        .child(pro_skeleton_bar(460., 14., 4., phase, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(640.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Sparkles,
                            tr(cx, "No answer generated yet", "尚未生成回答"),
                            tr(
                                cx,
                                "Ask a grounded question to search connected workspace sources.",
                                "输入问题以检索已连接的工作区知识源并生成带引用的回答。",
                            ),
                            Some((
                                pro_eid(&self.id, "empty-generate"),
                                tr(cx, "Generate sample answer", "生成示例回答").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    this.replay(cx);
                                }),
                            )),
                            cx,
                        )),
                );
        }

        // Determine two-way lit source indices (1-based).
        let mut lit_sources: Vec<usize> = Vec::new();
        if let Some(hs) = self.hovered_source {
            lit_sources.push(hs);
        }
        if let Some((_, ref cited)) = self.active_cite_chip {
            for &n in cited {
                if !lit_sources.contains(&n) {
                    lit_sources.push(n);
                }
            }
        }

        let max_cards = if self.expand_all_sources {
            self.sources.len()
        } else if compact {
            2
        } else {
            3
        };
        let shown_count = self.sources.len().min(max_cards);
        let hidden_count = self.sources.len().saturating_sub(shown_count);
        let overflow_lit = hidden_count > 0 && lit_sources.iter().any(|&n| n > shown_count);

        let mut sources_grid = div().w_full().flex().flex_wrap().gap(px(8.));
        if self.expand_all_sources {
            for (i, src) in self.sources.iter().enumerate() {
                let n = i + 1;
                let lit = lit_sources.contains(&n);
                sources_grid = sources_grid.child(
                    div()
                        .w(px(if compact { 160. } else { 182. }))
                        .flex()
                        .child(self.render_source_card(n, src, lit, cx)),
                );
            }
        } else {
            let mut row = div().w_full().flex().items_stretch().gap(px(8.));
            for (i, src) in self.sources.iter().take(shown_count).enumerate() {
                let n = i + 1;
                let lit = lit_sources.contains(&n);
                row = row.child(self.render_source_card(n, src, lit, cx));
            }
            if hidden_count > 0 {
                row = row.child(
                    div()
                        .id(pro_eid(&self.id, "src-more-tile"))
                        .role(Role::Button)
                        .aria_label(format!("+{hidden_count} more sources"))
                        .tab_index(0)
                        .w(px(68.))
                        .flex_none()
                        .p(px(9.))
                        .rounded(px(8.))
                        .bg(if overflow_lit {
                            p.accent_soft
                        } else if self.framed {
                            p.subtle
                        } else {
                            p.card
                        })
                        .border_1()
                        .border_color(if overflow_lit {
                            p.blue.opacity(0.45)
                        } else {
                            p.line
                        })
                        .cursor_pointer()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .focus_visible(move |s| s.border_color(p.focus))
                        .hover(move |s| s.bg(p.hover))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.expand_all_sources = true;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if overflow_lit { p.blue } else { p.strong })
                                .child(format!("+{hidden_count}")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .whitespace_nowrap()
                                .text_color(p.text)
                                .child(tr(cx, "Sources", "来源")),
                        ),
                );
            }
            sources_grid = sources_grid.child(row);
        }

        // Phase label row with twin-sparkle mark and shimmer text.
        let status_label = match self.phase {
            AiAnswerPhase::Reading => format!(
                "{} {} {}",
                tr(cx, "Reading", "正在阅读"),
                self.sources.len(),
                tr(cx, "sources", "个来源")
            ),
            AiAnswerPhase::Writing => tr(cx, "Writing", "正在撰写").to_owned(),
            AiAnswerPhase::Stopped => tr(cx, "Stopped", "已停止生成").to_owned(),
            AiAnswerPhase::Done => {
                if self.follow_up_override.is_some() {
                    tr(cx, "Follow-up answer", "追问回答").to_owned()
                } else {
                    format!(
                        "{} · {} {}",
                        tr(cx, "Answered", "已完成回答"),
                        self.sources.len(),
                        tr(cx, "sources", "个来源")
                    )
                }
            }
        };

        let phase_row = div()
            .mt(px(14.))
            .mb(px(6.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_svg("0 0 16 16", SVG_SPARK_MARK, 15., p.blue))
                    .child(pro_shimmer_text(
                        pro_eid(&self.id, "phase-shimmer"),
                        status_label,
                        p.text,
                        p.strong,
                        12.,
                        FontWeight::MEDIUM,
                        self.is_streaming(),
                    )),
            )
            .when(self.follow_up_override.is_some(), |d| {
                d.child(pro_btn(
                    pro_eid(&self.id, "back-main-q"),
                    tr(cx, "Back to main answer", "返回原问题"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.switch_version(this.active_version, cx);
                    },
                ))
            });

        // Citation preview popover card when hovering/focusing any `[n]` chip.
        let preview_nums = self
            .active_cite_chip
            .as_ref()
            .map(|(_, nums)| nums.clone())
            .unwrap_or_default();
        let preview_open = !preview_nums.is_empty();
        let preview_sources: Vec<(usize, AnswerSource, AnyElement)> = preview_nums
            .iter()
            .filter_map(|&n| {
                self.sources.get(n.saturating_sub(1)).cloned().map(|s| {
                    let tile = pro_app_tile(s.brand_key(), 14., cx);
                    (n, s, tile)
                })
            })
            .collect();
        let cite_popover = pro_presence(
            pro_eid(&self.id, "cite-popover"),
            preview_open,
            0.16,
            0.12,
            move |amt| {
                let mut pop = div()
                    .mb(px(8.))
                    .p(px(8.))
                    .rounded(px(9.))
                    .bg(if p.dark { rgb(0x222226) } else { rgb(0xffffff) })
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .opacity(amt)
                    .flex()
                    .flex_col()
                    .gap(px(6.));
                for (n, s, tile) in preview_sources {
                    pop = pop.child(
                        div()
                            .p(px(6.))
                            .rounded(px(6.))
                            .bg(p.subtle)
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(tile)
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.text)
                                            .child(s.name),
                                    )
                                    .child(
                                        div()
                                            .px(px(4.))
                                            .rounded(px(4.))
                                            .bg(p.chip)
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.blue)
                                            .child(format!("[{n}]")),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(s.title),
                            )
                            .when_some(s.snippet, |d, snip| {
                                d.child(
                                    div()
                                        .text_size(px(11.5))
                                        .line_height(px(16.5))
                                        .text_color(p.text)
                                        .child(snip),
                                )
                            }),
                    );
                }
                pop.into_any_element()
            },
        );

        // Render streamed answer blocks + pulsing caret while writing.
        let mut answer_col = div().w_full().flex().flex_col().gap(px(8.));
        let mut token_cursor = 0usize;
        for (bi, block) in blocks.iter().enumerate() {
            if token_cursor >= visible_tokens {
                break;
            }
            match block {
                AnswerBlock::Paragraph(toks) => {
                    let (mut row, next_c, _) = self.render_token_stream(
                        toks,
                        token_cursor,
                        visible_tokens,
                        &lit_sources,
                        &format!("b{bi}"),
                        cx,
                    );
                    if self.phase == AiAnswerPhase::Writing && next_c >= visible_tokens {
                        row = row.child(
                            div()
                                .ml(px(3.))
                                .size(px(6.))
                                .rounded(px(1.5))
                                .bg(p.blue.opacity(pulse_alpha)),
                        );
                    }
                    token_cursor = next_c;
                    answer_col = answer_col.child(row);
                }
                AnswerBlock::Bullets(items) => {
                    let mut list = div().w_full().flex().flex_col().gap(px(5.)).pl(px(4.));
                    for (li, item_toks) in items.iter().enumerate() {
                        if token_cursor >= visible_tokens {
                            break;
                        }
                        let (mut row, next_c, _) = self.render_token_stream(
                            item_toks,
                            token_cursor,
                            visible_tokens,
                            &lit_sources,
                            &format!("b{bi}-l{li}"),
                            cx,
                        );
                        if self.phase == AiAnswerPhase::Writing && next_c >= visible_tokens {
                            row = row.child(
                                div()
                                    .ml(px(3.))
                                    .size(px(6.))
                                    .rounded(px(1.5))
                                    .bg(p.blue.opacity(pulse_alpha)),
                            );
                        }
                        token_cursor = next_c;
                        list = list.child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .mt(px(7.))
                                        .size(px(4.5))
                                        .rounded_full()
                                        .flex_none()
                                        .bg(p.faint),
                                )
                                .child(div().flex_1().min_w_0().child(row)),
                        );
                    }
                    answer_col = answer_col.child(list);
                }
            }
        }
        if self.phase == AiAnswerPhase::Reading {
            answer_col = answer_col.child(
                div()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .child(div().size(px(6.)).rounded(px(1.5)).bg(p.blue)),
            );
        }

        // Related follow-up chips once streaming finishes.
        let show_follow_ups = matches!(self.phase, AiAnswerPhase::Done | AiAnswerPhase::Stopped)
            && !self.follow_ups.is_empty();
        let follow_section = pro_fold(
            pro_eid(&self.id, "follow-fold"),
            show_follow_ups,
            126.,
            0.,
            {
                let items = self.follow_ups.clone();
                let entity = cx.entity().clone();
                let base_id = self.id.clone();
                let label_txt = tr(cx, "Related follow-ups", "相关追问").to_owned();
                move || {
                    let mut col = div().mt(px(14.)).w_full().flex().flex_col().child(
                        div()
                            .pb(px(4.))
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.muted)
                            .child(label_txt.to_uppercase()),
                    );
                    for (fi, (q, reply)) in items.into_iter().enumerate() {
                        let ent = entity.clone();
                        let q_click = q.clone();
                        col = col.child(
                            div()
                                .id(pro_eid(&base_id, format!("follow-{fi}")))
                                .role(Role::Button)
                                .aria_label(q.clone())
                                .tab_index(0)
                                .min_h(px(33.))
                                .py(px(6.))
                                .px(px(4.))
                                .border_t_1()
                                .border_color(p.line)
                                .rounded(px(4.))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(10.))
                                .hover(move |s| s.bg(p.hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .on_click(move |_, _, app| {
                                    let q_next = q_click.clone();
                                    let r_next = reply.clone();
                                    ent.update(app, |this, cx| {
                                        this.ask_follow_up(q_next, r_next, cx);
                                    });
                                })
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .child(q),
                                )
                                .child(pro_icon(ProIcon::ArrowRight, 14., p.muted)),
                        );
                    }
                    col.into_any_element()
                }
            },
        );

        // Footer bar: Stop while streaming; Copy, Regenerate, 1/2 Version switcher, Thumbs Up/Down, and Replay when finished.
        let just_copied = self
            .copied_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < 2.0);
        let version_total = self.versions.len();
        let cur_ver = self.active_version;
        let is_up = self.feedback == Some(AiAnswerFeedback::Up);
        let is_down = self.feedback == Some(AiAnswerFeedback::Down);

        let footer = if self.is_streaming() {
            div()
                .h(px(44.))
                .px(px(if self.framed { 14. } else { 0. }))
                .when(self.framed, |d| d.border_t_1().border_color(p.line))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .id(pro_eid(&self.id, "stop-stream-btn"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Stop generating", "停止生成"))
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_color(p.focus))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.stop(visible_tokens, cx);
                        }))
                        .child(div().size(px(8.)).rounded(px(2.)).bg(p.ink))
                        .child(tr(cx, "Stop", "停止")),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(format!("{visible_tokens}/{total_tokens} tokens")),
                )
        } else {
            let status_note: Option<SharedString> = if just_copied {
                Some(tr(cx, "Copied", "已复制").into())
            } else if let Some(ref reason) = self.picked_reason {
                Some(format!("{}: {reason}", tr(cx, "Feedback noted", "已记录反馈")).into())
            } else if is_up {
                Some(tr(cx, "Thanks for the feedback", "感谢反馈").into())
            } else {
                None
            };

            div()
                .min_h(px(44.))
                .px(px(if self.framed { 14. } else { 0. }))
                .when(self.framed, |d| d.border_t_1().border_color(p.line))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        // Copy -> green Check
                        .child(
                            div()
                                .id(pro_eid(&self.id, "act-copy"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Copy answer", "复制回答"))
                                .tab_index(0)
                                .size(px(28.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .on_click(cx.listener(|this, _, _, cx| this.copy_answer(cx)))
                                .child(if just_copied {
                                    pro_icon(ProIcon::Check, 15., p.green_ink)
                                } else {
                                    pro_icon(ProIcon::Copy, 15., p.muted)
                                }),
                        )
                        // Regenerate
                        .child(
                            div()
                                .id(pro_eid(&self.id, "act-regen"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Regenerate answer", "重新生成回答"))
                                .tab_index(0)
                                .size(px(28.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .on_click(cx.listener(|this, _, _, cx| this.regenerate(cx)))
                                .child(pro_icon(ProIcon::Refresh, 15., p.muted)),
                        )
                        // Version switcher `1/2`
                        .when(
                            version_total > 1 && self.follow_up_override.is_none(),
                            |d| {
                                d.child(
                                    div()
                                        .mx(px(2.))
                                        .h(px(24.))
                                        .px(px(4.))
                                        .rounded(px(6.))
                                        .bg(p.chip)
                                        .flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .id(pro_eid(&self.id, "ver-prev"))
                                                .role(Role::Button)
                                                .aria_label(tr(cx, "Previous version", "上一版本"))
                                                .tab_index(0)
                                                .size(px(18.))
                                                .rounded(px(4.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(p.hover))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    let prev = if cur_ver == 0 {
                                                        version_total - 1
                                                    } else {
                                                        cur_ver - 1
                                                    };
                                                    this.switch_version(prev, cx);
                                                }))
                                                .child(pro_icon(ProIcon::ChevronLeft, 12., p.text)),
                                        )
                                        .child(
                                            div()
                                                .px(px(3.))
                                                .text_size(px(11.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.text)
                                                .child(format!(
                                                    "{}/{}",
                                                    cur_ver + 1,
                                                    version_total
                                                )),
                                        )
                                        .child(
                                            div()
                                                .id(pro_eid(&self.id, "ver-next"))
                                                .role(Role::Button)
                                                .aria_label(tr(cx, "Next version", "下一版本"))
                                                .tab_index(0)
                                                .size(px(18.))
                                                .rounded(px(4.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(p.hover))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    let next = (cur_ver + 1) % version_total;
                                                    this.switch_version(next, cx);
                                                }))
                                                .child(pro_icon(
                                                    ProIcon::ChevronRight,
                                                    12.,
                                                    p.text,
                                                )),
                                        ),
                                )
                            },
                        )
                        // Thumbs Up
                        .child(
                            div()
                                .id(pro_eid(&self.id, "act-thumb-up"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Good answer", "回答有帮助"))
                                .tab_index(0)
                                .size(px(28.))
                                .rounded(px(6.))
                                .bg(if is_up { p.chip } else { transparent_black() })
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_feedback(AiAnswerFeedback::Up, cx);
                                }))
                                .child(pro_svg(
                                    "0 0 24 24",
                                    SVG_THUMB_UP,
                                    15.,
                                    if is_up { p.green_ink } else { p.muted },
                                )),
                        )
                        // Thumbs Down
                        .child(
                            div()
                                .id(pro_eid(&self.id, "act-thumb-down"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Poor answer", "回答需改进"))
                                .tab_index(0)
                                .size(px(28.))
                                .rounded(px(6.))
                                .bg(if is_down { p.chip } else { transparent_black() })
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_feedback(AiAnswerFeedback::Down, cx);
                                }))
                                .child(pro_svg(
                                    "0 0 24 24",
                                    SVG_THUMB_DOWN,
                                    15.,
                                    if is_down { p.amber_ink } else { p.muted },
                                )),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when_some(status_note, |d, note| {
                            d.child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(if just_copied { p.green_ink } else { p.muted })
                                    .child(note),
                            )
                        })
                        .child(pro_btn(
                            pro_eid(&self.id, "act-replay"),
                            tr(cx, "Replay stream", "重播流式输出"),
                            Some(ProIcon::Play),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| this.replay(cx),
                        )),
                )
        };

        // Unfolding thumbs-down reason tags (`pro_fold`).
        let reasons_open = is_down && !self.is_streaming();
        let reasons_fold = pro_fold(pro_eid(&self.id, "reasons-fold"), reasons_open, 44., 0., {
            let reasons = self.feedback_reasons.clone();
            let picked = self.picked_reason.clone();
            let entity = cx.entity().clone();
            let base_id = self.id.clone();
            let framed = self.framed;
            move || {
                let mut wrap = div()
                    .w_full()
                    .px(px(if framed { 14. } else { 0. }))
                    .pb(px(if framed { 12. } else { 4. }))
                    .pt(px(2.))
                    .flex()
                    .flex_wrap()
                    .gap(px(6.));
                for (ri, r) in reasons.into_iter().enumerate() {
                    let active = picked.as_ref() == Some(&r);
                    let ent = entity.clone();
                    let r_click = r.clone();
                    wrap = wrap.child(
                        div()
                            .id(pro_eid(&base_id, format!("reason-{ri}")))
                            .role(Role::Button)
                            .aria_label(r.clone())
                            .tab_index(0)
                            .h(px(27.))
                            .px(px(10.))
                            .rounded(px(6.))
                            .bg(if active {
                                p.accent_soft
                            } else {
                                transparent_black()
                            })
                            .border_1()
                            .border_color(if active {
                                p.blue.opacity(0.45)
                            } else {
                                p.line_strong
                            })
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if active { p.blue } else { p.text })
                            .hover(move |s| s.bg(p.chip))
                            .focus_visible(move |s| s.border_color(p.focus))
                            .on_click(move |_, _, app| {
                                let chosen = r_click.clone();
                                ent.update(app, |this, cx| {
                                    this.pick_feedback_reason(chosen, cx);
                                });
                            })
                            .child(r),
                    );
                }
                wrap.into_any_element()
            }
        });

        let version_key = SharedString::from(format!(
            "ans-v{}-{}",
            self.active_version,
            self.follow_up_override.is_some()
        ));

        let card_shell = div()
            .w_full()
            .max_w(px(640.))
            .when(self.framed, |d| {
                d.rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow_xs()
            })
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(if self.framed {
                        if compact { 16. } else { 20. }
                    } else {
                        0.
                    }))
                    .pb(px(if self.framed { 16. } else { 8. }))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .mb(px(14.))
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(self.active_question.clone()),
                    )
                    .child(sources_grid)
                    .child(phase_row)
                    .child(cite_popover)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "answer-vtx"),
                        version_key,
                        answer_col,
                    ))
                    .child(follow_section),
            )
            .child(footer)
            .child(reasons_fold);

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.active_question.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .p(px(if compact { 12. } else { 20. }))
            .flex()
            .flex_col()
            .items_center()
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
            .child(card_shell)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{AnswerBlock, AnswerSource, AnswerToken, format_plain_answer, parse_answer_blocks};

    #[test]
    fn parses_bold_paragraphs_lists_and_filters_invalid_citations() {
        let raw = "Book **LX Factory** now [1,2] and skip broken [9].\n\n- First item [1]\n- **Second** item";
        let blocks = parse_answer_blocks(raw, 3);
        assert_eq!(blocks.len(), 2);

        match &blocks[0] {
            AnswerBlock::Paragraph(toks) => {
                let citations: Vec<&Vec<usize>> = toks
                    .iter()
                    .filter_map(|t| match t {
                        AnswerToken::Citation(v) => Some(v),
                        _ => None,
                    })
                    .collect();
                assert_eq!(citations, vec![&vec![1, 2]]);
                assert!(toks.iter().any(|t| matches!(
                    t,
                    AnswerToken::Word { text, bold: true } if text == "LX"
                )));
            }
            AnswerBlock::Bullets(_) => panic!("expected paragraph first"),
        }

        match &blocks[1] {
            AnswerBlock::Bullets(items) => assert_eq!(items.len(), 2),
            AnswerBlock::Paragraph(_) => panic!("expected bullet list second"),
        }
    }

    #[test]
    fn plain_copy_strips_markup_and_appends_numbered_bibliography() {
        let sources = vec![
            AnswerSource::new("Notion", "Offsite plan").with_href("https://notion.so/plan"),
            AnswerSource::new("Slack", "#team-offsite"),
        ];
        let copied = format_plain_answer("Hold it at **LX Factory** [1,2].", &sources);
        assert!(
            copied.starts_with("Hold it at LX Factory .")
                || copied.starts_with("Hold it at LX Factory.")
        );
        assert!(copied.contains("[1] Notion — Offsite plan (https://notion.so/plan)"));
        assert!(copied.contains("[2] Slack — #team-offsite"));
    }
}
