//! The four-office time dial and its overlap search.
use gpui::{prelude::*, *};
use kui::{
    components::time_dial::{TimeDial, TimeDialCity, TimeDialHomeChanged, TimeDialValueChanged},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
    },
};

pub const IDS: &[&str] = &["time-dial"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

const INITIAL: i32 = 1260;
const OFFICES: [i32; 4] = [-240, -420, 60, 540];

struct Demo(Entity<TimeDial>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let lang = UiLanguage::current(cx);
        let cities = [
            ("nyc", "New York", "纽约", "NYC"),
            ("sf", "San Francisco", "旧金山", "SF"),
            ("lon", "London", "伦敦", "LON"),
            ("tyo", "Tokyo", "东京", "TYO"),
        ]
        .into_iter()
        .zip(OFFICES)
        .map(|((id, en, zh, code), offset)| TimeDialCity::new(id, lang.text(en, zh), code, offset))
        .collect();
        let control = cx.new(|cx| {
            TimeDial::new(cities, cx)
                .with_default_value(INITIAL)
                .with_label(lang.text("Call time", "会议时间"))
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
        });
        cx.subscribe(&control, |_, _, _: &TimeDialValueChanged, cx| cx.notify())
            .detach();
        cx.subscribe(&control, |_, _, _: &TimeDialHomeChanged, cx| cx.notify())
            .detach();
        Self(control)
    }
}

fn working_count(utc: i64) -> usize {
    OFFICES
        .iter()
        .filter(|&&offset| (540..1080).contains(&(utc + i64::from(offset)).rem_euclid(1440)))
        .count()
}

fn nearest_overlap(utc: i32, home_offset: i32) -> i32 {
    let utc = i64::from(utc);
    let offset = i64::from(home_offset);
    let start = (utc + offset).div_euclid(1440) * 1440 - offset;
    let best = (0..96)
        .map(|tick| start + tick * 15)
        .min_by_key(|&minute| (usize::MAX - working_count(minute), (minute - utc).abs()))
        .unwrap_or(utc);
    best.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let dial = self.0.read(cx);
        let utc = dial.value();
        let home = dial.home();
        let best = nearest_overlap(utc, home.map_or(0, |city| city.offset_minutes));
        let reset = utc != INITIAL || home.is_some_and(|city| city.id.as_ref() != "nyc");
        let count = working_count(i64::from(utc));
        let overlap = lang.text("Find overlap", "寻找重叠时段");
        let reset_label = lang.text("Reset", "重置");
        let summary = match lang {
            UiLanguage::English => format!("{count} of {} offices at work", OFFICES.len()),
            UiLanguage::Chinese => format!("{} 个办公室中有 {count} 个在工作时段", OFFICES.len()),
        };
        let radius = f32::from(kui::foundation::surface_radius(cx));
        let action = |id: &'static str, name: &'static str, glyph, enabled: bool| {
            face(id, name, 36., radius, Tone::Clear, cx)
                .border_1()
                .border_color(p.line)
                .px(px(12.))
                .gap(px(7.))
                .when(!enabled, |d| {
                    d.opacity(0.42).tab_index(-1).cursor(CursorStyle::Arrow)
                })
                .when(enabled, |d| d.hover(|d| d.bg(p.hover)))
                .child(icon(glyph, 15., p.ink))
                .child(name)
        };
        let max_width = if w.viewport_size().width >= px(720.) {
            768.
        } else {
            570.
        };
        div()
            .w_full()
            .max_w(px(max_width))
            .flex()
            .flex_col()
            .px(px(24.))
            .py(px(42.))
            .gap(px(28.))
            .child(self.0.clone())
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_x(px(14.))
                    .gap_y(px(10.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.secondary)
                            .child(summary),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .child(
                                action("time-dial-overlap", overlap, Glyph::Clock, best != utc)
                                    .on_click(cx.listener(move |s, _, _, cx| {
                                        if best != utc {
                                            s.0.update(cx, |dial, cx| dial.set_value(best, cx));
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(
                                action("time-dial-reset", reset_label, Glyph::Undo, reset)
                                    .on_click(cx.listener(move |s, _, _, cx| {
                                        if reset {
                                            s.0.update(cx, |dial, cx| {
                                                dial.set_value(INITIAL, cx);
                                                dial.set_home("nyc", cx);
                                            });
                                            cx.notify();
                                        }
                                    })),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{INITIAL, nearest_overlap, working_count};

    #[test]
    fn finds_the_nearest_time_with_the_most_working_offices() {
        assert_eq!(nearest_overlap(INITIAL, -240), 1005);
        assert_eq!(nearest_overlap(1335, 60), 1005);
        assert_eq!(working_count(1005), 3);
        assert_eq!(nearest_overlap(1005, 60), 1005);
    }
}
