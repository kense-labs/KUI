//! Regression scenarios for the same workspace examples used in the browser.
use super::{build, code_suggestion, connector_setup, patch_demo, provider_keys, team_switcher};
use gpui::{Modifiers, TestAppContext, VisualTestContext};
use std::time::Duration;
pub(super) fn click(w: &mut VisualTestContext, label: &'static str) {
    let point = w
        .debug_bounds(label)
        .unwrap_or_else(|| panic!("Missing {label}"))
        .center();
    w.simulate_click(point, Modifiers::default());
    w.run_until_parked();
}
pub(super) fn frame(w: &mut VisualTestContext, ms: u64) {
    w.executor().advance_clock(Duration::from_millis(ms));
    w.run_until_parked();
    w.update(|win, cx| {
        win.simulate_next_frame(cx);
        win.refresh()
    });
    w.run_until_parked()
}
#[test]
fn operations_credential_remove_can_be_undone() {
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (demo, w) = app.add_window_view(|w, cx| build("provider-keys", w, cx));
    let control = w.update(|_, cx| {
        demo.read(cx)
            .view
            .clone()
            .downcast::<provider_keys::CredentialPanel>()
            .ok()
            .unwrap()
    });
    let mask = w.update(|_, cx| {
        control
            .read(cx)
            .providers
            .iter()
            .map(|p| p.masked.clone())
            .collect::<Vec<_>>()
    });
    click(w, "Remove key · Anthropic");
    assert!(w.debug_bounds("Undo removal 0").is_some());
    click(w, "Undo removal 0");
    w.update(|_, cx| {
        assert_eq!(
            mask,
            control
                .read(cx)
                .providers
                .iter()
                .map(|p| p.masked.clone())
                .collect::<Vec<_>>()
        )
    });
}
#[test]
fn operations_failure_retry_and_confirm_disconnect() {
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (demo, w) = app.add_window_view(|w, cx| build("connector-setup", w, cx));
    let control = w.update(|_, cx| {
        demo.read(cx)
            .view
            .clone()
            .downcast::<connector_setup::ConnectionWizard>()
            .ok()
            .unwrap()
    });
    click(w, "Fail next connection");
    click(w, "Connect");
    frame(w, 950);
    assert!(w.debug_bounds("Retry").is_some());
    click(w, "Retry");
    frame(w, 950);
    let count = w.update(|_, cx| {
        control
            .read(cx)
            .services
            .iter()
            .filter(|s| s.state == connector_setup::ConnectionState::Connected)
            .count()
    });
    assert_eq!(count, 3);
    click(w, "Connected");
    frame(w, 340);
    click(w, "Disconnect");
    assert!(w.debug_bounds("Confirm disconnect").is_some());
    click(w, "Confirm disconnect");
    w.update(|_, cx| {
        assert_eq!(
            control
                .read(cx)
                .services
                .iter()
                .filter(|s| s.state == connector_setup::ConnectionState::Connected)
                .count(),
            2
        )
    });
}
#[test]
fn operations_patch_accept_and_reject_exclude_decided_hunks() {
    let mut app = TestAppContext::single();
    app.update(|cx| {
        kui::foundation::init(cx);
        kui::Theme::global_mut(cx).reduced_motion = true;
    });
    let (demo, w) = app.add_window_view(|w, cx| build("code-suggestion", w, cx));
    let control = w.update(|_, cx| {
        demo.read(cx)
            .view
            .clone()
            .downcast::<patch_demo::PatchDemo>()
            .ok()
            .unwrap()
            .read(cx)
            .review()
            .clone()
    });
    click(w, "Accept change at line 3");
    click(w, "Reject all");
    w.update(|_, cx| {
        let hunks = &control.read(cx).hunks;
        assert_eq!(
            hunks
                .iter()
                .filter(|h| h.decision == code_suggestion::Decision::Accepted)
                .count(),
            1
        );
        assert_eq!(
            hunks
                .iter()
                .filter(|h| h.decision == code_suggestion::Decision::Rejected)
                .count(),
            2
        )
    });
}

