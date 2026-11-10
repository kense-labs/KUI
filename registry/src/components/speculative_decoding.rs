//! Explain speculative decoding using a seeded simulation or recorded rounds.
use crate::{
    components::animated_counter::AnimatedCounter,
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        control::activate,
        direct_touch::Contact,
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, measure, standard_ease},
        glyph::{Glyph, icon},
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
};

const NARROW: f32 = 560.;
const STAGGER: f32 = 0.020;
const FLIGHT: f32 = 0.26;

/// A token's actual text and the guess a draft model can make instead.
#[derive(Clone, Debug)]
pub struct DecodeToken {
    pub text: SharedString,
    pub ease: f32,
    pub miss: Option<SharedString>,
}

impl DecodeToken {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            ease: 0.85,
            miss: None,
        }
    }

    pub fn with_ease(mut self, ease: f32) -> Self {
        self.ease = probability(ease);
        self
    }

    pub fn with_miss(mut self, miss: impl Into<SharedString>) -> Self {
        self.miss = Some(miss.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct DecodeRound {
    pub draft: Vec<SharedString>,
    pub accepted: usize,
    pub target: Option<SharedString>,
}

impl DecodeRound {
    pub fn new(draft: Vec<SharedString>, accepted: usize, target: Option<SharedString>) -> Self {
        Self {
            accepted: accepted.min(draft.len()),
            draft,
            target,
        }
    }

    fn accepted(&self) -> usize {
        self.accepted.min(self.draft.len())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeStatus {
    Ready,
    Loading,
    Failed,
}

#[derive(Clone, Debug, Default)]
pub struct DecodeStats {
    pub passes: usize,
    pub drafted: usize,
    pub accepted: usize,
    pub tokens: usize,
    pub acceptance: f32,
    pub tokens_per_pass: f32,
    pub speedup: f32,
}

#[derive(Clone, Debug)]
pub struct DecodeCompleted(pub DecodeStats);
#[derive(Clone, Debug)]
pub struct DecodeRetry;
#[derive(Clone, Debug)]
pub struct DecodePlaybackChanged {
    pub playing: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeDraftLengthChanged(pub usize);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeCompareChanged(pub bool);

fn probability(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0., 1.)
    } else {
        0.85
    }
}

fn simulate(tokens: &[DecodeToken], length: usize, seed: u64) -> Vec<DecodeRound> {
    let mut rounds = Vec::new();
    let mut cursor = 0;
    let mut random = seed;
    while cursor < tokens.len() {
        let count = length.min(tokens.len() - cursor - 1);
        let mut draft = Vec::with_capacity(count);
        let mut accepted = 0;
        for (offset, token) in tokens[cursor..cursor + count].iter().enumerate() {
            random = random
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let chance = (random >> 40) as f32 / 16_777_216.;
            if chance < probability(token.ease) {
                draft.push(token.text.clone());
                if accepted == offset {
                    accepted += 1;
                }
            } else {
                draft.push(token.miss.clone().unwrap_or_else(|| {
                    if token.text.starts_with(' ') {
                        " …".into()
                    } else {
                        "…".into()
                    }
                }));
            }
        }
        cursor += accepted;
        rounds.push(DecodeRound::new(
            draft,
            accepted,
            Some(tokens[cursor].text.clone()),
        ));
        cursor += 1;
    }
    rounds
}

fn default_tokens(cx: &App) -> Vec<DecodeToken> {
    let chinese = crate::primitives::control::UiLanguage::current(cx)
        == crate::primitives::control::UiLanguage::Chinese;
    [
        ("Your", "你的", None, None),
        (" preview", "预览", None, None),
        (" deploy", "部署", Some(" build"), Some("构建")),
        (" finished", "完成", None, None),
        (" in", "耗时", None, None),
        (" 42", "42", Some(" 38"), Some("38")),
        (" seconds", "秒", None, None),
        (",", "，", None, None),
        (" and", "且", None, None),
        (" all", "全部", None, None),
        (" 318", "318", None, None),
        (" checks", "项检查", None, None),
        (" passed", "已通过", None, None),
        (",", "，", None, None),
        (" so", "所以", None, None),
        (" the", "这个", None, None),
        (" branch", "分支", Some(" build"), Some("构建")),
        (" is", "已经", None, None),
        (" ready", "可以", None, None),
        (" to", "进行", None, None),
        (" merge", "合并", None, None),
        (".", "。", None, None),
    ]
    .into_iter()
    .map(|(en, zh, miss_en, miss_zh)| {
        let (label, miss) = if chinese {
            (zh, miss_zh)
        } else {
            (en, miss_en)
        };
        let token = DecodeToken::new(label).with_ease(if miss.is_some() { 0. } else { 1. });
        if let Some(miss) = miss {
            token.with_miss(miss)
        } else {
            token
        }
    })
    .collect()
}

impl DecodeStats {
    fn record(&mut self, round: &DecodeRound, draft_cost: f32) {
        self.passes += 1;
        self.drafted += round.draft.len();
        self.accepted += round.accepted();
        self.tokens += round.accepted() + usize::from(round.target.is_some());
        self.acceptance = self.accepted as f32 / self.drafted.max(1) as f32;
        self.tokens_per_pass = self.tokens as f32 / self.passes as f32;
        self.speedup = self.tokens as f32 / (self.passes as f32 + self.drafted as f32 * draft_cost);
    }
}

#[derive(Clone, Copy)]
struct Beat {
    start: f32,
    drafted: f32,
    swept: f32,
    end: f32,
    first_token: usize,
    token_count: usize,
}
#[derive(Clone)]
struct OutputToken {
    text: SharedString,
    draft: bool,
    start: f32,
}

#[derive(Default)]
struct Plan {
    beats: Vec<Beat>,
    tokens: Vec<OutputToken>,
    words: Vec<Range<usize>>,
    totals: Vec<DecodeStats>,
}

fn starts_word(text: &str) -> bool {
    text.chars().next().is_some_and(|ch| {
        ch.is_whitespace() || matches!(ch, '\u{3400}'..='\u{9fff}' | '\u{20000}'..='\u{323af}')
    })
}

impl Plan {
    fn new(rounds: &[DecodeRound], pass: f32, draft_cost: f32) -> Self {
        let mut plan = Self {
            totals: vec![DecodeStats::default()],
            ..Self::default()
        };
        let mut start = 0.;
        let mut stats = DecodeStats::default();
        for round in rounds {
            let count = round.accepted() + usize::from(round.target.is_some());
            let drafted = start + round.draft.len() as f32 * pass * draft_cost;
            let swept = drafted + pass * 0.60;
            let end =
                (drafted + pass).max(swept + FLIGHT + count.saturating_sub(1) as f32 * STAGGER);
            plan.beats.push(Beat {
                start,
                drafted,
                swept,
                end,
                first_token: plan.tokens.len(),
                token_count: count,
            });
            for (slot, text) in round
                .draft
                .iter()
                .take(round.accepted())
                .chain(round.target.iter())
                .enumerate()
            {
                let index = plan.tokens.len();
                if index == 0 || starts_word(text) {
                    plan.words.push(index..index + 1);
                } else if let Some(word) = plan.words.last_mut() {
                    word.end = index + 1;
                }
                plan.tokens.push(OutputToken {
                    text: text.clone(),
                    draft: slot < round.accepted(),
                    start: swept + slot as f32 * STAGGER,
                });
            }
            stats.record(round, draft_cost);
            plan.totals.push(stats.clone());
            start = end;
        }
        plan
    }

    fn end(&self) -> f32 {
        self.beats.last().map_or(0., |beat| beat.end)
    }
    fn passed(&self, time: f32) -> usize {
        self.beats.partition_point(|beat| beat.end <= time)
    }
    fn text(&self, count: usize) -> String {
        self.tokens
            .iter()
            .take(count)
            .map(|t| t.text.as_ref())
            .collect()
    }
}

type TokenBounds = Rc<RefCell<Vec<Bounds<Pixels>>>>;
type CardStyle = Box<dyn Fn(Div) -> Div>;

fn mark_bounds(items: &TokenBounds, index: usize) -> impl IntoElement {
    let items = items.clone();
    canvas(
        move |bounds, _, _| items.borrow_mut()[index] = bounds,
        |_, _, _, _| (),
    )
    .absolute()
    .inset_0()
}

#[derive(Default)]
struct StatusLabel {
    shown: SharedString,
    previous: SharedString,
    transition: PhaseClock,
    shimmer: PhaseClock,
}

impl StatusLabel {
    fn view(
        &mut self,
        next: SharedString,
        loading: bool,
        height: f32,
        p: Paint,
        w: &Window,
        cx: &App,
    ) -> Div {
        if self.shown != next {
            if !self.shown.is_empty() {
                self.transition.restart(cx);
            }
            self.previous = std::mem::replace(&mut self.shown, next.clone());
        }
        let progress = (self.transition.time(0.16, w, cx) / 0.16).clamp(0., 1.);
        let wave = if loading && !disabled_motion(cx) {
            w.request_animation_frame();
            if !self.shimmer.age(cx).is_finite() {
                self.shimmer.restart(cx);
            }
            0.72 + 0.28 * (self.shimmer.age(cx) * 4.6).cos()
        } else {
            1.
        };
        div()
            .relative()
            .w_full()
            .h(px(height))
            .text_size(px(14.))
            .line_height(px(19.6))
            .text_color(p.secondary)
            .when(progress < 1., |d| {
                d.child(
                    div()
                        .absolute()
                        .inset_0()
                        .opacity(1. - progress)
                        .child(self.previous.clone()),
                )
            })
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .opacity(progress * wave)
                    .child(next),
            )
    }
}

pub struct SpeculativeDecoding {
    rounds: Vec<DecodeRound>,
    tokens: Option<Vec<DecodeToken>>,
    plan: Plan,
    status: DecodeStatus,
    error: SharedString,
    recorded: bool,
    allow_retry: bool,
    compare: bool,
    compare_controlled: bool,
    draft_length: usize,
    length_controlled: bool,
    seed: u64,
    elapsed: f32,
    last_age: f32,
    clock: PhaseClock,
    visible: bool,
    playing: bool,
    autoplay: bool,
    started: bool,
    completed: bool,
    draft_cost: f32,
    pass: f32,
    title: Option<SharedString>,
    model_labels: [Option<SharedString>; 4],
    id: ElementId,
    style: Option<CardStyle>,
    announcement: SharedString,
    status_label: StatusLabel,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    track: Rc<Cell<Bounds<Pixels>>>,
    slider_focus: Option<FocusHandle>,
    touch: Contact,
    dragging: bool,
    lanes: Option<Slide>,
    compare_height: Slide,
    compare_knob: Slide,
    counters: Option<[Entity<AnimatedCounter>; 3]>,
}

impl EventEmitter<DecodeCompleted> for SpeculativeDecoding {}
impl EventEmitter<DecodeRetry> for SpeculativeDecoding {}
impl EventEmitter<DecodePlaybackChanged> for SpeculativeDecoding {}
impl EventEmitter<DecodeDraftLengthChanged> for SpeculativeDecoding {}
impl EventEmitter<DecodeCompareChanged> for SpeculativeDecoding {}

impl SpeculativeDecoding {
    /// Replay recorded rounds. Use from_tokens for a simulated run and slider.
    pub fn new(rounds: Vec<DecodeRound>) -> Self {
        let plan = Plan::new(&rounds, 0.64, 0.1);
        Self {
            rounds,
            tokens: None,
            plan,
            status: DecodeStatus::Ready,
            error: "".into(),
            recorded: false,
            allow_retry: false,
            compare: false,
            compare_controlled: false,
            draft_length: 4,
            length_controlled: false,
            seed: 12,
            elapsed: 0.,
            last_age: 0.,
            clock: Default::default(),
            visible: true,
            playing: false,
            autoplay: true,
            started: false,
            completed: false,
            draft_cost: 0.1,
            pass: 0.64,
            title: None,
            model_labels: Default::default(),
            id: "speculative-decoding".into(),
            style: None,
            announcement: "".into(),
            status_label: Default::default(),
            bounds: Default::default(),
            track: Default::default(),
            slider_focus: None,
            touch: Default::default(),
            dragging: false,
            lanes: None,
            compare_height: Slide::new(0.),
            compare_knob: Slide::new(0.),
            counters: None,
        }
    }

    /// Simulate guesses for a caller's answer. The final token is always written by the target.
    pub fn from_tokens(tokens: Vec<DecodeToken>) -> Self {
        let mut value = Self::new(Vec::new());
        value.tokens = Some(tokens);
        value.replan();
        value
    }

    /// A local, illustrative run in the current language.
    pub fn sample(cx: &App) -> Self {
        Self::from_tokens(default_tokens(cx))
    }

    pub fn with_recorded(mut self, recorded: bool) -> Self {
        self.recorded = recorded;
        self
    }
    pub fn with_autoplay(mut self, autoplay: bool) -> Self {
        self.autoplay = autoplay;
        self
    }
    pub fn with_retry(mut self, enabled: bool) -> Self {
        self.allow_retry = enabled;
        self
    }
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn with_model_labels(
        mut self,
        draft: impl Into<SharedString>,
        draft_detail: impl Into<SharedString>,
        target: impl Into<SharedString>,
        target_detail: impl Into<SharedString>,
    ) -> Self {
        self.model_labels = [
            Some(draft.into()),
            Some(draft_detail.into()),
            Some(target.into()),
            Some(target_detail.into()),
        ];
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self.replan();
        self
    }
    pub fn with_draft_cost(mut self, cost: f32) -> Self {
        if cost.is_finite() {
            self.draft_cost = cost.max(0.);
            self.replan();
        }
        self
    }
    pub fn with_pass_duration_ms(mut self, milliseconds: u32) -> Self {
        self.pass = milliseconds.max(1) as f32 / 1000.;
        self.replan();
        self
    }
    pub fn with_default_draft_length(mut self, length: usize) -> Self {
        self.draft_length = length.clamp(2, 8);
        self.replan();
        self
    }
    pub fn with_draft_length(mut self, length: usize) -> Self {
        self.length_controlled = true;
        self.with_default_draft_length(length)
    }
    pub fn with_default_compare(mut self, compare: bool) -> Self {
        self.compare = compare;
        self.compare_knob.reset(if compare { 1. } else { 0. });
        self
    }
    pub fn with_compare(mut self, compare: bool) -> Self {
        self.compare_controlled = true;
        self.with_default_compare(compare)
    }

    fn replan(&mut self) {
        if let Some(tokens) = &self.tokens {
            self.rounds = simulate(tokens, self.draft_length, self.seed);
        }
        self.plan = Plan::new(&self.rounds, self.pass, self.draft_cost);
    }

    fn end(&self) -> f32 {
        self.plan.end().max(if self.compare {
            self.plan.tokens.len() as f32 * self.pass
        } else {
            0.
        })
    }

    fn clear_clock(&mut self, cx: &App) {
        self.clock.restart(cx);
        self.last_age = 0.;
    }

    pub fn set_rounds(&mut self, rounds: Vec<DecodeRound>, cx: &mut Context<Self>) {
        self.tokens = None;
        self.rounds = rounds;
        self.replan();
        self.elapsed = 0.;
        self.completed = false;
        self.playing &= !self.rounds.is_empty();
        self.clear_clock(cx);
        cx.notify();
    }

    pub fn set_draft_length(&mut self, length: usize, cx: &mut Context<Self>) {
        let length = length.clamp(2, 8);
        if self.draft_length == length || self.tokens.is_none() {
            return;
        }
        self.draft_length = length;
        self.replan();
        self.restart(cx);
        self.announcement = format!("{} · {length}", tr(cx, "Draft length", "草稿长度")).into();
        cx.notify();
    }

    fn change_length(&mut self, length: usize, cx: &mut Context<Self>) {
        if self.status != DecodeStatus::Ready || self.tokens.is_none() {
            return;
        }
        let length = length.clamp(2, 8);
        if length == self.draft_length {
            return;
        }
        if !self.length_controlled {
            self.set_draft_length(length, cx);
        }
        cx.emit(DecodeDraftLengthChanged(length));
    }

    pub fn set_status(
        &mut self,
        status: DecodeStatus,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.status = status;
        self.error = error.into();
        if status != DecodeStatus::Ready {
            self.playing = false;
            self.dragging = false;
        }
        self.clear_clock(cx);
        cx.notify();
    }

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        self.clear_clock(cx);
        if let Some(counters) = &self.counters {
            for counter in counters {
                counter.update(cx, |counter, cx| counter.set_visible(visible, cx));
            }
        }
        if visible {
            cx.notify();
        }
    }

    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.status != DecodeStatus::Ready || self.rounds.is_empty() {
            return;
        }
        if playing && self.elapsed >= self.end() {
            self.elapsed = 0.;
            self.completed = false;
        }
        if self.playing != playing {
            self.playing = playing;
            self.started = true;
            self.clear_clock(cx);
            self.announcement = if playing {
                tr(cx, "Playing", "正在播放")
            } else {
                tr(cx, "Paused", "已暂停")
            }
            .into();
            cx.emit(DecodePlaybackChanged { playing });
            cx.notify();
        }
    }

    pub fn restart(&mut self, cx: &mut Context<Self>) {
        if self.status != DecodeStatus::Ready || self.rounds.is_empty() {
            return;
        }
        self.elapsed = 0.;
        self.completed = false;
        self.started = true;
        self.playing = true;
        self.clear_clock(cx);
        self.announcement = tr(cx, "Restarted", "已重新开始").into();
        cx.notify();
    }

    pub fn set_compare(&mut self, compare: bool, cx: &mut Context<Self>) {
        if self.compare == compare {
            return;
        }
        self.compare = compare;
        if self.completed
            && compare
            && self.elapsed < self.end()
            && self.status == DecodeStatus::Ready
        {
            self.playing = true;
            self.clear_clock(cx);
        }
        if !compare && self.elapsed >= self.end() {
            self.playing = false;
        }
        self.announcement = if compare {
            tr(cx, "Comparison on", "已开启对照")
        } else {
            tr(cx, "Comparison off", "已关闭对照")
        }
        .into();
        cx.notify();
    }

    fn compare_clicked(&mut self, cx: &mut Context<Self>) {
        let compare = !self.compare;
        if !self.compare_controlled {
            self.set_compare(compare, cx);
        }
        cx.emit(DecodeCompareChanged(compare));
    }

    fn move_slider(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        let track = self.track.get();
        if track.size.width <= px(0.) {
            return;
        }
        let t = ((position.x - track.left()) / track.size.width).clamp(0., 1.);
        self.change_length(2 + (t * 6.).round() as usize, cx);
    }
}

