//! A bilingual membership card drawn on the shared perspective plane.
use gpui::{prelude::*, *};
use kui::{
    components::holo_card::{HoloCard, HoloCardOrientationRequested, HoloFace, HoloSurface},
    primitives::control::UiLanguage,
};
use std::sync::{Arc, OnceLock};

pub const IDS: &[&str] = &["holo-card"];
const WIDTH: f32 = 290.;
const HEIGHT: f32 = 406.;

/// System input provided by the host of this example.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct HoloPreviewPreferences {
    pub tilt: Option<Point<f32>>,
    pub reduced_transparency: bool,
}
impl Global for HoloPreviewPreferences {}

/// Connects the first touch to the host's sensor permission flow.
pub struct HoloPreviewHost(pub fn());
impl Global for HoloPreviewHost {}

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(MemberCard::new).into())
}

struct MemberCard {
    card: Entity<HoloCard>,
    _subscriptions: [Subscription; 2],
}

impl MemberCard {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let front = HoloFace::painted(
            lang.text(
                "Leon Zhu, design lead. Member since 2023. Shipped 148 projects.",
                "Leon Zhu，设计负责人。2023 年加入，已交付 148 个项目。",
            ),
            move |surface, w, cx| front(surface, lang, w, cx),
        );
        let back = HoloFace::painted(
            lang.text(
                "KUI member since March 2023",
                "KUI 创始成员，2023 年 3 月加入",
            ),
            move |surface, w, cx| back(surface, lang, w, cx),
        );
        let card = cx.new(|cx| {
            let settings = cx
                .try_global::<HoloPreviewPreferences>()
                .copied()
                .unwrap_or_default();
            HoloCard::new(front, cx)
                .with_id("membership-card")
                .with_back(back)
                .with_width(px(WIDTH))
                .with_reduced_transparency(settings.reduced_transparency)
        });
        let update = cx.observe_global::<HoloPreviewPreferences>(|s, cx| {
            let preferences = *cx.global::<HoloPreviewPreferences>();
            s.card.update(cx, |card, cx| {
                card.set_reduced_transparency(preferences.reduced_transparency, cx);
                card.set_device_tilt(preferences.tilt, cx);
            });
        });
        let permission = cx.subscribe(&card, |_, _, _: &HoloCardOrientationRequested, cx| {
            if let Some(host) = cx.try_global::<HoloPreviewHost>() {
                (host.0)();
            }
        });
        Self {
            card,
            _subscriptions: [update, permission],
        }
    }
}

impl Render for MemberCard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .min_h(px(604.))
            .px(px(14.))
            .flex()
            .items_center()
            .justify_center()
            .child(self.card.clone())
    }
}

fn ink(rgb_value: u32) -> Hsla {
    rgb(rgb_value).into()
}

fn gradient(angle: f32, from: Hsla, start: f32, to: Hsla, end: f32) -> Background {
    linear_gradient(
        angle,
        linear_color_stop(from, start),
        linear_color_stop(to, end),
    )
}

fn pos(x: f32, y: f32) -> Point<f32> {
    point(x / WIDTH, y / HEIGHT)
}

fn mark(surface: HoloSurface, x: f32, y: f32, side: f32, w: &mut Window, cx: &mut App) {
    static BRAND: OnceLock<Arc<Image>> = OnceLock::new();
    let image = BRAND
        .get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../../../../website/public/assets/mark-dark.png").to_vec(),
            ))
        })
        .clone();
    if let Some(image) = image.use_render_image(w, cx) {
        let a = surface.project(x / WIDTH, y / HEIGHT);
        let b = surface.project((x + side) / WIDTH, (y + side) / HEIGHT);
        let bounds = Bounds::from_corners(a, b);
        if bounds.size.width > px(1.) && bounds.size.height > px(1.) {
            let _ = w.paint_image(bounds, bounds, Corners::default(), image, 0, false);
        }
    }
}

#[derive(Clone, Copy)]
struct Lettering {
    size: f32,
    weight: FontWeight,
    color: Hsla,
}
impl Lettering {
    fn new(size: f32, color: Hsla) -> Self {
        Self {
            size,
            weight: FontWeight::NORMAL,
            color,
        }
    }
    fn medium(mut self) -> Self {
        self.weight = FontWeight::MEDIUM;
        self
    }