#[test]
fn operations_patch_stop_drops_incomplete_lines_and_retry_starts_fresh() {
    use code_suggestion::PatchPhase;
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (demo, w) = app.add_window_view(|w, cx| build("code-suggestion", w, cx));
    let control = w.update(|_, cx| {
        demo.read(cx)
            .view
            .clone()
            .downcast::<patch_demo::PatchDemo>()
            .ok()
            .unwrap()
            .read(cx)
            .review()
            .clone()
    });
    frame(w, 0);
    frame(w, 800);
    frame(w, 38);
    w.update(|_, cx| {
        let review = control.read(cx);
        assert_eq!(review.phase(), &PatchPhase::Streaming);
        assert_eq!(review.hunks[0].after.len(), 1);
    });
    click(w, "Stop");
    frame(w, 500);
    w.update(|_, cx| {
        assert_eq!(control.read(cx).phase(), &PatchPhase::Ready);
        assert!(control.read(cx).hunks.is_empty());
    });
    click(w, "Error");
    click(w, "Retry");
    w.update(|_, cx| assert_eq!(control.read(cx).phase(), &PatchPhase::Thinking));
}
#[test]
fn operations_workspace_selection_closes_menu() {
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (demo, w) = app.add_window_view(|w, cx| build("team-switcher", w, cx));
    let control = w.update(|_, cx| {
        demo.read(cx)
            .view
            .clone()
            .downcast::<team_switcher::WorkspacePicker>()
            .ok()
            .unwrap()
    });
    frame(w, 250);
    click(w, "Vercel");
    w.update(|_, cx| assert_eq!(control.read(cx).current, 1));
    frame(w, 650);
    assert!(w.debug_bounds("Create workspace").is_none());
}

#[test]
fn operations_mount_and_click_from_each_gallery_entry() {
    use crate::family_story::FamilyStory;
    let scenarios = [
        ("chat-thread", "Go offline", "Go online"),
        ("starter-prompts", "Write", "Reply to the launch thread"),
        ("ai-translate", "Translate", "English"),
        ("agent-timeline", "Pause", "Play"),
        ("context-window", "Attach file", "Compact now"),
        (
            "agent-status",
            "Coding agent",
            "Coding agent · Inspect workspace",
        ),
        ("tool-call", "Simulate failure", "Retry"),
        ("agent-plan", "Add step", "Save step"),
        ("reasoning-block", "Interrupted", "Interrupted"),
        ("source-cards", "Failed", "Retry"),
        ("generation-stats", "Details", "Details"),
        ("comment-thread", "Resolve", "Reopen"),
        ("code-suggestion", "Accept change at line 3", "Reject all"),
        ("model-card", "Compare", "Compare"),
        ("provider-keys", "Rotate key · Anthropic", "Save provider 0"),
        ("connector-setup", "Connected", "Disconnect"),
        ("ai-command", "Ask AI", "Ask"),
        ("share-access", "Done", "Share"),
        ("team-switcher", "Create workspace", "Workspace name"),
    ];
    for (id, action, after) in scenarios {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            if id == "code-suggestion" {
                kui::Theme::global_mut(cx).reduced_motion = true;
            }
        });
        let (_view, w) = app.add_window_view(|w, cx| FamilyStory::new(id, w, cx));
        click(w, action);
        frame(w, 185);
        assert!(
            w.debug_bounds(after).is_some(),
            "{id}: {after} missing after {action}"
        );
    }
}

