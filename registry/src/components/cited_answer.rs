//! AI answer wired two-way to its sources with unfolding highlighted quote passages.
//!
//! Connects every cited sentence and inline `[1]` marker to its source row:
//! hovering or focusing either end spotlights both the sentence in prose and
//! its source card, while clicking unfolds the retrieved passage with matching
//! phrases highlighted.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_mix_color, pro_popover_presence, pro_skeleton_bar, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, person_initials, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

#[derive(IntoElement)]
struct CiteGlideArrow {
    id: ElementId,
    active: bool,
    size: f32,
    rest_color: Hsla,
    active_color: Hsla,
}

impl RenderOnce for CiteGlideArrow {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init = self.active;
        let st = w.use_keyed_state(self.id, cx, |_, _| (init, Slide::snappy(f32::from(init))));
        let t = st.update(cx, |s, cx| {
            if s.0 != self.active {
                s.0 = self.active;
                s.1.set(f32::from(self.active), cx);
            }
            s.1.get(w, cx).clamp(0., 1.)
        });
        let col = pro_mix_color(self.rest_color, self.active_color, t);
        div()
            .relative()
            .left(px(2.0 * t))
            .flex_none()
            .child(pro_icon(ProIcon::ArrowRight, self.size, col))
    }
}

/// Grounding source entry wired to one or more [`CitedSegment`] claims.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CitedSource {
    /// Stable source identifier referenced by [`CitedSegment::cite`].
    pub id: SharedString,
    /// Publication, domain, or tool name (such as `"Tokenwise"`).
    pub site: SharedString,
    /// Full article or document title.
    pub title: SharedString,
    /// Optional canonical URL.
    pub url: Option<SharedString>,
    /// Optional publication date label (such as `"Sep 18, 2026"`).
    pub date: Option<SharedString>,
    /// Retrieved verbatim passage backing the cited claims.
    pub quote: SharedString,
    /// Exact phrases inside `quote` to highlight; when empty, shared ≥5-char words are marked automatically.
    pub highlight: Vec<SharedString>,
    /// Optional brand key passed to `pro_app_tile` when citing known apps.
    pub icon: Option<SharedString>,
}

impl CitedSource {
    /// Construct a [`CitedSource`] with `id`, `site`, and `title`.
    pub fn new(
        id: impl Into<SharedString>,
        site: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            site: site.into(),
            title: title.into(),
            url: None,
            date: None,
            quote: SharedString::default(),
            highlight: Vec::new(),
            icon: None,
        }
    }

    /// Attach a canonical source URL.
    pub fn with_url(mut self, url: impl Into<SharedString>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Attach a human-readable publication date.
    pub fn with_date(mut self, date: impl Into<SharedString>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Set the retrieved passage unfolded when this source is inspected.
    pub fn with_quote(mut self, quote: impl Into<SharedString>) -> Self {
        self.quote = quote.into();
        self
    }

    /// Specify exact phrases inside `quote` to mark when unfolded.
    pub fn with_highlights<I, S>(mut self, phrases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.highlight = phrases.into_iter().map(Into::into).collect();
        self
    }

    /// Associate a known brand icon key (such as `"notion"` or `"github"`).
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    fn domain_hint(&self) -> String {
        if let Some(url) = &self.url {
            let trimmed = url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.");
            if let Some((host, _)) = trimmed.split_once('/') {
                return host.to_owned();
            }
            return trimmed.to_owned();
        }
        self.site.to_string()
    }
}

/// Ordered sentence or clause in [`CitedAnswer`] prose.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CitedSegment {
    /// Sentence or clause text.
    pub text: SharedString,
    /// Referenced [`CitedSource::id`] keys (unknown ids are safely ignored).
    pub cite: Vec<SharedString>,
    /// Start a new paragraph before this segment when `true`.
    pub paragraph: bool,
}

impl CitedSegment {
    /// Construct an uncited [`CitedSegment`].
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            cite: Vec::new(),
            paragraph: false,
        }
    }

    /// Attach one or more [`CitedSource::id`] references to this segment.
    pub fn with_cite<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.cite = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Start a new paragraph before this segment.
    pub fn with_paragraph(mut self, paragraph: bool) -> Self {
        self.paragraph = paragraph;
        self
    }
}

/// Typed events emitted by [`CitedAnswer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CitedAnswerEvent {
    /// Unfolded source quote opened, switched, or closed (`None`).
    PreviewChanged(Option<SharedString>),
    /// User clicked `"Open source"` inside an unfolded source card.
    OpenSource(CitedSource),
    /// User copied a source's quoted passage.
    CopyQuote {
        source_id: SharedString,
        quote: SharedString,
    },
    /// User clicked the trailing footer action button (such as `"Follow up"`).
    ActionClicked(SharedString),
}

