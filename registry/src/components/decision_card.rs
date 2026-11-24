//! Structured agent question card docked above a chat composer with numbered keys, tiles, and Later fold.
//!
//! Presents an agent's blocking question directly above the composer in three interchangeable
//! layouts—numbered multiple-choice rows (`Recommended` badge), visual wireframe preview tiles,
//! or structured `Approve` / `Decline` facts—plus an inline `"Other — type your own…"` input,
//! instant `1`–`5` keyboard shortcuts, multi-decision paging, and `"Later"` one-line folding.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_popover_presence, pro_skeleton_bar,
        pro_step_transition, pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Presentation mode of an individual [`DecisionPrompt`] in [`DecisionCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecisionKind {
    /// Numbered option rows (`1`–`4`) with one-line summaries and optional `Recommended` pill.
    #[default]
    Choice,
    /// Visual wireframe preview tiles arranged in a 3-column grid (collapsing to rows below `420px`).
    Pick,
    /// Structured key-value facts paired with `Approve` and `Decline` choices.
    Approval,
}

/// Miniature wireframe artwork preset for [`DecisionKind::Pick`] tiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecisionTileWireframe {
    /// Three side-by-side plan columns.
    #[default]
    Columns,
    /// Feature comparison matrix with header strip and rows.
    Matrix,
    /// Interactive scrubber bar above a summary callout card.
    Slider,
}

/// Option selectable by click or number key inside a [`DecisionPrompt`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionOption {
    /// Stable option identifier (for example `"kafka"`).
    pub id: SharedString,
    /// Primary option title.
    pub title: SharedString,
    /// Optional single-line trade-off or consequence summary.
    pub summary: Option<SharedString>,
    /// Whether the agent marks this option as `Recommended`.
    pub recommended: bool,
    /// Wireframe preview preset used when the parent [`DecisionPrompt`] is [`DecisionKind::Pick`].
    pub wireframe: DecisionTileWireframe,
}

impl DecisionOption {
    /// Construct a [`DecisionOption`] with `id` and `title`.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            recommended: false,
            wireframe: DecisionTileWireframe::Columns,
        }
    }

    /// Attach a one-line summary under the option title.
    pub fn with_summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Mark this option with the `Recommended` badge.
    pub fn with_recommended(mut self, recommended: bool) -> Self {
        self.recommended = recommended;
        self
    }

    /// Choose the miniature wireframe preset for [`DecisionKind::Pick`] tiles.
    pub fn with_wireframe(mut self, wireframe: DecisionTileWireframe) -> Self {
        self.wireframe = wireframe;
        self
    }
}

/// Key-value fact row displayed inside [`DecisionKind::Approval`] decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionFact {
    /// Fact label (for example `"Service"`).
    pub label: SharedString,
    /// Fact value (for example `"ingest-worker · 6 replicas"`).
    pub value: SharedString,
}

impl DecisionFact {
    /// Construct a [`DecisionFact`].
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Open question queued in [`DecisionCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionPrompt {
    /// Stable decision identifier.
    pub id: SharedString,
    /// Task or workstream name shown in the card header (for example `"Ingest pipeline"`).
    pub task: SharedString,
    /// Primary question asked by the agent.
    pub question: SharedString,
    /// Visual layout variant (`Choice`, `Pick`, `Approval`).
    pub kind: DecisionKind,
    /// Selectable options (used by `Choice` and `Pick`; optional overrides for `Approval`).
    pub options: Vec<DecisionOption>,
    /// Key-value facts shown above `Approve` / `Decline` in [`DecisionKind::Approval`].
    pub facts: Vec<DecisionFact>,
}

impl DecisionPrompt {
    /// Construct a [`DecisionPrompt`] with `id`, `task`, and `question`.
    pub fn new(
        id: impl Into<SharedString>,
        task: impl Into<SharedString>,
        question: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            task: task.into(),
            question: question.into(),
            kind: DecisionKind::Choice,
            options: Vec::new(),
            facts: Vec::new(),
        }
    }

    /// Set the [`DecisionKind`] variant (`Choice`, `Pick`, `Approval`).
    pub fn with_kind(mut self, kind: DecisionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Provide selectable [`DecisionOption`] items.
    pub fn with_options(mut self, options: Vec<DecisionOption>) -> Self {
        self.options = options;
        self
    }

    /// Provide structured [`DecisionFact`] rows for [`DecisionKind::Approval`].
    pub fn with_facts(mut self, facts: Vec<DecisionFact>) -> Self {
        self.facts = facts;
        self
    }
}

/// Answer payload returned when a [`DecisionPrompt`] is resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionAnswer {
    /// User picked one of the numbered options or preview tiles (`option_id`).
    Option(SharedString),
    /// User approved a [`DecisionKind::Approval`] prompt.
    Approve,
    /// User declined a [`DecisionKind::Approval`] prompt.
    Decline,
    /// User wrote a custom instruction in the `"Other — type your own…"` row.
    Custom(SharedString),
}

/// Events emitted by [`DecisionCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionCardEvent {
    /// User answered a decision by click, number shortcut, or custom text.
    Decided {
        /// Identifier of the answered decision.
        id: SharedString,
        /// Chosen answer (`Option`, `Approve`, `Decline`, or `Custom`).
        answer: DecisionAnswer,
    },
    /// User folded the card to one summary line (`true`) or reopened it (`false`).
    FoldedChanged(bool),
    /// User paged to another open decision (`index`).
    PageChanged(usize),
    /// User sent a chat prompt from the docked composer below the card.
    ComposerSubmitted(SharedString),
    /// User replayed the decision queue in showcase mode.
    Reset,
}

impl EventEmitter<DecisionCardEvent> for DecisionCard {}