#[derive(Clone, Copy)]
struct ModelPaint {
    draft: Hsla,
    target: Hsla,
    theme: Paint,
}

impl ModelPaint {
    fn new(cx: &App) -> Self {
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        Self {
            draft: rgb(if dark { 0xb990f5 } else { 0x7941a5 }).into(),
            target: rgb(if dark { 0x6dbbff } else { 0x187bb4 }).into(),
            theme: Paint::get(cx),
        }
    }
}

fn text(label: impl Into<SharedString>, size: f32, leading: f32, ink: Hsla) -> Div {
    div()
        .text_size(px(size))
        .line_height(px(leading))
        .text_color(ink)
        .child(label.into())
}

fn lane_label(name: SharedString, detail: SharedString, color: Hsla, p: Paint) -> Div {
    row()
        .gap(px(10.))
        .h(px(32.))
        .child(div().size(px(7.)).rounded_full().bg(color).flex_none())
        .child(column().child(text(name, 14., 16., p.ink)).child(text(
            detail,
            12.,
            15.,
            p.secondary,
        )))
}

fn wrap_height(
    widths: impl Iterator<Item = f32>,
    width: f32,
    gap: f32,
    row_height: f32,
    row_gap: f32,
) -> f32 {
    let mut rows = 1;
    let mut filled = 0.;
    for item in widths {
        if filled > 0. && filled + gap + item > width + 0.01 {
            rows += 1;
            filled = 0.;
        }
        filled += if filled == 0. { item } else { gap + item };
    }
    rows as f32 * row_height + (rows - 1) as f32 * row_gap
}

