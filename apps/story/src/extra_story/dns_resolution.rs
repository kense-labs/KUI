use gpui::{prelude::*, *};
use kui::components::dns_resolution::{DnsNode, DnsNodeKind, DnsResolution, DnsResolutionOptions};

pub const IDS: &[&str] = &["dns-resolution"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| {
        crate::ensure_story_fonts(cx);
        let control = cx.new(|cx| {
            let cloudflare = std::sync::Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                include_bytes!("../../assets/orbit-logos/cloudflare.svg").to_vec(),
            ));
            let resolver = DnsNode {
                mark: Some(std::rc::Rc::new(move || {
                    img(cloudflare.clone())
                        .size(px(15.))
                        .object_fit(ObjectFit::Contain)
                        .into_any_element()
                })),
                ..Default::default()
            };
            DnsResolution::new(
                DnsResolutionOptions::default().with_node(DnsNodeKind::Resolver, resolver),
                cx,
            )
            .with_visibility(|cx| {
                cx.try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)())
            })
            .with_style(|e| e.py(px(44.)))
        });
        cx.new(|_| Demo(control)).into()
    })
}

struct Demo(Entity<DnsResolution>);

impl Render for Demo {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let narrow = w.viewport_size().width < px(640.);
        div()
            .w_full()
            .px(px(if narrow { 16. } else { 24. }))
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(if narrow { 544. } else { 1000. }))
                    .child(self.0.clone()),
            )
    }
}
