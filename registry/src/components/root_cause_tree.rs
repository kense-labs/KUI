//! Fault-tree diagnostic explorer stepping through hypotheses, ruling branches out, and lighting the confirmed causal path.
//!
//! Renders the top failure symptom branching downward through `OR` and `AND` logic gates into
//! fixed-height hypothesis cards. As hypotheses are checked, ruled-out branches fade onto dashed
//! rails while confirmed root causes illuminate the full causal chain back up to the top failure
//! and expose diffs, logs, and `Fix it` / `Retry from here` actions in the evidence panel below.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_connector_pulse, pro_eid, pro_empty_state,
        pro_shimmer_text, pro_skeleton_bar, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const FAULT_TREE_MARK_SVG: &str = r#"<path d="M12 7.8v3.4M6 15.8v-2.9h12v2.9" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" opacity="0.5"/><rect x="8.1" y="2.4" width="7.8" height="5.4" rx="1.8" fill="currentColor"/><circle cx="6" cy="18.6" r="2.8" fill="currentColor"/><circle cx="18" cy="18.6" r="2.8" fill="currentColor"/>"#;
const FILE_CHIP_SVG: &str = r#"<path d="M4.8 3.8c0-1.1.9-2 2-2h6.8l5.6 5.6v12.8c0 1.1-.9 2-2 2H6.8c-1.1 0-2-.9-2-2Z" fill="currentColor" opacity="0.45"/><path d="M13.6 1.8v3.8c0 1 .8 1.8 1.8 1.8h3.8Z" fill="currentColor"/>"#;
const HOLLOW_RING_SVG: &str =
    r#"<circle cx="12" cy="12" r="7" stroke="currentColor" stroke-width="2.4"/>"#;
const RULED_OUT_SVG: &str = r#"<circle cx="12" cy="12" r="8" stroke="currentColor" stroke-width="2"/><path d="M8.5 12h7" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#;

fn spinning_dot(side: f32, stroke: f32, color: Hsla, elapsed_s: f32, spin: bool) -> AnyElement {
    let angle = if spin {
        (elapsed_s / 0.85) * std::f32::consts::TAU
    } else {
        -std::f32::consts::FRAC_PI_2
    };
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let c = bounds.center();
            let r = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) - stroke)
                * 0.5_f32;
            if r <= 1.0 {
                return;
            }
            let mut ring = PathBuilder::stroke(px(stroke));
            for i in 0..=20usize {
                let a = (i as f32 / 20.0) * std::f32::consts::TAU;
                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
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
            for i in 0..=7usize {
                let a = angle + (i as f32 / 7.0) * std::f32::consts::FRAC_PI_2;
                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
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

fn paint_dashed_tree_link(
    parent_bottom: Point<Pixels>,
    child_top: Point<Pixels>,
    color: Hsla,
    w: &mut Window,
) {
    let x0 = f32::from(parent_bottom.x);
    let y0 = f32::from(parent_bottom.y);
    let x1 = f32::from(child_top.x);
    let y1 = f32::from(child_top.y);
    let ym = (y0 + y1) * 0.5_f32;
    let segs = [
        (point(px(x0), px(y0)), point(px(x0), px(ym))),
        (point(px(x0), px(ym)), point(px(x1), px(ym))),
        (point(px(x1), px(ym)), point(px(x1), px(y1))),
    ];
    for (a, b) in segs {
        let ax = f32::from(a.x);
        let ay = f32::from(a.y);
        let bx = f32::from(b.x);
        let by = f32::from(b.y);
        let dx = bx - ax;
        let dy = by - ay;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.5 {
            continue;
        }
        let step = 6.5_f32;
        let count = (len / step).ceil() as usize;
        for i in 0..count {
            let t0 = (i as f32 * step) / len;
            let t1 = ((i as f32 * step + 2.8) / len).min(1.0);
            if t1 <= t0 {
                continue;
            }
            let mut pb = PathBuilder::stroke(px(1.5));
            pb.move_to(point(px(ax + dx * t0), px(ay + dy * t0)));
            pb.line_to(point(px(ax + dx * t1), px(ay + dy * t1)));
            if let Ok(path) = pb.build() {
                w.paint_path(path, color);
            }
        }
    }
}

/// Combinator gate linking a parent cause to its child hypotheses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CauseGate {
    /// Any single confirmed child suffices to confirm the parent.
    #[default]
    Or,
    /// Every child must be confirmed together to confirm the parent.
    And,
}

/// Diagnostic verification state of a [`CauseNode`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CauseStatus {
    /// Hypothesis queued and not yet checked.
    #[default]
    Open,
    /// Agent is actively testing this hypothesis right now.
    Checking,
    /// Hypothesis was falsified and ruled out (kept visible on a dashed branch).
    RuledOut,
    /// Hypothesis was verified as contributing to the failure.
    Confirmed,
}

/// Monospace proof artifact attached to a [`CauseNode`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CauseEvidence {
    /// Kind label (`"Diff"`, `"Log"`, `"Tool call"`).
    pub kind: SharedString,
    /// Optional source file or command label (`"src/checkout/coupon.ts"`).
    pub source: Option<SharedString>,
    /// Multi-line proof content (diffs highlight `-` / `+` lines automatically).
    pub content: SharedString,
}

impl CauseEvidence {
    /// Construct a [`CauseEvidence`] block with `kind` and `content`.
    pub fn new(kind: impl Into<SharedString>, content: impl Into<SharedString>) -> Self {
        Self {
            kind: kind.into(),
            source: None,
            content: content.into(),
        }
    }

