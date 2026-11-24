//! Live context-window segmented bar and shimmer-to-summary compaction visualizer.
//!
//! Renders an agent's token budget as a proportional multi-kind top bar (`System`, `Tools`, `User`,
//! `Assistant`, `Tool results`, `Files`, `Summary`) crossed by a dashed amber auto-compaction mark,
//! synchronized on hover and keyboard focus with ordered entry rows below. Clicking `"Compact now"`
//! shimmers ([`pro_shimmer_text`]) older turns, folds them via [`Slide`] springs into a single
//! `"Compacted summary"` row, squeezes their bar slices, and rolls token totals down.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid, pro_empty_state,
        pro_fold, pro_rolling_number, pro_shimmer_text, pro_skeleton_bar, pro_spring_f32, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const SHIMMER_PHASE_SECS: f32 = 0.78;

/// Events emitted by [`ContextCompaction`].
#[derive(Clone, Debug, PartialEq)]
pub enum ContextCompactionEvent {
    /// Compaction began shimmering the given replaceable entry ids.
    CompactStarted(Vec<SharedString>),
    /// Compaction finished collapsing old entries into `summary_id`.
    Compacted {
        summary_id: SharedString,
        before_tokens: u64,
        after_tokens: u64,
        count: usize,
    },
    /// A new conversational or tool entry was appended to the window.
    TurnAppended(ContextEntry),
    /// Context entries were reset to the initial snapshot.
    Reset,
}

/// Semantic category of a [`ContextEntry`] determining its slice color and row icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextEntryKind {
    /// Persistent system prompt instructions (neutral slate).
    #[default]
    System,
    /// Registered tool schemas and MCP app definitions (amber).
    Tools,
    /// User message turn (pink).
    User,
    /// Assistant reasoning / response turn (blue).
    Assistant,
    /// Tool execution output payload (teal).
    ToolResult,
    /// Repository file read into context (purple).
    File,
    /// Condensed summary replacing compacted turns (green).
    Summary,
}

impl ContextEntryKind {
    fn slice_color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::System => {
                if p.dark {
                    hsla(0., 0., 1., 0.40)
                } else {
                    hsla(0., 0., 0., 0.30)
                }
            }
            Self::Tools => p.amber,
            Self::User => p.pink,
            Self::Assistant => p.blue,
            Self::ToolResult => p.teal,
            Self::File => p.purple,
            Self::Summary => p.green,
        }
    }

    fn kind_name(self, cx: &App) -> &'static str {
        match self {
            Self::System => tr(cx, "System prompt", "系统提示词"),
            Self::Tools => tr(cx, "Tool definitions", "工具定义"),
            Self::User => tr(cx, "You", "用户"),
            Self::Assistant => tr(cx, "Agent", "智能体"),
            Self::ToolResult => tr(cx, "Tool result", "工具输出"),
            Self::File => tr(cx, "File", "文件"),
            Self::Summary => tr(cx, "Summary", "压缩摘要"),
        }
    }
}

/// One ordered item occupying tokens inside [`ContextCompaction`].
#[derive(Clone, Debug, PartialEq)]
pub struct ContextEntry {
    /// Stable entry id.
    pub id: SharedString,
    /// Entry category (`System`, `Tools`, `User`, `Assistant`, `ToolResult`, `File`, `Summary`).
    pub kind: ContextEntryKind,
    /// Primary row label (`"src/cart/cart.ts"`, `"14 tool definitions"`).
    pub label: SharedString,
    /// Optional Chinese row label.
    pub label_zh: Option<SharedString>,
    /// Current token footprint.
    pub tokens: u64,
    /// Pre-compaction token total when `kind == ContextEntryKind::Summary`.
    pub before_tokens: Option<u64>,
    /// Archived `(label, tokens)` entries folded into this summary.
    pub archived: Vec<(SharedString, u64)>,
    /// Optional connected app keys (`"github"`, `"linear"`, `"stripe"`) for `Tools` rows.
    pub apps: Vec<SharedString>,
    /// Additional tool count beyond `apps.len()` (`"+11"`).
    pub extra_apps: usize,
}

