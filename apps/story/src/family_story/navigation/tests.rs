//! Keyboard, editing and local story regressions against mounted GPUI entities.
use super::{ConfirmPopover, ConfirmRequested, NavDemo};
use gpui::{KeyUpEvent, Keystroke, PlatformInput, px, size};
use gpui::{Modifiers, TestAppContext, VisualTestContext};
fn host() -> TestAppContext {
    let app = TestAppContext::single();
    app.update(|cx| {
        kui::foundation::init(cx);
        kui::foundation::Theme::global_mut(cx).reduced_motion = true
    });
    app
}
fn click(w: &mut VisualTestContext, label: &'static str) {
    let b = w
        .debug_bounds(label)
        .unwrap_or_else(|| panic!("missing {label}"));
    w.simulate_click(b.center(), Modifiers::default());
    w.run_until_parked()
}
#[test]
fn narrow_navigation_examples_mount() {
    let targets = [
        ("tooltip", "Project visibility information"),
        ("drawer", "Open details"),
        ("accordion", "What is included?"),
        ("breadcrumb", "Projects"),
        ("tree-view", "src"),
        ("confirm-popover", "Remove"),
        ("pagination", "Previous page"),
        ("filter-toolbar", "Add filter"),
        ("sortable-data-table", "Project"),
        ("reorderable-list", "Clarify onboarding"),
        ("expandable-card", "Launch checklist"),
        ("hover-card", "Jasmine Brooks"),
        ("expandable-text", "Read more"),
        ("toc-progress", "Why cold starts mattered"),
        ("code-block", "Copy code"),
        ("json-viewer", "Copy path"),
        ("data-grid", "Account"),
        ("lightbox-gallery", "Panorama I"),
    ];
    for (id, name) in targets {
        let mut app = host();
        let (_, w) = app.add_window_view(|w, cx| NavDemo::new(id, w, cx));
        w.simulate_resize(size(px(320.), px(570.)));
        w.run_until_parked();
        let b = w
            .debug_bounds(name)
            .unwrap_or_else(|| panic!("{id}: {name}"));
        assert!(
            b.size.width > px(0.) && b.left() >= px(-1.) && b.left() < px(320.),
            "{id}: {b:?}"
        )
    }
}
#[test]
fn drawer_dismisses_from_keyboard_and_restores_focus() {
    let mut app = host();
    let (_, w) = app.add_window_view(|w, cx| NavDemo::new("drawer", w, cx));
    w.simulate_resize(size(px(628.), px(520.)));
    w.run_until_parked();
    click(w, "Open details");
    let stage = w.debug_bounds("drawer-stage").unwrap();
    let shade = w.debug_bounds("drawer-backdrop").unwrap();
    for difference in [
        shade.left() - stage.left(),
        shade.top() - stage.top(),
        stage.right() - shade.right(),
        stage.bottom() - shade.bottom(),
    ] {
        assert!(
            (px(0.)..=px(2.)).contains(&difference),
            "contained drawer escaped its parent: {stage:?} {shade:?}"
        );
    }
    advance_frame(w, 320);
    let face = w.debug_bounds("drawer-sheet-face").unwrap();
    w.update(|win, _| {
        let scale = win.scale_factor();
        let quads = win.painted_quads();
        for surface in [stage, shade, face] {
            let bounds = surface.scale(scale);
            let quad = quads
                .iter()
                .find(|quad| quad.bounds == bounds && quad.corner_radii.top_left.0 > 0.)
                .expect("painted drawer surface");
            for corner in [
                quad.corner_radii.top_left,
                quad.corner_radii.top_right,
                quad.corner_radii.bottom_left,
                quad.corner_radii.bottom_right,
            ] {
                assert_eq!(corner.0 / scale, 8., "drawer corner at {surface:?}");
            }
        }
    });
    assert!(stage.contains(&w.debug_bounds("Close").unwrap().center()));
    w.simulate_keystrokes("escape");
    advance_frame(w, 160);
    assert!(w.debug_bounds("Close").is_none());
    w.simulate_keystrokes("enter");
    w.update(|win, cx| {
        win.dispatch_event(
            PlatformInput::KeyUp(KeyUpEvent {
                keystroke: Keystroke::parse("enter").unwrap(),
            }),
            cx,
        );
    });
    w.run_until_parked();
    assert!(w.debug_bounds("Close").is_some());
}
#[test]
fn typing_gate_rejects_incorrect_confirmation_and_stale_resolutions() {
    let mut app = host();
    let (c, w) = app.add_window_view(|w, cx| {
        ConfirmPopover::new(
            "Delete",
            "Type the project name",
            Some("Atlas".into()),
            w,
            cx,
        )
    });
    let tickets = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let store = tickets.clone();
    let _sub = w.update(|_, cx| {
        cx.subscribe(&c, move |_, request: &ConfirmRequested, _| {
            store.borrow_mut().push(request.0)
        })
    });
    for width in [622., 320.] {
        w.simulate_resize(size(px(width), px(560.)));
        w.update(|win, cx| win.simulate_next_frame(cx));
        w.run_until_parked();
        click(w, "Delete");
        w.update(|win, cx| win.simulate_next_frame(cx));
        let shell = w.debug_bounds("confirm-phrase-field").unwrap_or_else(|| {
            panic!(
                "phrase missing at {width}: {:?}",
                w.debug_bounds("Confirmation text")
            )
        });
        let content = w.debug_bounds("Confirmation text").unwrap();
        assert_eq!(shell.size.height, px(32.));
        assert_eq!(content.left() - shell.left(), px(13.));
        assert_eq!(shell.right() - content.right(), px(13.));
        for x in [shell.left() + px(2.), shell.right() - px(2.)] {
            w.update(|win, cx| win.blur(cx));
            w.simulate_click(gpui::point(x, shell.center().y), Modifiers::default());
            w.run_until_parked();
            w.update(|win, cx| win.simulate_next_frame(cx));
            w.simulate_keystrokes("cmd-a");
            w.simulate_input("At");
            w.simulate_keystrokes("enter");
            assert!(tickets.borrow().is_empty());
        }
        if width > 320. {
            click(w, "Cancel");
        }
    }
    click(w, "Confirm");
    assert!(tickets.borrow().is_empty());
    let shell = w.debug_bounds("confirm-phrase-field").unwrap();
    w.update(|win, cx| win.blur(cx));
    w.simulate_click(
        gpui::point(shell.left() + px(2.), shell.center().y),
        Modifiers::default(),
    );
    w.run_until_parked();
    w.update(|win, cx| win.simulate_next_frame(cx));
    w.simulate_keystrokes("cmd-a");
    w.simulate_input("Atlas");
    w.simulate_keystrokes("enter");
    w.run_until_parked();
    w.simulate_keystrokes("enter");
    w.run_until_parked();
    assert_eq!(tickets.borrow().len(), 1);
    let ticket = tickets.borrow()[0];
    c.update(w, |v, cx| v.resolve(ticket + 1, true, cx));
    c.update(w, |v, cx| v.resolve(ticket, false, cx));
    w.run_until_parked();
    click(w, "Retry");
    assert_eq!(tickets.borrow().len(), 2)
}
#[test]
fn tree_and_page_keyboard_changes_use_visible_order() {
    let mut app = host();
    let (v, w) = app.add_window_view(|w, cx| NavDemo::new("tree-view", w, cx));
    click(w, "components");
    assert!(w.debug_bounds("button.rs").is_none());
    w.simulate_keystrokes("right down");
    w.run_until_parked();
    assert_eq!(v.read_with(w, |s, _| s.notice.clone()), "button");
    let (v, w) = app.add_window_view(|w, cx| NavDemo::new("pagination", w, cx));
    click(w, "Next page");
    assert_eq!(v.read_with(w, |s, _| s.page), 3);
    click(w, "1");
    click(w, "Previous page");
    assert_eq!(v.read_with(w, |s, _| s.page), 1);
}
#[test]
fn grid_edits_backing_row_after_sort_and_fill_updates_only_selected_cells() {
    use kui::components::data_grid::{CellEdited, DataGrid};
    let mut app = host();
    let (grid, w) = app.add_window_view(|w, cx| {
        DataGrid::new(
            "Test",
            vec!["Name".into(), "Seats".into()],
            vec![
                vec!["Zulu".into(), "2".into()],
                vec!["Alpha".into(), "8".into()],
            ],
            w,
            cx,
        )
    });
    let edits = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let saved = edits.clone();
    let _sub = w.update(|_, cx| {
        cx.subscribe(&grid, move |_, ev: &CellEdited, _| {
            saved
                .borrow_mut()
                .push((ev.row, ev.column, ev.value.to_string()))
        })
    });
    click(w, "Name");
    click(w, "grid-cell-0-1");
    w.simulate_keystrokes("f2");
    w.simulate_input("12");
    w.simulate_keystrokes("enter");
    w.run_until_parked();
    assert_eq!(*edits.borrow(), vec![(1, 1, "12".to_owned())]);
    let start = w
        .debug_bounds("grid-fill")
        .expect("active fill handle")
        .center();
    let end = w.debug_bounds("grid-cell-1-1").unwrap().center();
    w.simulate_mouse_down(start, gpui::MouseButton::Left, Modifiers::default());
    w.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Modifiers::default());
    w.simulate_mouse_up(end, gpui::MouseButton::Left, Modifiers::default());
    w.run_until_parked();
    assert_eq!(edits.borrow().last().unwrap(), &(0, 1, "12".into()));
    click(w, "Search rows");
    w.simulate_input("alpha");
    w.run_until_parked();
    assert!(w.debug_bounds("grid-cell-1-1").is_none());
    click(w, "Reset");
    w.run_until_parked();
    assert!(w.debug_bounds("grid-cell-1-1").is_some());
    assert!(edits.borrow().contains(&(0, 1, "2".to_owned())));
    assert!(edits.borrow().contains(&(1, 1, "8".to_owned())));
}
#[test]
fn keyboard_reorder_keeps_the_handle_attached_to_its_item() {
    use kui::components::reorderable_list::{OrderChanged, OrderedItem, ReorderableList};
    let mut app = host();
    let (list, w) = app.add_window_view(|_, cx| {
        ReorderableList::new(
            vec![
                OrderedItem::new("a", "Alpha", "First"),
                OrderedItem::new("b", "Beta", "Second"),
            ],
            cx,
        )
    });
    let orders = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let saved = orders.clone();
    let _sub = w.update(|_, cx| {
        cx.subscribe(&list, move |_, ev: &OrderChanged, _| {
            saved
                .borrow_mut()
                .push(ev.0.iter().map(ToString::to_string).collect::<Vec<_>>())
        })
    });
    click(w, "Alpha");
    w.simulate_keystrokes("down up");
    assert_eq!(*orders.borrow(), vec![vec!["b", "a"], vec!["a", "b"]]);
}
#[test]
fn grid_sorts_formatted_amounts_and_text_in_a_consistent_order() {
    let mut app = host();
    let (grid, window) = app.add_window_view(|w, cx| {
        kui::components::data_grid::DataGrid::new(
            "Renewals",
            vec!["ARR".into()],
            ["$12,000", "Pending", "$240", "$960"]
                .map(|v| vec![v.into()])
                .to_vec(),
            w,
            cx,
        )
    });
    click(window, "ARR");
    window.update(|_, cx| {
        assert_eq!(
            grid.read(cx).csv(),
            "ARR\r\n$240\r\n$960\r\n\"$12,000\"\r\nPending\r\n"
        );
    });
}
#[test]
fn csv_follows_filtered_sorted_rows_visible_column_order_and_quotes() {
    use kui::components::data_grid::{DataGrid, GridExport};
    let mut app = host();
    let (grid, w) = app.add_window_view(|w, cx| {
        DataGrid::new(
            "Launch list",
            vec!["Name".into(), "Seats".into(), "Region".into()],
            vec![
                vec!["Zulu, Inc".into(), "2".into(), "EU".into()],
                vec!["Alpha \"Co\"".into(), "3".into(), "EU".into()],
                vec!["Other".into(), "4".into(), "US".into()],
            ],
            w,
            cx,
        )
    });
    let exports = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let output = exports.clone();
    let _subscription = w.update(|_, cx| {
        cx.subscribe(&grid, move |_, e: &GridExport, _| {
            output
                .borrow_mut()
                .push((e.filename.clone(), e.csv.clone()))
        })
    });
    click(w, "Filters");
    click(w, "Region filter");
    w.simulate_input("=EU");
    w.simulate_keystrokes("enter");
    w.run_until_parked();
    assert!(w.debug_bounds("grid-cell-2-0").is_none());
    click(w, "Name");
    click(w, "Columns");
    click(w, "Show column Region");
    click(w, "Move left Seats");
    click(w, "Columns");
    click(w, "Export");
    assert_eq!(
        *exports.borrow(),
        vec![(
            "launch-list.csv".to_string(),
            "Seats,Name\r\n3,\"Alpha \"\"Co\"\"\"\r\n2,\"Zulu, Inc\"\r\n".to_string()
        )]
    );
    click(w, "grid-cell-0-0");
    w.simulate_keystrokes("f2");
    w.simulate_input("9");
    w.simulate_keystrokes("enter");
    click(w, "Export");
    assert!(exports.borrow()[1].1.starts_with("Seats,Name\r\n9,"));
    click(w, "Reset");
    click(w, "Export");
    assert!(exports.borrow()[2].1.contains("Other,4,US\r\n"));
}
#[cfg(not(target_family = "wasm"))]
#[test]
fn native_story_export_saves_the_filtered_file_and_respects_cancel() {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    let mut app = host();
    let (_, w) = app.add_window_view(|w, cx| NavDemo::new("data-grid", w, cx));
    click(w, "Search rows");
    w.simulate_input("Northwind");
    w.run_until_parked();
    click(w, "Export");
    w.simulate_new_path_selection(|_| None);
    w.run_until_parked();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("kui-grid-test-{}-{nonce}.csv", std::process::id()));
    click(w, "Export");
    w.simulate_new_path_selection(|_| Some(path.clone()));
    w.run_until_parked();
    let contents = fs::read_to_string(&path).unwrap();
    fs::remove_file(path).unwrap();
    assert!(contents.starts_with("Account,Owner,Region,Stage,Seats,ARR,Health,Renews\r\n"));
    assert_eq!(contents.lines().count(), 1251);
    assert!(
        contents
            .lines()
            .skip(1)
            .all(|r| r.starts_with("Northwind Freight,"))
    );
}