/// Split `quote` into `(text_slice, is_highlighted)` runs using explicit phrases or shared ≥5-char words.
pub(crate) fn compute_quote_runs(
    quote: &str,
    explicit_phrases: &[SharedString],
    citing_texts: &[&str],
) -> Vec<(String, bool)> {
    if quote.is_empty() {
        return Vec::new();
    }
    let mut mask = vec![false; quote.len()];
    let lower_quote = quote.to_ascii_lowercase();

    if !explicit_phrases.is_empty() {
        for phrase in explicit_phrases {
            let needle = phrase.trim().to_ascii_lowercase();
            if needle.is_empty() || needle.len() > lower_quote.len() {
                continue;
            }
            let mut search_from = 0usize;
            while let Some(rel) = lower_quote[search_from..].find(&needle) {
                let start = search_from + rel;
                let end = (start + needle.len()).min(mask.len());
                for slot in &mut mask[start..end] {
                    *slot = true;
                }
                search_from = end;
            }
        }
    } else {
        // Automatic fallback: mark words ≥5 chars (or numeric tokens) shared with citing sentences.
        let mut shared_words: Vec<String> = Vec::new();
        for seg in citing_texts {
            for raw_w in seg.split(|c: char| !c.is_alphanumeric() && c != '%') {
                let w = raw_w.trim().to_ascii_lowercase();
                let has_digit = w.chars().any(|c| c.is_ascii_digit());
                if (w.chars().count() >= 5 || (has_digit && w.len() >= 2))
                    && !shared_words.contains(&w)
                {
                    shared_words.push(w);
                }
            }
        }
        let mut byte_idx = 0usize;
        for ch_group in quote.split_inclusive(|c: char| !c.is_alphanumeric() && c != '%') {
            let clean = ch_group.trim_matches(|c: char| !c.is_alphanumeric() && c != '%');
            if !clean.is_empty() {
                let lower_w = clean.to_ascii_lowercase();
                if shared_words.contains(&lower_w)
                    && let Some(rel) = ch_group.find(clean)
                {
                    let s = byte_idx + rel;
                    let e = (s + clean.len()).min(mask.len());
                    for slot in &mut mask[s..e] {
                        *slot = true;
                    }
                }
            }
            byte_idx += ch_group.len();
        }
    }

    // Coalesce byte mask along valid UTF-8 char boundaries into word/span runs.
    let mut runs: Vec<(String, bool)> = Vec::new();
    let mut cur_hl: Option<bool> = None;
    let mut cur_buf = String::new();
    for (idx, ch) in quote.char_indices() {
        let hl = mask.get(idx).copied().unwrap_or(false);
        match cur_hl {
            Some(prev) if prev == hl => cur_buf.push(ch),
            Some(prev) => {
                runs.push((std::mem::take(&mut cur_buf), prev));
                cur_buf.push(ch);
                cur_hl = Some(hl);
            }
            None => {
                cur_buf.push(ch);
                cur_hl = Some(hl);
            }
        }
    }
    if let Some(last_hl) = cur_hl
        && !cur_buf.is_empty()
    {
        runs.push((cur_buf, last_hl));
    }
    runs
}

fn default_sources() -> Vec<CitedSource> {
    vec![
        CitedSource::new(
            "caching",
            "Tokenwise",
            "Prompt caching, explained: what gets stored and for how long",
        )
        .with_url("https://tokenwise.dev/blog/prompt-caching")
        .with_date("Sep 18, 2026")
        .with_quote(
            "A cached prefix is read directly from KV memory instead of recomputed on every call, cutting time to first token by up to 80% on prompts over 10K tokens while the prefix stays identical.",
        )
        .with_highlights([
            "read directly from KV memory",
            "recomputed on every call",
            "up to 80%",
            "10K tokens",
        ]),
        CitedSource::new(
            "layout",
            "Latency Notes",
            "Designing cache-friendly prompts for production agents",
        )
        .with_url("https://latencynotes.io/cache-friendly-prompts")
        .with_date("Oct 04, 2026")
        .with_quote(
            "Keep the system prompt, tool schemas, and long reference documents first and append anything that changes per request last so your prefix hit rate stays above 90%.",
        )
        .with_highlights([
            "system prompt, tool schemas",
            "changes per request last",
            "prefix hit rate",
        ]),
        CitedSource::new(
            "semantic",
            "Inference Weekly",
            "Semantic caching for LLM apps in production",
        )
        .with_url("https://inferenceweekly.com/semantic-caching")
        .with_date("Oct 29, 2026")
        .with_quote(
            "Placing a vector semantic cache in front of the model returns stored answers for close matches in about 40 ms, covering nearly a third of FAQ-style traffic in production.",
        )
        .with_highlights([
            "semantic cache",
            "about 40 ms",
            "nearly a third of FAQ-style traffic",
        ]),
    ]
}

fn default_segments() -> Vec<CitedSegment> {
    vec![
        CitedSegment::new(
            "The biggest win is prompt caching: the provider reuses the processed start of your prompt, so a long system prompt isn't recomputed on every call.",
        )
        .with_cite(["caching"]),
        CitedSegment::new(
            "On prompts over 10K tokens that can cut time to first token by up to 80%.",
        )
        .with_cite(["caching"]),
        CitedSegment::new(
            "It only works while that start stays identical, so put the system prompt, tools and long documents first and anything that changes per request last.",
        )
        .with_cite(["caching", "layout"]),
        CitedSegment::new(
            "For repeated questions, add a semantic cache in front of the model: close matches get a stored answer in about 40 ms,",
        )
        .with_cite(["semantic"])
        .with_paragraph(true),
        CitedSegment::new("which covered nearly a third of FAQ-style traffic in one production setup.")
            .with_cite(["semantic"]),
        CitedSegment::new(
            "Stream the rest, so people see the first words while the answer is still being written.",
        ),
    ]
}