impl ContextEntry {
    /// Create a [`ContextEntry`] with `id`, `kind`, `label`, and `tokens`.
    pub fn new(
        id: impl Into<SharedString>,
        kind: ContextEntryKind,
        label: impl Into<SharedString>,
        tokens: u64,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            label_zh: None,
            tokens,
            before_tokens: None,
            archived: Vec::new(),
            apps: Vec::new(),
            extra_apps: 0,
        }
    }

    /// Set localized Chinese label.
    pub fn with_label_zh(mut self, label_zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(label_zh.into());
        self
    }

    /// Attach overlapping provider marks (`"github"`, `"linear"`, `"stripe"`) and overflow count.
    pub fn with_apps(mut self, apps: Vec<SharedString>, extra_apps: usize) -> Self {
        self.apps = apps;
        self.extra_apps = extra_apps;
        self
    }

    /// Mark pre-compaction token total and archived items on a `Summary` entry.
    pub fn with_summary_origin(
        mut self,
        before_tokens: u64,
        archived: Vec<(SharedString, u64)>,
    ) -> Self {
        self.before_tokens = Some(before_tokens);
        self.archived = archived;
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> &'a str {
        match &self.label_zh {
            Some(zh) => tr(cx, self.label.as_ref(), zh.as_ref()),
            None => self.label.as_ref(),
        }
    }
}

/// Format raw token counts into compact `K` units (`"400"`, `"3.2K"`, `"165K"`).
pub fn format_tokens_compact(tokens: u64) -> String {
    if tokens < 1_000 {
        tokens.to_string()
    } else if tokens < 100_000 {
        let k = tokens as f64 / 1_000.0;
        if (k - k.round()).abs() < 0.05 {
            format!("{}K", k.round() as u64)
        } else {
            format!("{k:.1}K")
        }
    } else {
        format!("{:.1}K", tokens as f64 / 1_000.0)
            .trim_end_matches(".0K")
            .to_string()
            + if (tokens % 1_000) < 50 { "K" } else { "" }
    }
}

/// Live context-window bar and shimmer-to-summary compaction visualizer.
pub struct ContextCompaction {
    id: ElementId,
    title: SharedString,
    title_zh: Option<SharedString>,
    model: Option<SharedString>,
    limit: u64,
    threshold: f32,
    entries: Vec<ContextEntry>,
    initial_entries: Vec<ContextEntry>,
    retired_entries: Vec<ContextEntry>,
    compacting_ids: Vec<SharedString>,
    compacting_started: Option<Instant>,
    summary_seq: usize,
    turn_seq: usize,
    hovered_id: Option<SharedString>,
    focused_idx: Option<usize>,
    expanded_id: Option<SharedString>,
    loading: bool,
    empty: bool,
    started_at: Instant,
}

impl EventEmitter<ContextCompactionEvent> for ContextCompaction {}