#[test]
fn grid_headers_and_virtual_rows_share_edges_before_and_after_horizontal_scroll() {
    use gpui::{ScrollDelta, ScrollEvent, point};
    use kui::components::data_grid::DataGrid;
    let mut app = host();
    let (_, w) = app.add_window_view(|window, cx| {
        DataGrid::new(
            "Grid",
            vec![
                "Account".into(),
                "Owner".into(),
                "Region".into(),
                "Stage".into(),
                "Seats".into(),
                "ARR".into(),
            ],
            vec![
                vec![
                    "Changing account width".into(),
                    "A".into(),
                    "Europe".into(),
                    "Draft".into(),
                    "4".into(),
                    "$250".into(),
                ],
                vec![
                    "X".into(),
                    "A different long name".into(),
                    "N".into(),
                    "Ready for review".into(),
                    "105".into(),
                    "$96,600".into(),
                ],
            ],
            window,
            cx,
        )
    });
    fn edges(w: &mut VisualTestContext) {
        for (name, one, two) in [
            (
                "grid-number-heading",
                "grid-row-number-0",
                "grid-row-number-1",
            ),
            ("Account", "grid-cell-0-0", "grid-cell-1-0"),
            ("Owner", "grid-cell-0-1", "grid-cell-1-1"),
            ("Region", "grid-cell-0-2", "grid-cell-1-2"),
            ("Stage", "grid-cell-0-3", "grid-cell-1-3"),
            ("Seats", "grid-cell-0-4", "grid-cell-1-4"),
            ("ARR", "grid-cell-0-5", "grid-cell-1-5"),
        ] {
            let h = w.debug_bounds(name).unwrap();
            for selector in [one, two] {
                let c = w.debug_bounds(selector).unwrap();
                assert!(
                    (c.left() - h.left()).abs() <= px(1.)
                        && (c.right() - h.right()).abs() <= px(1.),
                    "{name} misaligned: header {h:?}, cell {c:?}"
                );
            }
        }
    }
    w.simulate_resize(size(px(628.), px(520.)));
    w.run_until_parked();
    edges(w);
    w.simulate_resize(size(px(320.), px(520.)));
    w.run_until_parked();
    edges(w);
    let before = w.debug_bounds("Account").unwrap();
    w.update(|win, cx| {
        win.dispatch_event(
            PlatformInput::Scroll(ScrollEvent {
                position: before.center(),
                delta: ScrollDelta::Pixels(point(px(-180.), px(0.))),
                ..Default::default()
            }),
            cx,
        );
    });
    w.run_until_parked();
    assert!(
        w.debug_bounds("Account").unwrap().left() < before.left() - px(1.),
        "the test must actually scroll horizontally"
    );
    edges(w);
}

