//! Bilingual trail rail with locally bundled, credited photographs.
use gpui::{prelude::*, *};
use kui::{
    components::cover_flow::{CoverFlow, CoverFlowIndexChanged, CoverFlowItem},
    foundation::surface_radius,
    primitives::{
        control::{UiLanguage, activate},
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
        presentation::{column, hint, row},
    },
};
use std::sync::Arc;

pub const IDS: &[&str] = &["cover-flow"];

type Translation = (&'static str, &'static str);

struct Trail {
    id: &'static str,
    name: Translation,
    location: Translation,
    alt: Translation,
    distance: &'static str,
    ascent: &'static str,
    light: Translation,
    photo: &'static [u8],
}

const TRAILS: [Trail; 7] = [
    Trail {
        id: "sunrise",
        name: ("Sunrise Lake", "日出湖"),
        location: ("Mount Rainier, Washington", "华盛顿州雷尼尔山"),
        alt: ("An alpine lake below a forested slope", "林坡下的高山湖泊"),
        distance: "3.2 mi",
        ascent: "480 ft",
        light: ("Morning", "清晨"),
        photo: include_bytes!("../../assets/cover-flow/sunrise.jpg"),
    },
    Trail {
        id: "crescent",
        name: ("Mount Storm King", "风暴王山"),
        location: ("Olympic Peninsula, Washington", "华盛顿州奥林匹克半岛"),
        alt: ("A hiker looks out over Lake Crescent", "徒步者俯瞰新月湖"),
        distance: "4.0 mi",
        ascent: "2,060 ft",
        light: ("Late afternoon", "傍晚"),
        photo: include_bytes!("../../assets/cover-flow/crescent.jpg"),
    },
    Trail {
        id: "wonder",
        name: ("Wonder Lake", "奇迹湖"),
        location: ("Denali, Alaska", "阿拉斯加州德纳里"),
        alt: ("The mountains reflected in Wonder Lake", "奇迹湖倒映群山"),
        distance: "2.6 mi",
        ascent: "160 ft",
        light: ("Sunset", "日落"),
        photo: include_bytes!("../../assets/cover-flow/wonder.jpg"),
    },
    Trail {
        id: "cabin",
        name: ("Wonderland Trail", "仙境步道"),
        location: ("Mount Rainier, Washington", "华盛顿州雷尼尔山"),
        alt: (
            "A cabin and the cloudy peak beyond a pond",
            "雪山下的木屋与水塘",
        ),
        distance: "6.8 mi",
        ascent: "1,520 ft",
        light: ("Morning", "清晨"),
        photo: include_bytes!("../../assets/cover-flow/cabin.jpg"),
    },
    Trail {
        id: "lassen",
        name: ("Manzanita Lake", "曼萨尼塔湖"),
        location: ("Lassen Volcanic, California", "加利福尼亚州拉森火山"),
        alt: (
            "Snow on Lassen Peak above a quiet lake",
            "拉森雪峰与平静的湖泊",
        ),
        distance: "1.8 mi",
        ascent: "70 ft",
        light: ("Midday", "正午"),
        photo: include_bytes!("../../assets/cover-flow/lassen.jpg"),
    },
    Trail {
        id: "denali",
        name: ("Denali Overlook", "德纳里观景台"),
        location: ("Denali, Alaska", "阿拉斯加州德纳里"),
        alt: (
            "Layered mountains in Denali National Park",
            "德纳里国家公园的层叠山脉",
        ),
        distance: "2.9 mi",
        ascent: "420 ft",
        light: ("Blue hour", "暮色"),
        photo: include_bytes!("../../assets/cover-flow/denali.jpg"),
    },
    Trail {
        id: "paradise",
        name: ("Paradise Meadows", "天堂草甸"),
        location: ("Mount Rainier, Washington", "华盛顿州雷尼尔山"),
        alt: (
            "A trail through high mountain meadows",
            "穿过高山草甸的步道",
        ),
        distance: "5.2 mi",
        ascent: "1,400 ft",
        light: ("Sunrise", "日出"),
        photo: include_bytes!("../../assets/cover-flow/paradise.jpg"),
    },
];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Demo {
    rail: Entity<CoverFlow>,
    active: usize,
    saved: [bool; TRAILS.len()],
    _subscription: Subscription,
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |words: Translation| language.text(words.0, words.1);
        let items = TRAILS
            .iter()
            .map(|trail| {
                let photo = Arc::new(Image::from_bytes(ImageFormat::Jpeg, trail.photo.to_vec()));
                CoverFlowItem::new(trail.id, t(trail.name), photo)
                    .with_subtitle(t(trail.location))
                    .with_alt(t(trail.alt))
            })
            .collect();
        let rail = cx.new(|cx| {
            CoverFlow::new(t(("Hiking photos", "徒步风景")), items, cx)
                .with_default_index(2)
                .with_card_width(|width| {
                    if width <= 342. {
                        width * 0.54
                    } else {
                        let t = ((width - 342.) / 179.61).clamp(0., 1.);
                        184.68 + (172. - 184.68) * t
                    }
                })
        });
        let subscription = cx.subscribe(&rail, |s, _, event: &CoverFlowIndexChanged, cx| {
            s.active = event.0;
            cx.notify();
        });
        Self {
            rail,
            active: 2,
            saved: [false; TRAILS.len()],
            _subscription: subscription,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let compact = w.viewport_size().width <= px(440.);
        let trail = &TRAILS[self.active];
        let saved = self.saved[self.active];
        let save = if saved {
            t("Saved", "已收藏")
        } else {
            t("Save hike", "收藏路线")
        };
        let stats = row()
            .min_w_0()
            .gap(px(if compact { 10. } else { 22. }))
            .when(compact, |r| r.w_full())
            .children(
                [
                    (t("Round trip", "往返"), trail.distance),
                    (t("Elevation gain", "爬升"), trail.ascent),
                    (t("Best light", "最佳光线"), t(trail.light.0, trail.light.1)),
                ]
                .map(|(label, value)| {
                    column()
                        .min_w_0()
                        .when(compact, |cell| cell.flex_1())
                        .gap_1()
                        .child(hint(label, p))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(if compact { 14. } else { 17. }))
                                .line_height(px(22.))
                                .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                                .child(value),
                        )
                }),
            );
        let save = face(
            "cover-flow-save",
            save,
            36.,
            surface_radius(cx).into(),
            Tone::Surface,
            cx,
        )
        .aria_toggled(saved.into())
        .gap_2()
        .px(px(14.))
        .when(compact, |face| face.w_full())
        .child(icon(
            if saved { Glyph::Check } else { Glyph::Pin },
            16.,
            p.ink,
        ))
        .child(save);
        let save = activate(save, cx, |s, _, cx| {
            s.saved[s.active] = !s.saved[s.active];
            cx.notify();
        });
        let controls = div()
            .w_full()
            .flex()
            .when(compact, |r| r.flex_col())
            .when(!compact, |r| r.items_center().justify_between())
            .gap_3()
            .child(stats)
            .child(save);
        div()
            .w_full()
            .min_h(px(542.))
            .px_6()
            .py_10()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(
                column()
                    .w_full()
                    .max_w(px(521.61))
                    .gap(px(24.))
                    .child(self.rail.clone())
                    .child(controls),
            )
    }
}
