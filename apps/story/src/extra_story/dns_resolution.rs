use gpui::{prelude::*, *};
use kui::components::dns_resolution::{DnsResolution, DnsResolutionOptions};

pub const IDS: &[&str] = &["dns-resolution"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| {
        crate::ensure_story_fonts(cx);
        cx.new(|cx| {
            DnsResolution::new(DnsResolutionOptions::default(), cx)
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
                .with_style(|e| e.py(px(20.)))
        })
        .into()
    })
}
