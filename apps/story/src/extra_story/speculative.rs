//! A simulated recording for the speculative decoding example.
use gpui::{prelude::*, *};
use kui::{components::speculative_decoding::SpeculativeDecoding, primitives::family::Paint};
pub const IDS: &[&str] = &["speculative-decoding"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    chart: Entity<SpeculativeDecoding>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let chart = cx.new(|cx| SpeculativeDecoding::sample(cx));
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
        let weak = chart.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;
                if weak
                    .update(cx, |chart, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        chart.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self { chart }
    }
}
impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .items_center()
            .justify_center()
            .p(px(if w.viewport_size().width < px(340.) {
                12.
            } else {
                24.
            }))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.chart.clone())
    }
}
