//! A host-controlled fine-tuning monitor. It never starts or deploys jobs itself.
use crate::primitives::{
    family::{face, Paint, Tone},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::time::Duration;
use web_time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingRunStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug)]
pub struct TrainingLoss {
    pub step: u64,
    pub train: Option<f32>,
    pub validation: Option<f32>,
}
impl TrainingLoss {
    pub fn new(step: u64, train: Option<f32>, validation: Option<f32>) -> Self {
        Self {
            step,
            train,
            validation,
        }
    }
}
#[derive(Clone, Debug)]
pub struct TrainingEvent {
    pub id: SharedString,
    pub label: SharedString,
    pub step: Option<u64>,
}
impl TrainingEvent {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            step: None,
        }
    }
    pub fn with_step(mut self, step: u64) -> Self {
        self.step = Some(step);
        self
    }
}
#[derive(Clone, Debug)]
pub struct TrainingRunSnapshot {
    pub status: TrainingRunStatus,
    pub name: SharedString,
    pub model: SharedString,
    pub details: SharedString,
    pub dataset: SharedString,
    pub step: u64,
    pub total_steps: u64,
    pub epochs: u32,
    pub loss: Vec<TrainingLoss>,
    pub learning_rate: Option<f64>,
    pub elapsed_seconds: u64,
    pub eta_seconds: Option<u64>,
    pub events: Vec<TrainingEvent>,
    pub error: Option<SharedString>,
    pub endpoint: Option<SharedString>,
}
impl TrainingRunSnapshot {
    pub fn new(
        name: impl Into<SharedString>,
        model: impl Into<SharedString>,
        total_steps: u64,
        epochs: u32,
    ) -> Self {
        Self {
            status: TrainingRunStatus::Queued,
            name: name.into(),
            model: model.into(),
            details: "".into(),
            dataset: "".into(),
            step: 0,
            total_steps: total_steps.max(1),
            epochs: epochs.max(1),
            loss: Vec::new(),
            learning_rate: None,
            elapsed_seconds: 0,
            eta_seconds: None,
            events: Vec::new(),
            error: None,
            endpoint: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainingRunAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    Restart,
    Deploy,
}
#[derive(Clone, Debug)]
pub struct TrainingRunRequest {
    pub id: u64,
    pub action: TrainingRunAction,
}
pub struct TrainingRun {
    snapshot: TrainingRunSnapshot,
    allowed: Vec<TrainingRunAction>,
    pending: Option<TrainingRunRequest>,
    request_id: u64,
    cancel_until: Option<Instant>,
    failure: Option<SharedString>,
    inspected: Option<usize>,
    ceiling: f32,
    simulated: bool,
}
impl EventEmitter<TrainingRunRequest> for TrainingRun {}
impl TrainingRun {
    pub fn new(snapshot: TrainingRunSnapshot) -> Self {
        Self {
            snapshot,
            allowed: Vec::new(),
            pending: None,
            request_id: 0,
            cancel_until: None,
            failure: None,
            inspected: None,
            ceiling: 2.,
            simulated: false,
        }
    }
    pub fn with_actions(mut self, actions: Vec<TrainingRunAction>) -> Self {
        self.allowed = actions;
        self
    }
    pub fn with_loss_ceiling(mut self, value: f32) -> Self {
        if value.is_finite() && value > 0. {
            self.ceiling = value;
        }
        self
    }
    pub fn with_simulated(mut self, simulated: bool) -> Self {
        self.simulated = simulated;
        self
    }
    pub fn set_snapshot(&mut self, snapshot: TrainingRunSnapshot, cx: &mut Context<Self>) {
        if self.snapshot.status != snapshot.status {
            self.cancel_until = None;
            // A newer host status supersedes any outstanding action response.
            self.pending = None;
        }
        self.inspected = self.inspected.filter(|i| *i < snapshot.loss.len());
        self.snapshot = snapshot;
        cx.notify();
    }
    pub fn resolve(
        &mut self,
        id: u64,
        result: Result<TrainingRunSnapshot, SharedString>,
        cx: &mut Context<Self>,
    ) {
        if !self
            .pending
            .as_ref()
            .is_some_and(|request| request.id == id)
        {
            return;
        }
        self.pending = None;
        match result {
            Ok(snapshot) => {
                self.failure = None;
                self.set_snapshot(snapshot, cx);
            }
            Err(error) => {
                self.failure = Some(error);
                cx.notify();
            }
        }
    }
    fn request(&mut self, action: TrainingRunAction, cx: &mut Context<Self>) {
        if self.pending.is_some()
            || !self.allowed.contains(&action)
            || !self.action_available(action)
        {
            return;
        }
        if action == TrainingRunAction::Cancel {
            let now = cx.background_executor().now();
            if !self.cancel_until.is_some_and(|deadline| now < deadline) {
                self.cancel_until = Some(now + Duration::from_secs(3));
                cx.spawn(async move |view, cx| {
                    cx.background_executor().timer(Duration::from_secs(3)).await;
                    let _ = view.update(cx, |this, cx| {
                        if this
                            .cancel_until
                            .is_some_and(|deadline| cx.background_executor().now() >= deadline)
                        {
                            this.cancel_until = None;
                            cx.notify();
                        }
                    });
                })
                .detach();
                cx.notify();
                return;
            }
            self.cancel_until = None;
        }
        self.request_id = self.request_id.wrapping_add(1);
        let request = TrainingRunRequest {
            id: self.request_id,
            action,
        };
        self.pending = Some(request.clone());
        self.failure = None;
        cx.emit(request);
        cx.notify();
    }
    fn action_available(&self, action: TrainingRunAction) -> bool {
        use TrainingRunAction as A;
        use TrainingRunStatus as S;
        matches!(
            (action, self.snapshot.status),
            (A::Pause, S::Running)
                | (A::Resume, S::Paused)
                | (A::Cancel, S::Queued | S::Running | S::Paused)
                | (A::Retry, S::Failed)
                | (A::Restart, S::Failed | S::Completed | S::Cancelled)
                | (A::Deploy, S::Completed)
        )
    }
    fn inspect(&mut self, key: &str, shift: bool, cx: &mut Context<Self>) {
        let n = self.snapshot.loss.len();
        if n == 0 {
            return;
        }
        let current = self.inspected.unwrap_or(n - 1);
        self.inspected = match key {
            "escape" => None,
            "home" => Some(0),
            "end" => Some(n - 1),
            "left" => Some(current.saturating_sub(if shift { 10 } else { 1 })),
            "right" => Some(
                current
                    .saturating_add(if shift { 10 } else { 1 })
                    .min(n - 1),
            ),
            _ => return,
        };
        cx.notify();
        cx.stop_propagation();
    }
}
fn status(value: TrainingRunStatus, cx: &App) -> &'static str {
    match value {
        TrainingRunStatus::Queued => tr(cx, "Queued", "排队中"),
        TrainingRunStatus::Running => tr(cx, "Training", "训练中"),
        TrainingRunStatus::Paused => tr(cx, "Paused", "已暂停"),
        TrainingRunStatus::Completed => tr(cx, "Completed", "已完成"),
        TrainingRunStatus::Failed => tr(cx, "Failed", "失败"),
        TrainingRunStatus::Cancelled => tr(cx, "Cancelled", "已取消"),
    }
}
fn duration(s: u64) -> String {
    if s >= 60 {
        format!("{}m {}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}
fn latest(value: Option<f32>) -> String {
    value
        .filter(|v| v.is_finite())
        .map_or_else(|| "—".to_string(), |v| format!("{v:.3}"))
}
fn draw_line(
    samples: &[TrainingLoss],
    total: u64,
    ceiling: f32,
    val: bool,
    color: Hsla,
    b: Bounds<Pixels>,
    w: &mut Window,
) {
    let stride = samples.len().div_ceil(500).max(1);
    let mut path = PathBuilder::stroke(px(2.));
    let mut started = false;
    for (i, sample) in samples.iter().enumerate() {
        if i % stride != 0 && i + 1 != samples.len() {
            continue;
        }
        let y = if val { sample.validation } else { sample.train };
        let Some(y) = y.filter(|v| v.is_finite()) else {
            continue;
        };
        let x = sample.step.min(total) as f32 / total.max(1) as f32;
        let point = point(
            b.origin.x + b.size.width * x,
            b.origin.y + b.size.height * (1. - (y / ceiling).clamp(0., 1.)),
        );
        if started {
            path.line_to(point);
        } else {
            path.move_to(point);
            started = true;
        }
    }
    if started && let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
impl Render for TrainingRun {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let snapshot = &self.snapshot;
        let selected = self
            .inspected
            .and_then(|i| snapshot.loss.get(i))
            .or_else(|| snapshot.loss.last());
        let current_train = selected.and_then(|s| s.train);
        let current_val = selected.and_then(|s| s.validation);
        let step = selected.map_or(snapshot.step, |s| s.step);
        let samples = snapshot.loss.clone();
        let total = snapshot.total_steps.max(1);
        let ceiling = self.ceiling;
        let chart_label = format!(
            "{} {} / {}, {} {}, {} {}",
            tr(cx, "Step", "步骤"),
            step,
            total,
            tr(cx, "Train", "训练"),
            latest(current_train),
            tr(cx, "Validation", "验证"),
            latest(current_val)
        );
        let graph = div()
            .id("training-loss-graph")
            .role(Role::Group)
            .tab_index(0)
            .aria_label(chart_label)
            .w_full()
            .h(px(156.))
            .relative()
            .bg(p.stage)
            .rounded(crate::foundation::surface_radius(cx))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                s.inspect(e.keystroke.key.as_str(), e.keystroke.modifiers.shift, cx);
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, w, _| {
                        draw_line(&samples, total, ceiling, false, p.bright, b, w);
                        draw_line(&samples, total, ceiling, true, p.success, b, w);
                    },
                )
                .absolute()
                .inset_0(),
            );
        let progress = (snapshot.step.min(total) as f32 / total as f32).clamp(0., 1.);
        let epoch_count = snapshot.epochs.clamp(1, 20);
        let mut epochs = row().w_full().gap(px(4.));
        for index in 0..epoch_count {
            let part = (progress * epoch_count as f32 - index as f32).clamp(0., 1.);
            epochs = epochs.child(
                div()
                    .flex_1()
                    .h(px(4.))
                    .rounded(px(2.))
                    .bg(p.line)
                    .child(div().h_full().w(relative(part)).bg(p.bright)),
            );
        }
        let stats = row()
            .w_full()
            .gap(px(16.))
            .flex_wrap()
            .child(
                column()
                    .gap(px(2.))
                    .child(hint(tr(cx, "Epoch", "轮次"), p))
                    .child(format!(
                        "{} / {}",
                        (progress * snapshot.epochs.max(1) as f32).ceil() as u32,
                        snapshot.epochs
                    )),
            )
            .child(
                column()
                    .gap(px(2.))
                    .child(hint(tr(cx, "Learning rate", "学习率"), p))
                    .child(
                        snapshot
                            .learning_rate
                            .filter(|v| v.is_finite())
                            .map_or_else(|| "—".into(), |v| format!("{v:.2e}")),
                    ),
            )
            .child(
                column()
                    .gap(px(2.))
                    .child(hint(tr(cx, "Elapsed", "已用时间"), p))
                    .child(duration(snapshot.elapsed_seconds)),
            )
            .child(
                column()
                    .gap(px(2.))
                    .child(hint(tr(cx, "ETA", "预计剩余"), p))
                    .child(snapshot.eta_seconds.map_or_else(|| "—".into(), duration)),
            );
        let mut actions = row().gap_2().flex_wrap();
        for action in [
            TrainingRunAction::Pause,
            TrainingRunAction::Resume,
            TrainingRunAction::Cancel,
            TrainingRunAction::Retry,
            TrainingRunAction::Restart,
            TrainingRunAction::Deploy,
        ] {
            if !self.allowed.contains(&action) || !self.action_available(action) {
                continue;
            }
            let armed = self
                .cancel_until
                .is_some_and(|deadline| cx.background_executor().now() < deadline);
            let label = match action {
                TrainingRunAction::Pause => tr(cx, "Pause", "暂停"),
                TrainingRunAction::Resume => tr(cx, "Resume", "继续"),
                TrainingRunAction::Cancel if armed => tr(cx, "Confirm cancel", "确认取消"),
                TrainingRunAction::Cancel => tr(cx, "Cancel", "取消"),
                TrainingRunAction::Retry => tr(cx, "Retry", "重试"),
                TrainingRunAction::Restart => tr(cx, "Run again", "重新运行"),
                TrainingRunAction::Deploy => tr(cx, "Deploy", "部署"),
            };
            let tone = match action {
                TrainingRunAction::Cancel if armed => Tone::Danger,
                TrainingRunAction::Resume
                | TrainingRunAction::Retry
                | TrainingRunAction::Deploy => Tone::Primary,
                _ => Tone::Surface,
            };
            actions = actions.child(
                face(
                    ("training-action", action as usize),
                    label,
                    32.,
                    8.,
                    tone,
                    cx,
                )
                .px(px(10.))
                .when(self.pending.is_some(), |v| v.opacity(0.5))
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| s.request(action, cx)))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.request(action, cx);
                        cx.stop_propagation();
                    }
                })),
            );
        }
        let mut events = column().h(px(96.)).gap(px(0.));
        for event in snapshot.events.iter().rev().take(3) {
            events = events.child(
                div()
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .child(hint(
                        format!(
                            "{}{}",
                            event.label,
                            event.step.map_or_else(String::new, |v| format!(" · {v}"))
                        ),
                        p,
                    )),
            );
        }
        column()
            .w_full()
            .max_w(px(640.))
            .gap(px(16.))
            .pt(px(20.))
            .px(px(20.))
            .pb(px(16.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .h(px(40.))
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        column()
                            .gap(px(2.))
                            .child(title(snapshot.name.clone(), p))
                            .child(hint(
                                format!("{} · {}", snapshot.model, snapshot.details),
                                p,
                            )),
                    )
                    .child(hint(status(snapshot.status, cx), p)),
            )
            .child(
                row()
                    .justify_between()
                    .flex_wrap()
                    .gap_2()
                    .child(hint(
                        format!("{} {}", tr(cx, "Train", "训练"), latest(current_train)),
                        p,
                    ))
                    .child(hint(
                        format!("{} {}", tr(cx, "Validation", "验证"), latest(current_val)),
                        p,
                    ))
                    .child(hint(
                        format!("{} {step} / {total}", tr(cx, "Step", "步骤")),
                        p,
                    )),
            )
            .child(graph)
            .child(epochs)
            .child(stats)
            .child(events)
            .when(!snapshot.dataset.is_empty(), |v| {
                v.child(hint(snapshot.dataset.clone(), p))
            })
            .when_some(snapshot.error.clone(), |v, error| {
                v.child(div().text_color(p.danger).child(error))
            })
            .when_some(snapshot.endpoint.clone(), |v, endpoint| {
                v.child(hint(
                    format!("{} {endpoint}", tr(cx, "Endpoint", "端点")),
                    p,
                ))
            })
            .when_some(self.failure.clone(), |v, error| {
                v.child(div().text_color(p.danger).child(error))
            })
            .when(self.pending.is_some(), |v| {
                v.child(hint(
                    tr(cx, "Waiting for host confirmation…", "等待宿主确认…"),
                    p,
                ))
            })
            .child(actions)
            .when(self.simulated, |v| {
                v.child(hint(tr(cx, "Simulated training data", "模拟训练数据"), p))
            })
    }
}