fn advance_frame(w: &mut VisualTestContext, ms: u64) {
    w.executor()
        .advance_clock(std::time::Duration::from_millis(ms));
    w.run_until_parked();
    w.update(|win, cx| {
        win.simulate_next_frame(cx);
        win.refresh();
    });
    w.run_until_parked();
}

#[test]
fn tree_keeps_an_interrupted_branch_at_its_current_height() {
    let mut app = host();
    app.update(|cx| kui::foundation::Theme::global_mut(cx).reduced_motion = false);
    let (_, w) = app.add_window_view(|w, cx| NavDemo::new("tree-view", w, cx));
    w.run_until_parked();
    let distance = |w: &mut VisualTestContext| {
        w.debug_bounds("README.md").unwrap().top() - w.debug_bounds("components").unwrap().top()
    };
    let full = distance(w);
    click(w, "components");
    advance_frame(w, 60);
    let closing = distance(w);
    assert!(
        closing < full - px(1.) && closing > full - px(71.),
        "expected an intermediate two-row fold: {closing:?} versus {full:?}"
    );
    click(w, "components");
    assert!(
        (distance(w) - closing).abs() < px(1.),
        "a mid-flight reversal must retain the current geometry"
    );
    advance_frame(w, 800);
    assert!((distance(w) - full).abs() <= px(1.));
}

