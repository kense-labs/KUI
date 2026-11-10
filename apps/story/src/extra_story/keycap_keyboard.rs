//! A shared mechanical keyboard preview with two selectable layouts.
use gpui::{prelude::*, *};
use kui::{
    components::{
        keycap_keyboard::{KeycapKeyboard, KeycapLayout, KeycapListen},
        segmented_control::{FormOption, SegmentedControl, SelectionChanged},
    },
    primitives::control::UiLanguage,
};

pub const IDS: &[&str] = &["keycap-keyboard"];

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Demo::new(w, cx)).into())
}

struct Demo {
    keyboard: Entity<KeycapKeyboard>,
    layout: Entity<SegmentedControl>,
    _change: Subscription,
}

impl Demo {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let lang = UiLanguage::current(cx);
        let keyboard = cx.new(|cx| {
            KeycapKeyboard::new(cx)
                .with_placeholder(lang.text("Type anything", "试着输入文字"))
                .with_listen(KeycapListen::Window)
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
        });
        let layout = cx.new(|cx| {
            SegmentedControl::new(
                lang.text("Keyboard layout", "键盘布局"),
                vec![
                    FormOption::new("compact", lang.text("Compact", "完整")),
                    FormOption::new("letters", lang.text("Letters", "字母")),
                ],
                w,
                cx,
            )
            .with_selected(&["compact"])
        });
        let change = cx.subscribe(&layout, |s, _, event: &SelectionChanged, cx| {
            let layout = if event.0.first().is_some_and(|s| s.as_ref() == "letters") {
                KeycapLayout::Letters
            } else {
                KeycapLayout::Compact
            };
            s.keyboard
                .update(cx, |keyboard, cx| keyboard.set_layout(layout, cx));
        });
        Self {
            keyboard,
            layout,
            _change: change,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .max_w(px(546.))
            .px(px(12.))
            .py(px(36.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(18.))
            .child(self.keyboard.clone())
            .child(div().w(px(180.)).child(self.layout.clone()))
    }
}
