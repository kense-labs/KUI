//! A simulated recording for the speculative decoding example.
use gpui::{prelude::*, *};
use kui::{
    components::speculative_decoding::{DecodeRound, SpeculativeDecoding},
    primitives::{control::UiLanguage, family::Paint},
};
pub const IDS: &[&str] = &["speculative-decoding"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    chart: Entity<SpeculativeDecoding>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let rounds = if language == UiLanguage::Chinese {
            vec![
                DecodeRound::new(
                    vec!["预览".into(), "部署".into(), "失败".into()],
                    2,
                    Some("完成".into()),
                ),
                DecodeRound::new(
                    vec!["，".into(), "所有".into(), "检查".into()],
                    3,
                    Some("已通过".into()),
                ),
                DecodeRound::new(vec!["，".into(), "可以".into()], 2, Some("合并。".into())),
            ]
        } else {
            vec![
                DecodeRound::new(
                    vec![
                        "Your".into(),
                        " preview".into(),
                        " deploy".into(),
                        " failed".into(),
                    ],
                    3,
                    Some(" finished".into()),
                ),
                DecodeRound::new(
                    vec![" in".into(), " 42".into(), " seconds".into(), ",".into()],
                    4,
                    Some(" and".into()),
                ),
                DecodeRound::new(
                    vec![" all".into(), " checks".into(), " failed".into()],
                    2,
                    Some(" passed".into()),
                ),
                DecodeRound::new(
                    vec![",".into(), " ready".into()],
                    2,
                    Some(" to merge.".into()),
                ),
            ]
        };
        let chart = cx.new(|_| SpeculativeDecoding::new(rounds));
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
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .items_center()
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.chart.clone())
    }
}
