use gpui::{prelude::*, *};
use kui::{
    components::skeleton_morph::{MorphBlock, MorphLayout, MorphRadius, SkeletonMorph},
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};

pub const IDS: &[&str] = &["skeleton-morph"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Demo {
    profile: Entity<SkeletonMorph>,
    payouts: Entity<SkeletonMorph>,
    loading: bool,
    timer: Option<Task<()>>,
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let language = UiLanguage::current(cx);
        let profile = cx.new(|cx| {
            let follow = Rc::new(Cell::new(false));
            let view = cx.weak_entity();
            style(
                SkeletonMorph::new(
                    "skeleton-morph-profile",
                    true,
                    move |layout| profile(layout, language, &follow, &view).into_any_element(),
                    cx,
                ),
                language.text("Loading profile", "正在加载资料"),
            )
        });
        let payouts = cx.new(|cx| {
            style(
                SkeletonMorph::new(
                    "skeleton-morph-payouts",
                    true,
                    move |layout| payouts(layout, language).into_any_element(),
                    cx,
                ),
                language.text("Loading recent payouts", "正在加载近期打款"),
            )
        });
        let mut demo = Self {
            profile,
            payouts,
            loading: false,
            timer: None,
        };
        demo.reload(cx);
        demo
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        for card in [&self.profile, &self.payouts] {
            card.update(cx, |card, cx| card.set_loading(true, cx));
        }
        let delay = cx.background_executor().timer(Duration::from_millis(1420));
        self.timer = Some(cx.spawn(async move |weak, cx| {
            delay.await;
            let _ = weak.update(cx, |demo, cx| {
                demo.timer = None;
                demo.loading = false;
                for card in [&demo.profile, &demo.payouts] {
                    card.update(cx, |card, cx| card.set_loading(false, cx));
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}

fn style(card: SkeletonMorph, label: &'static str) -> SkeletonMorph {
    card.with_loading_label(label)
        .with_style(|e| {
            // Card color is inherited from the preview theme through the
            // wrapping slot so the component itself stays style-neutral.
            e.p(px(20.))
        })
        .with_visibility(|cx| {
            cx.try_global::<crate::StoryBridge>()
                .is_none_or(|bridge| (bridge.visible)())
        })
}

impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let lang = UiLanguage::current(cx);
        let stacked = w.viewport_size().width < px(608.);
        let slot = |view: Entity<SkeletonMorph>| {
            div()
                .w_full()
                .max_w(px(312.))
                .when(!stacked, |e| e.flex_1())
                .min_w_0()
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .rounded(radius)
                .overflow_hidden()
                .child(view)
        };
        let reload = lang.text("Reload", "重新加载");
        let key = face(
            "skeleton-reload",
            reload,
            36.,
            f32::from(radius),
            Tone::Surface,
            cx,
        )
        .debug_selector(|| "skeleton-reload".into())
        .w(px(if lang == UiLanguage::Chinese {
            112.
        } else {
            98.
        }))
        .border_1()
        .border_color(p.line)
        .text_color(p.ink)
        .opacity(if self.loading { 0.44 } else { 1. })
        .when(!self.loading, |e| e.hover(move |e| e.bg(p.hover)))
        .child(icon(Glyph::Redo, 14., p.secondary))
        .child(reload)
        .on_click(cx.listener(|s, _, _, cx| s.reload(cx)));
        div()
            .w_full()
            .px(px(8.))
            .py(px(if stacked { 22. } else { 32. }))
            .min_h(px(if stacked { 0. } else { 500. }))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(22.))
            .child(
                div()
                    .w_full()
                    .max_w(px(640.))
                    .flex()
                    .when(stacked, |e| e.flex_col().items_center())
                    .when(!stacked, |e| e.items_start().justify_center())
                    .gap(px(16.))
                    .child(slot(self.profile.clone()))
                    .child(slot(self.payouts.clone())),
            )
            .child(key)
    }
}

fn type_line(id: &'static str, width: f32, height: f32) -> MorphBlock {
    MorphBlock::new(id)
        .with_width(px(width))
        .with_lines(1)
        .with_line_height(px(height))
}

fn avatar_block(id: impl Into<ElementId>, diameter: f32) -> MorphBlock {
    MorphBlock::new(id)
        .with_width(px(diameter))
        .with_height(px(diameter))
        .with_radius(MorphRadius::Circle)
}

fn avatar(diameter: f32, [a, b, c]: [u32; 3]) -> Div {
    div()
        .size(px(diameter))
        .flex_none()
        .rounded_full()
        .bg(linear_gradient(
            138.,
            linear_color_stop(rgb(a), 0.),
            linear_color_stop(rgb(b), 0.92),
        ))
        .child(div().size_full().rounded_full().bg(linear_gradient(
            35.,
            linear_color_stop(rgb(c), 0.),
            linear_color_stop(transparent_black(), 0.64),
        )))
}

fn label(text: impl Into<SharedString>, font: f32, height: f32, color: Hsla) -> Div {
    div()
        .min_w_0()
        .max_w_full()
        .text_size(px(font))
        .line_height(px(height))
        .text_color(color)
        .child(text.into())
}

fn profile(
    layout: &mut MorphLayout<'_>,
    lang: UiLanguage,
    following: &Rc<Cell<bool>>,
    weak: &WeakEntity<SkeletonMorph>,
) -> Div {
    let image = layout.block(avatar_block("profile-avatar", 52.), |_, _| {
        avatar(52., [0x8ee6a3, 0xe35bd5, 0x71c9a6]).into_any_element()
    });
    let name = layout.block(type_line("profile-name", 124., 20.), |_, cx| {
        label(
            lang.text("Riley Morgan", "莱利·摩根"),
            16.,
            20.,
            Paint::get(cx).ink,
        )
        .font_weight(FontWeight::MEDIUM)
        .into_any_element()
    });
    let role = layout.block(type_line("profile-role", 164., 18.), |_, cx| {
        label(
            lang.text("Design lead at Northbank", "北岸设计负责人"),
            14.,
            18.,
            Paint::get(cx).secondary,
        )
        .into_any_element()
    });
    let bio = layout.block(MorphBlock::new("profile-bio").with_lines(3), |_, cx| {
        label(
            lang.text(
                "Designs thoughtful tools for busy teams. Now improving how Northbank plans and shares its work.",
                "为忙碌的团队设计易用工具，正在改善北岸团队的工作安排和协作体验。",
            ),
            14., 20., Paint::get(cx).secondary,
        ).into_any_element()
    });
    let stats = layout.block(
        type_line("profile-stats", 169., 20.).with_style(|e| e.flex_1().min_w_0()),
        |_, cx| {
            let ink = Paint::get(cx);
            let copy = lang.text("2.4k followers · 184 posts", "2.4k 关注者 · 184 条动态");
            label(copy, 14., 20., ink.secondary).into_any_element()
        },
    );
    let follow = layout.block(
        MorphBlock::new("profile-action")
            .with_width(px(90.))
            .with_height(px(36.)),
        |_, cx| {
            let active = following.get();
            let following = following.clone();
            let weak = weak.clone();
            let radius = Theme::global(cx).tokens.radius.md;
            let word = if active {
                lang.text("Following", "已关注")
            } else {
                lang.text("Follow", "关注")
            };
            face(
                "skeleton-morph-follow",
                word,
                36.,
                f32::from(radius),
                if active { Tone::Surface } else { Tone::Primary },
                cx,
            )
            .debug_selector(|| "skeleton-morph-follow".into())
            .w(px(90.))
            .border_1()
            .border_color(if active {
                Paint::get(cx).line
            } else {
                transparent_black()
            })
            .aria_toggled(active.into())
            .child(word)
            .on_click(move |_, _, cx| {
                following.set(!following.get());
                let _ = weak.update(cx, |_, cx| cx.notify());
            })
            .into_any_element()
        },
    );
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(image)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(2.))
                        .child(name)
                        .child(role),
                ),
        )
        .child(bio)
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.))
                .child(stats)
                .child(follow),
        )
}

