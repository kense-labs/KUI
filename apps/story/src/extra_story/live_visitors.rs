//! Bilingual live visitor dashboard with the control's local simulation.
use gpui::{prelude::*, *};
use kui::components::live_visitors::LiveVisitors;

pub const IDS: &[&str] = &["live-visitors"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example(Entity<LiveVisitors>);

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        Self(cx.new(|cx| {
            LiveVisitors::new(cx).with_visibility(|cx| {
                cx.try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)())
            })
        }))
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (horizontal, vertical) = if w.viewport_size().width >= px(600.) {
            (53.2, 53.)
        } else {
            (12., 28.)
        };
        div()
            .w_full()
            .py(px(vertical))
            .px(px(horizontal))
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}
