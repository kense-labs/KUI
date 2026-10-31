//! End-to-end scenarios against the same entities installed by consumers.
use super::{app, click, press_key, tick};
use gpui::{Focusable, Modifiers, MouseButton, VisualTestContext, div, point, prelude::*, px};
use kui::{
    components::{
        combobox::Combobox,
        dialog::Dialog,
        popover::Popover,
        resizable_panels::{Pane, ResizablePanels},
        select::Select,
        switch::Switch,
        tabs::{TabPage, Tabs},
    },
    primitives::{
        editor::{Editor, InputMode},
        picker::PickerOption,
    },
};
#[test]
fn a_modal_cycles_in_both_directions_and_restores_focus() {
    let mut app = app();
    let (s, w) = app.add_window_view(|_, cx| {
        Dialog::new("publish", "Review", "Publish", cx)
            .with_cancel("Cancel")
            .with_confirm("Publish")
    });
    click(w, "Review");
    tick(w, 550);
    let focused = |w: &mut VisualTestContext| w.update(|w, cx| w.focused(cx).unwrap());
    let first = focused(w);
    w.simulate_keystrokes("shift-tab");
    let last = focused(w);
    assert_ne!(first, last);
    w.simulate_keystrokes("tab");
    assert_eq!(focused(w), first);
    w.simulate_keystrokes("tab tab");
    assert_eq!(focused(w), last);
    w.simulate_keystrokes("escape");
    assert!(!s.read_with(w, |s, _| s.is_open()));
    assert!(w.update(|w, cx| s.focus_handle(cx).is_focused(w)));
}
#[test]
fn anchored_context_dismisses_outside_and_reopens_from_keyboard() {
    let mut app = app();
    let (s, w) = app
        .add_window_view(|_, cx| Popover::new("why", "Why?", "Checks", cx).with_cancel("Got it"));
    click(w, "Why?");
    tick(w, 600);
    assert!(w.debug_bounds("Got it").is_some());
    w.simulate_click(point(px(2.), px(700.)), Modifiers::default());
    assert!(!s.read_with(w, |s, _| s.is_open()));
    tick(w, 550);
    w.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    press_key(w, "enter");
    assert!(s.read_with(w, |s, _| s.is_open()));
    w.simulate_keystrokes("escape");
    assert!(w.update(|w, cx| s.focus_handle(cx).is_focused(w)));
}
fn models() -> Vec<PickerOption> {
    vec![
        PickerOption::new("fast", "Swift"),
        PickerOption::new("offline", "Unavailable").disabled(),
        PickerOption::new("deep", "Reasoning").with_keywords(["think", "推理"]),
    ]
}
#[test]
fn model_search_can_cancel_or_commit_a_keyword_match() {
    let mut app = app();
    let (s, w) = app.add_window_view(|w, cx| {
        Combobox::new("model", "Model", models(), w, cx).with_value("fast")
    });
    click(w, "Model");
    w.simulate_input("think");
    tick(w, 390);
    assert!(w.debug_bounds("Reasoning").is_some());
    assert!(w.debug_bounds("Swift").is_none());
    w.simulate_keystrokes("escape");
    assert_eq!(
        s.read_with(w, |v, _| v.value().map(str::to_owned)),
        Some("fast".into())
    );
    assert!(w.update(|w, cx| s.focus_handle(cx).is_focused(w)));
    w.simulate_input("推理");
    w.simulate_keystrokes("down enter");
    assert!(!s.read_with(w, |v, _| v.is_open()));
    assert_eq!(
        s.read_with(w, |v, _| v.value().map(str::to_owned)),
        Some("deep".into())
    );
}
#[test]
fn select_skips_unavailable_options_and_commits_one_keyboard_press() {
    let mut app = app();
    let (s, w) = app
        .add_window_view(|w, cx| Select::new("model", "Model", models(), w, cx).with_value("fast"));
    w.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    press_key(w, "space");
    assert!(s.read_with(w, |s, _| s.is_open()));
    w.simulate_keystrokes("down");
    press_key(w, "enter");
    assert!(!s.read_with(w, |s, _| s.is_open()));
    assert_eq!(
        s.read_with(w, |v, _| v.value().map(str::to_owned)),
        Some("deep".into())
    );
}
#[test]
fn switching_tabs_preserves_draft_and_selection() {
    let mut app = app();
    let mut retained = None;
    let (s, w) = app.add_window_view(|w, cx| {
        let ed = cx.new(|cx| Editor::new("Draft", InputMode::Single, w, cx));
        retained = Some(ed.clone());
        Tabs::new(
            "wiki",
            "Wiki",
            vec![
                TabPage::new("edit", "Edit", move |_, _| {
                    div().w(px(300.)).h(px(44.)).child(ed.clone())
                }),
                TabPage::new("preview", "Preview", |_, _| div().child("Preview")),
            ],
            cx,
        )
    });
    click(w, "Draft");
    w.simulate_input("draft");
    w.simulate_keystrokes("shift-left shift-left");
    click(w, "Preview");
    tick(w, 650);
    assert_eq!(
        s.read_with(w, |s, _| s.value().map(str::to_owned)),
        Some("preview".into())
    );
    w.simulate_keystrokes("left");
    tick(w, 650);
    let ed = retained.unwrap();
    w.update(|w, cx| ed.focus_handle(cx).focus(w, cx));
    w.simulate_input("wn");
    assert_eq!(ed.read_with(w, |e, _| e.value().to_owned()), "drawn");
}
#[test]
fn dragging_releases_content_and_keyboard_can_restore_a_collapsed_pane() {
    let mut app = app();
    let activated = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = activated.clone();
    let (_, w) = app.add_window_view(|_, cx| {
        ResizablePanels::new(
            "layout",
            "Layout",
            vec![
                Pane::new("files", "Files", 30., |_, _, _| div().child("Files")).collapsible(),
                Pane::new("edit", "Editor", 70., move |_, _, _| {
                    let count = observed.clone();
                    div()
                        .id("edit")
                        .debug_selector(|| "Edit document".into())
                        .size_full()
                        .on_click(move |_, _, _| count.set(count.get() + 1))
                        .child("Edit document")
                }),
            ],
            cx,
        )
    });
    tick(w, 90);
    let bounds = |w: &mut VisualTestContext| w.debug_bounds("divider-files").unwrap().center();
    let start = bounds(w);
    let end = start + point(px(60.), px(0.));
    w.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    w.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
    w.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    tick(w, 800);
    assert!((bounds(w).x - end.x).abs() < px(1.));
    click(w, "Edit document");
    assert_eq!(activated.get(), 1);
    click(w, "divider-files");
    w.simulate_keystrokes("enter");
    tick(w, 800);
    assert!(bounds(w).x < px(2.));
    w.simulate_keystrokes("enter");
    tick(w, 800);
    assert!((bounds(w).x - end.x).abs() < px(1.));
}
#[test]
fn switch_and_empty_view_can_be_reversed() {
    let mut app = app();
    let (s, w) = app.add_window_view(|w, cx| Switch::new("digest", "Weekly digest", w, cx));
    click(w, "Weekly digest");
    assert!(s.read_with(w, |s, _| s.is_checked()));
    press_key(w, "space");
    assert!(!s.read_with(w, |s, _| s.is_checked()));
    let (_, w) = app.add_window_view(|w, cx| super::FamilyStory::new("empty-state", w, cx));
    click(w, "Create view");
    tick(w, 330);
    assert!(w.debug_bounds("Undo").is_some());
    click(w, "Undo");
    tick(w, 330);
    assert!(w.debug_bounds("Create view").is_some())
}