#[test]
fn operations_detail_gap_animates_from_the_closed_height() {
    use crate::family_story::FamilyStory;
    use gpui::{px, size};

    fn height(w: &mut VisualTestContext) -> f32 {
        f32::from(
            w.debug_bounds("generation-readout")
                .expect("readout panel should be painted")
                .size
                .height,
        )
    }

    for reduced in [false, true] {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            kui::Theme::global_mut(cx).reduced_motion = reduced;
        });
        let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("generation-stats", w, cx));
        w.simulate_resize(size(px(628.), px(980.)));
        frame(w, 80);
        let closed = height(w);
        click(w, "Details");
        frame(w, 0);
        let first = height(w);
        frame(w, 64);
        let middle = height(w);
        frame(w, 600);
        let open = height(w);

        assert!(open > closed + 12.);
        if reduced {
            assert!((first - open).abs() <= 1.);
        } else {
            assert!((first - closed).abs() <= 1.);
            assert!(middle > closed && middle < open);
        }
    }
}

#[test]
fn operations_compaction_sweeps_in_order_and_keeps_new_messages() {
    use super::context_window::CapacityBands;
    use gpui::{px, size};

    fn cell_alpha(w: &mut VisualTestContext) -> Vec<f32> {
        let grid = w
            .debug_bounds("capacity-grid")
            .expect("token grid is painted");
        let side = f32::from((grid.size.width - px(38.)) / 20.);
        w.update(|w, _| {
            let scale = w.scale_factor();
            let mut cells: Vec<_> = w
                .painted_quads()
                .into_iter()
                .filter_map(|q| {
                    let bounds = q.bounds;
                    let solid = q.background.as_solid()?;
                    let center = bounds.center();
                    (grid.contains(&gpui::point(px(center.x.0 / scale), px(center.y.0 / scale)))
                        && solid.a > 0.
                        && (bounds.size.height.0 / scale - side).abs() < 0.6
                        && (bounds.size.width.0 / scale - side).abs() < 0.6)
                        .then_some((bounds.top().0, bounds.left().0, solid.a))
                })
                .collect();
            cells.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
            assert_eq!(cells.len(), 200);
            cells.into_iter().map(|c| c.2).collect()
        })
    }

    for reduced in [false, true] {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            kui::Theme::global_mut(cx).reduced_motion = reduced;
        });
        let (gallery, w) = app.add_window_view(|w, cx| build("context-window", w, cx));
        w.simulate_resize(size(px(628.), px(980.)));
        frame(w, 80);
        let control = w.update(|_, cx| {
            gallery
                .read(cx)
                .view
                .clone()
                .downcast::<CapacityBands>()
                .ok()
                .unwrap()
        });
        click(w, "Compact now");
        frame(w, 96);
        let alpha = cell_alpha(w);
        if reduced {
            assert!(alpha.iter().all(|alpha| *alpha > 0.99));
        } else {
            assert!(
                alpha[29] < 0.9 && alpha[80] > 0.99,
                "the oldest affected cells should dim first"
            );
        }
        let used = w.update(|_, cx| control.read(cx).used());
        click(w, "Send message");
        frame(w, 28);
        assert_eq!(w.update(|_, cx| control.read(cx).used()), used + 21000);
        let before = cell_alpha(w);
        click(w, "Compact now");
        frame(w, 0);
        let after = cell_alpha(w);
        assert!(
            before
                .iter()
                .zip(after.iter())
                .all(|(a, b)| (a - b).abs() < 0.06)
        );
        frame(w, 1600);
        assert!(cell_alpha(w).iter().all(|a| *a > 0.99));
    }
}

