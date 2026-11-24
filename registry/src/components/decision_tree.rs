//! Interactive branching decision tree for agent architectural and operational choices.
//!
//! Lays out an agent's blocking fork as a visual left-to-right tree with `PathBuilder` /
//! `pro_connector_pulse` curves, highlighting the agent's recommended path (`Agent's pick`),
//! surfacing cost, latency, and flagged risk notes along each branch, supporting nested follow-up
//! sub-choices, and offering an inline `"Other…"` custom instruction branch.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_connector_pulse, pro_eid, pro_empty_state,
        pro_skeleton_bar, pro_step_transition, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const TREE_MARK_SVG: &str = r#"<circle cx="3.5" cy="8" r="2" stroke="currentColor" stroke-width="1.4"></circle><circle cx="12.5" cy="3.5" r="2" stroke="currentColor" stroke-width="1.4"></circle><circle cx="12.5" cy="12.5" r="2" stroke="currentColor" stroke-width="1.4"></circle><path d="M5.5 8H7.5C8.3 8 8.5 7.6 8.5 6.8V5C8.5 4 9 3.5 10 3.5H10.5M8.5 8.2V11C8.5 12 9 12.5 10 12.5H10.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.4"></path>"#;

/// Risk badge tier displayed above the root question card in [`DecisionTree`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecisionTreeRisk {
    /// Low-impact fork (no badge shown).
    Low,
    /// Moderate operational impact (`Medium risk` amber badge).
    #[default]
    Medium,
    /// High financial or production impact (`High risk` red badge).
    High,
}

/// Consequence line with optional flagged warning span and explanatory tooltip note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchConsequence {
    /// Plain consequence prefix or full line (for example `"Cancels 3 subscriptions · reversible"`).
    pub text: SharedString,
    /// Optional highlighted warning span (for example `"$147 · not reversible"`).
    pub flagged: Option<SharedString>,
    /// Optional agent explanation shown when hovering the flag or pressing `F`.
    pub flag_note: Option<SharedString>,
    /// Optional source citation badge inside the flag note.
    pub flag_source: Option<SharedString>,
}

impl BranchConsequence {
    /// Create a plain consequence line.
    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            flagged: None,
            flag_note: None,
            flag_source: None,
        }
    }

    /// Create a consequence line containing an amber-flagged warning span and explanation note.
    pub fn flagged(
        prefix: impl Into<SharedString>,
        flagged: impl Into<SharedString>,
        note: impl Into<SharedString>,
    ) -> Self {
        Self {
            text: prefix.into(),
            flagged: Some(flagged.into()),
            flag_note: Some(note.into()),
            flag_source: None,
        }
    }

    /// Attach a source badge to a flagged consequence note.
    pub fn with_source(mut self, source: impl Into<SharedString>) -> Self {
        self.flag_source = Some(source.into());
        self
    }
}

/// Nested follow-up sub-choice hanging off a primary [`DecisionBranch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionSubOption {
    /// Sub-option label (for example `"Post on the status page"`).
    pub label: SharedString,
    /// Whether this sub-option is the agent's preferred path.
    pub pick: bool,
    /// Consequence line for this sub-option.
    pub effect: BranchConsequence,
    /// Connected app logos affected by this sub-option.
    pub apps: Vec<SharedString>,
}

impl DecisionSubOption {
    /// Construct a [`DecisionSubOption`].
    pub fn new(label: impl Into<SharedString>, effect: BranchConsequence) -> Self {
        Self {
            label: label.into(),
            pick: false,
            effect,
            apps: Vec::new(),
        }
    }

    /// Mark this sub-option as the agent's recommended sub-path.
    pub fn with_pick(mut self, pick: bool) -> Self {
        self.pick = pick;
        self
    }

    /// Attach app logos (`"slack"`, `"github"`, `"vercel"`, `"stripe"`).
    pub fn with_apps<I, S>(mut self, apps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.apps = apps.into_iter().map(Into::into).collect();
        self
    }
}

/// Primary option branch in a [`TreeDecision`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionBranch {
    /// Option action label (for example `"Roll back to #481"`).
    pub label: SharedString,
    /// Whether this branch is highlighted as `"Agent's pick"`.
    pub pick: bool,
    /// Direct consequence lines when there is no nested follow-up question.
    pub effects: Vec<BranchConsequence>,
    /// Optional follow-up sub-question and sub-options.
    pub sub_question: Option<SharedString>,
    /// Nested sub-options under `sub_question`.
    pub sub_options: Vec<DecisionSubOption>,
    /// Connected app logos shown on the leaf card.
    pub apps: Vec<SharedString>,
}

impl DecisionBranch {
    /// Construct a [`DecisionBranch`] with `label`.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            pick: false,
            effects: Vec::new(),
            sub_question: None,
            sub_options: Vec::new(),
            apps: Vec::new(),
        }
    }

    /// Mark this branch as `"Agent's pick"`.
    pub fn with_pick(mut self, pick: bool) -> Self {
        self.pick = pick;
        self
    }

    /// Attach leaf consequence lines.
    pub fn with_effects(mut self, effects: Vec<BranchConsequence>) -> Self {
        self.effects = effects;
        self
    }

    /// Attach a nested follow-up question and sub-choices.
    pub fn with_sub_choices(
        mut self,
        question: impl Into<SharedString>,
        sub_options: Vec<DecisionSubOption>,
    ) -> Self {
        self.sub_question = Some(question.into());
        self.sub_options = sub_options;
        self
    }

    /// Attach app brand logos to the leaf card.
    pub fn with_apps<I, S>(mut self, apps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.apps = apps.into_iter().map(Into::into).collect();
        self
    }
}

/// Single branching decision scenario queued in [`DecisionTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeDecision {
    /// Stable scenario identifier.
    pub id: SharedString,
    /// Asking agent name (for example `"Incident agent"`).
    pub agent: SharedString,
    /// Frontier model name (for example `"Opus 5.5"`).
    pub model: SharedString,
    /// Risk tier (`Low`, `Medium`, `High`).
    pub risk: DecisionTreeRisk,
    /// Root question asked by the agent.
    pub question: SharedString,
    /// Supporting telemetry or operational context below the question.
    pub context: Option<SharedString>,
    /// 2–4 selectable branches.
    pub branches: Vec<DecisionBranch>,
}

