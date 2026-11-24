//! Collapsible AI working panel with live timer, brand source pills, and step-by-step trace.
//!
//! Shows an assistant's live reasoning trail: a shimmering `"Thinking 0:08"`
//! header that collapses via `pro_fold`, searched workspace/web source pills
//! with brand icons and URL previews, progressive step arrival, and a
//! `Replay` / `Done` toggle.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_popover_presence, pro_shimmer_text, pro_skeleton_bar,
        pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::collections::BTreeSet;
use web_time::Instant;

/// Visual node type for a [`ThinkingStep`] in [`ThinkingState`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThinkingStepKind {
    /// Globe node with searched brand source pills underneath.
    Search,
    /// Reasoning or synthesis milestone node.
    #[default]
    Step,
}

/// Branded source pill revealed inside a [`ThinkingStepKind::Search`] step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThinkingSourcePill {
    /// Short domain or workspace source label (such as `"notion.so"` or `"#launch-q4"`).
    pub label: SharedString,
    /// Brand lookup key passed to `pro_app_tile` (`"notion"`, `"slack"`, `"github"`, `"linear"`).
    pub brand: SharedString,
    /// Optional URL displayed in the hover/focus preview tooltip.
    pub href: Option<SharedString>,
}

impl ThinkingSourcePill {
    /// Construct a [`ThinkingSourcePill`] with `label` and `brand` icon key.
    pub fn new(label: impl Into<SharedString>, brand: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            brand: brand.into(),
            href: None,
        }
    }

    /// Attach a destination URL previewed on hover or focus.
    pub fn with_href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }
}

/// Ordered search or reasoning step revealed by [`ThinkingState`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThinkingStep {
    /// Search vs standard reasoning step.
    pub kind: ThinkingStepKind,
    /// Step description.
    pub label: SharedString,
    /// Branded source pills (used by [`ThinkingStepKind::Search`]).
    pub sources: Vec<ThinkingSourcePill>,
    /// Number of source pills shown before collapsing the rest behind `"+N more"`.
    pub visible_source_limit: usize,
}

impl ThinkingStep {
    /// Construct a plain reasoning [`ThinkingStep`].
    pub fn step(label: impl Into<SharedString>) -> Self {
        Self {
            kind: ThinkingStepKind::Step,
            label: label.into(),
            sources: Vec::new(),
            visible_source_limit: 4,
        }
    }

    /// Construct a [`ThinkingStepKind::Search`] step with branded source pills.
    pub fn search(label: impl Into<SharedString>, sources: Vec<ThinkingSourcePill>) -> Self {
        Self {
            kind: ThinkingStepKind::Search,
            label: label.into(),
            sources,
            visible_source_limit: 4,
        }
    }

    /// Override how many source pills appear before the `"+N more"` disclosure pill.
    pub fn with_visible_limit(mut self, limit: usize) -> Self {
        self.visible_source_limit = limit.max(1);
        self
    }
}

/// Typed events emitted by [`ThinkingState`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThinkingStateEvent {
    /// Collapsible step list expanded (`true`) or folded (`false`).
    ExpandedChanged(bool),
    /// All reasoning steps finished after `elapsed_seconds`.
    Done { elapsed_seconds: u64 },
    /// User restarted the thinking sequence via `Replay`.
    Replayed,
    /// User clicked a searched source pill.
    SourceOpened(ThinkingSourcePill),
    /// User clicked `"+N more"` on a search step to reveal remaining sources.
    MoreSourcesExpanded { step_index: usize },
}

/// Format elapsed seconds as tabular `m:ss` (for example `8` -> `"0:08"`).
pub fn format_thinking_timer(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}

