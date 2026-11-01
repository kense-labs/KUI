use super::{app, click, tick};
use gpui::{Modifiers, point, px};
use kui::components::{
    alert::{Alert, Severity},
    toast::Toast,
};
use std::time::Duration;

#[test]
fn replacing_a_toast_cancels_the_previous_dismissal_and_hover_pauses_the_new_one() {
    let mut app = app();
    let (toast, w) = app.add_window_view(|_, _| Toast::new());
    toast.update(w, |s, cx| {
        s.show(
            "First",
            Some("First action".into()),
            Duration::from_millis(650),
            cx,
        )
    });
    tick(w, 300);
    toast.update(w, |s, cx| {
        s.show(
            "Second",
            Some("Undo".into()),
            Duration::from_millis(1500),
            cx,
        )
    });
    tick(w, 500);
    let undo = w
        .debug_bounds("Undo")
        .expect("replacement survives the first deadline");
    w.simulate_mouse_move(undo.center(), None, Modifiers::default());
    tick(w, 1);
    tick(w, 1800);
    assert!(w.debug_bounds("Undo").is_some());
    w.simulate_mouse_move(point(px(5.), px(700.)), None, Modifiers::default());
    tick(w, 1);
    tick(w, 1100);
    tick(w, 1700);
    assert!(w.debug_bounds("Undo").is_none());
}
#[test]
fn resolved_alert_removes_the_recovery_action() {
    let mut app = app();
    let (alert, w) = app.add_window_view(|_, _| {
        Alert::new("Sync paused", "Saved locally.", Severity::Warning).with_action("Reconnect")
    });
    w.update(|_, cx| {
        cx.subscribe(&alert, |alert, _, cx| {
            alert.update(cx, |s, cx| {
                s.replace(
                    Alert::new("Synced", "All changes uploaded.", Severity::Success),
                    cx,
                )
            })
        })
        .detach()
    });
    click(w, "Reconnect");
    assert!(w.debug_bounds("Reconnect").is_none());
}

#[test]
fn card_escape_reverses_from_the_rendered_width() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| super::FamilyStory::new("card", w, cx));
    w.simulate_resize(gpui::size(px(628.), px(520.)));
    tick(w, 16);
    click(w, "Lobby lounge");
    tick(w, 75);
    let width = |w: &mut gpui::VisualTestContext| {
        w.debug_bounds("card-quicklook")
            .expect("retained quick look")
            .size
            .width
    };
    let middle = width(w);
    assert!(middle > px(342.) && middle < px(480.));
    super::press_key(w, "escape");
    tick(w, 0);
    assert!((width(w) - middle).abs() < px(1.));
    tick(w, 1600);
    assert!(w.debug_bounds("card-quicklook").is_none());
    assert!(w.debug_bounds("Lobby lounge").is_some());
}

#[test]
fn step_validation_keeps_the_current_step_until_the_host_accepts() {
    use kui::components::stepper::{Step, StepEvent, Stepper};
    let valid = std::rc::Rc::new(std::cell::Cell::new(false));
    let revisions = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut app = app();
    let (control, w) = app.add_window_view(|_, _| {
        Stepper::new(
            vec![
                Step::new("Workspace", "Name the workspace").validate_with({
                    let valid = valid.clone();
                    move |_, _| (!valid.get()).then(|| "Enter a workspace name".into())
                }),
                Step::new("Team", "Invite collaborators"),
            ],
            0,
        )
    });
    w.update(|_, cx| {
        let revisions = revisions.clone();
        cx.subscribe(&control, move |_, e: &StepEvent, _| {
            if matches!(e, StepEvent::Changed(1)) {
                revisions.set(revisions.get() + 1)
            }
        })
        .detach()
    });
    click(w, "Continue");
    tick(w, 250);
    assert_eq!(revisions.get(), 0);
    valid.set(true);
    click(w, "Continue");
    tick(w, 250);
    assert_eq!(revisions.get(), 1);
    assert!(w.debug_bounds("Finish").is_some());
}

#[test]
fn saving_the_story_draft_gives_the_notification_a_visible_surface() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| super::FamilyStory::new("toast", w, cx));
    w.simulate_resize(gpui::size(px(628.), px(520.)));
    tick(w, 16);
    click(w, "Save a draft");
    tick(w, 250);
    let surface = w
        .debug_bounds("toast-surface")
        .expect("saving a draft opens the toast");
    assert!(surface.size.width > px(300.) && surface.size.height >= px(74.));
    assert!(surface.top() >= w.debug_bounds("Save a draft").unwrap().bottom());
}