fn default_decisions() -> Vec<DecisionPrompt> {
    vec![
        DecisionPrompt::new(
            "queue",
            "Ingest pipeline",
            "Which queue should the ingest service use?",
        )
        .with_kind(DecisionKind::Choice)
        .with_options(vec![
            DecisionOption::new("kafka", "Kafka")
                .with_summary("A durable log – a day of events to replay after a bad deploy")
                .with_recommended(true),
            DecisionOption::new("sqs", "SQS")
                .with_summary("Managed, nothing to run – but no replay"),
            DecisionOption::new("pg", "Postgres queue")
                .with_summary("The database you already have – won't keep up with the bursts"),
            DecisionOption::new("redis", "Redis Streams")
                .with_summary("Sub-millisecond fan-out – memory-capped retention"),
        ]),
        DecisionPrompt::new(
            "layout",
            "Pricing page",
            "Which layout should the plan comparison section use?",
        )
        .with_kind(DecisionKind::Pick)
        .with_options(vec![
            DecisionOption::new("columns", "Three columns")
                .with_summary("Side-by-side Free, Pro, and Team cards")
                .with_recommended(true)
                .with_wireframe(DecisionTileWireframe::Columns),
            DecisionOption::new("matrix", "Feature matrix")
                .with_summary("Sticky tier headers with grouped rows")
                .with_wireframe(DecisionTileWireframe::Matrix),
            DecisionOption::new("slider", "Seat slider")
                .with_summary("Live seat scrubber with annual savings callout")
                .with_wireframe(DecisionTileWireframe::Slider),
        ]),
        DecisionPrompt::new(
            "deploy",
            "Production release",
            "Deploy ingest-worker v2.4.0 to us-east-1?",
        )
        .with_kind(DecisionKind::Approval)
        .with_facts(vec![
            DecisionFact::new("Service", "ingest-worker · 6 replicas"),
            DecisionFact::new("Checks", "142 unit + staging smoke green"),
            DecisionFact::new("Rollback", "Auto-revert if 5xx exceeds 1% in 10m"),
        ]),
    ]
}

fn render_wireframe_preview(kind: DecisionTileWireframe, p: &ProPalette) -> AnyElement {
    let page_bg: Hsla = if p.dark {
        rgb(0x1c1c1f).into()
    } else {
        rgb(0xffffff).into()
    };
    let ink_bar = p.ink.opacity(0.44);
    let mid_bar = p.ink.opacity(0.18);
    let soft_bar = p.ink.opacity(0.08);

    match kind {
        DecisionTileWireframe::Columns => div()
            .size_full()
            .p(px(6.0))
            .rounded(px(5.0))
            .bg(page_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_stretch()
            .gap(px(5.0))
            .children((0..3usize).map(|idx| {
                let featured = idx == 1;
                div()
                    .flex_1()
                    .p(px(5.0))
                    .rounded(px(4.0))
                    .bg(if featured { p.accent_soft } else { soft_bar })
                    .border_1()
                    .border_color(if featured {
                        p.blue.opacity(0.35)
                    } else {
                        p.line
                    })
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(div().w(px(22.0)).h(px(4.0)).rounded(px(2.0)).bg(ink_bar))
                    .child(div().w_full().h(px(3.0)).rounded(px(1.5)).bg(mid_bar))
                    .child(div().w(px(28.0)).h(px(3.0)).rounded(px(1.5)).bg(mid_bar))
                    .child(div().flex_1())
                    .child(div().w_full().h(px(7.0)).rounded(px(2.5)).bg(if featured {
                        p.blue
                    } else {
                        mid_bar
                    }))
            }))
            .into_any_element(),
        DecisionTileWireframe::Matrix => div()
            .size_full()
            .p(px(6.0))
            .rounded(px(5.0))
            .bg(page_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .w_full()
                    .h(px(8.0))
                    .px(px(4.0))
                    .rounded(px(2.5))
                    .bg(soft_bar)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().w(px(26.0)).h(px(3.5)).rounded(px(1.5)).bg(ink_bar))
                    .child(div().w(px(14.0)).h(px(3.5)).rounded(px(1.5)).bg(mid_bar))
                    .child(div().w(px(14.0)).h(px(3.5)).rounded(px(1.5)).bg(p.blue)),
            )
            .children((0..4usize).map(|r| {
                div()
                    .w_full()
                    .h(px(7.0))
                    .px(px(4.0))
                    .border_b_1()
                    .border_color(p.line_soft)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .w(px(if r % 2 == 0 { 34.0 } else { 26.0 }))
                            .h(px(3.0))
                            .rounded(px(1.5))
                            .bg(mid_bar),
                    )
                    .child(div().size(px(4.5)).rounded_full().bg(mid_bar))
                    .child(div().size(px(4.5)).rounded_full().bg(p.blue.opacity(0.85)))
            }))
            .into_any_element(),
        DecisionTileWireframe::Slider => div()
            .size_full()
            .p(px(7.0))
            .rounded(px(5.0))
            .bg(page_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().w(px(32.0)).h(px(4.0)).rounded(px(2.0)).bg(ink_bar))
                    .child(div().w(px(18.0)).h(px(4.0)).rounded(px(2.0)).bg(p.blue)),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(5.0))
                    .rounded_full()
                    .bg(soft_bar)
                    .child(div().w(relative(0.62)).h_full().rounded_full().bg(p.blue))
                    .child(
                        div()
                            .absolute()
                            .left(relative(0.58))
                            .top(px(-2.5))
                            .size(px(10.0))
                            .rounded_full()
                            .bg(page_bg)
                            .border_2()
                            .border_color(p.blue),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(16.0))
                    .px(px(6.0))
                    .rounded(px(4.0))
                    .bg(soft_bar)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().w(px(36.0)).h(px(3.5)).rounded(px(1.5)).bg(mid_bar))
                    .child(div().w(px(22.0)).h(px(6.0)).rounded(px(2.5)).bg(ink_bar)),
            )
            .into_any_element(),
    }
}

