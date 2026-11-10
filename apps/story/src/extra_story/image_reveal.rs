//! Four local images share one sampling clock. Nothing is sent or generated.
use gpui::{prelude::*, *};
use kui::{
    components::image_reveal::{ImageReveal, ImageRevealEvent},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, disabled_motion, face},
    },
};
use std::{sync::Arc, time::Duration};

pub const IDS: &[&str] = &["image-reveal"];
const STEPS: u32 = 30;
const PHOTOS: [&[u8]; 4] = [
    include_bytes!("../../assets/navigation/room-0.jpg"),
    include_bytes!("../../assets/navigation/room-1.jpg"),
    include_bytes!("../../assets/navigation/room-3.jpg"),
    include_bytes!("../../assets/navigation/room-4.jpg"),
];
const LABELS: [(&str, &str); 4] = [
    ("A quiet room", "安静的房间"),
    ("A family drawing room", "家庭起居室"),
    ("The Pantheon in Rome", "罗马万神庙"),
    ("The interior of a church", "教堂内部"),
];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Live,
    Queued,
    Done,
    Error,
}

struct Example {
    views: [Entity<ImageReveal>; 4],
    photos: [ImageSource; 4],
    language: UiLanguage,
    mode: Mode,
    run: u64,
    _subscriptions: [Subscription; 4],
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let views = LABELS.map(|(en, zh)| {
            cx.new(|_| ImageReveal::new(language.text(en, zh), 1.).with_steps(STEPS))
        });
        let subscriptions = std::array::from_fn(|index| {
            cx.subscribe(&views[index], move |s, _, _: &ImageRevealEvent, cx| {
                s.start(Mode::Live, vec![index], cx);
            })
        });
        let mut demo = Self {
            views,
            photos: PHOTOS
                .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())).into()),
            language,
            mode: Mode::Live,
            run: 0,
            _subscriptions: subscriptions,
        };
        demo.start(Mode::Live, vec![0, 1, 2, 3], cx);
        demo
    }

    fn start(&mut self, mode: Mode, tiles: Vec<usize>, cx: &mut Context<Self>) {
        self.run += 1;
        self.mode = mode;
        let run = self.run;
        let reduced = disabled_motion(cx);
        for &tile in &tiles {
            let photo = self.photos[tile].clone();
            self.views[tile].update(cx, |view, cx| {
                view.begin(run, cx);
                match mode {
                    Mode::Live if !reduced => {}
                    Mode::Queued => {}
                    Mode::Error if tile == 2 => {
                        view.fail(run, self.language.text("Sample failed", "样例生成失败"), cx);
                    }
                    _ => view.finish(run, photo, cx),
                }
            });
        }
        cx.notify();
        if mode != Mode::Live || reduced {
            return;
        }
        cx.spawn(async move |this, cx| {
            let mut tick = 0;
            while tick <= STEPS {
                let delay = if tick == 0 { 400 } else { 153 };
                cx.background_executor()
                    .timer(Duration::from_millis(delay))
                    .await;
                let Ok(active) = this.update(cx, |demo, cx| {
                    if demo.run != run {
                        return false;
                    }
                    let visible = cx
                        .try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)());
                    if !visible {
                        return true;
                    }
                    for &tile in &tiles {
                        let photo = demo.photos[tile].clone();
                        demo.views[tile].update(cx, |view, cx| {
                            if tick == 0 {
                                view.set_preview(run, photo, cx);
                                view.set_progress(run, 0., cx);
                            } else if tick == STEPS {
                                view.finish(run, photo, cx);
                            } else {
                                view.set_progress(run, tick as f32 / STEPS as f32, cx);
                            }
                        });
                    }
                    tick += 1;
                    true
                }) else {
                    break;
                };
                if !active {
                    break;
                }
            }
        })
        .detach();
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let t = |en, zh| self.language.text(en, zh);
        let restart = t("Generate again", "重新生成");
        let mut controls = div().flex().flex_wrap().gap_1().justify_center();
        for (mode, label) in [
            (Mode::Live, t("Live", "实时")),
            (Mode::Queued, t("Queued", "排队中")),
            (Mode::Done, t("Done", "已完成")),
            (Mode::Error, t("Error", "失败")),
        ] {
            controls = controls.child(
                face(
                    ("reveal-demo-mode", mode as usize),
                    label,
                    32.,
                    8.,
                    if self.mode == mode {
                        Tone::Primary
                    } else {
                        Tone::Clear
                    },
                    cx,
                )
                .aria_toggled((self.mode == mode).into())
                .px_3()
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| s.start(mode, vec![0, 1, 2, 3], cx))),
            );
        }
        let mut grid = div().w_full().flex().flex_col().gap_2();
        for tiles in self.views.chunks(2) {
            grid = grid.child(
                div().flex().gap_2().children(
                    tiles
                        .iter()
                        .map(|view| div().flex_1().min_w_0().child(view.clone())),
                ),
            );
        }
        div()
            .size_full()
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(522.))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(t("Quiet interiors", "静谧室内"))
                                    .child(div().text_size(px(12.)).text_color(p.secondary).child(
                                        t(
                                            "30 steps · 4 images · Local simulation",
                                            "30 步 · 4 张图片 · 本地模拟",
                                        ),
                                    )),
                            )
                            .child(
                                face("reveal-demo-restart", restart, 32., 8., Tone::Surface, cx)
                                    .px_3()
                                    .child(restart)
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.start(Mode::Live, vec![0, 1, 2, 3], cx)
                                    })),
                            ),
                    )
                    .child(grid)
                    .child(controls),
            )
    }
}