#[derive(Clone, Copy)]
struct Moment {
    time: f32,
    round: Beat,
    reduced: bool,
}

impl Moment {
    fn proposal(self, index: usize, draft_len: usize) -> f32 {
        let step = (self.round.drafted - self.round.start) / draft_len.max(1) as f32;
        let age = self.time - self.round.start - index as f32 * step;
        if self.reduced {
            f32::from(age >= 0.)
        } else {
            standard_ease((age / step.max(0.05)).clamp(0., 1.))
        }
    }

    fn sweep(self, index: usize, draft_len: usize) -> f32 {
        let count = draft_len.max(1) as f32;
        let sweep = (self.time - self.round.drafted) / (self.round.swept - self.round.drafted)
            * count
            - index as f32;
        let sweep = sweep.clamp(0., 1.);
        if self.reduced {
            f32::from(sweep >= 1.)
        } else {
            sweep
        }
    }

    fn landing(self, time: f32) -> f32 {
        let progress = ((self.time - time) / FLIGHT).clamp(0., 1.);
        if self.reduced {
            f32::from(self.time >= time)
        } else {
            progress
        }
    }
}

fn cell_base(radius: Pixels, p: Paint) -> Div {
    row()
        .relative()
        .w_full()
        .h(px(32.))
        .px(px(10.))
        .gap(px(6.))
        .rounded(radius)
        .bg(p.stage)
        .overflow_hidden()
}

