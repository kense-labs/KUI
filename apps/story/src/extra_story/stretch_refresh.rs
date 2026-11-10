use gpui::{prelude::*, *};
use kui::{
    components::stretch_refresh::{StretchRefresh, StretchRefreshError},
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
        menu::MenuScrollRegion,
    },
};
use std::{sync::Arc, time::Duration};

pub const IDS: &[&str] = &["stretch-refresh"];
pub const PREVIEW_HEIGHT: f32 = 520.;

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

type Copy = (&'static str, &'static str);

#[derive(Clone, Copy)]
enum Avatar {
    Gradient(u32, u32, u32),
    Symbol(Glyph, u32),
}

#[derive(Clone)]
struct Row {
    key: &'static str,
    avatar: Avatar,
    name: Copy,
    verb: Copy,
    subject: Copy,
    meta: Copy,
    quote: Option<Copy>,
    photo: bool,
    fresh: bool,
}

fn existing() -> Vec<Row> {
    use Avatar::*;
    vec![
        Row {
            key: "merge-405",
            avatar: Gradient(0xf8ca55, 0x6e8bd6, 0xec639e),
            name: ("Nora Chen", "陈诺"),
            verb: (" merged ", "合并了"),
            subject: ("Retry failed payments automatically", "自动重试失败付款"),
            meta: ("Checkout · 14 min ago", "结账 · 14 分钟前"),
            quote: None,
            photo: false,
            fresh: false,
        },
        Row {
            key: "build-2420",
            avatar: Symbol(Glyph::Check, 0x59cf9d),
            name: ("Build 2420", "构建 2420"),
            verb: (" passed on ", "已通过"),
            subject: ("staging", "预发布环境"),
            meta: ("Deploys · 22 min ago", "部署 · 22 分钟前"),
            quote: None,
            photo: false,
            fresh: false,
        },
        Row {
            key: "roadmap",
            avatar: Gradient(0xd854c5, 0x66dd9d, 0xcda0f1),
            name: ("Theo Martin", "西奥·马丁"),
            verb: (" commented on ", "评论了"),
            subject: ("Q4 planning notes", "第四季度计划"),
            meta: ("Planning · 38 min ago", "规划 · 38 分钟前"),
            quote: Some((
                "“Can we move search up a week? The new filters depend on it.”",
                "“可以把搜索提前一周吗？新的筛选功能依赖它。”",
            )),
            photo: false,
            fresh: false,
        },
        Row {
            key: "invoice-2041",
            avatar: Symbol(Glyph::FileText, 0xa7a7a7),
            name: ("Marlow Supply", "玛洛供应"),
            verb: (" paid invoice ", "支付了发票 "),
            subject: ("INV-2041", "INV-2041"),
            meta: ("$4,720.00 · 1 hr ago", "$4,720.00 · 1 小时前"),
            quote: None,
            photo: false,
            fresh: false,
        },
        Row {
            key: "interior",
            avatar: Gradient(0x819de5, 0xf5b97c, 0xba82d8),
            name: ("Maya Park", "朴美雅"),
            verb: (" shared an artwork for ", "分享了一幅画到"),
            subject: ("Spring lookbook", "春季图集"),
            meta: ("Brand · 2 hr ago", "品牌 · 2 小时前"),
            quote: None,
            photo: true,
            fresh: false,
        },
        Row {
            key: "calendar",
            avatar: Gradient(0xb474e0, 0xa5d984, 0x5d81a1),
            name: ("Eli Moore", "伊莱·摩尔"),
            verb: (" closed ", "关闭了"),
            subject: ("Calendar focus jumps in Safari", "Safari 日历焦点跳动"),
            meta: ("Web app · 3 hr ago", "网页应用 · 3 小时前"),
            quote: None,
            photo: false,
            fresh: false,
        },
        Row {
            key: "notes",
            avatar: Gradient(0xd9ed88, 0x73d2c9, 0xc8a475),
            name: ("Ari Flores", "阿里·弗洛雷斯"),
            verb: (" published ", "发布了"),
            subject: ("September release notes", "九月发布说明"),
            meta: ("Docs · Yesterday", "文档 · 昨天"),
            quote: None,
            photo: false,
            fresh: false,
        },
    ]
}

