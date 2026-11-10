//! A local booking demonstration with fixed seating availability.
use chrono::{Datelike, NaiveDate, Timelike, Weekday};
use gpui::{prelude::*, *};
use kui::{components::booking_pill::BookingPill, primitives::control::UiLanguage};

pub const IDS: &[&str] = &["booking-pill"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Demo(Entity<BookingPill>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let lang = UiLanguage::current(cx);
        let start = NaiveDate::from_ymd_opt(2026, 9, 21).expect("valid sample date");
        Self(cx.new(|cx| {
            BookingPill::new("Olea", start, cx)
                .with_venue_detail(lang.text(
                    "432 Octavia Street, San Francisco",
                    "旧金山奥克塔维亚街 432 号",
                ))
                .with_availability(|date, time, party| {
                    let minute = time.num_seconds_from_midnight() / 60;
                    date.weekday() != Weekday::Mon
                        && !(1200..=1260).contains(&minute)
                        && (party <= 6 || minute <= 1200)
                })
        }))
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .max_w(px(444.))
            .h(px(456.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .justify_end()
            .items_center()
            .child(self.0.clone())
    }
}
