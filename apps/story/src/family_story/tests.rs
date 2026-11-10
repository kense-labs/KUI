//! Cross-input scenarios for the mounted GPUI examples.
use super::{FAMILY_IDS, FamilyStory, Theme};
use gpui::{
    Modifiers, MouseButton, TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext, px,
    size,
};
use std::time::Duration;
fn app() -> TestAppContext {
    let app = TestAppContext::single();
    app.update(kui::foundation::init);
    app
}

#[test]
fn theme_variant_routes_report_their_modes_and_trigger_bounds() {
    use super::ThemeBridge;
    use crate::StoryHost;
    use kui::components::theme_switch::{ThemeChange, Transition};
    use kui::foundation::ThemeMode;
    use std::cell::Cell;

    thread_local! {
        static REQUEST: Cell<Option<ThemeChange>> = const { Cell::new(None) };
    }
    for (id, mode) in [
        ("theme-switch-eclipse", Transition::Eclipse),
        ("theme-switch-split", Transition::Split),
        ("theme-switch-rise", Transition::Rise),
    ] {
        let mut app = app();
        app.update(|cx| {
            Theme::global_mut(cx).set_mode(ThemeMode::Light);
            cx.set_global(ThemeBridge(|request| REQUEST.set(Some(request))));
        });
        let (_, window) = app.add_window_view(|w, cx| StoryHost::new(Some(id), w, cx));
        let bounds = window.debug_bounds("Switch to dark mode").expect(id);
        assert_eq!(bounds.size.height, px(36.));
        window.update(|w, cx| w.focus_next(cx));
        press_key(window, "enter");
        let request = REQUEST.take().expect("theme request from keyboard");
        assert_eq!(request.next, ThemeMode::Dark);
        assert_eq!(request.mode, mode);
        assert_eq!(request.trigger, bounds);
        click(window, "Switch to dark mode");
        let request = REQUEST.take().expect("theme request from pointer");
        assert_eq!(request.mode, mode);
    }
}

#[test]
fn custom_accent_paints_the_saved_hex_on_the_mounted_primary_button() {
    use kui::foundation::{AccentPalette, ThemeMode};
    for (mode, hex) in [
        (ThemeMode::Light, 0x000000),
        (ThemeMode::Dark, 0xffffff),
        (ThemeMode::Light, 0xf9ed6f),
        (ThemeMode::Dark, 0x083142),
    ] {
        let mut app = app();
        let fill = gpui::rgb(hex).into();
        app.update(|cx| {
            let theme = Theme::global_mut(cx);
            theme.set_mode(mode);
            theme.set_accent(AccentPalette {
                light: fill,
                dark: fill,
            });
        });
        let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("button", w, cx));
        let primary = w.debug_bounds("Save changes").expect("primary button");
        w.update(|window, _| {
            let bounds = primary.scale(window.scale_factor());
            assert!(
                window
                    .painted_quads()
                    .iter()
                    .any(|q| q.bounds == bounds && q.background == gpui::Background::from(fill)),
                "primary button in {mode:?} must paint #{hex:06x}"
            );
        });
    }
}
fn click(w: &mut VisualTestContext, label: &'static str) {
    let p = w
        .debug_bounds(label)
        .unwrap_or_else(|| panic!("missing {label}"))
        .center();
    let p = if matches!(label, "Delete" | "Cancel" | "Retry" | "Undo")
        && w.debug_bounds("confirm-shell")
            .is_some_and(|b| !b.contains(&p))
    {
        tick(w, 450);
        let target = w.debug_bounds(label).unwrap().center();
        assert!(
            w.debug_bounds("confirm-shell").unwrap().contains(&target),
            "answer must be revealed and clickable"
        );
        target
    } else {
        p
    };
    w.simulate_click(p, Modifiers::default())
}
fn press_key(w: &mut VisualTestContext, name: &'static str) {
    w.simulate_keystrokes(name);
    w.update(|w, cx| {
        w.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse(name).unwrap(),
            }),
            cx,
        );
    });
    w.run_until_parked();
}
fn tick(w: &mut VisualTestContext, ms: u64) {
    w.executor().advance_clock(Duration::from_millis(ms));
    w.run_until_parked();
    // The deterministic host has no display link: present a frame at the advanced time.
    w.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.refresh()
    });
    w.run_until_parked()
}
#[test]
fn split_keys_select_and_restore_focus() {
    let mut app = app();
    let (s, w) = app.add_window_view(|w, cx| FamilyStory::new("split-button", w, cx));
    click(w, "Publish more actions");
    w.simulate_keystrokes("down");
    press_key(w, "enter");
    assert_eq!(w.update(|_, cx| s.read(cx).message.clone()), "Draft saved");
    assert!(w.debug_bounds("Save as draft").is_none());
    w.simulate_keystrokes("down");
    assert!(w.debug_bounds("Save as draft").is_some());
    w.simulate_keystrokes("escape");
    assert!(w.debug_bounds("Save as draft").is_none());
}
#[test]
fn confirm_countdown_cancel_retry_and_undo() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("confirm-morph", w, cx));
    click(w, "Countdown");
    click(w, "Delete project");
    tick(w, 450); // The answer becomes pointer-visible as the pill opens.
    click(w, "Delete");
    tick(w, 2999);
    assert!(w.debug_bounds("Cancel").is_some());
    click(w, "Cancel");
    tick(w, 1200);
    assert!(w.debug_bounds("Delete project").is_some());
    click(w, "Fails");
    click(w, "Click");
    click(w, "Delete project");
    tick(w, 450); // The answer becomes pointer-visible as the pill opens.
    click(w, "Delete");
    tick(w, 850);
    assert!(w.debug_bounds("Retry").is_some());
    click(w, "Succeeds");
    click(w, "Retry");
    tick(w, 850);
    click(w, "Undo");
    tick(w, 850);
    assert!(w.debug_bounds("Delete project").is_some());
}

