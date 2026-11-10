//! Host-supplied prompt-cache outcomes with explicit, retryable requests.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    glyph::{Glyph, icon},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
#[derive(Clone, Debug)]
pub struct CacheSegment {
    pub id: SharedString,
    pub label: SharedString,
    pub tokens: u64,
    pub color: Hsla,
}
impl CacheSegment {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        tokens: u64,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tokens,
            color,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheOutcome {
    Write,
    Hit,
    Partial,
    Miss,
}
#[derive(Clone, Debug)]
pub struct CacheResult {
    pub outcome: CacheOutcome,
    pub cached_tokens: u64,
    pub processed_tokens: u64,
    pub written_tokens: u64,
    pub cost: Option<f64>,
    pub first_token_ms: Option<u64>,
}
impl CacheResult {
    pub fn new(
        outcome: CacheOutcome,
        cached_tokens: u64,
        processed_tokens: u64,
        written_tokens: u64,
    ) -> Self {
        Self {
            outcome,
            cached_tokens,
            processed_tokens,
            written_tokens,
            cost: None,
            first_token_ms: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CacheRequest {
    pub id: SharedString,
    pub label: SharedString,
    pub message_tokens: u64,
    pub result: Option<CacheResult>,
}
impl CacheRequest {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        message_tokens: u64,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            message_tokens,
            result: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheState {
    Empty,
    Available,
    Expired,
}
#[derive(Clone, Debug)]
pub struct CacheSnapshot {
    pub requests: Vec<CacheRequest>,
    pub state: CacheState,
    pub cached_tokens: u64,
    pub expires_in_seconds: Option<u64>,
}
impl CacheSnapshot {
    pub fn new(requests: Vec<CacheRequest>) -> Self {
        Self {
            requests,
            state: CacheState::Empty,
            cached_tokens: 0,
            expires_in_seconds: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct PromptCacheAction {
    pub id: u64,
    pub request: SharedString,
    pub change_segment: Option<SharedString>,
}
pub struct PromptCache {
    segments: Vec<CacheSegment>,
    snapshot: CacheSnapshot,
    simulated: bool,
    change_segment: Option<SharedString>,
    change_armed: bool,
    next_id: u64,
    pending: Option<PromptCacheAction>,
    failure: Option<SharedString>,
    highlight: Slide,
    sample: Option<CacheSnapshot>,
    playing: bool,
    ticks: u16,
}
impl EventEmitter<PromptCacheAction> for PromptCache {}
impl PromptCache {
    pub fn new(segments: Vec<CacheSegment>, snapshot: CacheSnapshot) -> Self {
        Self {
            segments,
            snapshot,
            simulated: true,
            change_segment: None,
            change_armed: false,
            next_id: 0,
            pending: None,
            failure: None,
            highlight: Slide::new(0.),
            sample: None,
            playing: false,
            ticks: 0,
        }
    }
    /// Enables replay and automatic requests for a local simulation.
    pub fn with_sample_playback(mut self, autoplay: bool) -> Self {
        self.sample = Some(self.snapshot.clone());
        self.playing = autoplay;
        self
    }
    /// Advance the demo by one visible 250ms tick. Does not change host-only snapshots.
    pub fn advance_sample(&mut self, cx: &mut Context<Self>) {
        if self.sample.is_none()
            || !self.playing
            || self.pending.is_some()
            || self.failure.is_some()
        {
            return;
        }
        self.ticks = self.ticks.saturating_add(1);
        if self.snapshot.state == CacheState::Available
            && self.ticks.is_multiple_of(4)
            && let Some(ttl) = self.snapshot.expires_in_seconds.as_mut()
        {
            *ttl = ttl.saturating_sub(3);
            if *ttl == 0 {
                self.snapshot.state = CacheState::Expired;
                self.snapshot.cached_tokens = 0;
            }
            cx.notify();
        }
        if self.ticks >= 7 && self.snapshot.requests.iter().any(|r| r.result.is_none()) {
            self.ticks = 0;
            self.request_next(cx);
        }
        if self.snapshot.state == CacheState::Expired
            && self.snapshot.requests.iter().all(|r| r.result.is_some())
        {
            self.playing = false;
            cx.notify();
        }
    }
    fn rewind(&mut self, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.sample.clone() {
            self.pending = None;
            self.failure = None;
            self.change_armed = false;
            self.playing = false;
            self.ticks = 0;
            self.snapshot = snapshot;
            self.highlight.reset(0.);
            cx.notify();
        }
    }
    pub fn with_recorded(mut self, recorded: bool) -> Self {
        self.simulated = !recorded;
        self
    }
    pub fn with_change_segment(mut self, id: impl Into<SharedString>) -> Self {
        self.change_segment = Some(id.into());
        self
    }
    pub fn set_snapshot(&mut self, snapshot: CacheSnapshot, cx: &mut Context<Self>) {
        self.snapshot = snapshot;
        self.highlight.reset(0.);
        self.highlight.duration(1., 0.52, cx);
        cx.notify();
    }
    pub fn set_segments(&mut self, segments: Vec<CacheSegment>, cx: &mut Context<Self>) {
        self.segments = segments;
        if self
            .change_segment
            .as_ref()
            .is_some_and(|id| !self.segments.iter().any(|s| &s.id == id))
        {
            self.change_armed = false;
        }
        cx.notify();
    }
    pub fn set_change_armed(&mut self, armed: bool, cx: &mut Context<Self>) {
        if self.pending.is_none()
            && self
                .change_segment
                .as_ref()
                .is_some_and(|id| self.segments.iter().any(|s| &s.id == id))
            && self.change_armed != armed
        {
            self.change_armed = armed;
            cx.notify();
        }
    }
    pub fn request_next(&mut self, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            return;
        }
        let Some(request) = self.snapshot.requests.iter().find(|r| r.result.is_none()) else {
            return;
        };
        self.next_id = self.next_id.wrapping_add(1);
        let action = PromptCacheAction {
            id: self.next_id,
            request: request.id.clone(),
            change_segment: if self.change_armed {
                self.change_segment.clone()
            } else {
                None
            },
        };
        self.failure = None;
        self.pending = Some(action.clone());
        cx.emit(action);
        cx.notify();
    }
    pub fn resolve(
        &mut self,
        id: u64,
        result: Result<(CacheResult, CacheState, u64, Option<u64>), SharedString>,
        cx: &mut Context<Self>,
    ) {
        let Some(action) = self.pending.as_ref().filter(|r| r.id == id).cloned() else {
            return;
        };
        self.pending = None;
        match result {
            Ok((result, state, cached, expires)) => {
                if let Some(request) = self
                    .snapshot
                    .requests
                    .iter_mut()
                    .find(|r| r.id == action.request)
                {
                    request.result = Some(result);
                }
                self.snapshot.state = state;
                self.snapshot.cached_tokens = cached;
                self.snapshot.expires_in_seconds = expires;
                self.change_armed = false;
                self.highlight.reset(0.);
                self.highlight.duration(1., 0.52, cx);
            }
            Err(error) => self.failure = Some(error),
        }
        cx.notify();
    }
}
fn outcome(value: CacheOutcome, cx: &App) -> &'static str {
    match value {
        CacheOutcome::Write => tr(cx, "Cache write", "写入缓存"),
        CacheOutcome::Hit => tr(cx, "Cache hit", "命中缓存"),
        CacheOutcome::Partial => tr(cx, "Partial hit", "部分命中"),
        CacheOutcome::Miss => tr(cx, "Cache miss", "未命中"),
    }
}
impl Render for PromptCache {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let ready = self.pending.is_none();
        let progress = self.highlight.get(w, cx).clamp(0., 1.);
        let compact = w.viewport_size().width < px(540.);
        let side_by_side = w.viewport_size().width >= px(730.);
        let total: f32 = self
            .segments
            .iter()
            .map(|s| (s.tokens as f32).sqrt())
            .sum::<f32>()
            .max(1.);
        let state = match self.snapshot.state {
            CacheState::Empty => tr(cx, "Empty", "空"),
            CacheState::Available => tr(cx, "Available", "可用"),
            CacheState::Expired => tr(cx, "Expired", "已过期"),
        };
        let legend = row()
            .flex_wrap()
            .gap(px(16.))
            .children(self.segments.iter().map(|s| {
                row()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(10.)).rounded(px(3.)).bg(s.color))
                    .child(hint(format!("{} {}", s.label, s.tokens), p))
            }));
        let requests =
            column()
                .gap(px(16.))
                .children(
                    self.snapshot
                        .requests
                        .iter()
                        .enumerate()
                        .map(|(i, request)| {
                            let result = request.result.as_ref();
                            let summary = result.map_or_else(
                                || tr(cx, "Not sent", "尚未发送").to_string(),
                                |r| {
                                    format!(
                                        "{} · {} {} · {} {}",
                                        outcome(r.outcome, cx),
                                        r.cached_tokens,
                                        tr(cx, "cached", "缓存"),
                                        r.processed_tokens,
                                        tr(cx, "processed", "处理")
                                    )
                                },
                            );
                            let cost = result
                                .and_then(|r| r.cost)
                                .filter(|v| v.is_finite() && *v >= 0.)
                                .map_or_else(|| "—".into(), |v| format!("${v:.4}"));
                            let latency = result
                                .and_then(|r| r.first_token_ms)
                                .map_or_else(|| "—".into(), |v| format!("{v} ms"));
                            let mut cached = result.map_or(0, |r| r.cached_tokens);
                            let mut bars = row().w_full().h(px(12.)).gap(px(2.));
                            for segment in &self.segments {
                                let hit = cached.min(segment.tokens);
                                cached -= hit;
                                let fraction = hit as f32 / segment.tokens.max(1) as f32;
                                bars = bars.child(
                                    div()
                                        .h_full()
                                        .w(relative((segment.tokens as f32).sqrt() / total * 0.85))
                                        .rounded(px(4.))
                                        .overflow_hidden()
                                        .bg(segment.color.opacity(if result.is_some() {
                                            0.53 + progress * 0.25
                                        } else {
                                            0.18
                                        }))
                                        .when(fraction > 0., |v| {
                                            v.child(
                                                div()
                                                    .h_full()
                                                    .w(relative(fraction))
                                                    .bg(p.panel)
                                                    .border_1()
                                                    .border_color(segment.color.opacity(0.80))
                                                    .rounded(px(4.)),
                                            )
                                        }),
                                );
                            }
                            let bars = bars.child(
                                div().h_full().flex_1().rounded(px(4.)).bg(p
                                    .secondary
                                    .opacity(if result.is_some() { 0.65 } else { 0.2 })),
                            );
                            column()
                                .id(("prompt-cache-row", i))
                                .role(Role::ListItem)
                                .aria_label(format!(
                                    "{} · {} · {} · {}",
                                    request.label, summary, cost, latency
                                ))
                                .gap(px(6.))
                                .child(
                                    row()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(request.label.clone())
                                        .child(hint(summary, p)),
                                )
                                .child(bars)
                                .child(
                                    row()
                                        .justify_between()
                                        .gap_2()
                                        .child(hint(
                                            format!("{} {}", tr(cx, "Cost", "费用"), cost),
                                            p,
                                        ))
                                        .child(hint(
                                            format!(
                                                "{} {}",
                                                tr(cx, "First token", "首 token"),
                                                latency
                                            ),
                                            p,
                                        )),
                                )
                        }),
                );
        let change = self
            .change_segment
            .as_ref()
            .and_then(|id| self.segments.iter().find(|s| &s.id == id))
            .map(|s| s.label.clone());
        let filled = self
            .snapshot
            .expires_in_seconds
            .map_or(0., |s| (s as f32 / 300.).clamp(0., 1.));
        let totals = self
            .snapshot
            .requests
            .iter()
            .filter_map(|r| r.result.as_ref())
            .fold(
                (0_u64, 0_u64, 0_f64, 0_usize),
                |(cached, processed, cost, count), result| {
                    (
                        cached.saturating_add(result.cached_tokens),
                        processed.saturating_add(result.processed_tokens),
                        cost + result
                            .cost
                            .filter(|n| n.is_finite() && *n >= 0.)
                            .unwrap_or(0.),
                        count + 1,
                    )
                },
            );
        let cache = column()
            .flex_none()
            .w_full()
            .gap_2()
            .p_3()
            .when(side_by_side, |v| v.w(px(155.)))
            .border_1()
            .border_color(p.line)
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.stage)
            .child(
                row()
                    .justify_between()
                    .child(title(tr(cx, "Cache", "缓存"), p))
                    .child(hint(state, p)),
            )
            .child(hint(
                format!(
                    "{} {}",
                    self.snapshot.cached_tokens,
                    tr(cx, "tokens cached", "个 token 已缓存")
                ),
                p,
            ))
            .when_some(self.snapshot.expires_in_seconds, |v, time| {
                v.child(hint(
                    format!(
                        "{} {}:{:02}",
                        tr(cx, "Expires in", "有效期剩余"),
                        time / 60,
                        time % 60
                    ),
                    p,
                ))
                .child(
                    div().w_full().h(px(3.)).rounded_full().bg(p.line).child(
                        div()
                            .h_full()
                            .w(relative(filled))
                            .rounded_full()
                            .bg(p.bright),
                    ),
                )
            });
        let playback = self.sample.is_some();
        let play = if self.playing {
            tr(cx, "Pause", "暂停")
        } else {
            tr(cx, "Play", "播放")
        };
        let controls = row().flex_none().gap_1().children(
            [
                (
                    0,
                    play,
                    if self.playing {
                        Glyph::Pause
                    } else {
                        Glyph::Play
                    },
                ),
                (1, tr(cx, "Next request", "下一次请求"), Glyph::Arrow),
                (2, tr(cx, "Reset", "重置"), Glyph::Undo),
            ]
            .into_iter()
            .filter(|(i, _, _)| playback || *i == 1)
            .map(|(i, label, glyph)| {
                face(
                    ("cache-control", i as usize),
                    label,
                    36.,
                    crate::foundation::surface_radius(cx).into(),
                    Tone::Surface,
                    cx,
                )
                .size(px(36.))
                .child(icon(glyph, 16., p.ink))
                .on_click(cx.listener(move |s, _, _, cx| match i {
                    0 => {
                        s.playing = !s.playing;
                        cx.notify();
                    }
                    1 => {
                        s.playing = false;
                        s.request_next(cx);
                        cx.notify();
                    }
                    _ => s.rewind(cx),
                }))
            }),
        );
        column()
            .w_full()
            .max_w(px(760.))
            .gap(px(20.))
            .p(px(if compact { 16. } else { 24. }))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .justify_between()
                    .flex_wrap()
                    .child(
                        column()
                            .gap_1()
                            .child(title(tr(cx, "Prompt cache", "提示缓存"), p))
                            .child(hint(
                                if self.simulated {
                                    tr(cx, "Illustrative data", "示意数据")
                                } else {
                                    tr(cx, "Host data", "宿主数据")
                                },
                                p,
                            )),
                    )
                    .child(controls),
            )
            .child(legend)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .when(!side_by_side, |v| v.flex_col())
                    .child(cache)
                    .child(requests.flex_1().w_full().min_w_0()),
            )
            .child(row().gap_2().flex_wrap().when_some(change, |v, label| {
                v.child(
                    face(
                        "cache-change",
                        format!("{} {label}", tr(cx, "Change", "修改")),
                        36.,
                        crate::foundation::surface_radius(cx).into(),
                        if self.change_armed {
                            Tone::Primary
                        } else {
                            Tone::Surface
                        },
                        cx,
                    )
                    .px(px(10.))
                    .aria_toggled(self.change_armed.into())
                    .when(!ready, |v| v.opacity(0.45))
                    .child(format!("{} {label}", tr(cx, "Change", "修改")))
                    .on_click(cx.listener(|s, _, _, cx| s.set_change_armed(!s.change_armed, cx))),
                )
            }))
            .when(totals.3 > 0, |v| {
                v.child(
                    row()
                        .flex_wrap()
                        .justify_between()
                        .gap_2()
                        .pt_3()
                        .border_t_1()
                        .border_color(p.line)
                        .child(hint(
                            format!("{} {}", tr(cx, "From cache", "来自缓存"), totals.0),
                            p,
                        ))
                        .child(hint(
                            format!("{} {}", tr(cx, "Processed", "已处理"), totals.1),
                            p,
                        ))
                        .when(self.simulated, |v| {
                            v.child(hint(
                                format!("{} ${:.4}", tr(cx, "Sample cost", "样例成本"), totals.2),
                                p,
                            ))
                        }),
                )
            })
            .when(!ready, |v| {
                v.child(hint(
                    tr(cx, "Waiting for host confirmation…", "等待宿主确认…"),
                    p,
                ))
            })
            .when_some(self.failure.clone(), |v, error| {
                v.child(
                    row()
                        .gap_2()
                        .items_center()
                        .child(div().text_color(p.danger).child(error))
                        .child(
                            face(
                                "cache-retry",
                                tr(cx, "Retry", "重试"),
                                36.,
                                crate::foundation::surface_radius(cx).into(),
                                Tone::Surface,
                                cx,
                            )
                            .px(px(10.))
                            .child(tr(cx, "Retry", "重试"))
                            .on_click(cx.listener(|s, _, _, cx| s.request_next(cx))),
                        ),
                )
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{CacheOutcome, CacheRequest, CacheResult, CacheSnapshot, CacheState};
    #[test]
    fn host_values_remain_unchanged() {
        let mut r = CacheRequest::new("one", "Request", 100);
        r.result = Some(CacheResult::new(CacheOutcome::Partial, 30, 70, 20));
        let s = CacheSnapshot::new(vec![r]);
        assert_eq!(s.requests[0].result.as_ref().unwrap().cached_tokens, 30);
        assert_eq!(s.state, CacheState::Empty);
    }
}
