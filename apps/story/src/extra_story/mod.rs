//! Additional interactive examples and their embedding bridge.
mod action_morph;
mod agent_cursor;
mod agent_handoff;
mod agent_loop;
mod ai_mascot;
mod ai_workflow;
mod audio;
mod booking_pill;
mod branch_graph;
mod card_to_article;
mod charge_session;
mod charts;
mod control_center;
mod cover_flow;
pub use control_center::ControlCenterPreviewHost;
mod date_reel;
mod dns_resolution;
mod dock;
mod dot_grid;
mod feedback;
mod ghost;
mod glass_card;
mod glass_tabs;
mod holo_card;
mod payment_flow;
mod photo_grid;
pub use holo_card::{HoloPreviewHost, HoloPreviewPreferences};
mod image_generation;
mod image_reveal;
mod inputs;
mod install_snippet;
mod keycap_keyboard;
mod link_unfurl;
mod liquid_tabs;
mod live_visitors;
mod llm_cost_calculator;
mod loader;
mod media;
mod meeting_summary;
mod mixture_of_experts;
mod morph_nav;
mod orbit_logos;
mod orbit_photo;
mod stretch_refresh;
pub use orbit_photo::OrbitPhotoHost;
mod prompt_cache;
mod provider;
mod reasoning_tree;
mod referral_card;
mod selection_demos;
mod share_sheet;
mod skeleton_morph;
mod social_proof_toast;
mod spark;
mod speculative;
mod speech;
mod ticket_tear;
mod time_dial;
mod tone_tuner;
mod training_run;
mod trial_banner;
mod typewriter_terminal;
mod upgrade_prompt;
mod wallet_stack;

use gpui::{prelude::*, *};
use kui::{
    foundation::Theme,
    primitives::{control::UiLanguage, family::Paint},
};

pub fn story(id: &str, window: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    let child = share_sheet::story(id, window, cx)
        .or_else(|| selection_demos::story(id, window, cx))
        .or_else(|| inputs::story(id, window, cx))
        .or_else(|| booking_pill::story(id, window, cx))
        .or_else(|| payment_flow::story(id, window, cx))
        .or_else(|| dns_resolution::story(id, window, cx))
        .or_else(|| dot_grid::story(id, window, cx))
        .or_else(|| branch_graph::story(id, window, cx))
        .or_else(|| date_reel::story(id, window, cx))
        .or_else(|| time_dial::story(id, window, cx))
        .or_else(|| charge_session::story(id, window, cx))
        .or_else(|| ticket_tear::story(id, window, cx))
        .or_else(|| charts::story(id, window, cx))
        .or_else(|| media::story(id, window, cx))
        .or_else(|| audio::story(id, window, cx))
        .or_else(|| speech::story(id, window, cx))
        .or_else(|| stretch_refresh::story(id, window, cx))
        .or_else(|| skeleton_morph::story(id, window, cx))
        .or_else(|| social_proof_toast::story(id, window, cx))
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
        .or_else(|| tone_tuner::story(id, window, cx))
        .or_else(|| ai_mascot::story(id, window, cx))
        .or_else(|| install_snippet::story(id, window, cx))
        .or_else(|| upgrade_prompt::story(id, window, cx))
        .or_else(|| referral_card::story(id, window, cx))
        .or_else(|| trial_banner::story(id, window, cx))
        .or_else(|| keycap_keyboard::story(id, window, cx))
        .or_else(|| typewriter_terminal::story(id, window, cx))
        .or_else(|| morph_nav::story(id, window, cx))
        .or_else(|| dock::story(id, window, cx))
        .or_else(|| liquid_tabs::story(id, window, cx))
        .or_else(|| orbit_photo::story(id, window, cx))
        .or_else(|| glass_card::story(id, window, cx))
        .or_else(|| glass_tabs::story(id, window, cx))
        .or_else(|| action_morph::story(id, window, cx))
        .or_else(|| wallet_stack::story(id, window, cx))
        .or_else(|| control_center::story(id, window, cx))
        .or_else(|| holo_card::story(id, window, cx))
        .or_else(|| card_to_article::story(id, window, cx))
        .or_else(|| photo_grid::story(id, window, cx))
        .or_else(|| cover_flow::story(id, window, cx))
        .or_else(|| orbit_logos::story(id, window, cx))
        .or_else(|| live_visitors::story(id, window, cx))?;
    let min_height = if stretch_refresh::IDS.contains(&id) {
        stretch_refresh::PREVIEW_HEIGHT
    } else if social_proof_toast::IDS.contains(&id) {
        social_proof_toast::PREVIEW_HEIGHT
    } else if card_to_article::IDS.contains(&id) {
        card_to_article::PREVIEW_HEIGHT
    } else {
        520.
    };
    Some(
        cx.new(|cx| EmbeddedStory::new(child, min_height, cx))
            .into(),
    )
}

pub fn ids() -> impl Iterator<Item = &'static str> {
    inputs::IDS
        .iter()
        .chain(date_reel::IDS)
        .chain(share_sheet::IDS)
        .chain(booking_pill::IDS)
        .chain(payment_flow::IDS)
        .chain(dns_resolution::IDS)
        .chain(dot_grid::IDS)
        .chain(branch_graph::IDS)
        .chain(time_dial::IDS)
        .chain(charge_session::IDS)
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
        .chain(ai_mascot::IDS)
        .chain(install_snippet::IDS)
        .chain(upgrade_prompt::IDS)
        .chain(referral_card::IDS)
        .chain(trial_banner::IDS)
        .chain(keycap_keyboard::IDS)
        .chain(typewriter_terminal::IDS)
        .chain(morph_nav::IDS)
        .chain(dock::IDS)
        .chain(liquid_tabs::IDS)
        .chain(orbit_photo::IDS)
        .chain(glass_card::IDS)
        .chain(glass_tabs::IDS)
        .chain(action_morph::IDS)
        .chain(wallet_stack::IDS)
        .chain(control_center::IDS)
        .chain(holo_card::IDS)
        .chain(card_to_article::IDS)
        .chain(photo_grid::IDS)
        .chain(cover_flow::IDS)
        .chain(orbit_logos::IDS)
        .chain(ticket_tear::IDS)
        .chain(live_visitors::IDS)
        .chain(stretch_refresh::IDS)
        .chain(skeleton_morph::IDS)
        .chain(social_proof_toast::IDS)
        .chain(["morph-loader", "ai-spark"].iter())
        .copied()
}

struct EmbeddedStory {
    min_height: f32,
    child: AnyView,
    focus: FocusHandle,
    edges: [FocusHandle; 2],
}
impl EmbeddedStory {
    fn new(child: AnyView, min_height: f32, cx: &mut Context<Self>) -> Self {
        Self {
            min_height,
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
        let bridge = cx.try_global::<crate::StoryBridge>();
        let embedded = bridge.is_some();
        let min_height = bridge.map_or(self.min_height, |bridge| {
            self.min_height.max((bridge.min_height)())
        });
        let scene = div()
            .w_full()
            .min_h(px(min_height))
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