#[test]
fn long_code_block_reveals_rows_and_reverses_midway() {
    use kui::components::code_block::CodeBlock;
    let mut app = host();
    app.update(|cx| kui::foundation::Theme::global_mut(cx).reduced_motion = false);
    let (_, w) = app.add_window_view(|_, cx| {
        CodeBlock::new("sample.rs", "Rust", "one\ntwo\nthree\nfour\nfive\nsix", cx).max_lines(2)
    });
    w.run_until_parked();
    let top = w.debug_bounds("Show all").unwrap().top();
    click(w, "Show all");
    advance_frame(w, 75);
    let midway = w.debug_bounds("Show less").unwrap().top();
    assert!(
        midway > top + px(1.) && midway < top + px(79.),
        "unexpected mid reveal at {midway:?} from {top:?}"
    );
    click(w, "Show less");
    assert!((w.debug_bounds("Show all").unwrap().top() - midway).abs() < px(1.));
    advance_frame(w, 850);
    assert!((w.debug_bounds("Show all").unwrap().top() - top).abs() < px(1.));
}

#[test]
fn facet_chooser_keeps_the_toolbar_in_place_and_commits_the_selected_value() {
    let mut app = host();
    let (_, w) = app.add_window_view(|w, cx| NavDemo::new("filter-toolbar", w, cx));
    w.run_until_parked();
    let original = w.debug_bounds("Clear all").unwrap();
    click(w, "Add filter");
    click(w, "Priority");
    assert!((w.debug_bounds("Clear all").unwrap().top() - original.top()).abs() < px(1.));
    click(w, "High");
    assert!(w.debug_bounds("Priority · High").is_some());
    click(w, "Add filter");
    w.simulate_keystrokes("escape");
    w.run_until_parked();
    assert!(w.debug_bounds("High").is_none());
}