fn render_source_glyph(idx: usize, src: &CitedSource, side: f32, cx: &App) -> AnyElement {
    if let Some(ref icon_key) = src.icon {
        return pro_app_tile(icon_key, side, cx);
    }
    let p = ProPalette::get(cx);
    let (bg, fg) = match idx % 4 {
        0 => (p.accent_soft, p.blue),
        1 => (p.purple.opacity(0.16), p.purple),
        2 => (p.green_soft, p.green_ink),
        _ => (p.amber_soft, p.amber_ink),
    };
    let initial = person_initials(&src.site)
        .chars()
        .next()
        .map_or_else(|| "S".to_owned(), |c| c.to_string());
    div()
        .size(px(side))
        .flex_none()
        .rounded(px(4.5))
        .bg(bg)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px((side * 0.60).clamp(9., 11.)))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fg)
        .child(initial)
        .into_any_element()
}

/// AI answer component wired two-way between prose sentences, `[1]` markers, and unfolding source quotes.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::cited_answer::{CitedAnswer, CitedAnswerEvent};
///
/// fn mount(cx: &mut gpui::App) {
///     let card = cx.new(|cx| CitedAnswer::new("rag-cited-answer", cx).with_pages_read(12));
///     cx.subscribe(&card, |_, ev: &CitedAnswerEvent, _| {
///         let _ = ev;
///     })
///     .detach();
/// }
/// ```
pub struct CitedAnswer {
    id: ElementId,
    focus: FocusHandle,
    sources: Vec<CitedSource>,
    segments: Vec<CitedSegment>,
    pages_read: Option<usize>,
    open_source_id: Option<SharedString>,
    hovered_marker: Option<(usize, SharedString)>,
    last_hovered_marker: Option<SharedString>,
    hovered_source: Option<SharedString>,
    hovered_action: bool,
    copied_quote_for: Option<(SharedString, Instant)>,
    copied_all_at: Option<Instant>,
    action_label: Option<SharedString>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl EventEmitter<CitedAnswerEvent> for CitedAnswer {}

impl CitedAnswer {
    /// Create a [`CitedAnswer`] preloaded with 6 RAG claims wired to 3 inspectable sources.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            sources: default_sources(),
            segments: default_segments(),
            pages_read: Some(12),
            open_source_id: Some("caching".into()),
            hovered_marker: None,
            last_hovered_marker: None,
            hovered_source: None,
            hovered_action: false,
            copied_quote_for: None,
            copied_all_at: None,
            action_label: Some("Follow up".into()),
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(560.)),
            toast: None,
        }
    }

    /// Override the ordered [`CitedSegment`] sentences.
    pub fn with_segments(mut self, segments: Vec<CitedSegment>) -> Self {
        self.segments = segments;
        self
    }

    /// Override the numbered [`CitedSource`] list (`1..=N` in slice order).
    pub fn with_sources(mut self, sources: Vec<CitedSource>) -> Self {
        self.sources = sources;
        self
    }

    /// Set the number of pages read shown in the top metadata bar (`"3 sources · 12 pages read"`).
    pub fn with_pages_read(mut self, pages_read: usize) -> Self {
        self.pages_read = Some(pages_read);
        self
    }

    /// Set or clear (`None`) the source id whose quoted passage starts unfolded.
    pub fn with_default_open(mut self, source_id: Option<SharedString>) -> Self {
        self.open_source_id = source_id;
        self
    }

    /// Configure or hide (`None`) the trailing footer button (for example `Some("Follow up".into())`).
    pub fn with_action(mut self, label: Option<SharedString>) -> Self {
        self.action_label = label;
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

    /// Return the currently unfolded source id, if any.
    pub fn open_source_id(&self) -> Option<&str> {
        self.open_source_id.as_deref()
    }

    /// Return `(backed_sentences, total_sentences)` counting segments with at least one valid source id.
    pub fn backed_sentence_counts(&self) -> (usize, usize) {
        let backed = self
            .segments
            .iter()
            .filter(|seg| {
                seg.cite
                    .iter()
                    .any(|cid| self.sources.iter().any(|s| &s.id == cid))
            })
            .count();
        (backed, self.segments.len())
    }

    /// Unfold, switch, or fold closed the quoted passage for `source_id`.
    pub fn toggle_source_preview(&mut self, source_id: SharedString, cx: &mut Context<Self>) {
        if self.open_source_id.as_ref() == Some(&source_id) {
            self.open_source_id = None;
        } else {
            self.open_source_id = Some(source_id);
        }
        cx.emit(CitedAnswerEvent::PreviewChanged(
            self.open_source_id.clone(),
        ));
        cx.notify();
    }

    /// Copy the verbatim quote for `source_id` to the clipboard.
    pub fn copy_quote(&mut self, source_id: SharedString, cx: &mut Context<Self>) {
        let Some(src) = self.sources.iter().find(|s| s.id == source_id) else {
            return;
        };
        let quote = src.quote.clone();
        clipboard::write(&quote, cx).detach();
        self.copied_quote_for = Some((source_id.clone(), Instant::now()));
        cx.emit(CitedAnswerEvent::CopyQuote { source_id, quote });
        cx.notify();
    }

    fn source_number(&self, source_id: &str) -> Option<usize> {
        self.sources
            .iter()
            .position(|s| s.id.as_ref() == source_id)
            .map(|i| i + 1)
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.open_source_id.take().is_some() {
                    cx.emit(CitedAnswerEvent::PreviewChanged(None));
                    cx.notify();
                }
            }
            "right" | "down" if !self.sources.is_empty() => {
                let cur = self
                    .open_source_id
                    .as_ref()
                    .and_then(|id| self.sources.iter().position(|s| &s.id == id));
                let next = cur.map_or(0, |i| (i + 1) % self.sources.len());
                let next_id = self.sources[next].id.clone();
                self.open_source_id = Some(next_id.clone());
                self.hovered_source = Some(next_id.clone());
                cx.emit(CitedAnswerEvent::PreviewChanged(Some(next_id)));
                cx.notify();
            }
            "left" | "up" if !self.sources.is_empty() => {
                let len = self.sources.len();
                let cur = self
                    .open_source_id
                    .as_ref()
                    .and_then(|id| self.sources.iter().position(|s| &s.id == id));
                let prev = cur.map_or(len - 1, |i| if i == 0 { len - 1 } else { i - 1 });
                let prev_id = self.sources[prev].id.clone();
                self.open_source_id = Some(prev_id.clone());
                self.hovered_source = Some(prev_id.clone());
                cx.emit(CitedAnswerEvent::PreviewChanged(Some(prev_id)));
                cx.notify();
            }
            _ => {}
        }
    }
}