struct Payout {
    name: (&'static str, &'static str),
    note: (&'static str, &'static str),
    amount: &'static str,
    colors: [u32; 3],
}

const PAYOUTS: [Payout; 3] = [
    Payout {
        name: ("Morgan Lee", "摩根·李"),
        note: ("Paid today", "今天已支付"),
        amount: "$1,240",
        colors: [0x66e4bd, 0xed5f9d, 0x68deca],
    },
    Payout {
        name: ("Avery Tran", "艾弗里·陈"),
        note: ("Paid Sep 22", "9 月 22 日支付"),
        amount: "$860",
        colors: [0xa4dc55, 0xc3dc66, 0x7854d4],
    },
    Payout {
        name: ("Noah Kim", "诺亚·金"),
        note: ("Scheduled Sep 30", "定于 9 月 30 日"),
        amount: "$2,115",
        colors: [0x8be1e0, 0x3addd1, 0xe28383],
    },
];

fn payouts(layout: &mut MorphLayout<'_>, lang: UiLanguage) -> Stateful<Div> {
    let title = layout.block(
        type_line("payout-title", 100., 20.).with_role(Role::Heading),
        |_, cx| {
            label(
                lang.text("Recent payouts", "近期打款"),
                14.,
                20.,
                Paint::get(cx).ink,
            )
            .font_weight(FontWeight::MEDIUM)
            .into_any_element()
        },
    );
    let mut rows = div()
        .id("skeleton-payout-list")
        .role(Role::List)
        .w_full()
        .flex()
        .flex_col()
        .gap(px(14.));
    for (i, row) in PAYOUTS.iter().enumerate() {
        let marker = layout.block(avatar_block(("payout-avatar", i), 36.), |_, _| {
            avatar(36., row.colors).into_any_element()
        });
        let name = layout.block(
            MorphBlock::new(("payout-name", i))
                .with_width(px(104.))
                .with_lines(1)
                .with_line_height(px(18.)),
            |_, cx| {
                label(
                    lang.text(row.name.0, row.name.1),
                    14.,
                    18.,
                    Paint::get(cx).ink,
                )
                .into_any_element()
            },
        );
        let note = layout.block(
            MorphBlock::new(("payout-note", i))
                .with_width(px(76.))
                .with_lines(1)
                .with_line_height(px(16.)),
            |_, cx| {
                label(
                    lang.text(row.note.0, row.note.1),
                    12.,
                    16.,
                    Paint::get(cx).faint,
                )
                .into_any_element()
            },
        );
        let amount = layout.block(
            MorphBlock::new(("payout-amount", i))
                .with_width(px(44.))
                .with_lines(1)
                .with_line_height(px(18.)),
            |_, cx| label(row.amount, 14., 18., Paint::get(cx).ink).into_any_element(),
        );
        rows = rows.child(
            div()
                .id(("payout-row", i))
                .role(Role::ListItem)
                .w_full()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(marker)
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(2.))
                        .child(name)
                        .child(note),
                )
                .child(amount),
        );
    }
    div()
        .id("skeleton-payouts-layout")
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(16.))
        .child(title)
        .child(rows)
}
