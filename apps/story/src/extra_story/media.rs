//! Interactive examples for text and media primitives.
use gpui::{prelude::*, *};
use kui::{
    components::{
        bottom_sheet::BottomSheet,
        card_stack::{CardStack, StackCard},
        carousel::{Carousel, CarouselSlide},
        image_compare::ImageCompare,
        in_view_title::InViewTitle,
        product_gallery::{ProductColor, ProductGallery, ProductPhoto, ProductSize},
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
struct Demo {
    content: Example,
    language: UiLanguage,
    step: usize,
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
                    ImageCompare::new(before, after, t("Compare images", "图片对比"), cx)
                        .with_captions(t("Before", "之前"), t("After", "之后"))
                }))
            }
            "carousel" => {
                let images = [
                    include_bytes!("../../assets/navigation/room-0.jpg").as_slice(),
                    include_bytes!("../../assets/navigation/room-1.jpg").as_slice(),
                    include_bytes!("../../assets/navigation/room-2.jpg").as_slice(),
                ];
                let slides = images
                    .into_iter()
                    .enumerate()
                    .map(|(i, bytes)| {
                        CarouselSlide::new(
                            Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())),
                            format!("{} {}", t("Room", "空间"), i + 1),
                        )
                    })
                    .collect();
                Example::Carousel(cx.new(|cx| {
                    Carousel::new(t("Image carousel", "图片轮播"), slides, cx)
                        .with_action_labels(t("Previous", "上一张"), t("Next", "下一张"))
                }))
            }
            "card-stack" => {
                let cards = vec![
                    StackCard::new(
                        "lis",
                        t("Lisbon", "里斯本"),
                        t(
                            "Riverside workshops and quiet walks through tiled streets.",
                            "沿着河边漫步，在街巷里寻找灵感。",
                        ),
                    )
                    .with_eyebrow(t("LIS  ·  14–17 OCT", "LIS  ·  10 月 14–17 日"))
                    .with_details(vec![
                        t("Direct flight", "直达航班").into(),
                        t("22° and mostly sunny", "22°，晴间多云").into(),
                        t("From €780 per person", "每人 €780 起").into(),
                    ]),
                    StackCard::new(
                        "cph",
                        t("Copenhagen", "哥本哈根"),
                        t(
                            "Bikes, harbour views and room to create.",
                            "骑行、港口和自由创作的空间。",
                        ),
                    )
                    .with_eyebrow(t("CPH  ·  21–24 OCT", "CPH  ·  10 月 21–24 日"))
                    .with_details(vec![
                        t("Direct flight", "直达航班").into(),
                        t("11° and light rain", "11°，小雨").into(),
                    ]),
                    StackCard::new(
                        "kyo",
                        t("Kyoto", "京都"),
                        t(
                            "Temple mornings and peaceful evenings.",
                            "在寺院的清晨和安静的夜晚之间。",
                        ),
                    )
                    .with_eyebrow(t("KYO  ·  3–9 NOV", "KYO  ·  11 月 3–9 日"))
                    .with_details(vec![
                        t("One stop", "中转一次").into(),
                        t("17° and dry", "17°，干燥").into(),
                    ]),
                ];
                Example::CardStack(cx.new(|cx| {
                    CardStack::new(t("Choose a destination", "选择目的地"), cards, cx).with_labels(
                        t("Skip", "跳过"),
                        t("Choose", "收藏"),
                        t("Undo", "撤销"),
                        t("All done", "已完成"),
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
                let first = Arc::new(Image::from_bytes(
                    ImageFormat::Jpeg,
                    include_bytes!("../../assets/navigation/room-0.jpg").to_vec(),
                ));
                let second = Arc::new(Image::from_bytes(
                    ImageFormat::Jpeg,
                    include_bytes!("../../assets/navigation/room-1.jpg").to_vec(),
                ));
                let third = Arc::new(Image::from_bytes(
                    ImageFormat::Jpeg,
                    include_bytes!("../../assets/navigation/room-2.jpg").to_vec(),
                ));
                let colors = vec![
                    ProductColor::new(
                        "natural",
                        t("Natural", "原色"),
                        rgb(0xc9b99d).into(),
                        vec![
                            ProductPhoto::new(first.clone(), t("Front view", "正面")),
                            ProductPhoto::new(second, t("Detail view", "细节")),
                        ],
                    ),
                    ProductColor::new(
                        "charcoal",
                        t("Charcoal", "炭灰"),
                        rgb(0x42464b).into(),
                        vec![ProductPhoto::new(third, t("Side view", "侧面"))],
                    ),
                ];
                let sizes = vec![
                    ProductSize::new("s", "S"),
                    ProductSize::new("m", "M"),
                    ProductSize::new("l", "L").unavailable_for(vec!["charcoal".into()]),
                ];
                let product_title = t("Linen collection", "亚麻系列").to_string();
                let product_details = t(
                    "Natural texture for everyday spaces.",
                    "为日常空间带来自然质感。",
                )
                .to_string();
                Example::ProductGallery(cx.new(|cx| {
                    ProductGallery::new(t("Product photos", "商品图片"), colors, sizes, cx)
                        .with_header(move |_, _| {
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(div().text_lg().child(product_title.clone()))
                                .child(div().text_sm().child(product_details.clone()))
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
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