/// Structured agent decision card docked above a composer with numbered shortcuts, tiles, and Later fold.
pub struct DecisionCard {
    id: ElementId,
    focus: FocusHandle,
    decisions: Vec<DecisionPrompt>,
    initial_decisions: Vec<DecisionPrompt>,
    cursor: usize,
    highlighted_opt: usize,
    folded: bool,
    allow_own: bool,
    composer_chrome: bool,
    mode_menu_open: bool,
    composer_mode_idx: usize,
    model_menu_open: bool,
    composer_model_idx: usize,
    own_editor: Option<Entity<Editor>>,
    own_focused: bool,
    composer_editor: Option<Entity<Editor>>,
    composer_focused: bool,
    boot_at: Instant,
    step_serial: i32,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl DecisionCard {
    /// Create a [`DecisionCard`] preloaded with 3 open decisions (`Choice`, `Pick` tiles, `Approval` facts) above a composer.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let initial = default_decisions();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            decisions: initial.clone(),
            initial_decisions: initial,
            cursor: 0,
            highlighted_opt: 0,
            folded: false,
            allow_own: true,
            composer_chrome: true,
            mode_menu_open: false,
            composer_mode_idx: 0,
            model_menu_open: false,
            composer_model_idx: 0,
            own_editor: None,
            own_focused: false,
            composer_editor: None,
            composer_focused: false,
            boot_at: Instant::now(),
            step_serial: 0,
            measured_width: Rc::new(Cell::new(600.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Supply a custom queue of open [`DecisionPrompt`] items.
    pub fn with_decisions(mut self, decisions: Vec<DecisionPrompt>) -> Self {
        self.initial_decisions = decisions.clone();
        self.decisions = decisions;
        self.cursor = 0;
        self.highlighted_opt = 0;
        self
    }

    /// Set whether the card starts folded to the one-line summary bar (`"3 decisions waiting · Show"`).
    pub fn with_folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    /// Show or hide the bottom `"Other — type your own…"` row.
    pub fn with_allow_own(mut self, allow_own: bool) -> Self {
        self.allow_own = allow_own;
        self
    }

    /// Show or hide the surrounding status line and docked chat composer chrome.
    pub fn with_composer_chrome(mut self, composer_chrome: bool) -> Self {
        self.composer_chrome = composer_chrome;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Number of unresolved decisions remaining in the card.
    pub fn remaining_count(&self) -> usize {
        self.decisions.len()
    }

    /// Whether the card is currently folded via `"Later"`.
    pub fn is_folded(&self) -> bool {
        self.folded
    }

    fn effective_options(prompt: &DecisionPrompt, cx: &App) -> Vec<DecisionOption> {
        if prompt.kind == DecisionKind::Approval && prompt.options.is_empty() {
            vec![
                DecisionOption::new("approve", tr(cx, "Approve", "批准发布"))
                    .with_summary(tr(
                        cx,
                        "Roll out across 6 replicas with automatic canary check",
                        "滚动部署至 6 个副本并开启金丝雀健康检查",
                    ))
                    .with_recommended(true),
                DecisionOption::new("decline", tr(cx, "Decline", "暂缓发布")).with_summary(tr(
                    cx,
                    "Hold the release on staging for manual review",
                    "保留在预发环境等待人工复核",
                )),
            ]
        } else {
            prompt.options.clone()
        }
    }

    /// Resolve the active decision with `answer` and advance to the next open question.
    pub fn submit_answer(
        &mut self,
        answer: DecisionAnswer,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if self.decisions.is_empty() {
            return;
        }
        let idx = self.cursor.min(self.decisions.len() - 1);
        let removed = self.decisions.remove(idx);
        if self.cursor >= self.decisions.len() && !self.decisions.is_empty() {
            self.cursor = self.decisions.len() - 1;
        }
        self.highlighted_opt = 0;
        self.step_serial += 1;

        if let Some(win) = w
            && let Some(ed) = &self.own_editor
        {
            ed.update(cx, |e, cx| e.set_value("", win, cx));
        }

        let summary_msg: SharedString = match &answer {
            DecisionAnswer::Option(opt_id) => {
                format!("{}: {opt_id}", tr(cx, "Answered", "已选择")).into()
            }
            DecisionAnswer::Approve => tr(cx, "Approved release", "已批准发布").into(),
            DecisionAnswer::Decline => tr(cx, "Declined release", "已暂缓发布").into(),
            DecisionAnswer::Custom(txt) => format!("{}: {txt}", tr(cx, "Sent", "已发送")).into(),
        };
        self.toast = Some((summary_msg, Instant::now()));

        cx.emit(DecisionCardEvent::Decided {
            id: removed.id,
            answer,
        });
        cx.notify();
    }

    /// Fold or unfold the decision card (`"Later"` / `"Show"`).
    pub fn set_folded(&mut self, folded: bool, cx: &mut Context<Self>) {
        if self.folded != folded {
            self.folded = folded;
            cx.emit(DecisionCardEvent::FoldedChanged(folded));
            cx.notify();
        }
    }

    /// Reset all 3 decisions back into the queue.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.decisions = self.initial_decisions.clone();
        self.cursor = 0;
        self.highlighted_opt = 0;
        self.folded = false;
        self.step_serial += 1;
        cx.emit(DecisionCardEvent::Reset);
        cx.notify();
    }

    fn step_page(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.decisions.len();
        if len <= 1 {
            return;
        }
        let cur = self.cursor.min(len - 1) as isize;
        let next = (cur + delta).rem_euclid(len as isize) as usize;
        if next != self.cursor {
            self.cursor = next;
            self.highlighted_opt = 0;
            self.step_serial += if delta >= 0 { 1 } else { -1 };
            cx.emit(DecisionCardEvent::PageChanged(next));
            cx.notify();
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.allow_own && self.own_editor.is_none() {
            let ph: SharedString =
                tr(cx, "Other — type your own…", "其他 — 输入你自己的指令…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.own_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.own_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty() {
                        this.submit_answer(DecisionAnswer::Custom(val.into()), Some(w), cx);
                        this.focus.focus(w, cx);
                    }
                }
                EditEvent::Escape => {
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.own_editor = Some(ed);
        }

        if self.composer_chrome && self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Ask a follow-up or steer the running agents…",
                "继续追问或向运行中的智能体发送新指令…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, _, cx| match ev {
                EditEvent::Focused => {
                    this.composer_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.composer_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }
    }
}

impl Render for DecisionCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        self.ensure_editors(w, cx);

        if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() * 0.5).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(24.0))
                .px(px(16.0))
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(600.0))
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(pro_skeleton_bar(600.0, 220.0, 14.0, phase, cx))
                        .child(pro_skeleton_bar(600.0, 96.0, 16.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No open decisions", "暂无待回答问题"),
                    tr(
                        cx,
                        "Questions from running agents dock right above your chat composer when they need an answer.",
                        "当运行中的智能体需要你拍板选择时，问题卡片将停靠在聊天输入框正上方。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let cw = self.measured_width.get();
        let tiles_as_rows = cw < 420.0;
        let reduced = disabled_motion(cx);
        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.0;
        if !reduced && !self.decisions.is_empty() {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let well_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.035)
        } else {
            hsla(0.0, 0.0, 0.0, 0.025)
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.07)
        } else {
            hsla(0.0, 0.0, 0.0, 0.05)
        };

        let open_len = self.decisions.len();
        let cur_idx = if open_len == 0 {
            0
        } else {
            self.cursor.min(open_len - 1)
        };

        // Build the DecisionCard widget (fades/folds cleanly to zero when `open_len == 0`)
        let has_open = open_len > 0;
        let is_folded = self.folded;

        let folded_bar = if has_open {
            let wait_txt: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("{open_len} 项决策等待回答")
            } else {
                format!(
                    "{open_len} decision{} waiting",
                    if open_len == 1 { "" } else { "s" }
                )
            }
            .into();
            let show_lbl = tr(cx, "Show", "展开");
            div()
                .w_full()
                .p(px(6.0))
                .rounded(px(14.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .when(!p.dark, |d| d.shadow_sm())
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .ml(px(4.0))
                        .h(px(18.0))
                        .px(px(6.0))
                        .rounded(px(5.0))
                        .bg(p.amber_soft)
                        .text_color(p.amber_ink)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Needs you", "待你拍板")),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.5))
                        .text_color(p.text)
                        .truncate()
                        .child(wait_txt),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "show-folded-btn"),
                    show_lbl,
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.set_folded(false, cx);
                    },
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        let expanded_box =
            if let Some(prompt) = self.decisions.get(cur_idx).cloned() {
                let eff_opts = Self::effective_options(&prompt, cx);
                let opt_count = eff_opts.len();
                let own_num = opt_count + 1;

                let task_localized: SharedString = match prompt.task.as_ref() {
                    "Ingest pipeline" => tr(cx, "Ingest pipeline", "数据采集流水线").into(),
                    "Pricing page" => tr(cx, "Pricing page", "定价对比页").into(),
                    "Production release" => tr(cx, "Production release", "生产环境发布").into(),
                    _ => prompt.task.clone(),
                };
                let q_localized: SharedString = match prompt.question.as_ref() {
                    "Which queue should the ingest service use?" => tr(
                        cx,
                        "Which queue should the ingest service use?",
                        "数据采集服务应当采用哪种消息队列？",
                    )
                    .into(),
                    "Which layout should the plan comparison section use?" => tr(
                        cx,
                        "Which layout should the plan comparison section use?",
                        "套餐对比区块应当采用哪种版式？",
                    )
                    .into(),
                    "Deploy ingest-worker v2.4.0 to us-east-1?" => tr(
                        cx,
                        "Deploy ingest-worker v2.4.0 to us-east-1?",
                        "是否将 ingest-worker v2.4.0 部署至生产可用区 us-east-1？",
                    )
                    .into(),
                    _ => prompt.question.clone(),
                };

                let pager_tools = {
                    let later_lbl = tr(cx, "Later", "稍后");
                    let mut tools = div().flex().items_center().gap(px(4.0));
                    if open_len > 1 {
                        let prev_lbl = tr(cx, "Previous decision", "上一题");
                        let next_lbl = tr(cx, "Next decision", "下一题");
                        let page_str: SharedString =
                            format!("{} / {}", cur_idx + 1, open_len).into();
                        tools = tools.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "dc-prev"),
                                    prev_lbl,
                                    ProIcon::ChevronLeft,
                                    22.0,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.step_page(-1, cx);
                                    },
                                ))
                                .child(div().min_w(px(36.0)).flex().justify_center().child(
                                    pro_swap_label(
                                        pro_eid(&self.id, "dc-page-lbl"),
                                        page_str,
                                        12.0,
                                        FontWeight::NORMAL,
                                        p.muted,
                                    ),
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "dc-next"),
                                    next_lbl,
                                    ProIcon::ChevronRight,
                                    22.0,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.step_page(1, cx);
                                    },
                                )),
                        );
                    }
                    tools.child(pro_btn(
                        pro_eid(&self.id, "dc-later"),
                        later_lbl,
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.set_folded(true, cx);
                        },
                    ))
                };

                let facts_el = (!prompt.facts.is_empty()).then(|| {
                    let mut f_col = div()
                        .w_full()
                        .px(px(6.0))
                        .py(px(4.0))
                        .rounded(px(8.0))
                        .bg(well_bg)
                        .flex()
                        .flex_col()
                        .gap(px(3.0));
                    for fact in &prompt.facts {
                        f_col = f_col.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(12.0))
                                .text_size(px(12.5))
                                .child(
                                    div()
                                        .w(px(72.0))
                                        .flex_none()
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(fact.label.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(fact.value.clone()),
                                ),
                        );
                    }
                    f_col
                });

                // Options body: either Pick Tiles (above 420px) or Numbered Rows (both powered by `pro_tactile`)
                let options_el = if prompt.kind == DecisionKind::Pick && !tiles_as_rows {
                    let mut grid = div().w_full().flex().items_stretch().gap(px(6.0));
                    for (i, opt) in eff_opts.iter().enumerate() {
                        let is_hi = self.highlighted_opt == i && !self.own_focused;
                        let opt_id = opt.id.clone();
                        let rec_lbl = tr(cx, "Recommended", "推荐");
                        let tile_body = div()
                            .w_full()
                            .p(px(5.0))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .relative()
                                    .w_full()
                                    .h(px(86.0))
                                    .p(px(5.0))
                                    .rounded(px(7.0))
                                    .bg(well_bg)
                                    .border_1()
                                    .border_color(p.line)
                                    .child(render_wireframe_preview(opt.wireframe, &p))
                                    .when(opt.recommended, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .top(px(6.0))
                                                .right(px(6.0))
                                                .h(px(18.0))
                                                .px(px(6.0))
                                                .rounded(px(5.0))
                                                .bg(card_bg)
                                                .border_1()
                                                .border_color(p.line_strong)
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(rec_lbl),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .px(px(2.0))
                                    .flex()
                                    .items_start()
                                    .gap(px(7.0))
                                    .child(
                                        div()
                                            .size(px(20.0))
                                            .flex_none()
                                            .rounded(px(5.5))
                                            .bg(if is_hi { card_bg } else { fill_bg })
                                            .when(is_hi, |d| d.border_1().border_color(p.line))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if is_hi { p.ink } else { p.muted })
                                            .child(format!("{}", i + 1)),
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
                                                    .text_color(p.ink)
                                                    .truncate()
                                                    .child(opt.title.clone()),
                                            )
                                            .when_some(opt.summary.clone(), |d, sum| {
                                                d.child(
                                                    div()
                                                        .text_size(px(11.5))
                                                        .text_color(p.muted)
                                                        .truncate()
                                                        .child(sum),
                                                )
                                            }),
                                    ),
                            );

                        grid = grid.child(
                            pro_tactile(
                                pro_eid(&self.id, format!("tile-{i}")),
                                opt.title.clone(),
                                10.0,
                                transparent_black(),
                                fill_bg,
                                transparent_black(),
                                p.line,
                                is_hi,
                                tile_body,
                                cx,
                                move |this, w, cx| {
                                    this.submit_answer(
                                        DecisionAnswer::Option(opt_id.clone()),
                                        Some(w),
                                        cx,
                                    );
                                },
                            )
                            .flex_1()
                            .min_w_0()
                            .on_hover(cx.listener(
                                move |this, inside: &bool, _, cx| {
                                    if *inside && this.highlighted_opt != i {
                                        this.highlighted_opt = i;
                                        cx.notify();
                                    }
                                },
                            )),
                        );
                    }
                    grid.into_any_element()
                } else {
                    let mut rows = div().w_full().flex().flex_col().gap(px(2.0));
                    let is_approval =
                        prompt.kind == DecisionKind::Approval && prompt.options.is_empty();
                    for (i, opt) in eff_opts.iter().enumerate() {
                        let is_hi = self.highlighted_opt == i && !self.own_focused;
                        let opt_id = opt.id.clone();
                        let rec_lbl = tr(cx, "Recommended", "推荐");
                        let row_body = div()
                            .w_full()
                            .min_h(px(42.0))
                            .pl(px(6.0))
                            .pr(px(8.0))
                            .py(px(4.0))
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .size(px(22.0))
                                    .flex_none()
                                    .rounded(px(6.0))
                                    .bg(if is_hi { card_bg } else { fill_bg })
                                    .when(is_hi, |d| d.border_1().border_color(p.line))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if is_hi { p.ink } else { p.muted })
                                    .child(format!("{}", i + 1)),
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
                                            .line_height(px(19.0))
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(opt.title.clone()),
                                    )
                                    .when_some(opt.summary.clone(), |d, sum| {
                                        d.child(
                                            div()
                                                .text_size(px(12.0))
                                                .line_height(px(16.0))
                                                .text_color(p.muted)
                                                .truncate()
                                                .child(sum),
                                        )
                                    }),
                            )
                            .when(opt.recommended, |d| {
                                d.child(
                                    div()
                                        .h(px(19.0))
                                        .px(px(6.5))
                                        .flex_none()
                                        .rounded(px(5.0))
                                        .bg(if is_hi { card_bg } else { fill_bg })
                                        .when(is_hi, |b| b.border_1().border_color(p.line))
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.muted)
                                        .child(rec_lbl),
                                )
                            });

                        rows = rows.child(
                            pro_tactile(
                                pro_eid(&self.id, format!("opt-{i}")),
                                opt.title.clone(),
                                8.0,
                                transparent_black(),
                                fill_bg,
                                transparent_black(),
                                p.line,
                                is_hi,
                                row_body,
                                cx,
                                move |this, w, cx| {
                                    let ans = if is_approval {
                                        if i == 0 {
                                            DecisionAnswer::Approve
                                        } else {
                                            DecisionAnswer::Decline
                                        }
                                    } else {
                                        DecisionAnswer::Option(opt_id.clone())
                                    };
                                    this.submit_answer(ans, Some(w), cx);
                                },
                            )
                            .w_full()
                            .on_hover(cx.listener(
                                move |this, inside: &bool, _, cx| {
                                    if *inside && this.highlighted_opt != i {
                                        this.highlighted_opt = i;
                                        cx.notify();
                                    }
                                },
                            )),
                        );
                    }
                    rows.into_any_element()
                };

                let own_row = self.allow_own.then(|| {
                    let has_own_text = self
                        .own_editor
                        .as_ref()
                        .is_some_and(|ed| !ed.read(cx).value().trim().is_empty());
                    let ed_view = self.own_editor.clone();
                    let send_lbl = tr(cx, "Send custom answer", "发送自定义回答");
                    div()
                        .w_full()
                        .min_h(px(36.0))
                        .px(px(6.0))
                        .py(px(3.0))
                        .rounded(px(8.0))
                        .bg(if self.own_focused {
                            well_bg
                        } else {
                            transparent_black()
                        })
                        .border_1()
                        .border_color(if self.own_focused {
                            p.line_strong
                        } else {
                            transparent_black()
                        })
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .hover(move |s| s.bg(well_bg))
                        .child(
                            div()
                                .size(px(22.0))
                                .flex_none()
                                .rounded(px(6.0))
                                .bg(if self.own_focused { card_bg } else { fill_bg })
                                .when(self.own_focused, |d| d.border_1().border_color(p.line))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if self.own_focused { p.ink } else { p.muted })
                                .child(format!("{own_num}")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .h(px(22.0))
                                .when_some(ed_view, |d, ed| d.child(ed)),
                        )
                        .when(has_own_text, |d| {
                            d.child(pro_icon_btn(
                                pro_eid(&self.id, "own-send-btn"),
                                send_lbl,
                                ProIcon::ArrowUp,
                                24.0,
                                true,
                                cx,
                                |this, w, cx| {
                                    if let Some(ed) = &this.own_editor {
                                        let val = ed.read(cx).value().trim().to_owned();
                                        if !val.is_empty() {
                                            this.submit_answer(
                                                DecisionAnswer::Custom(val.into()),
                                                Some(w),
                                                cx,
                                            );
                                        }
                                    }
                                },
                            ))
                        })
                });

                let body_step = pro_step_transition(
                    pro_eid(&self.id, "dc-step"),
                    self.step_serial,
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(5.0))
                        .child(
                            div()
                                .px(px(6.0))
                                .pt(px(3.0))
                                .pb(px(1.0))
                                .text_size(px(14.0))
                                .font_weight(FontWeight::MEDIUM)
                                .line_height(px(20.0))
                                .text_color(p.strong)
                                .child(q_localized),
                        )
                        .when_some(facts_el, |d, f| d.child(f))
                        .child(options_el)
                        .when_some(own_row, |d, r| d.child(r)),
                );

                div()
                    .w_full()
                    .p(px(6.0))
                    .rounded(px(14.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .w_full()
                            .h(px(28.0))
                            .pl(px(6.0))
                            .pr(px(2.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .h(px(18.0))
                                    .px(px(6.0))
                                    .flex_none()
                                    .rounded(px(5.0))
                                    .bg(p.amber_soft)
                                    .text_color(p.amber_ink)
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Needs you", "待你拍板")),
                            )
                            .child(div().flex_1().min_w_0().overflow_hidden().child(
                                pro_swap_label(
                                    pro_eid(&self.id, "dc-task-lbl"),
                                    task_localized,
                                    12.0,
                                    FontWeight::NORMAL,
                                    p.muted,
                                ),
                            ))
                            .child(pager_tools),
                    )
                    .child(body_step)
                    .into_any_element()
            } else {
                div().into_any_element()
            };

        let decision_dock = div()
            .w_full()
            .flex()
            .flex_col()
            .child(pro_fold(
                pro_eid(&self.id, "folded-dock"),
                has_open && is_folded,
                42.0,
                0.0,
                move || folded_bar,
            ))
            .child(pro_fold(
                pro_eid(&self.id, "open-dock"),
                has_open && !is_folded,
                286.0,
                0.0,
                move || expanded_box,
            ));

        // Optional status line + docked composer matching the KUI Pro stage
        let mut column = div()
            .w_full()
            .max_w(px(600.0))
            .flex()
            .flex_col()
            .gap(px(10.0));

        if self.composer_chrome {
            let status_txt: SharedString = if open_len == 0 {
                tr(
                    cx,
                    "All 3 decisions answered – agents resumed full execution.",
                    "全部 3 项问题均已拍板——智能体已恢复全速执行。",
                )
                .into()
            } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("{open_len} 项任务等待你拍板——其余智能体仍在并行推进。").into()
            } else if open_len == 3 {
                "Three tasks wait on you – everything else keeps running.".into()
            } else {
                format!(
                    "{open_len} task{} wait on you – everything else keeps running.",
                    if open_len == 1 { "" } else { "s" }
                )
                .into()
            };

            let replay_lbl = tr(cx, "Replay 3 decisions", "重置 3 道问题");
            column = column.child(
                div()
                    .w_full()
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(pro_agent_avatar(
                        "orchestrator",
                        p.purple,
                        20.0,
                        open_len > 0,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(pro_swap_label(
                                pro_eid(&self.id, "dc-orch-status"),
                                status_txt,
                                13.0,
                                FontWeight::NORMAL,
                                p.muted,
                            )),
                    )
                    .when(open_len < self.initial_decisions.len(), |d| {
                        d.child(pro_btn(
                            pro_eid(&self.id, "dc-reset"),
                            replay_lbl,
                            Some(ProIcon::Undo),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| {
                                this.reset(cx);
                            },
                        ))
                    }),
            );
        }

        column = column.child(decision_dock);

        if self.composer_chrome {
            let composer_bg: Hsla = if p.dark {
                rgb(0x1c1c1e).into()
            } else {
                rgb(0xffffff).into()
            };
            let comp_ed = self.composer_editor.clone();
            let comp_has_text = self
                .composer_editor
                .as_ref()
                .is_some_and(|ed| !ed.read(cx).value().trim().is_empty());
            let send_lbl = tr(cx, "Send message", "发送消息");

            let modes: [(&'static str, ProIcon); 3] = [
                (tr(cx, "Ask before acting", "操作前询问"), ProIcon::Shield),
                (tr(cx, "Auto-edit safe", "安全变更自动执行"), ProIcon::Bolt),
                (tr(cx, "Plan only", "仅生成计划"), ProIcon::List),
            ];
            let active_mode_idx = self.composer_mode_idx.min(modes.len() - 1);
            let (active_mode_lbl, active_mode_icon) = modes[active_mode_idx];
            let mode_open = self.mode_menu_open;

            let mut mode_menu_col = div()
                .w(px(196.0))
                .p(px(4.0))
                .rounded(px(10.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(2.0));
            for (m_idx, (m_lbl, m_ic)) in modes.into_iter().enumerate() {
                let sel = m_idx == active_mode_idx;
                let row_inner = div()
                    .w_full()
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(pro_icon(m_ic, 13.5, if sel { p.ink } else { p.muted }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.0))
                            .font_weight(if sel {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if sel { p.strong } else { p.text })
                            .truncate()
                            .child(m_lbl),
                    )
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("dc-mode-chk-{m_idx}")),
                        sel,
                        14.0,
                        false,
                        p.primary,
                    ));
                mode_menu_col = mode_menu_col.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("dc-mode-opt-{m_idx}")),
                        m_lbl,
                        6.0,
                        transparent_black(),
                        fill_bg,
                        transparent_black(),
                        transparent_black(),
                        sel,
                        row_inner,
                        cx,
                        move |this, _, cx| {
                            this.composer_mode_idx = m_idx;
                            this.mode_menu_open = false;
                            cx.notify();
                        },
                    )
                    .w_full(),
                );
            }

            let models: [(&'static str, &'static str); 3] = [
                ("claude", "Sonnet 5.5"),
                ("openai", "GPT-6 Sol"),
                ("gemini", "Gemini 3.1 Pro"),
            ];
            let active_model_idx = self.composer_model_idx.min(models.len() - 1);
            let (active_brand, active_model_lbl) = models[active_model_idx];
            let model_open = self.model_menu_open;

            let mut model_menu_col = div()
                .w(px(184.0))
                .p(px(4.0))
                .rounded(px(10.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(2.0));
            for (md_idx, (brand, md_lbl)) in models.into_iter().enumerate() {
                let sel = md_idx == active_model_idx;
                let row_inner = div()
                    .w_full()
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .when_some(pro_app_artwork(brand, 13.0, cx), |d, m| d.child(m))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.0))
                            .font_weight(if sel {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if sel { p.strong } else { p.text })
                            .truncate()
                            .child(md_lbl),
                    )
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("dc-model-chk-{md_idx}")),
                        sel,
                        14.0,
                        false,
                        p.primary,
                    ));
                model_menu_col = model_menu_col.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("dc-model-opt-{md_idx}")),
                        md_lbl,
                        6.0,
                        transparent_black(),
                        fill_bg,
                        transparent_black(),
                        transparent_black(),
                        sel,
                        row_inner,
                        cx,
                        move |this, _, cx| {
                            this.composer_model_idx = md_idx;
                            this.model_menu_open = false;
                            cx.notify();
                        },
                    )
                    .w_full(),
                );
            }

            column = column.child(
                div()
                    .w_full()
                    .p(px(12.0))
                    .rounded(px(16.0))
                    .bg(composer_bg)
                    .border_1()
                    .border_color(if self.composer_focused {
                        p.focus
                    } else {
                        p.line_strong
                    })
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .w_full()
                            .h(px(38.0))
                            .px(px(2.0))
                            .when_some(comp_ed, |d, ed| d.child(ed)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .size(px(28.0))
                                            .rounded(px(8.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(pro_icon(ProIcon::Plus, 15.0, p.muted)),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .child(
                                                div()
                                                    .id(pro_eid(&self.id, "dc-mode-trigger"))
                                                    .role(Role::Button)
                                                    .aria_label(active_mode_lbl)
                                                    .aria_expanded(mode_open)
                                                    .tab_index(0)
                                                    .h(px(28.0))
                                                    .px(px(8.0))
                                                    .rounded(px(8.0))
                                                    .bg(if mode_open { fill_bg } else { well_bg })
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .cursor_pointer()
                                                    .hover(move |s| s.bg(fill_bg))
                                                    .focus_visible(move |s| {
                                                        s.border_1().border_color(p.focus)
                                                    })
                                                    .on_click(cx.listener(
                                                        |this, _: &ClickEvent, _, cx| {
                                                            this.mode_menu_open =
                                                                !this.mode_menu_open;
                                                            if this.mode_menu_open {
                                                                this.model_menu_open = false;
                                                            }
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(pro_icon(
                                                        active_mode_icon,
                                                        14.0,
                                                        p.muted,
                                                    ))
                                                    .child(pro_swap_label(
                                                        pro_eid(&self.id, "dc-mode-lbl"),
                                                        active_mode_lbl,
                                                        12.0,
                                                        FontWeight::NORMAL,
                                                        p.muted,
                                                    ))
                                                    .child(pro_icon(
                                                        if mode_open {
                                                            ProIcon::ChevronUp
                                                        } else {
                                                            ProIcon::ChevronDown
                                                        },
                                                        11.5,
                                                        p.muted,
                                                    )),
                                            )
                                            .child(
                                                div().absolute().bottom(px(34.0)).left_0().child(
                                                    pro_popover_presence(
                                                        pro_eid(&self.id, "dc-mode-pop"),
                                                        mode_open,
                                                        move || mode_menu_col.into_any_element(),
                                                    ),
                                                ),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .relative()
                                            .child(
                                                div()
                                                    .id(pro_eid(&self.id, "dc-model-trigger"))
                                                    .role(Role::Button)
                                                    .aria_label(active_model_lbl)
                                                    .aria_expanded(model_open)
                                                    .tab_index(0)
                                                    .h(px(28.0))
                                                    .px(px(8.0))
                                                    .rounded(px(8.0))
                                                    .bg(if model_open {
                                                        fill_bg
                                                    } else {
                                                        transparent_black()
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(5.0))
                                                    .cursor_pointer()
                                                    .hover(move |s| s.bg(fill_bg))
                                                    .focus_visible(move |s| {
                                                        s.border_1().border_color(p.focus)
                                                    })
                                                    .on_click(cx.listener(
                                                        |this, _: &ClickEvent, _, cx| {
                                                            this.model_menu_open =
                                                                !this.model_menu_open;
                                                            if this.model_menu_open {
                                                                this.mode_menu_open = false;
                                                            }
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .when_some(
                                                        pro_app_artwork(active_brand, 13.0, cx),
                                                        |d, m| d.child(m),
                                                    )
                                                    .child(pro_swap_label(
                                                        pro_eid(&self.id, "dc-model-lbl"),
                                                        active_model_lbl,
                                                        12.0,
                                                        FontWeight::NORMAL,
                                                        p.ink,
                                                    ))
                                                    .child(pro_icon(
                                                        if model_open {
                                                            ProIcon::ChevronUp
                                                        } else {
                                                            ProIcon::ChevronDown
                                                        },
                                                        12.0,
                                                        p.muted,
                                                    )),
                                            )
                                            .child(
                                                div().absolute().bottom(px(34.0)).right_0().child(
                                                    pro_popover_presence(
                                                        pro_eid(&self.id, "dc-model-pop"),
                                                        model_open,
                                                        move || model_menu_col.into_any_element(),
                                                    ),
                                                ),
                                            ),
                                    )
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, "composer-send"),
                                        send_lbl,
                                        ProIcon::ArrowUp,
                                        28.0,
                                        comp_has_text,
                                        cx,
                                        |this, w, cx| {
                                            if let Some(ed) = &this.composer_editor {
                                                let val = ed.read(cx).value().trim().to_owned();
                                                if !val.is_empty() {
                                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                                    this.toast = Some((
                                                        tr(
                                                            cx,
                                                            "Message sent to orchestrator",
                                                            "已向编排智能体发送消息",
                                                        )
                                                        .into(),
                                                        Instant::now(),
                                                    ));
                                                    cx.emit(DecisionCardEvent::ComposerSubmitted(
                                                        val.into(),
                                                    ));
                                                    cx.notify();
                                                }
                                            }
                                        },
                                    )),
                            ),
                    ),
            );
        }

        let width_sensor = self.measured_width.clone();
        let a11y_label = tr(
            cx,
            "Agent decision card docked above composer",
            "停靠于输入框上方的智能体结构化提问卡",
        );
        let focus_ring = p.focus;

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(a11y_label)
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .w_full()
            .py(px(22.0))
            .px(px(16.0))
            .flex()
            .flex_col()
            .items_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(16.0)).border_color(focus_ring))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.mode_menu_open || this.model_menu_open {
                    this.mode_menu_open = false;
                    this.model_menu_open = false;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && (this.mode_menu_open || this.model_menu_open) {
                    this.mode_menu_open = false;
                    this.model_menu_open = false;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if this.own_focused || this.composer_focused || this.decisions.is_empty() {
                    return;
                }
                let cur = this.cursor.min(this.decisions.len() - 1);
                let prompt = this.decisions[cur].clone();
                let eff_opts = Self::effective_options(&prompt, cx);
                let n = eff_opts.len();
                let is_approval =
                    prompt.kind == DecisionKind::Approval && prompt.options.is_empty();

                let key = ev.keystroke.key.as_str();
                if this.folded {
                    if matches!(key, "enter" | " " | "space") {
                        this.set_folded(false, cx);
                        cx.stop_propagation();
                    }
                    return;
                }

                if let Some(digit) = match key {
                    "1" => Some(1usize),
                    "2" => Some(2usize),
                    "3" => Some(3usize),
                    "4" => Some(4usize),
                    "5" => Some(5usize),
                    _ => None,
                } {
                    if digit <= n {
                        let pick_idx = digit - 1;
                        let ans = if is_approval {
                            if pick_idx == 0 {
                                DecisionAnswer::Approve
                            } else {
                                DecisionAnswer::Decline
                            }
                        } else {
                            DecisionAnswer::Option(eff_opts[pick_idx].id.clone())
                        };
                        this.submit_answer(ans, Some(w), cx);
                        cx.stop_propagation();
                    } else if digit == n + 1
                        && this.allow_own
                        && let Some(ed) = &this.own_editor
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                        cx.stop_propagation();
                    }
                    return;
                }

                match key {
                    "up" => {
                        if n > 0 {
                            this.highlighted_opt = this.highlighted_opt.saturating_sub(1);
                            cx.notify();
                        }
                        cx.stop_propagation();
                    }
                    "down" => {
                        if this.highlighted_opt + 1 < n {
                            this.highlighted_opt += 1;
                            cx.notify();
                        } else if this.allow_own
                            && let Some(ed) = &this.own_editor
                        {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                        cx.stop_propagation();
                    }
                    "left" => {
                        this.step_page(-1, cx);
                        cx.stop_propagation();
                    }
                    "right" => {
                        this.step_page(1, cx);
                        cx.stop_propagation();
                    }
                    "enter" | " " | "space" => {
                        if n > 0 {
                            let hi = this.highlighted_opt.min(n - 1);
                            let ans = if is_approval {
                                if hi == 0 {
                                    DecisionAnswer::Approve
                                } else {
                                    DecisionAnswer::Decline
                                }
                            } else {
                                DecisionAnswer::Option(eff_opts[hi].id.clone())
                            };
                            this.submit_answer(ans, Some(w), cx);
                            cx.stop_propagation();
                        }
                    }
                    "escape" => {
                        this.set_folded(true, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            let w_px = f32::from(b.size.width);
                            if w_px > 20.0 {
                                width_sensor.set(w_px);
                            }
                        }
                    })
                    .child(column),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{DecisionAnswer, DecisionCard, DecisionKind, DecisionPrompt};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn decision_card_cycles_three_variants_and_folds_cleanly(cx: &mut TestAppContext) {
        let card = cx.new(|cx| DecisionCard::new("test-decision-card", cx));

        card.update(cx, |c, cx| {
            assert_eq!(c.remaining_count(), 3);
            assert!(!c.is_folded());

            c.set_folded(true, cx);
            assert!(c.is_folded());
            c.set_folded(false, cx);

            c.submit_answer(DecisionAnswer::Option("kafka".into()), None, cx);
            assert_eq!(c.remaining_count(), 2);

            c.submit_answer(DecisionAnswer::Option("columns".into()), None, cx);
            assert_eq!(c.remaining_count(), 1);

            c.submit_answer(DecisionAnswer::Approve, None, cx);
            assert_eq!(c.remaining_count(), 0);

            c.reset(cx);
            assert_eq!(c.remaining_count(), 3);
        });

        let p = DecisionPrompt::new("q1", "Task", "Question?").with_kind(DecisionKind::Pick);
        assert_eq!(p.kind, DecisionKind::Pick);
    }
}