#[test]
fn operations_settled_comment_toolbars_paint_all_four_corners() {
    use crate::family_story::FamilyStory;
    use gpui::{px, size};
    use kui::{
        Theme, ThemeMode,
        primitives::{control::UiLanguage, family::Paint},
    };

    for (width, mode) in [
        (320., ThemeMode::Dark),
        (628., ThemeMode::Dark),
        (628., ThemeMode::Light),
    ] {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            Theme::global_mut(cx).set_mode(mode);
            cx.set_global(UiLanguage::Chinese);
        });
        let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("comment-thread", w, cx));
        w.simulate_resize(size(px(width), px(980.)));
        frame(w, 600);
        for label in ["回复", "编辑", "删除", "收起回复"] {
            let hit = w.debug_bounds(label).unwrap();
            w.simulate_mouse_move(hit.center(), None, Modifiers::default());
            frame(w, 0);
            w.update(|win, cx| {
                let expected = hit.scale(win.scale_factor());
                let quads = win.painted_quads();
                let fill = quads
                    .iter()
                    .find(|quad| {
                        quad.background == Paint::get(cx).hover.into() && quad.bounds == expected
                    })
                    .unwrap_or_else(|| panic!("Missing {label} hover at {width}px"));
                let clip = fill.content_mask.bounds;
                assert!(
                    clip.left() <= expected.left()
                        && clip.top() <= expected.top()
                        && clip.right() >= expected.right()
                        && clip.bottom() >= expected.bottom(),
                    "{label}: hover/focus surface clipped at {width}px"
                );
                assert!(
                    fill.corner_radii.bottom_left.0 > 0. && fill.corner_radii.bottom_right.0 > 0.
                );
            });
        }
    }
}

#[test]
fn workspace_forms_keep_search_and_return_focus_in_escape_order() {
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (_demo, w) = app.add_window_view(|w, cx| build("team-switcher", w, cx));
    click(w, "Find a workspace");
    w.simulate_input("Fig");
    frame(w, 180);
    assert!(w.debug_bounds("Figma").is_some() && w.debug_bounds("Stripe").is_none());
    click(w, "Create workspace");
    assert!(w.debug_bounds("Workspace name").is_some());
    assert!(w.debug_bounds("Find a workspace").is_none());
    w.simulate_input("Unsaved");
    w.simulate_keystrokes("escape");
    frame(w, 200);
    assert!(w.debug_bounds("Figma").is_some() && w.debug_bounds("Stripe").is_none());
    // The restored search receives the next key without a click.
    w.simulate_input("x");
    frame(w, 0);
    assert!(w.debug_bounds("Figma").is_none());
    w.simulate_keystrokes("escape");
    frame(w, 0);
    assert!(w.debug_bounds("Stripe").is_some());
    w.simulate_keystrokes("escape");
    frame(w, 600);
    assert!(w.debug_bounds("Find a workspace").is_none());
    w.simulate_keystrokes("enter");
    w.update(|win, cx| {
        win.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse("enter").unwrap(),
            }),
            cx,
        )
    });
    frame(w, 600);
    assert!(w.debug_bounds("Find a workspace").is_some());
}

#[test]
fn workspace_forms_emit_once_and_replace_the_previous_story_status() {
    use std::{cell::RefCell, rc::Rc};
    use team_switcher::{WorkspaceEvent, WorkspacePicker};
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (demo, w) = app.add_window_view(|w, cx| build("team-switcher", w, cx));
    let control = w.update(|_, cx| {
        demo.read(cx)
            .view
            .clone()
            .downcast::<WorkspacePicker>()
            .ok()
            .unwrap()
    });
    let initial_count = w.update(|_, cx| control.read(cx).teams.len());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let _sub = w.update(|_, cx| {
        cx.subscribe(&control, {
            let seen = seen.clone();
            move |_, event, _| {
                seen.borrow_mut().push(match event {
                    WorkspaceEvent::Switch(id) => format!("switch {id}"),
                    WorkspaceEvent::Create(name) => format!("create {name}"),
                    WorkspaceEvent::Join(code) => format!("join {code}"),
                })
            }
        })
    });
    click(w, "Figma");
    frame(w, 600);
    w.update(|_, cx| assert_eq!(demo.read(cx).info, "Workspace · Figma"));
    click(w, "Figma");
    frame(w, 600);
    click(w, "Create workspace");
    w.simulate_keystrokes("enter");
    assert_eq!(seen.borrow().len(), 1);
    w.simulate_input("  Studio North  ");
    w.simulate_keystrokes("enter");
    frame(w, 600);
    w.update(|_, cx| assert_eq!(demo.read(cx).info, "Created workspace · Studio North"));
    click(w, "Studio North");
    frame(w, 600);
    click(w, "Join with invite");
    assert!(w.debug_bounds("Workspace name").is_none());
    w.simulate_input("KENSE-244");
    w.simulate_keystrokes("enter");
    frame(w, 600);
    w.update(|_, cx| {
        let p = control.read(cx);
        assert_eq!(p.teams.len(), initial_count + 2);
        assert_eq!(p.teams[p.current].name, "Invited workspace");
        assert_eq!(
            demo.read(cx).info,
            "Joined demo workspace · Invited workspace"
        );
    });
    assert_eq!(
        &*seen.borrow(),
        &["switch Figma", "create Studio North", "join KENSE-244"]
    );
}