    /// Attach a source file path or command label to the evidence header.
    pub fn with_source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Node in the fault tree of [`RootCauseTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CauseNode {
    /// Stable node identifier.
    pub id: SharedString,
    /// Concise symptom or hypothesis title.
    pub title: SharedString,
    /// Optional metric detail next to the top failure badge (for example `"4 of 12"`).
    pub detail: Option<SharedString>,
    /// Optional brand/tool icon key (`"stripe"`, `"github"`, `"terminal"`, `"code"`, `"database"`).
    pub icon_key: SharedString,
    /// Files touched or inspected by this hypothesis.
    pub files: Vec<SharedString>,
    /// Logic gate combining `children` (`Or` by default).
    pub gate: CauseGate,
    /// Current verification status.
    pub status: CauseStatus,
    /// One-sentence explanation shown when ruled out or confirmed.
    pub verdict: SharedString,
    /// Optional code diff, log excerpt, or tool output proof.
    pub evidence: Option<CauseEvidence>,
    /// Child hypotheses branching underneath this node.
    pub children: Vec<CauseNode>,
}

impl CauseNode {
    /// Construct a [`CauseNode`] with `id` and `title`.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: None,
            icon_key: "code".into(),
            files: Vec::new(),
            gate: CauseGate::Or,
            status: CauseStatus::Open,
            verdict: SharedString::default(),
            evidence: None,
            children: Vec::new(),
        }
    }

    /// Set the short failure detail string (used on the root symptom node, e.g. `"4 of 12"`).
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the service/tool icon key (`"stripe"`, `"github"`, `"terminal"`, `"code"`, `"database"`).
    pub fn with_icon(mut self, icon_key: impl Into<SharedString>) -> Self {
        self.icon_key = icon_key.into();
        self
    }

    /// Attach file paths touched by this hypothesis (basename of the first shows on the card).
    pub fn with_files<I, S>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.files = files.into_iter().map(Into::into).collect();
        self
    }

    /// Set the `OR` / `AND` gate for child hypotheses.
    pub fn with_gate(mut self, gate: CauseGate) -> Self {
        self.gate = gate;
        self
    }

    /// Set the verification status (`Open`, `Checking`, `RuledOut`, `Confirmed`).
    pub fn with_status(mut self, status: CauseStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the one-sentence diagnostic verdict.
    pub fn with_verdict(mut self, verdict: impl Into<SharedString>) -> Self {
        self.verdict = verdict.into();
        self
    }

    /// Attach a [`CauseEvidence`] diff or log block.
    pub fn with_evidence(mut self, evidence: CauseEvidence) -> Self {
        self.evidence = Some(evidence);
        self
    }

    /// Attach child hypotheses beneath this node.
    pub fn with_children(mut self, children: Vec<CauseNode>) -> Self {
        self.children = children;
        self
    }
}

/// Remediation action triggered from a confirmed root cause in [`RootCauseTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootCauseAction {
    /// Apply the proposed fix for this root cause.
    Fix,
    /// Retry the run from this checkpoint/hypothesis.
    Retry,
}

/// Typed events emitted by [`RootCauseTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootCauseTreeEvent {
    /// Focused/inspected hypothesis node changed.
    NodeSelected(SharedString),
    /// Diagnostic playback advanced to `step`.
    InvestigationStep(usize),
    /// Operator clicked `Fix it` or `Retry from here` on a confirmed root cause.
    ActionTriggered {
        cause_id: SharedString,
        action: RootCauseAction,
    },
    /// Diagnostic sequence reset.
    Reset,
}

fn node_icon(key: &str, side: f32, muted: bool, p: &ProPalette, cx: &App) -> AnyElement {
    if !muted && let Some(mark) = pro_app_artwork(key, side, cx) {
        return mark;
    }
    let col = if muted { p.muted } else { p.ink };
    let ic = match key.to_ascii_lowercase().as_str() {
        "terminal" | "test" => ProIcon::Terminal,
        "database" | "db" => ProIcon::Database,
        "stripe" => ProIcon::CreditCard,
        "github" => ProIcon::GitBranch,
        _ => ProIcon::Code,
    };
    pro_icon(ic, side, col)
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn default_failure_template() -> CauseNode {
    CauseNode::new("failure", "Checkout tests failing")
        .with_detail("4 of 12")
        .with_icon("terminal")
        .with_files(["tests/checkout.test.ts"])
        .with_gate(CauseGate::Or)
        .with_verdict(
            "4 percentage-coupon assertions failed because discounted totals were serialized as cents and overwritten by cart recomputation.",
        )
        .with_evidence(
            CauseEvidence::new(
                "Log",
                "FAIL tests/checkout.test.ts > applies 20% annual promo code\nExpected: 7680\nReceived: 9600",
            )
            .with_source("tests/checkout.test.ts:84"),
        )
        .with_children(vec![
            CauseNode::new("coupon", "Coupon ignored")
                .with_icon("stripe")
                .with_files(["src/checkout/coupon.ts", "src/cart/total.ts"])
                .with_gate(CauseGate::And)
                .with_verdict(
                    "Both defects combined: coupon percent was mapped to amount_off AND cart recomputation discarded the discount.",
                )
                .with_children(vec![
                    CauseNode::new("sent-amount", "Sent as amount")
                        .with_icon("code")
                        .with_files(["src/checkout/coupon.ts"])
                        .with_verdict(
                            "20% promo code was passed as amount_off: 20 instead of percent_off: 20.",
                        )
                        .with_evidence(
                            CauseEvidence::new(
                                "Diff",
                                "- return { amount_off: promo.rate, currency: \"usd\" };\n+ return promo.kind === \"percent\"\n+   ? { percent_off: promo.rate }\n+   : { amount_off: promo.rate, currency: \"usd\" };",
                            )
                            .with_source("src/checkout/coupon.ts:42"),
                        ),
                    CauseNode::new("cart-drops", "Cart drops it")
                        .with_icon("database")
                        .with_files(["src/cart/total.ts", "src/cart/session.ts"])
                        .with_verdict(
                            "recomputeTotal() ran after applyCoupon() and reset state.payable back to raw subtotal.",
                        )
                        .with_evidence(
                            CauseEvidence::new(
                                "Diff",
                                "- state.payable = sumLineItems(state.items);\n+ state.payable = applyActiveCoupon(sumLineItems(state.items), state.coupon);",
                            )
                            .with_source("src/cart/total.ts:19"),
                        ),
                ]),
            CauseNode::new("stripe-err", "Stripe errors")
                .with_icon("stripe")
                .with_files(["logs/stripe.log"])
                .with_verdict("Every Stripe test-mode call returned 200 OK in under 190ms.")
                .with_evidence(
                    CauseEvidence::new(
                        "Log",
                        "POST /v1/checkout/sessions -> 200 OK (164ms)\nPOST /v1/coupons -> 200 OK (118ms)",
                    )
                    .with_source("logs/stripe.log"),
                ),
            CauseNode::new("stale-fix", "Stale fixtures")
                .with_icon("github")
                .with_files(["tests/fixtures/prices.json"])
                .with_verdict("Fixture SHA matches staging price catalog; no schema drift.")
                .with_evidence(
                    CauseEvidence::new(
                        "Tool call",
                        "$ git diff HEAD~3 -- tests/fixtures/prices.json\n(0 lines changed)",
                    )
                    .with_source("tests/fixtures/prices.json"),
                ),
        ])
}

/// Apply progressive investigation step `0..=4` onto the default checkout fault tree.
fn apply_demo_step(root: &mut CauseNode, step: usize) {
    // Children indices: [0] = coupon ([0]=sent-amount, [1]=cart-drops), [1] = stripe-err, [2] = stale-fix
    if root.children.len() < 3 || root.children[0].children.len() < 2 {
        return;
    }
    match step {
        0 => {
            root.children[1].status = CauseStatus::Checking;
        }
        1 => {
            root.children[1].status = CauseStatus::RuledOut;
            root.children[2].status = CauseStatus::Checking;
        }
        2 => {
            root.children[1].status = CauseStatus::RuledOut;
            root.children[2].status = CauseStatus::RuledOut;
            root.children[0].status = CauseStatus::Checking;
            root.children[0].children[0].status = CauseStatus::Checking;
        }
        3 => {
            root.children[1].status = CauseStatus::RuledOut;
            root.children[2].status = CauseStatus::RuledOut;
            root.children[0].status = CauseStatus::Checking;
            root.children[0].children[0].status = CauseStatus::Confirmed;
            root.children[0].children[1].status = CauseStatus::Checking;
        }
        _ => {
            root.children[1].status = CauseStatus::RuledOut;
            root.children[2].status = CauseStatus::RuledOut;
            root.children[0].children[0].status = CauseStatus::Confirmed;
            root.children[0].children[1].status = CauseStatus::Confirmed;
            root.children[0].status = CauseStatus::Confirmed;
            root.status = CauseStatus::Confirmed;
        }
    }
}

fn evaluate_causal_path(node: &CauseNode) -> bool {
    if node.children.is_empty() {
        return node.status == CauseStatus::Confirmed;
    }
    match node.gate {
        CauseGate::Or => node.children.iter().any(evaluate_causal_path),
        CauseGate::And => node.children.iter().all(evaluate_causal_path),
    }
}

fn count_hypotheses(node: &CauseNode) -> (usize, usize, bool) {
    let mut total = 0usize;
    let mut ruled = 0usize;
    let mut checking = false;
    for ch in &node.children {
        total += 1;
        if ch.status == CauseStatus::RuledOut {
            ruled += 1;
        }
        if ch.status == CauseStatus::Checking {
            checking = true;
        }
        let (ct, cr, cc) = count_hypotheses(ch);
        total += ct;
        ruled += cr;
        checking |= cc;
    }
    (total, ruled, checking)
}

fn find_node<'a>(node: &'a CauseNode, id: &str) -> Option<&'a CauseNode> {
    if node.id.as_ref() == id {
        return Some(node);
    }
    for ch in &node.children {
        if let Some(found) = find_node(ch, id) {
            return Some(found);
        }
    }
    None
}