impl ContextCompaction {
    /// Create a [`ContextCompaction`] card preloaded with `"Fix the cart total"` near its `160K` auto-compact mark.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let entries = default_entries();
        Self {
            id: id.into(),
            title: SharedString::from("Fix the cart total"),
            title_zh: Some(SharedString::from("修复购物车分币舍入误差")),
            model: Some(SharedString::from("Claude Sonnet")),
            limit: 200_000,
            threshold: 0.80,
            initial_entries: entries.clone(),
            entries,
            retired_entries: Vec::new(),
            compacting_ids: Vec::new(),
            compacting_started: None,
            summary_seq: 0,
            turn_seq: 0,
            hovered_id: None,
            focused_idx: None,
            expanded_id: None,
            loading: false,
            empty: false,
            started_at: Instant::now(),
        }
    }

    /// Set header task title (`"Fix the cart total"` default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self.title_zh = None;
        self
    }

    /// Set localized Chinese task title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the model pill shown beside the title (`"Claude Sonnet"`).
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set total context window token limit (`200_000` default).
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = limit.max(1_000);
        self
    }

    /// Set the auto-compaction share of `limit` (`0.80` default -> dashed mark at `80%`).
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.25, 0.98);
        self
    }

    /// Supply ordered [`ContextEntry`] slices.
    pub fn with_entries(mut self, entries: Vec<ContextEntry>) -> Self {
        self.initial_entries = entries.clone();
        self.entries = entries;
        self.retired_entries.clear();
        self
    }

    /// Show skeleton placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show empty context state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current live entries in order.
    pub fn entries(&self) -> &[ContextEntry] {
        &self.entries
    }

    /// Compute total active tokens across `entries()`.
    pub fn used_tokens(&self) -> u64 {
        self.entries.iter().map(|e| e.tokens).sum()
    }

    /// Return candidate entry ids eligible for compaction (excludes `System`, `Tools`, existing `Summary`, and the newest active `File`).
    pub fn compactable_ids(&self) -> Vec<SharedString> {
        let last_file_id = self
            .entries
            .iter()
            .rev()
            .find(|e| e.kind == ContextEntryKind::File)
            .map(|e| e.id.clone());

        self.entries
            .iter()
            .filter(|e| match e.kind {
                ContextEntryKind::System | ContextEntryKind::Tools | ContextEntryKind::Summary => {
                    false
                }
                ContextEntryKind::File => Some(&e.id) != last_file_id.as_ref(),
                ContextEntryKind::User
                | ContextEntryKind::Assistant
                | ContextEntryKind::ToolResult => true,
            })
            .map(|e| e.id.clone())
            .collect()
    }

    /// Trigger context compaction (`shimmers` first unless `reduced_motion` is active, then folds into a summary row).
    pub fn start_compact(&mut self, cx: &mut Context<Self>) -> bool {
        if self.compacting_started.is_some() {
            return false;
        }
        let targets = self.compactable_ids();
        if targets.is_empty() {
            return false;
        }
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        self.compacting_ids = targets.clone();
        cx.emit(ContextCompactionEvent::CompactStarted(targets));
        if reduced {
            self.finish_compaction(cx);
        } else {
            self.compacting_started = Some(Instant::now());
            cx.notify();
        }
        true
    }

    /// Complete the active compaction immediately, folding `compacting_ids` into a single `Summary` entry.
    pub fn finish_compaction(&mut self, cx: &mut Context<Self>) -> bool {
        let targets = if self.compacting_ids.is_empty() {
            self.compactable_ids()
        } else {
            std::mem::take(&mut self.compacting_ids)
        };
        self.compacting_started = None;
        if targets.is_empty() {
            return false;
        }

        let mut insert_pos: Option<usize> = None;
        let mut before_tokens = 0u64;
        let mut archived = Vec::new();
        let mut next_entries = Vec::with_capacity(self.entries.len());

        for e in self.entries.drain(..) {
            if targets.contains(&e.id) {
                if insert_pos.is_none() {
                    insert_pos = Some(next_entries.len());
                }
                before_tokens += e.tokens;
                archived.push((e.label.clone(), e.tokens));
                self.retired_entries.push(e);
            } else {
                next_entries.push(e);
            }
        }

        let count = archived.len();
        if count == 0 {
            self.entries = next_entries;
            return false;
        }

        self.summary_seq += 1;
        let after_tokens = ((before_tokens as f64 * 0.055).round() as u64).clamp(1_200, 9_600);
        let summary_id = SharedString::from(format!("summary-{}", self.summary_seq));
        let summary_entry = ContextEntry::new(
            summary_id.clone(),
            ContextEntryKind::Summary,
            format!("Compacted summary · {count} entries"),
            after_tokens,
        )
        .with_label_zh(format!("已压缩摘要 · 合并 {count} 条记录"))
        .with_summary_origin(before_tokens, archived);

        let pos = insert_pos
            .unwrap_or(next_entries.len())
            .min(next_entries.len());
        next_entries.insert(pos, summary_entry);
        self.entries = next_entries;
        self.expanded_id = Some(summary_id.clone());

        cx.emit(ContextCompactionEvent::Compacted {
            summary_id,
            before_tokens,
            after_tokens,
            count,
        });
        cx.notify();
        true
    }

    /// Append a new simulated turn to grow the context window again.
    pub fn append_sample_turn(&mut self, cx: &mut Context<Self>) {
        self.turn_seq += 1;
        let seq = self.turn_seq;
        let entry = match seq % 3 {
            1 => ContextEntry::new(
                format!("turn-tool-{seq}"),
                ContextEntryKind::ToolResult,
                format!("run_tests · checkout_rounding_{seq}.spec.ts"),
                24_600,
            )
            .with_label_zh(format!("run_tests · 结账舍入单测 #{seq}")),
            2 => ContextEntry::new(
                format!("turn-file-{seq}"),
                ContextEntryKind::File,
                format!("src/cart/discounts_{seq}.ts"),
                19_200,
            ),
            _ => ContextEntry::new(
                format!("turn-agent-{seq}"),
                ContextEntryKind::Assistant,
                "Patched integer cents accumulator and verified tax split",
                6_400,
            )
            .with_label_zh("已修复整数分累加器并核验税费拆分逻辑"),
        };
        self.entries.push(entry.clone());
        cx.emit(ContextCompactionEvent::TurnAppended(entry));
        cx.notify();
    }

    /// Restore the pre-compaction entry list.
    pub fn reset_context(&mut self, cx: &mut Context<Self>) {
        self.entries = self.initial_entries.clone();
        self.retired_entries.clear();
        self.compacting_ids.clear();
        self.compacting_started = None;
        self.expanded_id = None;
        cx.emit(ContextCompactionEvent::Reset);
        cx.notify();
    }
}

impl Render for ContextCompaction {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;

        // Check if shimmer stage has completed -> transition smoothly into folded summary!
        if let Some(started) = self.compacting_started {
            if reduced || started.elapsed().as_secs_f32() >= SHIMMER_PHASE_SECS {
                self.finish_compaction(cx);
            } else {
                w.request_animation_frame();
            }
        } else if !reduced && self.loading {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };

        let used = self.used_tokens();
        let pct_f = ((used as f64 / self.limit.max(1) as f64) * 100.0).clamp(0.0, 100.0);
        let over_threshold = (used as f32 / self.limit.max(1) as f32) >= self.threshold;
        let is_compacting = self.compacting_started.is_some();

        let title_str = match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()).to_string(),
            None => self.title.to_string(),
        };

        let header = div()
            .mb(px(11.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .truncate()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .when_some(self.model.clone(), |bar, mdl| {
                bar.child(
                    div()
                        .h(px(20.0))
                        .px(px(6.5))
                        .rounded(px(5.5))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child(mdl),
                )
            })
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(is_compacting, |st| {
                        st.child(
                            div()
                                .h(px(20.0))
                                .px(px(7.0))
                                .rounded(px(5.5))
                                .bg(p.accent_soft)
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(pro_agent_avatar(
                                    "compact",
                                    p.accent_info,
                                    14.0,
                                    true,
                                    self.started_at.elapsed().as_secs_f32() * 1000.0,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.accent_info)
                                        .child(tr(cx, "Compacting", "压缩中")),
                                ),
                        )
                    })
                    .when(!is_compacting && over_threshold, |st| {
                        st.child(
                            div()
                                .h(px(20.0))
                                .px(px(7.0))
                                .rounded(px(5.5))
                                .bg(p.amber_soft)
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.amber_ink)
                                .child(tr(cx, "Near limit", "临近阈值")),
                        )
                    })
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "pct-readout"),
                        pct_f,
                        0,
                        "",
                        "%",
                        12.5,
                        FontWeight::SEMIBOLD,
                        if over_threshold { p.amber_ink } else { p.ink },
                    )),
            );

        if self.loading {
            let phase = (self.started_at.elapsed().as_secs_f32() * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(520.0))
                        .p(px(16.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(header)
                        .child(pro_skeleton_bar(480.0, 8.0, 3.0, phase, cx))
                        .child(pro_skeleton_bar(480.0, 28.0, 6.0, phase + 0.15, cx))
                        .child(pro_skeleton_bar(480.0, 28.0, 6.0, phase + 0.30, cx))
                        .child(pro_skeleton_bar(480.0, 28.0, 6.0, phase + 0.45, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.entries.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(520.0))
                        .p(px(16.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(pro_empty_state(
                            ProIcon::Memory,
                            tr(cx, "Empty context window", "上下文窗口为空"),
                            tr(
                                cx,
                                "Add system instructions, files, or turns to inspect slice usage and compaction.",
                                "载入系统提示词、文件或对话轮次即可观察上下文占用与摘要压缩。",
                            ),
                            Some((
                                pro_eid(&self.id, "seed-ctx"),
                                SharedString::from(tr(cx, "Load session context", "加载会话上下文")),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    this.entries = default_entries();
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        let active_highlight_id: Option<SharedString> = self.hovered_id.clone().or_else(|| {
            self.focused_idx
                .and_then(|idx| self.entries.get(idx))
                .map(|e| e.id.clone())
        });

        // Top Segmented Window Bar + Dashed Auto-Compact Threshold Mark
        let meter = self.render_segmented_meter(active_highlight_id.as_ref(), w, &p, cx);

        // Metadata line (`164.8K of 200K` | `35.2K left · compacts at 160K`)
        let left_tokens = self.limit.saturating_sub(used);
        let threshold_tokens = (self.limit as f32 * self.threshold).round() as u64;
        let saved_tokens: u64 = self
            .entries
            .iter()
            .filter_map(|e| e.before_tokens.map(|b| b.saturating_sub(e.tokens)))
            .sum();

        let right_meta = if saved_tokens > 0 {
            if tr(cx, "en", "zh") == "zh" {
                format!(
                    "剩余 {} · 已节省 {}",
                    format_tokens_compact(left_tokens),
                    format_tokens_compact(saved_tokens)
                )
            } else {
                format!(
                    "{} left · saved {}",
                    format_tokens_compact(left_tokens),
                    format_tokens_compact(saved_tokens)
                )
            }
        } else if tr(cx, "en", "zh") == "zh" {
            format!(
                "剩余 {} · {} 自动压缩",
                format_tokens_compact(left_tokens),
                format_tokens_compact(threshold_tokens)
            )
        } else {
            format!(
                "{} left · compacts at {}",
                format_tokens_compact(left_tokens),
                format_tokens_compact(threshold_tokens)
            )
        };

        let meta_row = div()
            .mt(px(4.0))
            .mb(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.0))
            .text_size(px(12.0))
            .text_color(p.muted)
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(4.0))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "used-k-num"),
                        used as f64 / 1_000.0,
                        1,
                        "",
                        "K",
                        12.0,
                        FontWeight::SEMIBOLD,
                        p.ink,
                    ))
                    .child(format!(
                        "{} {}",
                        tr(cx, "of", "/"),
                        format_tokens_compact(self.limit)
                    )),
            )
            .child(div().truncate().child(right_meta));

        // Ordered Entry List (`Slide` fold on retired entries + active rows)
        let list_section = self.render_entries_list(active_highlight_id.as_ref(), &p, cx);

        // Footer Legend & `Compact now` / `+ Turn` / `Reset` controls
        let can_compact = !is_compacting && !self.compactable_ids().is_empty();
        let has_summary = self
            .entries
            .iter()
            .any(|e| e.kind == ContextEntryKind::Summary);

        let footer = div()
            .mt(px(8.0))
            .pt(px(9.0))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.0))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(swatch_key(p.pink, tr(cx, "You", "用户")))
                    .child(swatch_key(p.blue, tr(cx, "Agent", "智能体")))
                    .child(swatch_key(p.teal, tr(cx, "Tools", "工具")))
                    .child(swatch_key(p.purple, tr(cx, "Files", "文件")))
                    .child(swatch_key(p.green, tr(cx, "Summary", "摘要"))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .when(has_summary, |d| {
                        d.child(pro_btn(
                            pro_eid(&self.id, "reset-ctx-btn"),
                            tr(cx, "Reset", "还原"),
                            Some(ProIcon::Undo),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| {
                                this.reset_context(cx);
                            },
                        ))
                    })
                    .child(pro_btn(
                        pro_eid(&self.id, "add-turn-btn"),
                        tr(cx, "+ Turn", "+ 轮次"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.append_sample_turn(cx);
                        },
                    ))
                    .child(
                        div()
                            .opacity(if can_compact { 1.0 } else { 0.45 })
                            .child(pro_btn(
                                pro_eid(&self.id, "compact-now-btn"),
                                if is_compacting {
                                    tr(cx, "Compacting…", "压缩中…")
                                } else {
                                    tr(cx, "Compact now", "立即压缩")
                                },
                                Some(ProIcon::Sparkles),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.start_compact(cx);
                                },
                            )),
                    ),
            );

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(520.0))
                    .p(px(16.0))
                    .rounded(px(10.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_sm())
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(meter)
                    .child(meta_row)
                    .child(list_section)
                    .child(footer),
            )
            .into_any_element()
    }
}

