//! Kense assistant activity showcase.
use gpui::{prelude::*, *};
use kui::{
    components::ai_mascot::{AiMascot, MascotStatus},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, disabled_motion, face},
        glyph::{Glyph, icon},
    },
};
use std::time::Duration;

pub const IDS: &[&str] = &["ai-mascot"];
const STATES: [(MascotStatus, &str, &str, &str, &str, u32); 6] = [
    (
        MascotStatus::Idle,
        "Idle",
        "空闲",
        "Kense is ready when you are",
        "Kense 已就绪",
        2000,
    ),
    (
        MascotStatus::Listening,
        "Listening",
        "聆听",
        "Listening",
        "正在聆听",
        1800,
    ),
    (
        MascotStatus::Thinking,
        "Thinking",
        "思考",
        "Thinking",
        "正在思考",
        2300,
    ),
    (
        MascotStatus::Writing,
        "Writing",
        "生成",
        "Writing a reply",
        "正在生成回复",
        2200,
    ),
    (
        MascotStatus::Done,
        "Done",
        "完成",
        "Reply ready, 214 words",
        "回复已就绪 · 214 字",
        1500,
    ),
    (
        MascotStatus::Error,
        "Error",
        "出错",
        "Could not finish the reply",
        "未能完成回复",
        1400,
    ),
];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    mascot: Entity<AiMascot>,
    compact: Entity<AiMascot>,
    language: UiLanguage,
    selected: usize,
    playing: bool,
    elapsed: u32,
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let mascot = cx.new(|_| AiMascot::new("Kense").with_size(148.));
        let compact = cx.new(|_| AiMascot::new("Kense").with_size(24.).follow_pointer(false));
        cx.spawn(async |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if this.update(cx, |view, cx| view.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            mascot,
            compact,
            language: UiLanguage::current(cx),
            selected: 0,
            playing: !disabled_motion(cx),
            elapsed: 0,
        }
    }
    fn tick(&mut self, cx: &mut Context<Self>) {
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        self.mascot
            .update(cx, |view, cx| view.set_visible(visible, cx));
        self.compact
            .update(cx, |view, cx| view.set_visible(visible, cx));
        if visible && self.playing && !disabled_motion(cx) {
            self.elapsed += 100;
            if self.elapsed >= STATES[self.selected].5 {
                self.select((self.selected + 1) % STATES.len(), cx);
            }
        }
    }
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected = index;
        self.elapsed = 0;
        let status = STATES[index].0;
        self.mascot
            .update(cx, |view, cx| view.set_status(status, cx));
        self.compact
            .update(cx, |view, cx| view.set_status(status, cx));
        cx.notify();
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = kui::foundation::surface_radius(cx);
        let t = |en, zh| self.language.text(en, zh);
        let mut states = div()
            .id("mascot-states")
            .flex()
            .min_w_0()
            .overflow_x_scroll()
            .gap(px(2.))
            .p(px(3.))
            .rounded(radius)
            .bg(p.stage);
        for (i, (_, en, zh, _, _, _)) in STATES.into_iter().enumerate() {
            let label = t(en, zh);
            states = states.child(
                face(
                    ("mascot-state", i),
                    label,
                    36.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(10.))
                .aria_toggled((self.selected == i).into())
                .when(self.selected == i, |view| view.bg(p.panel))
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.playing = false;
                    s.select(i, cx);
                })),
            );
        }
        let playing = self.playing && !disabled_motion(cx);
        let (play_label, play_icon) = if playing {
            (t("Pause", "暂停"), Glyph::Pause)
        } else {
            (t("Play", "播放"), Glyph::Play)
        };
        let (_, _, _, en, zh, _) = STATES[self.selected];
        div()
            .w_full()
            .min_h(px(520.))
            .p(px(16.))
            .flex()
            .flex_col()
            .justify_center()
            .items_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(490.))
                    .rounded(radius)
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .p(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .h(px(208.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.mascot.clone()),
                    )
                    .child(
                        div()
                            .h(px(48.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(8.))
                            .text_size(px(14.))
                            .text_color(p.secondary)
                            .child(self.compact.clone())
                            .child(t(en, zh)),
                    )
                    .child(
                        div()
                            .border_t_1()
                            .border_color(p.line)
                            .pt(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(states.flex_1())
                            .when(!disabled_motion(cx), |row| {
                                row.child(
                                    face(
                                        "mascot-play",
                                        play_label,
                                        36.,
                                        radius.into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .px(px(10.))
                                    .child(icon(play_icon, 14., p.ink))
                                    .child(play_label)
                                    .on_click(cx.listener(
                                        |s, _, _, cx| {
                                            s.playing = !s.playing;
                                            cx.notify();
                                        },
                                    )),
                                )
                            }),
                    ),
            )
    }
}