#[test]
fn tree_staggers_only_new_labels_and_cancels_a_delayed_sibling() {
    let mut app = host();
    app.update(|cx| kui::foundation::Theme::global_mut(cx).reduced_motion = false);
    let (_, w) = app.add_window_view(|win, cx| NavDemo::new("tree-view", win, cx));
    w.run_until_parked();
    let rest = w.debug_bounds("button.rs").unwrap().left();
    click(w, "components");
    advance_frame(w, 850);
    click(w, "components");
    advance_frame(w, 30);
    // The first few subpixel positions round to the same layout pixel.
    advance_frame(w, 30);
    let first = w.debug_bounds("button.rs").unwrap().left();
    let next = w.debug_bounds("style.rs").unwrap().left();
    assert!(
        first > next && next < rest - px(5.),
        "the second label should trail by the 35ms stagger: {first:?} {next:?} of {rest:?}"
    );
    click(w, "components");
    advance_frame(w, 500);
    assert!(w.debug_bounds("style.rs").is_none());
    click(w, "components");
    advance_frame(w, 80);
    advance_frame(w, 750);
    assert!((w.debug_bounds("style.rs").unwrap().left() - rest).abs() < px(1.));
}

#[test]
fn long_pagination_moves_numbers_on_one_belt_and_reverses_without_crossing() {
    use kui::components::pagination::Pagination;
    let mut app = host();
    app.update(|cx| kui::foundation::Theme::global_mut(cx).reduced_motion = false);
    let (pages, w) = app.add_window_view(|_, _| Pagination::new(24, 2));
    w.run_until_parked();
    let x = |w: &mut VisualTestContext, n: &'static str| w.debug_bounds(n).unwrap().left();
    let start = x(w, "4");
    pages.update(w, |p, cx| p.select(5, cx));
    advance_frame(w, 65);
    let mid = x(w, "4");
    assert!(mid < start && mid > start - px(80.));
    assert!((x(w, "5") - mid - px(40.)).abs() < px(1.));
    pages.update(w, |p, cx| p.select(3, cx));
    w.run_until_parked();
    assert!((x(w, "4") - mid).abs() < px(1.));
    advance_frame(w, 120);
    assert!((x(w, "5") - x(w, "4") - px(40.)).abs() < px(1.));
    advance_frame(w, 900);
    assert!((x(w, "4") - start).abs() < px(1.));
    assert!(w.debug_bounds("6").is_none());
    w.update(|_, cx| kui::foundation::Theme::global_mut(cx).reduced_motion = true);
    pages.update(w, |p, cx| p.select(24, cx));
    w.run_until_parked();
    assert_eq!(pages.read_with(w, |p, _| p.page()), 24);
    assert!((x(w, "24") - x(w, "20") - px(160.)).abs() < px(1.));
    assert!(w.debug_bounds("4").is_none());
}

