//! Locally simulated training: no model is trained and no deployment is made.
use gpui::{prelude::*, *};
use kui::{
    components::training_run::{
        TrainingEvent, TrainingLoss, TrainingRun, TrainingRunAction, TrainingRunRequest,
        TrainingRunSnapshot, TrainingRunStatus,
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
pub const IDS: &[&str] = &["training-run"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    monitor: Entity<TrainingRun>,
    snapshot: Rc<RefCell<TrainingRunSnapshot>>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let mut initial = TrainingRunSnapshot::new(
            t("Support classifier", "支持请求分类器"),
            t("Sample model", "样例模型"),
            1200,
            3,
        );
        initial.status = TrainingRunStatus::Running;
        initial.details = "LoRA · 2 GPUs".into();
        initial.dataset = t("Sample dataset · 12,000 examples", "样例数据集 · 12,000 条").into();
        initial.learning_rate = Some(0.0002);
        initial.eta_seconds = Some(60);
        initial.events.push(TrainingEvent::new(
            "start",
            t("Simulation started", "模拟运行已开始"),
        ));
        let state = Rc::new(RefCell::new(initial));
        let actions = vec![
            TrainingRunAction::Pause,
            TrainingRunAction::Resume,
            TrainingRunAction::Cancel,
            TrainingRunAction::Retry,
            TrainingRunAction::Restart,
            TrainingRunAction::Deploy,
        ];
        let monitor = cx.new(|_| {
            TrainingRun::new(state.borrow().clone())
                .with_actions(actions)
                .with_simulated(true)
        });
        let weak = monitor.downgrade();
        let clock_state = state.clone();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(600))
                    .await;
                if weak
                    .update(cx, |monitor, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        if !visible {
                            return;
                        }
                        let mut snapshot = clock_state.borrow_mut();
                        if snapshot.status != TrainingRunStatus::Running {
                            return;
                        }
                        snapshot.step = (snapshot.step + 40).min(snapshot.total_steps);
                        snapshot.elapsed_seconds += 1;
                        snapshot.eta_seconds = Some((snapshot.total_steps - snapshot.step) / 40);
                        let progress = snapshot.step as f32 / snapshot.total_steps.max(1) as f32;
                        let loss = 1.5 * (-3.2 * progress).exp() + 0.1;
                        let step = snapshot.step;
                        snapshot.loss.push(TrainingLoss::new(
                            step,
                            Some(loss),
                            if step.is_multiple_of(160) {
                                Some(loss + 0.08)
                            } else {
                                None
                            },
                        ));
                        if step.is_multiple_of(400) {
                            snapshot.events.push(
                                TrainingEvent::new(
                                    format!("checkpoint-{step}"),
                                    lang.text("Simulated checkpoint", "模拟检查点"),
                                )
                                .with_step(step),
                            );
                        }
                        if snapshot.step == snapshot.total_steps {
                            snapshot.status = TrainingRunStatus::Completed;
                            snapshot.eta_seconds = Some(0);
                            snapshot.events.push(TrainingEvent::new(
                                "done",
                                lang.text("Simulation completed", "模拟运行已完成"),
                            ));
                        }
                        monitor.set_snapshot(snapshot.clone(), cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let fail_deploy = Rc::new(Cell::new(true));
        let action_state = state.clone();
        cx.subscribe(
            &monitor,
            move |_, monitor, request: &TrainingRunRequest, cx| {
                let id = request.id;
                let action = request.action;
                let state = action_state.clone();
                let weak = monitor.downgrade();
                let fail = action == TrainingRunAction::Deploy && fail_deploy.replace(false);
                cx.spawn(async move |_, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(280))
                        .await;
                    let _ = weak.update(cx, |monitor, cx| {
                        if fail {
                            monitor.resolve(
                                id,
                                Err(lang
                                    .text(
                                        "Simulated deployment failed; retry.",
                                        "模拟部署失败，请重试。",
                                    )
                                    .into()),
                                cx,
                            );
                            return;
                        }
                        let mut snapshot = state.borrow_mut();
                        match action {
                            TrainingRunAction::Pause => snapshot.status = TrainingRunStatus::Paused,
                            TrainingRunAction::Resume => {
                                snapshot.status = TrainingRunStatus::Running
                            }
                            TrainingRunAction::Cancel => {
                                snapshot.status = TrainingRunStatus::Cancelled
                            }
                            TrainingRunAction::Retry => {
                                snapshot.status = TrainingRunStatus::Running;
                                snapshot.error = None;
                            }
                            TrainingRunAction::Restart => {
                                snapshot.status = TrainingRunStatus::Running;
                                snapshot.step = 0;
                                snapshot.elapsed_seconds = 0;
                                snapshot.eta_seconds = Some(30);
                                snapshot.loss.clear();
                                snapshot.events.clear();
                                snapshot.error = None;
                                snapshot.endpoint = None;
                            }
                            TrainingRunAction::Deploy => {
                                snapshot.endpoint =
                                    Some(lang.text("Simulated endpoint", "模拟端点").into())
                            }
                        }
                        monitor.resolve(id, Ok(snapshot.clone()), cx);
                    });
                })
                .detach();
            },
        )
        .detach();
        Self {
            monitor,
            snapshot: state,
        }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let label = lang.text("Simulate failure", "模拟失败");
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .flex_col()
            .gap_3()
            .items_center()
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.monitor.clone())
            .child(
                face(
                    "training-simulate-failure",
                    label,
                    36.,
                    kui::foundation::surface_radius(cx).into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    let mut snapshot = s.snapshot.borrow_mut();
                    snapshot.status = TrainingRunStatus::Failed;
                    snapshot.error =
                        Some(lang.text("Simulated out of memory", "模拟显存不足").into());
                    snapshot.events.push(TrainingEvent::new(
                        "failure",
                        lang.text("Simulated failure", "模拟失败"),
                    ));
                    s.monitor
                        .update(cx, |m, cx| m.set_snapshot(snapshot.clone(), cx));
                })),
            )
    }
}
