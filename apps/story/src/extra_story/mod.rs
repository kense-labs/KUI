//! Additional interactive examples and their embedding bridge.
mod action_morph;
mod affect_pro;
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
mod observability;
mod orbit_logos;
mod orbit_photo;
mod stretch_refresh;
pub use orbit_photo::OrbitPhotoHost;
mod pro_screens;
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
        .or_else(|| live_visitors::story(id, window, cx))
        .or_else(|| observability::story(id, window, cx))
        .or_else(|| pro_screens::story(id, window, cx))
        .or_else(|| affect_pro::story(id, window, cx))?;
    let full_bleed = pro_screens::IDS.contains(&id);
    let min_height = if pro_screens::SCREEN_IDS.contains(&id) {
        pro_screens::PREVIEW_HEIGHT
    } else if id == "gradient-floor" {
        pro_screens::FLOOR_PREVIEW_HEIGHT
    } else if stretch_refresh::IDS.contains(&id) {
        stretch_refresh::PREVIEW_HEIGHT
    } else if social_proof_toast::IDS.contains(&id) {
        social_proof_toast::PREVIEW_HEIGHT
    } else if card_to_article::IDS.contains(&id) {
        card_to_article::PREVIEW_HEIGHT
    } else {
        520.
    };
    Some(
        cx.new(|cx| EmbeddedStory::new(child, min_height, full_bleed, cx))
            .into(),
    )
}

pub fn ids() -> impl Iterator<Item = &'static str> {
    [
        inputs::IDS,
        date_reel::IDS,
        share_sheet::IDS,
        booking_pill::IDS,
        payment_flow::IDS,
        dns_resolution::IDS,
        dot_grid::IDS,
        branch_graph::IDS,
        time_dial::IDS,
        charge_session::IDS,
        charts::IDS,
        media::IDS,
        audio::IDS,
        speech::IDS,
        provider::IDS,
        ghost::IDS,
        feedback::IDS,
        agent_loop::IDS,
        reasoning_tree::IDS,
        image_generation::IDS,
        speculative::IDS,
        image_reveal::IDS,
        prompt_cache::IDS,
        meeting_summary::IDS,
        ai_workflow::IDS,
        training_run::IDS,
        mixture_of_experts::IDS,
        agent_handoff::IDS,
        llm_cost_calculator::IDS,
        agent_cursor::IDS,
        link_unfurl::IDS,
        tone_tuner::IDS,
        ai_mascot::IDS,
        install_snippet::IDS,
        upgrade_prompt::IDS,
        referral_card::IDS,
        trial_banner::IDS,
        keycap_keyboard::IDS,
        typewriter_terminal::IDS,
        morph_nav::IDS,
        dock::IDS,
        liquid_tabs::IDS,
        orbit_photo::IDS,
        glass_card::IDS,
        glass_tabs::IDS,
        action_morph::IDS,
        wallet_stack::IDS,
        control_center::IDS,
        holo_card::IDS,
        card_to_article::IDS,
        photo_grid::IDS,
        cover_flow::IDS,
        observability::IDS,
        orbit_logos::IDS,
        ticket_tear::IDS,
        live_visitors::IDS,
        stretch_refresh::IDS,
        skeleton_morph::IDS,
        social_proof_toast::IDS,
        pro_screens::IDS,
        affect_pro::IDS,
        &["morph-loader", "ai-spark"],
    ]
    .into_iter()
    .flatten()
    .copied()
}

struct EmbeddedStory {
    min_height: f32,
    full_bleed: bool,
    child: AnyView,
    focus: FocusHandle,
    edges: [FocusHandle; 2],
    last_mode: Option<kui::foundation::ThemeMode>,
}
impl EmbeddedStory {
    fn new(child: AnyView, min_height: f32, full_bleed: bool, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        Self {
            min_height,
            full_bleed,
            child,
            focus: cx.focus_handle(),
            edges: [cx.focus_handle(), cx.focus_handle()],
            last_mode: None,
        }
    }
}
impl Render for EmbeddedStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = Theme::global(cx).mode;
        if self.last_mode != Some(mode) {
            self.last_mode = Some(mode);
            window.request_animation_frame();
        }
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
        let host_h = bridge.map_or(0., |bridge| (bridge.min_height)());
        let min_height = if self.full_bleed && host_h > 100. {
            host_h
        } else {
            self.min_height.max(host_h)
        };
        let full_bleed = self.full_bleed;
        let scene = div()
            .w_full()
            .min_h(px(min_height))
            .when(full_bleed, |d| d.h(px(min_height)).items_stretch())
            .when(!full_bleed, |d| d.items_center().justify_center())
            .flex_none()
            .flex()
            .flex_col()
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
            .when(full_bleed, |e| e.overflow_hidden())
            .when(!full_bleed, |e| e.overflow_y_scroll())
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
