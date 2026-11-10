use gpui::{prelude::*, *};
use kui::components::payment_flow::{PaymentFlow, PaymentFlowOptions};

pub const IDS: &[&str] = &["payment-flow"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| {
        crate::ensure_story_fonts(cx);
        cx.new(|cx| {
            Demo(cx.new(|cx| {
                PaymentFlow::new(PaymentFlowOptions::default(), cx).with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
            }))
        })
        .into()
    })
}

struct Demo(Entity<PaymentFlow>);

impl Render for Demo {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let width = w.viewport_size().width;
        div()
            .size_full()
            .min_h(px(552.))
            .flex()
            .items_center()
            .justify_center()
            .px(px(12.))
            .child(
                div()
                    .w_full()
                    .max_w(if width < px(724.) { px(522.) } else { px(761.) })
                    .child(self.0.clone()),
            )
    }
}