#[test]
fn confirm_success_disc_stays_round_as_the_translated_face_opens() {
    use kui::primitives::{control::UiLanguage, family::Paint};
    for (width, lang, trigger, answer, undo) in [
        (320., UiLanguage::Chinese, "删除项目", "删除", "撤销"),
        (
            700.,
            UiLanguage::English,
            "Delete project",
            "Delete",
            "Undo",
        ),
    ] {
        let mut app = app();
        app.update(|cx| cx.set_global(lang));
        let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("confirm-morph", w, cx));
        w.simulate_resize(size(px(width), px(520.)));
        click(w, trigger);
        tick(w, 500);
        click(w, answer);
        tick(w, 805);
        let mut last = 0.;
        for dt in [60, 110, 370] {
            tick(w, dt);
            let shell = w.debug_bounds("confirm-shell").expect("done face");
            let restore = w.debug_bounds(undo).expect("translated undo button");
            let diameter = w.update(|window, cx| {
                let scale = window.scale_factor();
                let success = Paint::get(cx).success;
                let disc = window
                    .painted_quads()
                    .into_iter()
                    .find(|q| {
                        q.background
                            .as_solid()
                            .is_some_and(|c| (c.h - success.h).abs() < 0.005 && c.a > 0.)
                            && shell.scale(scale).contains(&q.bounds.center())
                    })
                    .expect("green success disc");
                let width = disc.bounds.size.width.0 / scale;
                assert!((width - disc.bounds.size.height.0 / scale).abs() < 0.01);
                assert!(
                    (disc.bounds.center().y.0 / scale - f32::from(shell.center().y)).abs() < 0.6
                );
                width
            });
            assert!(diameter >= last);
            last = diameter;
            if dt == 370 {
                assert!((diameter - 16.).abs() < 0.01);
                assert!(restore.right() <= shell.right());
            }
        }
    }
}
#[test]
fn hold_requires_a_full_second() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("confirm-morph", w, cx));
    click(w, "Hold");
    click(w, "Delete project");
    tick(w, 220);
    let p = w.debug_bounds("Hold to delete").unwrap().center();
    w.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
    tick(w, 999);
    w.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
    tick(w, 100);
    assert!(w.debug_bounds("Hold to delete").is_some());
    w.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
    tick(w, 1000);
    w.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
    tick(w, 850);
    assert!(w.debug_bounds("Undo").is_some());
}
#[test]
fn narrow_touch_and_keyboard() {
    let targets = [
        "Upload file",
        "Save changes",
        "Publish more actions",
        "Zoom in",
        "Undo",
        "Forward",
        "Copy link",
        "Follow",
        "Open menu",
        "Delete project",
        "Switch to light mode",
    ];
    for (id, target) in FAMILY_IDS.into_iter().zip(targets) {
        let mut app = app();
        app.update(|cx| {
            let t = Theme::global_mut(cx);
            t.set_mode(kui::foundation::ThemeMode::Dark);
            t.reduced_motion = true;
        });
        let (s, w) = app.add_window_view(|w, cx| FamilyStory::new(id, w, cx));
        w.simulate_resize(size(px(320.), px(590.)));
        w.run_until_parked();
        let b = w
            .debug_bounds(target)
            .unwrap_or_else(|| panic!("{id}: {target}"));
        assert!(b.origin.x >= px(0.) && b.right() <= px(320.), "{id}: {b:?}");
        for phase in [TouchPhase::Started, TouchPhase::Ended] {
            w.simulate_event(TouchEvent {
                id: TouchId(2),
                phase,
                position: b.center(),
                ..Default::default()
            });
        }
        match id {
            "button" => assert!(w.debug_bounds("Upload file").is_some()),
            "action-button" => assert!(w.debug_bounds("Saving").is_some()),
            "split-button" => assert!(w.debug_bounds("Save as draft").is_some()),
            "button-group" => assert!(w.debug_bounds("110%").is_some()),
            "floating-button-group" | "expanding-button-group" => {
                assert!(!w.update(|_, cx| s.read(cx).message.is_empty()))
            }
            "copy-button" => {
                w.run_until_parked();
                assert!(w.debug_bounds("Copied").is_some())
            }
            "action-swap" => {
                tick(w, 20);
                assert!(w.debug_bounds("Following").is_some())
            }
            "icon-morph" => assert!(w.debug_bounds("Close menu").is_some()),
            "confirm-morph" => assert!(w.debug_bounds("Cancel").is_some()),
            "theme-switch" => assert_eq!(
                w.update(|_, cx| Theme::global(cx).mode),
                kui::foundation::ThemeMode::Light
            ),
            _ => unreachable!(),
        }
        w.simulate_keystrokes("tab");
    }
}