fn default_steps() -> Vec<ThinkingStep> {
    vec![
        ThinkingStep::search(
            "Searched workspace & web",
            vec![
                ThinkingSourcePill::new("Notion · Q4 Launch Spec", "notion")
                    .with_href("https://notion.so/kense/q4-launch-spec"),
                ThinkingSourcePill::new("Slack · #release-readiness", "slack")
                    .with_href("https://slack.com/archives/C08RELEASE"),
                ThinkingSourcePill::new("GitHub · PR #418 benchmarks", "github")
                    .with_href("https://github.com/kense-labs/kui/pull/418"),
                ThinkingSourcePill::new("Linear · KUI-209 blockers", "linear")
                    .with_href("https://linear.app/kense/issue/KUI-209"),
                ThinkingSourcePill::new("Drive · Latency matrix.xlsx", "google drive")
                    .with_href("https://drive.google.com/file/d/latency-matrix"),
                ThinkingSourcePill::new("Stripe · Billing webhook logs", "stripe")
                    .with_href("https://dashboard.stripe.com/webhooks"),
                ThinkingSourcePill::new("Figma · Chat chrome v4", "figma")
                    .with_href("https://figma.com/file/kui-chat-chrome"),
            ],
        )
        .with_visible_limit(4),
        ThinkingStep::step("Reading 7 sources on Q4 launch readiness"),
        ThinkingStep::step("Cross-checking p95 latency budgets against open Linear blockers"),
        ThinkingStep::step("Synthesizing rollout checklist and cited recommendations"),
    ]
}

fn render_pixel_mark(color: Hsla, working: bool, elapsed_ms: f32, reduced: bool) -> Div {
    let cells = agent_pixel_cells("kense-think");
    let mut grid = div()
        .size(px(16.))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(1.5));
    for r in 0..3usize {
        let mut row = div().flex().items_center().justify_center().gap(px(1.5));
        for c in 0..3usize {
            let idx = r * 3 + c;
            let on = cells[idx];
            let alpha = if working && !reduced {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[idx]) / 1200.0) * std::f32::consts::TAU;
                let wave = 0.55 + 0.45 * phase.cos();
                if on { wave } else { 0.15 * wave }
            } else if on {
                1.0
            } else {
                0.0
            };
            row = row.child(
                div()
                    .size(px(3.2))
                    .rounded(px(0.8))
                    .bg(color.opacity(alpha)),
            );
        }
        grid = grid.child(row);
    }
    grid
}

/// Collapsible AI working trace with live `0:08` timer, brand source pills, and progressive steps.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::thinking_state::{ThinkingState, ThinkingStateEvent};
///
/// fn mount(cx: &mut gpui::App) {
///     let panel = cx.new(|cx| {
///         ThinkingState::new("assistant-thinking", cx)
///             .with_done_label("Finished thinking")
///             .with_step_ms(1300)
///     });
///     cx.subscribe(&panel, |_, ev: &ThinkingStateEvent, _| {
///         let _ = ev;
///     })
///     .detach();
/// }
/// ```
pub struct ThinkingState {
    id: ElementId,
    focus: FocusHandle,
    steps: Vec<ThinkingStep>,
    done_label: SharedString,
    step_ms: u64,
    loop_playback: bool,
    expanded: bool,
    started_at: Instant,
    base_offset_ms: u64,
    completed_secs: Option<u64>,
    done_emitted: bool,
    done_settled_at: Option<Instant>,
    expanded_more_steps: BTreeSet<usize>,
    hovered_source_tip: Option<(usize, usize, SharedString)>,
    last_source_tip: Option<(usize, SharedString)>,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
}

impl EventEmitter<ThinkingStateEvent> for ThinkingState {}