fn find_active_focus_id(node: &CauseNode) -> SharedString {
    // Prefer a Checking leaf, then a Confirmed leaf root cause, else root.
    fn walk_status(n: &CauseNode, target: CauseStatus) -> Option<SharedString> {
        for ch in &n.children {
            if let Some(found) = walk_status(ch, target) {
                return Some(found);
            }
        }
        if n.children.is_empty() && n.status == target {
            return Some(n.id.clone());
        }
        None
    }
    walk_status(node, CauseStatus::Checking)
        .or_else(|| walk_status(node, CauseStatus::Confirmed))
        .unwrap_or_else(|| node.id.clone())
}

#[derive(Clone)]
struct PlacedNode {
    node: CauseNode,
    is_top_failure: bool,
    on_causal_path: bool,
    depth: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone)]
struct PlacedLink {
    p_parent: (f32, f32),
    p_child: (f32, f32),
    status: CauseStatus,
    on_causal_path: bool,
}

#[derive(Clone)]
struct PlacedGate {
    gate: CauseGate,
    x: f32,
    y: f32,
    on_causal_path: bool,
}

fn leaf_weight(n: &CauseNode) -> usize {
    if n.children.is_empty() {
        1
    } else {
        n.children.iter().map(leaf_weight).sum::<usize>().max(1)
    }
}