#[test]
fn workspace_menu_keeps_its_frame_and_uses_the_control_radius() {
    use crate::family_story::FamilyStory;
    use gpui::{Corners, px, size};
    use kui::{Theme, ThemeMode, primitives::family::Paint};

    fn shell(w: &mut VisualTestContext, expect_field: bool) -> gpui::Bounds<gpui::ScaledPixels> {
        w.update(|win, cx| {
            let scale = win.scale_factor();
            let quads = win.painted_quads();
            let panel = quads
                .iter()
                .find(|q| {
                    let width = q.bounds.size.width.0 / scale;
                    width <= 360.1
                        && width > 240.
                        && q.bounds.size.height.0 / scale > 380.
                        && q.background == Paint::get(cx).panel.into()
                })
                .expect("workspace form or menu");
            let corners = Corners::all(Theme::global(cx).tokens.radius.md.scale(scale));
            assert_eq!(panel.corner_radii, corners);
            let field = quads.iter().find(|q| {
                (q.bounds.size.height.0 / scale - 36.).abs() < 0.6
                    && ((panel.bounds.size.width.0 - q.bounds.size.width.0) / scale - 18.).abs()
                        < 0.6
                    && q.background == Paint::get(cx).panel.into()
            });
            if expect_field {
                assert_eq!(field.expect("visible form field").corner_radii, corners);
            } else {
                assert!(field.is_none(), "search displays an underlined row");
            }
            panel.bounds
        })
    }

    for (width, mode) in [(628., ThemeMode::Light), (288., ThemeMode::Dark)] {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            Theme::global_mut(cx).set_mode(mode)
        });
        let (_, w) = app.add_window_view(|w, cx| FamilyStory::new("team-switcher", w, cx));
        w.simulate_resize(size(px(width), px(950.)));
        frame(w, 240);
        let list = shell(w, false);
        let search = w
            .debug_bounds("Find a workspace")
            .expect("workspace search editor");
        assert!(
            search.size.height >= px(32.),
            "search text, caret and pointer target must fit"
        );
        click(w, "Create workspace");
        frame(w, 240);
        assert_eq!(shell(w, true), list);
        click(w, "Back to workspaces");
        frame(w, 240);
        assert_eq!(shell(w, false), list);
        click(w, "Join with invite");
        frame(w, 240);
        assert_eq!(shell(w, true), list);
    }
}

