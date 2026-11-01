//! Exercise the installed inspector at both sides of its container breakpoint.
use gpui::{Context, Entity, Modifiers, TestAppContext, Window, div, prelude::*, px, size};
use kui::components::tool_call::{Invocation, InvocationInspector, ToolAction, ToolState};
use std::{cell::RefCell, rc::Rc};

struct Host(Entity<InvocationInspector>);
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w_full().child(self.0.clone())
    }
}

#[test]
fn tool_columns_and_approval_controls_follow_the_measured_panel() {
    let mut app = TestAppContext::single();
    app.update(kui::init);
    let (host, w) = app.add_window_view(|_, cx| {
        Host(cx.new(|_| {
            let call = |name: &str, action: &str, state| Invocation {
                name: name.into(),
                action: action.into(),
                state,
                input: "Input data".into(),
                output: "Result".into(),
            };
            InvocationInspector::new(
                "tools",
                vec![
                    call("GitHub", "Search release blockers", ToolState::Done),
                    call("Linear", "Create a release checklist", ToolState::Approval),
                ],
            )
        }))
    });
    let control = w.update(|_, cx| host.read(cx).0.clone());
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = actions.clone();
    let _sub = w.update(|_, cx| {
        cx.subscribe(&control, move |_, ev: &ToolAction, _| {
            observed.borrow_mut().push(match ev {
                ToolAction::Approve(i) => ("allow", *i),
                ToolAction::Deny(i) => ("deny", *i),
                ToolAction::Retry(i) => ("retry", *i),
            });
        })
    });
    for (width, compact) in [
        (622., false),
        (320., true),
        (440., false),
        (439., true),
        (622., false),
    ] {
        w.simulate_resize(size(px(width), px(600.)));
        w.run_until_parked();
        w.update(|win, cx| win.simulate_next_frame(cx));
        w.run_until_parked();
        let gh = w.debug_bounds("tool-name-0").unwrap();
        let linear = w.debug_bounds("tool-name-1").unwrap();
        let search = w.debug_bounds("tool-title-0").unwrap();
        let create = w.debug_bounds("tool-title-1").unwrap();
        let summary = w.debug_bounds("tool-summary-1").unwrap();
        let deny = w.debug_bounds("Deny").unwrap();
        let allow = w.debug_bounds("Allow").unwrap();
        assert_eq!(gh.left(), linear.left());
        assert_eq!(gh.size.width, linear.size.width);
        assert_eq!(search.left(), create.left());
        assert_eq!(create.left() - linear.right(), px(10.));
        assert!((linear.center().y - summary.center().y).abs() <= px(1.));
        assert_eq!(deny.top(), allow.top());
        assert_eq!(allow.left() - deny.right(), px(6.));
        assert!(allow.right() <= px(width) && allow.size.height >= px(32.));
        if compact {
            assert_eq!(deny.left(), create.left());
            assert!(deny.top() >= create.bottom());
        } else {
            assert!(deny.left() >= create.right());
            assert!((deny.center().y - summary.center().y).abs() <= px(1.));
        }
    }
    let at = w.debug_bounds("Allow").unwrap().center();
    w.simulate_click(at, Modifiers::default());
    w.run_until_parked();
    assert!(w.debug_bounds("Allow").is_none());
    w.simulate_resize(size(px(320.), px(550.)));
    for (state, button) in [(ToolState::Approval, "Deny"), (ToolState::Failed, "Retry")] {
        w.update(|_, cx| {
            control.update(cx, |s, cx| {
                s.calls[1].state = state;
                cx.notify();
            })
        });
        w.run_until_parked();
        let b = w.debug_bounds(button).unwrap();
        let title = w.debug_bounds("tool-title-1").unwrap();
        assert_eq!(b.left(), title.left());
        assert!(b.top() >= title.bottom());
        w.simulate_click(b.center(), Modifiers::default());
        w.run_until_parked();
        assert!(w.debug_bounds(button).is_none());
    }
    assert_eq!(*actions.borrow(), [("allow", 1), ("deny", 1), ("retry", 1)]);
}