impl ContextCompaction {
    fn render_segmented_meter(
        &self,
        active_id: Option<&SharedString>,
        w: &mut Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let limit_f = self.limit.max(1) as f32;
        let mut track = div()
            .w_full()
            .h(px(8.0))
            .rounded(px(3.5))
            .bg(p.chip)
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(1.0));

        // Render active entries AND squeezing-out retired entries (which animate smoothly to 0 width via `Slide`!)
        for entry in &self.entries {
            let eid = entry.id.clone();
            let target_frac = (entry.tokens as f32 / limit_f).clamp(0.006, 1.0);
            let frac = pro_spring_f32(
                pro_eid(&self.id, format!("slice-{eid}")),
                target_frac,
                0.38,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let is_hot = active_id == Some(&eid);
            let dimmed = active_id.is_some() && !is_hot;
            let col = entry.kind.slice_color(p);

            track = track.child(
                div()
                    .id(pro_eid(&self.id, format!("bar-seg-{eid}")))
                    .h_full()
                    .w(relative(frac))
                    .bg(col)
                    .opacity(if dimmed { 0.35 } else { 1.0 })
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_id = Some(eid.clone());
                        } else if this.hovered_id.as_ref() == Some(&eid) {
                            this.hovered_id = None;
                        }
                        cx.notify();
                    })),
            );
        }

        // Retired entries squeeze down smoothly to `0.0` width on the bar via `Slide`
        for ret in &self.retired_entries {
            let rid = ret.id.clone();
            let squeezed =
                pro_spring_f32(pro_eid(&self.id, format!("slice-{rid}")), 0.0, 0.36, w, cx)
                    .clamp(0.0, 1.0);
            if squeezed > 0.002 {
                track = track.child(
                    div()
                        .h_full()
                        .w(relative(squeezed))
                        .bg(ret.kind.slice_color(p).opacity(0.55)),
                );
            }
        }

        // Dashed vertical auto-compact threshold mark
        let thresh_ratio = self.threshold.clamp(0.1, 0.98);
        let amber_mark = p.amber;
        let threshold_overlay = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let x = bounds.left() + bounds.size.width * thresh_ratio;
                let y0 = f32::from(bounds.top());
                let y1 = f32::from(bounds.bottom());
                let mut dash_b = PathBuilder::stroke(px(1.25));
                let mut cur_y = y0;
                while cur_y < y1 {
                    let end_y = (cur_y + 3.0).min(y1);
                    dash_b.move_to(point(x, px(cur_y)));
                    dash_b.line_to(point(x, px(end_y)));
                    cur_y += 5.0;
                }
                if let Ok(path) = dash_b.build() {
                    window.paint_path(path, amber_mark);
                }
            },
        )
        .absolute()
        .inset_0();

        div()
            .id(pro_eid(&self.id, "ctx-meter"))
            .role(Role::Image)
            .aria_label(format!(
                "{} / {} tokens",
                format_tokens_compact(self.used_tokens()),
                format_tokens_compact(self.limit)
            ))
            .py(px(3.0))
            .relative()
            .child(track)
            .child(threshold_overlay)
    }

    fn render_entries_list(
        &self,
        active_id: Option<&SharedString>,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let len = self.entries.len();
        let mut rows_col = div().mx(px(-6.0)).flex().flex_col().gap(px(1.5));

        for (idx, entry) in self.entries.iter().enumerate() {
            let eid = entry.id.clone();
            let is_hot = active_id == Some(&eid);
            let is_shimmering = self.compacting_ids.contains(&eid);
            let is_expandable = !entry.apps.is_empty() || entry.kind == ContextEntryKind::Summary;
            let is_expanded = self.expanded_id.as_ref() == Some(&eid);

            let lead_icon = self.render_entry_icon(entry, p, cx);
            let label_str = entry.display_label(cx).to_string();
            let tok_str = format_tokens_compact(entry.tokens);
            let kind_lbl = entry.kind.kind_name(cx);
            let hover_bg = p.hover;

            let label_el: AnyElement = if is_shimmering {
                pro_shimmer_text(
                    pro_eid(&self.id, format!("row-shim-{eid}")),
                    label_str.clone(),
                    p.muted,
                    p.accent_info,
                    13.0,
                    FontWeight::NORMAL,
                    true,
                )
            } else {
                div()
                    .truncate()
                    .text_size(px(13.0))
                    .font_weight(if entry.kind == ContextEntryKind::Summary {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(
                        if matches!(
                            entry.kind,
                            ContextEntryKind::System | ContextEntryKind::Tools
                        ) {
                            p.text
                        } else {
                            p.ink
                        },
                    )
                    .child(label_str.clone())
                    .into_any_element()
            };

            let mut app_badges = div().flex().items_center();
            for (a_i, app_key) in entry.apps.iter().take(3).enumerate() {
                app_badges =
                    app_badges.child(div().when(a_i > 0, |d| d.ml(px(-4.0))).child(pro_app_tile(
                        app_key.as_ref(),
                        18.0,
                        cx,
                    )));
            }
            if entry.extra_apps > 0 {
                app_badges = app_badges.child(
                    div()
                        .ml(px(4.0))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(format!("+{}", entry.extra_apps)),
                );
            }

            let row_line = div()
                .id(pro_eid(&self.id, format!("ctx-row-{eid}")))
                .role(Role::ListBoxOption)
                .aria_label(format!("{kind_lbl}: {label_str}, {tok_str} tokens"))
                .aria_selected(is_hot)
                .h(px(32.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .when(is_hot, |d| d.bg(hover_bg))
                .hover(move |s| s.bg(hover_bg))
                .when(is_expandable, |d| d.cursor_pointer())
                .flex()
                .items_center()
                .gap(px(8.0))
                .on_hover(cx.listener({
                    let eid = eid.clone();
                    move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_id = Some(eid.clone());
                            this.focused_idx = Some(idx);
                        } else if this.hovered_id.as_ref() == Some(&eid) {
                            this.hovered_id = None;
                        }
                        cx.notify();
                    }
                }))
                .on_click(cx.listener({
                    let eid = eid.clone();
                    move |this, _, _, cx| {
                        if is_expandable {
                            if this.expanded_id.as_ref() == Some(&eid) {
                                this.expanded_id = None;
                            } else {
                                this.expanded_id = Some(eid.clone());
                            }
                            cx.notify();
                        }
                    }
                }))
                .child(lead_icon)
                .child(div().min_w_0().overflow_hidden().child(label_el))
                .when(!entry.apps.is_empty(), |d| d.child(app_badges))
                .when_some(entry.before_tokens, |d, bef| {
                    d.child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(tr(cx, "was", "原"))
                            .child(
                                div()
                                    .line_through()
                                    .text_color(p.faint)
                                    .child(format_tokens_compact(bef)),
                            ),
                    )
                })
                .child(
                    div()
                        .ml_auto()
                        .flex_none()
                        .text_size(px(12.0))
                        .text_color(if entry.kind == ContextEntryKind::Summary {
                            p.green_ink
                        } else {
                            p.muted
                        })
                        .child(tok_str),
                );

            let detail_drawer = self.render_entry_drawer(entry, is_expanded, p, cx);

            // When this row is the newly created Summary, also fold out the retired rows smoothly via `pro_fold`
            let retired_fold =
                if entry.kind == ContextEntryKind::Summary && !self.retired_entries.is_empty() {
                    let count_ret = self.retired_entries.len();
                    pro_fold(
                        pro_eid(&self.id, format!("ret-squeeze-{}", entry.id)),
                        false,
                        count_ret as f32 * 28.0,
                        0.0,
                        || div().into_any_element(),
                    )
                } else {
                    div().into_any_element()
                };

            rows_col = rows_col.child(
                div()
                    .flex()
                    .flex_col()
                    .child(row_line)
                    .child(retired_fold)
                    .child(detail_drawer),
            );
        }

        div()
            .id(pro_eid(&self.id, "entries-listbox"))
            .role(Role::ListBox)
            .aria_label(tr(cx, "Entries in the context", "上下文窗口条目列表"))
            .tab_index(0)
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                if len == 0 {
                    return;
                }
                let cur = this.focused_idx.unwrap_or(0).min(len - 1);
                match ev.keystroke.key.as_str() {
                    "down" | "arrowdown" => {
                        let next = (cur + 1).min(len - 1);
                        this.focused_idx = Some(next);
                        this.hovered_id = Some(this.entries[next].id.clone());
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" | "arrowup" => {
                        let prev = cur.saturating_sub(1);
                        this.focused_idx = Some(prev);
                        this.hovered_id = Some(this.entries[prev].id.clone());
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "home" => {
                        this.focused_idx = Some(0);
                        this.hovered_id = Some(this.entries[0].id.clone());
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "end" => {
                        this.focused_idx = Some(len - 1);
                        this.hovered_id = Some(this.entries[len - 1].id.clone());
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter" | "space" | " " => {
                        let eid = this.entries[cur].id.clone();
                        if this.expanded_id.as_ref() == Some(&eid) {
                            this.expanded_id = None;
                        } else {
                            this.expanded_id = Some(eid);
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" if this.expanded_id.is_some() => {
                        this.expanded_id = None;
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(rows_col)
    }

    fn render_entry_icon(&self, entry: &ContextEntry, p: &ProPalette, cx: &App) -> AnyElement {
        let c = entry.kind.slice_color(p);
        match entry.kind {
            ContextEntryKind::System => pro_icon(ProIcon::Settings, 15.0, c),
            ContextEntryKind::Tools => pro_icon(ProIcon::Sliders, 15.0, c),
            ContextEntryKind::User => pro_icon(ProIcon::User, 15.0, p.ink),
            ContextEntryKind::Assistant => pro_agent_avatar("agent", c, 16.0, false, 0.0, cx),
            ContextEntryKind::ToolResult => pro_icon(ProIcon::Search, 15.0, c),
            ContextEntryKind::File => div()
                .w(px(16.0))
                .h(px(18.0))
                .rounded(px(3.0))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(7.5))
                .font_weight(FontWeight::BOLD)
                .text_color(c)
                .child("TS")
                .into_any_element(),
            ContextEntryKind::Summary => pro_icon(ProIcon::Sparkles, 15.0, c),
        }
    }

    fn render_entry_drawer(
        &self,
        entry: &ContextEntry,
        open: bool,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        if entry.kind == ContextEntryKind::Summary && !entry.archived.is_empty() {
            let archived = entry.archived.clone();
            let before = entry.before_tokens.unwrap_or(entry.tokens);
            let after = entry.tokens;
            let head_lbl = format!(
                "{} ({} → {})",
                tr(cx, "Compacted turns", "已归档条目"),
                format_tokens_compact(before),
                format_tokens_compact(after)
            );
            let subtle = p.subtle;
            let line = p.line;
            let ink = p.ink;
            let text = p.text;
            let muted = p.muted;

            return pro_fold(
                pro_eid(&self.id, format!("sum-drawer-{}", entry.id)),
                open,
                118.0,
                0.0,
                move || {
                    let mut box_el = div()
                        .mx(px(8.0))
                        .my(px(4.0))
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(subtle)
                        .border_1()
                        .border_color(line)
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(ink)
                                .child(head_lbl),
                        );
                    for (lbl, tok) in archived {
                        box_el = box_el.child(
                            div()
                                .h(px(20.0))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.0))
                                .text_size(px(11.5))
                                .child(div().truncate().text_color(text).child(lbl))
                                .child(
                                    div()
                                        .flex_none()
                                        .text_color(muted)
                                        .child(format_tokens_compact(tok)),
                                ),
                        );
                    }
                    box_el.into_any_element()
                },
            );
        }

        if !entry.apps.is_empty() {
            let apps = entry.apps.clone();
            let extra = entry.extra_apps;
            let subtle = p.subtle;
            let line = p.line;
            let text = p.text;
            let muted = p.muted;
            let more_lbl = format!("+{extra} {}", tr(cx, "built-in tools", "项内置工具"));

            return pro_fold(
                pro_eid(&self.id, format!("apps-drawer-{}", entry.id)),
                open,
                64.0,
                0.0,
                move || {
                    let mut row = div().flex().flex_wrap().items_center().gap(px(10.0));
                    for app_name in apps {
                        row = row.child(
                            div()
                                .px(px(7.0))
                                .py(px(3.0))
                                .rounded(px(5.0))
                                .border_1()
                                .border_color(line)
                                .text_size(px(11.5))
                                .text_color(text)
                                .child(app_name),
                        );
                    }
                    if extra > 0 {
                        row =
                            row.child(div().text_size(px(11.5)).text_color(muted).child(more_lbl));
                    }
                    div()
                        .mx(px(8.0))
                        .my(px(4.0))
                        .p(px(9.0))
                        .rounded(px(8.0))
                        .bg(subtle)
                        .border_1()
                        .border_color(line)
                        .child(row)
                        .into_any_element()
                },
            );
        }

        div().into_any_element()
    }
}

fn swatch_key(color: Hsla, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.5))
        .child(div().size(px(8.0)).rounded(px(2.0)).bg(color))
        .child(label.to_string())
}