#[test]
fn plan_completion_marks_keep_the_disc_and_check_centered() {
    use gpui::{px, size};
    use kui::{Theme, ThemeMode, components::agent_plan::TaskPlan, primitives::family::Paint};
    const DISCS: [&str; 6] = [
        "plan-disc-0",
        "plan-disc-1",
        "plan-disc-2",
        "plan-disc-3",
        "plan-disc-4",
        "plan-disc-5",
    ];
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            Theme::global_mut(cx).set_mode(mode);
        });
        let (plan, w) = app.add_window_view(|w, cx| {
            TaskPlan::new(
                "geometry",
                "Plan",
                (1..=6).map(|n| format!("Step {n}")).collect(),
                w,
                cx,
            )
        });
        w.simulate_scale_factor_change(if mode == ThemeMode::Dark { 2. } else { 1. });
        w.simulate_resize(size(px(628.), px(520.)));
        click(w, "Approve plan");
        frame(w, 16);
        let idle = DISCS.map(|id| w.debug_bounds(id).unwrap());
        plan.update(w, |p, cx| {
            p.completed = 6;
            cx.notify()
        });
        frame(w, 100);
        plan.update(w, |p, cx| {
            p.completed = 0;
            cx.notify()
        });
        frame(w, 40);
        plan.update(w, |p, cx| {
            p.completed = 6;
            cx.notify()
        });
        frame(w, 1500);
        let settled = DISCS.map(|id| w.debug_bounds(id).unwrap());
        assert_eq!(idle, settled);
        let slot = w.debug_bounds("plan-marker-0").unwrap();
        let glyph = w.debug_bounds("plan-check-0").unwrap();
        assert_eq!(slot.size, size(px(20.), px(20.)));
        assert_eq!(slot.center(), glyph.center());
        assert_eq!(glyph.center(), settled[0].center());
        w.update(|w, cx| {
            let scale = w.scale_factor();
            let green = Paint::get(cx).success.into();
            for disc in settled {
                assert_eq!(disc.size, size(px(14.), px(14.)));
                let center = disc.center().scale(scale);
                assert!(w.painted_quads().iter().any(|q| {
                    q.background == green
                        && q.bounds.center() == center
                        && (q.bounds.size.width.0 - 14. * scale).abs() < 0.01
                        && q.corner_radii.top_left.0 >= 7. * scale
                }));
            }
        });
    }
}

#[test]
fn credential_form_preserves_old_key_and_drops_canceled_tests() {
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (d, w) = app.add_window_view(|w, cx| build("provider-keys", w, cx));
    let panel = w.update(|_, cx| {
        d.read(cx)
            .view
            .clone()
            .downcast::<provider_keys::CredentialPanel>()
            .ok()
            .unwrap()
    });
    let mask =
        |w: &mut VisualTestContext| w.update(|_, cx| panel.read(cx).providers[0].masked.clone());
    click(w, "Rotate key · Anthropic");
    frame(w, 300);
    w.simulate_input("sk-ant-test-invalid");
    w.simulate_keystrokes("enter");
    frame(w, 700);
    assert!(w.debug_bounds("Save provider 0").is_some());
    assert_eq!(mask(w), "sk-ant-••••4f2a");
    w.simulate_keystrokes("ctrl-a");
    w.simulate_input("sk-ant-demo-AAAA");
    w.simulate_keystrokes("enter escape");
    frame(w, 700);
    assert_eq!(mask(w), "sk-ant-••••4f2a");
    w.simulate_keystrokes("enter");
    w.update(|win, cx| {
        win.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse("enter").unwrap(),
            }),
            cx,
        )
    });
    frame(w, 300);
    assert!(
        w.debug_bounds("Save provider 0").is_some(),
        "Escape restores the rotate trigger"
    );
    click(w, "Use sample key");
    w.simulate_keystrokes("enter");
    frame(w, 710);
    frame(w, 300);
    assert_eq!(mask(w), "sk-ant-••••7a2b");
    click(w, "Copy key · Anthropic");
    frame(w, 10);
    assert_eq!(
        w.update(|_, cx| cx.read_from_clipboard().and_then(|i| i.text()))
            .as_deref(),
        Some("sk-ant-sample-workspace-7a2b")
    );
}
#[test]
fn credential_removals_are_independent_and_commit_once_after_grace() {
    use std::{cell::RefCell, rc::Rc};
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (d, w) = app.add_window_view(|w, cx| build("provider-keys", w, cx));
    let panel = w.update(|_, cx| {
        d.read(cx)
            .view
            .clone()
            .downcast::<provider_keys::CredentialPanel>()
            .ok()
            .unwrap()
    });
    let removed = Rc::new(RefCell::new(Vec::<String>::new()));
    let seen = removed.clone();
    w.update(|_, cx| {
        cx.subscribe(&panel, move |_, a: &provider_keys::KeyAction, _| {
            if let provider_keys::KeyAction::Remove(id) = a {
                seen.borrow_mut().push(id.clone())
            }
        })
        .detach()
    });
    click(w, "Remove key · Anthropic");
    frame(w, 1500);
    click(w, "Remove key · OpenAI");
    click(w, "Undo removal 1");
    frame(w, 3400);
    assert!(removed.borrow().is_empty());
    frame(w, 200);
    assert_eq!(&*removed.borrow(), &["anthropic"]);
    frame(w, 2000);
    assert_eq!(removed.borrow().len(), 1);
    assert!(w.debug_bounds("Add key · Anthropic").is_some());
    assert!(w.debug_bounds("Copy key · OpenAI").is_some());
    click(w, "Add key · Anthropic");
    frame(w, 300);
    click(w, "Use sample key");
    w.simulate_keystrokes("enter");
    frame(w, 700);
    assert_eq!(
        w.update(|_, cx| panel.read(cx).providers[0].masked.clone()),
        "sk-ant-••••7a2b"
    );
}