#[test]
fn same_arrow_and_outside_press_dismiss_menu() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("split-button", w, cx));
    click(w, "Publish more actions");
    assert!(w.debug_bounds("Save as draft").is_some());
    click(w, "Publish more actions");
    assert!(w.debug_bounds("Save as draft").is_none());
    click(w, "Publish more actions");
    w.simulate_click(gpui::point(px(15.), px(15.)), Modifiers::default());
    assert!(w.debug_bounds("Save as draft").is_none());
}
#[test]
fn connection_switch_stretches_on_press_and_glides_from_its_current_position() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("action-swap", w, cx));
    let hit = w.debug_bounds("Simulate a lost connection").unwrap();
    let thumb = |w: &mut VisualTestContext| {
        w.update(|window, _| {
            let scale = window.scale_factor();
            let area = hit.scale(scale);
            let quad = window
                .painted_quads()
                .into_iter()
                .find(|q| {
                    area.contains(&q.bounds.center())
                        && (q.bounds.size.height.0 / scale - 18.).abs() < 0.6
                        && q.corner_radii.top_left.0 > 0.
                })
                .expect("visible switch thumb");
            (
                quad.bounds.left().0 / scale,
                quad.bounds.size.width.0 / scale,
            )
        })
    };
    let (left, width) = thumb(w);
    w.simulate_mouse_down(hit.center(), MouseButton::Left, Modifiers::default());
    tick(w, 320);
    let held = thumb(w);
    assert!((held.0 - left).abs() < 1. && held.1 > width + 3.);
    w.simulate_mouse_up(hit.center(), MouseButton::Left, Modifiers::default());
    tick(w, 100);
    let moving = thumb(w).0;
    assert!(moving > left + 1. && moving < left + 17.);
    click(w, "Simulate a lost connection");
    assert!((thumb(w).0 - moving).abs() <= 1.);
    tick(w, 800);
    assert_eq!(thumb(w), (left, width));
}

