//! Localized gallery and sample reading surfaces.
use gpui::{prelude::*, *};
use kui::{
    components::card_to_article::{ArticleItem, CardToArticle},
    primitives::{control::UiLanguage, family::Paint},
};
use std::sync::Arc;

pub const IDS: &[&str] = &["card-to-article"];
pub const PREVIEW_HEIGHT: f32 = 592.;

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
        id: "quiet-room",
        title: ["A quiet room in afternoon light", "午后光线里的安静房间"],
        alt: [
            "A painting of an interior with a window",
            "一幅带窗户的室内画",
        ],
        tag: ["Case study", "案例"],
        meta: ["5 min read", "阅读约 5 分钟"],
        opening: [
            "A room can feel different as the sun moves across it. At midday, the objects at the window are clear, while those further inside almost disappear into the wall. The painting follows that change across the floor.",
            "阳光移动时，一间房也会变得不同。正午时，靠窗的物件清晰可见，屋子深处的物件几乎融入墙面。画家把这一变化留在地板上。",
        ],
        photo: include_bytes!("../../assets/navigation/room-0.jpg"),
    },
    Story {
        id: "under-dome",
        title: ["An afternoon under the dome", "圆顶下的一个下午"],
        alt: [
            "A painting of the interior of the Pantheon",
            "一幅罗马万神殿内部的画",
        ],
        tag: ["Travel", "旅行"],
        meta: ["4 min read", "阅读约 4 分钟"],
        opening: [
            "From the doorway, the room seems to hold a single pool of light. After a few steps the scale changes. Look upward and the small square openings around the dome begin to make sense as a repeating pattern.",
            "从门口望去，殿内像是盛着一片光。往前走几步，空间的尺度便不同了。抬头时，圆顶上的方格开始显出重复的规律。",
        ],
        photo: include_bytes!("../../assets/navigation/room-3.jpg"),
    },
    Story {
        id: "drawing-room",
        title: ["Notes from the drawing room", "会客厅里的观察笔记"],
        alt: [
            "A painting of a family in a drawing room",
            "一幅描绘家人在会客厅相聚的画",
        ],
        tag: ["Notebook", "笔记"],
        meta: ["2 min read", "阅读约 2 分钟"],
        opening: [
            "The people in this room are arranged around their conversation. Start with the figure nearest the chair and follow the direction of each glance. You can trace the exchange without reading a word.",
            "室内人物的位置围绕着一场谈话。可以先看椅子旁的人，再顺着每一道视线观察。不读一句文字，也能看出他们在怎样交流。",
        ],
        photo: include_bytes!("../../assets/navigation/room-1.jpg"),
    },
    Story {
        id: "slow-looking",
        title: ["What we notice when we look slowly", "慢一点看，会发现什么"],
        alt: [
            "A painting of the interior of a synagogue",
            "一幅描绘会堂内部的画",
        ],
        tag: ["Essay", "随笔"],
        meta: ["6 min read", "阅读约 6 分钟"],
        opening: [
            "The details near the bottom of the frame are easy to overlook. Give them a moment, and the composition starts to show how a visitor would cross the room, where they might stop, and what they would see first.",
            "画面底部的细节很容易被忽略。多看一会儿，构图就会告诉你访客如何穿过房间、在哪里停下，以及他们最先看见什么。",
        ],
        photo: include_bytes!("../../assets/navigation/room-5.jpg"),
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
        Self(cx.new(|cx| CardToArticle::new(articles, cx)))
    }
}

fn article(story: &Story, zh: usize, cx: &App) -> AnyElement {
    let p = Paint::get(cx);
    let body = [
        story.opening[zh],
        [
            "Look at one edge of the painting, then the opposite edge. Repeated shapes link the two sides, even where their colors or sizes differ. The scene feels like it continues beyond the frame because the painter repeats those shapes at different depths.",
            "先看画面的一边，再看另一边。即使颜色和大小不同，重复的形状仍把两边连在一起。画家在不同深度重复这些形状，让场景像是延伸到了画框之外。",
        ][zh],
        [
            "Small details help establish distance: a brighter edge can bring a nearby object forward, while softer marks let the background recede. The technique is easiest to see if you compare two objects with roughly the same shape.",
            "小细节有助于建立距离感。更亮的边缘让近处的物件突出，柔和的笔触让背景退后。比较两个形状相近的物件，就容易看出这种手法。",
        ][zh],
        [
            "The last step is to look at the whole room again. Familiar objects now occupy clearer places in it. That change of scale, from the smallest detail back to the entire composition, is why the same picture can reward several visits.",
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