#[test]
fn an_open_fold_preserves_the_whole_action_in_a_short_scroller() {
    use gpui::{Context, IntoElement, Render, Window, div, prelude::*};
    use kui::primitives::nav::{Fold, column};

    struct Example(Fold);
    impl Render for Example {
        fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let action = div()
                .debug_selector(|| "fold-action".into())
                .h(px(34.))
                .child("Open");
            let fold = self
                .0
                .render(action, w, cx)
                .debug_selector(|| "open-fold".into());
            column()
                .h(px(51.))
                .id("scroller")
                .overflow_y_scroll()
                .child(div().h(px(24.)).flex_none())
                .child(fold)
        }
    }
    let mut app = host();
    let (_, w) = app.add_window_view(|_, _| Example(Fold::new(true, 34.)));
    w.simulate_resize(size(px(288.), px(200.)));
    w.run_until_parked();
    let wrapper = w.debug_bounds("open-fold").unwrap();
    let action = w.debug_bounds("fold-action").unwrap();
    assert!(
        wrapper.bottom() >= action.bottom(),
        "action {action:?} crosses fold {wrapper:?}"
    );
}

#[test]
fn json_viewer_matches_scroll_and_copy_the_real_row() {
    use super::JsonViewer;
    let mut app = host();
    let (story, w) = app.add_window_view(|win, cx| NavDemo::new("json-viewer", win, cx));
    w.simulate_resize(size(px(628.), px(646.)));
    w.run_until_parked();
    let view = w.update(|_, cx| {
        story.read(cx).views[0]
            .clone()
            .downcast::<JsonViewer>()
            .unwrap()
    });
    let path =
        |w: &mut VisualTestContext| w.update(|_, cx| view.read(cx).selected_path().to_owned());
    let clip = |w: &mut VisualTestContext| {
        w.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap())
    };
    click(w, "Search JSON");
    w.simulate_input("payment");
    w.run_until_parked();
    assert_eq!(path(w), "order.events[3].type");
    w.simulate_keystrokes("enter");
    assert_eq!(path(w), "order.events[8].type");
    w.simulate_keystrokes("shift-enter");
    assert_eq!(path(w), "order.events[3].type");
    w.simulate_keystrokes("escape");
    assert_eq!(path(w), "order.events");
    w.simulate_input("REFUND");
    w.run_until_parked();
    assert_eq!(path(w), "order.events[131].note");
    let body = w.debug_bounds("JSON tree").unwrap();
    let note = w
        .debug_bounds("note")
        .expect("result painted without any wheel input");
    assert!(note.top() >= body.top() && note.bottom() <= body.bottom());
    w.simulate_keystrokes("down ctrl-c");
    assert_eq!(clip(w), "Refund requested for a damaged item");
    w.simulate_keystrokes("ctrl-shift-c");
    assert_eq!(clip(w), "order.events[131].note");
    w.simulate_keystrokes("left");
    assert_eq!(path(w), "order.events[131]");
    w.simulate_keystrokes("/ escape");
    click(w, "Collapse all");
    assert_eq!(path(w), "order");
    assert!(w.debug_bounds("note").is_none());
}