#[test]
fn invited_only_after_success_and_momentary_states_reset() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("action-swap", w, cx));
    click(w, "Simulate a lost connection");
    click(w, "Send invite");
    tick(w, 1150);
    assert!(w.debug_bounds("Try again").is_some());
    assert!(w.debug_bounds("Sent").is_none());
    click(w, "Simulate a lost connection");
    click(w, "Try again");
    tick(w, 1150);
    assert!(w.debug_bounds("Sent").is_some());
    tick(w, 2000);
    assert!(w.debug_bounds("Send invite").is_some());
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("action-button", w, cx));
    click(w, "Save changes");
    click(w, "Saving");
    tick(w, 530);
    assert!(w.debug_bounds("Saved").is_some());
    tick(w, 2500);
    assert!(w.debug_bounds("Save changes").is_some());
}

#[test]
fn embedded_theme_waits_for_the_document_before_repainting() {
    let mut app = app();
    app.update(|cx| cx.set_global(super::ThemeBridge(|_| {})));
    let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("theme-switch", w, cx));
    click(w, "Switch to dark mode");
    assert_eq!(
        w.update(|_, cx| Theme::global(cx).mode),
        kui::foundation::ThemeMode::Light
    );
    assert!(w.debug_bounds("Switch to dark mode").is_some());
    w.update(|_, cx| {
        Theme::global_mut(cx).set_mode(kui::foundation::ThemeMode::Dark);
        cx.refresh_windows();
    });
    w.run_until_parked();
    assert!(w.debug_bounds("Switch to light mode").is_some());
}

#[test]
fn loading_button_keeps_keyboard_focus_and_suppresses_duplicate_commits() {
    use gpui::{Context, IntoElement, Render, Window};
    use kui::components::button::Button;
    struct Form {
        commits: usize,
        busy: bool,
    }
    impl Render for Form {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            Button::new("save")
                .label("Save")
                .loading(self.busy)
                .on_click(cx.listener(|v, _, _, cx| {
                    v.commits += 1;
                    v.busy = true;
                    cx.notify();
                }))
                .into_themed(cx)
        }
    }
    let mut app = app();
    let (form, w) = app.add_window_view(|_, _| Form {
        commits: 0,
        busy: false,
    });
    w.update(|w, cx| w.focus_next(cx));
    let before = w.update(|w, cx| w.focused(cx).expect("focusable form control"));
    press_key(w, "enter");
    w.run_until_parked();
    assert_eq!(form.read_with(w, |v, _| v.commits), 1);
    press_key(w, "space");
    press_key(w, "enter");
    click(w, "Save");
    assert_eq!(form.read_with(w, |v, _| v.commits), 1);
    assert_eq!(w.update(|w, cx| w.focused(cx)), Some(before));
}

#[test]
fn painted_highlight_matches_the_laid_out_segment() {
    use kui::primitives::family::Paint;
    // Includes the separator of the vertical toolbar and the end of a joined row.
    for width in [320., 628.] {
        for dark in [false, true] {
            for (id, targets, inset) in [
                ("button-group", ["Share", "More actions"], 3.),
                ("floating-button-group", ["Zoom in", "Hide layer"], 0.),
                ("expanding-button-group", ["Reply", "Forward"], 0.),
                ("split-button", ["Publish", "Publish more actions"], 3.),
            ] {
                let mut app = app();
                app.update(|cx| {
                    Theme::global_mut(cx).set_mode(if dark {
                        kui::foundation::ThemeMode::Dark
                    } else {
                        kui::foundation::ThemeMode::Light
                    });
                    Theme::global_mut(cx).tokens.radius.sm = px(2.);
                });
                let (_, w) = app.add_window_view(|w, cx| FamilyStory::new(id, w, cx));
                w.simulate_resize(size(px(width), px(520.)));
                for target in targets {
                    let hit = w.debug_bounds(target).expect(target);
                    w.simulate_mouse_move(hit.center(), None, Modifiers::default());
                    tick(w, 1000);
                    tick(w, 1000);
                    let expected = w.debug_bounds(target).unwrap().inset(px(inset));
                    w.update(|window, cx| {
                        let p = Paint::get(cx);
                        let bg = if id == "split-button" {
                            p.inverse.opacity(0.09)
                        } else {
                            p.hover
                        };
                        let scale = window.scale_factor();
                        let expected = expected.scale(scale);
                        let quads = window.painted_quads();
                        let fill = quads
                            .iter()
                            .find(|q| {
                                q.background == bg.into()
                                    && [
                                        q.bounds.left() - expected.left(),
                                        q.bounds.top() - expected.top(),
                                        q.bounds.right() - expected.right(),
                                        q.bounds.bottom() - expected.bottom(),
                                    ]
                                    .into_iter()
                                    .all(|d| d.0.abs() <= scale)
                            })
                            .unwrap_or_else(|| {
                                panic!("{id} at {width}px, dark={dark}: {target} highlight bounds")
                            });
                        let expected_radius = Theme::global(cx).tokens.radius.md.scale(scale);
                        assert_eq!(
                            fill.corner_radii,
                            gpui::Corners::all(expected_radius),
                            "{id}: {target} inner corners"
                        );
                    });
                }
            }
        }
    }
}