fn draft_columns(
    round: &DecodeRound,
    moment: Moment,
    widths: &[f32],
    bounds: &TokenBounds,
    p: ModelPaint,
    radius: Pixels,
) -> Div {
    let draft_len = round.draft.len();
    let accepted = round.accepted();
    row()
        .flex_wrap()
        .items_start()
        .gap_x(px(6.))
        .gap_y(px(8.))
        .children(widths.iter().enumerate().map(|(i, &width)| {
            let proposal = moment.proposal(i, draft_len);
            let sweep = moment.sweep(i, draft_len);
            let visible = if i > accepted {
                proposal * (1. - sweep * 0.68)
            } else {
                proposal
            };
            let discarded = i > accepted && sweep >= 1.;
            let rejected = i == accepted && sweep > 0. && i < draft_len;
            let has_target = i == accepted && round.target.is_some();
            let color = if rejected { p.theme.danger } else { p.target };
            let target_reveal = if has_target && i == draft_len {
                ((moment.time - moment.round.swept + 0.12) / 0.12).clamp(0., 1.)
            } else {
                sweep
            };
            let marker = if discarded {
                None
            } else if rejected {
                Some(Glyph::Close)
            } else if i < accepted && sweep > 0. {
                Some(Glyph::Check)
            } else {
                None
            };
            let draft = if let Some(label) = round.draft.get(i) {
                cell_base(radius, p.theme)
                    .left(px(if moment.reduced {
                        0.
                    } else {
                        -4. * (1. - proposal)
                    }))
                    .opacity(visible)
                    .child(
                        text(label.trim_start().to_string(), 14., 20., p.theme.ink)
                            .min_w_0()
                            .truncate()
                            .when(rejected, |d| {
                                d.relative().child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top(relative(0.51))
                                        .w(relative(sweep))
                                        .h(px(1.))
                                        .bg(p.theme.danger),
                                )
                            }),
                    )
            } else {
                div().h(px(32.))
            };
            let check = cell_base(radius, p.theme)
                .bg(p.theme.panel)
                .border_1()
                .border_color(p.theme.line)
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(radius)
                        .bg(p
                            .theme
                            .panel
                            .blend(color.opacity(if discarded { 0.05 } else { 0.13 })))
                        .w(relative(sweep)),
                )
                .when_some(marker, |d, glyph| {
                    d.child(div().opacity(sweep).child(icon(
                        glyph,
                        13.,
                        if glyph == Glyph::Close {
                            p.theme.danger
                        } else {
                            p.target
                        },
                    )))
                })
                .when(has_target, |d| {
                    d.child(
                        div()
                            .relative()
                            .min_w_0()
                            .flex_1()
                            .h(px(20.))
                            .opacity(target_reveal)
                            .child(
                                text(
                                    round
                                        .target
                                        .as_deref()
                                        .unwrap_or("")
                                        .trim_start()
                                        .to_string(),
                                    14.,
                                    20.,
                                    p.target,
                                )
                                .truncate(),
                            )
                            .child(mark_bounds(bounds, widths.len() + i)),
                    )
                });
            column()
                .flex_none()
                .w(px(width))
                .gap(px(8.))
                .child(draft.relative().child(mark_bounds(bounds, i)))
                .child(check)
        }))
}

fn output_line(
    plan: &Plan,
    count: usize,
    moment: Moment,
    baseline: bool,
    positions: Option<&TokenBounds>,
    p: ModelPaint,
    cx: &App,
) -> impl IntoElement {
    let spoken = if count == 0 {
        tr(cx, "No output yet", "尚无输出").to_string()
    } else {
        plan.text(count)
    };
    div()
        .id(if baseline {
            "decode-baseline"
        } else {
            "decode-output"
        })
        .role(Role::Label)
        .aria_label(spoken)
        .min_w_0()
        .w_full()
        .flex()
        .flex_wrap()
        .items_start()
        .children(plan.words.iter().map(|word| {
            row()
                .max_w_full()
                .flex_none()
                .h(px(25.))
                .children(word.clone().map(|i| {
                    let token = &plan.tokens[i];
                    let landing = moment.landing(token.start);
                    let opacity = if baseline {
                        f32::from(i < count)
                    } else {
                        ((landing - 0.82) / 0.18).clamp(0., 1.)
                    };
                    let color = if token.draft && !baseline {
                        p.draft
                    } else {
                        p.target
                    };
                    div()
                        .relative()
                        .min_w_0()
                        .flex_none()
                        .text_size(px(14.))
                        .line_height(px(25.))
                        .text_color(p.theme.ink)
                        .whitespace_nowrap()
                        .child(
                            div()
                                .relative()
                                .opacity(opacity)
                                .child(token.text.clone())
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(if token.text.starts_with(' ') { 3. } else { 0. }))
                                        .right_0()
                                        .bottom(px(1.))
                                        .h(px(1.))
                                        .bg(color.opacity(0.68)),
                                ),
                        )
                        .when_some(positions, |d, b| d.child(mark_bounds(b, i)))
                }))
        }))
}