#[test]
fn completed_step_dot_is_centered_inside_its_hover_target() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| super::FamilyStory::new("stepper", w, cx));
    for width in [628., 288.] {
        w.simulate_resize(gpui::size(px(width), px(520.)));
        tick(w, 16);
        let target = w.debug_bounds("step-hit-0").unwrap();
        w.simulate_mouse_move(target.center(), None, Modifiers::default());
        tick(w, 16);
        for part in ["step-face-0", "step-fill-0"] {
            let center = w.debug_bounds(part).unwrap().center();
            assert!(
                (center.x - target.center().x).abs() <= px(0.5)
                    && (center.y - target.center().y).abs() <= px(0.5),
                "{part} misplaced at {width}px"
            );
        }
    }
}

#[test]
fn stepper_connectors_stay_on_axis_through_state_and_orientation_changes() {
    use kui::components::stepper::{Step, Stepper};
    let mut app = app();
    let (control, w) = app.add_window_view(|_, _| {
        Stepper::new(vec![
            Step::new("Account", ""),
            Step::new("A label that takes several lines at a narrow width to explain how to finish setting up the workspace", ""),
            Step::new("Team", ""),
            Step::new("Review", ""),
        ], 1)
    });
    let faces = ["step-face-0", "step-face-1", "step-face-2", "step-face-3"];
    let rails = ["step-rail-0", "step-rail-1", "step-rail-2"];
    let near = |a: gpui::Pixels, b: gpui::Pixels| {
        assert!((a - b).abs() <= px(0.6), "expected {a:?} to equal {b:?}");
    };
    for width in [288., 628.] {
        w.simulate_resize(gpui::size(px(width), px(520.)));
        for vertical in [true, false] {
            control.update(w, |s, cx| s.set_vertical(vertical, cx));
            tick(w, 16);
            let baseline = faces.map(|id| w.debug_bounds(id).unwrap());
            for step in [2, 0, 3] {
                control.update(w, |s, cx| s.select(step, cx));
                tick(w, 64);
                let dots = faces.map(|id| w.debug_bounds(id).unwrap());
                for (dot, initial) in dots.iter().zip(baseline) {
                    near(dot.size.width, px(28.));
                    near(dot.size.height, px(28.));
                    near(dot.center().x, initial.center().x);
                    near(dot.center().y, initial.center().y);
                }
                for (i, id) in rails.iter().enumerate() {
                    let line = w.debug_bounds(id).unwrap();
                    let (a, b) = (dots[i], dots[i + 1]);
                    if vertical {
                        near(line.size.width, px(2.));
                        near(a.center().x, line.center().x);
                        near(b.center().x, line.center().x);
                        near(line.top() - a.bottom(), px(6.));
                        near(b.top() - line.bottom(), px(6.));
                        if i != 1 {
                            near(line.size.height, px(12.));
                        }
                        if i == 1 && width == 288. {
                            assert!(line.size.height > px(12.));
                        }
                    } else {
                        near(line.size.height, px(2.));
                        near(a.center().y, line.center().y);
                        near(b.center().y, line.center().y);
                        near(line.left() - a.right(), px(8.));
                        near(b.left() - line.right(), px(8.));
                    }
                }
            }
        }
    }
}

#[test]
fn compact_stepper_keeps_a_full_disc_and_outer_halo_beside_the_form() {
    let mut app = app();
    let (_, w) = app.add_window_view(|w, cx| super::FamilyStory::new("stepper", w, cx));
    let near = |a: gpui::Pixels, b: gpui::Pixels| {
        assert!((a - b).abs() <= px(0.6), "expected {a:?} to equal {b:?}");
    };
    for width in [628., 288.] {
        w.simulate_resize(gpui::size(px(width), px(520.)));
        tick(w, 16);
        click(w, "Vertical");
        tick(w, 500);
        let first = w.debug_bounds("step-fill-0").unwrap();
        let current = w.debug_bounds("step-face-1").unwrap();
        let halo = w.debug_bounds("step-ring-1").unwrap();
        let line = w.debug_bounds("step-rail-0").unwrap();
        let body = w.debug_bounds("step-body").unwrap();
        let actions = w.debug_bounds("step-actions").unwrap();
        near(first.size.width, px(28.));
        near(first.size.height, px(28.));
        near(halo.size.width, px(36.));
        near(halo.size.height, px(36.));
        near(current.center().x, halo.center().x);
        near(current.center().y, halo.center().y);
        near(line.center().x, first.center().x);
        near(line.center().x, current.center().x);
        near(halo.top() - line.bottom(), px(2.));
        near(line.top() - first.bottom(), px(6.));
        near(body.left() - first.right(), px(20.));
        near(body.left(), actions.left());
        assert!(body.right() <= px(width) && actions.right() <= px(width));
        click(w, "Horizontal");
    }
}
