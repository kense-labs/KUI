//! LLM-as-a-judge single-elimination tournament bracket with side-by-side rubric comparison.
//!
//! Pairs candidate completions (`Quarterfinal -> Semifinal -> Final`), shimmers the match currently
//! under evaluation by the judge model, fades eliminated contenders onto dashed bracket arms,
//! illuminates the champion's full path through the bracket, and opens any match's two completions
//! side by side with rubric scores and the judge's reasoning below.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_connector_pulse, pro_eid,
        pro_empty_state, pro_fold, pro_mix_color, pro_rolling_number, pro_shimmer_text,
        pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const BRACKET_MARK_SVG: &str = r#"<path d="M3 5.2h5.2v13.6H3M8.2 12h5.2" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" opacity="0.5"/><rect x="14.2" y="8.4" width="7.4" height="7.2" rx="2" fill="currentColor"/>"#;
const TROPHY_CUP_SVG: &str = r#"<path d="M8 4h8v4.5a4 4 0 0 1-8 0V4Z" stroke="currentColor" stroke-width="1.9" stroke-linejoin="round"/><path d="M8 6H5.5a1.8 1.8 0 0 0 0 3.6H8M16 6h2.5a1.8 1.8 0 0 1 0 3.6H16M12 12.5V17M8.5 20h7" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/>"#;

fn spinning_ring(side: f32, stroke: f32, color: Hsla, elapsed_s: f32, active: bool) -> AnyElement {
    let angle = if active {
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

fn paint_dashed_arm(p0: Point<Pixels>, p1: Point<Pixels>, color: Hsla, w: &mut Window) {
    let x0 = f32::from(p0.x);
    let y0 = f32::from(p0.y);
    let x1 = f32::from(p1.x);
    let y1 = f32::from(p1.y);
    let xm = (x0 + x1) * 0.5_f32;
    let segs = [
        (point(px(x0), px(y0)), point(px(xm), px(y0))),
        (point(px(xm), px(y0)), point(px(xm), px(y1))),
        (point(px(xm), px(y1)), point(px(x1), px(y1))),
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
        let n = (len / step).ceil() as usize;
        for i in 0..n {
            let t0 = (i as f32 * step) / len;
            let t1 = ((i as f32 * step + 2.8) / len).min(1.0);
            if t1 <= t0 {
                continue;
            }
            let mut pb = PathBuilder::stroke(px(1.4));
            pb.move_to(point(px(ax + dx * t0), px(ay + dy * t0)));
            pb.line_to(point(px(ax + dx * t1), px(ay + dy * t1)));
            if let Ok(path) = pb.build() {
                w.paint_path(path, color);
            }
        }
    }
}

/// Candidate completion seeded into [`JudgeBracket`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BracketContender {
    /// Stable contender identifier.
    pub id: SharedString,
    /// Short approach or variant title (`"Idempotency key"`, `"Sonnet · warm"`).
    pub name: SharedString,
    /// Model provider brand key (`"claude"`, `"gpt"`, `"gemini"`, `"deepseek"`, `"mistral"`).
    pub brand: SharedString,
    /// Compact diff/word/test summary (`"Sonnet · +18 −4 · 3 files · 42/42 tests"`).
    pub meta: SharedString,
    /// Candidate completion text or patch snippet displayed in the side-by-side match panel.
    pub output: SharedString,
}

impl BracketContender {
    /// Construct a [`BracketContender`] with `id`, `name`, `brand`, `meta`, and `output`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        brand: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        output: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            brand: brand.into(),
            meta: meta.into(),
            output: output.into(),
        }
    }
}

/// Judge's decision and rubric scores for one `(round, match_idx)` pairing.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchVerdict {
    /// Winning [`BracketContender::id`] (must match either the upper or lower contender of the match).
    pub winner_id: SharedString,
    /// `[upper_score, lower_score]` rubric scores out of `10.0`.
    pub scores: [f32; 2],
    /// One-sentence explanation of why the judge preferred `winner_id`.
    pub reason: SharedString,
}

impl MatchVerdict {
    /// Construct a [`MatchVerdict`] with `winner_id`, `[upper_score, lower_score]`, and `reason`.
    pub fn new(
        winner_id: impl Into<SharedString>,
        scores: [f32; 2],
        reason: impl Into<SharedString>,
    ) -> Self {
        Self {
            winner_id: winner_id.into(),
            scores,
            reason: reason.into(),
        }
    }
}

/// Coordinate `(round_index, match_index)` identifying a match inside [`JudgeBracket`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchKey {
    /// Zero-based round (`0` = Quarterfinal for 8 seeds, `1` = Semifinal, `2` = Final).
    pub round: usize,
    /// Zero-based match index from top to bottom within `round`.
    pub index: usize,
}

impl MatchKey {
    /// Create a [`MatchKey`] at `(round, index)`.
    pub const fn new(round: usize, index: usize) -> Self {
        Self { round, index }
    }
}

/// Typed events emitted by [`JudgeBracket`].
#[derive(Clone, Debug, PartialEq)]
pub enum JudgeBracketEvent {
    /// User clicked or navigated to inspect `match_key`.
    MatchSelected(MatchKey),
    /// Tournament judging advanced to `decided_count` completed matches.
    StepAdvanced(usize),
    /// User clicked `Use winner` on `contender`.
    WinnerPicked(BracketContender),
    /// Tournament reset to the opening match.
    Reset,
}

