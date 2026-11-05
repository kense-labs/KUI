//! Activity phases, composed variants and optical sizes for the assistant mark.
use gpui::{prelude::*, *};
use kui::{
    components::{
        ai_spark::{AiSpark, SparkStatus, SparkTone, SparkVariant},
        button::Button,
    },
    primitives::{
        Size,
        control::UiLanguage,
        family::{Paint, Slide, Tone, face},
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const STATES: [SparkStatus; 5] = [
    SparkStatus::Idle,
    SparkStatus::Thinking,
    SparkStatus::Writing,
    SparkStatus::Done,
    SparkStatus::Error,
];
const SIZES: [f32; 6] = [12., 16., 20., 24., 32., 48.];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    (id == "ai-spark").then(|| cx.new(Demo::new).into())
}
struct Demo {
    sparks: Vec<Entity<AiSpark>>,
    status: SparkStatus,
    language: UiLanguage,
    playing: bool,
    cycle: Option<Task<()>>,
    focus: [FocusHandle; 5],
    highlight: Rc<RefCell<(bool, Slide, Slide)>>,
    scroll: ScrollHandle,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let labels = [
            language.text("Ready", "就绪"),
            language.text("Thinking", "思考中"),
            language.text("Writing", "生成中"),
            language.text("Done", "完成"),
            language.text("Couldn't finish", "未能完成"),
        ];
        let aria_labels = [
            language.text("AI ready", "智能助手已就绪"),
            language.text("AI is thinking", "智能助手正在思考"),
            language.text("AI is writing", "智能助手正在生成"),
            language.text("AI finished", "智能助手已完成"),
            language.text("AI couldn't finish", "智能助手未能完成"),
        ];
        let mut configs = vec![
            AiSpark::new("spark-main").with_size(120.),
            AiSpark::new("spark-avatar")
                .with_variant(SparkVariant::Avatar)
                .with_announce(false),
            AiSpark::new("spark-label")
                .with_variant(SparkVariant::Pill)
                .with_announce(false),
            AiSpark::new("spark-foreground")
                .with_variant(SparkVariant::Pill)
                .with_tone(SparkTone::Foreground)
                .with_announce(false),
        ];
        for (i, size) in SIZES.into_iter().enumerate() {
            configs.push(
                AiSpark::new(("spark-size", i))
                    .with_size(size)
                    .with_announce(false),
            );
        }
        let sparks: Vec<_> = configs
            .into_iter()
            .map(|spark| {
                cx.new(|_| {
                    spark
                        .with_status(SparkStatus::Idle)
                        .with_labels(labels)
                        .with_aria_labels(aria_labels)
                })
            })
            .collect();
        let weak = cx.weak_entity();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                if weak
                    .update(cx, |demo: &mut Demo, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        for spark in &demo.sparks {
                            spark.update(cx, |v, cx| v.set_visible(visible, cx));
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            sparks,
            status: SparkStatus::Idle,
            language,
            playing: true,
            cycle: None,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            highlight: Rc::new(RefCell::new((false, Slide::new(0.), Slide::new(0.)))),
            scroll: ScrollHandle::new(),
        }
    }
    fn apply(&mut self, status: SparkStatus, cx: &mut Context<Self>) {
        self.status = status;
        self.scroll
            .scroll_to_item(STATES.iter().position(|s| *s == status).unwrap_or(0));
        for spark in &self.sparks {
            spark.update(cx, |v, cx| v.set_status(status, cx));
        }
        cx.notify();
    }
    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        self.playing = false;
        self.cycle = None;
        self.apply(STATES[index], cx);
    }
    fn cycle(&mut self, cx: &mut Context<Self>) {
        if self.playing && self.cycle.is_none() {
            let i = STATES.iter().position(|s| *s == self.status).unwrap_or(0);
            let delay = [1800, 2800, 3600, 2200, 600][i];
            self.cycle = Some(cx.spawn(async move |weak, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(delay))
                    .await;
                let _ = weak.update(cx, |v, cx| {
                    v.cycle = None;
                    v.apply(STATES[if i == 4 { 0 } else { (i + 1) % 4 }], cx);
                });
            }));
        }
    }
}
impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.cycle(cx);
        let p = Paint::get(cx);
        let narrow = w.viewport_size().width <= px(440.);
        let t = |en, zh| self.language.text(en, zh);
        let radius = kui::foundation::surface_radius(cx);
        let index = STATES.iter().position(|s| *s == self.status).unwrap_or(0);
        let target = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let paint_target = target.clone();
        let highlight = self.highlight.clone();
        let mut options = div().flex().gap(px(2.)).p(px(3.)).relative().child(
            canvas(
                |_, _, _| (),
                move |bounds, _, w, cx| {
                    let Some(b) = paint_target.get() else {
                        return;
                    };
                    let x = f32::from(b.left() - bounds.left());
                    let width = f32::from(b.size.width);
                    let mut s = highlight.borrow_mut();
                    if !s.0 {
                        s.1.reset(x);
                        s.2.reset(width);
                        s.0 = true;
                    } else {
                        s.1.duration(x, 0.42, cx);
                        s.2.duration(width, 0.42, cx);
                    }
                    let rect = Bounds::new(
                        point(bounds.left() + px(s.1.get(w, cx)), b.top()),
                        size(px(s.2.get(w, cx)), b.size.height),
                    )
                    .intersect(&bounds);
                    w.paint_quad(fill(rect, p.tray).corner_radii(px(4.)));
                },
            )
            .absolute()
            .inset_0(),
        );
        let mut row = div().flex().gap(px(2.));
        for (i, label) in [
            t("Idle", "空闲"),
            t("Thinking", "思考"),
            t("Writing", "生成"),
            t("Done", "完成"),
            t("Error", "失败"),
        ]
        .into_iter()
        .enumerate()
        {
            row = row.child(
                face(("spark-state", i), label, 36., 4., Tone::Clear, cx)
                    .px(px(if narrow { 9. } else { 12. }))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(if index == i { p.ink } else { p.secondary })
                    .child(label)
                    .aria_toggled((index == i).into())
                    .track_focus(&self.focus[i])
                    .tab_stop(index == i)
                    .on_click(cx.listener(move |v, _, _, cx| v.choose(i, cx)))
                    .on_key_down(cx.listener(move |v, e: &KeyDownEvent, w, cx| {
                        let next = match e.keystroke.key.as_str() {
                            "left" | "up" => (i + 4) % 5,
                            "right" | "down" => (i + 1) % 5,
                            "home" => 0,
                            "end" => 4,
                            "enter" | "space" => i,
                            _ => return,
                        };
                        w.prevent_default();
                        cx.stop_propagation();
                        v.choose(next, cx);
                        v.focus[next].focus(w, cx);
                    })),
            );
        }
        options = options.child(
            row.on_children_prepainted(move |bounds, _, _| target.set(bounds.get(index).copied()))
                .id("spark-state-track")
                .min_w(px(0.))
                .max_w(w.viewport_size().width - px(if narrow { 56. } else { 80. }))
                .overflow_x_scroll()
                .track_scroll(&self.scroll),
        );
        let controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .px(px(if narrow { 12. } else { 16. }))
            .pb(px(24.))
            .child(
                div()
                    .id("spark-state-group")
                    .debug_selector(|| "Spark state".into())
                    .role(Role::Group)
                    .aria_label(t("Spark state", "光芒状态"))
                    .max_w(relative(1.))
                    .border_1()
                    .border_color(p.line)
                    .rounded(radius)
                    .child(options),
            )
            .child(
                Button::new("spark-playthrough")
                    .ghost()
                    .with_size(Size::Sm)
                    .label(if self.playing {
                        t("Pause", "暂停")
                    } else {
                        t("Play through", "自动播放")
                    })
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.playing = !v.playing;
                        v.cycle = None;
                        cx.notify();
                    })),
            );
        let mut ladder = div()
            .flex()
            .items_end()
            .justify_between()
            .gap(px(8.))
            .px(px(if narrow { 16. } else { 24. }))
            .pt(px(16.))
            .pb(px(20.))
            .border_t_1()
            .border_color(p.line);
        for (i, size) in SIZES.into_iter().enumerate() {
            ladder = ladder.child(
                div()
                    .min_w(px(32.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .h(px(48.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.sparks[4 + i].clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(format!("{size:.0}")),
                    ),
            );
        }
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .items_center()
            .justify_center()
            .p(px(if narrow { 12. } else { 20. }))
            .bg(p.stage)
            .child(
                div()
                    .w_full()
                    .max_w(px(580.))
                    .border_1()
                    .border_color(p.line)
                    .rounded(radius)
                    .overflow_hidden()
                    .bg(p.panel)
                    .child(
                        div()
                            .h(px(if narrow { 200. } else { 240. }))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.sparks[0].clone()),
                    )
                    .child(controls)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .when(narrow, |v| v.justify_center())
                            .when(!narrow, |v| v.justify_between())
                            .gap_x(px(16.))
                            .gap_y(px(12.))
                            .py(px(16.))
                            .px(px(if narrow { 16. } else { 24. }))
                            .border_t_1()
                            .border_color(p.line)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(self.sparks[1].clone())
                                    .child(self.sparks[2].clone()),
                            )
                            .child(self.sparks[3].clone()),
                    )
                    .child(ladder),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::Demo;
    use gpui::{Modifiers, TestAppContext, px, size};
    use kui::components::ai_spark::SparkStatus;
    use std::time::Duration;
    #[test]
    fn manual_selection_and_pause_stop_only_the_state_cycle() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (demo, window) = app.add_window_view(|_, cx| Demo::new(cx));
        window.simulate_resize(size(px(620.), px(560.)));
        window.run_until_parked();
        let thinking = window.debug_bounds("Thinking").unwrap().center();
        window.simulate_click(thinking, Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(!demo.read(cx).playing);
            assert_eq!(demo.read(cx).status, SparkStatus::Thinking);
        });
        window.executor().advance_clock(Duration::from_secs(4));
        window.run_until_parked();
        window.update(|_, cx| assert_eq!(demo.read(cx).status, SparkStatus::Thinking));
        let callbacks = window.update(|w, cx| w.simulate_next_frame(cx));
        assert!(callbacks > 0, "manual selection froze the mark");
        window.run_until_parked();
        let play = window.debug_bounds("Play through").unwrap().center();
        window.simulate_click(play, Modifiers::default());
        window.run_until_parked();
        window.executor().advance_clock(Duration::from_millis(2800));
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(demo.read(cx).playing);
            assert_eq!(demo.read(cx).status, SparkStatus::Writing);
        });
        let pause = window.debug_bounds("Pause").unwrap().center();
        window.simulate_click(pause, Modifiers::default());
        window.run_until_parked();
        window.executor().advance_clock(Duration::from_secs(4));
        window.run_until_parked();
        window.update(|_, cx| assert_eq!(demo.read(cx).status, SparkStatus::Writing));
        let callbacks = window.update(|w, cx| w.simulate_next_frame(cx));
        assert!(callbacks > 0, "Pause froze the independently animated mark");
    }
    #[test]
    fn narrow_demo_scrolls_the_selected_status_into_view() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (demo, window) = app.add_window_view(|_, cx| Demo::new(cx));
        window.simulate_resize(size(px(320.), px(600.)));
        window.run_until_parked();
        let group = window.debug_bounds("Spark state").unwrap();
        assert!(group.left() >= px(0.) && group.right() <= px(320.));
        for (index, label) in [(4, "Error"), (0, "Idle")] {
            window.update(|_, cx| demo.update(cx, |v, cx| v.choose(index, cx)));
            window.run_until_parked();
            let b = window.debug_bounds(label).unwrap();
            assert!(
                b.left() >= group.left() && b.right() <= group.right(),
                "{label} exceeds the scroll viewport: {b:?} / {group:?}"
            );
        }
    }
}