fn incoming() -> Vec<Row> {
    use Avatar::*;
    vec![
        Row {
            key: "reply-406",
            avatar: Gradient(0xb474e0, 0xa5d984, 0x5d81a1),
            name: ("Eli Moore", "伊莱·摩尔"),
            verb: (" replied on ", "回复了"),
            subject: ("Retry failed payments automatically", "自动重试失败付款"),
            meta: ("Checkout · Just now", "结账 · 刚刚"),
            quote: Some(("“Ready to ship after lunch.”", "“午餐后可以发布。”")),
            photo: false,
            fresh: true,
        },
        Row {
            key: "build-2421",
            avatar: Symbol(Glyph::Close, 0xe777a1),
            name: ("Build 2421", "构建 2421"),
            verb: (" failed on ", "未通过"),
            subject: ("main", "主分支"),
            meta: ("2 checkout tests · Just now", "2 个结账测试 · 刚刚"),
            quote: None,
            photo: false,
            fresh: true,
        },
        Row {
            key: "search-review",
            avatar: Gradient(0xd9ed88, 0x73d2c9, 0xc8a475),
            name: ("Ari Flores", "阿里·弗洛雷斯"),
            verb: (" requested a review of ", "请求审阅"),
            subject: ("Search filters", "搜索筛选"),
            meta: ("Web app · Just now", "网页应用 · 刚刚"),
            quote: None,
            photo: false,
            fresh: true,
        },
    ]
}

fn local(lang: UiLanguage, pair: Copy) -> &'static str {
    lang.text(pair.0, pair.1)
}

