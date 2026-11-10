//! Interactive examples for text and media primitives.
use gpui::{prelude::*, *};
use kui::{
    components::{
        bottom_sheet::BottomSheet,
        card_stack::{CardStack, StackCard, StackFact},
        carousel::{Carousel, CarouselSlide},
        image_compare::{CompareOrientation, ImageCompare},
        in_view_title::InViewTitle,
        product_gallery::{ProductColor, ProductGallery, ProductPhoto, ProductSize},
        segmented_control::{FormOption, SegmentedControl, SelectionChanged},
        sheet_stack::{SheetLink, SheetPage, SheetStack},
        slot_text::SlotText,
        text_morph::TextMorph,
        text_reveal::TextReveal,
        text_shimmer::TextShimmer,
        toast_stack::ToastStack,
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
    Title(Entity<InViewTitle>),
    Morph(Entity<TextMorph>),
    Shimmer(Entity<TextShimmer>),
    Slot(Entity<SlotText>),
    Compare(Entity<ImageCompare>),
    Carousel(Entity<Carousel>),
    CardStack(Entity<CardStack>),
    ToastStack(Entity<ToastStack>),
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
}
impl Demo {
    fn new(id: &str, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let content = match id {
            "text-reveal" => Example::Reveal(cx.new(|cx| {
                TextReveal::new(
                    t(
                        "Thoughtful details make space for the work.",
                        "让细节安静下来，让作品自然呈现。",
                    ),
                    cx,
                )
            })),
            "in-view-title" => Example::Title(
                cx.new(|_| InViewTitle::new(t("A little more room to think", "给灵感更多空间"))),
            ),
            "text-morph" => {
                Example::Morph(cx.new(|_| TextMorph::new(t("Make it simple", "让想法发生"))))
            }
            "text-shimmer" => Example::Shimmer(cx.new(|cx| {
                TextShimmer::new(t("Thinking through the details…", "正在思考细节…"), cx)
            })),
            "slot-text" => Example::Slot(cx.new(|_| SlotText::new("12,480"))),
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
                Example::ToastStack(cx.new(|_| ToastStack::new(t("Notifications", "通知"))))
            }
            "bottom-sheet" => {
                let title = t("Lisbon, 3 nights", "里斯本，3 晚").to_string();
                let details = t("Flights, hotel and bookings", "航班、酒店和行程安排").to_string();
                let body = Box::new(move |_: &mut Window, _: &mut App| {
                    div().child(details.clone()).into_any_element()
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
        }
    }
    fn change(&mut self, cx: &mut Context<Self>) {
        self.step += 1;
        let t = |en, zh| self.language.text(en, zh);
        match &self.content {
            Example::Reveal(e) => e.update(cx, |e, cx| {
                e.set_text(
                    t(
                        "Thoughtful details make space for the work.",
                        "让细节安静下来，让作品自然呈现。",
                    ),
                    cx,
                )
            }),
            Example::Title(e) => e.update(cx, |e, cx| e.replay(cx)),
            Example::Morph(e) => {
                let (a, b) = if !self.step.is_multiple_of(2) {
                    ("Make it useful", "让细节发光")
                } else {
                    ("Make it simple", "让想法发生")
                };
                e.update(cx, |e, cx| e.set_text(t(a, b), cx));
            }
            Example::Shimmer(e) => {
                e.update(cx, |e, cx| e.set_active(self.step.is_multiple_of(2), cx))
            }
            Example::Slot(e) => {
                let value = ["15,280", "9,876", "12,480"][self.step % 3];
                e.update(cx, |e, cx| e.set_value(value, cx));
            }
            Example::Compare(e) => e.update(cx, |e, cx| e.set_position(0.5, cx)),
            Example::Carousel(e) => e.update(cx, |e, cx| e.set_index(0, cx)),
            Example::CardStack(e) => e.update(cx, |e, cx| e.undo(cx)),
            Example::BottomSheet(_) | Example::SheetStack(_) | Example::ProductGallery(_) => {}
            Example::ToastStack(e) => {
                let message = t("Your changes were saved", "更改已保存");
                e.update(cx, |e, cx| {
                    e.push(message, None, std::time::Duration::from_secs(5), cx);
                });
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
        let view: AnyView = match &self.content {
            Example::Reveal(e) => e.clone().into(),
            Example::Title(e) => e.clone().into(),
            Example::Morph(e) => e.clone().into(),
            Example::Shimmer(e) => e.clone().into(),
            Example::Slot(e) => e.clone().into(),
            Example::Compare(e) => e.clone().into(),
            Example::Carousel(e) => e.clone().into(),
            Example::CardStack(e) => e.clone().into(),
            Example::ToastStack(e) => e.clone().into(),
            Example::BottomSheet(e) => e.clone().into(),
            Example::SheetStack(e) => e.clone().into(),
            Example::ProductGallery(e) => e.clone().into(),
        };
        let label = match self.content {
            Example::Reveal(_) => self.language.text("Replay", "重新播放"),
            Example::Title(_) => self.language.text("Reveal", "显示标题"),
            Example::Morph(_) | Example::Slot(_) => self.language.text("Change text", "切换文字"),
            Example::Compare(_) | Example::Carousel(_) => self.language.text("Reset", "重置"),
            Example::CardStack(_) => self.language.text("Undo", "撤销"),
            Example::ToastStack(_) => self.language.text("Show notification", "显示通知"),
            Example::BottomSheet(_) | Example::SheetStack(_) | Example::ProductGallery(_) => {
                self.language.text("Open", "打开")
            }
            Example::Shimmer(_) if self.step.is_multiple_of(2) => {
                self.language.text("Pause", "暂停")
            }
            Example::Shimmer(_) => self.language.text("Resume", "继续"),
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
            .child(
                div()
                    .max_w_full()
                    .when(
                        matches!(
                            self.content,
                            Example::BottomSheet(_) | Example::SheetStack(_)
                        ),
                        |d| d.w_full().h(px(440.)),
                    )
                    .child(view),
            )
            .when(
                !matches!(
                    self.content,
                    Example::BottomSheet(_) | Example::SheetStack(_) | Example::ProductGallery(_)
                ),
                |d| {
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
                },
            )
    }
}