impl TreeDecision {
    /// Construct a [`TreeDecision`].
    pub fn new(
        id: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        question: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            agent: agent.into(),
            model: "Opus 5.5".into(),
            risk: DecisionTreeRisk::High,
            question: question.into(),
            context: None,
            branches: Vec::new(),
        }
    }

    /// Set the model subtitle under the agent name.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the [`DecisionTreeRisk`] tier.
    pub fn with_risk(mut self, risk: DecisionTreeRisk) -> Self {
        self.risk = risk;
        self
    }

    /// Set the supporting context line below the root question.
    pub fn with_context(mut self, context: impl Into<SharedString>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Supply the selectable [`DecisionBranch`] options.
    pub fn with_branches(mut self, branches: Vec<DecisionBranch>) -> Self {
        self.branches = branches;
        self
    }
}

/// Choice confirmed by the user on a [`TreeDecision`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeChoice {
    /// User selected branch `branch_index` (and optional `sub_index`).
    Branch {
        /// Zero-based branch index.
        branch_index: usize,
        /// Branch label.
        label: SharedString,
        /// Optional nested sub-option label.
        sub_label: Option<SharedString>,
    },
    /// User wrote and confirmed a custom instruction via `"Other…"`.
    Custom(SharedString),
}

/// Events emitted by [`DecisionTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionTreeEvent {
    /// User confirmed a branch or custom instruction on `decision_id`.
    Chosen {
        /// Identifier of the resolved [`TreeDecision`].
        decision_id: SharedString,
        /// Confirmed branch or custom path.
        choice: TreeChoice,
    },
    /// User navigated to another decision in the header pager.
    ActiveDecisionChanged(usize),
    /// User toggled or hovered a flagged consequence note.
    FlagNoteShown(Option<SharedString>),
    /// User reset the decision tree queue.
    Reset,
}

impl EventEmitter<DecisionTreeEvent> for DecisionTree {}

fn default_tree_decisions() -> Vec<TreeDecision> {
    vec![
        TreeDecision::new(
            "incident",
            "Incident agent",
            "Checkout errors jumped to 18% after deploy #482 – what should I do?",
        )
        .with_model("Opus 5.5")
        .with_risk(DecisionTreeRisk::High)
        .with_context("2,140 errors in 12 minutes · about $3.2k in orders an hour at risk.")
        .with_branches(vec![
            DecisionBranch::new("Roll back to #481")
                .with_pick(true)
                .with_sub_choices(
                    "Tell customers about it?",
                    vec![
                        DecisionSubOption::new(
                            "Post on the status page",
                            BranchConsequence::plain("Restores checkout in ~90s · $0 extra cost"),
                        )
                        .with_pick(true)
                        .with_apps(["vercel", "slack"]),
                        DecisionSubOption::new(
                            "Only tell the team",
                            BranchConsequence::plain("Posts revert summary to #incidents · ~45s"),
                        )
                        .with_apps(["github", "slack"]),
                    ],
                ),
            DecisionBranch::new("Ship a hotfix").with_sub_choices(
                "Wait for the test run?",
                vec![
                    DecisionSubOption::new(
                        "Ship right away",
                        BranchConsequence::flagged(
                            "Deploys in 2m · ",
                            "skips 14m CI suite",
                            "Deploy #482 also bypassed staging load checks before errors spiked.",
                        )
                        .with_source("ci/deploy-482"),
                    )
                    .with_apps(["github"]),
                    DecisionSubOption::new(
                        "After the tests",
                        BranchConsequence::plain("14m wait · ~$750 orders exposed"),
                    )
                    .with_apps(["github"]),
                ],
            ),
            DecisionBranch::new("Turn the flag off")
                .with_effects(vec![
                    BranchConsequence::plain("Disables new Stripe Elements · instant"),
                    BranchConsequence::plain("Keeps #482 live for non-checkout routes"),
                ])
                .with_apps(["stripe"]),
        ]),
        TreeDecision::new(
            "dupes",
            "Billing agent",
            "3 customers have two active subscriptions – how should I proceed?",
        )
        .with_model("Sonnet 5.5")
        .with_risk(DecisionTreeRisk::High)
        .with_context("All three signed up twice during the checkout outage.")
        .with_branches(vec![
            DecisionBranch::new("Cancel the newer ones")
                .with_pick(true)
                .with_effects(vec![
                    BranchConsequence::plain("Cancels 3 subscriptions · reversible"),
                    BranchConsequence::plain("Sends 3 confirmation emails"),
                ])
                .with_apps(["stripe", "gmail"]),
            DecisionBranch::new("Refund the newer ones")
                .with_effects(vec![
                    BranchConsequence::flagged(
                        "Refunds ",
                        "$147 · not reversible",
                        "One charge is two cycles old and Stripe processing fees are not returned.",
                    )
                    .with_source("stripe/ch_3Q9k"),
                ])
                .with_apps(["stripe"]),
            DecisionBranch::new("Ask the customers")
                .with_effects(vec![BranchConsequence::plain(
                    "Sends 3 emails · waits ~24h for answers",
                )])
                .with_apps(["gmail"]),
        ]),
        TreeDecision::new(
            "cache",
            "Infra agent",
            "EU read latency hit 420ms – how should I warm the Frankfurt cache?",
        )
        .with_model("GPT-6 Sol")
        .with_risk(DecisionTreeRisk::Medium)
        .with_context("18,400 active sessions routed through eu-central-1.")
        .with_branches(vec![
            DecisionBranch::new("Shadow-replay live reads")
                .with_pick(true)
                .with_effects(vec![BranchConsequence::plain(
                    "Zero stale keys · +$14 egress · 6m warm-up",
                )]),
            DecisionBranch::new("Snapshot from us-east-1").with_effects(vec![
                BranchConsequence::flagged(
                    "Ready in 45s · ",
                    "up to 4m stale carts",
                    "22 open checkouts in EU could see outdated cart totals.",
                )
                .with_source("redis/eu-central-1"),
            ]),
            DecisionBranch::new("Throttle EU traffic 30%").with_effects(vec![
                BranchConsequence::plain("Caps p99 at 180ms · queues background webhooks"),
            ]),
        ]),
    ]
}

#[derive(Clone, Debug)]
struct ConfirmedState {
    branch_idx: usize,
    summary: SharedString,
    agent_reply: SharedString,
}

