//! Verify KUI Pro spring motion primitives preserve hit-box stability, exit presence, and reduced-motion collapse.

use gpui::{
    Context, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext,
    Window, div, prelude::*, px, rgb, size,
};
use kui::{
    components::pro_chrome::{
        ProBtnStyle, pro_btn, pro_fold, pro_mark_selected, pro_pill_indicator, pro_presence,
        pro_selection_sensor, pro_spring_bar, pro_switch, pro_view_transition,
    },
    foundation::Theme,
};

struct MotionHarness {
    selected: usize,
    fold_open: bool,
    popover_open: bool,
    switch_on: bool,
    clicks: usize,
}

impl Render for MotionHarness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sensor = pro_selection_sensor();
        let s0 = sensor.clone();
        let s1 = sensor.clone();
        let sel = self.selected;
        let strip = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(pro_pill_indicator(
                "pill-ind",
                format!("tab-{sel}"),
                sensor,
                6.,
                rgb(0xe5e5e5).into(),
                None,
                false,
            ))
            .child(
                div()
                    .debug_selector(|| "tab-0".into())
                    .relative()
                    .w(px(72.))
                    .h(px(28.))
                    .when(sel == 0, |d| d.child(pro_mark_selected(&s0)))
                    .child("Overview"),
            )
            .child(
                div()
                    .debug_selector(|| "tab-1".into())
                    .relative()
                    .w(px(96.))
                    .h(px(28.))
                    .when(sel == 1, |d| d.child(pro_mark_selected(&s1)))
                    .child("Activity"),
            );

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().debug_selector(|| "btn-wrap".into()).child(pro_btn(
                        "save-btn",
                        if self.clicks == 0 { "Save" } else { "Saved" },
                        None,
                        ProBtnStyle::Primary,
                        false,
                        cx,
                        |this, _, cx| {
                            this.clicks += 1;
                            cx.notify();
                        },
                    )))
                    .child(
                        div()
                            .debug_selector(|| "sibling-anchor".into())
                            .w(px(40.))
                            .h(px(30.)),
                    )
                    .child(pro_switch(
                        "demo-sw",
                        "Toggle feature",
                        self.switch_on,
                        cx,
                        |this, _, cx| {
                            this.switch_on = !this.switch_on;
                            cx.notify();
                        },
                    )),
            )
            .child(strip)
            .child(pro_spring_bar(
                "quota-bar",
                if self.switch_on { 0.8 } else { 0.25 },
                6.,
                999.,
                rgb(0xe5e5e5).into(),
                rgb(0x111111).into(),
            ))
            .child(div().debug_selector(|| "fold-wrap".into()).child(pro_fold(
                "details-fold",
                self.fold_open,
                48.,
                0.,
                || {
                    div()
                        .debug_selector(|| "fold-body".into())
                        .w_full()
                        .h(px(48.))
                        .child("Expanded section")
                        .into_any_element()
                },
            )))
            .child(pro_presence(
                "pop-pres",
                self.popover_open,
                0.18,
                0.14,
                |amount| {
                    div()
                        .debug_selector(|| "pop-body".into())
                        .w(px(120.))
                        .h(px(36.))
                        .opacity(amount)
                        .child("Floating menu")
                        .into_any_element()
                },
            ))
            .child(pro_view_transition(
                "body-vtx",
                format!("view-{sel}"),
                div().h(px(24.)).child(format!("Pane {sel}")),
            ))
    }
}

#[test]
fn pro_motion_controls_preserve_hitbox_and_honor_exit_and_reduced_motion() {
    let mut app = TestAppContext::single();
    app.update(kui::init);
    let (entity, w) = app.add_window_view(|_, _| MotionHarness {
        selected: 0,
        fold_open: true,
        popover_open: true,
        switch_on: false,
        clicks: 0,
    });

    w.simulate_resize(size(px(640.), px(480.)));
    w.run_until_parked();
    w.update(|win, cx| win.simulate_next_frame(cx));
    w.run_until_parked();

    // 1. Tactile button press must never shift outer hit-box or sibling bounds.
    let btn_rest = w.debug_bounds("btn-wrap").expect("btn-wrap mounted");
    let sib_rest = w
        .debug_bounds("sibling-anchor")
        .expect("sibling-anchor mounted");
    let center = btn_rest.center();
    w.simulate_event(MouseMoveEvent {
        position: center,
        pressed_button: None,
        modifiers: Modifiers::default(),
    });
    w.simulate_event(MouseDownEvent {
        position: center,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    w.update(|win, cx| win.simulate_next_frame(cx));
    w.run_until_parked();
    let btn_pressed = w
        .debug_bounds("btn-wrap")
        .expect("btn-wrap mounted while pressed");
    let sib_pressed = w
        .debug_bounds("sibling-anchor")
        .expect("sibling-anchor mounted while pressed");
    assert_eq!(btn_rest, btn_pressed);
    assert_eq!(sib_rest, sib_pressed);

    w.simulate_event(MouseUpEvent {
        position: center,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    w.run_until_parked();
    assert_eq!(w.update(|_, cx| entity.read(cx).clicks), 1);

    // 2. With normal motion enabled, closing `pro_fold` and `pro_presence` keeps exit frames mounted.
    assert!(w.debug_bounds("fold-body").is_some());
    assert!(w.debug_bounds("pop-body").is_some());
    w.update(|_, cx| {
        entity.update(cx, |m, cx| {
            m.selected = 1;
            m.fold_open = false;
            m.popover_open = false;
            m.switch_on = true;
            cx.notify();
        });
    });
    w.run_until_parked();
    assert!(
        w.debug_bounds("pop-body").is_some(),
        "pro_presence should remain mounted during exit animation"
    );
    assert!(
        w.debug_bounds("fold-wrap").expect("fold-wrap").size.height > px(0.),
        "pro_fold should animate height down smoothly rather than snapping to 0 on frame 1"
    );

    // 3. Under `Theme.reduced_motion`, disclosures and presence collapse immediately.
    w.update(|_, cx| {
        cx.update_global::<Theme, _>(|t, _| t.reduced_motion = true);
        entity.update(cx, |m, cx| {
            m.fold_open = true;
            m.popover_open = true;
            cx.notify();
        });
    });
    w.run_until_parked();
    assert!(w.debug_bounds("pop-body").is_some());
    assert_eq!(
        w.debug_bounds("fold-wrap").expect("fold-wrap").size.height,
        px(48.)
    );

    w.update(|_, cx| {
        entity.update(cx, |m, cx| {
            m.fold_open = false;
            m.popover_open = false;
            cx.notify();
        });
    });
    w.run_until_parked();
    assert!(
        w.debug_bounds("pop-body").is_none(),
        "reduced_motion should unmount closed presence immediately"
    );
    assert_eq!(
        w.debug_bounds("fold-wrap").expect("fold-wrap").size.height,
        px(0.),
        "reduced_motion should collapse fold height immediately"
    );
}
