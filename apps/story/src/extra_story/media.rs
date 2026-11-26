//! Interactive examples for text and media primitives.
use gpui::{prelude::*, *};
use kui::{
    components::{
        bottom_sheet::BottomSheet,
        card_stack::{CardStack, StackCard, StackFact},
        carousel::{Carousel, CarouselSlide},
        image_compare::{CompareOrientation, ImageCompare},
        in_view_title::{InViewTitle, TitleEntrance},
        product_gallery::{ProductColor, ProductGallery, ProductPhoto, ProductSize},
        segmented_control::{FormOption, SegmentedControl, SelectionChanged},
        sheet_stack::{SheetLink, SheetPage, SheetStack, SheetStackPresentation},
        slot_text::SlotText,
        text_morph::TextMorph,
        text_reveal::TextReveal,
        text_shimmer::TextShimmer,
        toast_stack::{ToastId, ToastStack},
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
        nav::button,
    },
};
use std::sync::Arc;

pub const IDS: &[&str] = &[
    "text-reveal",
    "in-view-title",
    "text-morph",
    "text-shimmer",
    "slot-text",
    "image-compare",
    "carousel",
    "card-stack",
    "toast-stack",
    "bottom-sheet",
    "sheet-stack",
    "product-gallery",
];

pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Demo::new(id, cx)).into())
}