    fn paint(
        self,
        surface: HoloSurface,
        text: &'static str,
        anchor: Point<f32>,
        centered: bool,
        w: &mut Window,
    ) {
        let mut run = w.text_style().to_run(text.len());
        run.font.weight = self.weight;
        run.color = self.color;
        let scale = f32::from(surface.width()) / WIDTH;
        let em = px(self.size * scale);
        let line = w.text_system().shape_line(text.into(), em, &[run], None);
        let origin = if centered {
            anchor.x - (line.width / surface.width()) * 0.5
        } else {
            anchor.x
        };
        let y = anchor.y + f32::from(line.ascent) / (HEIGHT * scale);
        for run in &line.runs {
            for glyph in &run.glyphs {
                let x = origin + glyph.position.x / surface.width();
                let y = y + f32::from(glyph.position.y) / (HEIGHT * scale);
                let at = surface.project(x, y);
                let width_ratio = f32::from((surface.project(x + 0.02, y).x - at.x).abs())
                    / (0.02 * f32::from(surface.width()));
                if width_ratio > 0.09 {
                    let fit = px((f32::from(em) * width_ratio.min(1.17) * 2.).round() / 2.);
                    let _ = w.paint_glyph(
                        at,
                        run.font_id,
                        glyph.id,
                        fit,
                        self.color.opacity((width_ratio * 2.4).min(1.)),
                    );
                }
            }
        }
    }
}

fn front(surface: HoloSurface, lang: UiLanguage, w: &mut Window, cx: &mut App) {
    surface.fill(gradient(90., ink(0x7adf8f), 0., ink(0x9ab9b1), 1.), w);
    surface.fill(
        gradient(119., ink(0xe270dc).opacity(0.), 0.34, ink(0xe270dc), 0.78),
        w,
    );
    surface.fill(
        gradient(180., ink(0x111111).opacity(0.), 0.35, ink(0x101011), 0.67),
        w,
    );
    mark(surface, 20., 20., 24., w, cx);
    let pill = (0..32).map(|i| {
        let a = i as f32 * std::f32::consts::TAU / 32.;
        let x = if a.cos() >= 0. { 259.5 } else { 241.5 };
        pos(x + a.cos() * 10.5, 32. + a.sin() * 10.5)
    });
    surface.polygon(pill, ink(0xf6f4f7).opacity(0.92), w);
    Lettering::new(11., ink(0x202020))
        .medium()
        .paint(surface, "Pro", pos(250.5, 25.5), true, w);
    Lettering::new(28., white())
        .medium()
        .paint(surface, "Leon Zhu", pos(20., 266.), false, w);
    Lettering::new(14., ink(0xc1c1c1)).paint(
        surface,
        lang.text("Design lead", "设计负责人"),
        pos(20., 301.),
        false,
        w,
    );
    surface.polygon(
        [
            pos(20., 334.5),
            pos(270., 334.5),
            pos(270., 335.5),
            pos(20., 335.5),
        ],
        white().opacity(0.15),
        w,
    );
    for (x, title, value) in [
        (20., lang.text("Member since", "加入时间"), "2023"),
        (151., lang.text("Shipped", "已交付"), "148"),
    ] {
        Lettering::new(12., ink(0xa3a3a3)).paint(surface, title, pos(x, 348.), false, w);
        Lettering::new(14., white())
            .medium()
            .paint(surface, value, pos(x, 366.), false, w);
    }
}

fn back(surface: HoloSurface, lang: UiLanguage, w: &mut Window, cx: &mut App) {
    surface.fill(gradient(145., ink(0x141414), 0., ink(0x0e0f10), 1.), w);
    surface.fill(
        gradient(
            20.,
            ink(0x52704f).opacity(0.10),
            0.,
            ink(0x101014).opacity(0.),
            0.62,
        ),
        w,
    );
    mark(surface, 109., 126., 72., w, cx);
    Lettering::new(28., white())
        .medium()
        .paint(surface, "KUI Pro", pos(145., 221.), true, w);
    Lettering::new(14., ink(0xbdbac3)).paint(
        surface,
        lang.text("Founding member, March 2023", "创始成员 · 2023 年 3 月"),
        pos(145., 260.),
        true,
        w,
    );
    Lettering::new(12., ink(0xaca9b4)).paint(
        surface,
        "kense-labs.github.io/kui",
        pos(145., 369.),
        true,
        w,
    );
}