struct Demo {
    view: Entity<StretchRefresh<Row>>,
    can_reset: bool,
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let language = UiLanguage::current(cx);
        let demo = cx.weak_entity();
        let photo: ImageSource = Arc::new(Image::from_bytes(
            ImageFormat::Jpeg,
            include_bytes!("../../assets/navigation/room-1.jpg").to_vec(),
        ))
        .into();
        Self {
            can_reset: false,
            view: cx.new(|cx| {
                StretchRefresh::new(
                    language.text("Workspace activity", "工作区动态"),
                    existing(),
                    |row: &Row| row.key.into(),
                    move |row, _, cx| paint_row(row, photo.clone(), cx).into_any_element(),
                    move |_, cx| {
                        let demo = demo.clone();
                        let timer = cx.background_executor().timer(Duration::from_millis(910));
                        cx.spawn(async move |view, cx| {
                            timer.await;
                            let _ = demo.update(cx, |demo, cx| {
                                demo.can_reset = true;
                                cx.notify();
                            });
                            view.update(cx, |view, cx| {
                                let lang = UiLanguage::current(cx);
                                view.set_subtitle(lang.text("Updated just now", "刚刚更新"), cx);
                                if view.items().any(|row| row.fresh) {
                                    return None;
                                }
                                let mut rows = incoming();
                                rows.extend(view.items().cloned());
                                view.set_items(rows, cx);
                                Some(lang.text("3 new updates", "3 条新动态").into())
                            })
                            .map_err(|_| StretchRefreshError("Preview closed".into()))
                        })
                    },
                    cx,
                )
                .with_subtitle(language.text("Updated 14 min ago", "14 分钟前更新"))
                .with_refresh_label(language.text("Refresh activity", "刷新动态"))
                .with_scroll_region(|bounds, before, after, cx| {
                    if let Some(host) = cx.try_global::<MenuScrollRegion>() {
                        (host.0)(bounds, before, after);
                    }
                })
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
            }),
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = Theme::global(cx).tokens.radius.md;
        let lang = UiLanguage::current(cx);
        let name = lang.text("Reset", "重置");
        let reset = face(
            "stretch-refresh-reset",
            name,
            32.,
            f32::from(radius),
            Tone::Clear,
            cx,
        )
        .debug_selector(|| "stretch-refresh-reset".into())
        .px(px(12.))
        .border_1()
        .border_color(p.line)
        .text_color(p.secondary)
        .tab_stop(self.can_reset)
        .opacity(if self.can_reset { 1. } else { 0.48 })
        .child(name)
        .when(self.can_reset, |e| {
            e.hover(move |s| s.bg(p.hover))
                .on_click(cx.listener(move |demo, _, _, cx| {
                    if demo.view.read(cx).is_refreshing() {
                        return;
                    }
                    demo.view.update(cx, |view, cx| {
                        view.set_items(existing(), cx);
                        view.set_subtitle(lang.text("Updated 14 min ago", "14 分钟前更新"), cx);
                        view.scroll_to_top(cx);
                    });
                    demo.can_reset = false;
                    cx.notify();
                }))
        });
        div()
            .w_full()
            .h(px(PREVIEW_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .px(px(8.))
            .child(
                div()
                    .w_full()
                    .max_w(px(376.))
                    .h(px(392.))
                    .flex_none()
                    .child(self.view.clone()),
            )
            .child(reset)
    }
}

fn paint_row(row: &Row, photo: ImageSource, cx: &App) -> Div {
    let lang = UiLanguage::current(cx);
    let p = Paint::get(cx);
    let radius = Theme::global(cx).tokens.radius.md;
    let name = local(lang, row.name);
    let verb = local(lang, row.verb);
    let subject = local(lang, row.subject);
    let full: SharedString = format!("{name}{verb}{subject}").into();
    let neutral = HighlightStyle {
        color: Some(p.secondary),
        ..Default::default()
    };
    let styled =
        StyledText::new(full).with_highlights([(name.len()..name.len() + verb.len(), neutral)]);
    let badge = div()
        .size(px(32.))
        .flex_none()
        .rounded_full()
        .overflow_hidden();
    let badge = match row.avatar {
        Avatar::Gradient(a, b, c) => badge
            .bg(linear_gradient(
                138.,
                linear_color_stop(rgb(a), 0.),
                linear_color_stop(rgb(b), 0.62),
            ))
            .child(div().size_full().rounded_full().bg(linear_gradient(
                38.,
                linear_color_stop(rgb(c), 0.),
                linear_color_stop(transparent_black(), 0.68),
            ))),
        Avatar::Symbol(glyph, ink) => {
            let ink = rgb(ink).into();
            let terminal = if glyph == Glyph::FileText {
                icon(glyph, 17., p.secondary).into_any_element()
            } else {
                div()
                    .size(px(17.))
                    .rounded_full()
                    .border_1()
                    .border_color(ink)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(glyph, 11., ink))
                    .into_any_element()
            };
            badge.flex().items_center().justify_center().child(terminal)
        }
    };
    let content = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(3.))
        .child(div().text_size(px(14.)).line_height(px(20.)).child(styled))
        .when_some(row.quote, |e, quote| {
            e.child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .child(local(lang, quote)),
            )
        })
        .when(row.photo, |e| {
            e.child(
                div()
                    .id("stretch-room-image")
                    .role(Role::Image)
                    .aria_label(lang.text(
                        "Painting of an interior beside a fireplace",
                        "一幅描绘壁炉旁室内场景的画",
                    ))
                    .w_full()
                    .h(px(190.))
                    .rounded(radius)
                    .overflow_hidden()
                    .child(img(photo).size_full().object_fit(ObjectFit::Cover)),
            )
        })
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(p.faint)
                .child(local(lang, row.meta)),
        );
    div()
        .mx(px(20.))
        .py(px(11.))
        .flex()
        .items_start()
        .gap(px(12.))
        .border_b_1()
        .border_color(p.line.opacity(0.72))
        .child(badge)
        .child(content)
        .when(row.fresh, |e| {
            e.child(
                div()
                    .mt(px(7.))
                    .size(px(8.))
                    .flex_none()
                    .rounded_full()
                    .bg(p.secondary),
            )
        })
}
