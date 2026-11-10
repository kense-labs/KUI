//! A scrollable backdrop for the floating glass navigation example.
use gpui::{prelude::*, *};
use kui::{
    components::glass_tabbar::{GlassTabBackdrop, GlassTabBar, GlassTabBarItem, GlassTabIcon},
    foundation::{Theme, ThemeMode},
    primitives::{control::UiLanguage, editor::EditorScrollRegion},
};
use std::{cell::RefCell, fmt::Write, rc::Rc, sync::Arc};

pub const IDS: &[&str] = &["glass-tabbar"];
const HEIGHT: f32 = 560.;
const FEED_HEIGHT: f32 = 1518.;

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

// The unobtrusive feed uses original neutral cards with a few subdued sections below the fold.
fn feed_image(dark: bool, blurred: bool) -> Arc<Image> {
    if blurred {
        // Pre-rasterize the same feed with its 12px SVG Gaussian filter.
        let pixels: &[u8] = if dark {
            include_bytes!("../../assets/glass/feed-soft-dark.png")
        } else {
            include_bytes!("../../assets/glass/feed-soft-light.png")
        };
        return Arc::new(Image::from_bytes(ImageFormat::Png, pixels.to_vec()));
    }
    let base = if dark { "#242424" } else { "#f4f4f4" };
    let neutral = if dark { "#282828" } else { "#e9e9e9" };
    let accent = if dark {
        ["#3f4c50", "#4b3d4b"]
    } else {
        ["#c7d8d9", "#e0d1e2"]
    };
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="420" height="{FEED_HEIGHT}" viewBox="0 0 420 {FEED_HEIGHT}">
<rect width="420" height="{FEED_HEIGHT}" fill="{base}"/>"#,
    );
    let mut y = 24;
    for (i, (wide, tall)) in [
        (true, true),
        (false, false),
        (true, false),
        (true, false),
        (false, true),
        (true, false),
        (false, true),
        (true, true),
        (false, false),
        (true, false),
        (true, true),
        (false, false),
        (true, false),
        (true, true),
    ]
    .into_iter()
    .enumerate()
    {
        let width = if wide { 364 } else { 226 };
        let height = if tall { 132 } else { 64 };
        let x = if wide || i % 2 == 0 { 28 } else { 392 - width };
        let ink = match i {
            6 | 8 => accent[0],
            7 | 10 | 13 => accent[1],
            _ => neutral,
        };
        let _ = write!(
            svg,
            r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="8" fill="{ink}"/>"#
        );
        y += height + 12;
    }
    svg.push_str("</svg>");
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}

#[derive(Default)]
struct Scene {
    image: Option<Arc<RenderImage>>,
    soft: Option<Arc<RenderImage>>,
}

struct Example {
    bar: Entity<GlassTabBar>,
    handle: ScrollHandle,
    frames: [[Arc<Image>; 2]; 2],
    scene: Rc<RefCell<Scene>>,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let handle = ScrollHandle::new();
        let scene = Rc::new(RefCell::new(Scene::default()));
        let source = scene.clone();
        let scroll = handle.clone();
        let bar = cx.new(|cx| {
            GlassTabBar::new(
                vec![
                    GlassTabBarItem::new("home", t("Home", "首页"), GlassTabIcon::Home),
                    GlassTabBarItem::new("explore", t("Explore", "探索"), GlassTabIcon::Explore),
                    GlassTabBarItem::new("saved", t("Saved", "收藏"), GlassTabIcon::Saved)
                        .with_badge(3),
                    GlassTabBarItem::new("profile", t("Profile", "我的"), GlassTabIcon::Profile),
                ],
                cx,
            )
            .with_id("glass-demo")
            .with_label(t("Main", "主要导航"))
            .with_action(
                GlassTabBarItem::new("search", t("Search", "搜索"), GlassTabIcon::Search),
                cx,
            )
            .with_scroll_handle(scroll.clone())
            .with_backdrop(move || {
                let source = source.borrow();
                let mut bounds = scroll.bounds();
                bounds.origin.y += scroll.offset().y;
                bounds.size.height = px(FEED_HEIGHT);
                Some(GlassTabBackdrop {
                    image: source.image.clone()?,
                    soft: source.soft.clone(),
                    bounds,
                })
            })
            .with_style(|d| {
                d.absolute()
                    .bottom(px(22.))
                    .left(px(12.))
                    .right(px(12.))
                    .w_auto()
            })
        });
        Self {
            bar,
            handle,
            frames: [false, true].map(|dark| [feed_image(dark, false), feed_image(dark, true)]),
            scene,
        }
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = usize::from(Theme::global(cx).mode == ThemeMode::Dark);
        let image = self.frames[dark][0].clone().use_render_image(w, cx);
        let soft = self.frames[dark][1].clone().use_render_image(w, cx);
        let handle = self.handle.clone();
        let bar = self.bar.clone();
        let mut scene = self.scene.borrow_mut();
        let changed = scene.image != image || scene.soft != soft;
        *scene = Scene {
            image: image.clone(),
            soft,
        };
        drop(scene);
        if changed {
            bar.update(cx, |_, cx| cx.notify());
        }
        div()
            .id("glass-tab-demo")
            .relative()
            .w(px(420.))
            .max_w_full()
            .h(px(HEIGHT))
            .flex_none()
            .child(
                div()
                    .id("glass-tab-feed")
                    .debug_selector(|| "glass-tab-feed".into())
                    .absolute()
                    .inset_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.handle)
                    .on_scroll(move |_, _, cx| cx.notify(bar.entity_id()))
                    .child(
                        div()
                            .w_full()
                            .h(px(FEED_HEIGHT))
                            .flex_none()
                            .when_some(image, |d, image| {
                                d.child(img(image).size_full().object_fit(ObjectFit::Fill))
                            }),
                    ),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |b, _, _, cx| {
                        if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                            let off = -handle.offset().y;
                            (host.0)(b, off > px(0.1), off + px(0.1) < handle.max_offset().y);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(self.bar.clone())
    }
}
