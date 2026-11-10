//! Localized gallery and sample reading surfaces.
use gpui::{prelude::*, *};
use kui::{
    components::card_to_article::{ArticleItem, CardToArticle},
    primitives::{control::UiLanguage, family::Paint},
};
use std::sync::Arc;

pub const IDS: &[&str] = &["card-to-article"];
pub const PREVIEW_HEIGHT: f32 = 520.;

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Story {
    id: &'static str,
    title: [&'static str; 2],
    alt: [&'static str; 2],
    tag: [&'static str; 2],
    meta: [&'static str; 2],
    opening: [&'static str; 2],
    photo: &'static [u8],
}

const STORIES: [Story; 4] = [
    Story {
        id: "pool-house",
        title: [
            "A pool house built for bright summers",
            "为明亮夏日建造的泳池住宅",
        ],
        alt: [
            "A glass house reflected in a pool",
            "倒映在泳池里的玻璃住宅",
        ],
        tag: ["Case study", "案例"],
        meta: ["5 min read", "阅读约 5 分钟"],
        opening: [
            "Deep terraces shade the rooms before the afternoon sun reaches the glass. The pool sets the distance between the garden and the building.",
            "宽阔的露台挡住下午照向玻璃的阳光。泳池把花园与住宅隔开，留下足够的空间。",
        ],
        photo: include_bytes!("../../assets/places/pool-house.jpg"),
    },
    Story {
        id: "lisbon-tram",
        title: ["Across Lisbon by tram", "乘电车穿越里斯本"],
        alt: [
            "A yellow tram between tiled buildings",
            "瓷砖建筑之间的黄色电车",
        ],
        tag: ["Travel", "旅行"],
        meta: ["4 min read", "阅读约 4 分钟"],
        opening: [
            "The first tram rounds the corner before the cafés open. A short ride traces the slope of the hill from the old streets to the river.",
            "第一班电车在咖啡店开门前转过街角。短短一程，沿着山坡从旧街一路开向河边。",
        ],
        photo: include_bytes!("../../assets/places/tram.jpg"),
    },
    Story {
        id: "dinner-menu",
        title: ["A menu for the day's catch", "随当天渔获变化的菜单"],
        alt: ["A plated meal on a wood table", "木桌上的一盘菜"],
        tag: ["Changelog", "更新"],
        meta: ["2 min read", "阅读约 2 分钟"],
        opening: [
            "The kitchen rewrites the menu each morning. Ingredients that arrive in small batches can sell out before the last seating.",
            "厨房每天早上重写菜单。数量不多的食材，可能在最后一桌客人到来之前就卖完。",
        ],
        photo: include_bytes!("../../assets/places/dinner.jpg"),
    },
    Story {
        id: "atlantic-seasons",
        title: [
            "A coast that changes with the seasons",
            "随季节变换的大西洋海岸",
        ],
        alt: [
            "Atlantic waves breaking at sunset",
            "日落时拍打礁石的大西洋海浪",
        ],
        tag: ["Essay", "随笔"],
        meta: ["6 min read", "阅读约 6 分钟"],
        opening: [
            "In summer the path stays open until dusk. By autumn the water moves higher up the cliffs and the same walk feels completely different.",
            "夏天，小径直到黄昏仍可通行。入秋后，海浪拍得更高，同一条路也有了完全不同的感受。",
        ],
        photo: include_bytes!("../../assets/places/coast.jpg"),
    },
];

struct Demo(Entity<CardToArticle>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let zh = usize::from(UiLanguage::current(cx) == UiLanguage::Chinese);
        let articles = STORIES
            .iter()
            .map(|story| {
                ArticleItem::new(
                    story.id,
                    story.title[zh],
                    Arc::new(Image::from_bytes(ImageFormat::Jpeg, story.photo.to_vec())),
                    story.alt[zh],
                    move |_, cx| article(story, zh, cx),
                )
                .with_tag(story.tag[zh])
                .with_meta(story.meta[zh])
            })
            .collect();
        Self(cx.new(|cx| CardToArticle::new(articles, cx).with_aspect_ratio(1.42)))
    }
}

fn article(story: &Story, zh: usize, cx: &App) -> AnyElement {
    let p = Paint::get(cx);
    let body = [
        story.opening[zh],
        [
            "Look from one edge of the photograph to the opposite edge. Repeated shapes connect the foreground with the details in the distance.",
            "从照片的一边看向另一边。重复的形状把近处和远处的细节连在一起。",
        ][zh],
        [
            "Small details help establish distance: a brighter edge can bring a nearby object forward, while softer marks let the background recede. The technique is easiest to see if you compare two objects with roughly the same shape.",
            "小细节有助于建立距离感。更亮的边缘让近处的物件突出，柔和的笔触让背景退后。比较两个形状相近的物件，就容易看出这种手法。",
        ][zh],
        [
            "Step back and look at the whole scene again. The smallest details make more sense when they are seen alongside the rest of the photograph.",
            "最后再看一遍整个房间。熟悉的物件在其中有了更清楚的位置。从最小的细节回到整体构图，这个尺度的变化让同一幅画值得多看几遍。",
        ][zh],
    ];
    div()
        .flex()
        .flex_col()
        .gap(px(18.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(14.))
                .child(
                    div()
                        .size(px(28.))
                        .rounded_full()
                        .bg(p.tray)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("K"),
                )
                .child(["Kense Journal", "Kense 读刊"][zh])
                .child(
                    div()
                        .text_color(p.faint)
                        .child(["Nov 10, 2026", "2026 年 11 月 10 日"][zh]),
                ),
        )
        .child(body[0])
        .child(body[1])
        .child(
            div()
                .border_l_2()
                .border_color(p.line)
                .pl(px(14.))
                .text_size(px(22.))
                .line_height(px(31.))
                .child(
                    [
                        "Look for the places where the eye naturally pauses.",
                        "留意目光自然停下的地方。",
                    ][zh],
                ),
        )
        .child(body[2])
        .child(body[3])
        .into_any_element()
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .px(px(26.))
            .py(px(24.))
            .flex()
            .justify_center()
            .child(div().w_full().max_w(px(522.)).child(self.0.clone()))
    }
}