fn default_entries() -> Vec<ContextEntry> {
    vec![
        ContextEntry::new("sys", ContextEntryKind::System, "System prompt", 3_200)
            .with_label_zh("系统提示词"),
        ContextEntry::new("tools", ContextEntryKind::Tools, "14 tools", 9_800)
            .with_label_zh("14 项工具定义")
            .with_apps(
                vec![
                    SharedString::from("github"),
                    SharedString::from("linear"),
                    SharedString::from("stripe"),
                ],
                11,
            ),
        ContextEntry::new(
            "u1",
            ContextEntryKind::User,
            "The cart total is off by a cent – can you fix it?",
            1_400,
        )
        .with_label_zh("购物车总价总是差一美分，能排查修复吗？"),
        ContextEntry::new(
            "a1",
            ContextEntryKind::Assistant,
            "Looking at how line items and tax rounding add up",
            4_600,
        )
        .with_label_zh("正在检查商品行小计与税率舍入累加逻辑"),
        ContextEntry::new(
            "tr1",
            ContextEntryKind::ToolResult,
            "search_code · cart total & tax rounding",
            48_200,
        ),
        ContextEntry::new(
            "f-legacy",
            ContextEntryKind::File,
            "src/cart/pricing_legacy.ts",
            36_400,
        ),
        ContextEntry::new(
            "tr2",
            ContextEntryKind::ToolResult,
            "run_tests · 48 vitest suites",
            42_800,
        ),
        ContextEntry::new(
            "f-active",
            ContextEntryKind::File,
            "src/cart/cart.ts",
            18_400,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ContextCompaction, ContextEntryKind, format_tokens_compact};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn token_formatter_renders_compact_units() {
        assert_eq!(format_tokens_compact(400), "400");
        assert_eq!(format_tokens_compact(3_200), "3.2K");
        assert_eq!(format_tokens_compact(200_000), "200K");
    }

    #[gpui::test]
    fn compaction_preserves_system_tools_and_active_file_while_saving_tokens(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let view = cx.new(|cx| ContextCompaction::new("test-compaction", cx));

        view.update(cx, |v, cx| {
            let before = v.used_tokens();
            assert_eq!(before, 164_800);
            assert_eq!(v.compactable_ids().len(), 5);

            assert!(v.finish_compaction(cx));
            let after = v.used_tokens();
            assert!(
                after < 45_000,
                "expected dramatic token reduction, got {after}"
            );
            assert_eq!(v.entries()[0].kind, ContextEntryKind::System);
            assert_eq!(v.entries()[1].kind, ContextEntryKind::Tools);
            assert_eq!(v.entries()[2].kind, ContextEntryKind::Summary);
            assert_eq!(
                v.entries().last().map(|e| e.id.as_ref()),
                Some("f-active"),
                "newest active file stays uncompacted"
            );

            v.append_sample_turn(cx);
            assert!(v.used_tokens() > after);
        });
    }
}