fn labelled_line(
    name: &'static str,
    passes: usize,
    narrow: bool,
    value: impl IntoElement,
    p: Paint,
    cx: &App,
) -> Div {
    let count = if crate::primitives::control::UiLanguage::current(cx)
        == crate::primitives::control::UiLanguage::Chinese
    {
        format!("{passes} 次验证")
    } else if passes == 1 {
        "1 pass".to_owned()
    } else {
        format!("{passes} passes")
    };
    let label = div()
        .flex()
        .flex_none()
        .gap(px(2.))
        .when(narrow, |d| d.w_full().items_center().justify_between())
        .when(!narrow, |d| d.w(px(120.)).flex_col().pt(px(3.)))
        .child(text(name, 14., 17., p.ink))
        .child(text(count, 12., 16., p.secondary));
    div()
        .flex()
        .w_full()
        .min_w_0()
        .items_start()
        .when(narrow, |d| d.flex_col().gap(px(4.)))
        .when(!narrow, |d| d.gap(px(16.)))
        .child(label)
        .child(div().min_w_0().flex_1().w_full().child(value))
}

fn token_flight(
    active: Beat,
    time: f32,
    tokens: Vec<OutputToken>,
    columns: usize,
    sources: TokenBounds,
    destinations: TokenBounds,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |_, _, w, cx| {
            let sources = sources.borrow();
            let destinations = destinations.borrow();
            for (slot, token) in tokens.iter().enumerate() {
                let t = (time - token.start) / FLIGHT;
                if !(0. ..1.).contains(&t) {
                    continue;
                }
                let source_index = if token.draft { slot } else { columns + slot };
                let Some(source) = sources.get(source_index) else {
                    continue;
                };
                let Some(dest) = destinations.get(active.first_token + slot) else {
                    continue;
                };
                if source.size.height <= px(0.) || dest.size.width <= px(0.) {
                    continue;
                }
                let start = if token.draft {
                    source.origin + point(px(10.), px(6.))
                } else {
                    source.origin
                };
                let mut run = w.text_style().to_run(token.text.trim_start().len());
                run.color = Paint::get(cx)
                    .ink
                    .opacity((1. - ((t - 0.82) / 0.18).clamp(0., 1.)).min(t * 8.));
                let line = w.text_system().shape_line(
                    token.text.trim_start().to_string().into(),
                    px(14.),
                    &[run],
                    None,
                );
                let progress = ((1. - (1. + 8. * t) * (-8. * t).exp()) / 0.99698).clamp(0., 1.);
                let finish = dest.origin
                    + point(
                        px(if token.text.starts_with(' ') { 3. } else { 0. }),
                        px(2.5),
                    );
                let _ = line.paint(
                    start + (finish - start) * progress,
                    px(20.),
                    TextAlign::Left,
                    None,
                    w,
                    cx,
                );
            }
        },
    )
    .absolute()
    .inset_0()
}

impl SpeculativeDecoding {
    fn tick(&mut self, w: &Window, cx: &mut Context<Self>) {
        let bounds = self.bounds.get();
        let viewport = w.viewport_size();
        let visible_height = bounds.bottom().min(viewport.height) - bounds.top().max(px(0.));
        let in_view = visible_height > bounds.size.height * 0.15;
        let ready = self.visible
            && w.is_visible()
            && in_view
            && self.status == DecodeStatus::Ready
            && !self.rounds.is_empty();
        if ready
            && self.autoplay
            && !self.started
            && !disabled_motion(cx)
            && visible_height > bounds.size.height * 0.60
        {
            self.started = true;
            self.playing = true;
            self.clear_clock(cx);
        }
        if !ready || !self.playing {
            self.clear_clock(cx);
            return;
        }
        let age = self.clock.age(cx);
        if !age.is_finite() {
            self.clear_clock(cx);
            return;
        }
        self.elapsed = (self.elapsed + (age - self.last_age).clamp(0., 0.05)).min(self.end());
        self.last_age = age;
        if !self.completed && self.elapsed >= self.plan.end() {
            self.completed = true;
            let stats = self.plan.totals.last().cloned().unwrap_or_default();
            self.announcement = if crate::primitives::control::UiLanguage::current(cx)
                == crate::primitives::control::UiLanguage::Chinese
            {
                format!("完成 · {} 次验证", stats.passes).into()
            } else {
                format!("Done in {} target passes", stats.passes).into()
            };
            cx.emit(DecodeCompleted(stats));
        }
        if self.elapsed < self.end() {
            w.request_animation_frame();
        } else {
            self.playing = false;
        }
    }

