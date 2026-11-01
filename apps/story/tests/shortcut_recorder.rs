//! Exercise actual recorder hit regions, keyboard ownership and non-destructive conflict review.
use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use kui::components::shortcut_recorder::{ShortcutChanged, ShortcutPlatform, ShortcutRecorder};
use std::{cell::RefCell, rc::Rc};

#[test]
fn recorder_conflict_cancel_replace_clear_restore_and_layout() {
    let mut app = TestAppContext::single();
    app.update(|cx| {
        kui::init(cx);
        kui::foundation::Theme::global_mut(cx).reduced_motion = true;
    });
    let (field, w) = app.add_window_view(|_, cx| {
        ShortcutRecorder::new("Open command menu", Some("Mod+K".into()), cx)
            .with_conflicts(vec!["Option+N".into()])
    });
    let events = Rc::new(RefCell::new(Vec::new()));
    let saved = events.clone();
    let _sub = w.update(|_, cx| {
        cx.subscribe(&field, move |_, ev: &ShortcutChanged, _| {
            saved.borrow_mut().push(ev.0.clone());
        })
    });
    for width in [472., 240.] {
        w.simulate_resize(size(px(width), px(220.)));
        w.run_until_parked();
        let trigger = w.debug_bounds("Open command menu").unwrap();
        let clear = w.debug_bounds("Clear shortcut").unwrap();
        assert_eq!(trigger.top(), px(29.));
        assert_eq!(trigger.size.height, px(42.));
        assert_eq!(clear.size.width, px(32.));
        assert_eq!(clear.right(), px(width - 5.));
        assert!(trigger.right() <= clear.left());
    }
    let value =
        |w: &mut VisualTestContext| w.update(|_, cx| field.read(cx).value().map(str::to_owned));
    let click = |w: &mut VisualTestContext, s: &'static str| {
        let pos = w.debug_bounds(s).unwrap().center();
        w.simulate_click(pos, Modifiers::default());
        w.run_until_parked();
    };
    click(w, "Open command menu");
    w.simulate_keystrokes("alt-n");
    assert!(w.debug_bounds("Use anyway").is_some());
    assert_eq!(value(w).as_deref(), Some("Mod+K"));
    w.simulate_keystrokes("escape");
    assert!(events.borrow().is_empty());
    assert!(w.debug_bounds("Use anyway").is_none());
    click(w, "Open command menu");
    w.simulate_keystrokes("alt-n");
    click(w, "Use anyway");
    assert_eq!(value(w).as_deref(), Some("Alt+N"));
    click(w, "Clear shortcut");
    assert_eq!(value(w), None);
    click(w, "Reset shortcut");
    assert_eq!(value(w).as_deref(), Some("Mod+K"));
    assert_eq!(events.borrow().len(), 3);
    w.simulate_keystrokes("enter");
    assert!(w.update(|_, cx| field.read(cx).is_recording()));
    w.simulate_keystrokes("a");
    assert!(w.update(|_, cx| field.read(cx).is_recording()));
    w.simulate_keystrokes("tab");
    assert!(!w.update(|_, cx| field.read(cx).is_recording()));
    assert_eq!(events.borrow().len(), 3);
}

#[test]
fn recorder_draws_and_matches_platform_modifiers_in_system_order() {
    let mac = ShortcutPlatform::Mac;
    let win = ShortcutPlatform::Windows;
    assert_eq!(
        mac.spoken("Mod+Shift+Alt+Ctrl+."),
        "Control Option Shift Command Period"
    );
    assert_eq!(
        win.spoken("Win+Shift+Mod+Alt+."),
        "Control Alt Shift Win Period"
    );
    assert!(mac.matches("Option+N", "Alt+n"));
    assert!(win.matches("Control+K", "Mod+K"));
    assert!(!mac.matches("Ctrl+K", "Mod+K"));
}