#[test]
fn sun_moon_tile_colors_match_the_selected_mode_in_each_page_theme() {
    use kui::{foundation::ThemeMode, primitives::family::Paint};

    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let mut app = app();
        app.update(|cx| Theme::global_mut(cx).set_mode(mode));
        let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("icon-morph", w, cx));
        let neighbor = w.debug_bounds("Open menu").unwrap();
        for (label, tile_mode) in [
            ("Switch tile to dark", ThemeMode::Light),
            ("Switch tile to light", ThemeMode::Dark),
        ] {
            let tile = w.debug_bounds(label).unwrap();
            w.update(|window, cx| {
                assert_eq!(Theme::global(cx).mode, mode);
                let scale = window.scale_factor();
                for (bounds, palette_mode) in [(tile, tile_mode), (neighbor, mode)] {
                    let color = Paint::for_mode(palette_mode).panel;
                    let expected = bounds.scale(scale);
                    let painted = window.painted_quads().iter().any(|quad| {
                        quad.background == color.into()
                            && quad.bounds.contains(&expected.center())
                            && (quad.bounds.size.width - expected.size.width).0.abs() <= scale
                    });
                    assert!(painted, "{tile_mode:?} tile on a {mode:?} page");
                }
            });
            click(w, label);
            tick(w, 200);
        }
    }
}

#[test]
fn standalone_faces_follow_the_hosts_radius_token() {
    for outer_radius in [8., 11.] {
        for (id, labels) in [
            (
                "button",
                &[
                    "Save changes",
                    "Secondary",
                    "Delete",
                    "Add item",
                    "Continue",
                    "Large",
                    "Disabled",
                ][..],
            ),
            ("action-button", &["Save changes"][..]),
            ("action-swap", &["Follow"][..]),
            ("copy-button", &["Copy link"][..]),
            ("icon-morph", &["Open menu"][..]),
            ("confirm-morph", &["Delete project"][..]),
            ("theme-switch", &["Switch to dark mode"][..]),
        ] {
            let mut app = app();
            app.update(|cx| Theme::global_mut(cx).tokens.radius.md = px(outer_radius));
            let (_, w) = app.add_window_view(|w, cx| FamilyStory::new(id, w, cx));
            for label in labels {
                let hit = w.debug_bounds(label).expect(label);
                w.update(|window, _| {
                    let scale = window.scale_factor();
                    let expected = hit.scale(scale);
                    let corners = gpui::Corners::all(px(outer_radius).scale(scale));
                    let painted = window.painted_quads().iter().any(|quad| {
                        let offsets = [
                            quad.bounds.left() - expected.left(),
                            quad.bounds.top() - expected.top(),
                            quad.bounds.right() - expected.right(),
                            quad.bounds.bottom() - expected.bottom(),
                        ];
                        !quad.background.is_transparent()
                            && quad.corner_radii == corners
                            && offsets.iter().all(|d| d.0.abs() <= scale)
                    });
                    assert!(
                        painted,
                        "{id}: {label} should paint {outer_radius}px corners"
                    );
                });
            }
        }
    }
}

mod workspace;

mod insights;
