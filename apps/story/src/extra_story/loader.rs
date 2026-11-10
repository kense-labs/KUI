//! Status and shape controls for the morphing loader.
use gpui::{prelude::*, *};
use kui::{
    components::morph_loader::{LoaderStatus, LoaderVariant, MorphLoader},
    primitives::{
        control::UiLanguage,
        family::{Paint, Slide, Tone, face},
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const VARIANTS: [LoaderVariant; 4] = [
    LoaderVariant::Dots,
    LoaderVariant::Bars,
    LoaderVariant::Ring,
    LoaderVariant::Square,
];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    (id == "morph-loader").then(|| cx.new(Demo::new).into())
}

struct Demo {
    loader: Entity<MorphLoader>,
    language: UiLanguage,
    selected: usize,
    status: LoaderStatus,
    reset_generation: u64,
    focus: [FocusHandle; 4],
    highlight: Rc<RefCell<(Option<usize>, Slide, Slide)>>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let labels = [
            language.text("Loading", "加载中"),
            language.text("Published", "已发布"),
            language.text("Publish failed", "发布失败"),
        ];
        Self {
            loader: cx.new(|_| {
                MorphLoader::new("morph-loader")
                    .with_size(72.)
                    .with_labels(labels)
            }),
            language,
            selected: 0,
            status: LoaderStatus::Loading,
            reset_generation: 0,
            focus: std::array::from_fn(|_| cx.focus_handle()),
            highlight: Rc::new(RefCell::new((None, Slide::new(0.), Slide::new(0.)))),
        }
    }
    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected = index;
        self.reset_generation = self.reset_generation.wrapping_add(1);
        self.status = LoaderStatus::Loading;
        self.loader.update(cx, |loader, cx| {
            loader.set_variant(VARIANTS[index], cx);
            loader.set_status(LoaderStatus::Loading, cx);
        });
        cx.notify();
    }
    fn finish(&mut self, status: LoaderStatus, cx: &mut Context<Self>) {
        if self.status != LoaderStatus::Loading {
            return;
        }
        self.status = status;
        self.loader
            .update(cx, |loader, cx| loader.set_status(status, cx));
        self.reset_generation = self.reset_generation.wrapping_add(1);
        let generation = self.reset_generation;
        cx.spawn(async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1700))
                .await;
            let _ = weak.update(cx, |demo, cx| {
                if demo.reset_generation == generation {
                    demo.status = LoaderStatus::Loading;
                    demo.loader.update(cx, |loader, cx| {
                        loader.set_status(LoaderStatus::Loading, cx)
                    });
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }
}
impl Render for Demo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let narrow = window.viewport_size().width <= px(420.);
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        self.loader
            .update(cx, |loader, cx| loader.set_visible(visible, cx));
        let t = |en, zh| self.language.text(en, zh);
        let target = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let target_for_paint = target.clone();
        let highlight = self.highlight.clone();
        let selected = self.selected;
        let radius = kui::foundation::surface_radius(cx);
        let mut variants = div()
            .id("loader-shapes")
            .relative()
            .flex()
            .p(px(2.))
            .h(px(36.))
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .role(Role::RadioGroup)
            .aria_label(t("Loader shape", "加载图形"))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let Some(target) = target_for_paint.get() else {
                            return;
                        };
                        let x = f32::from(target.left() - bounds.left());
                        let width = f32::from(target.size.width);
                        let mut state = highlight.borrow_mut();
                        if state.0.is_none() {
                            state.1.reset(x);
                            state.2.reset(width);
                        } else {
                            state.1.duration(x, 0.42, cx);
                            state.2.duration(width, 0.42, cx);
                        }
                        state.0 = Some(selected);
                        let rect = Bounds::new(
                            point(bounds.left() + px(state.1.get(window, cx)), target.top()),
                            size(px(state.2.get(window, cx)), target.size.height),
                        )
                        .intersect(&bounds);
                        window.paint_quad(fill(rect, p.tray).corner_radii(radius));
                    },
                )
                .absolute()
                .inset_0(),
            );
        let mut row = div().flex();
        for (index, (id, label)) in [
            ("dots", t("Dots", "圆点")),
            ("bars", t("Bars", "竖条")),
            ("ring", t("Ring", "圆环")),
            ("square", t("Square", "方形")),
        ]
        .into_iter()
        .enumerate()
        {
            row = row.child(
                face(id, label, 30., radius.into(), Tone::Clear, cx)
                    .min_w(px(if narrow { 0. } else { 60. }))
                    .px(px(if narrow { 10. } else { 12. }))
                    .font_weight(FontWeight::NORMAL)
                    .child(label)
                    .text_color(if selected == index {
                        p.ink
                    } else {
                        p.secondary
                    })
                    .role(Role::RadioButton)
                    .aria_toggled((selected == index).into())
                    .track_focus(&self.focus[index])
                    .tab_stop(selected == index)
                    .on_click(cx.listener(move |demo, _, _, cx| demo.choose(index, cx)))
                    .on_key_down(cx.listener(move |demo, event: &KeyDownEvent, window, cx| {
                        let next = match event.keystroke.key.as_str() {
                            "left" | "up" => (index + 3) % 4,
                            "right" | "down" => (index + 1) % 4,
                            "home" => 0,
                            "end" => 3,
                            "enter" | "space" => index,
                            _ => return,
                        };
                        window.prevent_default();
                        cx.stop_propagation();
                        demo.choose(next, cx);
                        demo.focus[next].focus(window, cx);
                    })),
            );
        }
        variants = variants.child(row.on_children_prepainted(move |bounds, _, _| {
            target.set(bounds.get(selected).copied());
        }));
        let mut actions = div().flex().items_center().gap(px(8.));
        for (id, label, status) in [
            (
                "loader-success",
                t("Succeed", "成功"),
                LoaderStatus::Success,
            ),
            ("loader-fail", t("Fail", "失败"), LoaderStatus::Error),
        ] {
            let enabled = self.status == LoaderStatus::Loading;
            actions = actions.child(
                face(id, label, 36., radius.into(), Tone::Clear, cx)
                    .px(px(14.))
                    .border_1()
                    .border_color(p.line)
                    .child(label)
                    .when(!enabled, |view| {
                        view.opacity(0.45).cursor_default().tab_stop(false)
                    })
                    .on_click(cx.listener(move |demo, _, _, cx| demo.finish(status, cx)))
                    .on_key_down(cx.listener(move |demo, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                            demo.finish(status, cx);
                        }
                    })),
            );
        }
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .child(
                div()
                    .w_full()
                    .max_w(px(480.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(20.))
                    .child(
                        div()
                            .w_full()
                            .h(px(220.))
                            .text_color(kui::foundation::Theme::global(cx).tokens.colors.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.loader.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_center()
                            .gap(px(10.))
                            .child(variants)
                            .child(actions),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::Demo;
    use gpui::{Modifiers, TestAppContext, px, size};
    use kui::components::morph_loader::{LoaderStatus, LoaderVariant};
    use std::time::Duration;

    #[test]
    fn selecting_a_shape_after_failure_restarts_the_painted_loader() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (demo, window) = app.add_window_view(|_, cx| Demo::new(cx));
        window.simulate_resize(size(px(628.), px(520.)));
        window.run_until_parked();
        let fail = window.debug_bounds("Fail").unwrap().center();
        window.simulate_click(fail, Modifiers::default());
        window.run_until_parked();
        for _ in 0..60 {
            window.executor().advance_clock(Duration::from_millis(16));
            window.update(|window, cx| {
                window.simulate_next_frame(cx);
            });
            window.run_until_parked();
        }
        let ring = window.debug_bounds("Ring").unwrap().center();
        window.simulate_click(ring, Modifiers::default());
        window.run_until_parked();
        for _ in 0..120 {
            window.executor().advance_clock(Duration::from_millis(16));
            window.update(|window, cx| {
                window.simulate_next_frame(cx);
            });
            window.run_until_parked();
        }
        window.update(|_, cx| {
            let demo = demo.read(cx);
            let loader = demo.loader.read(cx);
            assert_eq!(demo.status, LoaderStatus::Loading);
            assert_eq!(loader.status(), LoaderStatus::Loading);
            assert_eq!(loader.variant(), LoaderVariant::Ring);
        });
        // The loading loop must still request frames, after the obsolete reset timer expires.
        let callbacks = window.update(|window, cx| window.simulate_next_frame(cx));
        assert!(
            callbacks > 0,
            "loading loop stopped after the result view was cached"
        );
    }
}
