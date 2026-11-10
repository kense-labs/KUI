//! Fictitious example notifications for the gallery.
use gpui::{prelude::*, *};
use kui::components::social_proof_toast::{SocialProofOptions, SocialProofToast};

pub const IDS: &[&str] = &["social-proof-toast"];
pub const PREVIEW_HEIGHT: f32 = 300.;

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| {
        let child = cx.new(|cx| {
            SocialProofToast::new(
                SocialProofOptions {
                    prefill: 2,
                    looping: false,
                    limit: Some(8),
                    ..Default::default()
                },
                w,
                cx,
            )
            .with_visibility(|cx| {
                cx.try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)())
            })
        });
        cx.new(|_| Example(child)).into()
    })
}

struct Example(Entity<SocialProofToast>);

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(PREVIEW_HEIGHT))
            .px(px(16.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .items_center()
            .justify_end()
            .child(self.0.clone())
    }
}