    fn slider(&mut self, ready: bool, p: Paint, cx: &mut Context<Self>) -> Div {
        let focus = self
            .slider_focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let name = tr(cx, "Draft length", "草稿长度");
        let value = format!("{} {}", self.draft_length, tr(cx, "tokens", "个 token"));
        let fraction = (self.draft_length - 2) as f32 / 6.;
        let track = self.track.clone();
        let drag =
            cx.listener(|s, event: &MouseMoveEvent, _, cx| s.move_slider(event.position, cx));
        let end = cx.listener(|s, _: &MouseUpEvent, _, _| s.dragging = false);
        let touch = self.touch.listen(cx, |s, event, w, cx| match event.phase {
            TouchPhase::Started if s.status == DecodeStatus::Ready => {
                s.dragging = true;
                if let Some(focus) = &s.slider_focus {
                    focus.focus(w, cx);
                }
                s.move_slider(event.position, cx);
            }
            TouchPhase::Moved => s.move_slider(event.position, cx),
            TouchPhase::Ended | TouchPhase::Cancelled => s.dragging = false,
            _ => {}
        });
        let thumb = div()
            .id("decode-length-thumb")
            .role(Role::Slider)
            .aria_label(name)
            .aria_value(value.clone())
            .aria_numeric_value(self.draft_length as f64)
            .aria_min_numeric_value(2.)
            .aria_max_numeric_value(8.)
            .aria_numeric_value_step(1.)
            .track_focus(&focus)
            .tab_stop(ready)
            .absolute()
            .left(relative(fraction))
            .ml(px(-11.))
            .top(px(9.))
            .size(px(22.))
            .rounded_full()
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .shadow_sm()
            .focus_visible(move |d| d.border_2().border_color(p.focus))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let value = match e.keystroke.key.as_str() {
                    "left" | "down" => s.draft_length.saturating_sub(1),
                    "right" | "up" => s.draft_length + 1,
                    "home" => 2,
                    "end" => 8,
                    _ => return,
                };
                s.change_length(value, cx);
                w.prevent_default();
                cx.stop_propagation();
            }));
        let measured = canvas(
            move |bounds, _, _| track.set(bounds),
            move |_, _, w, _| {
                if ready {
                    w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                        if phase.capture() {
                            drag(e, w, cx);
                        }
                    });
                    w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                        if phase.capture() && e.button == MouseButton::Left {
                            end(e, w, cx);
                        }
                    });
                }
            },
        )
        .absolute()
        .inset_0();
        let rail = div()
            .relative()
            .mx(px(11.))
            .h(px(40.))
            .child(measured)
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(17.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(p.hover),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .w(relative(fraction))
                    .top(px(17.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(p.bright),
            )
            .child(thumb);
        let slot = div()
            .id("decode-length")
            .relative()
            .cursor_pointer()
            .child(rail)
            .when(ready, |d| {
                d.child(touch).on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, w, cx| {
                        if w.last_input_was_touch() {
                            return;
                        }
                        s.dragging = true;
                        if let Some(focus) = &s.slider_focus {
                            focus.focus(w, cx);
                        }
                        s.move_slider(e.position, cx);
                    }),
                )
            });
        column()
            .min_w_0()
            .w_full()
            .max_w(px(320.))
            .gap(px(4.))
            .child(
                row()
                    .justify_between()
                    .gap(px(4.))
                    .child(text(name, 14., 20., p.ink))
                    .child(text(value, 14., 20., p.secondary)),
            )
            .child(slot)
    }

    fn stats_row(
        &mut self,
        stats: DecodeStats,
        narrow: bool,
        p: Paint,
        cx: &mut Context<Self>,
    ) -> Div {
        let font_size = if narrow { 20. } else { 22. };
        let values = [
            stats.speedup as f64,
            (stats.acceptance * 100.).round() as f64,
            stats.tokens_per_pass as f64,
        ];
        let counters = self.counters.get_or_insert_with(|| {
            [0, 1, 2].map(|i| {
                cx.new(|_| {
                    AnimatedCounter::new(0.)
                        .with_size(font_size)
                        .with_accessibility(false)
                        .with_decimals(usize::from(i != 1))
                        .with_suffix(["×", "%", ""][i])
                })
            })
        });
        let labels = [
            tr(cx, "Faster", "加速比"),
            tr(cx, "Drafts accepted", "草稿接受率"),
            tr(cx, "Tokens per pass", "每次验证词元"),
        ];
        row()
            .w_full()
            .items_start()
            .gap(px(if narrow { 12. } else { 16. }))
            .children(labels.into_iter().zip(counters.iter()).enumerate().map(
                |(i, (label, counter))| {
                    let value = values[i];
                    counter.update(cx, |c, cx| {
                        c.set_wheel_size(font_size);
                        c.set_visible(self.visible, cx);
                        c.set_value(value, cx);
                    });
                    let available = if i == 1 {
                        stats.drafted > 0
                    } else {
                        stats.passes > 0
                    };
                    let spoken = if available {
                        format!("{label} {:.1}{}", value, ["×", "%", ""][i])
                    } else {
                        format!("{label} · {}", tr(cx, "not yet", "尚无数据"))
                    };
                    column()
                        .id(("decode-stat", i))
                        .role(Role::Label)
                        .aria_label(spoken)
                        .min_w_0()
                        .flex_1()
                        .gap(px(2.))
                        .child(
                            div()
                                .h(px(font_size + 5.))
                                .flex()
                                .items_center()
                                .when(available, |d| d.child(counter.clone()))
                                .when(!available, |d| {
                                    d.child(text("–", font_size, font_size + 5., p.secondary))
                                }),
                        )
                        .child(text(label, 12., 17., p.secondary))
                },
            ))
    }
}

