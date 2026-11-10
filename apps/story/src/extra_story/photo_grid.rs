//! Bilingual photo grid with existing public-domain images.
use gpui::{prelude::*, *};
use kui::{
    components::photo_grid::{GridPhoto, PhotoGrid},
    primitives::{control::UiLanguage, family::Paint},
};
use std::sync::Arc;

pub const IDS: &[&str] = &["photo-grid"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Photo {
    id: &'static str,
    en: &'static str,
    zh: &'static str,
    size: [f32; 2],
    bytes: &'static [u8],
}

const fn photo(
    id: &'static str,
    en: &'static str,
    zh: &'static str,
    size: [f32; 2],
    bytes: &'static [u8],
) -> Photo {
    Photo {
        id,
        en,
        zh,
        size,
        bytes,
    }
}

const PHOTOS: [Photo; 16] = [
    photo(
        "interior",
        "A quiet room",
        "安静的房间",
        [338., 460.],
        include_bytes!("../../assets/navigation/room-0.jpg"),
    ),
    photo(
        "pantheon",
        "Light through the dome",
        "穹顶上的天光",
        [354., 460.],
        include_bytes!("../../assets/navigation/room-3.jpg"),
    ),
    photo(
        "drawing-room",
        "The drawing room",
        "会客厅",
        [460., 366.],
        include_bytes!("../../assets/navigation/room-1.jpg"),
    ),
    photo(
        "kitchen",
        "The Nantucket kitchen",
        "楠塔基特的厨房",
        [460., 313.],
        include_bytes!("../../assets/navigation/room-2.jpg"),
    ),
    photo(
        "church",
        "A quiet aisle",
        "静谧的过道",
        [398., 460.],
        include_bytes!("../../assets/navigation/room-4.jpg"),
    ),
    photo(
        "synagogue",
        "Inside the synagogue",
        "会堂内部",
        [460., 376.],
        include_bytes!("../../assets/navigation/room-5.jpg"),
    ),
    photo(
        "teabowl",
        "Glazed teabowl",
        "釉色茶碗",
        [460., 322.],
        include_bytes!("../../assets/navigation/teabowl.jpg"),
    ),
    photo(
        "blue-marble",
        "The Blue Marble",
        "蓝色弹珠",
        [415., 520.],
        include_bytes!("../../assets/navigation/S72-37257.jpg"),
    ),
    photo(
        "lunar-orbit",
        "Lunar orbit",
        "月球轨道",
        [427., 520.],
        include_bytes!("../../assets/navigation/9312448.jpg"),
    ),
    photo(
        "apollo-17",
        "Apollo 17",
        "阿波罗十七号",
        [516., 520.],
        include_bytes!("../../assets/navigation/7035876.jpg"),
    ),
    photo(
        "flight",
        "Preparing for flight",
        "飞行准备",
        [435., 520.],
        include_bytes!("../../assets/navigation/7036672.jpg"),
    ),
    photo(
        "lunar-surface",
        "On the lunar surface",
        "月球表面",
        [532., 520.],
        include_bytes!("../../assets/navigation/as17-134-20476.jpg"),
    ),
    photo(
        "flag",
        "Apollo mission photograph",
        "登月任务照片",
        [520., 520.],
        include_bytes!("../../assets/navigation/as17-134-20481.jpg"),
    ),
    photo(
        "lunar-rock",
        "The lunar field",
        "月面考察",
        [520., 520.],
        include_bytes!("../../assets/navigation/as17-137-20989.jpg"),
    ),
    photo(
        "moon",
        "Across the Moon",
        "横越月面",
        [520., 520.],
        include_bytes!("../../assets/navigation/as17-147-22465.jpg"),
    ),
    photo(
        "horizon",
        "A lunar panorama",
        "月面全景",
        [660., 122.],
        include_bytes!("../../assets/navigation/jsc2004e52775.jpg"),
    ),
];

struct Demo(Entity<PhotoGrid>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let photos = PHOTOS
            .iter()
            .enumerate()
            .map(|(i, photo)| {
                let title = if zh { photo.zh } else { photo.en };
                let source = Arc::new(Image::from_bytes(ImageFormat::Jpeg, photo.bytes.to_vec()));
                let detail = if i < 7 {
                    if zh {
                        "馆藏精选"
                    } else {
                        "Museum collection"
                    }
                } else if zh {
                    "阿波罗档案"
                } else {
                    "Apollo archive"
                };
                GridPhoto::new(
                    photo.id,
                    source,
                    title,
                    title,
                    size(px(photo.size[0]), px(photo.size[1])),
                )
                .with_detail(detail)
            })
            .collect();
        let title = if zh {
            "光与远方"
        } else {
            "Light and distance"
        };
        Self(cx.new(|cx| {
            PhotoGrid::new(title, photos, cx)
                .with_default_favorites(["drawing-room".into(), "moon".into()])
        }))
    }
}

impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        let hint = if self.0.read(cx).viewed_photo().is_some() {
            if zh {
                "滑动照片或拖动胶片条切换，下拉即可返回。"
            } else {
                "Swipe or scrub the filmstrip between photos. Drag down to return."
            }
        } else if zh {
            "捏合或按住 Control 滚动以缩放，点按照片查看。"
        } else {
            "Pinch, or hold Control and scroll, to zoom. Tap a photo to open it."
        };
        let narrow = w.viewport_size().width < px(480.);
        div()
            .w_full()
            .px(px(if narrow { 0. } else { 24. }))
            .py(px(24.))
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(522.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .child(self.0.clone())
                    .child(
                        div()
                            .text_center()
                            .text_size(px(14.))
                            .text_color(Paint::get(cx).secondary)
                            .child(hint),
                    ),
            )
    }
}