#[test]
fn credential_reveal_remasks_and_status_retests_once() {
    use std::{cell::Cell, rc::Rc};
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (d, w) = app.add_window_view(|w, cx| build("provider-keys", w, cx));
    let panel = w.update(|_, cx| {
        d.read(cx)
            .view
            .clone()
            .downcast::<provider_keys::CredentialPanel>()
            .ok()
            .unwrap()
    });
    click(w, "Reveal key · Anthropic");
    assert!(w.debug_bounds("Hide key · Anthropic").is_some());
    frame(w, 4700);
    click(w, "Hide key · Anthropic");
    click(w, "Reveal key · Anthropic");
    frame(w, 400);
    assert!(
        w.debug_bounds("Hide key · Anthropic").is_some(),
        "old expiry cannot hide the new reveal"
    );
    frame(w, 4700);
    assert!(w.debug_bounds("Reveal key · Anthropic").is_some());
    let count = Rc::new(Cell::new(0));
    let copy = count.clone();
    w.update(|_, cx| {
        cx.subscribe(&panel, move |_, _: &provider_keys::RetestKey, _| {
            copy.set(copy.get() + 1)
        })
        .detach()
    });
    click(w, "Test connection · Mistral");
    click(w, "Test connection · Mistral");
    assert_eq!(count.get(), 1);
    frame(w, 710);
    w.update(|_, cx| {
        let p = &panel.read(cx).providers[3];
        assert!(!p.valid);
        assert_eq!(p.last_used, "Rejected just now")
    });
    click(w, "Test connection · Mistral");
    assert_eq!(count.get(), 2);
}

#[test]
fn status_steps_keep_bottom_padding_after_wrapping_or_progress_updates() {
    use gpui::{px, size};
    use kui::components::agent_status::{Step, WorkBadge};

    let mut app = TestAppContext::single();
    app.update(kui::init);
    let (control, w) = app.add_window_view(|_, _| {
        WorkBadge::new(
            "run",
            "Assistant",
            "Running",
            vec![Step {
                label: "Inspect workspace".into(),
                result: String::new(),
                complete: false,
            }],
        )
    });
    click(w, "Assistant · Running");
    for width in [622., 320.] {
        w.simulate_resize(size(px(width), px(760.)));
        w.update(|_, cx| control.update(cx, |s, cx| {
            s.steps[0].complete = true;
            s.steps[0].result = "Checked the workspace and all dependency manifests. The release checklist is available for review. ".repeat(3);
            cx.notify();
        }));
        for _ in 0..18 {
            frame(w, 32)
        }
        let panel = w.debug_bounds("agent-step-panel").unwrap();
        let result = w.debug_bounds("agent-step-result-0").unwrap();
        assert!(result.size.height > px(34.));
        assert!(
            (panel.bottom() - result.bottom() - px(17.)).abs() <= px(1.),
            "{panel:?} {result:?}"
        );
    }
    click(w, "Assistant · Running");
    for _ in 0..18 {
        frame(w, 32)
    }
    assert!(w.debug_bounds("agent-step-panel").is_none());
}

