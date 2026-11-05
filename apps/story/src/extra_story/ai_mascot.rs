//! Host-driven Kense mascot showcase; activity states are simulated.
use gpui::{prelude::*, *};
use kui::{
    components::ai_mascot::{AiMascot, MascotStatus},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
    },
};
use std::time::Duration;
pub const IDS: &[&str] = &["ai-mascot"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    mascot: Entity<AiMascot>,
    compact: Entity<AiMascot>,
    language: UiLanguage,
    status: MascotStatus,
    paused: bool,
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let mascot = cx.new(|_| AiMascot::new("Kense").with_size(128.));
        let compact = cx.new(|_| AiMascot::new("Kense").with_size(32.));
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        mascot.update(cx, |v, cx| v.set_visible(visible, cx));
        compact.update(cx, |v, cx| v.set_visible(visible, cx));
        let large = mascot.downgrade();
        let small = compact.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                let large_alive = large
                    .update(cx, |v, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        v.set_visible(visible, cx);
                    })
                    .is_ok();
                let small_alive = small
                    .update(cx, |v, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        v.set_visible(visible, cx);
                    })
                    .is_ok();
                if !large_alive && !small_alive {
                    break;
                }
            }
        })
        .detach();
        Self {
            mascot,
            compact,
            language,
            status: MascotStatus::Idle,
            paused: false,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let t = |en, zh| self.language.text(en, zh);
        let mut controls = div().flex().flex_wrap().justify_center().gap_2();
        for (i, (status, label)) in [
            (MascotStatus::Idle, t("Idle", "空闲")),
            (MascotStatus::Listening, t("Listening", "聆听")),
            (MascotStatus::Thinking, t("Thinking", "思考")),
            (MascotStatus::Writing, t("Writing", "生成")),
            (MascotStatus::Done, t("Done", "完成")),
            (MascotStatus::Error, t("Error", "出错")),
        ]
        .into_iter()
        .enumerate()
        {
            controls = controls.child(
                face(
                    ("mascot-state", i),
                    label,
                    32.,
                    8.,
                    if self.status == status {
                        Tone::Primary
                    } else {
                        Tone::Surface
                    },
                    cx,
                )
                .px_2()
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.status = status;
                    s.mascot.update(cx, |v, cx| v.set_status(status, cx));
                    s.compact.update(cx, |v, cx| v.set_status(status, cx));
                    cx.notify();
                })),
            );
        }
        let pause = if self.paused {
            t("Resume", "继续")
        } else {
            t("Pause", "暂停")
        };
        controls = controls.child(
            face("mascot-pause", pause, 32., 8., Tone::Surface, cx)
                .px_2()
                .child(pause)
                .on_click(cx.listener(|s, _, _, cx| {
                    s.paused = !s.paused;
                    s.mascot.update(cx, |v, cx| v.set_paused(s.paused, cx));
                    s.compact.update(cx, |v, cx| v.set_paused(s.paused, cx));
                    cx.notify();
                })),
        );
        div()
            .size_full()
            .min_h(px(520.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(t("Simulated activity states", "模拟活动状态")),
            )
            .child(self.mascot.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(self.compact.clone())
                    .child("Kense"),
            )
            .child(controls)
    }
}