impl Render for SpeculativeDecoding {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tick(w, cx);
        let colors = ModelPaint::new(cx);
        let p = colors.theme;
        let radius = surface_radius(cx);
        let reduced = disabled_motion(cx);
        let bounds = self.bounds.get();
        let width = if bounds.size.width > px(0.) {
            f32::from(bounds.size.width)
        } else {
            (f32::from(w.viewport_size().width) - 32.).clamp(200., 720.)
        };
        let narrow = width < NARROW;
        let stacked_lanes = width < 640.;
        let pad = if narrow { 16. } else { 24. };
        let inner_width = width - pad * 2. - 2.;
        let line_width = (inner_width - if stacked_lanes { 0. } else { 136. }).max(50.);
        let ready = self.status == DecodeStatus::Ready;
        let time = self.elapsed;
        let done = self.plan.passed(time);
        let finished = self.completed;
        let active = self.plan.beats.get(done).copied();
        let round = self.rounds.get(done);
        let totals = self.plan.totals[done].clone();
        let baseline = (time / self.pass).floor() as usize;
        let baseline = baseline.min(self.plan.tokens.len());
        let status = match self.status {
            DecodeStatus::Loading => tr(cx, "Loading run…", "正在加载运行记录…").to_owned(),
            DecodeStatus::Failed if self.error.is_empty() => {
                tr(cx, "Unable to load the run", "无法加载运行记录").to_owned()
            }
            DecodeStatus::Failed => self.error.to_string(),
            DecodeStatus::Ready if self.rounds.is_empty() => {
                tr(cx, "No rounds supplied", "没有可回放的轮次").to_owned()
            }
            DecodeStatus::Ready
                if finished && self.compare && baseline < self.plan.tokens.len() =>
            {
                if tr(cx, "en", "zh") == "zh" {
                    format!("推测解码已用 {done} 次验证完成，仅目标模型仍在写入")
                } else {
                    format!("Speculation done in {done} passes, target alone still writing")
                }
            }
            DecodeStatus::Ready if finished => {
                if tr(cx, "en", "zh") == "zh" {
                    format!("完成 · {done} 次目标验证")
                } else {
                    format!("Done in {done} target passes")
                }
            }
            DecodeStatus::Ready if !self.started && self.recorded => {
                tr(cx, "Recorded run", "回放记录").to_owned()
            }
            DecodeStatus::Ready if !self.started && self.tokens.is_some() => {
                if tr(cx, "en", "zh") == "zh" {
                    format!("草稿预写 {} 个 token，目标一次验证", self.draft_length)
                } else {
                    format!(
                        "Draft guesses {} tokens ahead, target checks them in one pass",
                        self.draft_length
                    )
                }
            }
            DecodeStatus::Ready if !self.playing => tr(cx, "Paused", "已暂停").to_owned(),
            DecodeStatus::Ready => {
                if let (Some(round), Some(beat)) = (round, active) {
                    let length = round.draft.len();
                    let accepted = round.accepted();
                    if tr(cx, "en", "zh") == "zh" {
                        if time < beat.drafted {
                            format!("草稿模型提出 {length} 个 token")
                        } else if time < beat.swept {
                            format!("目标模型一次验证 {length} 个 token")
                        } else {
                            format!("接受 {accepted} / {length} · 目标模型写入答案")
                        }
                    } else if time < beat.drafted {
                        format!("Draft model proposes {length} tokens")
                    } else if time < beat.swept {
                        format!("Target checks all {length} in one pass")
                    } else if accepted < length {
                        format!("{accepted} of {length} accepted, target replaces the first miss")
                    } else {
                        format!("{accepted} of {length} accepted, target adds the next token")
                    }
                } else {
                    tr(cx, "Ready to replay", "准备回放").to_owned()
                }
            }
        };
        let title = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Speculative decoding", "推测解码").into());
        let status_view = self.status_label.view(
            status.into(),
            self.status == DecodeStatus::Loading && self.visible,
            if narrow { 40. } else { 20. },
            p,
            w,
            cx,
        );
        let playing = self.playing;
        let play_name = if playing {
            tr(cx, "Pause", "暂停")
        } else {
            tr(cx, "Play", "播放")
        };
        let restart_name = tr(cx, "Restart", "重新开始");
        let play_icon = if playing { Glyph::Pause } else { Glyph::Play };
        let actions = row()
            .flex_none()
            .gap(px(8.))
            .child(activate(
                face("decode-play", play_name, 32., 999., Tone::Surface, cx)
                    .w(px(32.))
                    .rounded_full()
                    .border_1()
                    .border_color(p.line)
                    .tab_stop(ready)
                    .opacity(if ready { 1. } else { 0.5 })
                    .child(icon(play_icon, 15., p.secondary)),
                cx,
                |s, _, cx| s.set_playing(!s.playing, cx),
            ))
            .child(activate(
                face("decode-restart", restart_name, 32., 999., Tone::Surface, cx)
                    .w(px(32.))
                    .rounded_full()
                    .border_1()
                    .border_color(p.line)
                    .tab_stop(ready)
                    .opacity(if ready { 1. } else { 0.5 })
                    .child(icon(Glyph::Undo, 15., p.secondary)),
                cx,
                |s, _, cx| s.restart(cx),
            ));
        let labels: [SharedString; 4] = [
            tr(cx, "Draft model", "草稿模型"),
            tr(cx, "Small, fast", "小而快"),
            tr(cx, "Target model", "目标模型"),
            tr(cx, "Large", "大型"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, label)| self.model_labels[i].clone().unwrap_or_else(|| label.into()))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_default();
        let lane_names = div()
            .flex()
            .flex_none()
            .gap(px(if stacked_lanes { 20. } else { 8. }))
            .when(stacked_lanes, |d| d.w_full().min_h(px(38.)).flex_wrap())
            .when(!stacked_lanes, |d| d.w(px(120.)).flex_col())
            .child(lane_label(
                labels[0].clone(),
                labels[1].clone(),
                colors.draft,
                p,
            ))
            .child(lane_label(
                labels[2].clone(),
                labels[3].clone(),
                colors.target,
                p,
            ));
        let mut widths = Vec::new();
        if self.started
            && let Some(round) = round
        {
            let accepted = round.accepted();
            let columns = round
                .draft
                .len()
                .max(accepted + usize::from(round.target.is_some()));
            for i in 0..columns {
                let draft = round
                    .draft
                    .get(i)
                    .map_or(0., |s| measure(&s.trim_start().to_string().into(), 14., w));
                let target = if i == accepted {
                    round.target.as_ref().map_or(0., |s| {
                        measure(&s.trim_start().to_string().into(), 14., w) + 19.
                    })
                } else {
                    0.
                };
                widths.push((draft.max(target) + 20.).clamp(44., line_width));
            }
        }
        let columns = widths.len();
        let sources = Rc::new(RefCell::new(vec![Bounds::default(); columns * 2]));
        let destinations = Rc::new(RefCell::new(vec![
            Bounds::default();
            self.plan.tokens.len()
        ]));
        let desired = wrap_height(widths.iter().copied(), line_width, 6., 72., 8.);
        let lanes = self.lanes.get_or_insert_with(|| Slide::new(desired));
        lanes.duration(desired, 0.4, cx);
        let lane_height = lanes.get(w, cx).max(72.);
        let default_beat = Beat {
            start: 0.,
            drafted: 0.,
            swept: 0.01,
            end: 0.01,
            first_token: 0,
            token_count: 0,
        };
        let moment = Moment {
            time,
            round: active.unwrap_or(default_beat),
            reduced,
        };
        let lane_content = div()
            .relative()
            .min_w_0()
            .flex_1()
            .w_full()
            .h(px(lane_height))
            .overflow_hidden()
            .when(columns > 0, |d| {
                d.child(draft_columns(
                    round.expect("columns require an active round"),
                    moment,
                    &widths,
                    &sources,
                    colors,
                    radius,
                ))
            })
            .when(columns == 0, |d| {
                d.child(
                    column().gap(px(8.)).children(
                        [
                            if finished {
                                if tr(cx, "en", "zh") == "zh" {
                                    format!(
                                        "草拟 {} 个，接受 {} 个",
                                        totals.drafted, totals.accepted
                                    )
                                } else {
                                    format!(
                                        "{} tokens drafted, {} accepted",
                                        totals.drafted, totals.accepted
                                    )
                                }
                            } else {
                                tr(cx, "–", "–").to_owned()
                            },
                            if finished {
                                if tr(cx, "en", "zh") == "zh" {
                                    format!(
                                        "{} 次验证写入 {} 个 token",
                                        totals.passes, totals.tokens
                                    )
                                } else {
                                    format!("{} passes for {} tokens", totals.passes, totals.tokens)
                                }
                            } else {
                                tr(cx, "–", "–").to_owned()
                            },
                        ]
                        .into_iter()
                        .map(|s| {
                            div().h(px(32.)).flex().items_center().child(text(
                                s,
                                14.,
                                20.,
                                p.secondary,
                            ))
                        }),
                    ),
                )
            });
        let landed = if reduced {
            self.plan.tokens.partition_point(|t| t.start <= time)
        } else {
            self.plan
                .tokens
                .partition_point(|t| t.start + FLIGHT <= time + 0.001)
        };
        let output_name = if self.compare {
            tr(cx, "Speculative", "推测解码")
        } else {
            tr(cx, "Output", "输出")
        };
        let written = labelled_line(
            output_name,
            done,
            narrow,
            output_line(
                &self.plan,
                landed,
                moment,
                false,
                Some(&destinations),
                colors,
                cx,
            ),
            p,
            cx,
        );
        let compare_size = wrap_height(
            self.plan.words.iter().map(|word| {
                word.clone()
                    .map(|i| measure(&self.plan.tokens[i].text, 14., w))
                    .sum()
            }),
            line_width,
            0.,
            25.,
            0.,
        )
        .max(38.)
            + if narrow { 25. } else { 0. };
        self.compare_height
            .duration(if self.compare { compare_size + 12. } else { 0. }, 0.4, cx);
        let compare_height = self.compare_height.get(w, cx).max(0.);
        let output = column()
            .pt(px(if narrow { 16. } else { 20. }))
            .border_t_1()
            .border_color(p.line)
            .child(written)
            .when(compare_height > 0.1, |d| {
                d.child(
                    div()
                        .h(px(compare_height))
                        .pt(px(12.))
                        .overflow_hidden()
                        .child(labelled_line(
                            tr(cx, "Target only", "仅目标模型"),
                            baseline,
                            narrow,
                            output_line(&self.plan, baseline, moment, true, None, colors, cx),
                            p,
                            cx,
                        )),
                )
            });
        let knob_target = if self.compare { 1. } else { 0. };
        self.compare_knob.duration(knob_target, 0.3, cx);
        let knob = self.compare_knob.get(w, cx).clamp(0., 1.);
        let comparison = tr(cx, "Compare without speculation", "对照仅目标模型");
        let switch = activate(
            face(
                "decode-compare",
                comparison,
                44.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .role(Role::Switch)
            .aria_toggled(self.compare.into())
            .tab_stop(ready)
            .justify_start()
            .gap(px(12.))
            .child(
                div()
                    .relative()
                    .w(px(42.))
                    .h(px(24.))
                    .rounded_full()
                    .flex_none()
                    .bg(p.hover.blend(p.bright.opacity(knob)))
                    .child(
                        div()
                            .absolute()
                            .left(px(3. + 18. * knob))
                            .top(px(3.))
                            .size(px(18.))
                            .rounded_full()
                            .bg(Paint::for_mode(ThemeMode::Light).panel)
                            .shadow_sm(),
                    ),
            )
            .child(text(comparison, 14., 20., p.ink)),
            cx,
            |s, _, cx| {
                if s.status == DecodeStatus::Ready {
                    s.compare_clicked(cx);
                }
            },
        );
        let has_slider = self.tokens.is_some();
        let slider = has_slider.then(|| self.slider(ready, p, cx));
        let controls = div()
            .flex()
            .flex_col()
            .items_start()
            .w_full()
            .min_w_0()
            .gap(px(8.))
            .when_some(slider, |d, slider| d.child(slider.w_full()))
            .child(switch)
            .opacity(if ready { 1. } else { 0.5 });
        let stats = self.stats_row(totals, narrow, p, cx);
        let status_announcement = self.announcement.clone();
        let saved = self.bounds.clone();
        let outer = column()
            .relative()
            .w_full()
            .max_w(px(720.))
            .min_w_0()
            .p(px(pad))
            .gap(px(if narrow { 16. } else { 20. }))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .child(
                row()
                    .items_start()
                    .justify_between()
                    .gap(px(16.))
                    .child(
                        column()
                            .flex_1()
                            .gap(px(2.))
                            .child(
                                text(title.clone(), 16., 22., p.ink)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(status_view),
                    )
                    .child(actions),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_start()
                    .min_w_0()
                    .opacity(if ready { 1. } else { 0.5 })
                    .when(stacked_lanes, |d| d.flex_col().gap(px(12.)))
                    .when(!narrow, |d| d.gap(px(16.)))
                    .child(lane_names)
                    .child(lane_content),
            )
            .child(output)
            .child(
                column()
                    .pt(px(if narrow { 12. } else { 20. }))
                    .border_t_1()
                    .border_color(p.line)
                    .gap(px(16.))
                    .child(stats)
                    .child(controls),
            )
            .when(
                self.status == DecodeStatus::Failed && self.allow_retry,
                |d| {
                    d.child(activate(
                        face(
                            "decode-retry",
                            tr(cx, "Retry", "重试"),
                            32.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(10.))
                        .child(tr(cx, "Retry", "重试")),
                        cx,
                        |_, _, cx| cx.emit(DecodeRetry),
                    ))
                },
            )
            .when_some(
                active.filter(|a| !reduced && time >= a.swept && columns > 0),
                |d, active| {
                    d.child(token_flight(
                        active,
                        time,
                        self.plan.tokens
                            [active.first_token..active.first_token + active.token_count]
                            .to_vec(),
                        columns,
                        sources,
                        destinations,
                    ))
                },
            )
            .child(
                div()
                    .id("decode-announcement")
                    .role(Role::Status)
                    .aria_label(status_announcement)
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden(),
            )
            .child(
                canvas(
                    move |bounds, w, _| {
                        let old = saved.replace(bounds);
                        let visible = |b: Bounds<Pixels>| {
                            b.bottom() > px(0.) && b.top() < w.viewport_size().height
                        };
                        if old.size != bounds.size || visible(old) != visible(bounds) {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            );
        let outer = if let Some(style) = &self.style {
            style(outer)
        } else {
            outer
        };
        outer
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title)
    }
}

#[cfg(test)]
mod tests {
    use super::{DecodeRound, DecodeToken, Plan, default_tokens, simulate};
    use gpui::TestAppContext;

    #[test]
    fn recorded_rounds_preserve_leading_spaces_and_costs() {
        let rounds = vec![
            DecodeRound::new(
                vec!["The".into(), " build".into(), " failed".into()],
                2,
                Some(" passed".into()),
            ),
            DecodeRound::new(vec![], 0, Some(".".into())),
        ];
        let plan = Plan::new(&rounds, 0.64, 0.1);
        assert_eq!(plan.text(4), "The build passed.");
        assert_eq!(plan.words, vec![0..1, 1..2, 2..4]);
        let s = &plan.totals[2];
        assert_eq!((s.passes, s.drafted, s.accepted, s.tokens), (2, 3, 2, 4));
        assert!((s.speedup - 4. / 2.3).abs() < 0.0001);
    }

    #[test]
    fn simulation_rolls_back_after_first_miss_and_preserves_the_final_token() {
        let tokens = vec![
            DecodeToken::new("The").with_ease(1.),
            DecodeToken::new(" build").with_ease(0.).with_miss(" test"),
            DecodeToken::new(" passed").with_ease(1.),
            DecodeToken::new(".").with_ease(1.),
        ];
        let rounds = simulate(&tokens, 4, 12);
        assert_eq!(rounds[0].accepted(), 1);
        assert_eq!(rounds[0].draft[1], " test");
        assert_eq!(rounds[0].target.as_deref(), Some(" build"));
        assert_eq!(rounds[1].target.as_deref(), Some("."));
        assert_eq!(Plan::new(&rounds, 0.64, 0.1).text(4), "The build passed.");
    }

    #[test]
    fn sample_uses_the_reference_totals_and_can_replan() {
        let app = TestAppContext::single();
        app.update(|cx| {
            let tokens = default_tokens(cx);
            let rounds = simulate(&tokens, 4, 12);
            let plan = Plan::new(&rounds, 0.64, 0.1);
            let stats = plan.totals.last().unwrap();
            assert_eq!(
                (stats.passes, stats.drafted, stats.accepted, stats.tokens),
                (6, 24, 16, 22)
            );
            let short = Plan::new(&simulate(&tokens, 2, 12), 0.64, 0.1);
            assert!(short.end() > plan.end());
            assert_eq!(short.text(22), plan.text(22));
        });
    }
}