#[derive(Clone, Default)]
struct TreeBoundsMap {
    root: Rc<Cell<Option<Bounds<Pixels>>>>,
    branches: [Rc<Cell<Option<Bounds<Pixels>>>>; 5],
    leaves: [Rc<Cell<Option<Bounds<Pixels>>>>; 5],
}

/// Interactive branching decision tree with live `PathBuilder` connectors, consequence flags, and custom path input.
pub struct DecisionTree {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    decisions: Vec<TreeDecision>,
    initial_decisions: Vec<TreeDecision>,
    cursor: usize,
    hovered_branch: Option<usize>,
    other_open: bool,
    other_editor: Option<Entity<Editor>>,
    other_focused: bool,
    active_flag_note: Option<(SharedString, Option<SharedString>)>,
    confirmed: Option<ConfirmedState>,
    allow_other: bool,
    boot_at: Instant,
    step_serial: i32,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl DecisionTree {
    /// Create a [`DecisionTree`] preloaded with 3 operational & architectural branching scenarios.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let initial = default_tree_decisions();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Decisions".into(),
            decisions: initial.clone(),
            initial_decisions: initial,
            cursor: 0,
            hovered_branch: None,
            other_open: false,
            other_editor: None,
            other_focused: false,
            active_flag_note: None,
            confirmed: None,
            allow_other: true,
            boot_at: Instant::now(),
            step_serial: 0,
            measured_width: Rc::new(Cell::new(780.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the top bar heading (defaults to `"Decisions"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Supply custom [`TreeDecision`] items.
    pub fn with_decisions(mut self, decisions: Vec<TreeDecision>) -> Self {
        self.initial_decisions = decisions.clone();
        self.decisions = decisions;
        self.cursor = 0;
        self
    }

    /// Show or hide the `"Other…"` custom instruction branch.
    pub fn with_allow_other(mut self, allow_other: bool) -> Self {
        self.allow_other = allow_other;
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

    /// Number of open decisions remaining in the queue.
    pub fn remaining_count(&self) -> usize {
        self.decisions.len()
    }

    /// Choose branch `branch_idx` (and optional `sub_idx`) on the current decision.
    pub fn choose_branch(
        &mut self,
        branch_idx: usize,
        sub_idx: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let Some(dec) = self.decisions.get(self.cursor).cloned() else {
            return;
        };
        let Some(branch) = dec.branches.get(branch_idx).cloned() else {
            return;
        };
        let picked_sub = sub_idx
            .and_then(|si| branch.sub_options.get(si))
            .or_else(|| branch.sub_options.iter().find(|so| so.pick))
            .or_else(|| branch.sub_options.first());

        let sub_label = picked_sub.map(|so| so.label.clone());
        let summary: SharedString = if let Some(sl) = &sub_label {
            format!("{} → {sl}", branch.label).into()
        } else {
            branch.label.clone()
        };
        let reply: SharedString = tr(
            cx,
            "Approved · executing selected path now",
            "已确认路径 · 智能体正在沿所选分支执行",
        )
        .into();

        self.other_open = false;
        self.active_flag_note = None;
        self.confirmed = Some(ConfirmedState {
            branch_idx,
            summary: summary.clone(),
            agent_reply: reply,
        });
        self.toast = Some((summary, Instant::now()));

        cx.emit(DecisionTreeEvent::Chosen {
            decision_id: dec.id,
            choice: TreeChoice::Branch {
                branch_index: branch_idx,
                label: branch.label,
                sub_label,
            },
        });
        cx.notify();
    }

    /// Submit a custom path instruction on the `"Other…"` branch.
    pub fn submit_custom(&mut self, instruction: SharedString, cx: &mut Context<Self>) {
        let Some(dec) = self.decisions.get(self.cursor).cloned() else {
            return;
        };
        let other_idx = dec.branches.len();
        let reply: SharedString = tr(
            cx,
            "Got it — adapting plan to your custom instruction.",
            "收到——正在按你的自定义指令调整执行路径。",
        )
        .into();
        self.other_open = false;
        self.confirmed = Some(ConfirmedState {
            branch_idx: other_idx,
            summary: instruction.clone(),
            agent_reply: reply,
        });
        self.toast = Some((
            tr(cx, "Custom path sent to agent", "已发送自定义路径指令").into(),
            Instant::now(),
        ));
        cx.emit(DecisionTreeEvent::Chosen {
            decision_id: dec.id,
            choice: TreeChoice::Custom(instruction),
        });
        cx.notify();
    }

    /// Dismiss the confirmed outcome and advance to the next open [`TreeDecision`].
    pub fn advance_after_confirm(&mut self, cx: &mut Context<Self>) {
        if self.confirmed.take().is_some() && !self.decisions.is_empty() {
            let idx = self.cursor.min(self.decisions.len() - 1);
            self.decisions.remove(idx);
            if self.cursor >= self.decisions.len() && !self.decisions.is_empty() {
                self.cursor = self.decisions.len() - 1;
            }
            self.hovered_branch = None;
            self.active_flag_note = None;
            self.step_serial += 1;
            cx.notify();
        }
    }

    /// Reset all 3 branching scenarios back into the queue.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.decisions = self.initial_decisions.clone();
        self.cursor = 0;
        self.confirmed = None;
        self.other_open = false;
        self.hovered_branch = None;
        self.active_flag_note = None;
        self.step_serial += 1;
        cx.emit(DecisionTreeEvent::Reset);
        cx.notify();
    }

    fn step_decision(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.decisions.len();
        if len <= 1 {
            return;
        }
        let cur = self.cursor.min(len - 1) as isize;
        let next = (cur + delta).rem_euclid(len as isize) as usize;
        if next != self.cursor {
            self.cursor = next;
            self.confirmed = None;
            self.other_open = false;
            self.hovered_branch = None;
            self.active_flag_note = None;
            self.step_serial += if delta >= 0 { 1 } else { -1 };
            cx.emit(DecisionTreeEvent::ActiveDecisionChanged(next));
            cx.notify();
        }
    }

    fn cycle_flag_note(&mut self, cx: &mut Context<Self>) {
        let Some(dec) = self.decisions.get(self.cursor) else {
            return;
        };
        if self.active_flag_note.is_some() {
            self.active_flag_note = None;
            cx.emit(DecisionTreeEvent::FlagNoteShown(None));
            cx.notify();
            return;
        }
        for b in &dec.branches {
            for eff in &b.effects {
                if let Some(note) = &eff.flag_note {
                    self.active_flag_note = Some((note.clone(), eff.flag_source.clone()));
                    cx.emit(DecisionTreeEvent::FlagNoteShown(Some(note.clone())));
                    cx.notify();
                    return;
                }
            }
            for so in &b.sub_options {
                if let Some(note) = &so.effect.flag_note {
                    self.active_flag_note = Some((note.clone(), so.effect.flag_source.clone()));
                    cx.emit(DecisionTreeEvent::FlagNoteShown(Some(note.clone())));
                    cx.notify();
                    return;
                }
            }
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.allow_other && self.other_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Write your own path for the agent…",
                "输入自定义执行路径或指令…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.other_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.other_focused = false;
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty() {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                        this.submit_custom(val.into(), cx);
                        this.focus.focus(w, cx);
                    }
                }
                EditEvent::Escape => {
                    this.other_open = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditorShortcutEvent, w, cx| {
                let EditorShortcutEvent::CommandSubmit = ev;
                let val = ed_ent.read(cx).value().trim().to_owned();
                if !val.is_empty() {
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    this.submit_custom(val.into(), cx);
                    this.focus.focus(w, cx);
                }
            })
            .detach();
            self.other_editor = Some(ed);
        }
    }

    fn render_consequence_line(
        &self,
        eff: &BranchConsequence,
        key_suffix: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        if let Some(flagged_txt) = &eff.flagged {
            let flag_bg = p.amber.opacity(if p.dark { 0.18 } else { 0.16 });
            let flag_fg = p.amber_ink;
            let note_opt = eff.flag_note.clone();
            let src_opt = eff.flag_source.clone();
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(3.0))
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(p.text)
                .when(!eff.text.is_empty(), |d| d.child(eff.text.clone()))
                .child(
                    div()
                        .id(pro_eid(&self.id, format!("flag-{key_suffix}")))
                        .role(Role::Button)
                        .aria_label(flagged_txt.clone())
                        .tab_index(0)
                        .px(px(5.0))
                        .rounded(px(4.0))
                        .bg(flag_bg)
                        .border_b_1()
                        .border_color(p.orange.opacity(0.6))
                        .text_color(flag_fg)
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .on_hover(cx.listener({
                            let note_opt = note_opt.clone();
                            let src_opt = src_opt.clone();
                            move |this, inside: &bool, _, cx| {
                                if *inside {
                                    if let Some(n) = &note_opt {
                                        this.active_flag_note = Some((n.clone(), src_opt.clone()));
                                    }
                                } else {
                                    this.active_flag_note = None;
                                }
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if let Some(n) = &note_opt {
                                this.active_flag_note = Some((n.clone(), src_opt.clone()));
                                cx.stop_propagation();
                                cx.notify();
                            }
                        }))
                        .child(flagged_txt.clone()),
                )
                .into_any_element()
        } else {
            div()
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(p.text)
                .child(eff.text.clone())
                .into_any_element()
        }
    }
}

fn render_app_cluster(
    apps: &[SharedString],
    card_bg: Hsla,
    p: &ProPalette,
    cx: &App,
) -> Option<Div> {
    if apps.is_empty() {
        return None;
    }
    let mut row = div().flex_none().flex().items_center();
    for (idx, app_key) in apps.iter().take(3).enumerate() {
        if let Some(mark) = pro_app_artwork(app_key.as_ref(), 12.0, cx) {
            row = row.child(
                div()
                    .when(idx > 0, |d| d.ml(px(-3.0)))
                    .size(px(20.0))
                    .rounded(px(6.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(mark),
            );
        }
    }
    Some(row)
}

impl Render for DecisionTree {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        if self.other_open {
            self.ensure_editor(w, cx);
        }

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
                        .max_w(px(780.0))
                        .flex()
                        .flex_col()
                        .gap(px(16.0))
                        .child(pro_skeleton_bar(240.0, 18.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(760.0, 320.0, 12.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No branching decisions waiting", "暂无待拍板的分支决策"),
                    tr(
                        cx,
                        "When an agent reaches a fork with trade-offs, its branching options and consequences appear here.",
                        "当智能体遇到存在利弊权衡的岔路时，各分支路径与后果将在此展开。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let cw = self.measured_width.get();
        let stacked = cw < 640.0;
        let reduced = disabled_motion(cx);
        let elapsed_secs = self.boot_at.elapsed().as_secs_f32();
        if !reduced && !self.decisions.is_empty() {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let leaf_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.025)
        } else {
            hsla(0.0, 0.0, 0.0, 0.018)
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.08)
        } else {
            hsla(0.0, 0.0, 0.0, 0.06)
        };

        let open_len = self.decisions.len();
        let cur_idx = if open_len == 0 {
            0
        } else {
            self.cursor.min(open_len - 1)
        };

        // Top Bar
        let title_str: SharedString = if self.title.as_ref() == "Decisions" {
            tr(cx, "Decisions", "智能体决策树").into()
        } else {
            self.title.clone()
        };
        let waiting_str: SharedString = if open_len == 0 {
            tr(cx, "All clear", "已全部处理").into()
        } else if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("{open_len} 项等待你决策").into()
        } else {
            format!("{open_len} waiting for you").into()
        };

        let header_bar = div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .size(px(16.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(crate::primitives::pro_icons::pro_svg(
                        "0 0 16 16",
                        TREE_MARK_SVG,
                        16.0,
                        p.blue,
                    )),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .when(!stacked, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.0))
                        .text_color(p.muted)
                        .truncate()
                        .child(waiting_str),
                )
            })
            .when(stacked, |d| d.child(div().flex_1()))
            .when(open_len > 1, |d| {
                let prev_lbl = tr(cx, "Previous decision", "上一项决策");
                let next_lbl = tr(cx, "Next decision", "下一项决策");
                let mut dots = div().flex().items_center().gap(px(4.0));
                for di in 0..open_len {
                    let active = di == cur_idx;
                    dots = dots.child(
                        div()
                            .size(px(if active { 6.0 } else { 5.0 }))
                            .rounded(px(1.5))
                            .bg(if active { p.ink } else { p.line_strong }),
                    );
                }
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(dots)
                        .child(div().text_size(px(12.0)).text_color(p.muted).child(format!(
                            "{} of {}",
                            cur_idx + 1,
                            open_len
                        )))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .id(pro_eid(&self.id, "dt-prev"))
                                        .role(Role::Button)
                                        .aria_label(prev_lbl)
                                        .tab_index(0)
                                        .size(px(22.0))
                                        .rounded(px(5.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(fill_bg))
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.step_decision(-1, cx);
                                        }))
                                        .child(pro_icon(ProIcon::ChevronLeft, 12.0, p.muted)),
                                )
                                .child(
                                    div()
                                        .id(pro_eid(&self.id, "dt-next"))
                                        .role(Role::Button)
                                        .aria_label(next_lbl)
                                        .tab_index(0)
                                        .size(px(22.0))
                                        .rounded(px(5.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(fill_bg))
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.step_decision(1, cx);
                                        }))
                                        .child(pro_icon(ProIcon::ChevronRight, 12.0, p.muted)),
                                ),
                        ),
                )
            });

        // Main Tree Stage
        let tree_stage = if let Some(dec) = self.decisions.get(cur_idx).cloned() {
            let bounds_map = TreeBoundsMap::default();
            let branch_len = dec.branches.len().min(4);
            let total_rows = if self.allow_other {
                branch_len + 1
            } else {
                branch_len
            };
            let pick_idx = dec.branches.iter().position(|b| b.pick);
            let hot_idx = self
                .confirmed
                .as_ref()
                .map(|c| c.branch_idx)
                .or(self.hovered_branch)
                .or_else(|| self.other_open.then_some(branch_len));
            let pulse_prog = if reduced {
                None
            } else {
                Some((elapsed_secs / 1.9).fract())
            };

            // Connector Canvas painted behind the nodes (only in wide left-to-right mode)
            let connector_layer = (!stacked).then(|| {
                let bmap = bounds_map.clone();
                let confirmed_idx = self.confirmed.as_ref().map(|c| c.branch_idx);
                let link_idle = p.ink.opacity(0.16);
                let link_pick = p.blue.opacity(0.46);
                let link_hot = p.ink.opacity(0.42);
                let link_pick_hot = p.blue.opacity(0.76);
                let ok_color = p.green;
                let blue_pulse = p.blue;

                canvas(
                    |_, _, _| (),
                    move |surface, _, window, _| {
                        let Some(root_b) = bmap.root.get() else {
                            return;
                        };
                        let _ = surface;
                        let p_root = point(root_b.right(), root_b.center().y);
                        for i in 0..total_rows.min(5) {
                            let Some(br_b) = bmap.branches[i].get() else {
                                continue;
                            };
                            let p_br_in = point(br_b.left(), br_b.center().y);
                            let is_pick = pick_idx == Some(i);
                            let is_chosen = confirmed_idx == Some(i);
                            let is_hot = hot_idx == Some(i);
                            let dimmed = confirmed_idx.is_some_and(|ci| ci != i)
                                || (confirmed_idx.is_none() && hot_idx.is_some_and(|hi| hi != i));

                            let (stroke_c, p_col, prog, sw) = if is_chosen {
                                (ok_color.opacity(0.75), ok_color, pulse_prog, 1.6)
                            } else if dimmed {
                                (link_idle.opacity(0.08), blue_pulse, None, 1.0)
                            } else if is_pick && is_hot {
                                (link_pick_hot, blue_pulse, pulse_prog, 1.5)
                            } else if is_pick {
                                (link_pick, blue_pulse, pulse_prog, 1.35)
                            } else if is_hot {
                                (link_hot, blue_pulse, pulse_prog, 1.35)
                            } else {
                                (link_idle, blue_pulse, None, 1.0)
                            };

                            pro_connector_pulse(
                                p_root, p_br_in, 8.0, false, stroke_c, p_col, prog, sw, window,
                            );

                            if let Some(lf_b) = bmap.leaves[i].get() {
                                let p_br_out = point(br_b.right(), br_b.center().y);
                                let p_lf_in = point(lf_b.left(), lf_b.center().y);
                                pro_connector_pulse(
                                    p_br_out, p_lf_in, 8.0, false, stroke_c, p_col, prog, sw,
                                    window,
                                );
                            }
                        }
                    },
                )
                .absolute()
                .inset_0()
            });

            // Left Root Question Card
            let risk_pill = match dec.risk {
                DecisionTreeRisk::Low => None,
                DecisionTreeRisk::Medium => Some(
                    div()
                        .h(px(20.0))
                        .px(px(7.0))
                        .rounded(px(5.0))
                        .bg(p.amber_soft)
                        .text_color(p.amber_ink)
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Medium risk", "中风险")),
                ),
                DecisionTreeRisk::High => Some(
                    div()
                        .h(px(20.0))
                        .px(px(7.0))
                        .rounded(px(5.0))
                        .bg(p.red_soft)
                        .text_color(p.red_ink)
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "High risk", "高风险")),
                ),
            };

            let root_sensor = bounds_map.root.clone();
            let root_col = div()
                .when(!stacked, |d| d.w(px(208.0)).flex_none())
                .when(stacked, |d| d.w_full())
                .flex()
                .flex_col()
                .items_start()
                .gap(px(6.0))
                .when_some(risk_pill, |d, pill| d.child(pill))
                .child(
                    div()
                        .w_full()
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(b) = bounds.first() {
                                root_sensor.set(Some(*b));
                            }
                        })
                        .child(
                            div()
                                .w_full()
                                .p(px(14.0))
                                .rounded(px(9.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.line_strong)
                                .when(!p.dark, |d| d.shadow_xs())
                                .flex()
                                .flex_col()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(9.0))
                                        .child(pro_agent_avatar(
                                            dec.agent.as_ref(),
                                            p.orange,
                                            28.0,
                                            self.confirmed.is_none(),
                                            elapsed_secs * 1000.0,
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
                                                        .text_size(px(13.0))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.ink)
                                                        .truncate()
                                                        .child(dec.agent.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.5))
                                                        .text_color(p.muted)
                                                        .truncate()
                                                        .child(dec.model.clone()),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .line_height(px(20.0))
                                        .text_color(p.strong)
                                        .child(dec.question.clone()),
                                )
                                .when_some(dec.context.clone(), |d, ctx_txt| {
                                    d.child(
                                        div()
                                            .text_size(px(12.0))
                                            .line_height(px(17.0))
                                            .text_color(p.muted)
                                            .child(ctx_txt),
                                    )
                                }),
                        ),
                );

            // Right Branches Column
            let mut branches_col = div()
                .flex_1()
                .min_w_0()
                .when(stacked, |d| {
                    d.pl(px(16.0)).border_l_2().border_color(p.line_strong)
                })
                .flex()
                .flex_col()
                .gap(px(14.0));

            for (bi, branch) in dec.branches.iter().take(4).enumerate() {
                let is_chosen = self.confirmed.as_ref().is_some_and(|c| c.branch_idx == bi);
                let dimmed = self.confirmed.as_ref().is_some_and(|c| c.branch_idx != bi)
                    || (self.confirmed.is_none() && self.hovered_branch.is_some_and(|hi| hi != bi));
                let is_hot = self.hovered_branch == Some(bi) || is_chosen;

                let br_border = if is_chosen {
                    p.green
                } else if branch.pick {
                    p.blue.opacity(if is_hot { 0.65 } else { 0.38 })
                } else if is_hot {
                    p.line_strong
                } else {
                    p.line
                };

                let br_sensor = bounds_map.branches[bi].clone();
                let lf_sensor = bounds_map.leaves[bi].clone();
                let pick_tag = tr(cx, "Agent's pick", "智能体推荐");

                let opt_button = div()
                    .when(!stacked, |d| d.w(px(192.0)).flex_none())
                    .when(stacked, |d| d.w_full())
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            br_sensor.set(Some(*b));
                        }
                    })
                    .child(
                        div()
                            .id(pro_eid(&self.id, format!("dt-br-{bi}")))
                            .role(Role::Button)
                            .aria_label(branch.label.clone())
                            .tab_index(0)
                            .w_full()
                            .px(px(12.0))
                            .py(px(9.0))
                            .rounded(px(8.0))
                            .bg(card_bg)
                            .border_1()
                            .border_color(br_border)
                            .when(!p.dark, |d| d.shadow_xs())
                            .flex()
                            .items_start()
                            .gap(px(8.0))
                            .cursor_pointer()
                            .focus_visible(move |s| s.border_color(p.focus))
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_branch = Some(bi);
                                } else if this.hovered_branch == Some(bi) {
                                    this.hovered_branch = None;
                                }
                                cx.notify();
                            }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.choose_branch(bi, None, cx);
                            }))
                            .child(
                                div()
                                    .size(px(17.0))
                                    .mt(px(1.0))
                                    .flex_none()
                                    .rounded(px(4.5))
                                    .bg(fill_bg)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(format!("{}", bi + 1)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .line_height(px(18.0))
                                            .text_color(p.ink)
                                            .child(branch.label.clone()),
                                    )
                                    .when(branch.pick, |d| {
                                        d.child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.blue)
                                                .child(pick_tag),
                                        )
                                    }),
                            ),
                    );

                // Right-hand Leaf or Sub-choices or Confirmed Outcome Card
                let right_node = if is_chosen && let Some(conf) = &self.confirmed {
                    let next_lbl = if open_len > 1 {
                        tr(cx, "Next decision →", "下一项决策 →")
                    } else {
                        tr(cx, "Done", "完成")
                    };
                    div()
                        .px(px(12.0))
                        .py(px(9.0))
                        .rounded(px(8.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.green.opacity(0.5))
                        .flex()
                        .flex_col()
                        .gap(px(5.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.green_ink)
                                .child(pro_icon(ProIcon::Check, 13.0, p.green))
                                .child(conf.summary.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.text)
                                .child(conf.agent_reply.clone()),
                        )
                        .child(
                            div()
                                .id(pro_eid(&self.id, "dt-advance"))
                                .role(Role::Button)
                                .aria_label(next_lbl)
                                .tab_index(0)
                                .mt(px(2.0))
                                .self_start()
                                .h(px(24.0))
                                .px(px(9.0))
                                .rounded(px(6.0))
                                .bg(p.primary)
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.on_primary)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.advance_after_confirm(cx);
                                }))
                                .child(next_lbl),
                        )
                        .into_any_element()
                } else if let Some(sub_q) = &branch.sub_question {
                    let mut sub_col = div().flex().flex_col().items_start().gap(px(6.0));
                    sub_col = sub_col.child(
                        div()
                            .px(px(10.0))
                            .py(px(5.5))
                            .rounded(px(8.0))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(sub_q.clone()),
                    );
                    let mut sub_list = div()
                        .pl(px(14.0))
                        .border_l_1()
                        .border_color(p.line_strong)
                        .flex()
                        .flex_col()
                        .gap(px(5.0));
                    for (si, so) in branch.sub_options.iter().enumerate() {
                        let eff_el =
                            self.render_consequence_line(&so.effect, format!("s-{bi}-{si}"), cx);
                        let apps_el = render_app_cluster(&so.apps, card_bg, &p, cx);
                        sub_list = sub_list.child(
                            div()
                                .id(pro_eid(&self.id, format!("dt-sub-{bi}-{si}")))
                                .role(Role::Button)
                                .aria_label(so.label.clone())
                                .tab_index(0)
                                .px(px(10.0))
                                .py(px(6.0))
                                .rounded(px(8.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(if so.pick {
                                    p.blue.opacity(0.35)
                                } else {
                                    p.line
                                })
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .cursor_pointer()
                                .hover(move |s| s.border_color(p.line_strong))
                                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                    if *inside {
                                        this.hovered_branch = Some(bi);
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.choose_branch(bi, Some(si), cx);
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(7.0))
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(so.label.clone()),
                                        )
                                        .when(so.pick, |d| {
                                            d.child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(p.blue)
                                                    .child(pick_tag),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(eff_el)
                                        .when_some(apps_el, |d, a| d.child(a)),
                                ),
                        );
                    }
                    sub_col.child(sub_list).into_any_element()
                } else {
                    let mut lines = div().flex_1().min_w_0().flex().flex_col().gap(px(2.0));
                    for (ei, eff) in branch.effects.iter().enumerate() {
                        lines = lines.child(self.render_consequence_line(
                            eff,
                            format!("e-{bi}-{ei}"),
                            cx,
                        ));
                    }
                    let apps_el = render_app_cluster(&branch.apps, card_bg, &p, cx);
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(8.0))
                        .bg(leaf_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(lines)
                        .when_some(apps_el, |d, a| d.child(a))
                        .into_any_element()
                };

                branches_col = branches_col.child(
                    div()
                        .w_full()
                        .opacity(if dimmed { 0.36 } else { 1.0 })
                        .flex()
                        .when(stacked, |d| d.flex_col().items_start().gap(px(8.0)))
                        .when(!stacked, |d| d.items_center().gap(px(26.0)))
                        .child(opt_button)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .when(stacked, |d| d.pl(px(14.0)))
                                .on_children_prepainted(move |bounds, _, _| {
                                    if let Some(b) = bounds.first() {
                                        lf_sensor.set(Some(*b));
                                    }
                                })
                                .child(right_node),
                        ),
                );
            }

            // Branch N+1: `"Other…"` custom instruction branch
            if self.allow_other {
                let oi = branch_len;
                let other_chosen = self.confirmed.as_ref().is_some_and(|c| c.branch_idx == oi);
                let dimmed = self.confirmed.as_ref().is_some_and(|c| c.branch_idx != oi)
                    || (self.confirmed.is_none()
                        && !self.other_open
                        && self.hovered_branch.is_some_and(|hi| hi != oi));
                let br_sensor = bounds_map.branches[oi].clone();
                let lf_sensor = bounds_map.leaves[oi].clone();
                let other_lbl = tr(cx, "Other…", "其他路径…");

                let other_left = if self.other_open {
                    let ed_view = self.other_editor.clone();
                    let confirm_lbl = tr(cx, "Confirm", "确认发送");
                    div()
                        .w_full()
                        .p(px(9.0))
                        .rounded(px(8.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.focus)
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(div().h(px(42.0)).when_some(ed_view, |d, ed| d.child(ed)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(p.muted)
                                        .child("↵ / ⌘↵"),
                                )
                                .child(
                                    div()
                                        .id(pro_eid(&self.id, "dt-other-confirm"))
                                        .role(Role::Button)
                                        .aria_label(confirm_lbl)
                                        .tab_index(0)
                                        .h(px(24.0))
                                        .px(px(9.0))
                                        .rounded(px(6.0))
                                        .bg(p.primary)
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.on_primary)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                                            if let Some(ed) = &this.other_editor {
                                                let val = ed.read(cx).value().trim().to_owned();
                                                if !val.is_empty() {
                                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                                    this.submit_custom(val.into(), cx);
                                                }
                                            }
                                        }))
                                        .child(confirm_lbl),
                                ),
                        )
                        .into_any_element()
                } else {
                    div()
                        .id(pro_eid(&self.id, "dt-other-btn"))
                        .role(Role::Button)
                        .aria_label(other_lbl)
                        .tab_index(0)
                        .w_full()
                        .h(px(36.0))
                        .px(px(12.0))
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(if other_chosen { p.green } else { p.line_strong })
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(card_bg))
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_branch = Some(oi);
                            } else if this.hovered_branch == Some(oi) {
                                this.hovered_branch = None;
                            }
                            cx.notify();
                        }))
                        .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                            this.other_open = true;
                            this.ensure_editor(w, cx);
                            if let Some(ed) = &this.other_editor {
                                let fh = ed.read(cx).focus.clone();
                                fh.focus(w, cx);
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .size(px(17.0))
                                .rounded(px(4.5))
                                .bg(fill_bg)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(format!("{}", oi + 1)),
                        )
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(p.muted)
                                .child(other_lbl),
                        )
                        .into_any_element()
                };

                let other_right = other_chosen.then(|| {
                    let conf = self.confirmed.as_ref();
                    let next_lbl = if open_len > 1 {
                        tr(cx, "Next decision →", "下一项决策 →")
                    } else {
                        tr(cx, "Done", "完成")
                    };
                    div()
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(b) = bounds.first() {
                                lf_sensor.set(Some(*b));
                            }
                        })
                        .child(
                            div()
                                .px(px(12.0))
                                .py(px(9.0))
                                .rounded(px(8.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.green.opacity(0.5))
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .when_some(conf, |d, c| {
                                    d.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.green_ink)
                                            .child(pro_icon(ProIcon::Check, 13.0, p.green))
                                            .child(c.summary.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(p.text)
                                            .child(c.agent_reply.clone()),
                                    )
                                })
                                .child(
                                    div()
                                        .id(pro_eid(&self.id, "dt-other-adv"))
                                        .role(Role::Button)
                                        .aria_label(next_lbl)
                                        .tab_index(0)
                                        .mt(px(2.0))
                                        .self_start()
                                        .h(px(24.0))
                                        .px(px(9.0))
                                        .rounded(px(6.0))
                                        .bg(p.primary)
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.on_primary)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.advance_after_confirm(cx);
                                        }))
                                        .child(next_lbl),
                                ),
                        )
                });

                branches_col = branches_col.child(
                    div()
                        .w_full()
                        .opacity(if dimmed { 0.38 } else { 1.0 })
                        .flex()
                        .when(stacked, |d| d.flex_col().items_start().gap(px(8.0)))
                        .when(!stacked, |d| d.items_center().gap(px(26.0)))
                        .child(
                            div()
                                .when(!stacked, |d| {
                                    d.w(px(if self.other_open { 248.0 } else { 192.0 }))
                                        .flex_none()
                                })
                                .when(stacked, |d| d.w_full())
                                .on_children_prepainted(move |bounds, _, _| {
                                    if let Some(b) = bounds.first() {
                                        br_sensor.set(Some(*b));
                                    }
                                })
                                .child(other_left),
                        )
                        .when_some(other_right, |d, r| d.child(r)),
                );
            }

            // Flagged note callout banner (when hovered or toggled via `F`)
            let flag_banner = self.active_flag_note.as_ref().map(|(note_txt, src_opt)| {
                div()
                    .w_full()
                    .max_w(px(420.0))
                    .p(px(10.0))
                    .rounded(px(9.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.amber.opacity(0.45))
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.amber_ink)
                            .child(pro_icon(ProIcon::AlertTriangle, 12.0, p.amber_ink))
                            .child(tr(cx, "AGENT RISK NOTE", "智能体风险注解")),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.0))
                            .text_color(p.ink)
                            .child(note_txt.clone()),
                    )
                    .when_some(src_opt.clone(), |d, src| {
                        d.child(
                            div()
                                .self_start()
                                .px(px(6.0))
                                .py(px(1.0))
                                .rounded(px(4.0))
                                .bg(fill_bg)
                                .font_family("Menlo")
                                .text_size(px(11.0))
                                .text_color(p.muted)
                                .child(src),
                        )
                    })
            });

            // Footer keyboard hint bar
            let hint_bar = div()
                .w_full()
                .pt(px(8.0))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .text_size(px(12.0))
                        .text_color(p.muted)
                        .child(tr(cx, "Pick a branch", "按数字键选分支"))
                        .child(
                            div()
                                .px(px(5.0))
                                .h(px(16.0))
                                .rounded(px(4.0))
                                .bg(fill_bg)
                                .flex()
                                .items_center()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("1–{total_rows}")),
                        )
                        .child("·")
                        .child(tr(cx, "Flag note", "查看风险注解"))
                        .child(
                            div()
                                .px(px(5.0))
                                .h(px(16.0))
                                .rounded(px(4.0))
                                .bg(fill_bg)
                                .flex()
                                .items_center()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child("F"),
                        )
                        .when(open_len > 1, |d| {
                            d.child("·").child(tr(cx, "Next", "切换")).child(
                                div()
                                    .px(px(5.0))
                                    .h(px(16.0))
                                    .rounded(px(4.0))
                                    .bg(fill_bg)
                                    .flex()
                                    .items_center()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("← →"),
                            )
                        }),
                )
                .when_some(flag_banner, |d, fb| d.child(fb));

            pro_step_transition(
                pro_eid(&self.id, "tree-step"),
                self.step_serial,
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .py(px(14.0))
                            .flex()
                            .when(stacked, |d| d.flex_col().items_start().gap(px(18.0)))
                            .when(!stacked, |d| d.items_center().gap(px(34.0)))
                            .when_some(connector_layer, |d, cl| d.child(cl))
                            .child(root_col)
                            .child(branches_col),
                    )
                    .child(hint_bar),
            )
        } else {
            let replay_lbl = tr(cx, "Replay 3 decisions", "重新体验 3 组决策树");
            div()
                .w_full()
                .py(px(44.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(
                    div()
                        .size(px(42.0))
                        .rounded(px(10.0))
                        .bg(p.green_soft)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Check, 18.0, p.green_ink)),
                )
                .child(
                    div()
                        .text_size(px(15.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(
                            cx,
                            "All branching decisions resolved",
                            "所有分支决策均已拍板",
                        )),
                )
                .child(div().text_size(px(13.0)).text_color(p.muted).child(tr(
                    cx,
                    "Incident, Billing, and Infra agents are executing your chosen paths.",
                    "故障处理、账单与基础设施智能体正沿你选定的分支执行。",
                )))
                .child(
                    div()
                        .id(pro_eid(&self.id, "dt-replay-all"))
                        .role(Role::Button)
                        .aria_label(replay_lbl)
                        .tab_index(0)
                        .mt(px(6.0))
                        .h(px(30.0))
                        .px(px(12.0))
                        .rounded(px(7.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .hover(move |s| s.bg(fill_bg))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.reset(cx);
                        }))
                        .child(pro_icon(ProIcon::Refresh, 12.5, p.muted))
                        .child(replay_lbl),
                )
                .into_any_element()
        };

        let width_sensor = self.measured_width.clone();
        let a11y_label = tr(
            cx,
            "Interactive agent branching decision tree",
            "交互式智能体分支决策树",
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
            .focus_visible(move |s| s.rounded(px(14.0)).border_color(focus_ring))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if this.other_focused || this.decisions.is_empty() {
                    return;
                }
                let cur = this.cursor.min(this.decisions.len() - 1);
                let b_len = this.decisions[cur].branches.len().min(4);
                let key = ev.keystroke.key.as_str();

                if let Some(d) = match key {
                    "1" => Some(1usize),
                    "2" => Some(2usize),
                    "3" => Some(3usize),
                    "4" => Some(4usize),
                    "5" => Some(5usize),
                    _ => None,
                } {
                    if d <= b_len {
                        this.choose_branch(d - 1, None, cx);
                        cx.stop_propagation();
                    } else if d == b_len + 1 && this.allow_other {
                        this.other_open = true;
                        this.ensure_editor(w, cx);
                        if let Some(ed) = &this.other_editor {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                    return;
                }

                match key {
                    "right" | "j" => {
                        if this.confirmed.is_some() {
                            this.advance_after_confirm(cx);
                        } else {
                            this.step_decision(1, cx);
                        }
                        cx.stop_propagation();
                    }
                    "left" | "k" => {
                        this.step_decision(-1, cx);
                        cx.stop_propagation();
                    }
                    "f" => {
                        this.cycle_flag_note(cx);
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.other_open = false;
                        this.active_flag_note = None;
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(800.0))
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            let w_px = f32::from(b.size.width);
                            if w_px > 20.0 {
                                width_sensor.set(w_px);
                            }
                        }
                    })
                    .child(header_bar)
                    .child(tree_stage),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{BranchConsequence, DecisionBranch, DecisionTree, DecisionTreeRisk, TreeDecision};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn decision_tree_selects_branch_custom_path_and_advances(cx: &mut TestAppContext) {
        let tree = cx.new(|cx| DecisionTree::new("test-decision-tree", cx));

        tree.update(cx, |t, cx| {
            assert_eq!(t.remaining_count(), 3);
            t.choose_branch(0, Some(0), cx);
            t.advance_after_confirm(cx);
            assert_eq!(t.remaining_count(), 2);

            t.submit_custom("Pause billing webhooks for 15m".into(), cx);
            t.advance_after_confirm(cx);
            assert_eq!(t.remaining_count(), 1);

            t.reset(cx);
            assert_eq!(t.remaining_count(), 3);
        });

        let d = TreeDecision::new("d1", "Ops", "Scale up?")
            .with_risk(DecisionTreeRisk::Medium)
            .with_branches(vec![
                DecisionBranch::new("Yes")
                    .with_pick(true)
                    .with_effects(vec![BranchConsequence::plain("+2 replicas")]),
            ]);
        assert_eq!(d.branches.len(), 1);
    }
}