enum Example {
    Reveal(Entity<TextReveal>),
    Title {
        view: Entity<InViewTitle>,
        mode: TitleEntrance,
    },
    Morph(Entity<TextMorph>),
    Shimmer {
        primary: Entity<TextShimmer>,
        secondary: Entity<TextShimmer>,
        active: bool,
    },
    Slot {
        view: Entity<SlotText>,
        period: usize,
    },
    Compare(Entity<ImageCompare>),
    Carousel(Entity<Carousel>),
    CardStack(Entity<CardStack>),
    ToastStack {
        stack: Entity<ToastStack>,
        ids: Vec<ToastId>,
        seq: usize,
    },
    BottomSheet(Entity<BottomSheet>),
    SheetStack(Entity<SheetStack>),
    ProductGallery(Entity<ProductGallery>),
}
/// Live bag counter for the product-gallery header; owned as its own entity
/// so the add button can notify it directly from a plain `&mut App` callback.
struct BagPill {
    count: u32,
}
impl Render for BagPill {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        div()
            .id("bag-count")
            .role(Role::Status)
            .aria_label(if zh {
                format!("购物袋中有 {} 件", self.count)
            } else {
                format!("{} items in bag", self.count)
            })
            .flex()
            .items_center()
            .gap(px(6.))
            .h(px(32.))
            .px(px(10.))
            .rounded(kui::foundation::surface_radius(cx))
            .bg(p.panel)
            .text_color(p.secondary)
            .child(icon(Glyph::Bag, 16., p.secondary))
            .child(div().child(self.count.to_string()))
    }
}
struct Demo {
    content: Example,
    language: UiLanguage,
    step: usize,
    /// Orientation switch for the compare demo, built on first render.
    orientation: Option<Entity<SegmentedControl>>,
    sheet_presentation: SheetStackPresentation,
}
impl Demo {
    fn new(id: &str, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let content = match id {
            "text-reveal" => Example::Reveal(cx.new(|cx| {
                TextReveal::new(
                    t(
                        "Small details make\nevery interaction\nfeel considered.",
                        "微小的细节，\n让每一次交互\n都显得深思熟虑。",
                    ),
                    cx,
                )
                .with_heading_level(2)
            })),
            "in-view-title" => {
                let view = cx.new(|cx| {
                    let mut title =
                        InViewTitle::new(t("Make progress visible", "让每一处进展清晰可见"))
                            .with_entrance(TitleEntrance::Blur);
                    title.replay(cx);
                    title
                });
                Example::Title {
                    view,
                    mode: TitleEntrance::Blur,
                }
            }
            "text-morph" => {
                Example::Morph(cx.new(|_| TextMorph::new(t("Make it simple", "让想法发生"))))
            }
            "text-shimmer" => {
                let primary = cx.new(|cx| {
                    TextShimmer::new(t("Generating summary", "正在生成摘要"), cx)
                        .with_font_size(14.)
                });
                let secondary = cx.new(|cx| {
                    TextShimmer::new(t("Syncing 24 files", "正在同步 24 个文件"), cx)
                        .with_font_size(13.5)
                        .with_duration(std::time::Duration::from_millis(2200))
                });
                Example::Shimmer {
                    primary,
                    secondary,
                    active: true,
                }
            }
            "slot-text" => Example::Slot {
                view: cx.new(|_| SlotText::new("$31,904").with_font_size(28.)),
                period: 1,
            },
            "image-compare" => {
                let before: ImageSource = Arc::new(Image::from_bytes(
                    ImageFormat::Jpeg,
                    include_bytes!("../../assets/navigation/room-0.jpg").to_vec(),
                ))
                .into();
                let after: ImageSource = Arc::new(Image::from_bytes(
                    ImageFormat::Jpeg,
                    include_bytes!("../../assets/navigation/room-1.jpg").to_vec(),
                ))
                .into();
                Example::Compare(cx.new(|cx| {
                    ImageCompare::new(
                        before,
                        after,
                        t("Living room, raw and graded", "客厅，原图与成片对比"),
                        cx,
                    )
                    .with_captions(t("Before", "之前"), t("After", "之后"))
                }))
            }
            "carousel" => {
                let entries = [
                    (
                        include_bytes!("../../assets/navigation/room-0.jpg").as_slice(),
                        t("Oak lounge chair", "橡木休闲椅"),
                        t("Woven seat on a solid oak frame", "编织坐面，实心橡木框架"),
                        "$1,450",
                    ),
                    (
                        include_bytes!("../../assets/navigation/room-1.jpg").as_slice(),
                        t("Opal table lamp", "乳白台灯"),
                        t(
                            "Hand-blown glass shade, dimmable",
                            "手工吹制玻璃灯罩，可调光",
                        ),
                        "$640",
                    ),
                    (
                        include_bytes!("../../assets/navigation/room-2.jpg").as_slice(),
                        t("Stonewashed linen throw", "石洗亚麻毯"),
                        t("Oat, 130 × 180 cm", "燕麦色，130 × 180 厘米"),
                        "$180",
                    ),
                    (
                        include_bytes!("../../assets/navigation/room-0.jpg").as_slice(),
                        t("Ribbed glass carafe", "棱纹玻璃水瓶"),
                        t("Mouth-blown, holds 1 liter", "人工吹制，容量 1 升"),
                        "$95",
                    ),
                    (
                        include_bytes!("../../assets/navigation/room-1.jpg").as_slice(),
                        t("Sea at dusk", "暮色海景"),
                        t(
                            "Giclée print on cotton rag, 50 × 50 cm",
                            "棉浆艺术微喷，50 × 50 厘米",
                        ),
                        "$220",
                    ),
                ];
                let slides = entries
                    .into_iter()
                    .map(|(bytes, name, detail, price)| {
                        CarouselSlide::new(
                            Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())),
                            name,
                        )
                        .with_detail(detail)
                        .with_price(price)
                    })
                    .collect();
                Example::Carousel(cx.new(|cx| {
                    Carousel::new(t("Furniture and objects", "家具与器物"), slides, cx)
                        .with_index(1)
                        .with_slide_width(256.)
                        .with_interval(std::time::Duration::from_millis(5000))
                        .with_action_labels(
                            t("Previous slide", "上一张"),
                            t("Next slide", "下一张"),
                        )
                        .with_rotation_labels(
                            t("Start slide rotation", "开始轮播"),
                            t("Pause slide rotation", "暂停轮播"),
                        )
                }))
            }
            "card-stack" => {
                let cards = vec![
                    StackCard::new("lis", t("Lisbon", "里斯本"), t("Portugal", "葡萄牙"))
                        .with_meta("LIS", t("14–17 Oct", "10 月 14–17 日"))
                        .with_pitch(t(
                            "Tiled hills, long lunches, and a riverside workshop space for 18.",
                            "山丘绵延、午宴悠长，还有一间可容 18 人的河畔工坊。",
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Plane,
                            t("3h 35m direct from Berlin", "3 小时 35 分柏林直达"),
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Sun,
                            t("22° and mostly sunny", "22°，晴间多云"),
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Wallet,
                            t("From €780 per person", "每人 €780 起"),
                        )),
                    StackCard::new("cph", t("Copenhagen", "哥本哈根"), t("Denmark", "丹麦"))
                        .with_meta("CPH", t("21–24 Oct", "10 月 21–24 日"))
                        .with_pitch(t(
                            "Harbour saunas, bikes for everyone, and a studio loft in Nørrebro.",
                            "港口桑拿、人手一辆单车，还有诺雷布罗的阁楼工作室。",
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Plane,
                            t("1h 10m direct from Berlin", "1 小时 10 分柏林直达"),
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Sun,
                            t("11° with light rain", "11°，小雨"),
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Wallet,
                            t("From €1,140 per person", "每人 €1,140 起"),
                        )),
                    StackCard::new("opo", t("Porto", "波尔图"), t("Portugal", "葡萄牙"))
                        .with_meta("OPO", t("14–17 Oct", "10 月 14–17 日"))
                        .with_pitch(t(
                            "A river-view warehouse by day and dinners in Ribeira by night.",
                            "白天是河景仓库，夜晚在里贝拉区晚餐。",
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Plane,
                            t("3h 20m direct from Berlin", "3 小时 20 分柏林直达"),
                        ))
                        .with_fact(StackFact::new(Glyph::Sun, t("19° and clear", "19°，晴")))
                        .with_fact(StackFact::new(
                            Glyph::Wallet,
                            t("From €690 per person", "每人 €690 起"),
                        )),
                    StackCard::new("kix", t("Kyoto", "京都"), t("Japan", "日本"))
                        .with_meta("KIX", t("3–9 Nov", "11 月 3–9 日"))
                        .with_pitch(t(
                            "Temple mornings, ryokan evenings, and a long flight to earn them.",
                            "寺院的清晨、旅店的夜晚，还有一段配得上它们的长途飞行。",
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Plane,
                            t("15h 40m with one stop", "15 小时 40 分，中转一次"),
                        ))
                        .with_fact(StackFact::new(Glyph::Sun, t("17° and dry", "17°，干燥")))
                        .with_fact(StackFact::new(
                            Glyph::Wallet,
                            t("From €2,380 per person", "每人 €2,380 起"),
                        )),
                    StackCard::new("edi", t("Edinburgh", "爱丁堡"), t("Scotland", "苏格兰"))
                        .with_meta("EDI", t("18–21 Nov", "11 月 18–21 日"))
                        .with_pitch(t(
                            "Museum afternoons, a rehearsal room with a view of the castle.",
                            "博物馆的午后，还有一间望得见城堡的排练室。",
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Plane,
                            t("2h 25m direct from Berlin", "2 小时 25 分柏林直达"),
                        ))
                        .with_fact(StackFact::new(Glyph::Sun, t("8° and windy", "8°，有风")))
                        .with_fact(StackFact::new(
                            Glyph::Wallet,
                            t("From €520 per person", "每人 €520 起"),
                        )),
                    StackCard::new("svq", t("Seville", "塞维利亚"), t("Spain", "西班牙"))
                        .with_meta("SVQ", t("25–28 Nov", "11 月 25–28 日"))
                        .with_pitch(t(
                            "Courtyard tapas and a rooftop session under the lights.",
                            "庭院里的塔帕斯，还有华灯之下的天台会议。",
                        ))
                        .with_fact(StackFact::new(
                            Glyph::Plane,
                            t("3h 55m with one stop", "3 小时 55 分，中转一次"),
                        ))
                        .with_fact(StackFact::new(Glyph::Sun, t("18° and sunny", "18°，晴")))
                        .with_fact(StackFact::new(
                            Glyph::Wallet,
                            t("From €460 per person", "每人 €460 起"),
                        )),
                ];
                Example::CardStack(cx.new(|cx| {
                    CardStack::new(t("Offsite destinations", "团建目的地"), cards, cx)
                        .with_labels(t("Skip", "跳过"), t("Shortlist", "收藏"), t("Undo", "撤销"))
                        .with_empty(
                            t("Shortlist ready", "收藏清单已就绪"),
                            t("Nothing shortlisted yet", "还没有收藏"),
                            t("Start over", "重新开始"),
                        )
                }))
            }
            "toast-stack" => {
                let mut initial_id = None;
                let stack = cx.new(|cx| {
                    let mut s = ToastStack::new(t("Notifications", "通知"));
                    initial_id = Some(s.push(
                        t(
                            "Release v2.4.0 published to production",
                            "版本 v2.4.0 已发布至生产环境",
                        ),
                        Some(t("View", "查看").into()),
                        std::time::Duration::from_secs(60),
                        cx,
                    ));
                    s
                });
                Example::ToastStack {
                    stack,
                    ids: initial_id.into_iter().collect(),
                    seq: 1,
                }
            }
            "bottom-sheet" => {
                let title = t("Lisbon, 3 nights", "里斯本，3 晚").to_string();
                let rows = [
                    (
                        t("Flight", "航班"),
                        t("Berlin → Lisbon · 09:15", "柏林 → 里斯本 · 09:15"),
                    ),
                    (
                        t("Hotel", "酒店"),
                        t(
                            "Santa Clara 1728 · Check-in 15:00",
                            "Santa Clara 1728 · 15:00 入住",
                        ),
                    ),
                    (
                        t("Workshop", "工坊"),
                        t("Riverside Studio · 18 seats", "河畔工作室 · 18 个席位"),
                    ),
                ];
                let body = Box::new(move |_: &mut Window, cx: &mut App| {
                    let p = Paint::get(cx);
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .children(rows.iter().enumerate().map(|(i, (k, v))| {
                            div()
                                .py(px(8.))
                                .when(i > 0, |d| d.border_t_1().border_color(p.line))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(12.))
                                .text_size(px(13.))
                                .child(div().text_color(p.secondary).child(*k))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(*v),
                                )
                        }))
                        .into_any_element()
                });
                Example::BottomSheet(cx.new(|cx| {
                    BottomSheet::new(t("View trip details", "查看行程详情"), title, body, cx)
                        .contained()
                }))
            }
            "sheet-stack" => {
                let pages = vec![
                    SheetPage::new(
                        "account",
                        t("Settings", "设置"),
                        t("Manage your workspace settings", "管理工作区设置"),
                    )
                    .with_links(vec![
                        SheetLink::new("notifications", t("Notifications", "通知设置")),
                        SheetLink::new("privacy", t("Privacy", "隐私设置")),
                    ]),
                    SheetPage::new(
                        "notifications",
                        t("Notifications", "通知设置"),
                        t("Choose how to receive updates", "选择接收更新的方式"),
                    ),
                    SheetPage::new(
                        "privacy",
                        t("Privacy", "隐私设置"),
                        t("Manage account visibility", "管理账户可见性"),
                    ),
                ];
                Example::SheetStack(cx.new(|cx| {
                    SheetStack::new(t("Open settings", "打开设置"), pages, cx)
                        .with_action_labels(t("Back", "返回"), t("Close all", "全部关闭"))
                        .with_launcher_context(move |_, cx| {
                            let p = Paint::get(cx);
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(5.))
                                .text_center()
                                .child(
                                    img(crate::family_story::collections::portrait_image(0))
                                        .size(px(64.))
                                        .rounded_full(),
                                )
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("Emma Collins"),
                                )
                                .child(div().text_size(px(12.)).text_color(p.secondary).child(
                                    language.text(
                                        "Product designer · Studio team",
                                        "产品设计师 · 工作室团队",
                                    ),
                                ))
                        })
                        .contained()
                }))
            }
            "product-gallery" => {
                let shot = |bytes: &[u8], alt: (&'static str, &'static str)| {
                    let image = Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec()));
                    ProductPhoto::new(image, t(alt.0, alt.1))
                };
                let colors = vec![
                    ProductColor::new(
                        "oat",
                        t("Oat", "燕麦"),
                        rgb(0xdccfbc).into(),
                        vec![
                            shot(
                                include_bytes!("../../assets/product-gallery/oat-bed.jpg"),
                                (
                                    "Oat linen duvet and striped pillows",
                                    "燕麦色亚麻被套和条纹枕头",
                                ),
                            ),
                            shot(
                                include_bytes!("../../assets/product-gallery/oat-fold.jpg"),
                                ("Folded oat linen duvet", "折叠的燕麦色亚麻被套"),
                            ),
                            shot(
                                include_bytes!("../../assets/product-gallery/oat-detail.jpg"),
                                ("Oat linen weave in daylight", "日光下的燕麦色亚麻纹理"),
                            ),
                        ],
                    ),
                    ProductColor::new(
                        "forest",
                        t("Forest", "森林绿"),
                        rgb(0x365541).into(),
                        vec![
                            shot(
                                include_bytes!("../../assets/product-gallery/forest-bed.jpg"),
                                (
                                    "Forest green linen duvet on the bed",
                                    "铺在床上的森林绿亚麻被套",
                                ),
                            ),
                            shot(
                                include_bytes!("../../assets/product-gallery/forest-fold.jpg"),
                                ("Folded forest green linen duvet", "折叠的森林绿亚麻被套"),
                            ),
                        ],
                    ),
                    ProductColor::new(
                        "walnut",
                        t("Walnut", "胡桃"),
                        rgb(0x6b4631).into(),
                        vec![shot(
                            include_bytes!("../../assets/product-gallery/walnut-bed.jpg"),
                            (
                                "Walnut brown linen duvet on the bed",
                                "铺在床上的胡桃色亚麻被套",
                            ),
                        )],
                    ),
                ];
                let sizes = vec![
                    ProductSize::new("twin", t("Twin", "单人")),
                    ProductSize::new("full", t("Full", "双人")),
                    ProductSize::new("queen", t("Queen", "大床")),
                    ProductSize::new("king", t("King", "特大"))
                        .unavailable_for(vec!["forest".into()]),
                    ProductSize::new("cal-king", t("Cal king", "加长特大"))
                        .unavailable_for(vec!["oat".into(), "forest".into()]),
                ];
                let bag = cx.new(|_| BagPill { count: 0 });
                let bag_button = bag.clone();
                let title = t("Washed linen duvet set", "水洗亚麻被套组").to_string();
                let note = t("From $188 · queen shown", "单件 $188 · 展示为大床").to_string();
                let copy = t(
                    "Stonewashed European flax that softens with every wash. Hover the photo to look closer.",
                    "石洗欧洲亚麻，越洗越柔。把指针移到图片上可以细看纹理。",
                )
                .to_string();
                let add_label = t("Add to bag", "加入购物袋").to_string();
                Example::ProductGallery(cx.new(|cx| {
                    ProductGallery::new(t("Product photos", "商品图片"), colors, sizes, cx)
                        .with_header(move |_, cx| {
                            let p = Paint::get(cx);
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(12.))
                                .child(
                                    div()
                                        .flex()
                                        .items_start()
                                        .justify_between()
                                        .gap(px(12.))
                                        .child(
                                            div()
                                                .text_size(px(28.))
                                                .line_height(px(30.8))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(title.clone()),
                                        )
                                        .child(bag.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_baseline()
                                        .gap(px(10.))
                                        .child(
                                            div()
                                                .text_size(px(22.))
                                                .line_height(px(30.8))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child("$248"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .line_height(px(19.6))
                                                .text_color(p.faint)
                                                .child(note.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(19.6))
                                        .text_color(p.secondary)
                                        .max_w(px(468.))
                                        .child(copy.clone()),
                                )
                                .into_any_element()
                        })
                        .with_footer(move |_, cx| {
                            let p = Paint::get(cx);
                            let bag_button = bag_button.clone();
                            button("product-add", add_label.clone(), cx)
                                .w_full()
                                .h(px(50.))
                                .text_size(px(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.inverse)
                                .bg(p.bright)
                                .hover(move |s| s.bg(p.bright))
                                .child(add_label.clone())
                                .on_click(move |_, _, cx| {
                                    bag_button.update(cx, |bag, cx| {
                                        bag.count = bag.count.wrapping_add(1);
                                        cx.notify();
                                    });
                                })
                                .into_any_element()
                        })
                }))
            }
            _ => unreachable!(),
        };
        Self {
            content,
            language,
            step: 0,
            orientation: None,
            sheet_presentation: SheetStackPresentation::Auto,
        }
    }
    fn change(&mut self, cx: &mut Context<Self>) {
        self.step += 1;
        let t = |en, zh| self.language.text(en, zh);
        match &mut self.content {
            Example::Reveal(e) => e.update(cx, |e, cx| {
                e.set_text(
                    t(
                        "Small details make\nevery interaction\nfeel considered.",
                        "微小的细节，\n让每一次交互\n都显得深思熟虑。",
                    ),
                    cx,
                )
            }),
            Example::Title { view, .. } => view.update(cx, |e, cx| e.replay(cx)),
            Example::Morph(e) => {
                let (a, b) = if !self.step.is_multiple_of(2) {
                    ("Make it useful", "让细节发光")
                } else {
                    ("Make it simple", "让想法发生")
                };
                e.update(cx, |e, cx| e.set_text(t(a, b), cx));
            }
            Example::Shimmer {
                primary,
                secondary,
                active,
            } => {
                *active = !*active;
                let on = *active;
                primary.update(cx, |e, cx| e.set_active(on, cx));
                secondary.update(cx, |e, cx| e.set_active(on, cx));
                cx.notify();
            }
            Example::Slot { view, period } => {
                *period = (*period + 1) % 4;
                let value = ["$4,820", "$31,904", "$128,450", "$1,492,000"][*period];
                view.update(cx, |e, cx| e.set_value(value, cx));
                cx.notify();
            }
            Example::Compare(e) => e.update(cx, |e, cx| e.set_position(0.5, cx)),
            Example::Carousel(e) => e.update(cx, |e, cx| e.set_index(0, cx)),
            Example::CardStack(_)
            | Example::BottomSheet(_)
            | Example::SheetStack(_)
            | Example::ProductGallery(_) => {}
            Example::ToastStack { stack, ids, seq } => {
                *seq += 1;
                let n = *seq;
                let message = format!("{} v2.4.{n}", t("Published release", "已发布版本"));
                let id = stack.update(cx, |e, cx| {
                    e.push(message, None, std::time::Duration::from_secs(30), cx)
                });
                ids.push(id);
                cx.notify();
            }
        }
    }
}
impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        // The compare demo carries the reference chrome: caption bar,
        // orientation switch, and a center reset.
        if let Example::Compare(compare) = &self.content {
            if self.orientation.is_none() {
                let options = vec![
                    FormOption::new("0", self.language.text("Side by side", "并排对比")),
                    FormOption::new("1", self.language.text("Stacked", "上下对比")),
                ];
                let control = cx.new(|cx| {
                    SegmentedControl::new("Orientation", options, w, cx).with_selected(&["0"])
                });
                cx.subscribe(&control, |s, _, event: &SelectionChanged, cx| {
                    let vertical = event.0.first().map(|k| k.as_ref() == "1").unwrap_or(false);
                    if let Example::Compare(e) = &s.content {
                        e.update(cx, |e, cx| {
                            e.set_orientation(
                                if vertical {
                                    CompareOrientation::Vertical
                                } else {
                                    CompareOrientation::Horizontal
                                },
                                cx,
                            )
                        });
                    }
                })
                .detach();
                self.orientation = Some(control);
            }
            let control = self
                .orientation
                .clone()
                .expect("orientation control built above");
            let center_text = self.language.text("Center", "居中");
            let ink = p.ink;
            // Rotate-ccw glyph: an open counter-clockwise arc with a corner arrow.
            let icon = canvas(
                |_, _, _| (),
                move |b, _, w, _| {
                    let o = b.origin;
                    let mut path = PathBuilder::stroke(px(1.75));
                    let mut first = true;
                    let mut deg: f32 = 170.;
                    while deg >= -80.5 {
                        let a = deg.to_radians();
                        let pt = point(o.x + px(8. + 6. * a.cos()), o.y + px(8. + 6. * a.sin()));
                        if first {
                            path.move_to(pt);
                            first = false;
                        } else {
                            path.line_to(pt);
                        }
                        deg -= 10.;
                    }
                    path.move_to(point(o.x + px(2.), o.y + px(9.)));
                    path.line_to(point(o.x + px(2.), o.y + px(4.5)));
                    path.line_to(point(o.x + px(6.5), o.y + px(4.5)));
                    if let Ok(path) = path.build() {
                        w.paint_path(path, ink);
                    }
                },
            )
            .size(px(16.))
            .flex_none();
            let center = face(
                "compare-center",
                center_text,
                44.,
                f32::from(kui::foundation::surface_radius(cx)),
                Tone::Surface,
                cx,
            )
            .px_3()
            .gap(px(6.))
            .child(icon)
            .child(center_text)
            .on_click(cx.listener(|s, _, _, cx| {
                if let Example::Compare(e) = &s.content {
                    e.update(cx, |e, cx| e.set_position(0.5, cx));
                }
            }));
            let bar = div()
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_y(px(12.))
                .gap_x(px(16.))
                .child(
                    div()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(self.language.text("Living room", "客厅")),
                        )
                        .child(
                            div().text_size(px(12.)).text_color(p.faint).child(
                                self.language
                                    .text("Raw capture and final grade", "原始拍摄与最终调色"),
                            ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(control)
                        .child(center),
                );
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .gap(px(32.))
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(540.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(compare.clone())
                        .child(bar),
                );
        }
        // The carousel demo keeps the reference 720px-wide figure.
        if let Example::Carousel(carousel) = &self.content {
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .gap(px(32.))
                .items_center()
                .justify_center()
                .child(div().w_full().max_w(px(720.)).child(carousel.clone()));
        }
        if let Example::ProductGallery(gallery) = &self.content {
            return div()
                .size_full()
                .min_h(px(520.))
                .p(px(
                    (f32::from(w.viewport_size().width) * 0.087).clamp(16., 54.)
                ))
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .gap(px(32.))
                .items_center()
                .child(div().w_full().max_w(px(824.)).child(gallery.clone()));
        }
        if let Example::SheetStack(e) = &self.content {
            let control = e.clone();
            let t = |en, zh| self.language.text(en, zh);
            let mut modes = div().flex().items_center().gap(px(6.)).flex_wrap();
            for (mode, label) in [
                (SheetStackPresentation::Auto, t("Auto", "自动")),
                (SheetStackPresentation::Sheet, t("Sheets", "面板")),
                (SheetStackPresentation::Dialog, t("Dialogs", "对话框")),
            ] {
                let handle = control.clone();
                let selected = mode == self.sheet_presentation;
                modes = modes.child(
                    button(("sheet-mode", mode as usize), label, cx)
                        .aria_toggled(selected.into())
                        .when(selected, |b| b.bg(p.panel))
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.sheet_presentation = mode;
                            handle.update(cx, |v, cx| v.set_presentation(mode, cx));
                            cx.notify();
                        })),
                );
            }
            let stage = div()
                .w_full()
                .max_w(px(522.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .w_full()
                        .h(px(565.))
                        .rounded(kui::foundation::surface_radius(cx))
                        .border_1()
                        .border_color(p.line)
                        .child(control.clone()),
                )
                .child(
                    modes.child(
                        button("sheet-reset", t("Reset", "重置"), cx)
                            .child(t("Reset", "重置"))
                            .on_click(cx.listener(move |s, _, w, cx| {
                                s.sheet_presentation = SheetStackPresentation::Auto;
                                control.update(cx, |v, cx| {
                                    v.close(w, cx);
                                    v.set_presentation(SheetStackPresentation::Auto, cx);
                                });
                                cx.notify();
                            })),
                    ),
                );
            return div()
                .w_full()
                .px_5()
                .py(px(if w.viewport_size().width < px(460.) {
                    22.
                } else {
                    56.
                }))
                .bg(p.stage)
                .flex()
                .justify_center()
                .child(stage);
        }
        if let Example::Reveal(reveal) = &self.content {
            let t = |en, zh| self.language.text(en, zh);
            let replay = t("Replay", "重新播放");
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(480.))
                        .p(px(28.))
                        .rounded(kui::foundation::surface_radius(cx))
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(16.))
                        .child(
                            div()
                                .text_size(px(26.))
                                .line_height(px(33.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(reveal.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(21.))
                                .text_color(p.secondary)
                                .child(t(
                                    "Each word rises out of its line and sharpens as it lands.",
                                    "每个词从行间升起，落定之时渐趋清晰。",
                                )),
                        )
                        .child(
                            face(
                                "reveal-replay",
                                replay,
                                32.,
                                kui::foundation::surface_radius(cx).into(),
                                Tone::Surface,
                                cx,
                            )
                            .px(px(12.))
                            .border_1()
                            .border_color(p.line)
                            .text_size(px(12.5))
                            .child(replay)
                            .on_click(cx.listener(|s, _, _, cx| s.change(cx))),
                        ),
                );
        }
        if let Example::Title { view, mode } = &self.content {
            let t = |en, zh| self.language.text(en, zh);
            let current = *mode;
            let replay = t("Replay", "重新播放");
            let modes = [
                (TitleEntrance::Blur, t("Blur", "柔焦")),
                (TitleEntrance::Words, t("Words", "逐词")),
                (TitleEntrance::Lines, t("Lines", "逐行")),
                (TitleEntrance::Rise, t("Rise", "升起")),
            ];
            let radius = kui::foundation::surface_radius(cx);
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(500.))
                        .p(px(28.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(14.))
                        .child(
                            div()
                                .text_size(px(26.))
                                .line_height(px(33.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(view.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(21.))
                                .text_color(p.secondary)
                                .child(t(
                                    "Section headings enter once substantially in view, with word, line, or soft-rise choreography.",
                                    "章节标题在进入视口后自然浮现，支持逐词、逐行与柔和升起节奏。",
                                )),
                        )
                        .child(
                            div()
                                .pt(px(4.))
                                .w_full()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .p(px(3.))
                                        .rounded(radius)
                                        .bg(p.tray)
                                        .flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .children(modes.into_iter().enumerate().map(
                                            |(idx, (next_mode, name))| {
                                                let selected = next_mode == current;
                                                face(
                                                    ("title-mode", idx),
                                                    name,
                                                    28.,
                                                    radius.into(),
                                                    Tone::Clear,
                                                    cx,
                                                )
                                                .px(px(10.))
                                                .text_size(px(12.))
                                                .aria_toggled(selected.into())
                                                .when(selected, |b| {
                                                    b.bg(p.panel)
                                                        .text_color(p.ink)
                                                        .font_weight(FontWeight::MEDIUM)
                                                })
                                                .when(!selected, |b| b.text_color(p.secondary))
                                                .child(name)
                                                .on_click(cx.listener(move |s, _, _, cx| {
                                                    let lang = s.language;
                                                    let copy = if next_mode == TitleEntrance::Lines {
                                                        lang.text(
                                                            "Make progress\nvisible across teams",
                                                            "让每一处进展\n在团队间清晰可见",
                                                        )
                                                    } else {
                                                        lang.text(
                                                            "Make progress visible",
                                                            "让每一处进展清晰可见",
                                                        )
                                                    };
                                                    let rebuilt = cx.new(|cx| {
                                                        let mut v = InViewTitle::new(copy)
                                                            .with_entrance(next_mode);
                                                        v.replay(cx);
                                                        v
                                                    });
                                                    s.content = Example::Title {
                                                        view: rebuilt,
                                                        mode: next_mode,
                                                    };
                                                    cx.notify();
                                                }))
                                            },
                                        )),
                                )
                                .child(
                                    face(
                                        "title-replay",
                                        replay,
                                        32.,
                                        radius.into(),
                                        Tone::Surface,
                                        cx,
                                    )
                                    .px(px(12.))
                                    .border_1()
                                    .border_color(p.line)
                                    .text_size(px(12.5))
                                    .child(replay)
                                    .on_click(cx.listener(|s, _, _, cx| s.change(cx))),
                                ),
                        ),
                );
        }
        if let Example::Shimmer {
            primary,
            secondary,
            active,
        } = &self.content
        {
            let t = |en, zh| self.language.text(en, zh);
            let radius = kui::foundation::surface_radius(cx);
            let is_active = *active;
            let toggle_label = if is_active {
                t("Pause", "暂停")
            } else {
                t("Resume", "继续")
            };
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(480.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .px(px(18.))
                                .py(px(12.))
                                .border_b_1()
                                .border_color(p.line)
                                .bg(p.panel.blend(p.stage.opacity(0.3)))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(t("Workspace tasks", "工作区任务")),
                                )
                                .child(
                                    face(
                                        "shimmer-toggle",
                                        toggle_label,
                                        28.,
                                        radius.into(),
                                        Tone::Surface,
                                        cx,
                                    )
                                    .px(px(10.))
                                    .border_1()
                                    .border_color(p.line)
                                    .text_size(px(12.))
                                    .child(toggle_label)
                                    .on_click(cx.listener(|s, _, _, cx| s.change(cx))),
                                ),
                        )
                        .child(
                            div()
                                .px(px(18.))
                                .py(px(15.))
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(10.))
                                        .child(div().size(px(7.)).rounded_full().bg(if is_active {
                                            p.success
                                        } else {
                                            p.faint
                                        }))
                                        .child(primary.clone()),
                                )
                                .child(div().text_size(px(12.5)).text_color(p.faint).child(t(
                                    "Q3 planning notes · 18 pages",
                                    "Q3 规划笔记 · 18 页",
                                ))),
                        )
                        .child(
                            div()
                                .px(px(18.))
                                .py(px(14.))
                                .border_t_1()
                                .border_color(p.line)
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(10.))
                                        .child(
                                            div()
                                                .size(px(7.))
                                                .rounded_full()
                                                .bg(p.ink.opacity(0.35)),
                                        )
                                        .child(secondary.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .text_color(p.faint)
                                        .child(t("Brand assets", "品牌素材")),
                                ),
                        ),
                );
        }
        if let Example::Slot { view, period } = &self.content {
            let t = |en, zh| self.language.text(en, zh);
            let radius = kui::foundation::surface_radius(cx);
            let selected = *period;
            let deltas = ["+6.8%", "+12.4%", "+18.2%", "+31.6%"];
            let periods = [
                (t("Today", "今日"), "$4,820"),
                (t("Week", "本周"), "$31,904"),
                (t("Month", "本月"), "$128,450"),
                (t("Year", "全年"), "$1,492,000"),
            ];
            let slot_handle = view.clone();
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(460.))
                        .p(px(22.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .flex()
                        .flex_col()
                        .gap(px(16.))
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .child(div().size(px(7.)).rounded_full().bg(p.success))
                                        .child(
                                            div()
                                                .text_size(px(13.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.secondary)
                                                .child(t("Revenue", "营业收入")),
                                        ),
                                )
                                .child(
                                    div()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded_full()
                                        .bg(p.success.opacity(0.12))
                                        .text_color(p.success)
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(deltas[selected.min(3)]),
                                ),
                        )
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(view.clone()))
                        .child(
                            div()
                                .w_full()
                                .p(px(3.))
                                .rounded(radius)
                                .bg(p.tray)
                                .flex()
                                .items_center()
                                .children(periods.into_iter().enumerate().map(
                                    |(idx, (name, amount))| {
                                        let is_sel = idx == selected;
                                        let handle = slot_handle.clone();
                                        face(
                                            ("slot-period", idx),
                                            name,
                                            30.,
                                            radius.into(),
                                            Tone::Clear,
                                            cx,
                                        )
                                        .flex_1()
                                        .text_size(px(12.5))
                                        .aria_toggled(is_sel.into())
                                        .when(is_sel, |b| {
                                            b.bg(p.panel)
                                                .text_color(p.ink)
                                                .font_weight(FontWeight::MEDIUM)
                                        })
                                        .when(!is_sel, |b| b.text_color(p.secondary))
                                        .child(name)
                                        .on_click(
                                            cx.listener(move |s, _, _, cx| {
                                                if let Example::Slot { period, .. } = &mut s.content
                                                {
                                                    *period = idx;
                                                }
                                                handle.update(cx, |v, cx| v.set_value(amount, cx));
                                                cx.notify();
                                            }),
                                        )
                                    },
                                )),
                        ),
                );
        }
        if let Example::ToastStack { stack, .. } = &self.content {
            let t = |en, zh| self.language.text(en, zh);
            let radius = kui::foundation::surface_radius(cx);
            let active_count = stack.read(cx).len();
            let badge_text = if self.language == UiLanguage::Chinese {
                format!("{active_count} 条活动通知")
            } else {
                format!("{active_count} active")
            };
            let stack_view = stack.clone();
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(24.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(468.))
                        .p(px(18.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel)
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .text_size(px(15.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(p.ink)
                                                .child(t("Release desk", "发布工作台")),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .text_color(p.secondary)
                                                .child(t(
                                                    "Hover the stack below to expand and pause timers.",
                                                    "悬停下方通知堆叠以展开并暂停自动关闭。",
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded_full()
                                        .bg(p.stage)
                                        .text_size(px(11.5))
                                        .text_color(p.secondary)
                                        .child(badge_text),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    face(
                                        "toast-publish",
                                        t("Publish release", "发布版本"),
                                        32.,
                                        radius.into(),
                                        Tone::Primary,
                                        cx,
                                    )
                                    .px(px(12.))
                                    .text_size(px(12.5))
                                    .child(t("Publish release", "发布版本"))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        let lang = s.language;
                                        if let Example::ToastStack { stack, ids, seq } =
                                            &mut s.content
                                        {
                                            *seq += 1;
                                            let n = *seq;
                                            let msg = if lang == UiLanguage::Chinese {
                                                format!("版本 v2.4.{n} 已发布至生产环境")
                                            } else {
                                                format!("Release v2.4.{n} published to production")
                                            };
                                            let act = lang.text("View", "查看").into();
                                            let id = stack.update(cx, |st, cx| {
                                                st.push(
                                                    msg,
                                                    Some(act),
                                                    std::time::Duration::from_secs(30),
                                                    cx,
                                                )
                                            });
                                            ids.push(id);
                                            cx.notify();
                                        }
                                    })),
                                )
                                .child(
                                    face(
                                        "toast-upload",
                                        t("Upload assets", "上传素材"),
                                        32.,
                                        radius.into(),
                                        Tone::Surface,
                                        cx,
                                    )
                                    .px(px(11.))
                                    .border_1()
                                    .border_color(p.line)
                                    .text_size(px(12.5))
                                    .child(t("Upload assets", "上传素材"))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        let lang = s.language;
                                        if let Example::ToastStack { stack, ids, .. } =
                                            &mut s.content
                                        {
                                            let msg = lang.text(
                                                "14 brand assets uploaded to CDN",
                                                "14 个品牌素材已上传至 CDN",
                                            );
                                            let act = lang.text("Open", "打开").into();
                                            let id = stack.update(cx, |st, cx| {
                                                st.push(
                                                    msg,
                                                    Some(act),
                                                    std::time::Duration::from_secs(30),
                                                    cx,
                                                )
                                            });
                                            ids.push(id);
                                            cx.notify();
                                        }
                                    })),
                                )
                                .child(
                                    face(
                                        "toast-invite",
                                        t("Invite teammate", "邀请成员"),
                                        32.,
                                        radius.into(),
                                        Tone::Surface,
                                        cx,
                                    )
                                    .px(px(11.))
                                    .border_1()
                                    .border_color(p.line)
                                    .text_size(px(12.5))
                                    .child(t("Invite teammate", "邀请成员"))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        let lang = s.language;
                                        if let Example::ToastStack { stack, ids, .. } =
                                            &mut s.content
                                        {
                                            let msg = lang.text(
                                                "Invitation sent to Maya Chen",
                                                "已向 Maya Chen 发送工作区邀请",
                                            );
                                            let act = lang.text("Undo", "撤销").into();
                                            let id = stack.update(cx, |st, cx| {
                                                st.push(
                                                    msg,
                                                    Some(act),
                                                    std::time::Duration::from_secs(30),
                                                    cx,
                                                )
                                            });
                                            ids.push(id);
                                            cx.notify();
                                        }
                                    })),
                                )
                                .child(
                                    face(
                                        "toast-dismiss-all",
                                        t("Dismiss all", "全部关闭"),
                                        32.,
                                        radius.into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .px(px(10.))
                                    .text_size(px(12.5))
                                    .text_color(p.secondary)
                                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                    .child(t("Dismiss all", "全部关闭"))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        if let Example::ToastStack { stack, ids, .. } =
                                            &mut s.content
                                        {
                                            let pending = std::mem::take(ids);
                                            stack.update(cx, |st, cx| {
                                                for id in pending {
                                                    st.dismiss(id, cx);
                                                }
                                            });
                                            cx.notify();
                                        }
                                    })),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .min_h(px(108.))
                        .flex()
                        .items_end()
                        .justify_center()
                        .child(stack_view),
                );
        }
        if let Example::BottomSheet(sheet) = &self.content {
            let t = |en, zh| self.language.text(en, zh);
            let radius = kui::foundation::surface_radius(cx);
            return div()
                .size_full()
                .min_h(px(520.))
                .p_5()
                .bg(p.stage)
                .text_color(p.ink)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .relative()
                        .w_full()
                        .max_w(px(522.))
                        .h(px(460.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.panel.blend(p.stage.opacity(0.35)))
                        .overflow_hidden()
                        .child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .justify_between()
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .h(px(36.))
                                                .px(px(14.))
                                                .border_b_1()
                                                .border_color(p.line)
                                                .bg(p.panel)
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(6.))
                                                        .children((0..3).map(|_| {
                                                            div()
                                                                .size(px(7.))
                                                                .rounded_full()
                                                                .bg(p.line)
                                                        })),
                                                )
                                                .child(
                                                    div()
                                                        .px(px(10.))
                                                        .py(px(2.))
                                                        .rounded_full()
                                                        .bg(p.stage.opacity(0.75))
                                                        .text_size(px(11.))
                                                        .text_color(p.faint)
                                                        .child("trips.kense.dev/lisbon"),
                                                )
                                                .child(div().w(px(33.))),
                                        )
                                        .child(
                                            div()
                                                .p(px(20.))
                                                .flex()
                                                .flex_col()
                                                .gap(px(6.))
                                                .child(
                                                    div()
                                                        .text_size(px(11.5))
                                                        .text_color(p.faint)
                                                        .child(t(
                                                            "TEAM OFFSITE · OCT 14–17",
                                                            "团队出行 · 10 月 14–17 日",
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(18.))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(p.ink)
                                                        .child(t(
                                                            "Lisbon autumn studio",
                                                            "里斯本秋季工坊",
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .text_color(p.secondary)
                                                        .child(t(
                                                            "18 confirmed · Alfama & riverside loft",
                                                            "18 人已确认 · 阿尔法玛与河畔阁楼",
                                                        )),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .p(px(20.))
                                        .flex()
                                        .gap(px(10.))
                                        .children((0..3).map(|_| {
                                            div()
                                                .flex_1()
                                                .h(px(48.))
                                                .rounded(radius)
                                                .border_1()
                                                .border_color(p.line.opacity(0.7))
                                                .bg(p.panel.opacity(0.65))
                                        })),
                                ),
                        )
                        .child(div().absolute().inset_0().child(sheet.clone())),
                );
        }
        let view: AnyView = match &self.content {
            Example::Reveal(e) => e.clone().into(),
            Example::Title { view, .. } => view.clone().into(),
            Example::Morph(e) => e.clone().into(),
            Example::Shimmer { primary, .. } => primary.clone().into(),
            Example::Slot { view, .. } => view.clone().into(),
            Example::Compare(e) => e.clone().into(),
            Example::Carousel(e) => e.clone().into(),
            Example::CardStack(e) => e.clone().into(),
            Example::ToastStack { stack, .. } => stack.clone().into(),
            Example::BottomSheet(e) => e.clone().into(),
            Example::SheetStack(e) => e.clone().into(),
            Example::ProductGallery(e) => e.clone().into(),
        };
        let label = match &self.content {
            Example::Morph(_) => self.language.text("Change text", "切换文字"),
            Example::Compare(_) | Example::Carousel(_) => self.language.text("Reset", "重置"),
            _ => self.language.text("Open", "打开"),
        };
        div()
            .size_full()
            .min_h(px(520.))
            .p_5()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .gap(px(32.))
            .items_center()
            .justify_center()
            .child(div().max_w_full().child(view))
            .when(matches!(self.content, Example::Morph(_)), |d| {
                d.child(
                    face(
                        "change-text-motion",
                        label,
                        36.,
                        kui::foundation::surface_radius(cx).into(),
                        Tone::Surface,
                        cx,
                    )
                    .px_3()
                    .child(label)
                    .on_click(cx.listener(|s, _, _, cx| s.change(cx))),
                )
            })
    }
}