fn layout_fault_tree(
    root: &CauseNode,
    stage_w: f32,
) -> (Vec<PlacedNode>, Vec<PlacedLink>, Vec<PlacedGate>, f32) {
    let total_leaves = leaf_weight(root).max(1) as f32;
    let gap_x = 12.0_f32;
    let hyp_w = ((stage_w - gap_x * (total_leaves - 1.0)) / total_leaves).clamp(132.0, 172.0);
    let root_w = 212.0_f32.min(stage_w);
    let card_h = 84.0_f32;
    let level_gap = 44.0_f32;
    let total_span_w = total_leaves * hyp_w + (total_leaves - 1.0).max(0.0) * gap_x;
    let start_x = ((stage_w - total_span_w) * 0.5_f32).max(0.0);

    let mut nodes = Vec::new();
    let mut links = Vec::new();
    let mut gates = Vec::new();
    let mut max_y = card_h;

    #[allow(clippy::too_many_arguments)]
    fn place(
        n: &CauseNode,
        depth: usize,
        leaf_cursor: &mut usize,
        start_x: f32,
        hyp_w: f32,
        root_w: f32,
        gap_x: f32,
        card_h: f32,
        level_gap: f32,
        parent_causal: bool,
        nodes: &mut Vec<PlacedNode>,
        links: &mut Vec<PlacedLink>,
        gates: &mut Vec<PlacedGate>,
        max_y: &mut f32,
    ) -> (f32, f32, bool) {
        let own_causal = parent_causal && evaluate_causal_path(n);
        let w = if depth == 0 { root_w } else { hyp_w };
        let y = depth as f32 * (card_h + level_gap);
        *max_y = max_y.max(y + card_h);

        let cx_center = if n.children.is_empty() {
            let slot = *leaf_cursor as f32;
            *leaf_cursor += 1;
            start_x + slot * (hyp_w + gap_x) + hyp_w * 0.5_f32
        } else {
            let mut child_centers = Vec::with_capacity(n.children.len());
            for ch in &n.children {
                let (ccx, ccy, ch_causal) = place(
                    ch,
                    depth + 1,
                    leaf_cursor,
                    start_x,
                    hyp_w,
                    root_w,
                    gap_x,
                    card_h,
                    level_gap,
                    own_causal,
                    nodes,
                    links,
                    gates,
                    max_y,
                );
                child_centers.push((ccx, ccy, ch.status, ch_causal));
            }
            let first_x = child_centers.first().map(|c| c.0).unwrap_or(0.0);
            let last_x = child_centers.last().map(|c| c.0).unwrap_or(first_x);
            let my_cx = (first_x + last_x) * 0.5_f32;
            let p_bot = (my_cx, y + card_h);
            for (ccx, ccy, st, ch_causal) in &child_centers {
                links.push(PlacedLink {
                    p_parent: p_bot,
                    p_child: (*ccx, *ccy),
                    status: *st,
                    on_causal_path: own_causal && *ch_causal,
                });
            }
            if n.children.len() > 1 {
                gates.push(PlacedGate {
                    gate: n.gate,
                    x: my_cx,
                    y: y + card_h + level_gap * 0.5_f32,
                    on_causal_path: own_causal,
                });
            }
            my_cx
        };

        nodes.push(PlacedNode {
            node: n.clone(),
            is_top_failure: depth == 0,
            on_causal_path: own_causal,
            depth,
            x: cx_center - w * 0.5_f32,
            y,
            w,
            h: card_h,
        });
        (cx_center, y, own_causal)
    }

    let root_causal = evaluate_causal_path(root);
    let mut cursor = 0usize;
    place(
        root,
        0,
        &mut cursor,
        start_x,
        hyp_w,
        root_w,
        gap_x,
        card_h,
        level_gap,
        root_causal,
        &mut nodes,
        &mut links,
        &mut gates,
        &mut max_y,
    );

    (nodes, links, gates, max_y)
}

/// Fault-tree diagnostic explorer with live verification steps, OR/AND gates, causal path lighting, and evidence panel.
pub struct RootCauseTree {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    custom_failure: Option<CauseNode>,
    demo_step: usize,
    step_started: Instant,
    live: bool,
    selected_id: Option<SharedString>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl RootCauseTree {
    /// Create a [`RootCauseTree`] preloaded with the `"Checkout tests failing"` investigation.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Root cause".into(),
            custom_failure: None,
            demo_step: 0,
            step_started: now,
            live: true,
            selected_id: None,
            boot_at: now,
            measured_width: Rc::new(Cell::new(680.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the top bar heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Supply a custom root failure [`CauseNode`] tree (disables built-in demo stepping).
    pub fn with_failure(mut self, failure: CauseNode) -> Self {
        self.custom_failure = Some(failure);
        self.live = false;
        self
    }

    /// Set the starting demo step (`0..=4`, where `4` has confirmed the `AND` root causes and lit the causal chain).
    pub fn with_demo_step(mut self, step: usize) -> Self {
        self.demo_step = step.min(4);
        self
    }

    /// Enable or disable live automatic investigation stepping.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self.step_started = Instant::now();
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

    /// Resolve the current root [`CauseNode`] tree.
    pub fn current_failure(&self) -> CauseNode {
        if let Some(custom) = &self.custom_failure {
            return custom.clone();
        }
        let mut tree = default_failure_template();
        apply_demo_step(&mut tree, self.demo_step);
        tree
    }

    /// Whether a complete causal path from leaf root cause(s) up to the top failure is confirmed.
    pub fn is_root_cause_confirmed(&self) -> bool {
        evaluate_causal_path(&self.current_failure())
    }

    /// Select a specific node id to inspect its verdict and evidence (`None` follows active investigation).
    pub fn select_node(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        self.selected_id = id.clone();
        if let Some(nid) = id {
            cx.emit(RootCauseTreeEvent::NodeSelected(nid));
        }
        cx.notify();
    }

    /// Advance one step in the built-in diagnostic walkthrough (`0 -> 1 -> 2 -> 3 -> 4 -> 0`).
    pub fn step_investigation(&mut self, cx: &mut Context<Self>) {
        self.custom_failure = None;
        self.demo_step = (self.demo_step + 1) % 5;
        self.step_started = Instant::now();
        cx.emit(RootCauseTreeEvent::InvestigationStep(self.demo_step));
        cx.notify();
    }

    /// Reset investigation back to step `0`.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.demo_step = 0;
        self.selected_id = None;
        self.step_started = Instant::now();
        cx.emit(RootCauseTreeEvent::Reset);
        cx.notify();
    }

    /// Trigger `Fix it` or `Retry from here` on `cause_id`.
    pub fn trigger_action(
        &mut self,
        cause_id: impl Into<SharedString>,
        action: RootCauseAction,
        cx: &mut Context<Self>,
    ) {
        let cid: SharedString = cause_id.into();
        let msg: SharedString = match action {
            RootCauseAction::Fix => tr(
                cx,
                "Queued patch for root cause",
                "已为该根因生成修复补丁任务",
            )
            .into(),
            RootCauseAction::Retry => tr(
                cx,
                "Re-running verification from node",
                "正在从该节点重新运行验证",
            )
            .into(),
        };
        self.toast = Some((msg, Instant::now()));
        cx.emit(RootCauseTreeEvent::ActionTriggered {
            cause_id: cid,
            action,
        });
        cx.notify();
    }

    fn localize_str<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Root cause" => tr(cx, "Root cause", "根因诊断树"),
            "Checkout tests failing" => tr(cx, "Checkout tests failing", "结账测试套件失败"),
            "4 of 12" => tr(cx, "4 of 12", "12 项中 4 项失败"),
            "Coupon ignored" => tr(cx, "Coupon ignored", "优惠券未生效"),
            "Sent as amount" => tr(cx, "Sent as amount", "折扣误传为固定金额"),
            "Cart drops it" => tr(cx, "Cart drops it", "重算总价覆盖折扣"),
            "Stripe errors" => tr(cx, "Stripe errors", "Stripe 接口报错"),
            "Stale fixtures" => tr(cx, "Stale fixtures", "测试夹具过期"),
            other => other,
        }
    }
}

