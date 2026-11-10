//! Locally simulated training: no model is trained and no deployment is made.
use gpui::{prelude::*, *};
use kui::{
    components::training_run::{
        TrainingEvent, TrainingLoss, TrainingRun, TrainingRunAction, TrainingRunRequest,
        TrainingRunSnapshot, TrainingRunStatus,
    },
    primitives::{
        control::UiLanguage,
        family::Paint,
        ops_surface::{btn, row},
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
    failure_scenario: Rc<Cell<bool>>,
    baseline: TrainingRunSnapshot,
    generation: Rc<Cell<u64>>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let mut initial = TrainingRunSnapshot::new(
            "support-triage-v3",
            t("Kense sample model", "Kense 样例模型"),
            1860,
            3,
        );
        initial.details = "LoRA r16 · 8× H200".into();
        initial.dataset = t("Sample dataset · 48,210 examples", "样例数据集 · 48,210 条").into();
        initial.learning_rate = Some(0.0002);
        initial.eta_seconds = Some(1120);
        let state = Rc::new(RefCell::new(initial.clone()));
        let failure_scenario = Rc::new(Cell::new(false));
        let scenario = failure_scenario.clone();
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
            let mut queued = 0;
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
                        if snapshot.status == TrainingRunStatus::Queued {
                            queued += 1;
                            if queued >= 4 {
                                snapshot.status = TrainingRunStatus::Running;
                                snapshot.events.push(TrainingEvent::new(
                                    "start",
                                    lang.text("Training started", "训练已开始"),
                                ));
                                monitor.set_snapshot(snapshot.clone(), cx);
                                queued = 0;
                            }
                            return;
                        }
                        queued = 0;
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
                        if scenario.get()
                            && step >= 720
                            && !snapshot
                                .events
                                .iter()
                                .any(|event| event.id.as_ref() == "oom")
                        {
                            snapshot.status = TrainingRunStatus::Failed;
                            snapshot.error = Some(
                                lang.text(
                                    "GPU out of memory. Retry with a smaller batch.",
                                    "GPU 显存不足。请减小批量后重试。",
                                )
                                .into(),
                            );
                            snapshot.events.push(
                                TrainingEvent::new("oom", lang.text("Out of memory", "显存不足"))
                                    .with_step(step),
                            );
                        } else if snapshot.step == snapshot.total_steps {
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
        let generation = Rc::new(Cell::new(0_u64));
        let guard = generation.clone();
        cx.subscribe(
            &monitor,
            move |_, monitor, request: &TrainingRunRequest, cx| {
                let id = request.id;
                let action = request.action;
                let state = action_state.clone();
                let generation = guard.clone();
                let issued = generation.get();
                let weak = monitor.downgrade();
                let fail = action == TrainingRunAction::Deploy && fail_deploy.replace(false);
                cx.spawn(async move |_, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(280))
                        .await;
                    let _ = weak.update(cx, |monitor, cx| {
                        if issued != generation.get() {
                            return;
                        }
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
            failure_scenario,
            baseline: initial,
            generation,
        }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
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
                row().flex_wrap().justify_center().children(
                    [("Healthy run", "正常运行"), ("Out of memory", "显存不足")]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (en, zh))| {
                            btn(
                                ("training-scenario", i),
                                lang.text(en, zh),
                                self.failure_scenario.get() == (i == 1),
                                cx,
                                move |s, _, cx| {
                                    s.generation.set(s.generation.get().wrapping_add(1));
                                    s.failure_scenario.set(i == 1);
                                    *s.snapshot.borrow_mut() = s.baseline.clone();
                                    s.monitor
                                        .update(cx, |v, cx| v.set_snapshot(s.baseline.clone(), cx));
                                    cx.notify();
                                },
                            )
                        }),
                ),
            )
    }
}