fn default_contenders() -> Vec<BracketContender> {
    vec![
        BracketContender::new(
            "idempotency",
            "Idempotency key",
            "claude",
            "Sonnet · +18 −4 · 3 files · 42/42 tests",
            "Sends one deterministic idempotency key per checkout so Stripe returns the original PaymentIntent on any retry.\nstripe.paymentIntents.create(params, { idempotencyKey: checkout.id })",
        ),
        BracketContender::new(
            "retry-guard",
            "Retry guard",
            "gpt",
            "GPT-5 · +9 −1 · 1 file · 41/42 tests",
            "Skips retries in memory while the first request promise is still pending.\nif (inFlight.has(checkout.id)) return inFlight.get(checkout.id);\nFails: cross-instance retries still double-charge",
        ),
        BracketContender::new(
            "row-lock",
            "Row lock",
            "gemini",
            "Gemini · +24 −6 · 2 files · 42/42 tests",
            "Acquires a Postgres row lock before calling Stripe.\nSELECT id FROM checkouts WHERE id = $1 FOR UPDATE;\nHolds DB connection open across external Stripe latency",
        ),
        BracketContender::new(
            "no-retries",
            "No retries",
            "deepseek",
            "DeepSeek · +3 −8 · 1 file · 38/42 tests",
            "Removes automatic client & gateway retry middleware on /api/checkout.\nfetch('/api/checkout', { retries: 0 });\nFails: 4 transient-network test cases fail",
        ),
        BracketContender::new(
            "webhook-dedupe",
            "Webhook dedupe",
            "claude",
            "Sonnet · +31 −7 · 4 files · 42/42 tests",
            "Deduplicates incoming payment_intent.succeeded webhooks via a unique event_id constraint.\nINSERT INTO stripe_events (id) VALUES ($1) ON CONFLICT DO NOTHING;",
        ),
        BracketContender::new(
            "state-machine",
            "State machine",
            "gpt",
            "GPT-5 · +64 −19 · 6 files · 42/42 tests",
            "Adds compare-and-swap checkout status transitions before creating a charge.\nUPDATE checkouts SET status = 'charging' WHERE id = $1 AND status = 'open';",
        ),
        BracketContender::new(
            "debounce",
            "Button debounce",
            "gemini",
            "Gemini · +7 −2 · 1 file · 39/42 tests",
            "Disables the submit button for 3000ms after click.\nsetDisabled(true); setTimeout(() => setDisabled(false), 3000);\nFails: API-level retries still charge twice",
        ),
        BracketContender::new(
            "refund-later",
            "Refund later",
            "mistral",
            "Mistral · +29 −3 · 2 files · 40/42 tests",
            "Runs a cron job every 5 minutes to refund duplicate PaymentIntents.\nawait stripe.refunds.create({ payment_intent: dup.id });\nCustomers still see duplicate holds on their statement",
        ),
    ]
}

fn default_tournament_script() -> Vec<(MatchKey, MatchVerdict)> {
    vec![
        (
            MatchKey::new(0, 0),
            MatchVerdict::new(
                "idempotency",
                [9.4, 6.8],
                "Enforces single-charge semantics across server replicas and passes all 42/42 tests, whereas an in-memory map misses multi-worker retries.",
            ),
        ),
        (
            MatchKey::new(0, 1),
            MatchVerdict::new(
                "row-lock",
                [8.5, 5.1],
                "Prevents concurrent charges without breaking legitimate network retries on flaky mobile connections.",
            ),
        ),
        (
            MatchKey::new(0, 2),
            MatchVerdict::new(
                "state-machine",
                [8.8, 8.1],
                "Blocks the second Stripe API call before money moves instead of only deduplicating downstream webhook delivery.",
            ),
        ),
        (
            MatchKey::new(0, 3),
            MatchVerdict::new(
                "debounce",
                [6.4, 4.9],
                "Prevents accidental double-clicks cleanly without placing temporary duplicate holds on customer cards.",
            ),
        ),
        (
            MatchKey::new(1, 0),
            MatchVerdict::new(
                "idempotency",
                [9.5, 8.3],
                "Achieves stronger guarantees without holding database row locks open during external Stripe HTTP calls.",
            ),
        ),
        (
            MatchKey::new(1, 1),
            MatchVerdict::new(
                "state-machine",
                [8.9, 5.8],
                "Guards API and mobile client retries at the persistence boundary, whereas UI debounce only blocks browser clicks.",
            ),
        ),
        (
            MatchKey::new(2, 0),
            MatchVerdict::new(
                "idempotency",
                [9.6, 8.7],
                "Solves the root cause in 18 lines across 3 files using Stripe's native idempotency contract rather than a 6-file schema change.",
            ),
        ),
    ]
}

fn round_title(round: usize, total_rounds: usize, cx: &App) -> &'static str {
    let remaining = total_rounds.saturating_sub(round);
    match remaining {
        1 => tr(cx, "Final", "决赛"),
        2 => tr(cx, "Semifinal", "半决赛"),
        3 => tr(cx, "Quarterfinal", "四分之一决赛"),
        _ => tr(cx, "Round 1", "第一轮"),
    }
}

fn round_match_title(key: MatchKey, total_rounds: usize, cx: &App) -> String {
    let base = round_title(key.round, total_rounds, cx);
    if key.round + 1 == total_rounds {
        base.to_owned()
    } else {
        format!("{base} {}", key.index + 1)
    }
}

#[derive(Clone)]
struct ResolvedMatch {
    key: MatchKey,
    upper: Option<BracketContender>,
    lower: Option<BracketContender>,
    verdict: Option<MatchVerdict>,
    judging: bool,
}