impl Render for CitedAnswer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let measure_cell = self.measured_width.clone();
        let compact = width < 420.;

        if let Some((_, started)) = self.copied_quote_for
            && started.elapsed().as_secs_f32() < 2.2
            && !reduced
        {
            w.request_animation_frame();
        }
        if let Some(started) = self.copied_all_at
            && started.elapsed().as_secs_f32() < 2.2
            && !reduced
        {
            w.request_animation_frame();
        }

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(540.))
                        .p(px(20.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(180., 16., 4., 0.25, cx))
                        .child(pro_skeleton_bar(480., 15., 4., 0.45, cx))
                        .child(pro_skeleton_bar(440., 15., 4., 0.65, cx))
                        .child(pro_skeleton_bar(360., 36., 7., 0.85, cx)),
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
                        .max_w(px(540.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::PageDoc,
                            tr(cx, "No cited claims to inspect", "暂无可核验的引用回答"),
                            tr(
                                cx,
                                "Pass retrieved sources and cited sentence segments to wire two-way highlights.",
                                "传入检索到的来源与分句片段即可启用双向句子高亮与原文展开。",
                            ),
                            Some((
                                pro_eid(&self.id, "restore-cited"),
                                tr(cx, "Load sample answer", "加载示例回答").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                );
        }

        // Stacked header glyphs + `"3 sources · 12 pages read"`.
        let mut glyph_stack = div().flex().items_center();
        for (idx, src) in self.sources.iter().take(4).enumerate() {
            glyph_stack = glyph_stack.child(
                div()
                    .when(idx > 0, |d| d.ml(px(-4.)))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(p.card)
                    .child(render_source_glyph(idx, src, 16., cx)),
            );
        }
        let header_meta = match self.pages_read {
            Some(pages) => format!(
                "{} {} · {} {}",
                self.sources.len(),
                tr(cx, "sources", "个来源"),
                pages,
                tr(cx, "pages read", "页已读")
            ),
            None => format!("{} {}", self.sources.len(), tr(cx, "sources", "个来源")),
        };

        let head_row = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(glyph_stack)
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(header_meta),
            );

        // Two-way active focus determination:
        // - Hovering a marker `(seg_idx, src_id)` highlights `seg_idx` + `src_id`.
        // - Hovering a source row `src_id` highlights every segment citing `src_id` + `src_id`.
        let any_pointer_spotlight = self.hovered_marker.is_some() || self.hovered_source.is_some();
        let active_source_spotlight: Option<&SharedString> = self
            .hovered_source
            .as_ref()
            .or_else(|| self.hovered_marker.as_ref().map(|(_, sid)| sid));

        let hl_sentence_bg: Hsla = if p.dark {
            hsla(214. / 360., 0.95, 0.65, 0.16)
        } else {
            hsla(212. / 360., 1.0, 0.45, 0.085)
        };
        let mark_bg: Hsla = if p.dark {
            hsla(214. / 360., 0.95, 0.65, 0.28)
        } else {
            hsla(212. / 360., 1.0, 0.45, 0.14)
        };

        // Group segments into paragraphs.
        let mut paragraphs: Vec<Vec<(usize, CitedSegment)>> = Vec::new();
        for (si, seg) in self.segments.iter().cloned().enumerate() {
            if seg.paragraph || paragraphs.is_empty() {
                paragraphs.push(vec![(si, seg)]);
            } else if let Some(last_p) = paragraphs.last_mut() {
                last_p.push((si, seg));
            }
        }

        let mut prose_col = div().mt(px(12.)).w_full().flex().flex_col().gap(px(10.));
        for para in paragraphs {
            let mut para_col = div().w_full().flex().flex_col().gap(px(4.));
            for (si, seg) in para {
                let valid_cites: Vec<(usize, SharedString)> = seg
                    .cite
                    .iter()
                    .filter_map(|cid| self.source_number(cid).map(|num| (num, cid.clone())))
                    .collect();

                let seg_lit = if let Some((hm_seg, _)) = &self.hovered_marker {
                    *hm_seg == si
                } else if let Some(hs_id) = &self.hovered_source {
                    valid_cites.iter().any(|(_, cid)| cid == hs_id)
                } else if let Some(open_id) = &self.open_source_id {
                    valid_cites.iter().any(|(_, cid)| cid == open_id)
                } else {
                    false
                };

                let seg_dim = any_pointer_spotlight && !seg_lit;
                let first_cite = valid_cites.first().map(|(_, cid)| cid.clone());

                let mut seg_row = div()
                    .id(pro_eid(&self.id, format!("seg-{si}")))
                    .px(px(5.))
                    .py(px(2.5))
                    .mx(px(-5.))
                    .rounded(px(5.))
                    .bg(if seg_lit {
                        hl_sentence_bg
                    } else {
                        transparent_black()
                    })
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x(px(3.5))
                    .gap_y(px(2.));

                if let Some(primary_cid) = first_cite {
                    let cid_click = primary_cid.clone();
                    seg_row = seg_row
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_marker = Some((si, primary_cid.clone()));
                            } else if this
                                .hovered_marker
                                .as_ref()
                                .is_some_and(|(idx, _)| *idx == si)
                            {
                                this.hovered_marker = None;
                            }
                            cx.notify();
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_source_preview(cid_click.clone(), cx);
                        }));
                }

                for word in seg.text.split_whitespace() {
                    seg_row = seg_row.child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(if seg_lit {
                                p.strong
                            } else if seg_dim {
                                p.faint
                            } else {
                                p.ink
                            })
                            .child(word.to_owned()),
                    );
                }

                for (mi, (num, cid)) in valid_cites.into_iter().enumerate() {
                    let marker_lit = active_source_spotlight == Some(&cid)
                        || self.open_source_id.as_ref() == Some(&cid);
                    let src_title = self
                        .sources
                        .get(num - 1)
                        .map(|s| format!("Source {}: {} — {}", num, s.site, s.title))
                        .unwrap_or_else(|| format!("Source {num}"));
                    let cid_hov = cid.clone();
                    let cid_clk = cid.clone();

                    seg_row = seg_row.child(
                        div()
                            .id(pro_eid(&self.id, format!("marker-{si}-{mi}")))
                            .role(Role::Button)
                            .aria_label(src_title)
                            .aria_expanded(self.open_source_id.as_ref() == Some(&cid))
                            .tab_index(0)
                            .min_w(px(17.))
                            .h(px(17.))
                            .px(px(4.5))
                            .rounded(px(4.))
                            .bg(if marker_lit { p.accent_soft } else { p.chip })
                            .border_1()
                            .border_color(if marker_lit {
                                p.blue.opacity(0.45)
                            } else {
                                p.line
                            })
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if marker_lit { p.blue } else { p.text })
                            .when(seg_dim && !marker_lit, |d| d.opacity(0.45))
                            .focus_visible(move |s| s.border_color(p.focus))
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_marker = Some((si, cid_hov.clone()));
                                } else if this
                                    .hovered_marker
                                    .as_ref()
                                    .is_some_and(|(idx, id)| *idx == si && id == &cid_hov)
                                {
                                    this.hovered_marker = None;
                                }
                                cx.notify();
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.toggle_source_preview(cid_clk.clone(), cx);
                            }))
                            .child(format!("{num}")),
                    );
                }

                para_col = para_col.child(seg_row);
            }
            prose_col = prose_col.child(para_col);
        }

        // Hovered citation preview card (`pro_popover_presence` smooth enter & exit!)
        let hover_pop_cid = self
            .hovered_marker
            .as_ref()
            .map(|(_, cid)| cid.clone())
            .filter(|cid| self.open_source_id.as_ref() != Some(cid));
        let hover_pop_open = hover_pop_cid.is_some();
        if let Some(ref cid) = hover_pop_cid {
            self.last_hovered_marker = Some(cid.clone());
        }
        let shown_pop_cid = hover_pop_cid.or_else(|| self.last_hovered_marker.clone());
        let hover_pop_data = shown_pop_cid.and_then(|cid| {
            self.sources
                .iter()
                .enumerate()
                .find(|(_, s)| s.id == cid)
                .map(|(i, s)| (i, s.clone(), render_source_glyph(i, s, 15., cx)))
        });
        let hover_cite_popover = pro_popover_presence(
            pro_eid(&self.id, "hover-cite-pop"),
            hover_pop_open,
            move || {
                let Some((idx, s, glyph)) = hover_pop_data else {
                    return div().into_any_element();
                };
                div()
                    .mt(px(10.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(if p.dark { rgb(0x222226) } else { rgb(0xffffff) })
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(glyph)
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.strong)
                                    .child(s.site.clone()),
                            )
                            .child(
                                div()
                                    .px(px(4.5))
                                    .rounded(px(4.))
                                    .bg(p.accent_soft)
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.blue)
                                    .child(format!("[{}]", idx + 1)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(s.domain_hint()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(17.5))
                            .text_color(p.text)
                            .line_clamp(2)
                            .child(s.quote),
                    )
                    .into_any_element()
            },
        );

        // Source Rows wired two-way + `pro_fold` quoted passage directly underneath each source!
        let mut sources_list = div().mt(px(14.)).w_full().flex().flex_col().gap(px(6.));
        for (idx, src) in self.sources.iter().cloned().enumerate() {
            let num = idx + 1;
            let sid = src.id.clone();
            let is_open = self.open_source_id.as_ref() == Some(&sid);
            let is_spotlit = active_source_spotlight == Some(&sid);
            let is_dim = any_pointer_spotlight && !is_spotlit && !is_open;
            let cite_count = self
                .segments
                .iter()
                .filter(|seg| seg.cite.iter().any(|c| c == &sid))
                .count();
            let citing_texts: Vec<&str> = self
                .segments
                .iter()
                .filter(|seg| seg.cite.iter().any(|c| c == &sid))
                .map(|seg| seg.text.as_ref())
                .collect();
            let quote_runs = compute_quote_runs(&src.quote, &src.highlight, &citing_texts);
            let just_copied_this = self
                .copied_quote_for
                .as_ref()
                .is_some_and(|(cid, t)| cid == &sid && t.elapsed().as_secs_f32() < 2.0);

            let sid_hov = sid.clone();
            let sid_clk = sid.clone();

            let row_header_inner = div()
                .px(px(10.))
                .py(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(render_source_glyph(idx, &src, 18., cx))
                .child(
                    div().flex_1().min_w_0().flex().flex_col().child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if is_spotlit || is_open {
                                        p.strong
                                    } else {
                                        p.ink
                                    })
                                    .child(src.site.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(format!("— {}", src.title)),
                            ),
                    ),
                )
                .child(
                    div()
                        .h(px(18.))
                        .px(px(5.5))
                        .rounded(px(4.))
                        .bg(if is_spotlit || is_open {
                            p.accent_soft
                        } else {
                            p.chip
                        })
                        .flex()
                        .items_center()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if is_spotlit || is_open {
                            p.blue
                        } else {
                            p.muted
                        })
                        .child(format!("[{num}] · {cite_count}")),
                )
                .child(pro_icon(
                    if is_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.muted,
                ));

            let row_header = pro_tactile(
                pro_eid(&self.id, format!("source-row-{idx}")),
                format!("Source {}: {} — {}", num, src.site, src.title),
                7.,
                transparent_black(),
                p.hover,
                transparent_black(),
                transparent_black(),
                is_spotlit || is_open,
                row_header_inner,
                cx,
                move |this, _, cx| {
                    this.toggle_source_preview(sid_clk.clone(), cx);
                },
            )
            .aria_expanded(is_open)
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_source = Some(sid_hov.clone());
                } else if this.hovered_source.as_ref() == Some(&sid_hov) {
                    this.hovered_source = None;
                }
                cx.notify();
            }));

            let quote_fold_body: AnyElement = {
                let domain = src.domain_hint();
                let date = src.date.clone();
                let title = src.title.clone();
                let src_open = src.clone();
                let sid_copy = sid.clone();
                let copy_txt: SharedString = if just_copied_this {
                    tr(cx, "Quote copied", "已复制引文").into()
                } else {
                    tr(cx, "Copy quote", "复制引文").into()
                };
                let open_txt: SharedString = tr(cx, "Open source", "打开原文").into();
                let is_row_hov = self.hovered_source.as_ref() == Some(&sid);

                let mut quote_wrap = div()
                    .pl(px(10.))
                    .border_l_2()
                    .border_color(p.blue.opacity(0.60))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x(px(3.5))
                    .gap_y(px(2.));

                for (span_str, highlighted) in quote_runs {
                    for word in span_str.split_whitespace() {
                        quote_wrap = quote_wrap.child(
                            div()
                                .when(highlighted, |d| d.px(px(3.)).rounded(px(3.)).bg(mark_bg))
                                .text_size(px(12.))
                                .line_height(px(18.5))
                                .font_weight(if highlighted {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if highlighted { p.strong } else { p.text })
                                .child(word.to_owned()),
                        );
                    }
                }

                let open_btn_inner = div()
                    .h(px(26.))
                    .px(px(9.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .whitespace_nowrap()
                    .child(open_txt.clone())
                    .child(CiteGlideArrow {
                        id: pro_eid(&self.id, format!("open-src-arr-{idx}")),
                        active: is_row_hov,
                        size: 12.,
                        rest_color: p.muted,
                        active_color: p.strong,
                    });

                let copy_icon_slot = div()
                    .relative()
                    .size(px(13.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!just_copied_this, |d| {
                        d.child(pro_icon(ProIcon::Copy, 13., p.muted))
                    })
                    .child(
                        div()
                            .when(!just_copied_this, |d| d.absolute().inset_0().opacity(0.))
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("copy-quote-chk-{idx}")),
                                just_copied_this,
                                13.,
                                false,
                                p.green,
                            )),
                    );

                let copy_btn_inner = div()
                    .h(px(26.))
                    .px(px(9.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(copy_icon_slot)
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("copy-quote-lbl-{idx}")),
                        copy_txt.clone(),
                        12.,
                        FontWeight::MEDIUM,
                        if just_copied_this {
                            p.green_ink
                        } else {
                            p.text
                        },
                    ));

                div()
                    .px(px(12.))
                    .pb(px(11.))
                    .pt(px(6.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(7.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(match date {
                                Some(dt) => format!("{domain} · {dt}"),
                                None => domain,
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title),
                    )
                    .child(quote_wrap)
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("open-src-{idx}")),
                                    open_txt,
                                    6.,
                                    p.card,
                                    p.hover,
                                    p.line_strong,
                                    p.line_strong,
                                    false,
                                    open_btn_inner,
                                    cx,
                                    move |this, _, cx| {
                                        let s_ev = src_open.clone();
                                        this.toast = Some((
                                            SharedString::from(format!(
                                                "{} · {}",
                                                s_ev.site,
                                                s_ev.domain_hint()
                                            )),
                                            Instant::now(),
                                        ));
                                        cx.emit(CitedAnswerEvent::OpenSource(s_ev));
                                        cx.notify();
                                    },
                                )
                                .flex_none(),
                            )
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("copy-quote-{idx}")),
                                    copy_txt,
                                    6.,
                                    if just_copied_this {
                                        p.green_soft
                                    } else {
                                        transparent_black()
                                    },
                                    p.hover,
                                    transparent_black(),
                                    if just_copied_this {
                                        p.green.opacity(0.35)
                                    } else {
                                        transparent_black()
                                    },
                                    just_copied_this,
                                    copy_btn_inner,
                                    cx,
                                    move |this, _, cx| {
                                        this.copy_quote(sid_copy.clone(), cx);
                                    },
                                )
                                .flex_none(),
                            ),
                    )
                    .into_any_element()
            };

            let quote_fold = pro_fold(
                pro_eid(&self.id, format!("quote-fold-{idx}")),
                is_open,
                118.,
                0.,
                move || quote_fold_body,
            );

            sources_list = sources_list.child(
                div()
                    .w_full()
                    .rounded(px(8.))
                    .bg(if is_spotlit || is_open {
                        p.subtle
                    } else {
                        transparent_black()
                    })
                    .border_1()
                    .border_color(if is_spotlit {
                        p.blue.opacity(0.45)
                    } else if is_open {
                        p.line_strong
                    } else {
                        p.line
                    })
                    .when(is_dim, |d| d.opacity(0.55))
                    .flex()
                    .flex_col()
                    .child(row_header)
                    .child(quote_fold),
            );
        }

        // Dynamic Footer Tray (`5 of 6 sentences backed by a source` / hovered source summary / `Quote copied`).
        let (backed_n, total_n) = self.backed_sentence_counts();
        let any_just_copied = self
            .copied_quote_for
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() < 2.0);
        let all_just_copied = self
            .copied_all_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < 2.0);
        let (tray_key, tray_caption, tray_ok) = if any_just_copied || all_just_copied {
            (
                SharedString::from("copied"),
                if all_just_copied {
                    tr(cx, "Copied answer & citations", "已复制全文与引用").to_owned()
                } else {
                    tr(cx, "Quote copied to clipboard", "引文已复制到剪贴板").to_owned()
                },
                true,
            )
        } else if let Some(spot_id) = active_source_spotlight
            && let Some((idx, src)) = self
                .sources
                .iter()
                .enumerate()
                .find(|(_, s)| &s.id == spot_id)
        {
            let cnt = self
                .segments
                .iter()
                .filter(|seg| seg.cite.iter().any(|c| c == spot_id))
                .count();
            (
                SharedString::from(format!("spot-{spot_id}")),
                format!(
                    "[{}] {} · {} {}",
                    idx + 1,
                    src.site,
                    cnt,
                    tr(cx, "sentences cited", "处句子引用")
                ),
                false,
            )
        } else {
            (
                SharedString::from("rest"),
                format!(
                    "{backed_n} {} {total_n} {}",
                    tr(cx, "of", "/"),
                    tr(cx, "sentences backed by a source", "句回答有来源支撑")
                ),
                false,
            )
        };

        let copy_all_lbl = if all_just_copied {
            tr(cx, "Copied", "已复制")
        } else {
            tr(cx, "Copy", "复制")
        };
        let copy_all_icon = div()
            .relative()
            .size(px(13.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(!all_just_copied, |d| {
                d.child(pro_icon(ProIcon::Copy, 13., p.muted))
            })
            .child(
                div()
                    .when(!all_just_copied, |d| d.absolute().inset_0().opacity(0.))
                    .child(pro_check_draw(
                        pro_eid(&self.id, "tray-copy-chk"),
                        all_just_copied,
                        13.,
                        false,
                        p.green,
                    )),
            );
        let copy_all_inner = div()
            .h(px(28.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(5.))
            .child(copy_all_icon)
            .child(pro_swap_label(
                pro_eid(&self.id, "tray-copy-lbl"),
                copy_all_lbl,
                12.,
                FontWeight::MEDIUM,
                if all_just_copied { p.green_ink } else { p.text },
            ));

        let footer_tray = div()
            .h(px(44.))
            .px(px(if compact { 14. } else { 20. }))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div().flex_1().min_w_0().child(pro_view_transition(
                    pro_eid(&self.id, "tray-vtx"),
                    tray_key,
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .font_weight(if tray_ok {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if tray_ok { p.green_ink } else { p.muted })
                        .child(tray_caption),
                )),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "tray-copy-btn"),
                            tr(cx, "Copy answer", "复制全文"),
                            6.,
                            if all_just_copied {
                                p.green_soft
                            } else {
                                transparent_black()
                            },
                            p.hover,
                            transparent_black(),
                            if all_just_copied {
                                p.green.opacity(0.35)
                            } else {
                                transparent_black()
                            },
                            all_just_copied,
                            copy_all_inner,
                            cx,
                            |this, _, cx| {
                                let joined = this
                                    .segments
                                    .iter()
                                    .map(|s| s.text.as_ref())
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                clipboard::write(&joined, cx).detach();
                                this.copied_all_at = Some(Instant::now());
                                cx.notify();
                            },
                        )
                        .flex_none(),
                    )
                    .when_some(self.action_label.clone(), |d, act_lbl| {
                        let lbl_emit = act_lbl.clone();
                        let act_hov = self.hovered_action;
                        let act_inner = div()
                            .h(px(28.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .whitespace_nowrap()
                            .child(act_lbl.clone())
                            .child(CiteGlideArrow {
                                id: pro_eid(&self.id, "tray-act-arr"),
                                active: act_hov,
                                size: 13.,
                                rest_color: p.muted,
                                active_color: p.strong,
                            });
                        d.child(
                            pro_tactile(
                                pro_eid(&self.id, "tray-action"),
                                act_lbl,
                                6.,
                                p.card,
                                p.hover,
                                p.line_strong,
                                p.line_strong,
                                false,
                                act_inner,
                                cx,
                                move |this, _, cx| {
                                    this.toast = Some((
                                        SharedString::from(format!(
                                            "{}: {}",
                                            lbl_emit,
                                            this.sources.len()
                                        )),
                                        Instant::now(),
                                    ));
                                    cx.emit(CitedAnswerEvent::ActionClicked(lbl_emit.clone()));
                                    cx.notify();
                                },
                            )
                            .flex_none()
                            .on_hover(cx.listener(
                                |this, inside: &bool, _, cx| {
                                    if this.hovered_action != *inside {
                                        this.hovered_action = *inside;
                                        cx.notify();
                                    }
                                },
                            )),
                        )
                    }),
            );

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(tr(cx, "Answer with sources", "带来源引用的回答"))
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
            .child(
                div()
                    .w_full()
                    .max_w(px(540.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow_xs()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .p(px(if compact { 15. } else { 20. }))
                            .pb(px(16.))
                            .flex()
                            .flex_col()
                            .child(head_row)
                            .child(prose_col)
                            .child(hover_cite_popover)
                            .child(sources_list),
                    )
                    .child(footer_tray),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_quote_runs, default_segments, default_sources};

    #[test]
    fn marks_explicit_phrases_and_auto_shared_five_letter_words() {
        let explicit = compute_quote_runs(
            "A cached prefix cuts latency by up to 80% on 10K tokens.",
            &["up to 80%".into()],
            &[],
        );
        let marked: Vec<&str> = explicit
            .iter()
            .filter(|(_, hl)| *hl)
            .map(|(s, _)| s.as_str())
            .collect();
        assert_eq!(marked, vec!["up to 80%"]);

        let auto = compute_quote_runs(
            "Vector semantic cache answers queries in 40 ms.",
            &[],
            &["Add a semantic cache in front of the model for 40 ms hits."],
        );
        let auto_marked: String = auto
            .iter()
            .filter(|(_, hl)| *hl)
            .map(|(s, _)| s.as_str())
            .collect::<Vec<_>>()
            .join("|");
        assert!(auto_marked.contains("semantic"));
        assert!(auto_marked.contains("cache"));
        assert!(auto_marked.contains("40"));
    }

    #[test]
    fn default_dataset_backs_five_of_six_sentences() {
        let sources = default_sources();
        let segments = default_segments();
        let backed = segments
            .iter()
            .filter(|seg| {
                seg.cite
                    .iter()
                    .any(|cid| sources.iter().any(|s| &s.id == cid))
            })
            .count();
        assert_eq!(backed, 5);
        assert_eq!(segments.len(), 6);
    }
}
