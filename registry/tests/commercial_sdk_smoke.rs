//! Commercial public-SDK integration smoke tests across KUI component families.

use gpui::{TestAppContext, UpdateGlobal, prelude::*, px, size};
use kui::{
    components::{
        analytics::AnalyticsScreen,
        app_shell_flat::{AppShellFlat, AppShellFlatEvent},
        brand_regions::BrandRegionsScreen,
        button::Button,
        fallback_chain::FallbackChain,
        files::FilesScreen,
        inline_diff::InlineDiff,
        kanban::KanbanScreen,
        metric_ticker::{MetricTicker, TickerMetric},
        mission_control::MissionControlBlock,
        realtime_stream::RealtimeStream,
        voice_orb::VoiceOrb,
    },
    foundation::{AccentPalette, Theme, ThemeMode},
    primitives::control::UiLanguage,
};

struct ButtonHost;

impl Render for ButtonHost {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div().child(
            Button::new("sdk-save")
                .primary()
                .label("Save")
                .into_themed(cx),
        )
    }
}

#[gpui::test]
fn foundation_theme_modes_accents_and_button_variants_render(cx: &mut TestAppContext) {
    cx.update(kui::init);
    cx.update(|app| {
        assert_eq!(Theme::global(app).mode, ThemeMode::Light);
        Theme::update_global(app, |t, _| {
            t.mode = ThemeMode::Dark;
            t.reduced_motion = true;
            t.set_accent(AccentPalette {
                light: gpui::rgb(0x245bc4).into(),
                dark: gpui::rgb(0x8bb4ff).into(),
            });
        });
        assert_eq!(Theme::global(app).mode, ThemeMode::Dark);
        assert!(Theme::global(app).reduced_motion);
    });

    let _win = cx.add_window(|_, _| ButtonHost);
}

#[gpui::test]
fn upgraded_screens_mount_cleanly_at_desktop_and_mobile_widths(cx: &mut TestAppContext) {
    cx.update(|app| {
        kui::init(app);
        app.set_global(UiLanguage::Chinese);
        Theme::update_global(app, |t, _| t.mode = ThemeMode::Dark);
    });

    let win = cx.open_window(size(px(894.), px(680.)), |_, cx| {
        AnalyticsScreen::new("sdk-analytics", cx)
    });
    win.update(cx, |_, w, _| w.refresh())
        .expect("refresh analytics");

    let _brand = cx.open_window(size(px(894.), px(680.)), |_, cx| {
        BrandRegionsScreen::new("sdk-brand-regions", cx)
    });
    let _kanban = cx.open_window(size(px(894.), px(680.)), |_, cx| {
        KanbanScreen::new("sdk-kanban", cx)
    });
    let _files = cx.open_window(size(px(894.), px(680.)), |_, cx| {
        FilesScreen::new("sdk-files", cx)
    });
    let flat = cx.add_window(|_, cx| AppShellFlat::new("sdk-flat-shell", cx));
    cx.update(|app| {
        let entity = flat.root(app).expect("flat root");
        app.subscribe(&entity, |_, _: &AppShellFlatEvent, _| {})
            .detach();
    });
}

#[gpui::test]
fn upgraded_multi_agent_and_arc_flagships_mount_under_reduced_motion(cx: &mut TestAppContext) {
    cx.update(|app| {
        kui::init(app);
        Theme::update_global(app, |t, _| {
            t.mode = ThemeMode::Dark;
            t.reduced_motion = true;
        });
    });

    let _mc = cx.add_window(|_, cx| MissionControlBlock::new("sdk-mc", cx));
    let _fc = cx.add_window(|_, cx| FallbackChain::new("sdk-fc", cx));
    let _id = cx.add_window(|_, cx| InlineDiff::new("sdk-id", cx));
    let _vo = cx.add_window(|_, cx| VoiceOrb::new(cx));
    let _rs = cx.add_window(|_, _| RealtimeStream::new("Checkout API"));
    let _mt = cx.add_window(|_, cx| {
        MetricTicker::new(
            "Product health",
            vec![
                TickerMetric::new("mrr", "MRR", 84210.0),
                TickerMetric::new("lat", "Latency", 182.0),
            ],
            cx,
        )
    });
}
