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
                    })
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
                        let expected_radius = Theme::global(cx).tokens.radius.sm.scale(scale);
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