#[test]
fn json_viewer_array_pages_and_special_keys_keep_value_types() {
    use super::{JsonValue, JsonViewer};
    let mut app = host();
    let data: JsonValue = serde_json::json!({"a/b~c": {"with space": [true,null,42,"quoted"]}, "list":(0..130).collect::<Vec<_>>()});
    let (view, w) = app
        .add_window_view(|win, cx| JsonViewer::new("payload", data, win, cx).with_max_height(160.));
    w.simulate_resize(size(px(320.), px(420.)));
    w.run_until_parked();
    click(w, "a/b~c");
    w.simulate_keystrokes("right right right ctrl-c");
    let clip = |w: &mut VisualTestContext| {
        w.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap())
    };
    assert_eq!(clip(w), "true");
    w.update(|_, cx| assert_eq!(view.read(cx).selected_pointer(), "/a~1b~0c/with space/0"));
    w.simulate_keystrokes("ctrl-shift-c");
    assert_eq!(clip(w), r#"payload["a/b~c"]["with space"][0]"#);
    w.simulate_keystrokes("down ctrl-c");
    assert_eq!(clip(w), "null");
    w.simulate_keystrokes("down ctrl-c");
    assert_eq!(clip(w), "42");
    w.simulate_keystrokes("down ctrl-c");
    assert_eq!(clip(w), "quoted");
    click(w, "Collapse all");
    click(w, "list");
    w.simulate_keystrokes("end enter");
    assert_eq!(
        w.update(|_, cx| view.read(cx).selected_path().to_owned()),
        "payload.list[50]"
    );
    assert!(w.debug_bounds("50").is_some() && w.debug_bounds("0").is_none());
    w.simulate_keystrokes("end enter");
    assert_eq!(
        w.update(|_, cx| view.read(cx).selected_path().to_owned()),
        "payload.list[100]"
    );
    w.simulate_keystrokes("home");
    assert!(w.debug_bounds("payload").is_some());
}

#[test]
fn json_viewer_large_search_folds_to_a_painted_root() {
    use std::time::Duration;
    let mut app = TestAppContext::single();
    app.update(kui::foundation::init);
    let (_, w) = app.add_window_view(|win, cx| NavDemo::new("json-viewer", win, cx));
    w.simulate_resize(size(px(628.), px(646.)));
    w.run_until_parked();
    let settle = |w: &mut VisualTestContext| {
        for _ in 0..30 {
            w.executor().advance_clock(Duration::from_millis(20));
            w.update(|win, cx| win.simulate_next_frame(cx));
            w.run_until_parked()
        }
    };
    click(w, "Search JSON");
    w.simulate_input("evt_");
    w.simulate_keystrokes("down end");
    settle(w);
    assert!(w.debug_bounds("order").is_none());
    click(w, "Collapse all");
    w.update(|win, cx| win.simulate_next_frame(cx));
    w.run_until_parked();
    assert!(
        w.debug_bounds("order").is_some(),
        "collapsed view stays populated on its first frame"
    );
}

#[test]
fn json_branch_keeps_its_exit_space_while_text_fades() {
    let mut app = host();
    app.update(|cx| kui::foundation::Theme::global_mut(cx).reduced_motion = false);
    let (_, w) = app.add_window_view(|win, cx| NavDemo::new("json-viewer", win, cx));
    w.simulate_resize(size(px(628.), px(646.)));
    w.run_until_parked();
    click(w, "order");
    let advance = |w: &mut VisualTestContext, ms| {
        w.executor()
            .advance_clock(std::time::Duration::from_millis(ms));
        w.update(|win, cx| win.simulate_next_frame(cx));
        w.run_until_parked();
    };
    advance(w, 220);
    let midway = w.debug_bounds("JSON tree").unwrap().size.height;
    assert!(
        midway > px(85.) && midway < px(200.),
        "expected residual row space at 220ms, got {midway:?}"
    );
    advance(w, 900);
    assert!(w.debug_bounds("JSON tree").unwrap().size.height < px(50.));
}
