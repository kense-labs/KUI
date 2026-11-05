//! Source-verified rating and descriptive radio-card examples.
use gpui::{prelude::*, *};
use kui::{
    components::{
        avatar::{Avatar, Person},
        button::Button,
        radio_cards::{RadioCardLayout, RadioCardOption, RadioCards},
        rating::{Rating, RatingChanged},
        textarea::Textarea,
    },
    foundation::Theme,
    primitives::{
        Size,
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion, ease, standard_ease},
        field::FieldEvent,
    },
};
use std::time::Duration;

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    match id {
        "rating" => Some(cx.new(|cx| RatingDemo::new(w, cx)).into()),
        "radio-cards" => Some(cx.new(RadioDemo::new).into()),
        _ => None,
    }
}
struct RadioDemo {
    plan: Entity<RadioCards>,
    delivery: Entity<RadioCards>,
    language: UiLanguage,
}
impl RadioDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let plan = vec![
            RadioCardOption::new("one", t("One bag", "一袋"))
                .with_detail(t("12 oz, whole bean", "12 盎司，整豆"))
                .with_price("$18"),
            RadioCardOption::new("two", t("Two bags", "两袋"))
                .with_detail(t("Two origins each month", "每月两种产地"))
                .with_price("$34"),
            RadioCardOption::new("bulk", t("Bulk", "大包装"))
                .with_detail(t("5 lb for the office", "办公室用 5 磅装"))
                .with_price("$72"),
            RadioCardOption::new("decaf", t("Decaf", "低因"))
                .with_price("$19")
                .with_disabled_reason(t("Back in stock October 14", "10 月 14 日补货")),
        ];
        let delivery = vec![
            RadioCardOption::new("standard", t("Standard", "标准配送"))
                .with_detail(t("5 to 7 business days", "5 至 7 个工作日"))
                .with_price(t("Free", "免费")),
            RadioCardOption::new("express", t("Express", "快递"))
                .with_detail(t("2 business days", "2 个工作日"))
                .with_price("$12"),
            RadioCardOption::new("overnight", t("Overnight", "次日送达"))
                .with_detail(t("Tomorrow by 10 am", "明天上午 10 点前"))
                .with_price("$29"),
            RadioCardOption::new("pickup", t("Pickup", "自取"))
                .with_price(t("Free", "免费"))
                .with_disabled_reason(t("Not available for your address", "你的地址暂不支持")),
        ];
        Self {
            plan: cx.new(|cx| {
                RadioCards::new(t("Plan", "方案"), plan, cx)
                    .with_min_column_width(200.)
                    .with_selected("two")
            }),
            delivery: cx.new(|cx| {
                RadioCards::new(t("Delivery", "配送"), delivery, cx)
                    .with_layout(RadioCardLayout::List)
                    .with_selected("standard")
            }),
            language,
        }
    }
}
impl Render for RadioDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &Theme::global(cx).tokens.colors;
        let group = |label, view: AnyView| {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(div().font_weight(FontWeight::MEDIUM).child(label))
                .child(view)
        };
        div()
            .w_full()
            .max_w(px(680.))
            .p(px(20.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .text_size(px(14.))
            .line_height(relative(1.4))
            .text_color(colors.foreground)
            .child(group(
                self.language.text("Plan", "方案"),
                self.plan.clone().into(),
            ))
            .child(group(
                self.language.text("Delivery", "配送"),
                self.delivery.clone().into(),
            ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoteMode {
    Auto,
    Note,
    Skipped,
    Sent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Follow {
    Thanks,
    Note,
    Sent,
}
struct RatingDemo {
    rating: Entity<Rating>,
    avatar: Entity<Avatar>,
    language: UiLanguage,
    band: Option<bool>,
    mode: NoteMode,
    note: String,
    pending: Option<Task<()>>,
    field: Entity<Textarea>,
    field_mode: NoteMode,
    note_sub: Option<Subscription>,
    _rating_sub: Subscription,
    shown: Option<Follow>,
    outgoing: Option<Follow>,
    clock: PhaseClock,
    height: Slide,
    entry_y: Slide,
    entry_started: bool,
    measured: f32,
    focus_note: bool,
    focus_edit: bool,
    edit_focus: FocusHandle,
}
impl RatingDemo {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let rating = cx.new(|cx| {
            Rating::new(
                language.text("How was the handoff?", "这次交接体验如何？"),
                5,
                0.,
                cx,
            )
        });
        let field = cx.new(|cx| {
            Textarea::new(
                "rating-note",
                language.text("What would have made it smoother?", "哪些地方可以更顺畅？"),
                w,
                cx,
            )
            .with_placeholder(
                language.text(
                    "Missing states, unclear specs, timing",
                    "缺失状态、规格不清或时间安排",
                ),
                cx,
            )
        });
        let note_sub = Some(cx.subscribe(&field, |s, _, e: &FieldEvent, cx| {
            if let FieldEvent::Changed(text) = e {
                s.note = text.to_string();
                cx.notify();
            }
        }));
        let rating_sub = cx.subscribe_in(&rating, w, |s, _, e: &RatingChanged, w, cx| {
            s.pending = None;
            if e.0 == 0. {
                s.band = None;
                s.mode = NoteMode::Auto;
                s.note.clear();
                s.field.update(cx, |f, cx| f.set_value("", w, cx));
                cx.notify();
                return;
            }
            let low = e.0 <= 3.;
            s.pending = Some(cx.spawn(async move |weak, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(240))
                    .await;
                let _ = weak.update(cx, |s, cx| {
                    s.pending = None;
                    s.band = Some(low);
                    cx.notify();
                });
            }));
        });
        Self {
            rating,
            avatar: cx.new(|_| Avatar::new(Person::new("Emma Collins"), 28.)),
            language,
            band: None,
            mode: NoteMode::Auto,
            note: String::new(),
            pending: None,
            field,
            field_mode: NoteMode::Auto,
            note_sub,
            _rating_sub: rating_sub,
            shown: None,
            outgoing: None,
            clock: Default::default(),
            height: Slide::new(0.),
            entry_y: Slide::new(-8.),
            entry_started: false,
            measured: 0.,
            focus_note: false,
            focus_edit: false,
            edit_focus: cx.focus_handle(),
        }
    }
    fn follow(&self) -> Option<Follow> {
        self.band.map(|low| match self.mode {
            NoteMode::Sent => Follow::Sent,
            NoteMode::Note => Follow::Note,
            NoteMode::Skipped => Follow::Thanks,
            NoteMode::Auto => {
                if low || !self.note.trim().is_empty() {
                    Follow::Note
                } else {
                    Follow::Thanks
                }
            }
        })
    }
    fn open_note(&mut self, cx: &mut Context<Self>) {
        self.mode = NoteMode::Note;
        self.focus_note = true;
        cx.notify();
    }
    fn send(&mut self, cx: &mut Context<Self>) {
        if self.note.trim().is_empty() {
            return;
        }
        self.mode = NoteMode::Sent;
        self.focus_edit = true;
        cx.notify();
    }
}
impl Render for RatingDemo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Theme::global(cx).tokens.colors.clone();
        let radius = kui::foundation::surface_radius(cx);
        let reduced = disabled_motion(cx);
        let t = |en, zh| self.language.text(en, zh);
        let kind = self.follow();
        if kind != self.shown {
            self.outgoing = self.shown;
            self.shown = kind;
            self.clock.restart(cx);
            self.measured = 0.;
            self.entry_y.reset(-8.);
            self.entry_started = false;
        }
        if kind == Some(Follow::Note) && self.field_mode != self.mode {
            let label = if self.mode == NoteMode::Auto {
                t("What would have made it smoother?", "哪些地方可以更顺畅？")
            } else {
                t("Anything Emma should know?", "还有什么想告诉 Emma？")
            };
            let value = self.note.clone();
            self.field = cx.new(|cx| {
                Textarea::new("rating-note", label, w, cx)
                    .with_value(value, w, cx)
                    .with_placeholder(
                        t(
                            "Missing states, unclear specs, timing",
                            "缺失状态、规格不清或时间安排",
                        ),
                        cx,
                    )
            });
            self.note_sub = Some(cx.subscribe(&self.field, |s, _, e: &FieldEvent, cx| {
                if let FieldEvent::Changed(text) = e {
                    s.note = text.to_string();
                    cx.notify();
                }
            }));
            self.field_mode = self.mode;
        }
        if kind == Some(Follow::Note) && self.focus_note {
            self.field.focus_handle(cx).focus(w, cx);
            self.focus_note = false;
        }
        if kind == Some(Follow::Sent) && self.focus_edit {
            self.edit_focus.focus(w, cx);
            self.focus_edit = false;
        }
        let shell = |inner: Div| {
            div()
                .w_full()
                .relative()
                .pt(px(31.))
                .px(px(4.))
                .pb(px(4.))
                .child(
                    div()
                        .absolute()
                        .left(px(4.))
                        .right(px(4.))
                        .top(px(16.))
                        .h(px(1.))
                        .bg(colors.border),
                )
                .child(inner)
        };
        let field = self.field.clone();
        let note = self.note.clone();
        let edit_focus = self.edit_focus.clone();
        let body = |kind: Option<Follow>| -> Div {
            match kind {
                Some(Follow::Thanks) => div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_x(px(12.))
                    .gap_y(px(8.))
                    .child(t(
                        "Thanks, Emma will see your rating",
                        "谢谢，Emma 会看到你的评分",
                    ))
                    .child(
                        Button::new("rating-add-note")
                            .ghost()
                            .with_size(Size::Sm)
                            .label(t("Add a note", "补充说明"))
                            .on_click(cx.listener(|s, _, _, cx| s.open_note(cx))),
                    ),
                Some(Follow::Note) => div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(field.clone())
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                Button::new("rating-skip-note")
                                    .ghost()
                                    .with_size(Size::Sm)
                                    .label(t("Not now", "暂不填写"))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.mode = NoteMode::Skipped;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("rating-send-note")
                                    .with_size(Size::Sm)
                                    .label(t("Send note", "发送说明"))
                                    .disabled(note.trim().is_empty())
                                    .on_click(cx.listener(|s, _, _, cx| s.send(cx))),
                            ),
                    ),
                Some(Follow::Sent) => div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_x(px(12.))
                    .gap_y(px(8.))
                    .child(
                        div()
                            .id("rating-note-sent")
                            .role(Role::Status)
                            .font_weight(FontWeight::MEDIUM)
                            .child(t("Note sent to Emma", "说明已发送给 Emma")),
                    )
                    .child(
                        div()
                            .id("rating-edit-focus")
                            .track_focus(&edit_focus)
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    s.open_note(cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            }))
                            .child(
                                Button::new("rating-edit-note")
                                    .ghost()
                                    .with_size(Size::Sm)
                                    .label(t("Edit note", "编辑说明"))
                                    .on_click(cx.listener(|s, _, _, cx| s.open_note(cx))),
                            ),
                    ),
                None => div(),
            }
        };
        let content = shell(body(kind));
        let weak = cx.weak_entity();
        let expected = self.measured;
        let target = if kind.is_some() { self.measured } else { 0. };
        self.height.duration(target, 0.4, cx);
        let height = self.height.get(w, cx);
        let age = self.clock.time(0.52, w, cx);
        if age >= 0.06 && !self.entry_started {
            self.entry_y.duration(0., 0.4, cx);
            self.entry_started = true;
        }
        let entry_y = if reduced { 0. } else { self.entry_y.get(w, cx) };
        let enter = if reduced {
            standard_ease((age / 0.12).clamp(0., 1.))
        } else {
            ease(((age - 0.06) / 0.24).clamp(0., 1.))
        };
        // The outgoing content fades on its own shorter curve so the two bodies
        // cross-fade instead of blending on one symmetric ramp. It drifts the
        // same way as the entering body: entry_y descends from -8 to 0, so the
        // leaving body continues downward (0 to +6) and the swap reads as one
        // conveyor. Sending it the other way would drive the two bodies toward
        // each other and read as a collapse rather than a swap.
        let exit = standard_ease((age / 0.12).clamp(0., 1.));
        let exit_y = if reduced { 0. } else { exit * 6. };
        if enter >= 1. {
            self.outgoing = None;
        }
        let exit_content = self.outgoing.map(|out| shell(body(Some(out))));
        let follow = div()
            .relative()
            .h(px(height))
            .mx(px(-4.))
            .overflow_hidden()
            .when(kind.is_some(), |d| {
                d.child(
                    div()
                        .on_children_prepainted(move |bounds, _, cx| {
                            if let Some(b) = bounds.first() {
                                let h = f32::from(b.size.height);
                                if (expected - h).abs() > 0.5 {
                                    let _ = weak.update(cx, |s, cx| {
                                        s.measured = h;
                                        cx.notify();
                                    });
                                }
                            }
                        })
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(entry_y))
                        .opacity(enter)
                        .child(content),
                )
            })
            .when_some(exit_content, |d, exit_content| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(exit_y))
                        .opacity(1. - exit)
                        .child(exit_content),
                )
            });
        div().w_full().max_w(px(420.)).p(px(20.)).child(
            div()
                .w_full()
                .min_h(px(376.))
                .flex()
                .flex_col()
                .items_start()
                .child(
                    div()
                        .w_full()
                        .border_1()
                        .border_color(colors.border)
                        .rounded(radius)
                        .bg(colors.surface)
                        .pt(px(18.))
                        .px(px(20.))
                        .pb(px(20.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .mb(px(14.))
                                .text_size(px(12.))
                                .line_height(relative(1.4))
                                .text_color(colors.text_muted)
                                .child(self.avatar.clone())
                                .child(
                                    div().flex_1().min_w_0().child(
                                        StyledText::new(t(
                                            "Emma Collins handed off the checkout specs",
                                            "Emma Collins 已交接结账页面规格",
                                        ))
                                        .with_highlights(
                                            [(
                                                0..12,
                                                HighlightStyle {
                                                    color: Some(colors.foreground),
                                                    font_weight: Some(FontWeight::MEDIUM),
                                                    ..Default::default()
                                                },
                                            )],
                                        ),
                                    ),
                                ),
                        )
                        .child(self.rating.clone())
                        .child(follow),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Follow, NoteMode, RatingDemo};
    use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
    use std::time::Duration;

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }
    #[test]
    fn rating_follow_up_cancels_stale_delays_and_clearing_resets_the_draft() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (demo, w) = app.add_window_view(RatingDemo::new);
        w.simulate_resize(size(px(500.), px(700.)));
        w.run_until_parked();
        let low = w
            .debug_bounds("How was the handoff?: 2 of 5")
            .unwrap()
            .center();
        w.simulate_click(low, Modifiers::default());
        tick(w, 239);
        assert_eq!(w.update(|_, cx| demo.read(cx).follow()), None);
        let high = w
            .debug_bounds("How was the handoff?: 5 of 5")
            .unwrap()
            .center();
        w.simulate_click(high, Modifiers::default());
        tick(w, 1);
        assert_eq!(w.update(|_, cx| demo.read(cx).follow()), None);
        tick(w, 239);
        assert_eq!(
            w.update(|_, cx| demo.read(cx).follow()),
            Some(Follow::Thanks)
        );
        w.simulate_keystrokes("backspace");
        tick(w, 600);
        w.update(|_, cx| {
            let s = demo.read(cx);
            assert_eq!(s.rating.read(cx).value(), 0.);
            assert_eq!(s.follow(), None);
            assert!(s.note.is_empty());
            assert!(s.pending.is_none());
        });
    }
    #[test]
    fn note_skip_send_edit_preserves_the_draft_and_focus() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (demo, w) = app.add_window_view(RatingDemo::new);
        w.simulate_resize(size(px(500.), px(750.)));
        w.run_until_parked();
        let low = w
            .debug_bounds("How was the handoff?: 2 of 5")
            .unwrap()
            .center();
        w.simulate_click(low, Modifiers::default());
        tick(w, 240);
        tick(w, 600);
        assert_eq!(w.update(|_, cx| demo.read(cx).follow()), Some(Follow::Note));
        w.update(|_, cx| demo.update(cx, |s, cx| s.send(cx)));
        assert_eq!(w.update(|_, cx| demo.read(cx).mode), NoteMode::Auto);
        w.update(|w, cx| {
            let field = demo.read(cx).field.clone();
            field.update(cx, |f, cx| f.set_value("Missing keyboard states", w, cx));
        });
        w.run_until_parked();
        let skip = w.debug_bounds("Not now").unwrap().center();
        w.simulate_click(skip, Modifiers::default());
        tick(w, 600);
        assert_eq!(
            w.update(|_, cx| demo.read(cx).follow()),
            Some(Follow::Thanks)
        );
        let add = w.debug_bounds("Add a note").unwrap().center();
        w.simulate_click(add, Modifiers::default());
        tick(w, 600);
        w.update(|_, cx| {
            let s = demo.read(cx);
            assert_eq!(s.note, "Missing keyboard states");
            assert_eq!(s.field.read(cx).value(cx), "Missing keyboard states");
        });
        let send = w.debug_bounds("Send note").unwrap().center();
        w.simulate_click(send, Modifiers::default());
        tick(w, 600);
        w.update(|w, cx| {
            let s = demo.read(cx);
            assert_eq!(s.follow(), Some(Follow::Sent));
            assert!(s.edit_focus.is_focused(w));
        });
        w.simulate_keystrokes("enter");
        tick(w, 600);
        assert_eq!(w.update(|_, cx| demo.read(cx).follow()), Some(Follow::Note));
    }
}
