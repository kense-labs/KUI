//! Scripted terminal with the shared GPUI playback and seek controls.
use gpui::{prelude::*, *};
use kui::components::typewriter_terminal::TypewriterTerminal;

pub const IDS: &[&str] = &["typewriter-terminal"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Demo(Entity<TypewriterTerminal>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        Self(cx.new(|cx| {
            TypewriterTerminal::new(cx)
                .with_font_family("Fira Code")
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
        }))
    }
}

impl Render for Demo {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let padding = if w.viewport_size().width >= px(600.) {
            53.2
        } else {
            12.
        };
        div()
            .w_full()
            .px(px(padding))
            .py(px(36.))
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}
