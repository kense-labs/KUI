use gpui::{prelude::*, *};
use kui::{
    components::dot_grid::{DotGrid, DotGridOptions},
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
    },
};

pub const IDS: &[&str] = &["dot-grid"];
const GAPS: [f32; 3] = [14., 22., 32.];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Density {
    selected: usize,
    grid: WeakEntity<DotGrid>,
}

impl Render for Density {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let lang = UiLanguage::current(cx);
        let labels = [
            lang.text("Dense", "紧密"),
            lang.text("Default", "默认"),
            lang.text("Loose", "疏朗"),
        ];
        div()
            .id("dot-grid-density")
            .role(Role::Group)
            .aria_label(lang.text("Density", "点阵密度"))
            .h(px(36.))
            .w(px(234.))
            .flex()
            .items_center()
            .p(px(2.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel.opacity(0.8))
            .children(labels.into_iter().enumerate().map(|(i, name)| {
                let selected = self.selected == i;
                face(
                    ("dot-density", i),
                    name,
                    30.,
                    f32::from(radius),
                    Tone::Clear,
                    cx,
                )
                .w(px(76.))
                .aria_toggled(selected.into())
                .text_size(px(14.))
                .font_weight(FontWeight::NORMAL)
                .text_color(if selected { p.ink } else { p.secondary })
                .border_1()
                .border_color(if selected {
                    p.line
                } else {
                    transparent_black()
                })
                .hover(move |e| e.text_color(p.ink))
                .child(name)
                .on_click(cx.listener(move |s, _, _, cx| {
                    if s.selected == i {
                        return;
                    }
                    s.selected = i;
                    let _ = s.grid.update(cx, |grid, cx| {
                        grid.set_options(
                            DotGridOptions {
                                gap: GAPS[i],
                                ..Default::default()
                            },
                            cx,
                        )
                    });
                    cx.notify();
                }))
            }))
    }
}

struct Demo(Entity<DotGrid>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        Self(cx.new(|cx| {
            let weak = cx.weak_entity();
            let buttons = cx.new(|_| Density {
                selected: 1,
                grid: weak,
            });
            DotGrid::new(DotGridOptions::default(), cx)
                .with_visibility(
                    |cx| {
                        cx.try_global::<crate::StoryBridge>()
                            .is_none_or(|b| (b.visible)())
                    },
                    cx,
                )
                .with_style(|e| e.h_full().flex().justify_center().items_end().pb(px(20.)))
                .with_content(move |_, _| buttons.clone())
        }))
    }
}

impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let narrow = w.viewport_size().width < px(480.);
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        div()
            .w_full()
            .max_w(px(572.))
            .px(px(if narrow { 12. } else { 22. }))
            .py(px(12.))
            .child(
                div()
                    .w_full()
                    .h(px(if narrow { 380. } else { 420. }))
                    .bg(p.panel)
                    .rounded(radius)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .child(self.0.clone()),
            )
    }
}
