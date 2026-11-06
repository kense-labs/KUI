//! Host-supplied prompt-cache outcomes with explicit, retryable requests.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
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
        let can_send = self.snapshot.requests.iter().any(|r| r.result.is_none());
        let progress = self.highlight.get(w, cx).clamp(0., 1.);
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
                            let bars = row()
                                .w_full()
                                .h(px(10.))
                                .gap(px(2.))
                                .children(self.segments.iter().map(|segment| {
                                    div()
                                        .h_full()
                                        .w(relative((segment.tokens as f32).sqrt() / total * 0.85))
                                        .rounded(px(3.))
                                        .bg(segment.color.opacity(if result.is_some() {
                                            0.45 + progress * 0.4
                                        } else {
                                            0.18
                                        }))
                                }))
                                .child(
                                    div().h_full().flex_1().rounded(px(3.)).bg(p
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
        column()
            .w_full()
            .max_w(px(760.))
            .gap(px(20.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .items_center()
                    .justify_between()
                    .gap_2()
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
            .child(legend)
            .child(requests)
            .child(
                row()
                    .gap_3()
                    .flex_wrap()
                    .child(hint(
                        format!(
                            "{} · {} {}",
                            state,
                            self.snapshot.cached_tokens,
                            tr(cx, "tokens cached", "个 token 已缓存")
                        ),
                        p,
                    ))
                    .when_some(self.snapshot.expires_in_seconds, |v, s| {
                        v.child(hint(
                            format!("{} {s}s", tr(cx, "Time remaining", "剩余")),
                            p,
                        ))
                    }),
            )
            .child(
                row()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        face(
                            "cache-next",
                            tr(cx, "Next request", "下一次请求"),
                            36.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Primary,
                            cx,
                        )
                        .px(px(10.))
                        .when(!ready || !can_send, |v| v.opacity(0.45))
                        .child(tr(cx, "Next request", "下一次请求"))
                        .on_click(cx.listener(|s, _, _, cx| s.request_next(cx))),
                    )
                    .when_some(change, |v, label| {
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
                            .on_click(
                                cx.listener(|s, _, _, cx| s.set_change_armed(!s.change_armed, cx)),
                            ),
                        )
                    }),
            )
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