#[test]
fn workspace_list_scrolls_between_fixed_controls_and_clears_its_region() {
    use gpui::{Bounds, Pixels, PlatformInput, ScrollDelta, ScrollEvent, point, px, size};
    use kui::primitives::editor::EditorScrollRegion;
    use std::cell::Cell;
    thread_local! {
        static REGION: Cell<(Bounds<Pixels>, bool, bool)> = Cell::default();
    }
    let mut app = TestAppContext::single();
    app.update(|cx| {
        kui::init(cx);
        kui::Theme::global_mut(cx).reduced_motion = true;
        cx.set_global(EditorScrollRegion(|r, before, after| {
            REGION.set((r, before, after))
        }));
    });
    let (_, w) = app.add_window_view(|w, cx| build("team-switcher", w, cx));
    w.simulate_resize(size(px(288.), px(540.)));
    frame(w, 300);
    let (region, before, after) = REGION.get();
    assert_eq!(region.size.height, px(288.));
    assert!(!before && after);
    let heading = w.debug_bounds("Find a workspace").unwrap();
    let footer = w.debug_bounds("Create workspace").unwrap();
    assert_eq!(w.debug_bounds("Vercel").unwrap().size.height, px(48.));
    w.update(|win, cx| {
        win.dispatch_event(
            PlatformInput::Scroll(ScrollEvent {
                position: region.center(),
                delta: ScrollDelta::Pixels(point(px(0.), px(-600.))),
                ..Default::default()
            }),
            cx,
        );
    });
    frame(w, 40);
    assert_eq!(REGION.get(), (region, true, false));
    assert_eq!(heading, w.debug_bounds("Find a workspace").unwrap());
    assert_eq!(footer, w.debug_bounds("Create workspace").unwrap());
    assert!(w.debug_bounds("Platform").unwrap().bottom() <= region.bottom() + px(1.));
    click(w, "Find a workspace");
    w.simulate_keystrokes("home");
    frame(w, 30);
    assert_eq!(REGION.get(), (region, false, true));
    w.simulate_keystrokes("end");
    frame(w, 30);
    assert_eq!(REGION.get(), (region, true, false));
    w.simulate_input("Vercel");
    frame(w, 30);
    assert_eq!(REGION.get(), (region, false, false));
    assert!(w.debug_bounds("Vercel").is_some());
    w.simulate_keystrokes("escape");
    frame(w, 30);
    assert_eq!(REGION.get(), (region, false, true));
    w.simulate_resize(size(px(288.), px(360.)));
    frame(w, 40);
    let (short, before, after) = REGION.get();
    assert!(short.size.height >= px(48.) && short.size.height < px(288.));
    assert!(!before && after);
    assert_eq!(w.debug_bounds("Vercel").unwrap().size.height, px(48.));
    assert!(w.debug_bounds("Create workspace").unwrap().top() >= short.bottom());
    click(w, "Create workspace");
    frame(w, 300);
    assert_eq!(REGION.get(), (Bounds::default(), false, false));
    w.simulate_keystrokes("escape escape");
    frame(w, 300);
    assert_eq!(REGION.get(), (Bounds::default(), false, false));
}
