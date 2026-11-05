//! International phone entry with searchable country/calling-code pairs.
use crate::primitives::{
    editor::{EditEvent, Editor, EditorScrollRegion, InputMode},
    family::{Paint, Slide, Tone, face},
    form_chrome::*,
    glyph::{Glyph, icon},
    phone_flags,
};
use gpui::{prelude::*, *};
const ROW_HEIGHT: f32 = 36.;
const ROW_GAP: f32 = 2.;
const LIST_HEIGHT: f32 = 280.;

fn flag(iso: &str) -> Div {
    div()
        .w(px(20.))
        .h(px(20.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .when_some(phone_flags::image(iso), |view, image| {
            view.child(img(image).w(px(20.)).h(px(15.)).rounded(px(1.5)))
        })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneValue {
    pub country: &'static str,
    pub national: String,
    pub international: String,
    pub possible: bool,
}
pub struct PhoneInput {
    label: SharedString,
    country: usize,
    edit: Entity<Editor>,
    query: Entity<Editor>,
    open: bool,
    reveal: FormReveal,
    indicator: Slide,
    dial: RollingValue,
    direction: f32,
    active: usize,
    scroll: ScrollHandle,
    reveal_active: bool,
    clearing_search: bool,
    focus: FocusHandle,
    _subs: Vec<Subscription>,
}
impl EventEmitter<PhoneValue> for PhoneInput {}
impl PhoneInput {
    pub fn new(
        label: impl Into<SharedString>,
        iso: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let country = COUNTRIES.iter().position(|c| c.0 == iso).unwrap_or(0);
        let edit = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Single, w, cx);
            e.set_placeholder(COUNTRIES[country].4);
            e
        });
        let query = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Find country", "搜索国家或地区"),
                InputMode::Combobox,
                w,
                cx,
            );
            e.set_placeholder(tr(cx, "Country or calling code…", "国家、地区或区号…"));
            e
        });
        let edit_sub = cx.subscribe_in(&edit, w, |s: &mut Self, e, ev: &EditEvent, w, cx| {
            if *ev == EditEvent::Changed {
                cx.emit(s.value(cx));
                cx.notify()
            }
            if *ev == EditEvent::Blurred {
                let val = s.value(cx);
                if ["US", "CA"].contains(&val.country) && val.national.len() == 10 {
                    let n = val.national;
                    e.update(cx, |e, cx| {
                        e.set_value(format!("({}) {}-{}", &n[..3], &n[3..6], &n[6..]), w, cx)
                    })
                }
            }
        });
        let q_sub = cx.subscribe_in(&query, w, |s: &mut Self, _, ev: &EditEvent, w, cx| {
            let ids = s.matches(cx);
            match ev {
                EditEvent::Changed => {
                    let opening = s.clearing_search && s.query.read(cx).value().is_empty();
                    s.clearing_search = false;
                    if !opening {
                        s.active = 0;
                        s.scroll.set_offset(point(px(0.), px(0.)));
                        s.indicator.reset(0.);
                        s.reveal_active = true;
                    }
                }
                EditEvent::Escape => s.toggle(w, cx),
                EditEvent::Navigate(next) if !ids.is_empty() => {
                    s.active = if *next {
                        (s.active + 1).min(ids.len() - 1)
                    } else {
                        s.active.saturating_sub(1)
                    };
                    s.reveal_active = true;
                }
                EditEvent::Submitted => {
                    if let Some(&i) = ids.get(s.active) {
                        s.choose(i, w, cx)
                    }
                }
                _ => {}
            }
            cx.notify()
        });
        Self {
            label,
            country,
            edit,
            query,
            open: false,
            reveal: FormReveal::new(false),
            indicator: Slide::snappy(0.),
            dial: Default::default(),
            direction: 1.,
            active: 0,
            scroll: ScrollHandle::new(),
            reveal_active: false,
            clearing_search: false,
            focus: cx.focus_handle().tab_stop(true),
            _subs: vec![edit_sub, q_sub],
        }
    }
    pub fn value(&self, cx: &App) -> PhoneValue {
        let c = COUNTRIES[self.country];
        let raw = self.edit.read(cx).value().trim();
        let digits = raw.chars().filter(char::is_ascii_digit).collect::<String>();
        let national = if raw.starts_with('+') {
            digits
                .strip_prefix(c.3.trim_start_matches('+'))
                .unwrap_or(&digits)
                .to_owned()
        } else {
            digits
        };
        let possible = match c.0 {
            "US" | "CA" => national.len() == 10,
            "CN" => national.len() == 11,
            _ => (4..=15 - c.3.len() + 1).contains(&national.len()),
        };
        PhoneValue {
            country: c.0,
            international: format!("{}{}", c.3, national),
            national,
            possible,
        }
    }
    fn matches(&self, cx: &App) -> Vec<usize> {
        let q = self.query.read(cx).value().trim().to_lowercase();
        COUNTRIES
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                [c.0, c.1, c.2, c.3]
                    .into_iter()
                    .any(|v| v.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn choose(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.direction = if i >= self.country { 1. } else { -1. };
        self.country = i;
        self.edit.update(cx, |edit, cx| {
            edit.set_placeholder(COUNTRIES[i].4);
            cx.notify();
        });
        self.open = false;
        self.edit.read(cx).focus.clone().focus(w, cx);
        cx.emit(self.value(cx));
        cx.notify()
    }
    fn toggle(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            self.clearing_search = !self.query.read(cx).value().is_empty();
            self.query.update(cx, |e, cx| {
                e.set_value("", w, cx);
                e.focus(w, cx)
            });
            self.active = self.country;
            self.indicator.reset(self.active as f32);
            self.scroll.set_offset(point(px(0.), px(0.)));
            self.reveal_active = true;
        } else {
            self.focus.focus(w, cx)
        }
        cx.notify()
    }
}
impl Render for PhoneInput {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let c = COUNTRIES[self.country];
        let zh = tr(cx, "en", "zh") == "zh";
        let progress = self.reveal.sample(self.open, 0.40, 0.32, w, cx);
        self.indicator.duration(self.active as f32, 0.28, cx);
        let code = self.dial.label(c.3.to_owned(), self.direction, w, cx);
        let mut root = section(380., cx)
            .id("phone")
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if !s.open || !s.query.read(cx).focus.is_focused(w) {
                    return;
                }
                if e.keystroke.key == "tab" {
                    s.open = false;
                    cx.notify();
                    return;
                }
                let n = s.matches(cx).len();
                if n == 0 {
                    return;
                }
                let next = match e.keystroke.key.as_str() {
                    "home" if s.query.read(cx).value().is_empty() => 0,
                    "end" if s.query.read(cx).value().is_empty() => n - 1,
                    "pageup" => s.active.saturating_sub(8),
                    "pagedown" => (s.active + 8).min(n - 1),
                    _ => return,
                };
                s.active = next;
                s.reveal_active = true;
                w.prevent_default();
                cx.stop_propagation();
                cx.notify();
            }))
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.open {
                    s.open = false;
                    cx.notify()
                }
            }))
            .child(self.label.clone())
            .child(
                row()
                    .h(px(44.))
                    .p(px(4.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .child(
                        button("country", "", cx)
                            .child(flag(c.0))
                            .child(code)
                            .aria_label(format!("{} {}", if zh { c.2 } else { c.1 }, c.3))
                            .aria_expanded(self.open)
                            .debug_selector(|| "phone-country-trigger".into())
                            .track_focus(&self.focus)
                            .child(icon(Glyph::Down, 13., p.faint))
                            .on_click(cx.listener(|s, _, w, cx| s.toggle(w, cx)))
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                                if !s.open && matches!(e.keystroke.key.as_str(), "up" | "down") {
                                    s.toggle(w, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            })),
                    )
                    .child(div().flex_1().min_w_0().h(px(30.)).child(self.edit.clone())),
            );
        if self.open || progress > 0. {
            let ids = self.matches(cx);
            let height = if ids.is_empty() {
                44.
            } else {
                (ids.len() as f32 * (ROW_HEIGHT + ROW_GAP) - ROW_GAP).min(LIST_HEIGHT)
            };
            if self.reveal_active && self.scroll.bounds_for_item(self.active + 1).is_some() {
                self.scroll.scroll_to_item(self.active + 1);
                self.reveal_active = false;
            }
            let mut rows = col()
                .id("countries")
                .size_full()
                .gap(px(ROW_GAP))
                .role(Role::ListBox)
                .aria_label(tr(cx, "Countries", "国家或地区"))
                .debug_selector(|| "phone-country-list".into())
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .on_scroll(cx.listener(|_, _, _, cx| cx.notify()));
            rows = rows.relative().child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px((ROW_HEIGHT + ROW_GAP) * self.indicator.get(w, cx)))
                    .h(px(ROW_HEIGHT))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.hover)
                    .when(ids.is_empty(), |highlight| highlight.opacity(0.)),
            );
            for (pos, &i) in ids.iter().enumerate() {
                let c = COUNTRIES[i];
                rows = rows.child(
                    face(
                        ("country", i),
                        "",
                        ROW_HEIGHT,
                        crate::foundation::surface_radius(cx).into(),
                        Tone::Clear,
                        cx,
                    )
                    .w_full()
                    .h(px(ROW_HEIGHT))
                    .flex_none()
                    .gap(px(10.))
                    .px(px(10.))
                    .justify_start()
                    .role(Role::ListBoxOption)
                    .tab_index(-1)
                    .aria_selected(i == self.country)
                    .text_color(if pos == self.active || i == self.country {
                        p.ink
                    } else {
                        p.secondary
                    })
                    .aria_label(format!("{} {}", if zh { c.2 } else { c.1 }, c.3))
                    .debug_selector(move || format!("phone-country-{}", c.0))
                    .child(flag(c.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .font_weight(FontWeight::MEDIUM)
                            .child(if zh { c.2 } else { c.1 }),
                    )
                    .child(div().text_size(px(12.)).text_color(p.faint).child(c.3))
                    .child(
                        div()
                            .size(px(16.))
                            .flex_none()
                            .when(i == self.country, |v| {
                                v.child(icon(Glyph::Check, 16., p.ink))
                            }),
                    )
                    .on_hover(cx.listener(move |s, over: &bool, w, cx| {
                        if *over && !w.last_input_was_touch() && s.active != pos {
                            s.active = pos;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |s, _, w, cx| s.choose(i, w, cx))),
                )
            }
            if ids.is_empty() {
                rows = rows.child(div().px(px(10.)).py(px(12.)).text_color(p.faint).child(tr(
                    cx,
                    "No countries found",
                    "没有匹配的国家",
                )));
            }
            let scroll = self.scroll.clone();
            let report = self.open;
            let viewport = div()
                .relative()
                .w_full()
                .h(px(height))
                .flex_none()
                .child(rows)
                .child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, _, cx| {
                            if report && let Some(host) = cx.try_global::<EditorScrollRegion>() {
                                let top = -scroll.offset().y;
                                (host.0)(
                                    bounds,
                                    top > px(0.1),
                                    top + px(0.1) < scroll.max_offset().y,
                                );
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                );
            root = root.child(
                self.reveal
                    .wrap(
                        panel(cx)
                            .child(div().h(px(32.)).child(self.query.clone()))
                            .child(viewport),
                        progress,
                        self.open,
                        height + 56.,
                        0.,
                    )
                    .w(px(105. + 275. * progress))
                    .max_w_full(),
            )
        }
        root.child(hint(
            tr(
                cx,
                "Type + to search by calling code",
                "输入 + 可按国际区号搜索",
            ),
            cx,
        ))
    }
}
const COUNTRIES: &[(&str, &str, &str, &str, &str)] = &[
    ("AR", "Argentina", "阿根廷", "+54", "11 2345-6789"),
    ("AU", "Australia", "澳大利亚", "+61", "412 345 678"),
    ("AT", "Austria", "奥地利", "+43", "664 1234567"),
    ("BE", "Belgium", "比利时", "+32", "470 12 34 56"),
    ("BR", "Brazil", "巴西", "+55", "(11) 96123-4567"),
    ("CA", "Canada", "加拿大", "+1", "(506) 555-0123"),
    ("CL", "Chile", "智利", "+56", "2 2123 4567"),
    ("CN", "China", "中国", "+86", "131 2345 6789"),
    ("CO", "Colombia", "哥伦比亚", "+57", "321 123 4567"),
    ("CZ", "Czechia", "捷克", "+420", "601 123 456"),
    ("DK", "Denmark", "丹麦", "+45", "32 12 34 56"),
    ("EG", "Egypt", "埃及", "+20", "100 123 4567"),
    ("FI", "Finland", "芬兰", "+358", "41 234 5678"),
    ("FR", "France", "法国", "+33", "6 12 34 56 78"),
    ("DE", "Germany", "德国", "+49", "151 23456789"),
    ("GR", "Greece", "希腊", "+30", "691 234 5678"),
    ("HK", "Hong Kong", "中国香港", "+852", "5123 4567"),
    ("IN", "India", "印度", "+91", "81234-56789"),
    ("ID", "Indonesia", "印度尼西亚", "+62", "812-3456-7890"),
    ("IE", "Ireland", "爱尔兰", "+353", "85 012 3456"),
    ("IL", "Israel", "以色列", "+972", "50-234-5678"),
    ("IT", "Italy", "意大利", "+39", "312 345 6789"),
    ("JP", "Japan", "日本", "+81", "90-1234-5678"),
    ("KE", "Kenya", "肯尼亚", "+254", "712 123456"),
    ("MX", "Mexico", "墨西哥", "+52", "22 2123 4567"),
    ("NL", "Netherlands", "荷兰", "+31", "6 12345678"),
    ("NZ", "New Zealand", "新西兰", "+64", "21 123 4567"),
    ("NG", "Nigeria", "尼日利亚", "+234", "802 123 4567"),
    ("NO", "Norway", "挪威", "+47", "406 12 345"),
    ("PK", "Pakistan", "巴基斯坦", "+92", "301 2345678"),
    ("PE", "Peru", "秘鲁", "+51", "912 345 678"),
    ("PH", "Philippines", "菲律宾", "+63", "905 123 4567"),
    ("PL", "Poland", "波兰", "+48", "512 345 678"),
    ("PT", "Portugal", "葡萄牙", "+351", "912 345 678"),
    ("SA", "Saudi Arabia", "沙特阿拉伯", "+966", "51 234 5678"),
    ("SG", "Singapore", "新加坡", "+65", "8123 4567"),
    ("ZA", "South Africa", "南非", "+27", "71 123 4567"),
    ("KR", "South Korea", "韩国", "+82", "10-2000-0000"),
    ("ES", "Spain", "西班牙", "+34", "612 34 56 78"),
    ("SE", "Sweden", "瑞典", "+46", "70 123 45 67"),
    ("CH", "Switzerland", "瑞士", "+41", "78 123 45 67"),
    ("TW", "Taiwan", "中国台湾", "+886", "912 345 678"),
    ("TH", "Thailand", "泰国", "+66", "81 234 5678"),
    ("TR", "Turkey", "土耳其", "+90", "501 234 56 78"),
    ("UA", "Ukraine", "乌克兰", "+380", "50 123 45 67"),
    (
        "AE",
        "United Arab Emirates",
        "阿联酋",
        "+971",
        "50 123 4567",
    ),
    ("GB", "United Kingdom", "英国", "+44", "7400 123456"),
    ("US", "United States", "美国", "+1", "(201) 555-0123"),
    ("VN", "Vietnam", "越南", "+84", "91 234 56 78"),
];

#[cfg(test)]
mod tests {
    use super::{COUNTRIES, PhoneInput};
    use crate::primitives::{editor::EditorScrollRegion, phone_flags};
    use gpui::{
        Bounds, Modifiers, Pixels, PlatformInput, ScrollDelta, ScrollEvent, TestAppContext,
        TouchEvent, TouchId, TouchPhase, VisualTestContext, point, px, size,
    };
    use std::{cell::Cell, time::Duration};
    thread_local! {
        static REGION: Cell<(Bounds<Pixels>, bool, bool, usize)> = Cell::default();
    }
    fn tick(w: &mut VisualTestContext, millis: u64) {
        w.executor().advance_clock(Duration::from_millis(millis));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }
    #[test]
    fn country_list_reports_scrolls_reveals_keyboard_targets_and_restores_focus() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            cx.set_global(EditorScrollRegion(|b, before, after| {
                REGION.with(|r| r.set((b, before, after, r.get().3 + 1)))
            }));
        });
        let (phone, mut w) = app.add_window_view(|w, cx| PhoneInput::new("Phone", "US", w, cx));
        w.simulate_resize(size(px(520.), px(750.)));
        let trigger = w.debug_bounds("phone-country-trigger").unwrap().center();
        w.simulate_click(trigger, Modifiers::default());
        tick(&mut w, 500);
        tick(&mut w, 30);
        let (region, before, _, _) = REGION.with(Cell::get);
        assert!(
            before,
            "opening must reveal the selected US row near the end"
        );
        assert!((f32::from(region.size.height) - 280.).abs() < 1.);
        let offset = w.update(|_, cx| phone.read(cx).scroll.offset().y);
        w.update(|w, cx| {
            w.dispatch_event(
                PlatformInput::Scroll(ScrollEvent {
                    position: region.center(),
                    delta: ScrollDelta::Pixels(point(px(0.), px(130.))),
                    ..Default::default()
                }),
                cx,
            );
        });
        tick(&mut w, 30);
        assert!(
            w.update(|_, cx| phone.read(cx).scroll.offset().y) > offset,
            "wheel up scrolls the inner list"
        );
        w.simulate_keystrokes("home");
        tick(&mut w, 40);
        tick(&mut w, 40);
        assert_eq!(w.update(|_, cx| phone.read(cx).active), 0);
        assert!(!REGION.with(|r| r.get().1));
        let center = REGION.with(|r| r.get().0.center());
        for (phase, dy) in [
            (TouchPhase::Started, 0.),
            (TouchPhase::Moved, -90.),
            (TouchPhase::Cancelled, -90.),
        ] {
            w.simulate_event(TouchEvent {
                id: TouchId(71),
                phase,
                position: center + point(px(0.), px(dy)),
                force: None,
                predicted_position: None,
            });
            tick(&mut w, 20);
        }
        w.update(|_, cx| {
            let s = phone.read(cx);
            assert!(
                s.scroll.offset().y < -px(1.),
                "a touch pan must scroll the list"
            );
            assert_eq!(
                s.value(cx).country,
                "US",
                "panning must not select the touched row"
            );
        });
        w.simulate_keystrokes("end");
        tick(&mut w, 40);
        tick(&mut w, 40);
        w.update(|_, cx| {
            let s = phone.read(cx);
            assert_eq!(s.active, COUNTRIES.len() - 1);
            let row = s.scroll.bounds_for_item(s.active + 1).unwrap();
            let viewport = s.scroll.bounds();
            assert!(
                row.bottom() + s.scroll.offset().y <= viewport.bottom() + px(1.),
                "End target must be visible"
            );
        });
        w.simulate_keystrokes("escape");
        tick(&mut w, 600);
        w.update(|w, cx| {
            assert!(!phone.read(cx).open);
            assert!(phone.read(cx).focus.is_focused(w));
        });
        let count = REGION.with(|r| r.get().3);
        tick(&mut w, 100);
        assert_eq!(
            REGION.with(|r| r.get().3),
            count,
            "closed list must stop intercepting wheel"
        );
    }
    #[test]
    fn country_filter_empty_and_reopen_keep_the_committed_country() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (phone, mut w) = app.add_window_view(|w, cx| PhoneInput::new("Phone", "US", w, cx));
        w.update(|w, cx| phone.update(cx, |s, cx| s.toggle(w, cx)));
        tick(&mut w, 500);
        w.update(|w, cx| {
            let q = phone.read(cx).query.clone();
            q.update(cx, |e, cx| e.set_value("+86", w, cx));
        });
        tick(&mut w, 80);
        assert_eq!(w.update(|_, cx| phone.read(cx).matches(cx)), vec![7]);
        w.simulate_keystrokes("enter");
        tick(&mut w, 500);
        assert_eq!(w.update(|_, cx| phone.read(cx).value(cx).country), "CN");
        w.update(|w, cx| phone.update(cx, |s, cx| s.toggle(w, cx)));
        tick(&mut w, 500);
        tick(&mut w, 30);
        w.update(|_, cx| {
            let s = phone.read(cx);
            assert_eq!(
                s.matches(cx)[s.active],
                s.country,
                "reopening must focus the selected country"
            );
        });
        w.update(|w, cx| {
            let q = phone.read(cx).query.clone();
            q.update(cx, |e, cx| e.set_value("not-a-country", w, cx));
        });
        tick(&mut w, 100);
        assert!(w.update(|_, cx| phone.read(cx).matches(cx).is_empty()));
        w.simulate_keystrokes("enter");
        assert_eq!(w.update(|_, cx| phone.read(cx).value(cx).country), "CN");
        assert!(w.update(|_, cx| phone.read(cx).open));
    }
    #[test]
    fn every_selectable_country_has_an_embedded_image_and_flags_are_cached() {
        for country in COUNTRIES {
            assert!(
                phone_flags::image(country.0).is_some(),
                "missing flag {}",
                country.0
            );
        }
        let first = phone_flags::image("US").unwrap();
        let second = phone_flags::image("US").unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(phone_flags::image("XX").is_none());
    }
}
