//! Additional interactive examples and their embedding bridge.
mod agent_cursor;
mod agent_handoff;
mod agent_loop;
mod ai_workflow;
mod audio;
mod charts;
mod feedback;
mod ghost;
mod image_generation;
mod image_reveal;
mod inputs;
mod link_unfurl;
mod llm_cost_calculator;
mod loader;
mod media;
mod meeting_summary;
mod mixture_of_experts;
mod prompt_cache;
mod provider;
mod reasoning_tree;
mod selection_demos;
mod spark;
mod speculative;
mod speech;
mod tone_tuner;
mod training_run;

use gpui::{prelude::*, *};
use kui::{
    foundation::Theme,
    primitives::{control::UiLanguage, family::Paint},
};

pub fn story(id: &str, window: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    let child = selection_demos::story(id, window, cx)
        .or_else(|| inputs::story(id, window, cx))
        .or_else(|| charts::story(id, window, cx))
        .or_else(|| media::story(id, window, cx))
        .or_else(|| audio::story(id, window, cx))
        .or_else(|| speech::story(id, window, cx))
        .or_else(|| loader::story(id, window, cx))
        .or_else(|| spark::story(id, window, cx))
        .or_else(|| provider::story(id, window, cx))
        .or_else(|| ghost::story(id, window, cx))
        .or_else(|| feedback::story(id, window, cx))
        .or_else(|| agent_loop::story(id, window, cx))
        .or_else(|| reasoning_tree::story(id, window, cx))
        .or_else(|| image_generation::story(id, window, cx))
        .or_else(|| speculative::story(id, window, cx))
        .or_else(|| image_reveal::story(id, window, cx))
        .or_else(|| prompt_cache::story(id, window, cx))
        .or_else(|| meeting_summary::story(id, window, cx))
        .or_else(|| ai_workflow::story(id, window, cx))
        .or_else(|| training_run::story(id, window, cx))
        .or_else(|| mixture_of_experts::story(id, window, cx))
        .or_else(|| agent_handoff::story(id, window, cx))
        .or_else(|| llm_cost_calculator::story(id, window, cx))
        .or_else(|| agent_cursor::story(id, window, cx))
        .or_else(|| link_unfurl::story(id, window, cx))
        .or_else(|| tone_tuner::story(id, window, cx))?;
    Some(cx.new(|cx| EmbeddedStory::new(child, cx)).into())
}

pub fn ids() -> impl Iterator<Item = &'static str> {
    inputs::IDS
        .iter()
        .chain(charts::IDS)
        .chain(media::IDS)
        .chain(audio::IDS)
        .chain(speech::IDS)
        .chain(provider::IDS)
        .chain(ghost::IDS)
        .chain(feedback::IDS)
        .chain(agent_loop::IDS)
        .chain(reasoning_tree::IDS)
        .chain(image_generation::IDS)
        .chain(speculative::IDS)
        .chain(image_reveal::IDS)
        .chain(prompt_cache::IDS)
        .chain(meeting_summary::IDS)
        .chain(ai_workflow::IDS)
        .chain(training_run::IDS)
        .chain(mixture_of_experts::IDS)
        .chain(agent_handoff::IDS)
        .chain(llm_cost_calculator::IDS)
        .chain(agent_cursor::IDS)
        .chain(link_unfurl::IDS)
        .chain(tone_tuner::IDS)
        .chain(["morph-loader", "ai-spark"].iter())
        .copied()
}

struct EmbeddedStory {
    child: AnyView,
    focus: FocusHandle,
    edges: [FocusHandle; 2],
}
impl EmbeddedStory {
    fn new(child: AnyView, cx: &mut Context<Self>) -> Self {
        Self {
            child,
            focus: cx.focus_handle(),
            edges: [cx.focus_handle(), cx.focus_handle()],
        }
    }
}
impl Render for EmbeddedStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        let edge = |i| {
            div()
                .id(("extra-edge", i))
                .track_focus(&self.edges[i])
                .tab_stop(true)
                .size(px(0.))
        };
        let embedded = cx.try_global::<crate::StoryBridge>().is_some();
        let scene = div()
            .w_full()
            .min_h(px(520.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(paint.stage)
            .when(embedded, |e| e.child(edge(0)))
            .child(self.child.clone())
            .when(embedded, |e| e.child(edge(1)));
        div()
            .on_children_prepainted(|bounds, _, cx| {
                if let (Some(bounds), Some(bridge)) =
                    (bounds.first(), cx.try_global::<crate::StoryBridge>())
                {
                    (bridge.frame)(bounds.size.height.into(), Theme::global(cx).mode);
                }
            })
            .id("extra-story")
            .track_focus(&self.focus)
            .tab_stop(false)
            .tab_group()
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .font_family("KUI Preview Latin")
            .text_color(paint.ink)
            .text_size(px(14.))
            .when(
                cfg!(target_family = "wasm") && UiLanguage::current(cx) == UiLanguage::Chinese,
                |e| e.font_family("KUI Story Sans"),
            )
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "tab" {
                    let back = e.keystroke.modifiers.shift;
                    let entering = v.focus.is_focused(w);
                    let step = |w: &mut Window, cx: &mut Context<Self>| {
                        if back {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        }
                    };
                    step(w, cx);
                    if entering && v.edges[usize::from(back)].is_focused(w) {
                        step(w, cx);
                    }
                    if v.edges[usize::from(!back)].is_focused(w) {
                        v.focus.focus(w, cx);
                        if let Some(b) = cx.try_global::<crate::StoryBridge>() {
                            (b.leave)(back);
                        }
                    }
                    cx.stop_propagation();
                }
            }))
            .child(scene)
    }
}