impl ThinkingState {
    /// Create a [`ThinkingState`] panel preloaded with workspace search pills and 4 reasoning steps.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            steps: default_steps(),
            done_label: "Finished thinking".into(),
            step_ms: 1600,
            loop_playback: false,
            expanded: true,
            started_at: Instant::now(),
            // Start 2.4s into the run so frame 0 shows searched brand pills + active step immediately,
            // then ticks live up to 0:08 as the remaining steps arrive!
            base_offset_ms: 2400,
            completed_secs: None,
            done_emitted: false,
            done_settled_at: None,
            expanded_more_steps: BTreeSet::new(),
            hovered_source_tip: None,
            last_source_tip: None,
            loading: false,
            empty: false,
            toast: None,
        }
    }

    /// Supply custom [`ThinkingStep`] items.
    pub fn with_steps(mut self, steps: Vec<ThinkingStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Override the final completion step label (`"Finished thinking"` by default).
    pub fn with_done_label(mut self, label: impl Into<SharedString>) -> Self {
        self.done_label = label.into();
        self
    }

    /// Set the interval in milliseconds between consecutive steps (`1600` by default).
    pub fn with_step_ms(mut self, step_ms: u64) -> Self {
        self.step_ms = step_ms.clamp(250, 8000);
        self
    }

    /// Automatically restart a few seconds after finishing when `true`.
    pub fn with_loop(mut self, loop_playback: bool) -> Self {
        self.loop_playback = loop_playback;
        self
    }

    /// Set whether the collapsible step trail starts open (`true` by default).
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Start immediately in the finished (`0:08`) state when `true`.
    pub fn with_completed(mut self, completed: bool) -> Self {
        if completed {
            let total_secs = ((self.steps.len() as u64 * self.step_ms) / 1000).max(8);
            self.completed_secs = Some(total_secs);
            self.done_emitted = true;
        } else {
            self.completed_secs = None;
            self.done_emitted = false;
        }
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

    /// Return whether the step list is currently expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Return whether all thinking steps have finished.
    pub fn is_done(&self) -> bool {
        self.completed_secs.is_some()
    }

    /// Return the current or final elapsed seconds shown in the header timer.
    pub fn elapsed_seconds(&self) -> u64 {
        if let Some(done_s) = self.completed_secs {
            return done_s;
        }
        (self.base_offset_ms + self.started_at.elapsed().as_millis() as u64) / 1000
    }

    /// Expand or collapse the step list (`pro_fold`).
    pub fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.emit(ThinkingStateEvent::ExpandedChanged(self.expanded));
        cx.notify();
    }

    /// Restart the live thinking sequence from `0:00` and step 1.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.started_at = Instant::now();
        self.base_offset_ms = 0;
        self.completed_secs = None;
        self.done_emitted = false;
        self.done_settled_at = None;
        self.expanded = true;
        cx.emit(ThinkingStateEvent::Replayed);
        cx.notify();
    }

    /// Jump immediately to the finished state (`0:08`).
    pub fn complete_now(&mut self, cx: &mut Context<Self>) {
        let secs = ((self.steps.len() as u64 + 1) * self.step_ms / 1000).max(8);
        self.completed_secs = Some(secs);
        self.done_settled_at = Some(Instant::now());
        if !self.done_emitted {
            self.done_emitted = true;
            cx.emit(ThinkingStateEvent::Done {
                elapsed_seconds: secs,
            });
        }
        cx.notify();
    }
}