impl EventEmitter<RootCauseTreeEvent> for RootCauseTree {}

impl Focusable for RootCauseTree {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for RootCauseTree {
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
                .child(pro_skeleton_bar(200., 18., 6., phase, cx))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(pro_skeleton_bar(204., 84., 10., phase + 0.1, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(160., 84., 10., phase + 0.2, cx))
                        .child(pro_skeleton_bar(160., 84., 10., phase + 0.3, cx))
                        .child(pro_skeleton_bar(160., 84., 10., phase + 0.4, cx)),
                )
                .child(pro_skeleton_bar(660., 96., 12., phase + 0.5, cx).w_full())
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
                    ProIcon::AlertTriangle,
                    tr(cx, "No failed run to diagnose", "暂无需要诊断的失败运行"),
                    tr(
                        cx,
                        "Pass a failure symptom and candidate hypotheses to trace the causal chain.",
                        "传入顶层故障现象与候选假设树即可自动追踪根因链路。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-rct"),
                        tr(cx, "Load sample diagnosis", "加载示例故障树").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.reset(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        // Auto-advance demo steps smoothly when live (pausing longer on step 4 so the lit causal chain is admired!)
        if self.live && !reduced && self.custom_failure.is_none() && self.selected_id.is_none() {
            let hold = if self.demo_step == 4 {
                4.2_f32
            } else {
                2.1_f32
            };
            if self.step_started.elapsed().as_secs_f32() >= hold {
                self.step_investigation(cx);
            }
            w.request_animation_frame();
        }

        let failure = self.current_failure();
        let (total_hyp, ruled_out, any_checking) = count_hypotheses(&failure);
        let root_found = evaluate_causal_path(&failure);

        if (any_checking || root_found) && !reduced {
            w.request_animation_frame();
        }

        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let pulse_t = if reduced {
            None
        } else {
            Some(((elapsed_s / 1.35).fract()).clamp(0.0, 1.0))
        };

        let active_id = self
            .selected_id
            .clone()
            .unwrap_or_else(|| find_active_focus_id(&failure));

        let width_cell = self.measured_width.clone();
        let stage_w = width_cell.get().max(320.0);
        let narrow = stage_w < 580.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.07)
        } else {
            hsla(0., 0., 0., 0.05)
        };
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.035)
        } else {
            hsla(0., 0., 0., 0.025)
        };
        let rail_col: Hsla = if p.dark {
            rgb(0x3a3a3f).into()
        } else {
            rgb(0xd9d9dc).into()
        };
        let path_blue: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0190fe).into()
        };
        let brand_bg: Hsla = if p.dark {
            rgba(0x0077ff3a).into()
        } else {
            rgba(0x008ff519).into()
        };
        let brand_fg: Hsla = if p.dark {
            rgb(0x70b8ff).into()
        } else {
            rgb(0x006dcb).into()
        };
        let focus_col = p.focus;
        let hover_border = p.line_strong;

        // 1. Header Bar
        let sum_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("{total_hyp} 项假设 · 已排除 {ruled_out} 项")
        } else {
            format!("{total_hyp} hypotheses · {ruled_out} ruled out")
        };

        let head_bar = div()
            .w_full()
            .pb(px(14.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(pro_svg("0 0 24 24", FAULT_TREE_MARK_SVG, 16., p.muted))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.localize_str(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(any_checking, |d| {
                        d.child(
                            div()
                                .h(px(18.))
                                .pl(px(5.))
                                .pr(px(6.))
                                .rounded(px(4.))
                                .bg(brand_bg)
                                .text_color(brand_fg)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(spinning_dot(9.5, 1.5, brand_fg, elapsed_s, !reduced))
                                .child(tr(cx, "Investigating", "正在排查")),
                        )
                    })
                    .when(root_found && !any_checking, |d| {
                        d.child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .rounded(px(4.))
                                .bg(brand_bg)
                                .text_color(brand_fg)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .child(tr(cx, "Root cause found", "已定位根因")),
                        )
                    })
                    .child(div().text_size(px(12.)).text_color(p.muted).child(sum_txt)),
            );

        // Card renderer shared by tree & narrow indented view
        let render_card = |pn: &PlacedNode, full_width: bool, cx: &mut Context<Self>| {
            let n = &pn.node;
            let nid = n.id.clone();
            let is_sel = nid == active_id;
            let is_ruled = n.status == CauseStatus::RuledOut;
            let is_checking = n.status == CauseStatus::Checking;
            let is_leaf_root =
                !pn.is_top_failure && n.children.is_empty() && n.status == CauseStatus::Confirmed;

            let border_c = if is_sel {
                path_blue.opacity(0.65)
            } else if pn.on_causal_path {
                path_blue.opacity(0.45)
            } else if is_leaf_root {
                p.red.opacity(0.45)
            } else {
                p.line
            };

            let title_str = self.localize_str(&n.title, cx).to_owned();
            let first_file = n.files.first().map(|f| basename(f).to_owned());
            let extra_files = n.files.len().saturating_sub(1);

            let bottom_meta: AnyElement = if pn.is_top_failure {
                div()
                    .h(px(18.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .min_w_0()
                    .child(
                        div()
                            .h(px(16.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.red_soft)
                            .text_color(p.red_ink)
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(tr(cx, "Failed", "失败")),
                    )
                    .when_some(n.detail.as_ref(), |d, det| {
                        d.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(self.localize_str(det, cx).to_owned()),
                        )
                    })
                    .child(if root_found {
                        div()
                            .h(px(16.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(brand_bg)
                            .text_color(brand_fg)
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(tr(cx, "Found", "已定位"))
                            .into_any_element()
                    } else {
                        div()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(tr(cx, "Investigating", "排查中"))
                            .into_any_element()
                    })
                    .into_any_element()
            } else if is_leaf_root {
                div()
                    .h(px(18.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(pro_icon(ProIcon::AlertTriangle, 11., p.red_ink))
                    .child(
                        div()
                            .h(px(16.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.red_soft)
                            .text_color(p.red_ink)
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(tr(cx, "Root cause", "根因确认")),
                    )
                    .into_any_element()
            } else {
                let (glyph, txt, col): (AnyElement, &'static str, Hsla) = match n.status {
                    CauseStatus::Checking => (
                        spinning_dot(10., 1.5, brand_fg, elapsed_s, !reduced),
                        tr(cx, "Checking", "验证中"),
                        p.muted,
                    ),
                    CauseStatus::RuledOut => (
                        pro_svg("0 0 24 24", RULED_OUT_SVG, 10., p.muted),
                        tr(cx, "Ruled out", "已排除"),
                        p.muted,
                    ),
                    CauseStatus::Confirmed => (
                        pro_icon(ProIcon::Check, 11., brand_fg),
                        tr(cx, "Confirmed", "已证实"),
                        brand_fg,
                    ),
                    CauseStatus::Open => (
                        pro_svg("0 0 24 24", HOLLOW_RING_SVG, 10., p.faint),
                        tr(cx, "Not checked", "待验证"),
                        p.faint,
                    ),
                };
                div()
                    .h(px(18.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(glyph)
                    .child(div().text_size(px(11.)).text_color(col).child(txt))
                    .into_any_element()
            };

            let nid_click = nid.clone();
            activate(
                div()
                    .id(pro_eid(&self.id, format!("rcnode-{nid}")))
                    .role(Role::Button)
                    .aria_label(title_str.clone())
                    .aria_selected(is_sel)
                    .tab_index(0)
                    .when(!full_width, |d| d.w(px(pn.w)).h(px(pn.h)))
                    .when(full_width, |d| d.w_full())
                    .p(px(10.))
                    .pb(px(9.))
                    .rounded(px(10.))
                    .bg(if is_ruled {
                        transparent_black()
                    } else {
                        card_bg
                    })
                    .border_1()
                    .border_color(border_c)
                    .when(!is_ruled && !p.dark, |d| d.shadow_xs())
                    .opacity(if is_ruled && !is_sel { 0.58 } else { 1.0 })
                    .hover(move |s| s.border_color(hover_border).opacity(1.0))
                    .focus_visible(move |s| s.border_2().border_color(focus_col))
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .gap(px(6.))
                    .child(
                        div()
                            .h(px(18.))
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .min_w_0()
                            .child(
                                div()
                                    .size(px(18.))
                                    .flex_none()
                                    .rounded(px(5.))
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(p.line)
                                    .opacity(if is_ruled { 0.55 } else { 1.0 })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(node_icon(&n.icon_key, 11., is_ruled, &p, cx)),
                            )
                            .child(div().flex_1().min_w_0().overflow_hidden().child(
                                if is_checking {
                                    pro_shimmer_text(
                                        pro_eid(&self.id, format!("rcshim-{nid}")),
                                        title_str,
                                        p.ink,
                                        p.strong,
                                        13.,
                                        FontWeight::MEDIUM,
                                        true,
                                    )
                                } else {
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(18.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if is_ruled {
                                            p.muted
                                        } else if pn.is_top_failure {
                                            p.strong
                                        } else {
                                            p.ink
                                        })
                                        .truncate()
                                        .child(title_str)
                                        .into_any_element()
                                },
                            )),
                    )
                    .child(
                        div()
                            .h(px(18.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .min_w_0()
                            .when_some(first_file, |d, fname| {
                                d.child(
                                    div()
                                        .h(px(18.))
                                        .pl(px(5.))
                                        .pr(px(6.))
                                        .rounded(px(5.))
                                        .bg(fill_bg)
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .min_w_0()
                                        .child(pro_svg("0 0 24 24", FILE_CHIP_SVG, 11., p.muted))
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(11.))
                                                .text_color(p.text)
                                                .child(fname),
                                        ),
                                )
                            })
                            .when(extra_files > 0, |d| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .child(format!("+ {extra_files}")),
                                )
                            }),
                    )
                    .child(bottom_meta),
                cx,
                move |this, _, cx| {
                    this.select_node(Some(nid_click.clone()), cx);
                },
            )
        };

        let eff_tree_w = stage_w.min(720.0);
        let (placed_nodes, placed_links, placed_gates, tree_h) =
            layout_fault_tree(&failure, eff_tree_w);

        let tree_el = if narrow {
            let mut list = div().w_full().flex().flex_col().gap(px(6.));
            // Flatten pre-order for narrow indented list
            let mut ordered = placed_nodes.clone();
            ordered.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
            for pn in &ordered {
                list = list.child(
                    div()
                        .pl(px((pn.depth as f32 * 16.0).min(48.0)))
                        .child(render_card(pn, true, cx)),
                );
            }
            list.into_any_element()
        } else {
            let links_c = placed_links;
            let lines_canvas = canvas(
                |_, _, _| (),
                move |surface, _, window, _| {
                    let ox = f32::from(surface.origin.x);
                    let oy = f32::from(surface.origin.y);
                    for lk in &links_c {
                        let p_par = point(px(ox + lk.p_parent.0), px(oy + lk.p_parent.1));
                        let p_ch = point(px(ox + lk.p_child.0), px(oy + lk.p_child.1));
                        if lk.status == CauseStatus::RuledOut {
                            paint_dashed_tree_link(p_par, p_ch, rail_col, window);
                        } else if lk.on_causal_path {
                            // Causal path runs upward from confirmed child -> parent!
                            pro_connector_pulse(
                                p_ch,
                                p_par,
                                8.0,
                                true,
                                path_blue,
                                rgb(0xb9e0ff).into(),
                                pulse_t,
                                1.85,
                                window,
                            );
                        } else {
                            pro_connector_pulse(
                                p_par, p_ch, 8.0, true, rail_col, path_blue, None, 1.5, window,
                            );
                        }
                    }
                },
            )
            .absolute()
            .inset_0();

            let mut stage_div = div()
                .w(px(eff_tree_w))
                .h(px(tree_h))
                .mx_auto()
                .relative()
                .child(lines_canvas);

            for g in &placed_gates {
                let g_txt = match g.gate {
                    CauseGate::Or => "OR",
                    CauseGate::And => "AND",
                };
                let gw = match g.gate {
                    CauseGate::Or => 26.0_f32,
                    CauseGate::And => 34.0_f32,
                };
                stage_div = stage_div.child(
                    div()
                        .absolute()
                        .left(px(g.x - gw * 0.5_f32))
                        .top(px(g.y - 8.0))
                        .w(px(gw))
                        .h(px(16.))
                        .rounded(px(4.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(if g.on_causal_path {
                            path_blue.opacity(0.55)
                        } else {
                            p.line_strong
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(9.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if g.on_causal_path { brand_fg } else { p.muted })
                        .child(g_txt),
                );
            }

            for pn in &placed_nodes {
                stage_div = stage_div.child(
                    div()
                        .absolute()
                        .left(px(pn.x))
                        .top(px(pn.y))
                        .child(render_card(pn, false, cx)),
                );
            }
            stage_div.into_any_element()
        };

        // 3. Bottom Evidence & Action Inspector Card
        let inspected_node = find_node(&failure, active_id.as_ref())
            .cloned()
            .unwrap_or_else(|| failure.clone());
        let is_leaf_root_cause = inspected_node.id != failure.id
            && inspected_node.children.is_empty()
            && inspected_node.status == CauseStatus::Confirmed;

        let ev_status_txt = if inspected_node.id == failure.id {
            if root_found {
                tr(cx, "Root cause confirmed", "已确认根因链路")
            } else {
                tr(cx, "Top failure symptom", "顶层故障现象")
            }
        } else if is_leaf_root_cause {
            tr(cx, "Root cause", "已确认根因")
        } else {
            match inspected_node.status {
                CauseStatus::Checking => tr(cx, "Checking", "正在验证"),
                CauseStatus::RuledOut => tr(cx, "Ruled out", "已排除"),
                CauseStatus::Confirmed => tr(cx, "Confirmed", "已证实"),
                CauseStatus::Open => tr(cx, "Not checked yet", "尚未验证"),
            }
        };

        let verdict_line = if inspected_node.status == CauseStatus::Checking {
            tr(
                cx,
                "Checking this hypothesis now…",
                "正在采集日志与代码证据验证该假设…",
            )
            .to_owned()
        } else if inspected_node.status == CauseStatus::Open && inspected_node.id != failure.id {
            tr(cx, "Queued for verification.", "排队等待验证。").to_owned()
        } else {
            inspected_node.verdict.to_string()
        };

        let mut file_chips = div().flex().flex_wrap().items_center().gap(px(6.));
        for fpath in &inspected_node.files {
            file_chips = file_chips.child(
                div()
                    .h(px(20.))
                    .pl(px(6.))
                    .pr(px(7.))
                    .rounded(px(5.))
                    .bg(fill_bg)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(pro_svg("0 0 24 24", FILE_CHIP_SVG, 11., p.muted))
                    .child(
                        div()
                            .font_family("Menlo")
                            .text_size(px(11.))
                            .text_color(p.text)
                            .child(fpath.clone()),
                    ),
            );
        }

        let evidence_code_el = if inspected_node.status != CauseStatus::Open
            && let Some(ev) = &inspected_node.evidence
        {
            let mut lines_box = div().flex().flex_col().gap(px(1.));
            for line in ev.content.lines() {
                let (l_bg, l_fg) = if line.starts_with("+ ") || line.starts_with('+') {
                    (p.green_soft, p.green_ink)
                } else if line.starts_with("- ") || line.starts_with('-') {
                    (p.red_soft, p.red_ink)
                } else {
                    (transparent_black(), p.ink)
                };
                lines_box = lines_box.child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(4.))
                        .bg(l_bg)
                        .font_family("Menlo")
                        .text_size(px(11.5))
                        .line_height(px(17.))
                        .text_color(l_fg)
                        .child(line.to_owned()),
                );
            }
            Some(
                div()
                    .w_full()
                    .p(px(8.))
                    .rounded(px(8.))
                    .bg(well_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        div()
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(ev.kind.to_uppercase())
                            .when_some(ev.source.as_ref(), |d, src| {
                                d.child(
                                    div()
                                        .font_family("Menlo")
                                        .text_color(p.faint)
                                        .child(src.clone()),
                                )
                            }),
                    )
                    .child(lines_box),
            )
        } else {
            None
        };

        let cid_fix = inspected_node.id.clone();
        let cid_retry = inspected_node.id.clone();
        let live_now = self.live;

        let evidence_card = pro_view_transition(
            pro_eid(&self.id, "ev-vtx"),
            format!("{}:{:?}", inspected_node.id, inspected_node.status),
            div()
                .id(pro_eid(&self.id, "ev-panel"))
                .role(Role::Status)
                .w_full()
                .max_w(px(720.))
                .mx_auto()
                .p(px(14.))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .when(!p.dark, |d| d.shadow_xs())
                .flex()
                .flex_col()
                .gap(px(9.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(20.))
                                .rounded(px(5.))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(node_icon(&inspected_node.icon_key, 11.5, false, &p, cx)),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(self.localize_str(&inspected_node.title, cx).to_owned()),
                        )
                        .child(
                            div()
                                .ml_auto()
                                .text_size(px(12.))
                                .text_color(if is_leaf_root_cause {
                                    p.red_ink
                                } else {
                                    p.muted
                                })
                                .font_weight(if is_leaf_root_cause {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .child(ev_status_txt),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(if inspected_node.status == CauseStatus::Checking {
                            p.muted
                        } else {
                            p.text
                        })
                        .child(verdict_line),
                )
                .when(!inspected_node.files.is_empty(), |d| d.child(file_chips))
                .when_some(evidence_code_el, |d, code| d.child(code))
                .child(
                    div()
                        .pt(px(2.))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(if self.custom_failure.is_none() {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_btn(
                                    pro_eid(&self.id, "rc-step"),
                                    tr(cx, "Step check", "单步验证"),
                                    Some(ProIcon::ArrowRight),
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.live = false;
                                        this.selected_id = None;
                                        this.step_investigation(cx);
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, "rc-live"),
                                    if live_now {
                                        tr(cx, "Pause", "暂停")
                                    } else {
                                        tr(cx, "Auto", "自动排查")
                                    },
                                    Some(if live_now {
                                        ProIcon::Pause
                                    } else {
                                        ProIcon::Play
                                    }),
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        this.live = !live_now;
                                        this.selected_id = None;
                                        this.step_started = Instant::now();
                                        cx.notify();
                                    },
                                ))
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        })
                        .when(is_leaf_root_cause, |d| {
                            d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "rc-retry"),
                                        tr(cx, "Retry from here", "从此处重试"),
                                        Some(ProIcon::Refresh),
                                        ProBtnStyle::Secondary,
                                        true,
                                        cx,
                                        move |this, _, cx| {
                                            this.trigger_action(
                                                cid_retry.clone(),
                                                RootCauseAction::Retry,
                                                cx,
                                            );
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "rc-fix"),
                                        tr(cx, "Fix it", "自动修复"),
                                        Some(ProIcon::Sparkles),
                                        ProBtnStyle::Primary,
                                        true,
                                        cx,
                                        move |this, _, cx| {
                                            this.trigger_action(
                                                cid_fix.clone(),
                                                RootCauseAction::Fix,
                                                cx,
                                            );
                                        },
                                    )),
                            )
                        }),
                ),
        );

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_str(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" && this.selected_id.is_some() {
                    this.select_node(None, cx);
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .gap(px(16.))
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
            .child(div().w_full().max_w(px(720.)).mx_auto().child(head_bar))
            .child(tree_el)
            .child(evidence_card)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{CauseGate, CauseNode, CauseStatus, RootCauseAction, RootCauseTree};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn root_cause_tree_evaluates_and_or_gates_and_steps_to_confirmation(cx: &mut TestAppContext) {
        let view = cx.new(|cx| RootCauseTree::new("test-root-cause-tree", cx));

        view.update(cx, |rct, cx| {
            assert!(!rct.is_root_cause_confirmed());

            // Advance 4 steps (0 -> 1 -> 2 -> 3 -> 4) so both AND children confirm!
            for _ in 0..4 {
                rct.step_investigation(cx);
            }
            assert!(rct.is_root_cause_confirmed());

            rct.trigger_action("sent-amount", RootCauseAction::Fix, cx);
            rct.reset(cx);
            assert!(!rct.is_root_cause_confirmed());
        });

        // Verify AND gate requires ALL children before lighting causal path
        let partial_and = CauseNode::new("top", "Failure")
            .with_gate(CauseGate::And)
            .with_children(vec![
                CauseNode::new("c1", "A").with_status(CauseStatus::Confirmed),
                CauseNode::new("c2", "B").with_status(CauseStatus::RuledOut),
            ]);
        let custom = cx.new(|cx| RootCauseTree::new("custom-rct", cx).with_failure(partial_and));
        custom.read_with(cx, |rct, _| {
            assert!(!rct.is_root_cause_confirmed());
        });
    }
}