/// LLM-as-a-judge tournament bracket pairing candidate completions round by round with side-by-side rubric inspection.
pub struct JudgeBracket {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    task: SharedString,
    judge: SharedString,
    contenders: Vec<BracketContender>,
    custom_verdicts: Option<HashMap<MatchKey, MatchVerdict>>,
    custom_judging: Option<MatchKey>,
    script: Vec<(MatchKey, MatchVerdict)>,
    demo_decided: usize,
    step_started: Instant,
    live: bool,
    selected_match: Option<MatchKey>,
    hovered_match: Option<MatchKey>,
    last_verdict_reason: Option<SharedString>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl JudgeBracket {
    /// Create a [`JudgeBracket`] preloaded with 8 candidate patches judged by `"Claude Opus 4.6"`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Judge".into(),
            task: "Fix: customers charged twice when a payment is retried".into(),
            judge: "Claude Opus 4.6".into(),
            contenders: default_contenders(),
            custom_verdicts: None,
            custom_judging: None,
            script: default_tournament_script(),
            demo_decided: 0,
            step_started: now,
            live: true,
            selected_match: None,
            hovered_match: None,
            last_verdict_reason: None,
            boot_at: now,
            measured_width: Rc::new(Cell::new(680.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the bar heading (defaults to `"Judge"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Override the muted task description shown beside the heading.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = task.into();
        self
    }

    /// Override the judge model name (defaults to `"Claude Opus 4.6"`).
    pub fn with_judge(mut self, judge: impl Into<SharedString>) -> Self {
        self.judge = judge.into();
        self
    }

    /// Supply seeded [`BracketContender`] entries (2, 4, or 8 contenders).
    pub fn with_contenders(mut self, contenders: Vec<BracketContender>) -> Self {
        self.contenders = contenders;
        self
    }

    /// Supply explicit [`MatchVerdict`] decisions and optional currently `judging` match key.
    pub fn with_verdicts(
        mut self,
        verdicts: HashMap<MatchKey, MatchVerdict>,
        judging: Option<MatchKey>,
    ) -> Self {
        self.custom_verdicts = Some(verdicts);
        self.custom_judging = judging;
        self.live = false;
        self
    }

    /// Set how many matches start decided (`0..=7`) in the built-in tournament walkthrough.
    pub fn with_decided_count(mut self, count: usize) -> Self {
        self.demo_decided = count.min(self.script.len());
        self
    }

    /// Enable or pause live tournament progression.
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self.step_started = Instant::now();
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

    fn effective_seed_count(&self) -> usize {
        let n = self.contenders.len();
        if n >= 8 {
            8
        } else if n >= 4 {
            4
        } else if n >= 2 {
            2
        } else {
            0
        }
    }

    fn total_rounds(&self) -> usize {
        match self.effective_seed_count() {
            8 => 3,
            4 => 2,
            2 => 1,
            _ => 0,
        }
    }

    fn active_verdicts_and_judging(&self) -> (HashMap<MatchKey, MatchVerdict>, Option<MatchKey>) {
        if let Some(cv) = &self.custom_verdicts {
            return (cv.clone(), self.custom_judging);
        }
        let mut map = HashMap::new();
        let upto = self.demo_decided.min(self.script.len());
        for (k, v) in self.script.iter().take(upto) {
            map.insert(*k, v.clone());
        }
        let judging = self.script.get(upto).map(|(k, _)| *k);
        (map, judging)
    }

    fn build_rounds(&self) -> Vec<Vec<ResolvedMatch>> {
        let seeds = self.effective_seed_count();
        let rounds_n = self.total_rounds();
        if seeds < 2 || rounds_n == 0 {
            return Vec::new();
        }
        let (verdicts, judging_key) = self.active_verdicts_and_judging();
        let mut rounds: Vec<Vec<ResolvedMatch>> = Vec::with_capacity(rounds_n);

        for r in 0..rounds_n {
            let match_count = seeds >> (r + 1);
            let mut r_matches = Vec::with_capacity(match_count);
            for m in 0..match_count {
                let key = MatchKey::new(r, m);
                let (upper, lower) = if r == 0 {
                    (
                        self.contenders.get(m * 2).cloned(),
                        self.contenders.get(m * 2 + 1).cloned(),
                    )
                } else {
                    let prev_a = &rounds[r - 1][m * 2];
                    let prev_b = &rounds[r - 1][m * 2 + 1];
                    let win_a = prev_a.verdict.as_ref().and_then(|v| {
                        [prev_a.upper.as_ref(), prev_a.lower.as_ref()]
                            .into_iter()
                            .flatten()
                            .find(|c| c.id == v.winner_id)
                            .cloned()
                    });
                    let win_b = prev_b.verdict.as_ref().and_then(|v| {
                        [prev_b.upper.as_ref(), prev_b.lower.as_ref()]
                            .into_iter()
                            .flatten()
                            .find(|c| c.id == v.winner_id)
                            .cloned()
                    });
                    (win_a, win_b)
                };
                let verdict = if upper.is_some() && lower.is_some() {
                    verdicts.get(&key).cloned()
                } else {
                    None
                };
                let judging = verdict.is_none()
                    && upper.is_some()
                    && lower.is_some()
                    && judging_key == Some(key);
                r_matches.push(ResolvedMatch {
                    key,
                    upper,
                    lower,
                    verdict,
                    judging,
                });
            }
            rounds.push(r_matches);
        }
        rounds
    }

    /// Tournament champion once the final round match has been decided.
    pub fn champion(&self) -> Option<BracketContender> {
        let rounds = self.build_rounds();
        let final_match = rounds.last()?.first()?;
        let v = final_match.verdict.as_ref()?;
        [final_match.upper.as_ref(), final_match.lower.as_ref()]
            .into_iter()
            .flatten()
            .find(|c| c.id == v.winner_id)
            .cloned()
    }

    /// Select a specific `(round, index)` match to inspect side by side (`None` tracks active judging).
    pub fn select_match(&mut self, key: Option<MatchKey>, cx: &mut Context<Self>) {
        self.selected_match = key;
        if let Some(k) = key {
            cx.emit(JudgeBracketEvent::MatchSelected(k));
        }
        cx.notify();
    }

    /// Advance one match decision in the built-in tournament walkthrough (`0..=7`).
    pub fn step_tournament(&mut self, cx: &mut Context<Self>) {
        if self.script.is_empty() {
            return;
        }
        self.custom_verdicts = None;
        self.demo_decided = (self.demo_decided + 1) % (self.script.len() + 1);
        self.step_started = Instant::now();
        cx.emit(JudgeBracketEvent::StepAdvanced(self.demo_decided));
        cx.notify();
    }

    /// Reset tournament progression back to `0` decided matches.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.demo_decided = 0;
        self.selected_match = None;
        self.step_started = Instant::now();
        cx.emit(JudgeBracketEvent::Reset);
        cx.notify();
    }

    /// Accept and emit `WinnerPicked` for `contender`.
    pub fn pick_winner(&mut self, contender: BracketContender, cx: &mut Context<Self>) {
        let msg = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("已选用方案：{}", contender.name)
        } else {
            format!("Selected winner: {}", contender.name)
        };
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(JudgeBracketEvent::WinnerPicked(contender));
        cx.notify();
    }

    fn localize_str<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Judge" => tr(cx, "Judge", "评审对决"),
            "Fix: customers charged twice when a payment is retried" => tr(
                cx,
                "Fix: customers charged twice when a payment is retried",
                "修复：支付重试导致客户被重复扣款",
            ),
            "Idempotency key" => tr(cx, "Idempotency key", "幂等键校验"),
            "Retry guard" => tr(cx, "Retry guard", "内存重试锁"),
            "Row lock" => tr(cx, "Row lock", "数据库行锁"),
            "No retries" => tr(cx, "No retries", "禁用重试"),
            "Webhook dedupe" => tr(cx, "Webhook dedupe", "回调幂等去重"),
            "State machine" => tr(cx, "State machine", "状态机流转"),
            "Button debounce" => tr(cx, "Button debounce", "按钮防抖"),
            "Refund later" => tr(cx, "Refund later", "定时退款补偿"),
            other => other,
        }
    }
}

impl EventEmitter<JudgeBracketEvent> for JudgeBracket {}

impl Focusable for JudgeBracket {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for JudgeBracket {
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
                .max_w(px(720.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(pro_skeleton_bar(280., 18., 6., phase, cx))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .gap(px(20.))
                        .child(pro_skeleton_bar(200., 340., 10., phase + 0.1, cx).flex_1())
                        .child(pro_skeleton_bar(200., 260., 10., phase + 0.2, cx).flex_1())
                        .child(pro_skeleton_bar(200., 180., 10., phase + 0.3, cx).flex_1()),
                )
                .child(pro_skeleton_bar(680., 170., 12., phase + 0.4, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.effective_seed_count() < 2 {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(720.))
                .mx_auto()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "Need at least two contenders", "至少需要两个候选结果"),
                    tr(
                        cx,
                        "Seed 2, 4, or 8 candidate completions so the judge model can compare them.",
                        "传入 2、4 或 8 个候选回答即可启动大模型成对淘汰评审。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-jb"),
                        tr(cx, "Load 8-seed bracket", "加载 8 强对决示例").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.contenders = default_contenders();
                            this.reset(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        // Advance tournament smoothly when live and user isn't manually inspecting a specific match
        if self.live && !reduced && self.custom_verdicts.is_none() && self.selected_match.is_none()
        {
            let hold = if self.demo_decided >= self.script.len() {
                4.2_f32
            } else {
                1.85_f32
            };
            if self.step_started.elapsed().as_secs_f32() >= hold {
                self.step_tournament(cx);
            }
            w.request_animation_frame();
        }

        let rounds = self.build_rounds();
        let total_rounds = rounds.len();
        let total_matches: usize = rounds.iter().map(Vec::len).sum();
        let decided_matches: usize = rounds
            .iter()
            .flatten()
            .filter(|m| m.verdict.is_some())
            .count();
        let active_judging = rounds.iter().flatten().find(|m| m.judging).map(|m| m.key);
        let champ = self.champion();

        if (active_judging.is_some() || champ.is_some()) && !reduced {
            w.request_animation_frame();
        }

        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let pulse_t = if reduced {
            None
        } else {
            Some(((elapsed_s / 1.40).fract()).clamp(0.0, 1.0))
        };

        let inspected_key = self
            .selected_match
            .or(active_judging)
            .unwrap_or_else(|| MatchKey::new(total_rounds.saturating_sub(1), 0));

        let width_cell = self.measured_width.clone();
        let stage_w = width_cell.get().max(320.0);
        let stacked = stage_w < 600.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.035)
        } else {
            hsla(0., 0., 0., 0.025)
        };
        let hint_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.015)
        } else {
            hsla(0., 0., 0., 0.012)
        };
        let rail_col: Hsla = if p.dark {
            rgb(0x333338).into()
        } else {
            rgb(0xe2e2e5).into()
        };
        let won_col: Hsla = if p.dark {
            rgb(0x5a5a61).into()
        } else {
            rgb(0xb4b4ba).into()
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
        let hover_border = p.line_strong;

        // 1. Header Bar
        let cur_round_lbl = if let Some(jk) = active_judging {
            round_title(jk.round, total_rounds, cx)
        } else {
            round_title(total_rounds.saturating_sub(1), total_rounds, cx)
        };

        let head_bar = div()
            .w_full()
            .pb(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(pro_svg("0 0 24 24", BRACKET_MARK_SVG, 16., p.muted))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.localize_str(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(120.))
                    .truncate()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(self.localize_str(&self.task, cx).to_owned()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(active_judging.is_some(), |d| {
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
                                .child(spinning_ring(9.5, 1.5, brand_fg, elapsed_s, !reduced))
                                .child(tr(cx, "Judging", "评审中")),
                        )
                    })
                    .when_some(champ.as_ref(), |d, ch| {
                        d.child(
                            div()
                                .h(px(19.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(brand_bg)
                                .text_color(brand_fg)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(pro_svg("0 0 24 24", TROPHY_CUP_SVG, 11.5, brand_fg))
                                .child(self.localize_str(&ch.name, cx).to_owned()),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_swap_label(
                                pro_eid(&self.id, "jb-round-lbl"),
                                cur_round_lbl,
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .child(div().text_size(px(12.)).text_color(p.faint).child("·"))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "jb-decided-cnt"),
                                decided_matches as f64,
                                0,
                                "",
                                format!("/{total_matches}"),
                                12.,
                                FontWeight::MEDIUM,
                                p.muted,
                            )),
                    ),
            );

        // Connected-branch spotlight from hovered match
        let spotlight_key = self.hovered_match;
        let spotlight_cids: Vec<SharedString> = spotlight_key
            .and_then(|hk| rounds.get(hk.round).and_then(|rm| rm.get(hk.index)))
            .map(|hm| {
                [hm.upper.as_ref(), hm.lower.as_ref()]
                    .into_iter()
                    .flatten()
                    .map(|c| c.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        let is_lineage = |a: MatchKey, b: MatchKey| -> bool {
            if a.round == b.round {
                a.index == b.index
            } else if a.round < b.round {
                (a.index >> (b.round - a.round)) == b.index
            } else {
                (b.index >> (a.round - b.round)) == a.index
            }
        };

        let mut match_opacities: Vec<Vec<f32>> = Vec::with_capacity(total_rounds);
        for (r, r_matches) in rounds.iter().enumerate() {
            let mut row_ops = Vec::with_capacity(r_matches.len());
            for (m_idx, rm) in r_matches.iter().enumerate() {
                let mk = rm.key;
                let lit = match spotlight_key {
                    None => true,
                    Some(hk) if mk == hk => true,
                    Some(hk) if is_lineage(mk, hk) => {
                        if spotlight_cids.is_empty() {
                            true
                        } else if mk.round < hk.round {
                            rm.verdict
                                .as_ref()
                                .is_some_and(|v| spotlight_cids.contains(&v.winner_id))
                                || (rm.verdict.is_none()
                                    && [rm.upper.as_ref(), rm.lower.as_ref()]
                                        .into_iter()
                                        .flatten()
                                        .any(|c| spotlight_cids.contains(&c.id)))
                        } else {
                            (rm.upper.is_none() && rm.lower.is_none())
                                || [rm.upper.as_ref(), rm.lower.as_ref()]
                                    .into_iter()
                                    .flatten()
                                    .any(|c| spotlight_cids.contains(&c.id))
                        }
                    }
                    Some(_) => false,
                };
                let target_op = if lit { 1.0 } else { 0.34 };
                row_ops.push(pro_spring_f32(
                    pro_eid(&self.id, format!("jb-mop-{r}-{m_idx}")),
                    target_op,
                    0.22,
                    w,
                    cx,
                ));
            }
            match_opacities.push(row_ops);
        }

        // Slot visual builder
        let champ_id = champ.as_ref().map(|c| c.id.clone());
        let build_slot = |cont: Option<&BracketContender>,
                          score: Option<f32>,
                          is_winner: bool,
                          is_loser: bool,
                          is_judging: bool,
                          is_final_slot: bool,
                          slot_w: Option<f32>,
                          slot_key: String,
                          cx: &App|
         -> Div {
            let Some(c) = cont else {
                return div()
                    .when_some(slot_w, |d, sw| d.w(px(sw)))
                    .when(slot_w.is_none(), |d| d.w_full())
                    .h(px(40.))
                    .px(px(10.))
                    .rounded(px(10.))
                    .bg(hint_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .text_size(px(11.5))
                    .text_color(p.faint)
                    .child(tr(cx, "To be decided", "待晋级"));
            };

            let on_champ_way = champ_id.as_ref() == Some(&c.id) && (is_winner || is_final_slot);
            let cname = self.localize_str(&c.name, cx).to_owned();
            let bdr = if on_champ_way && champ_id.is_some() {
                path_blue.opacity(0.55)
            } else {
                p.line
            };

            div()
                .when_some(slot_w, |d, sw| d.w(px(sw)))
                .when(slot_w.is_none(), |d| d.w_full())
                .h(px(40.))
                .px(px(10.))
                .rounded(px(10.))
                .bg(if is_loser {
                    transparent_black()
                } else {
                    card_bg
                })
                .border_1()
                .border_color(bdr)
                .when(!is_loser && !p.dark, |d| d.shadow_xs())
                .opacity(if is_loser { 0.48 } else { 1.0 })
                .flex()
                .items_center()
                .gap(px(7.))
                .child(
                    div()
                        .size(px(18.))
                        .flex_none()
                        .rounded(px(5.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            pro_app_artwork(&c.brand, 11., cx)
                                .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 10., p.ink)),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(if is_judging {
                            pro_shimmer_text(
                                pro_eid(&self.id, format!("jshim-{slot_key}")),
                                cname,
                                p.ink,
                                p.strong,
                                12.5,
                                FontWeight::MEDIUM,
                                true,
                            )
                        } else {
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if is_loser { p.muted } else { p.ink })
                                .truncate()
                                .child(cname)
                                .into_any_element()
                        }),
                )
                .when(is_final_slot && is_winner && champ_id.is_some(), |d| {
                    d.child(pro_svg("0 0 24 24", TROPHY_CUP_SVG, 13., brand_fg))
                })
                .when(is_winner && !(is_final_slot && champ_id.is_some()), |d| {
                    d.child(pro_check_draw(
                        pro_eid(&self.id, format!("slot-chk-{slot_key}")),
                        true,
                        12.,
                        false,
                        p.green,
                    ))
                })
                .when_some(score, |d, sc| {
                    d.child(div().flex_none().child(pro_rolling_number(
                        pro_eid(&self.id, format!("slot-sc-{slot_key}")),
                        sc as f64,
                        1,
                        "",
                        "",
                        11.5,
                        if is_winner {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        },
                        if is_winner { p.strong } else { p.muted },
                    )))
                })
        };

        // 2. Bracket Layout (Side-by-side tournament columns OR narrow stacked list)
        let eff_w = stage_w.min(720.0);
        let bracket_el = if stacked {
            let mut col = div().w_full().flex().flex_col().gap(px(12.));
            for (r, r_matches) in rounds.iter().enumerate() {
                col = col.child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(round_title(r, total_rounds, cx)),
                );
                for (m_idx, m) in r_matches.iter().enumerate() {
                    let mk = m.key;
                    let is_sel = mk == inspected_key;
                    let m_op = match_opacities[r][m_idx];
                    let w_id = m.verdict.as_ref().map(|v| v.winner_id.clone());
                    let sc_u = m.verdict.as_ref().map(|v| v.scores[0]);
                    let sc_l = m.verdict.as_ref().map(|v| v.scores[1]);
                    let up_won = w_id.is_some() && m.upper.as_ref().map(|c| &c.id) == w_id.as_ref();
                    let lo_won = w_id.is_some() && m.lower.as_ref().map(|c| &c.id) == w_id.as_ref();
                    let is_fin = r + 1 == total_rounds;

                    let inner_stack = div()
                        .w_full()
                        .p(px(6.))
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(build_slot(
                            m.upper.as_ref(),
                            sc_u,
                            up_won,
                            lo_won,
                            m.judging,
                            is_fin,
                            None,
                            format!("{}-{}-u", mk.round, mk.index),
                            cx,
                        ))
                        .child(build_slot(
                            m.lower.as_ref(),
                            sc_l,
                            lo_won,
                            up_won,
                            m.judging,
                            is_fin,
                            None,
                            format!("{}-{}-l", mk.round, mk.index),
                            cx,
                        ));

                    let stack_card = pro_tactile(
                        pro_eid(&self.id, format!("mstack-{}-{}", mk.round, mk.index)),
                        round_match_title(mk, total_rounds, cx),
                        12.,
                        if is_sel { well_bg } else { transparent_black() },
                        well_bg,
                        if is_sel {
                            path_blue.opacity(0.5)
                        } else {
                            p.line
                        },
                        if is_sel {
                            path_blue.opacity(0.75)
                        } else {
                            hover_border
                        },
                        is_sel,
                        inner_stack,
                        cx,
                        move |this, _, cx| {
                            this.select_match(Some(mk), cx);
                        },
                    )
                    .w_full()
                    .opacity(m_op)
                    .on_hover(cx.listener(
                        move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_match = Some(mk);
                            } else if this.hovered_match == Some(mk) {
                                this.hovered_match = None;
                            }
                            cx.notify();
                        },
                    ));

                    col = col.child(stack_card);
                }
            }
            col.into_any_element()
        } else {
            let col_gap = if total_rounds >= 3 {
                28.0_f32
            } else {
                44.0_f32
            };
            let col_w = ((eff_w - col_gap * (total_rounds.saturating_sub(1)) as f32)
                / total_rounds as f32)
                .clamp(160.0, 224.0);
            let actual_w =
                col_w * total_rounds as f32 + col_gap * (total_rounds.saturating_sub(1)) as f32;

            // Exact symmetric binary-tree Y centers!
            // Round 0 matches sit every `100px`: upper center `y = m*100 + 20`, lower center `y = m*100 + 64`.
            // Round r > 0 match m has upper slot centered at midpoint of feeding match `2*m`
            // and lower slot centered at midpoint of feeding match `2*m + 1`!
            let r0_matches = rounds[0].len();
            let bracket_h = (r0_matches as f32 * 100.0 - 16.0).max(84.0);

            let mut slot_centers: Vec<Vec<(f32, f32)>> = Vec::with_capacity(total_rounds);
            let mut r0_centers = Vec::with_capacity(r0_matches);
            for m in 0..r0_matches {
                let base_y = m as f32 * 100.0;
                r0_centers.push((base_y + 20.0, base_y + 64.0));
            }
            slot_centers.push(r0_centers);

            for r in 1..total_rounds {
                let mc = rounds[r].len();
                let mut rc = Vec::with_capacity(mc);
                for m in 0..mc {
                    let prev_a = slot_centers[r - 1][m * 2];
                    let prev_b = slot_centers[r - 1][m * 2 + 1];
                    let cy_u = (prev_a.0 + prev_a.1) * 0.5_f32;
                    let cy_l = (prev_b.0 + prev_b.1) * 0.5_f32;
                    rc.push((cy_u, cy_l));
                }
                slot_centers.push(rc);
            }

            // Column headers row
            let mut col_heads = div()
                .w(px(actual_w))
                .mx_auto()
                .pb(px(8.))
                .flex()
                .gap(px(col_gap));
            for r in 0..total_rounds {
                col_heads = col_heads.child(
                    div()
                        .w(px(col_w))
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(round_title(r, total_rounds, cx)),
                );
            }

            // Precompute connector arms from each match's upper & lower slots into the next round's slot!
            #[derive(Clone, Copy)]
            enum ArmState {
                Open,
                Won,
                Lost,
                ChampPath,
            }
            #[derive(Clone, Copy)]
            struct BracketArm {
                p0: Point<Pixels>,
                p1: Point<Pixels>,
                state: ArmState,
                vis_a: f32,
                spt_a: f32,
            }
            let mut arms: Vec<BracketArm> = Vec::new();
            let mut arm_idx = 0usize;
            for r in 0..total_rounds.saturating_sub(1) {
                let x_right = r as f32 * (col_w + col_gap) + col_w;
                let x_next_left = (r + 1) as f32 * (col_w + col_gap);
                for (m_idx, rm) in rounds[r].iter().enumerate() {
                    let (cy_u, cy_l) = slot_centers[r][m_idx];
                    let dst_mk = MatchKey::new(r + 1, m_idx / 2);
                    let next_cy = if m_idx % 2 == 0 {
                        slot_centers[r + 1][m_idx / 2].0
                    } else {
                        slot_centers[r + 1][m_idx / 2].1
                    };
                    let w_id = rm.verdict.as_ref().map(|v| v.winner_id.clone());
                    let up_state = match &w_id {
                        None => ArmState::Open,
                        Some(wid) if rm.upper.as_ref().map(|c| &c.id) == Some(wid) => {
                            if champ_id.as_ref() == Some(wid) {
                                ArmState::ChampPath
                            } else {
                                ArmState::Won
                            }
                        }
                        Some(_) => ArmState::Lost,
                    };
                    let lo_state = match &w_id {
                        None => ArmState::Open,
                        Some(wid) if rm.lower.as_ref().map(|c| &c.id) == Some(wid) => {
                            if champ_id.as_ref() == Some(wid) {
                                ArmState::ChampPath
                            } else {
                                ArmState::Won
                            }
                        }
                        Some(_) => ArmState::Lost,
                    };

                    for (cy_slot, slot_c, st) in [
                        (cy_u, rm.upper.as_ref(), up_state),
                        (cy_l, rm.lower.as_ref(), lo_state),
                    ] {
                        let spt_on = spotlight_key.is_some_and(|hk| {
                            is_lineage(rm.key, hk)
                                && is_lineage(dst_mk, hk)
                                && slot_c.is_some_and(|c| {
                                    spotlight_cids.contains(&c.id) && !matches!(st, ArmState::Lost)
                                })
                        });
                        let vis_target = match spotlight_key {
                            None => 1.0,
                            Some(_) if spt_on => 1.0,
                            Some(hk) if rm.key == hk && !matches!(st, ArmState::Lost) => 0.88,
                            Some(_) => 0.28,
                        };
                        let vis_a = pro_spring_f32(
                            pro_eid(&self.id, format!("jb-arm-op-{arm_idx}")),
                            vis_target,
                            0.22,
                            w,
                            cx,
                        );
                        let spt_a = pro_spring_f32(
                            pro_eid(&self.id, format!("jb-arm-sp-{arm_idx}")),
                            f32::from(spt_on),
                            0.22,
                            w,
                            cx,
                        );
                        arm_idx += 1;
                        arms.push(BracketArm {
                            p0: point(px(x_right), px(cy_slot)),
                            p1: point(px(x_next_left), px(next_cy)),
                            state: st,
                            vis_a,
                            spt_a,
                        });
                    }
                }
            }

            let lines_canvas = canvas(
                |_, _, _| (),
                move |surface, _, window, _| {
                    let ox = surface.origin.x;
                    let oy = surface.origin.y;
                    // Paint non-winner arms first so winner / champion paths sit cleanly on top!
                    for arm in &arms {
                        let a = point(ox + arm.p0.x, oy + arm.p0.y);
                        let b = point(ox + arm.p1.x, oy + arm.p1.y);
                        match arm.state {
                            ArmState::Lost => {
                                paint_dashed_arm(
                                    a,
                                    b,
                                    rail_col.opacity(rail_col.a * arm.vis_a),
                                    window,
                                );
                            }
                            ArmState::Open => {
                                let base_c = pro_mix_color(rail_col, path_blue, arm.spt_a * 0.72);
                                pro_connector_pulse(
                                    a,
                                    b,
                                    6.0,
                                    false,
                                    base_c.opacity(base_c.a * arm.vis_a),
                                    rgb(0xc6e6ff).into(),
                                    if arm.spt_a > 0.25 { pulse_t } else { None },
                                    1.4 + 0.35 * arm.spt_a,
                                    window,
                                );
                            }
                            _ => {}
                        }
                    }
                    for arm in &arms {
                        let a = point(ox + arm.p0.x, oy + arm.p0.y);
                        let b = point(ox + arm.p1.x, oy + arm.p1.y);
                        match arm.state {
                            ArmState::Won => {
                                let base_c = pro_mix_color(won_col, path_blue, arm.spt_a);
                                pro_connector_pulse(
                                    a,
                                    b,
                                    6.0,
                                    false,
                                    base_c.opacity(base_c.a * arm.vis_a),
                                    rgb(0xc6e6ff).into(),
                                    if arm.spt_a > 0.25 { pulse_t } else { None },
                                    1.55 + 0.30 * arm.spt_a,
                                    window,
                                );
                            }
                            ArmState::ChampPath => {
                                pro_connector_pulse(
                                    a,
                                    b,
                                    6.0,
                                    false,
                                    path_blue.opacity(path_blue.a * arm.vis_a),
                                    rgb(0xc6e6ff).into(),
                                    pulse_t,
                                    1.85 + 0.25 * arm.spt_a,
                                    window,
                                );
                            }
                            _ => {}
                        }
                    }
                },
            )
            .absolute()
            .inset_0();

            let mut bracket_canvas = div()
                .w(px(actual_w))
                .h(px(bracket_h))
                .mx_auto()
                .relative()
                .child(lines_canvas);

            for (r, r_matches) in rounds.iter().enumerate() {
                let col_x = r as f32 * (col_w + col_gap);
                let is_fin = r + 1 == total_rounds;
                for (m_idx, rm) in r_matches.iter().enumerate() {
                    let (cy_u, cy_l) = slot_centers[r][m_idx];
                    let top_y = cy_u - 20.0;
                    let box_h = (cy_l + 20.0) - top_y;
                    let mk = rm.key;
                    let is_sel = mk == inspected_key;
                    let m_op = match_opacities[r][m_idx];
                    let w_id = rm.verdict.as_ref().map(|v| v.winner_id.clone());
                    let sc_u = rm.verdict.as_ref().map(|v| v.scores[0]);
                    let sc_l = rm.verdict.as_ref().map(|v| v.scores[1]);
                    let up_won =
                        w_id.is_some() && rm.upper.as_ref().map(|c| &c.id) == w_id.as_ref();
                    let lo_won =
                        w_id.is_some() && rm.lower.as_ref().map(|c| &c.id) == w_id.as_ref();

                    let match_inner = div()
                        .w(px(col_w))
                        .h(px(box_h))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(build_slot(
                            rm.upper.as_ref(),
                            sc_u,
                            up_won,
                            lo_won,
                            rm.judging,
                            is_fin,
                            Some(col_w),
                            format!("{}-{}-u", mk.round, mk.index),
                            cx,
                        ))
                        .child(build_slot(
                            rm.lower.as_ref(),
                            sc_l,
                            lo_won,
                            up_won,
                            rm.judging,
                            is_fin,
                            Some(col_w),
                            format!("{}-{}-l", mk.round, mk.index),
                            cx,
                        ));

                    let match_btn = pro_tactile(
                        pro_eid(&self.id, format!("mbtn-{}-{}", mk.round, mk.index)),
                        round_match_title(mk, total_rounds, cx),
                        11.,
                        transparent_black(),
                        well_bg,
                        if is_sel {
                            path_blue.opacity(0.45)
                        } else {
                            transparent_black()
                        },
                        if is_sel {
                            path_blue.opacity(0.75)
                        } else {
                            hover_border
                        },
                        is_sel,
                        match_inner,
                        cx,
                        move |this, _, cx| {
                            this.select_match(Some(mk), cx);
                        },
                    )
                    .w(px(col_w))
                    .h(px(box_h))
                    .opacity(m_op)
                    .on_hover(cx.listener(
                        move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_match = Some(mk);
                            } else if this.hovered_match == Some(mk) {
                                this.hovered_match = None;
                            }
                            cx.notify();
                        },
                    ));

                    bracket_canvas = bracket_canvas.child(
                        div()
                            .absolute()
                            .left(px(col_x))
                            .top(px(top_y))
                            .child(match_btn),
                    );
                }
            }

            div()
                .w_full()
                .flex()
                .flex_col()
                .child(col_heads)
                .child(bracket_canvas)
                .into_any_element()
        };

        // 3. Side-by-Side Match Comparison & Judge Reasoning Panel
        let inspected_match = rounds
            .get(inspected_key.round)
            .and_then(|rm| rm.get(inspected_key.index))
            .cloned()
            .or_else(|| rounds.first().and_then(|r| r.first()).cloned());

        if let Some(im_v) = inspected_match.as_ref().and_then(|im| im.verdict.as_ref()) {
            self.last_verdict_reason = Some(im_v.reason.clone());
        }
        let cached_reason = self.last_verdict_reason.clone();

        let match_panel = if let Some(im) = inspected_match {
            let match_heading = round_match_title(im.key, total_rounds, cx);
            let meta_right: AnyElement = if im.judging {
                let cmp_txt = if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("{} 正在对比评分…", self.judge)
                } else {
                    format!("{} is comparing…", self.judge)
                };
                pro_shimmer_text(
                    pro_eid(&self.id, "judge-cmp-shim"),
                    cmp_txt,
                    p.muted,
                    p.strong,
                    12.,
                    FontWeight::NORMAL,
                    true,
                )
            } else if im.verdict.is_some() {
                div()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(format!(
                        "{} {}",
                        tr(cx, "Judged by", "裁判模型："),
                        self.judge
                    ))
                    .into_any_element()
            } else {
                div()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(tr(cx, "Waiting for earlier round", "等待上一轮晋级结果"))
                    .into_any_element()
            };

            let render_answer_card = |opt_c: Option<&BracketContender>,
                                      score: Option<f32>,
                                      won: bool,
                                      lost: bool,
                                      side_tag: &'static str|
             -> Div {
                let Some(c) = opt_c else {
                    return div()
                        .flex_1()
                        .min_w_0()
                        .p(px(12.))
                        .rounded(px(9.))
                        .bg(well_bg)
                        .border_1()
                        .border_color(p.line)
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(tr(cx, "Contender not decided yet", "对阵席位待定"));
                };

                div()
                    .flex_1()
                    .min_w_0()
                    .p(px(11.))
                    .rounded(px(9.))
                    .bg(well_bg)
                    .border_1()
                    .border_color(if won { p.green.opacity(0.45) } else { p.line })
                    .opacity(if lost { 0.72 } else { 1.0 })
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        div()
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
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_app_artwork(&c.brand, 11., cx).unwrap_or_else(
                                        || pro_icon(ProIcon::Sparkles, 10., p.ink),
                                    )),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(self.localize_str(&c.name, cx).to_owned()),
                            )
                            .when_some(score, |d, sc| {
                                d.child(
                                    div()
                                        .h(px(19.))
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(if won { p.green_soft } else { p.chip })
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .when(won, |b| {
                                            b.child(pro_check_draw(
                                                pro_eid(
                                                    &self.id,
                                                    format!("ans-chk-{side_tag}-{}", c.id),
                                                ),
                                                true,
                                                12.,
                                                false,
                                                p.green,
                                            ))
                                        })
                                        .child(pro_rolling_number(
                                            pro_eid(
                                                &self.id,
                                                format!("ans-sc-{side_tag}-{}", c.id),
                                            ),
                                            sc as f64,
                                            1,
                                            "",
                                            "",
                                            11.,
                                            FontWeight::MEDIUM,
                                            if won { p.green_ink } else { p.muted },
                                        )),
                                )
                            }),
                    )
                    .when_some(score, |d, sc| {
                        d.child(pro_spring_bar(
                            pro_eid(&self.id, format!("ans-bar-{side_tag}")),
                            (sc / 10.0).clamp(0.0, 1.0),
                            4.0,
                            2.0,
                            p.chip,
                            if won {
                                p.green
                            } else {
                                path_blue.opacity(0.55)
                            },
                        ))
                    })
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .truncate()
                            .child(c.meta.clone()),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .p(px(8.))
                            .rounded(px(6.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line)
                            .font_family(crate::components::pro_chrome::pro_mono_name())
                            .text_size(px(11.5))
                            .line_height(px(17.))
                            .text_color(p.text)
                            .child(c.output.clone()),
                    )
            };

            let w_id = im.verdict.as_ref().map(|v| v.winner_id.clone());
            let sc_u = im.verdict.as_ref().map(|v| v.scores[0]);
            let sc_l = im.verdict.as_ref().map(|v| v.scores[1]);
            let up_won = w_id.is_some() && im.upper.as_ref().map(|c| &c.id) == w_id.as_ref();
            let lo_won = w_id.is_some() && im.lower.as_ref().map(|c| &c.id) == w_id.as_ref();

            let match_winner = if up_won {
                im.upper.clone()
            } else if lo_won {
                im.lower.clone()
            } else {
                champ.clone()
            };
            let live_now = self.live;
            let verdict_reason_txt = im
                .verdict
                .as_ref()
                .map(|v| v.reason.clone())
                .or(cached_reason);
            let verdict_open = im.verdict.is_some() && verdict_reason_txt.is_some();
            let v_ink = p.ink;

            pro_view_transition(
                pro_eid(&self.id, "jb-panel-vtx"),
                format!("{}:{}", im.key.round, im.key.index),
                div()
                    .id(pro_eid(&self.id, "jb-panel"))
                    .role(Role::Status)
                    .w_full()
                    .p(px(14.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(match_heading),
                            )
                            .child(meta_right),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, "jb-verdict-fold"),
                        verdict_open,
                        38.0,
                        10.0,
                        move || {
                            div()
                                .px(px(10.))
                                .py(px(7.))
                                .rounded(px(7.))
                                .bg(brand_bg)
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(v_ink)
                                .child(verdict_reason_txt.unwrap_or_default())
                                .into_any_element()
                        },
                    ))
                    .child(if stacked {
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(render_answer_card(
                                im.upper.as_ref(),
                                sc_u,
                                up_won,
                                lo_won,
                                "u",
                            ))
                            .child(render_answer_card(
                                im.lower.as_ref(),
                                sc_l,
                                lo_won,
                                up_won,
                                "l",
                            ))
                    } else {
                        div()
                            .w_full()
                            .flex()
                            .items_stretch()
                            .gap(px(10.))
                            .child(render_answer_card(
                                im.upper.as_ref(),
                                sc_u,
                                up_won,
                                lo_won,
                                "u",
                            ))
                            .child(render_answer_card(
                                im.lower.as_ref(),
                                sc_l,
                                lo_won,
                                up_won,
                                "l",
                            ))
                    })
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(if self.custom_verdicts.is_none() {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "jb-step"),
                                        tr(cx, "Next match", "下一场评审"),
                                        Some(ProIcon::ArrowRight),
                                        ProBtnStyle::Secondary,
                                        true,
                                        cx,
                                        |this, _, cx| {
                                            this.live = false;
                                            this.selected_match = None;
                                            this.step_tournament(cx);
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "jb-live"),
                                        if live_now {
                                            tr(cx, "Pause", "暂停")
                                        } else {
                                            tr(cx, "Auto", "自动演进")
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
                                            this.selected_match = None;
                                            this.step_started = Instant::now();
                                            cx.notify();
                                        },
                                    ))
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .when_some(match_winner, |d, win_c| {
                                let label = if champ.as_ref().is_some_and(|ch| ch.id == win_c.id) {
                                    format!(
                                        "{} {}",
                                        tr(cx, "Use champion:", "选用冠军方案："),
                                        self.localize_str(&win_c.name, cx)
                                    )
                                } else {
                                    format!(
                                        "{} {}",
                                        tr(cx, "Use", "选用"),
                                        self.localize_str(&win_c.name, cx)
                                    )
                                };
                                d.child(pro_btn(
                                    pro_eid(&self.id, "jb-pick"),
                                    label,
                                    Some(ProIcon::Check),
                                    ProBtnStyle::Primary,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        this.pick_winner(win_c.clone(), cx);
                                    },
                                ))
                            }),
                    ),
            )
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_str(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                let cur = this.selected_match.unwrap_or(inspected_key);
                match ev.keystroke.key.as_str() {
                    "escape" if this.selected_match.is_some() => {
                        this.select_match(None, cx);
                    }
                    "up" if cur.index > 0 => {
                        this.select_match(Some(MatchKey::new(cur.round, cur.index - 1)), cx);
                    }
                    "down" => {
                        let max_m =
                            (this.effective_seed_count() >> (cur.round + 1)).saturating_sub(1);
                        if cur.index < max_m {
                            this.select_match(Some(MatchKey::new(cur.round, cur.index + 1)), cx);
                        }
                    }
                    "right" if cur.round + 1 < this.total_rounds() => {
                        this.select_match(Some(MatchKey::new(cur.round + 1, cur.index / 2)), cx);
                    }
                    "left" if cur.round > 0 => {
                        this.select_match(Some(MatchKey::new(cur.round - 1, cur.index * 2)), cx);
                    }
                    _ => {}
                }
            }))
            .w_full()
            .max_w(px(720.))
            .mx_auto()
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
            .child(head_bar)
            .child(bracket_el)
            .child(match_panel)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{JudgeBracket, MatchKey};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn judge_bracket_advances_rounds_crowns_champion_and_picks_winner(cx: &mut TestAppContext) {
        let view = cx.new(|cx| JudgeBracket::new("test-judge-bracket", cx));

        view.update(cx, |jb, cx| {
            assert!(jb.champion().is_none());

            // Advance through all 7 matches (4 QF + 2 SF + 1 Final)
            for _ in 0..7 {
                jb.step_tournament(cx);
            }
            let champ = jb.champion().expect("champion decided after 7 matches");
            assert_eq!(champ.id, "idempotency");

            jb.select_match(Some(MatchKey::new(1, 1)), cx);
            jb.pick_winner(champ, cx);

            jb.reset(cx);
            assert!(jb.champion().is_none());
        });
    }
}