impl Render for ThinkingState {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

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
                        .max_w(px(480.))
                        .p(px(16.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(150., 20., 5., 0.25, cx))
                        .child(pro_skeleton_bar(360., 28., 6., 0.50, cx))
                        .child(pro_skeleton_bar(280., 16., 4., 0.75, cx)),
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
                        .max_w(px(480.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Activity,
                            tr(cx, "No active reasoning trace", "暂无运行中的思考步骤"),
                            tr(
                                cx,
                                "Start a reasoning run to stream searched sources and thinking milestones.",
                                "启动推理任务后将按顺序流式呈现检索来源与思考节点。",
                            ),
                            Some((
                                pro_eid(&self.id, "start-thinking"),
                                tr(cx, "Start thinking trace", "开始思考演示").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    this.replay(cx);
                                }),
                            )),
                            cx,
                        )),
                );
        }

        let total_run_ms = (self.steps.len() as u64 + 1) * self.step_ms;
        let raw_elapsed_ms = self.base_offset_ms + self.started_at.elapsed().as_millis() as u64;

        if self.completed_secs.is_none() {
            if reduced || raw_elapsed_ms >= total_run_ms {
                let final_secs = (total_run_ms / 1000).max(8);
                self.completed_secs = Some(final_secs);
                self.done_settled_at = Some(Instant::now());
                if !self.done_emitted {
                    self.done_emitted = true;
                    cx.emit(ThinkingStateEvent::Done {
                        elapsed_seconds: final_secs,
                    });
                }
            } else {
                w.request_animation_frame();
            }
        } else if self.loop_playback
            && !reduced
            && let Some(settled) = self.done_settled_at
        {
            if settled.elapsed().as_secs_f32() >= 3.0 {
                self.replay(cx);
            } else {
                w.request_animation_frame();
            }
        }

        let is_done = self.completed_secs.is_some();
        let shown_secs = self
            .completed_secs
            .unwrap_or_else(|| (raw_elapsed_ms / 1000).min(999));
        let timer_str = format_thinking_timer(shown_secs);

        // Determine how many steps are currently revealed (`1..=steps.len()`).
        let revealed_steps = if is_done || reduced {
            self.steps.len()
        } else {
            ((raw_elapsed_ms / self.step_ms) as usize + 1).clamp(1, self.steps.len())
        };

        if let Some((si, _, ref url)) = self.hovered_source_tip {
            self.last_source_tip = Some((si, url.clone()));
        }

        // Header button (`pro_tactile` + `pro_check_draw` when Done + `pro_shimmer_text` + live `0:08` timer) + Replay/Done toggle.
        let head_lead_icon = div()
            .relative()
            .size(px(16.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(!is_done, |d| {
                d.child(render_pixel_mark(
                    p.blue,
                    true,
                    raw_elapsed_ms as f32,
                    reduced,
                ))
            })
            .child(
                div()
                    .when(!is_done, |d| d.absolute().inset_0().opacity(0.))
                    .child(pro_check_draw(
                        pro_eid(&self.id, "head-done-chk"),
                        is_done,
                        16.,
                        false,
                        p.green,
                    )),
            );

        let head_btn_inner = div()
            .h(px(36.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(9.))
            .child(head_lead_icon)
            .child(if is_done {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "head-status-lbl"),
                        tr(cx, "Worked for", "思考耗时"),
                        13.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(p.muted)
                            .child(timer_str.clone()),
                    )
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(pro_shimmer_text(
                        pro_eid(&self.id, "head-shimmer"),
                        tr(cx, "Thinking", "正在思考"),
                        p.muted,
                        p.strong,
                        13.,
                        FontWeight::MEDIUM,
                        true,
                    ))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.faint)
                            .child(timer_str.clone()),
                    )
                    .into_any_element()
            })
            .child(pro_icon(
                if self.expanded {
                    ProIcon::ChevronDown
                } else {
                    ProIcon::ChevronRight
                },
                14.,
                p.muted,
            ));

        let header_row = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div().mx(px(-10.)).child(
                    pro_tactile(
                        pro_eid(&self.id, "think-head-btn"),
                        if is_done {
                            format!("{} {timer_str}", tr(cx, "Worked for", "思考耗时"))
                        } else {
                            format!("{} {timer_str}", tr(cx, "Thinking", "思考中"))
                        },
                        8.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        false,
                        head_btn_inner,
                        cx,
                        |this, _, cx| this.toggle_expanded(cx),
                    )
                    .flex_none()
                    .aria_expanded(self.expanded),
                ),
            )
            .child(if is_done {
                pro_btn(
                    pro_eid(&self.id, "btn-replay-think"),
                    tr(cx, "Replay", "重播"),
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.replay(cx),
                )
            } else {
                pro_btn(
                    pro_eid(&self.id, "btn-finish-think"),
                    tr(cx, "Done", "完成"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.complete_now(cx),
                )
            });

        // Collapsible step timeline (`pro_fold` + `pro_tactile` pills + `pro_check_draw` completion node).
        let zh_locale = UiLanguage::current(cx) == UiLanguage::Chinese;
        let steps_snapshot: Vec<ThinkingStep> = self
            .steps
            .iter()
            .take(revealed_steps)
            .map(|st| {
                let mut cloned = st.clone();
                if zh_locale {
                    cloned.label = match cloned.label.as_ref() {
                        "Searched workspace & web" => "已检索工作区与网页来源".into(),
                        "Reading 7 sources on Q4 launch readiness" => {
                            "正在研读 7 份 Q4 发布就绪度相关资料".into()
                        }
                        "Cross-checking p95 latency budgets against open Linear blockers" => {
                            "正在交叉核对 p95 延迟预算与 Linear 阻塞项".into()
                        }
                        "Synthesizing rollout checklist and cited recommendations" => {
                            "正在汇总上线核对清单与引用建议".into()
                        }
                        _ => cloned.label,
                    };
                }
                cloned
            })
            .collect();
        let active_step_idx = if is_done {
            None
        } else {
            Some(revealed_steps.saturating_sub(1))
        };
        let done_label: SharedString =
            if zh_locale && self.done_label.as_ref() == "Finished thinking" {
                "思考完成".into()
            } else {
                self.done_label.clone()
            };
        let expanded_sets = self.expanded_more_steps.clone();
        let hovered_tip = self.hovered_source_tip.clone();
        let last_tip = self.last_source_tip.clone();
        let step_ms = self.step_ms;
        let base_id = self.id.clone();
        let more_word = tr(cx, "more", "更多");

        let body_content: AnyElement = {
            let total_rows = steps_snapshot.len() + usize::from(is_done);
            let mut list = div().pt(px(8.)).flex().flex_col();

            for (si, step) in steps_snapshot.into_iter().enumerate() {
                let is_active = active_step_idx == Some(si);
                let has_next_rail = si + 1 < total_rows;

                // Smooth fade + 6px vertical glide for newly arriving steps!
                let (step_alpha, step_dy) = if is_done || reduced {
                    (1.0, 0.0)
                } else {
                    let step_birth_ms = (si as u64) * step_ms;
                    let age_ms = raw_elapsed_ms.saturating_sub(step_birth_ms) as f32;
                    let t = pro_ease((age_ms / 380.0).clamp(0.0, 1.0));
                    (t, 6.0 * (1.0 - t))
                };

                let node_icon = match step.kind {
                    ThinkingStepKind::Search => pro_icon(ProIcon::Globe, 15., p.muted),
                    ThinkingStepKind::Step => {
                        if is_active {
                            let pulse = if reduced {
                                1.0
                            } else {
                                0.45 + 0.55 * ((raw_elapsed_ms as f32 / 160.0).sin() * 0.5 + 0.5)
                            };
                            div()
                                .size(px(6.))
                                .rounded(px(1.5))
                                .bg(p.blue.opacity(pulse))
                                .into_any_element()
                        } else {
                            div()
                                .size(px(5.))
                                .rounded_full()
                                .bg(p.blue.opacity(0.65))
                                .into_any_element()
                        }
                    }
                };

                let label_el = if is_active && !reduced {
                    pro_shimmer_text(
                        pro_eid(&base_id, format!("step-shim-{si}")),
                        step.label,
                        p.muted,
                        p.strong,
                        13.,
                        FontWeight::NORMAL,
                        true,
                    )
                } else {
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(step.label)
                        .into_any_element()
                };

                // Branded source pills (`pro_tactile`) + `"+N more"` disclosure + URL preview (`pro_popover_presence`).
                let sources_el = if !step.sources.is_empty() {
                    let show_all = expanded_sets.contains(&si);
                    let limit = if show_all {
                        step.sources.len()
                    } else {
                        step.visible_source_limit.min(step.sources.len())
                    };
                    let hidden_n = step.sources.len().saturating_sub(limit);
                    let mut pills_row = div()
                        .mt(px(7.))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(6.));

                    for (pi, pill) in step.sources.into_iter().take(limit).enumerate() {
                        let tile = pro_app_tile(&pill.brand, 14., cx);
                        let pill_clk = pill.clone();
                        let tip_url = pill.href.clone().unwrap_or_else(|| pill.label.clone());
                        let is_tip_active = hovered_tip
                            .as_ref()
                            .is_some_and(|(s_idx, p_idx, _)| *s_idx == si && *p_idx == pi);

                        let pill_inner = div()
                            .h(px(26.))
                            .pl(px(6.))
                            .pr(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .whitespace_nowrap()
                            .child(tile)
                            .child(pill.label.clone());

                        pills_row = pills_row.child(
                            pro_tactile(
                                pro_eid(&base_id, format!("src-pill-{si}-{pi}")),
                                pill.label,
                                6.,
                                p.card,
                                p.subtle,
                                p.line,
                                p.blue.opacity(0.45),
                                is_tip_active,
                                pill_inner,
                                cx,
                                move |this, _, cx| {
                                    let p_ev = pill_clk.clone();
                                    this.toast = Some((
                                        SharedString::from(format!(
                                            "{} · {}",
                                            p_ev.label,
                                            p_ev.href.as_deref().unwrap_or("workspace")
                                        )),
                                        Instant::now(),
                                    ));
                                    cx.emit(ThinkingStateEvent::SourceOpened(p_ev));
                                    cx.notify();
                                },
                            )
                            .flex_none()
                            .on_hover(cx.listener(
                                move |this, inside: &bool, _, cx| {
                                    if *inside {
                                        this.hovered_source_tip = Some((si, pi, tip_url.clone()));
                                    } else if this
                                        .hovered_source_tip
                                        .as_ref()
                                        .is_some_and(|(a, b, _)| *a == si && *b == pi)
                                    {
                                        this.hovered_source_tip = None;
                                    }
                                    cx.notify();
                                },
                            )),
                        );
                    }

                    if hidden_n > 0 {
                        let more_inner = div()
                            .h(px(26.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .whitespace_nowrap()
                            .child(format!("+{hidden_n} {more_word}"));
                        pills_row = pills_row.child(
                            pro_tactile(
                                pro_eid(&base_id, format!("more-srcs-{si}")),
                                format!("+{hidden_n} more"),
                                6.,
                                transparent_black(),
                                p.hover,
                                p.line_strong,
                                p.line_strong,
                                false,
                                more_inner,
                                cx,
                                move |this, _, cx| {
                                    this.expanded_more_steps.insert(si);
                                    cx.emit(ThinkingStateEvent::MoreSourcesExpanded {
                                        step_index: si,
                                    });
                                    cx.notify();
                                },
                            )
                            .flex_none(),
                        );
                    }

                    let live_tip = hovered_tip
                        .as_ref()
                        .filter(|(s_idx, _, _)| *s_idx == si)
                        .map(|(_, _, u)| u.clone());
                    let tip_open = live_tip.is_some();
                    let shown_tip = live_tip.or_else(|| {
                        last_tip
                            .as_ref()
                            .filter(|(s_idx, _)| *s_idx == si)
                            .map(|(_, u)| u.clone())
                    });

                    let tip_banner = pro_popover_presence(
                        pro_eid(&base_id, format!("src-tip-{si}")),
                        tip_open,
                        move || {
                            let url = shown_tip.unwrap_or_default();
                            div()
                                .mt(px(5.))
                                .px(px(8.))
                                .py(px(4.))
                                .rounded(px(6.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(pro_icon(ProIcon::Link, 11., p.blue))
                                .child(url)
                                .into_any_element()
                        },
                    );

                    Some(div().flex().flex_col().child(pills_row).child(tip_banner))
                } else {
                    None
                };

                list = list.child(
                    div()
                        .relative()
                        .top(px(step_dy))
                        .opacity(step_alpha)
                        .pb(px(12.))
                        .flex()
                        .items_start()
                        .gap(px(10.))
                        .when(has_next_rail, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .left(px(7.5))
                                    .top(px(21.))
                                    .bottom(px(2.))
                                    .w(px(1.))
                                    .bg(p.line_strong),
                            )
                        })
                        .child(
                            div()
                                .w(px(16.))
                                .h(px(20.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(node_icon),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(label_el)
                                .when_some(sources_el, |d, s| d.child(s)),
                        ),
                );
            }

            // Always keep `pro_check_draw` mounted so flipping `!is_done -> is_done` progressively draws the checkmark!
            list = list.child(
                div()
                    .when(!is_done, |d| d.h(px(0.)).overflow_hidden().opacity(0.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(20.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_check_draw(
                                pro_eid(&base_id, "step-done-chk"),
                                is_done,
                                15.,
                                false,
                                p.blue,
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(done_label),
                    ),
            );

            list.into_any_element()
        };

        let body_fold = pro_fold(
            pro_eid(&self.id, "think-body-fold"),
            self.expanded,
            230.,
            0.,
            move || body_content,
        );

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(tr(cx, "AI thinking state", "AI 思考工作面板"))
            .track_focus(&self.focus)
            .relative()
            .w_full()
            .min_w_0()
            .p(px(20.))
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(480.))
                    .p(px(16.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow_xs()
                    .flex()
                    .flex_col()
                    .child(header_row)
                    .child(body_fold),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{ThinkingStepKind, default_steps, format_thinking_timer};

    #[test]
    fn formats_tabular_minutes_and_seconds_cleanly() {
        assert_eq!(format_thinking_timer(0), "0:00");
        assert_eq!(format_thinking_timer(8), "0:08");
        assert_eq!(format_thinking_timer(65), "1:05");
        assert_eq!(format_thinking_timer(612), "10:12");
    }

    #[test]
    fn default_steps_start_with_branded_search_and_more_overflow() {
        let steps = default_steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].kind, ThinkingStepKind::Search);
        assert_eq!(steps[0].sources.len(), 7);
        assert_eq!(steps[0].visible_source_limit, 4);
    }
}
